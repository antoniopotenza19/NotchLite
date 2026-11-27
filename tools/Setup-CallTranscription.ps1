# Original NotchLite helper. SPDX-License-Identifier: Apache-2.0
# Downloads pinned local CPU/Vulkan dependencies only. Never captures audio or starts a server.
[CmdletBinding()]
param(
    [string]$DestinationRoot = (Join-Path $env:LOCALAPPDATA 'NotchLite\whisper'),
    [ValidateSet('tiny','base','small','turbo')][string]$Model = 'small',
    [ValidateSet('cpu','vulkan')][string]$Backend = 'cpu',
    [ValidateRange(0,31)][int]$Device = 0,
    [ValidateRange(1,8)][int]$Threads = 4,
    [ValidatePattern('^(auto|[a-z]{2,3})$')][string]$Language = 'it'
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$Backend = $Backend.ToLowerInvariant()
$Model = $Model.ToLowerInvariant()
$Language = $Language.ToLowerInvariant()
$models = @{
    tiny = @{ file='ggml-tiny-q5_1.bin'; sha='818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7'; revision='c521a4b02f422512d734391fdf08bb08c0862f68' }
    base = @{ file='ggml-base-q5_1.bin'; sha='422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898'; revision='c521a4b02f422512d734391fdf08bb08c0862f68' }
    small = @{ file='ggml-small-q5_1.bin'; sha='ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb'; revision='5359861c739e955e79d9a303bcbc70fb988958b1' }
    turbo = @{ file='ggml-large-v3-turbo-q5_0.bin'; sha='394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2'; revision='5359861c739e955e79d9a303bcbc70fb988958b1' }
}
$modelRevision = $models[$Model].revision
$modelFile = $models[$Model].file
$modelSha = $models[$Model].sha
if ($Backend -eq 'cpu') {
    $version = 'v1.9.2'
    $archiveSha = '49dcc16de826f20bd53d44f947a1ae49dfa81f86cad67a64d80820cb192d674a'
    $archiveName = 'whisper-bin-x64-v1.9.2.zip'
    $archiveUrl = 'https://github.com/ggml-org/whisper.cpp/releases/download/v1.9.2/whisper-bin-x64.zip'
    $cliSha = '95e3c0b0e778ad9499eb0125f97c1dcf437dd9eb4ea77050b043574f93c2631d'
    $runtimeSource = 'ggml-org/whisper.cpp official CPU release'
    $upstreamRevision = '306c88f4d1286aec1bf96e544632897886af5501'
    $whitelist = '^Release/(whisper-cli\.exe|whisper\.dll|ggml(?:-base|-cpu-[a-z0-9]+)?\.dll)$'
    $requiredFiles = @('whisper-cli.exe','whisper.dll','ggml.dll','ggml-base.dll','ggml-cpu-x64.dll')
} else {
    $version = 'whispercpp-195'
    $archiveSha = '89f2c36455939ad42e08b2107bd6b47105b6d9c1a2b0916cbb500ec27deb520a'
    $archiveName = 'whisper-vulkan-x64-whispercpp-195.zip'
    $archiveUrl = 'https://github.com/SubtitleEdit/support-files/releases/download/whispercpp-195/whisper-vulkan-x64.zip'
    $cliSha = '1ef8f16edf2f8df429e7e1d18c9af073f66d594813e9aae739e5bdf015a61985'
    $runtimeSource = 'SubtitleEdit/support-files Vulkan build; CLI reports 1.9.5-dev'
    $upstreamRevision = 'd1be6fde11ac6e0407606b4e42fe72d34add8037'
    $whitelist = '^(whisper-cli\.exe|whisper\.dll|ggml(?:-base|-cpu|-vulkan)?\.dll)$'
    $requiredFiles = @('whisper-cli.exe','whisper.dll','ggml.dll','ggml-base.dll','ggml-cpu.dll','ggml-vulkan.dll')
}
$vadRevision = '9ffd54a1e1ee413ddf265af9913beaf518d1639b'
$vadFile = 'ggml-silero-v6.2.0.bin'
$vadSha = '2aa269b785eeb53a82983a20501ddf7c1d9c48e33ab63a41391ac6c9f7fb6987'
$modelUrl = 'https://huggingface.co/ggerganov/whisper.cpp/resolve/' + $modelRevision + '/' + $modelFile
$vadUrl = 'https://huggingface.co/ggml-org/whisper-vad/resolve/' + $vadRevision + '/' + $vadFile
if (-not [Environment]::Is64BitOperatingSystem -or $env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { throw 'This pinned runtime requires Windows x64.' }
if ($DestinationRoot -notmatch '^[A-Za-z]:[\\/]') { throw 'Choose an absolute local directory.' }
$root = [IO.Path]::GetFullPath($DestinationRoot)
if ([IO.DriveInfo]::new([IO.Path]::GetPathRoot($root)).DriveType -eq [IO.DriveType]::Network) { throw 'Choose a local disk.' }
function Assert-NoReparse([string]$Path) {
    $candidate = $Path
    $parents = [Collections.Generic.List[string]]::new()
    while ($candidate) {
        $parents.Insert(0,$candidate)
        $candidate = Split-Path -Parent $candidate
    }
    foreach ($candidate in $parents) {
        if (Test-Path -LiteralPath $candidate) {
            if ((Get-Item -LiteralPath $candidate -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Runtime paths cannot contain reparse points.' }
        }
    }
}
Assert-NoReparse $root
New-Item -ItemType Directory -Force -Path $root | Out-Null
$archive = Join-Path $root $archiveName
$modelPath = Join-Path $root $modelFile
$vad = Join-Path $root $vadFile
function Get-Verified([string]$Url, [string]$Path, [string]$Sha) {
    Assert-NoReparse $Path
    if (-not (Test-Path -LiteralPath $Path)) {
        $partial = $Path + '.' + [Guid]::NewGuid().ToString('N') + '.download'
        try {
            Invoke-WebRequest -Uri $Url -OutFile $partial
            if ((Get-FileHash -LiteralPath $partial -Algorithm SHA256).Hash.ToLowerInvariant() -ne $Sha) { throw 'Pinned download SHA-256 mismatch.' }
            Move-Item -LiteralPath $partial -Destination $Path
        } finally { if (Test-Path -LiteralPath $partial) { Remove-Item -LiteralPath $partial } }
    }
    if ((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $Sha) { throw 'Existing download SHA-256 mismatch; nothing is executed.' }
}
Get-Verified $archiveUrl $archive $archiveSha
Get-Verified $modelUrl $modelPath $modelSha
Get-Verified $vadUrl $vad $vadSha
Add-Type -AssemblyName System.IO.Compression.FileSystem
$packageName = $Backend + '-' + $version + '-' + [Guid]::NewGuid().ToString('N')
$packageRoot = Join-Path $root $packageName
New-Item -ItemType Directory -Path $packageRoot | Out-Null
$zip = [IO.Compression.ZipFile]::OpenRead($archive)
try {
    if ($zip.Entries.Count -gt 200 -or $zip.Entries.Count -eq 0) { throw 'Unexpected archive entry count.' }
    $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($entry in $zip.Entries) {
        $name = $entry.FullName.Replace('\','/')
        if ($name.StartsWith('/') -or $name.Contains(':') -or ($name.Split('/') -contains '..') -or ((($entry.ExternalAttributes -shr 16) -band 0xF000) -eq 0xA000) -or (-not $seen.Add($name))) { throw 'Unsafe or duplicate archive entry.' }
        # Separate whitelists exclude servers, streaming/GUI examples and bundled VAD.
        # Never mix DLLs from CPU and Vulkan archives or previous runtime directories.
        if ($name -match $whitelist) {
            if ($entry.Length -gt 100MB) { throw 'Unexpected runtime entry size.' }
            [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, (Join-Path $packageRoot $entry.Name), $false)
        }
    }
} finally { $zip.Dispose() }
foreach ($required in $requiredFiles) {
    if (-not (Test-Path -LiteralPath (Join-Path $packageRoot $required) -PathType Leaf)) { throw 'Incomplete selected runtime.' }
}
$cli = Join-Path $packageRoot 'whisper-cli.exe'
if ((Get-FileHash -LiteralPath $cli -Algorithm SHA256).Hash.ToLowerInvariant() -ne $cliSha) { throw 'Unexpected CLI binary.' }
# Retain upstream MIT notices beside the optional dependency. No dependency is linked into NotchLite.
$mitPermission = @'
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
$notices = @{
    'LICENSE-whisper.cpp.txt' = 'Copyright (c) 2023-2026 The ggml authors'
    'LICENSE-Whisper-model.txt' = 'Copyright (c) 2022 OpenAI'
    'LICENSE-Silero-VAD.txt' = 'Copyright (c) 2020-present Silero Team'
}
foreach ($notice in $notices.GetEnumerator()) {
    [IO.File]::WriteAllText((Join-Path $packageRoot $notice.Key), ("MIT License`n`n" + $notice.Value + "`n`n" + $mitPermission))
}
$manifest = [ordered]@{ version=$version; executable=($packageName + '/whisper-cli.exe'); model=$modelFile; vadModel=$vadFile; language=$Language; backend=$Backend; device=$Device; threads=$Threads; runtimeSource=$runtimeSource; upstreamReleaseRevision=$upstreamRevision; cliSha256=$cliSha; archiveSha256=$archiveSha; modelSha256=$modelSha; modelRevision=$modelRevision; archiveUrl=$archiveUrl; modelUrl=$modelUrl; vadModelSha256=$vadSha; vadModelRevision=$vadRevision; vadModelUrl=$vadUrl }
if ($Backend -eq 'vulkan') { $manifest['buildWorkflowRevision']='358047ebca4410be9636974795e68e01427b6f53' }
$manifestTemp = Join-Path $root ('runtime-' + [Guid]::NewGuid().ToString('N') + '.json')
[IO.File]::WriteAllText($manifestTemp, ($manifest | ConvertTo-Json))
Assert-NoReparse (Join-Path $root 'runtime.json')
Move-Item -LiteralPath $manifestTemp -Destination (Join-Path $root 'runtime.json') -Force
Write-Output ('Local transcription prepared: ' + $Model + ', backend ' + $Backend + ', threads ' + $Threads + '. SHA-256 verified.')
Write-Output 'No capture, model inference, server, autorun, global PATH, SDK or driver change. GPU selection is explicit; actual offload requires a separate runtime check.'
Write-Output ('Runtime configuration: ' + (Join-Path $root 'runtime.json'))
