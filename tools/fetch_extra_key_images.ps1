param(
    [string]$OutputRoot = (Join-Path $PSScriptRoot '..\assets\packs')
)

$ErrorActionPreference = 'Stop'
$notoRevision = '8998f5dd683424a73e2314a8c1f1e359c19e8742'
$fluentRevision = '62ecdc0d7ca5c6df32148c169556bc8d3782fca4'
$manifest = Import-Csv (Join-Path $PSScriptRoot '..\assets\images\extra-key-emoji.csv')

function Escape-PathSegment([string]$Value) {
    [System.Uri]::EscapeDataString($Value).Replace('%2F', '/')
}

foreach ($item in $manifest) {
    $itemName = $item.name.ToLowerInvariant()
    $directory = Join-Path $OutputRoot (Join-Path $item.set $itemName)
    [System.IO.Directory]::CreateDirectory($directory) | Out-Null

    $notoUrl = "https://raw.githubusercontent.com/googlefonts/noto-emoji/$notoRevision/png/128/emoji_u$($item.codepoint).png"
    $notoPath = Join-Path $directory "android-$($item.codepoint).png"
    Invoke-WebRequest -UseBasicParsing -Uri $notoUrl -OutFile $notoPath

    $fluentFile = $item.fluent_name.Replace(' ', '_').ToLowerInvariant() + '_3d.png'
    $fluentFolder = Escape-PathSegment $item.fluent_name
    $fluentUrl = "https://raw.githubusercontent.com/microsoft/fluentui-emoji/$fluentRevision/assets/$fluentFolder/3D/$fluentFile"
    $fluentPath = Join-Path $directory "fluent-$($item.codepoint).png"
    Invoke-WebRequest -UseBasicParsing -Uri $fluentUrl -OutFile $fluentPath
}
