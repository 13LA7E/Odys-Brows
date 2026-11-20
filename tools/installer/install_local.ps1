# Install Constellations for the current user (no admin rights, no NSIS needed).
#   powershell -ExecutionPolicy Bypass -File tools\installer\install_local.ps1 [-SkipBuild]
# Builds the release exe, stages the runtime files, copies them to
# %LOCALAPPDATA%\Programs\Constellations and adds a Start menu entry (so Windows search finds it)
# and an entry in Settings > Apps for uninstalling. Your browsing data is kept on uninstall.
param([switch]$SkipBuild, [switch]$Desktop)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
$installDir = Join-Path $env:LOCALAPPDATA 'Programs\Constellations'
$payload = Join-Path $root 'out\installer\payload'

if (-not $SkipBuild) {
    # build_installer.ps1 builds release and stages the payload; it only skips the NSIS step
    # when makensis is missing.
    & powershell -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'build_installer.ps1')
    if ($LASTEXITCODE -ne 0) { throw 'Build/staging failed.' }
}
if (-not (Test-Path (Join-Path $payload 'constellations.exe'))) { throw "Payload not found at $payload. Run without -SkipBuild." }

# Stop a running copy so files can be replaced.
Get-Process constellations -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "$installDir*" } | Stop-Process -Force
Start-Sleep -Milliseconds 500

New-Item -ItemType Directory -Force $installDir | Out-Null
Copy-Item (Join-Path $payload '*') $installDir -Recurse -Force
Copy-Item (Join-Path $PSScriptRoot 'constellations.ico') $installDir -Force
Copy-Item (Join-Path $PSScriptRoot 'uninstall_local.ps1') (Join-Path $installDir 'uninstall.ps1') -Force

$exe = Join-Path $installDir 'constellations.exe'
$ico = Join-Path $installDir 'constellations.ico'
$wsh = New-Object -ComObject WScript.Shell
function New-Shortcut($path) {
    $lnk = $wsh.CreateShortcut($path)
    $lnk.TargetPath = $exe
    $lnk.WorkingDirectory = $installDir
    $lnk.IconLocation = "$ico,0"
    $lnk.Description = 'Constellations browser'
    $lnk.Save()
}
$startMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
New-Shortcut (Join-Path $startMenu 'Constellations.lnk')
if ($Desktop) { New-Shortcut (Join-Path ([Environment]::GetFolderPath('Desktop')) 'Constellations.lnk') }

# Settings > Apps entry (per user).
$key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Constellations'
New-Item -Path $key -Force | Out-Null
Set-ItemProperty $key DisplayName 'Constellations'
Set-ItemProperty $key DisplayIcon $ico
Set-ItemProperty $key InstallLocation $installDir
Set-ItemProperty $key Publisher 'Constellations Authors'
Set-ItemProperty $key UninstallString "powershell -ExecutionPolicy Bypass -File `"$installDir\uninstall.ps1`""
Set-ItemProperty $key NoModify 1
Set-ItemProperty $key NoRepair 1

Write-Host "Installed to $installDir"
Write-Host "Start menu entry: $startMenu\Constellations.lnk (search for 'Constellations')"
