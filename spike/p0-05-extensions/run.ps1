# Run the P0-05 spike with switches, wait, screenshot its window (PrintWindow), stop it.
# -Ext loads the unpacked extensions from $env:P005_EXT (a folder holding unpacked-* dirs).
param([string]$AppArgs = "", [string]$Shot = "", [int]$Wait = 12, [switch]$Ext, [switch]$Fresh, [switch]$KeepRunning)
Set-Location $PSScriptRoot
Get-Process p0-05-extensions -ErrorAction SilentlyContinue | Stop-Process -Force; Start-Sleep -Milliseconds 800
if ($Fresh) { Remove-Item -Recurse -Force (Join-Path $env:LOCALAPPDATA 'constellations-spike\p0-05') -ErrorAction SilentlyContinue }
$target = Resolve-Path ..\p0-04-helm\target   # shared build folder (same cef crate)
$cef = (Get-ChildItem "$target\debug\build\cef-dll-sys" -Recurse -Filter libcef.dll | Select-Object -First 1).DirectoryName
$env:PATH = "$cef;$env:PATH"
$list = @($AppArgs -split ' ' | Where-Object { $_ })
if ($Ext) { $list += '--load-extension=' + ((Get-ChildItem $env:P005_EXT -Directory -Filter 'unpacked-*' | ForEach-Object FullName) -join ',') }
$err = Join-Path $env:TEMP 'p005-err.txt'
$p = Start-Process -FilePath "$target\debug\p0-05-extensions.exe" -ArgumentList $list -PassThru -RedirectStandardError $err
Start-Sleep $Wait
if ($Shot) { powershell -NoProfile -ExecutionPolicy Bypass -File .\shot-main.ps1 -OnlyMain -Out (Join-Path '..\shots' $Shot) | Select-Object -Last 1 }
Get-Content $err | Where-Object { $_ -notmatch 'external_registry' } | Select-Object -Last 8
if (-not $KeepRunning) { Get-Process p0-05-extensions -ErrorAction SilentlyContinue | Stop-Process -Force }
