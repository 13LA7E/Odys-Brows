# Remove Constellations for the current user. Your browsing data in %LOCALAPPDATA%\ConstellationsBrowser is kept.
$installDir = Join-Path $env:LOCALAPPDATA 'Programs\Constellations'
Get-Process constellations -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "$installDir*" } | Stop-Process -Force
Start-Sleep -Milliseconds 500
Remove-Item (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Constellations.lnk') -ErrorAction SilentlyContinue
Remove-Item (Join-Path ([Environment]::GetFolderPath('Desktop')) 'Constellations.lnk') -ErrorAction SilentlyContinue
Remove-Item 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Constellations' -Recurse -ErrorAction SilentlyContinue
# This script lives inside the install folder, so remove the folder from a detached process.
Start-Process cmd -ArgumentList '/c', "ping 127.0.0.1 -n 3 >nul & rmdir /s /q `"$installDir`"" -WindowStyle Hidden
Write-Host 'Constellations removed.'
