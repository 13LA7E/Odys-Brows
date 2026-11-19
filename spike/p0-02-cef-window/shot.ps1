# Screenshot the top-level window of a process (by id) into a PNG. Spike helper.
param([int]$ProcessId, [string]$Out, [int]$WaitSeconds = 12)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public class W {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
  public struct RECT { public int L, T, R, B; }
}
"@
[W]::SetProcessDPIAware() | Out-Null
$deadline = (Get-Date).AddSeconds($WaitSeconds)
do { Start-Sleep -Milliseconds 500; $p = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue; $p.Refresh() } while ($p -and $p.MainWindowHandle -eq 0 -and (Get-Date) -lt $deadline)
if (-not $p -or $p.MainWindowHandle -eq 0) { Write-Error "no window"; exit 1 }
Start-Sleep -Seconds 6   # let the page paint
$h = $p.MainWindowHandle; [W]::SetForegroundWindow($h) | Out-Null; Start-Sleep -Milliseconds 700
$r = New-Object W+RECT; [W]::GetWindowRect($h, [ref]$r) | Out-Null
$w = $r.R - $r.L; $hh = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $hh
# PrintWindow captures only this window, never whatever else is on screen (privacy).
$g = [System.Drawing.Graphics]::FromImage($bmp); $dc = $g.GetHdc(); $ok = [W]::PrintWindow($h, $dc, 2); $g.ReleaseHdc($dc)
if (-not $ok) { Write-Error "PrintWindow failed"; exit 1 }
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png); "saved $Out ${w}x${hh} title='$($p.MainWindowTitle)'"
