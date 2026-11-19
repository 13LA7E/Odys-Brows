# Composite screenshot of every visible top-level window of the spike's processes, each captured
# with PrintWindow (never the screen, so nothing else on the desktop is captured).
param([string]$Name = 'p0-04-helm', [string]$Out)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Collections.Generic; using System.Runtime.InteropServices;
public class SA {
  public delegate bool CB(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(CB cb, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr h, uint c);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint f);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern int GetWindowRgn(IntPtr h, IntPtr rgn);
  [DllImport("gdi32.dll")] public static extern IntPtr CreateRectRgn(int a, int b, int c, int d);
  public struct R { public int L, T, Rt, B; }
  public static List<IntPtr> Find(uint[] pids) {
    var list = new List<IntPtr>();
    EnumWindows((h, l) => { uint id; GetWindowThreadProcessId(h, out id);
      if (Array.IndexOf(pids, id) >= 0 && IsWindowVisible(h)) list.Add(h); return true; }, IntPtr.Zero);
    return list; }
}
"@
[SA]::SetProcessDPIAware() | Out-Null
$pids = [uint32[]]@(Get-Process $Name -ErrorAction SilentlyContinue | ForEach-Object { $_.Id })
$wins = [SA]::Find($pids) | ForEach-Object { $r = New-Object SA+R; [SA]::GetWindowRect($_, [ref]$r) | Out-Null
  [pscustomobject]@{ H = $_; L = $r.L; T = $r.T; W = $r.Rt - $r.L; Ht = $r.B - $r.T; Owner = [SA]::GetWindow($_, 4) } } |
  Where-Object { $_.W -gt 40 -and $_.Ht -gt 20 }
$wins | ForEach-Object { "window $($_.H) at $($_.L),$($_.T) $($_.W)x$($_.Ht) owner=$($_.Owner)" }
$main = $wins | Sort-Object { $_.W * $_.Ht } -Descending | Select-Object -First 1
if (-not $main) { Write-Error 'no window'; exit 1 }
$canvas = New-Object System.Drawing.Bitmap $main.W, $main.Ht
$g = [System.Drawing.Graphics]::FromImage($canvas)
$skip = @()
if ($env:HELM_DUMP -and (Test-Path $env:HELM_DUMP)) { $raw0 = [IO.File]::ReadAllBytes($env:HELM_DUMP); $skip = @($wins | Where-Object { $_.W -eq [BitConverter]::ToInt32($raw0, 0) -and $_.Ht -eq [BitConverter]::ToInt32($raw0, 4) } | ForEach-Object { $_.H }) }
foreach ($w in @($main) + @($wins | Where-Object { $_.H -ne $main.H -and $skip -notcontains $_.H })) {
  $b = New-Object System.Drawing.Bitmap $w.W, $w.Ht; $bg = [System.Drawing.Graphics]::FromImage($b)
  $dc = $bg.GetHdc(); [SA]::PrintWindow($w.H, $dc, 2) | Out-Null; $bg.ReleaseHdc($dc); $bg.Dispose()
  # Apply the window's own region (PrintWindow ignores it), so the composite shows what the user sees.
  $hr = [SA]::CreateRectRgn(0, 0, 0, 0); $kind = [SA]::GetWindowRgn($w.H, $hr)
  "  region type for $($w.H): $kind (0 none/error, 1 null, 2 simple, 3 complex)"
  if ($kind -ge 2) { $rg = [System.Drawing.Region]::FromHrgn($hr); $rg.Translate($w.L - $main.L, $w.T - $main.T); $g.Clip = $rg } else { $g.ResetClip() }
  $g.DrawImage($b, $w.L - $main.L, $w.T - $main.T); $g.ResetClip(); $b.Dispose()
}
# Off-screen Helm (P0-04b): draw its last frame (premultiplied BGRA dumped by the app) at its window position.
if ($env:HELM_DUMP -and (Test-Path $env:HELM_DUMP)) {
  $raw = [IO.File]::ReadAllBytes($env:HELM_DUMP); $hw = [BitConverter]::ToInt32($raw, 0); $hh = [BitConverter]::ToInt32($raw, 4)
  $layer = $wins | Where-Object { $_.W -eq $hw -and $_.Ht -eq $hh } | Select-Object -First 1
  if ($layer) {
    $bm = New-Object System.Drawing.Bitmap $hw, $hh, ([System.Drawing.Imaging.PixelFormat]::Format32bppPArgb)
    $ld = $bm.LockBits((New-Object System.Drawing.Rectangle 0, 0, $hw, $hh), 'WriteOnly', $bm.PixelFormat)
    [Runtime.InteropServices.Marshal]::Copy($raw, 8, $ld.Scan0, $hw * $hh * 4); $bm.UnlockBits($ld)
    $g.DrawImage($bm, $layer.L - $main.L, $layer.T - $main.T); "  drew off-screen Helm frame ${hw}x${hh}"
  }
}
$canvas.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png); "saved $Out"
