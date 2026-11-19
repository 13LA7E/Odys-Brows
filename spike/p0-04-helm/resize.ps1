# Resize the running spike's main window and capture it (composite of its own windows only).
param([string]$Mode = 'overlay')
Set-Location $PSScriptRoot
Add-Type @"
using System; using System.Runtime.InteropServices;
public class RZ {
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int cx, int cy, uint f);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
"@
[RZ]::SetProcessDPIAware() | Out-Null
Get-Process p0-04-helm -ErrorAction SilentlyContinue | Stop-Process -Force; Start-Sleep 1
$cef = (Get-ChildItem target\debug\build\cef-dll-sys -Recurse -Filter libcef.dll | Select-Object -First 1).DirectoryName
$env:PATH = "$cef;$env:PATH"
Start-Process -FilePath (Resolve-Path .\target\debug\p0-04-helm.exe) -ArgumentList "--mode=$Mode" | Out-Null
Start-Sleep 9
$main =(Get-Process p0-04-helm | Where-Object MainWindowHandle -ne 0 | Select-Object -First 1).MainWindowHandle
# Ask for 700x500 physical px: the 800x600 DIP minimum must win (1000x750 at 125%).
[RZ]::SetWindowPos($main, [IntPtr]::Zero, 40, 40, 700, 500, 0x0014) | Out-Null; Start-Sleep 2
powershell -NoProfile -ExecutionPolicy Bypass -File .\shot-all.ps1 -Out "..\shots\p0-04-$Mode-min.png"
[RZ]::ShowWindow($main, 3) | Out-Null; Start-Sleep 2   # maximise
powershell -NoProfile -ExecutionPolicy Bypass -File .\shot-all.ps1 -Out "..\shots\p0-04-$Mode-max.png"
Get-Process p0-04-helm -ErrorAction SilentlyContinue | Stop-Process -Force
