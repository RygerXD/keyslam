param(
    [string]$OutputRoot = (Join-Path $PSScriptRoot '..\assets\sounds'),
    [string]$PacksRoot = (Join-Path $PSScriptRoot '..\assets\packs'),
    [switch]$SkipCommon,
    [switch]$ExtraKeySetsOnly,
    [ValidateSet('animals', 'foods', 'instruments', 'directions')]
    [string[]]$Sets = @('animals', 'foods', 'instruments', 'directions'),
    [string]$Voice = 'Microsoft Zira Desktop',
    [int]$Rate = -1,
    [switch]$ReplaceExisting
)

# Generate in isolation first. Never read the personal asset-path setting.
$ErrorActionPreference = 'Stop'
$repo = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$staging = Join-Path $repo "target\speech-$([guid]::NewGuid().ToString('N'))"
$clips = [System.Collections.Generic.List[object]]::new()
Add-Type -AssemblyName System.Speech
$synth = [System.Speech.Synthesis.SpeechSynthesizer]::new()
$synth.SelectVoice($Voice)
$synth.Rate = $Rate

function Add-Clip([string]$Category, [string]$RelativePath, [string]$Text) {
    $wav = Join-Path $staging "$Category\$RelativePath.wav"
    [System.IO.Directory]::CreateDirectory((Split-Path -Parent $wav)) | Out-Null
    $synth.SetOutputToWaveFile($wav)
    try { $synth.Speak($Text) }
    finally { $synth.SetOutputToNull() }
    if ((Get-Item -LiteralPath $wav).Length -le 46) {
        throw "The selected voice produced no audio for '$Text'"
    }
    $clips.Add([pscustomobject]@{
        category = $Category
        relative = "$RelativePath.opus"
        text = $Text
    })
}

try {
    if (-not $SkipCommon -and -not $ExtraKeySetsOnly) {
        foreach ($letter in [char[]]'ABCDEFGHIJKLMNOPQRSTUVWXYZ') {
            $name = $letter.ToString().ToLowerInvariant()
            Add-Clip 'sounds' "letters\$name\$name" "$letter."
        }
        foreach ($digit in 0..9) {
            Add-Clip 'sounds' "numbers\$digit\$digit" "$digit"
        }
        # Includes all paint colors, not just the twelve figure colors.
        $colors = @('Red', 'Orange', 'Yellow', 'Green', 'Blue', 'Indigo', 'Violet',
            'Pink', 'Brown', 'White', 'Gray', 'Black', 'Amber', 'Lime-green', 'Orange-red', 'Teal')
        foreach ($color in $colors) {
            $name = $color.ToLowerInvariant()
            $text = $color.Replace('-', ' ')
            Add-Clip 'sounds' "colors\standalone\$name\$name" "$text."
            Add-Clip 'sounds' "colors\modifier\$name\$name" $text
        }
        $shapes = @('Star', 'Cross', 'Heart', 'Oval', 'Rectangle', 'Triangle',
            'Square', 'Pentagon', 'Hexagon', 'Septagon', 'Octagon', 'Trapezoid', 'Circle')
        foreach ($shape in $shapes) {
            $name = $shape.ToLowerInvariant()
            Add-Clip 'sounds' "shapes\$name\$name" $shape
        }
    }
    foreach ($set in $Sets) {
        $directory = Join-Path $PacksRoot $set
        if (-not (Test-Path -LiteralPath $directory -PathType Container)) {
            throw "Missing public pack: $directory"
        }
        $manifest = Get-Content -LiteralPath (Join-Path $directory 'pack.json') -Raw | ConvertFrom-Json
        foreach ($item in (Get-ChildItem -LiteralPath $directory -Directory | Sort-Object Name)) {
            $name = $item.Name
            $label = $manifest.keys | Where-Object { $_.item -eq $name -and $_.name } |
                Select-Object -First 1 -ExpandProperty name
            if (-not $label) { $label = $name.Replace('-', ' ').Replace('_', ' ') }
            Add-Clip 'packs' "$set\$name\$name" $label
        }
    }
}
finally { $synth.Dispose() }

Push-Location $repo
try {
    cargo run --quiet --example encode_speech -- $staging
    if ($LASTEXITCODE -ne 0) { throw "Opus encoding failed: $LASTEXITCODE" }
}
finally { Pop-Location }

$outputRoots = @{ sounds = [System.IO.Path]::GetFullPath($OutputRoot); packs = [System.IO.Path]::GetFullPath($PacksRoot) }
# Refuse to erase existing recordings unless explicitly requested.
foreach ($clip in $clips) {
    $destination = Join-Path $outputRoots[$clip.category] $clip.relative
    $parent = Split-Path -Parent $destination
    $existing = @(Get-ChildItem -LiteralPath $parent -Filter '*.opus' -File -ErrorAction SilentlyContinue)
    if ($existing.Count -gt 0 -and -not $ReplaceExisting) {
        throw "Existing audio in $parent. Back it up, then use -ReplaceExisting to replace the voice cycle."
    }
}

$inventory = @()
$manifestPath = Join-Path $repo 'assets\public-audio.json'
if (Test-Path -LiteralPath $manifestPath) {
    $inventory = @((Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json).clips)
}
foreach ($clip in $clips) {
    $destination = Join-Path $outputRoots[$clip.category] $clip.relative
    $parent = Split-Path -Parent $destination
    [System.IO.Directory]::CreateDirectory($parent) | Out-Null
    # Preserve replaced takes in the ignored staging folder as an extra backup.
    $backup = Join-Path $staging ("replaced\$($clip.category)\" + (Split-Path -Parent $clip.relative))
    [System.IO.Directory]::CreateDirectory($backup) | Out-Null
    Get-ChildItem -LiteralPath $parent -Filter '*.opus' -File |
        ForEach-Object { Move-Item -LiteralPath $_.FullName -Destination $backup }
    Copy-Item -LiteralPath (Join-Path $staging "$($clip.category)\$($clip.relative)") -Destination $destination
    $publicPath = "assets/$($clip.category)/$($clip.relative.Replace('\', '/'))"
    $inventory = @($inventory | Where-Object { $_.path -ne $publicPath })
    $inventory += [pscustomobject]@{
        path = $publicPath
        text = $clip.text
        engine = 'Windows System.Speech'
        voice = $Voice
        rate = $Rate
        sha256 = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}
# Only the standard public destinations update the checked-in provenance inventory.
if ($outputRoots.sounds -eq (Join-Path $repo 'assets\sounds') -and
    $outputRoots.packs -eq (Join-Path $repo 'assets\packs')) {
    [pscustomobject]@{ clips = @($inventory | Sort-Object path) } |
        ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $manifestPath -Encoding UTF8
    $rules = @('# Generated by tools/generate_starter_speech.ps1; only approved audio is tracked.',
        '*.opus', '*.mp3', '*.ogg', '*.flac')
    $rules += @($inventory | Sort-Object path | ForEach-Object { '!/' + $_.path.Substring(7) })
    $rules | Set-Content -LiteralPath (Join-Path $repo 'assets\.gitignore') -Encoding UTF8
}
Write-Output "Generated $($clips.Count) synthetic clips with $Voice."
