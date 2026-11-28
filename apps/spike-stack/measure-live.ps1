# PROTOTYPE (wayfinder ticket #28): measure the stack spike against a live iRacing session.
#
# Run from an ELEVATED PowerShell (PresentMon needs admin for ETW), from the repository root,
# with iRacing in a session in borderless-windowed mode (overlays don't show over exclusive
# fullscreen) and the car on track (an AI race or a replay playing at 1x).
#
#   pwsh -ExecutionPolicy Bypass -File apps/spike-stack/measure-live.ps1 -PresentMon C:\path\PresentMon-2.x-x64.exe
#
# Each configuration runs the spike for 70 s while PresentMon records the sim for 60 s of it.
# Baselines (no spike at all) open and close the run, to show drift. Results land in
# %USERPROFILE%\reo-spike-results\<timestamp>\, outside the repository. They hold frame times and
# the spike's own numbers, no driver names (the spike shows "Driver <number>" unless --names).

param(
    [Parameter(Mandatory)] [string] $PresentMon,
    [string] $Sim = "iRacingSim64DX11.exe",
    [int] $Widgets = 9,
    [int] $Seconds = 60
)
$ErrorActionPreference = "Stop"
$spike = Join-Path $PSScriptRoot "..\..\target\release\spike-stack.exe"
if (-not (Test-Path $spike)) { throw "build it first: cargo build --release -p spike-stack" }
if (-not (Get-Process ($Sim -replace '\.exe$','') -ErrorAction SilentlyContinue)) { throw "$Sim is not running" }
$out = Join-Path $env:USERPROFILE ("reo-spike-results\" + (Get-Date -Format "yyyyMMdd-HHmmss"))
New-Item -ItemType Directory -Force $out | Out-Null

$configs = @(
    @{ name = "baseline-1" },
    @{ name = "native-per-widget";   args = @("--stack", "native",  "--layout", "per-widget") },
    @{ name = "native-per-monitor";  args = @("--stack", "native",  "--layout", "per-monitor") },
    @{ name = "webview-per-widget";  args = @("--stack", "webview", "--layout", "per-widget") },
    @{ name = "webview-per-monitor"; args = @("--stack", "webview", "--layout", "per-monitor") },
    @{ name = "native-per-widget-normal-priority"; args = @("--stack", "native", "--layout", "per-widget", "--normal-priority") },
    @{ name = "baseline-2" }
)

foreach ($c in $configs) {
    Write-Host "== $($c.name)"
    $proc = $null
    if ($c.args) {
        $a = $c.args + @("--widgets", "$Widgets", "--seconds", "$($Seconds + 5)")
        $proc = Start-Process -FilePath $spike -ArgumentList $a -PassThru -NoNewWindow `
            -RedirectStandardOutput (Join-Path $out "$($c.name).txt") -RedirectStandardError (Join-Path $out "$($c.name).err")
        Start-Sleep -Seconds 8   # 5 s spike warm-up, plus WebView2 start-up
    }
    & $PresentMon --process_name $Sim --output_file (Join-Path $out "$($c.name).csv") `
        --timed $Seconds --terminate_after_timed --stop_existing_session --no_console_stats | Out-Null
    if ($proc) { $proc.WaitForExit() }
    Start-Sleep -Seconds 3
}
Write-Host "done: $out"
Write-Host "summarize: node apps/spike-stack/summarize.mjs `"$out`""
