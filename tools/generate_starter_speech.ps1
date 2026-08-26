param(
    [string]$OutputRoot = (Join-Path $PSScriptRoot '..\assets\sounds'),
    [string]$PacksRoot = (Join-Path $PSScriptRoot '..\assets\packs'),
    [switch]$SkipCommon,
    [switch]$ExtraKeySetsOnly
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech

function New-LocaleSynthesizer([string]$Locale) {
    $synth = [System.Speech.Synthesis.SpeechSynthesizer]::new()
    $language = $Locale.Split('-')[0]
    $voice = $synth.GetInstalledVoices() |
        ForEach-Object { $_.VoiceInfo } |
        Where-Object {
            $_.Culture.Name -ieq $Locale -or $_.Culture.TwoLetterISOLanguageName -ieq $language
        } |
        Select-Object -First 1
    if ($null -ne $voice) {
        $synth.SelectVoice($voice.Name)
    }
    $synth
}

function Write-Clip($Synth, [string]$RelativePath, [string]$Text) {
    $path = Join-Path $OutputRoot $RelativePath
    $parent = Split-Path -Parent $path
    [System.IO.Directory]::CreateDirectory($parent) | Out-Null
    $Synth.SetOutputToWaveFile($path)
    $Synth.Speak($Text)
    if ((Get-Item -LiteralPath $path).Length -le 46) {
        Remove-Item -LiteralPath $path -Force
        throw "The selected voice produced no audio for '$Text' ($RelativePath)"
    }
}

function Write-PackClip($Synth, [string]$RelativePath, [string]$Text) {
    $previousRoot = $script:OutputRoot
    try {
        $script:OutputRoot = $PacksRoot
        Write-Clip $Synth $RelativePath $Text
    }
    finally {
        $script:OutputRoot = $previousRoot
    }
}

$commonSynth = $null
if (-not $SkipCommon -and -not $ExtraKeySetsOnly) {
    $commonSynth = New-LocaleSynthesizer 'en-EN'
    try {
        foreach ($letter in [char[]]'ABCDEFGHIJKLMNOPQRSTUVWXYZ') {
            Write-Clip $commonSynth "letters\$($letter.ToString().ToLowerInvariant()).wav" $letter
        }
        foreach ($digit in 0..9) {
            Write-Clip $commonSynth "numbers\$digit.wav" $digit
        }

        $animalFolders = Get-ChildItem -LiteralPath (Join-Path $PacksRoot 'animals') -Directory |
            Select-Object -ExpandProperty Name
        foreach ($folder in $animalFolders) {
            $animal = (Get-Culture).TextInfo.ToTitleCase($folder.Replace('-', ' ').Replace('_', ' '))
            Write-PackClip $commonSynth "animals\$folder\$folder.wav" $animal
        }
    }
    finally {
        $commonSynth.Dispose()
    }
}

$extraSynth = New-LocaleSynthesizer 'en-EN'
try {
    $extraItems = Import-Csv (Join-Path $PSScriptRoot '..\assets\images\extra-key-emoji.csv')
    foreach ($item in $extraItems) {
        $key = $item.name.ToLowerInvariant()
        Write-PackClip $extraSynth "$($item.set)\$key\$key.wav" $item.name
    }
}
finally {
    $extraSynth.Dispose()
}

if (-not $ExtraKeySetsOnly) {
    $colors = @('Red', 'Orange', 'Yellow', 'Green', 'Blue', 'Indigo', 'Violet', 'Pink', 'Brown', 'White', 'Gray', 'Black')
    $shapes = @('Star', 'Cross', 'Heart', 'Oval', 'Rectangle', 'Triangle', 'Square', 'Pentagon', 'Hexagon', 'Septagon', 'Octagon', 'Trapezoid', 'Circle')
    $locale = 'en-EN'
    $strings = [System.IO.File]::ReadAllText((Join-Path $PSScriptRoot '..\assets\strings\en-EN.json')) | ConvertFrom-Json
    $synth = New-LocaleSynthesizer $locale
    try {
        $strings.PSObject.Properties |
            Where-Object { $_.Name -match '^[A-Za-z0-9]$' } |
            ForEach-Object {
                Write-Clip $synth "letters\$($_.Name.ToLowerInvariant()).wav" $_.Value
            }
        foreach ($color in $colors) {
            $property = $strings.PSObject.Properties[$color]
            $text = if ($null -eq $property) { $color } else { $property.Value }
            Write-Clip $synth "colors\standalone\$($color.ToLowerInvariant()).wav" "$text."
            Write-Clip $synth "colors\modifier\$($color.ToLowerInvariant()).wav" $text
        }
        foreach ($shape in $shapes) {
            $property = $strings.PSObject.Properties[$shape]
            $text = if ($null -eq $property) { $shape } else { $property.Value }
            Write-Clip $synth "shapes\$($shape.ToLowerInvariant()).wav" $text
        }
    }
    finally {
        $synth.Dispose()
    }
}

Push-Location (Join-Path $PSScriptRoot '..')
try {
    cargo run --quiet --example encode_speech -- $OutputRoot
    if ($LASTEXITCODE -ne 0) {
        throw "The Opus encoder failed with exit code $LASTEXITCODE"
    }
    cargo run --quiet --example encode_speech -- $PacksRoot
    if ($LASTEXITCODE -ne 0) {
        throw "The pack Opus encoder failed with exit code $LASTEXITCODE"
    }
}
finally {
    Pop-Location
}

$resolvedOutput = [System.IO.Path]::GetFullPath($OutputRoot)
$expectedOutput = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\assets\sounds'))
if ($resolvedOutput -eq $expectedOutput) {
    Get-ChildItem -LiteralPath $resolvedOutput -Recurse -Filter '*.wav' |
        Remove-Item -Force
}
$resolvedPacks = [System.IO.Path]::GetFullPath($PacksRoot)
$expectedPacks = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\assets\packs'))
if ($resolvedPacks -eq $expectedPacks) {
    Get-ChildItem -LiteralPath $resolvedPacks -Recurse -Filter '*.wav' |
        Remove-Item -Force
}
