$GhArguments = $args

$gh = Get-Command gh -ErrorAction SilentlyContinue
if ($null -eq $gh) {
    $installedGh = Join-Path $env:ProgramFiles 'GitHub CLI\gh.exe'
    if (-not (Test-Path -LiteralPath $installedGh)) {
        throw 'GitHub CLI is not installed.'
    }
    $ghPath = $installedGh
}
else {
    $ghPath = $gh.Source
}

$credentialInput = "protocol=https`nhost=github.com`n`n"
$credentialLines = $credentialInput | git credential fill
$credential = @{}

foreach ($line in $credentialLines) {
    $separator = $line.IndexOf('=')
    if ($separator -gt 0) {
        $credential[$line.Substring(0, $separator)] = $line.Substring($separator + 1)
    }
}

if (-not $credential.ContainsKey('password')) {
    throw 'No GitHub credential is available from Git Credential Manager.'
}

$previousToken = $env:GH_TOKEN
try {
    $env:GH_TOKEN = $credential['password']
    & $ghPath @GhArguments
    $ghExitCode = $LASTEXITCODE
}
finally {
    $env:GH_TOKEN = $previousToken
}

exit $ghExitCode
