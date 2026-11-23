[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateRange(1, 2147483647)]
    [int]$TargetProcessId,
    [ValidateRange(1, 3600)]
    [int]$Seconds = 60,
    [ValidateRange(0, 300)]
    [int]$WarmupSeconds = 15,
    [ValidatePattern('^[a-zA-Z0-9_-]+$')]
    [string]$Label = 'idle',
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '..\measurements')
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$process = [System.Diagnostics.Process]::GetProcessById($TargetProcessId)
$processName = $process.ProcessName
$processStart = $process.StartTime.ToUniversalTime().ToString('o')
$logicalProcessors = [Environment]::ProcessorCount
if ($WarmupSeconds -gt 0) { Start-Sleep -Seconds $WarmupSeconds }
$process.Refresh()
if ($process.HasExited) { throw 'Il processo e terminato durante la stabilizzazione.' }
$previousCpu = $process.TotalProcessorTime.TotalMilliseconds
$watch = [Diagnostics.Stopwatch]::StartNew()
$previousElapsed = $watch.Elapsed.TotalMilliseconds
$rows = [System.Collections.Generic.List[object]]::new()
for ($index = 0; $index -lt $Seconds; $index++) {
    Start-Sleep -Seconds 1
    $process.Refresh()
    if ($process.HasExited) { throw 'Il processo e terminato: campionamento incompleto.' }
    $elapsed = $watch.Elapsed.TotalMilliseconds
    $cpu = $process.TotalProcessorTime.TotalMilliseconds
    $singleCorePercent = 100.0 * ($cpu - $previousCpu) / ($elapsed - $previousElapsed)
    $rows.Add([pscustomobject]@{
        utc = [DateTime]::UtcNow.ToString('o')
        elapsed_seconds = [Math]::Round($elapsed / 1000.0, 3)
        interval_ms = $elapsed - $previousElapsed
        cpu_ms = $cpu - $previousCpu
        cpu_percent_total = [Math]::Round($singleCorePercent / $logicalProcessors, 4)
        cpu_percent_one_core = [Math]::Round($singleCorePercent, 4)
        private_mib = [Math]::Round($process.PrivateMemorySize64 / 1MB, 3)
        working_set_mib = [Math]::Round($process.WorkingSet64 / 1MB, 3)
        threads = $process.Threads.Count
        handles = $process.HandleCount
    })
    $previousElapsed = $elapsed
    $previousCpu = $cpu
}
$watch.Stop()
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$stem = '{0}-{1}' -f $Label, [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss-fff')
$csvPath = Join-Path $OutputDirectory "$stem.csv"
$jsonPath = Join-Path $OutputDirectory "$stem.json"
$rows | Export-Csv -LiteralPath $csvPath -NoTypeInformation -Encoding UTF8
$totalCpu = ($rows | Measure-Object cpu_ms -Sum).Sum
$totalElapsed = ($rows | Measure-Object interval_ms -Sum).Sum
$summary = [ordered]@{
    label = $Label
    process_name = $processName
    process_id = $TargetProcessId
    process_start_utc = $processStart
    os = [Environment]::OSVersion.VersionString
    logical_processors = $logicalProcessors
    samples = $rows.Count
    elapsed_seconds = [Math]::Round($totalElapsed / 1000.0, 3)
    cpu_percent_total = [Math]::Round(100.0 * $totalCpu / $totalElapsed / $logicalProcessors, 4)
    cpu_percent_one_core = [Math]::Round(100.0 * $totalCpu / $totalElapsed, 4)
    private_mib_mean = ($rows | Measure-Object private_mib -Average).Average
    private_mib_sampled_max = ($rows | Measure-Object private_mib -Maximum).Maximum
    working_set_mib_mean = ($rows | Measure-Object working_set_mib -Average).Average
    working_set_mib_sampled_max = ($rows | Measure-Object working_set_mib -Maximum).Maximum
    threads_sampled_max = ($rows | Measure-Object threads -Maximum).Maximum
    handles_sampled_max = ($rows | Measure-Object handles -Maximum).Maximum
    excludes = @('GPU memory', 'other processes', 'visual latency')
    csv_file = [IO.Path]::GetFileName($csvPath)
}
$summary | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $jsonPath -Encoding UTF8
$process.Dispose()
Write-Output $jsonPath
