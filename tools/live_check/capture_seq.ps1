# Capture N frames of the running app's own window (BitBlt with CAPTUREBLT, window made topmost for
# the duration). Used to make the evidence GIFs. Only the app's rectangle is copied.
param([string]$Prefix, [int]$Count = 10, [int]$IntervalMs = 120)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public class CS {
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint f);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern IntPtr GetDC(IntPtr h);
  [DllImport("user32.dll")] public static extern int ReleaseDC(IntPtr h, IntPtr dc);
  [DllImport("gdi32.dll")] public static extern bool BitBlt(IntPtr d, int x, int y, int w, int h, IntPtr s, int sx, int sy, uint rop);
  public struct R { public int L, T, Rt, B; }
}
"@
[CS]::SetProcessDPIAware() | Out-Null
$main = (Get-Process constellations | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1).MainWindowHandle
[CS]::SetWindowPos($main, [IntPtr](-1), 0, 0, 0, 0, 0x0003) | Out-Null
Start-Sleep -Milliseconds 800
$r = New-Object CS+R; [CS]::GetWindowRect($main, [ref]$r) | Out-Null
$w = $r.Rt - $r.L; $h = $r.B - $r.T
for ($i = 0; $i -lt $Count; $i++) {
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $dst = $g.GetHdc(); $src = [CS]::GetDC([IntPtr]::Zero)
    [CS]::BitBlt($dst, 0, 0, $w, $h, $src, $r.L, $r.T, 0x40CC0020) | Out-Null
    [CS]::ReleaseDC([IntPtr]::Zero, $src) | Out-Null; $g.ReleaseHdc($dst); $g.Dispose()
    $bmp.Save(("{0}-{1:D2}.png" -f $Prefix, $i), [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
    Start-Sleep -Milliseconds $IntervalMs
}
[CS]::SetWindowPos($main, [IntPtr](-2), 0, 0, 0, 0, 0x0003) | Out-Null
"captured $Count frames"
