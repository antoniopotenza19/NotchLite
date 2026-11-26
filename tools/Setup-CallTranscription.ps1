# Original NotchLite helper. SPDX-License-Identifier: Apache-2.0
# Downloads pinned local CPU dependencies only. Never captures audio or starts a server.
[CmdletBinding()]
param([string]$DestinationRoot = (Join-Path $env:LOCALAPPDATA 'NotchLite\whisper'))
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$version = 'v1.9.2'
$archiveSha = '49dcc16de826f20bd53d44f947a1ae49dfa81f86cad67a64d80820cb192d674a'
$modelRevision = 'c521a4b02f422512d734391fdf08bb08c0862f68'
$modelSha = '818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7'
$archiveUrl = 'https://github.com/ggml-org/whisper.cpp/releases/download/' + $version + '/whisper-bin-x64.zip'
$modelUrl = 'https://huggingface.co/ggerganov/whisper.cpp/resolve/' + $modelRevision + '/ggml-tiny-q5_1.bin'
if (-not [Environment]::Is64BitOperatingSystem -or $env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { throw 'This pinned runtime requires Windows x64.' }
if ($DestinationRoot -notmatch '^[A-Za-z]:[\\/]') { throw 'Choose an absolute local directory.' }
$root = [IO.Path]::GetFullPath($DestinationRoot)
if ([IO.DriveInfo]::new([IO.Path]::GetPathRoot($root)).DriveType -eq [IO.DriveType]::Network) { throw 'Choose a local disk.' }
function Assert-NoReparse([string]$Path) {
    $candidate = $Path
    while ($candidate) {
        if (Test-Path -LiteralPath $candidate) {
            if ((Get-Item -LiteralPath $candidate -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Runtime paths cannot contain reparse points.' }
        }
        $candidate = Split-Path -Parent $candidate
    }
}
Assert-NoReparse $root
New-Item -ItemType Directory -Force -Path $root | Out-Null
$archive = Join-Path $root 'whisper-bin-x64-v1.9.2.zip'
$model = Join-Path $root 'ggml-tiny-q5_1.bin'
function Get-Verified([string]$Url, [string]$Path, [string]$Sha) {
    Assert-NoReparse $Path
    if (-not (Test-Path -LiteralPath $Path)) {
        $partial = $Path + '.' + [Guid]::NewGuid().ToString('N') + '.download'
        try {
            Invoke-WebRequest -Uri $Url -OutFile $partial
            if ((Get-FileHash -LiteralPath $partial -Algorithm SHA256).Hash.ToLowerInvariant() -ne $Sha) { throw 'Official download SHA-256 mismatch.' }
            Move-Item -LiteralPath $partial -Destination $Path
        } finally { if (Test-Path -LiteralPath $partial) { Remove-Item -LiteralPath $partial } }
    }
    if ((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $Sha) { throw 'Existing download SHA-256 mismatch; nothing is executed.' }
}
Get-Verified $archiveUrl $archive $archiveSha
Get-Verified $modelUrl $model $modelSha
Add-Type -AssemblyName System.IO.Compression.FileSystem
$packageName = 'cpu-' + $version + '-' + [Guid]::NewGuid().ToString('N')
$packageRoot = Join-Path $root $packageName
New-Item -ItemType Directory -Path $packageRoot | Out-Null
$zip = [IO.Compression.ZipFile]::OpenRead($archive)
try {
    if ($zip.Entries.Count -gt 200 -or $zip.Entries.Count -eq 0) { throw 'Unexpected archive entry count.' }
    $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($entry in $zip.Entries) {
        $name = $entry.FullName.Replace('\','/')
        if ($name.StartsWith('/') -or $name.Contains(':') -or ($name.Split('/') -contains '..') -or (($entry.ExternalAttributes -shr 16) -band 0xF000) -eq 0xA000) { throw 'Unsafe archive entry.' }
        # Exclude unrelated server, streaming and GUI examples. CPU DLLs are needed by CLI dispatch.
        if ($name -match '^Release/(whisper-cli\.exe|whisper\.dll|ggml(?:-base|-cpu-[a-z0-9]+)?\.dll)$') {
            if (-not $seen.Add($entry.Name) -or $entry.Length -gt 20MB) { throw 'Unexpected runtime entry.' }
            [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, (Join-Path $packageRoot $entry.Name), $false)
        }
    }
} finally { $zip.Dispose() }
foreach ($required in @('whisper-cli.exe','whisper.dll','ggml.dll','ggml-base.dll','ggml-cpu-x64.dll')) {
    if (-not (Test-Path -LiteralPath (Join-Path $packageRoot $required) -PathType Leaf)) { throw 'Incomplete CPU runtime.' }
}
$cli = Join-Path $packageRoot 'whisper-cli.exe'
if ((Get-FileHash -LiteralPath $cli -Algorithm SHA256).Hash.ToLowerInvariant() -ne '95e3c0b0e778ad9499eb0125f97c1dcf437dd9eb4ea77050b043574f93c2631d') { throw 'Unexpected CLI binary.' }
# Retain upstream MIT notices beside the optional dependency. No dependency is linked into NotchLite.
$license = @'
MIT License

whisper.cpp: Copyright (c) 2023-2026 The ggml authors
Whisper model: Copyright (c) 2022 OpenAI

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
'@
[IO.File]::WriteAllText((Join-Path $packageRoot 'LICENSE-MIT.txt'), $license)
$manifest = [ordered]@{ version=$version; executable=($packageName + '/whisper-cli.exe'); model='ggml-tiny-q5_1.bin'; archiveSha256=$archiveSha; modelSha256=$modelSha; modelRevision=$modelRevision; archiveUrl=$archiveUrl; modelUrl=$modelUrl }
$manifestTemp = Join-Path $root ('runtime-' + [Guid]::NewGuid().ToString('N') + '.json')
[IO.File]::WriteAllText($manifestTemp, ($manifest | ConvertTo-Json))
Assert-NoReparse (Join-Path $root 'runtime.json')
Move-Item -LiteralPath $manifestTemp -Destination (Join-Path $root 'runtime.json') -Force
Write-Output 'Local CPU transcription prepared and SHA-256 verified. No capture, model inference, server, autorun or global PATH change.'
Write-Output ('Runtime configuration: ' + (Join-Path $root 'runtime.json'))
