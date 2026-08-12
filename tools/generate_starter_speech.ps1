param(
    [string]$OutputRoot = (Join-Path $PSScriptRoot '..\assets\speech'),
    [string[]]$Locales = @(),
    [switch]$SkipCommon
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

$commonSynth = $null
if (-not $SkipCommon) {
    $commonSynth = New-LocaleSynthesizer 'en-EN'
    try {
        foreach ($letter in [char[]]'ABCDEFGHIJKLMNOPQRSTUVWXYZ') {
            Write-Clip $commonSynth "common\letters\$($letter.ToString().ToLowerInvariant()).wav" $letter
        }
        foreach ($digit in 0..9) {
            Write-Clip $commonSynth "common\numbers\$digit.wav" $digit
        }

        $source = [System.IO.File]::ReadAllText((Join-Path $PSScriptRoot '..\src\responses.rs'))
        $body = [regex]::Match(
            $source,
            'const OTHER_ANIMALS:.*?= \[(?<body>.*?)\];',
            [System.Text.RegularExpressions.RegexOptions]::Singleline
        ).Groups['body'].Value
        $animals = @('Bear', 'Tiger') + @(
            [regex]::Matches($body, '\(".*?", "(?<name>[^"]+)"\)') |
                ForEach-Object { $_.Groups['name'].Value }
        )
        foreach ($animal in $animals) {
            Write-Clip $commonSynth "common\animals\$($animal.ToLowerInvariant()).wav" $animal
        }
    }
    finally {
        $commonSynth.Dispose()
    }
}

$colors = @('Red', 'Orange', 'Yellow', 'Green', 'Blue', 'Indigo', 'Violet', 'Pink', 'Brown', 'White', 'Gray', 'Black')
$shapes = @('Star', 'Oval', 'Rectangle', 'Triangle', 'Square', 'Pentagon', 'Hexagon', 'Septagon', 'Octagon', 'Trapezoid', 'Circle')
$stringFiles = Get-ChildItem (Join-Path $PSScriptRoot '..\assets\strings\*.json')
if ($Locales.Count -gt 0) {
    $stringFiles = $stringFiles | Where-Object { $_.BaseName -in $Locales }
}
$stringFiles | ForEach-Object {
    $locale = $_.BaseName
    $strings = [System.IO.File]::ReadAllText($_.FullName) | ConvertFrom-Json
    $synth = New-LocaleSynthesizer $locale
    try {
        $strings.PSObject.Properties |
            Where-Object { $_.Name -match '^[A-Za-z0-9]$' } |
            ForEach-Object {
                Write-Clip $synth "$locale\letters\$($_.Name.ToLowerInvariant()).wav" $_.Value
            }
        foreach ($color in $colors) {
            $property = $strings.PSObject.Properties[$color]
            $text = if ($null -eq $property) { $color } else { $property.Value }
            Write-Clip $synth "$locale\colors\$($color.ToLowerInvariant()).wav" $text
        }
        foreach ($shape in $shapes) {
            $property = $strings.PSObject.Properties[$shape]
            $text = if ($null -eq $property) { $shape } else { $property.Value }
            Write-Clip $synth "$locale\shapes\$($shape.ToLowerInvariant()).wav" $text
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
}
finally {
    Pop-Location
}

$resolvedOutput = [System.IO.Path]::GetFullPath($OutputRoot)
$expectedOutput = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\assets\speech'))
if ($resolvedOutput -eq $expectedOutput) {
    Get-ChildItem -LiteralPath $resolvedOutput -Recurse -Filter '*.wav' |
        Remove-Item -Force
}
