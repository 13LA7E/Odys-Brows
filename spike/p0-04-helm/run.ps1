# Spike helper: stop old copies, build, run with arguments, screenshot, print the log, stop.
param([string]$AppArgs = "", [string]$Shot = "", [int]$Seconds = 14, [switch]$KeepRunning)
$ErrorActionPreference = 'Continue'
Set-Location $PSScriptRoot
Get-Process p0-04-helm -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 800
cargo build 2>&1 | Select-String -Pattern '^(error|warning)|-->' | ForEach-Object { $_.Line }
$cef = (Get-ChildItem target\debug\build\cef-dll-sys -Recurse -Filter libcef.dll | Select-Object -First 1).DirectoryName
$env:PATH = "$cef;$env:PATH"
$out = Join-Path $env:TEMP 'p004-out.txt'; $err = Join-Path $env:TEMP 'p004-err.txt'
$p = Start-Process -FilePath (Resolve-Path .\target\debug\p0-04-helm.exe) -ArgumentList ($AppArgs -split " " | Where-Object { $_ }) -PassThru -RedirectStandardOutput $out -RedirectStandardError $err
if ($Shot) {
  powershell -NoProfile -ExecutionPolicy Bypass -File .\shot.ps1 -ProcessId $p.Id -Out (Join-Path '..\shots' $Shot) -WaitSeconds 15
  Start-Sleep -Seconds ([Math]::Max(0, $Seconds - 8))
} else { Start-Sleep -Seconds $Seconds }
Get-Content $out | Where-Object { $_ -notmatch '^layout ' -and $_ -notmatch 'runtime_style asked' }
Get-Content $err | Where-Object { $_ -notmatch 'external_registry' } | Select-Object -First 6
if (-not $KeepRunning) { Get-Process p0-04-helm -ErrorAction SilentlyContinue | Stop-Process -Force }
