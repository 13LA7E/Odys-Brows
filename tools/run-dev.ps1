# Run the development build of the browser with CEF's runtime on PATH.
#   powershell -ExecutionPolicy Bypass -File tools\run-dev.ps1 [-Release] [-- extra args]
param([switch]$Release, [Parameter(ValueFromRemainingArguments = $true)][string[]]$Rest)
$root = Split-Path -Parent $PSScriptRoot
$profileName = if ($Release) { 'release' } else { 'debug' }
$lib = Get-ChildItem (Join-Path $root "target\$profileName\build") -Recurse -Filter libcef.dll -ErrorAction SilentlyContinue |
  Where-Object { $_.DirectoryName -match 'cef_windows' } | Select-Object -First 1
if (-not $lib) { Write-Error "CEF runtime not found. Run: cargo build$(if ($Release) { ' --release' })"; exit 1 }
$env:PATH = "$($lib.DirectoryName);$env:PATH"
& (Join-Path $root "target\$profileName\constellations.exe") @Rest
