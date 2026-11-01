param(
    [string]$BackupDirectory = (Join-Path $PSScriptRoot '..\..\KeySlam-private\release-backups')
)

$ErrorActionPreference = 'Stop'
$repo = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$release = Join-Path $repo 'target\release'
$backupRoot = [System.IO.Path]::GetFullPath($BackupDirectory)
if ($backupRoot -eq $repo -or $backupRoot.StartsWith("$repo\", [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'Release asset backups must stay outside the public checkout.'
}
if ($env:KEYSLAM_ASSETS_DIR) {
    throw 'KEYSLAM_ASSETS_DIR overrides public playback. Unset it before preparing the public release.'
}
$executable = Join-Path $release 'keyslam.exe'
if (Get-Process keyslam -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $executable }) {
    throw 'Close the public KeySlam executable before rebuilding it.'
}

Push-Location $repo
try {
    python tools/check_public_assets.py
    if ($LASTEXITCODE -ne 0) { throw 'Public source asset verification failed.' }
    cargo build --release --target-dir (Join-Path $repo 'target')
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }

    $backup = Join-Path $backupRoot ((Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [guid]::NewGuid().ToString('N'))
    foreach ($name in @('assets', '.keyslam-assets-path')) {
        $source = Join-Path $release $name
        if (Test-Path -LiteralPath $source) {
            # Verify the absolute move source is directly inside this checkout's release folder.
            $resolved = (Get-Item -LiteralPath $source -Force).FullName
            if ([System.IO.Path]::GetDirectoryName($resolved) -ne $release) {
                throw "Unexpected release path: $resolved"
            }
            [System.IO.Directory]::CreateDirectory($backup) | Out-Null
            Move-Item -LiteralPath $resolved -Destination $backup
        }
    }
    Copy-Item -LiteralPath (Join-Path $repo 'assets') -Destination $release -Recurse
    python tools/check_public_assets.py --assets (Join-Path $release 'assets')
    if ($LASTEXITCODE -ne 0) { throw 'Installed public asset verification failed.' }
    Write-Output "Public executable ready: $executable"
    if (Test-Path -LiteralPath $backup) { Write-Output "Previous runtime assets preserved in: $backup" }
}
finally { Pop-Location }
