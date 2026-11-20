# Launch an exe with args, make its main window topmost briefly, copy ONLY that window's rectangle
# from the screen (what the user sees), save PNG, restore, close.
param([string]$Exe, [string]$Out, [string]$ArgList = '', [int]$Wait = 12, [string]$ProfileDir = '', [string]$After = '')
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public class TS {
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint f);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern IntPtr GetDC(IntPtr h);
  [DllImport("user32.dll")] public static extern int ReleaseDC(IntPtr h, IntPtr dc);
  [DllImport("gdi32.dll")] public static extern bool BitBlt(IntPtr d, int x, int y, int w, int h, IntPtr s, int sx, int sy, uint rop);
  public struct R { public int L, T, Rt, B; }
}
"@
[TS]::SetProcessDPIAware() | Out-Null
if ($ProfileDir) { $env:LOCALAPPDATA = $ProfileDir }
$p = Start-Process $Exe -ArgumentList $ArgList -PassThru
Start-Sleep $Wait
$main = (Get-Process constellations | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1).MainWindowHandle
$HWND_TOPMOST = [IntPtr](-1); $HWND_NOTOPMOST = [IntPtr](-2)
[TS]::SetWindowPos($main, $HWND_TOPMOST, 0, 0, 0, 0, 0x0003) | Out-Null   # NOMOVE|NOSIZE
Start-Sleep 2
$r = New-Object TS+R; [TS]::GetWindowRect($main, [ref]$r) | Out-Null
$w = $r.Rt - $r.L; $h = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdcDst = $g.GetHdc(); $hdcSrc = [TS]::GetDC([IntPtr]::Zero)
[TS]::BitBlt($hdcDst, 0, 0, $w, $h, $hdcSrc, $r.L, $r.T, 0x40CC0020) | Out-Null   # SRCCOPY | CAPTUREBLT (includes layered windows)
[TS]::ReleaseDC([IntPtr]::Zero, $hdcSrc) | Out-Null; $g.ReleaseHdc($hdcDst)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
[TS]::SetWindowPos($main, $HWND_NOTOPMOST, 0, 0, 0, 0, 0x0003) | Out-Null
"captured ${w}x${h} -> $Out"
if ($After -ne 'keep') { Get-Process constellations -ErrorAction SilentlyContinue | Stop-Process -Force }
