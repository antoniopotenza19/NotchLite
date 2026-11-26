# Original NotchLite helper. SPDX-License-Identifier: Apache-2.0
[CmdletBinding()]
param(
    [string]$DestinationRoot = (Join-Path $env:LOCALAPPDATA 'NotchLite\codex-setup'),
    [switch]$Start
)
$ErrorActionPreference = 'Stop'
$packageVersion = '0.171.0-alpha.13.2'
$assetName = 'codex-package-x86_64-pc-windows-msvc.tar.gz'
$expectedSha256 = '6d3be3ea2e3f759f77f23b543ee882aae6433dc6a6cd260788c2e8793d616eac'
$downloadUrl = 'https://github.com/openai/codex/releases/download/rust-v' + $packageVersion + '/' + $assetName
if ($DestinationRoot -notmatch '^[A-Za-z]:[\\/]') {
    throw 'Indica una cartella locale con percorso assoluto.'
}
$setupRoot = [IO.Path]::GetFullPath($DestinationRoot)
$destinationDrive = [IO.DriveInfo]::new([IO.Path]::GetPathRoot($setupRoot))
if ($destinationDrive.DriveType -eq [IO.DriveType]::Network) { throw 'La cartella di destinazione deve essere su un disco locale.' }
New-Item -ItemType Directory -Force -Path $setupRoot | Out-Null
$archivePath = Join-Path $setupRoot $assetName
if (-not (Test-Path -LiteralPath $archivePath)) {
    Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath
}
if ((Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expectedSha256) {
    throw 'SHA-256 diverso dalla release ufficiale: nessuna estrazione o esecuzione.'
}
$entries = @(& tar -tf $archivePath)
if ($LASTEXITCODE -ne 0 -or $entries.Count -eq 0 -or $entries.Count -gt 10000) { throw 'Archivio non valido.' }
foreach ($entry in $entries) {
    $normalized = $entry.Replace('\','/')
    if ($normalized.StartsWith('/') -or $normalized.Contains(':') -or ($normalized.Split('/') -contains '..')) {
        throw 'Percorso non sicuro nell''archivio.'
    }
}
$details = @(& tar -tvf $archivePath)
if ($LASTEXITCODE -ne 0 -or ($details | Where-Object { $_ -match '^[lh]' })) { throw 'Archivio con collegamenti o metadati non validi.' }
# A fresh directory avoids overwriting an existing extracted package.
$packageRoot = Join-Path $setupRoot ('package-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $packageRoot | Out-Null
& tar -xf $archivePath -C $packageRoot
if ($LASTEXITCODE -ne 0) { throw 'Estrazione non riuscita.' }
$manifest = Get-Content -LiteralPath (Join-Path $packageRoot 'codex-package.json') -Raw | ConvertFrom-Json
if ($manifest.version -ne $packageVersion -or $manifest.entrypoint -ne 'bin/codex.exe' -or $manifest.target -ne 'x86_64-pc-windows-msvc') {
    throw 'Manifest del pacchetto inatteso.'
}
foreach ($relativePath in @('bin/codex.exe','bin/codex-code-mode-host.exe','codex-path/rg.exe','codex-resources/codex-command-runner.exe','codex-resources/codex-windows-sandbox-setup.exe')) {
    if (-not (Test-Path -LiteralPath (Join-Path $packageRoot $relativePath) -PathType Leaf)) { throw 'Pacchetto CLI incompleto.' }
}
$cliPath = Join-Path $packageRoot 'bin/codex.exe'
$cliVersion = (& $cliPath --version | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $cliVersion -ne ('codex-cli ' + $packageVersion)) { throw 'Versione CLI inattesa.' }
Write-Output ('Pacchetto ufficiale verificato: ' + $cliVersion)
Write-Output ('CLI: ' + $cliPath)
if ($Start) {
    # The official command manages its own package/state under the existing CODEX_HOME.
    & $cliPath app-server daemon start
    if ($LASTEXITCODE -ne 0) { throw 'Avvio del daemon non riuscito.' }
} else {
    Write-Output 'Server non avviato. Usa -Start per avviarlo esplicitamente; nessun login, autorun o PATH globale modificato.'
}
