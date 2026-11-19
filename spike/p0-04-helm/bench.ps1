# P0-04 benchmark: for each mode, scroll pacing (page rAF probe), Helm input latency (synthetic
# clicks and keys through SendInput / SendKeys), and for window mode whether the Helm follows the
# main window when it moves and hides when it is minimised. Moves the real mouse while it runs.
param([string[]]$Modes = @('none', 'overlay', 'window'))
Set-Location $PSScriptRoot
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System; using System.Collections.Generic; using System.Runtime.InteropServices;
public class B {
  public delegate bool CB(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(CB cb, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref P p);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int cx, int cy, uint f);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  public struct R { public int L, T, Rt, B; }
  public struct P { public int X, Y; }
  public static List<IntPtr> Find(uint[] pids) { var l = new List<IntPtr>();
    EnumWindows((h, x) => { uint id; GetWindowThreadProcessId(h, out id);
      if (Array.IndexOf(pids, id) >= 0 && IsWindowVisible(h)) { R r; GetWindowRect(h, out r); if (r.Rt - r.L > 40) l.Add(h); } return true; }, IntPtr.Zero);
    return l; }
}
"@
[B]::SetProcessDPIAware() | Out-Null
function Rect($h) { $r = New-Object B+R; [B]::GetWindowRect($h, [ref]$r) | Out-Null; $r }
function Click($x, $y) { [B]::SetCursorPos($x, $y) | Out-Null; [B]::mouse_event(2, 0, 0, 0, [IntPtr]::Zero); [B]::mouse_event(4, 0, 0, 0, [IntPtr]::Zero) }

$cef = (Get-ChildItem target\debug\build\cef-dll-sys -Recurse -Filter libcef.dll | Select-Object -First 1).DirectoryName
$env:PATH = "$cef;$env:PATH"
$exe = Resolve-Path .\target\debug\p0-04-helm.exe
foreach ($m in $Modes) {
  Get-Process p0-04-helm -ErrorAction SilentlyContinue | Stop-Process -Force; Start-Sleep 1
  $out = Join-Path $env:TEMP "p004-bench-$m.txt"
  $p = Start-Process -FilePath $exe -ArgumentList "--mode=$m", '--probe' -PassThru -RedirectStandardOutput $out -RedirectStandardError (Join-Path $env:TEMP "p004-bench-$m-err.txt")
  Start-Sleep 4
  $pids = [uint32[]]@(Get-Process p0-04-helm | ForEach-Object { $_.Id })
  $wins = @([B]::Find($pids)); $main = $wins | Sort-Object { $r = Rect $_; ($r.Rt - $r.L) * ($r.B - $r.T) } -Descending | Select-Object -First 1
  [B]::SetForegroundWindow($main) | Out-Null
  "== $m"
  Start-Sleep 14   # page load + scroll probe (starts 2.5 s after load, ~2 s long)
  if ($m -ne 'none') {
    $s = [B]::GetDpiForWindow($main) / 96.0
    $c = New-Object B+R; [B]::GetClientRect($main, [ref]$c) | Out-Null; $o = New-Object B+P; [B]::ClientToScreen($main, [ref]$o) | Out-Null
    $w = $c.Rt / $s; $h = $c.B / $s
    # Address field: Helm left + 200 DIP, Helm vertical centre.
    $x = [int]($o.X + (56 + ($w - 56 - 560) / 2 + 200) * $s); $y = [int]($o.Y + ($h - 28 - 26) * $s)
    "click target $x,$y (client $($c.Rt)x$($c.B) px at $($o.X),$($o.Y), scale $s)"
    for ($i = 0; $i -lt 10; $i++) { Click $x $y; Start-Sleep -Milliseconds 200 }
    [System.Windows.Forms.SendKeys]::SendWait('abcdefghij'); Start-Sleep -Milliseconds 300
    [System.Windows.Forms.SendKeys]::SendWait('{ESC}'); Start-Sleep -Milliseconds 300
    # Click-through: 20 DIP above the pill (inside the off-screen Helm's shadow margin) must reach the page.
    Click $x ([int]($y - (26 + 20) * $s)); Start-Sleep -Milliseconds 400
  }
  if ($m -eq 'window' -or $m -eq 'osr') {
    $helm = $wins | Where-Object { $_ -ne $main } | Select-Object -First 1
    $before = Rect $helm; $mr = Rect $main
    [B]::SetWindowPos($main, [IntPtr]::Zero, $mr.L + 120, $mr.T + 40, 0, 0, 0x0015) | Out-Null; Start-Sleep 1   # NOSIZE|NOZORDER|NOACTIVATE
    $after = Rect $helm
    "follow: main moved +120,+40 -> helm moved $($after.L - $before.L),$($after.T - $before.T)"
    [B]::ShowWindow($main, 6) | Out-Null; Start-Sleep 1; "minimised main -> helm visible=$([B]::IsWindowVisible($helm))"
    [B]::ShowWindow($main, 9) | Out-Null; Start-Sleep 1; "restored main -> helm visible=$([B]::IsWindowVisible($helm))"
  }
  [B]::PostMessage($main, 0x10, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null; Start-Sleep 3   # WM_CLOSE, so output is flushed
  Get-Process p0-04-helm -ErrorAction SilentlyContinue | Stop-Process -Force; Start-Sleep 1
  $lines = Get-Content $out
  $lines | Where-Object { $_ -match 'METRIC scroll' }
  $pp = @($lines | Where-Object { $_ -match 'METRIC pagepointer' }); "page clicks received: $($pp.Count) $($pp -join ' | ')"
  $op = @($lines | Where-Object { $_ -match 'METRIC osrpaint' } | ForEach-Object { [double]($_ -split ' ')[-1] })
  if ($op.Count) { $st = $op | Measure-Object -Average -Maximum; "osr input-to-paint n=$($op.Count) avg_ms=$([Math]::Round($st.Average,2)) max_ms=$([Math]::Round($st.Maximum,2))" }
  foreach ($t in 'pointerdown', 'keydown') {
    $v = @($lines | Where-Object { $_ -match "METRIC input $t " } | ForEach-Object { [double]($_ -split ' ')[-1] })
    if ($v.Count) { $st = $v | Measure-Object -Average -Maximum; "input $t n=$($v.Count) avg_ms=$([Math]::Round($st.Average,2)) max_ms=$([Math]::Round($st.Maximum,2))" }
  }
}
