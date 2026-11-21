"""Settings: a switch keeps your place on the page, and the page shows no scroll bar."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *
S = lambda e: ui('JSON.stringify(%s)' % e)
V = lambda e: json.loads(S(e))
prof = tempfile.mkdtemp(prefix='cons-prof-')
p = launch([], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(3)
    import ctypes, ctypes.wintypes as wt
    u32 = ctypes.windll.user32; u32.SetProcessDPIAware()
    pids = set(int(l.split(',')[1].strip('"')) for l in subprocess.run(['tasklist', '/FI', 'IMAGENAME eq constellations.exe', '/FO', 'CSV', '/NH'], capture_output=True, text=True).stdout.splitlines() if l.strip())
    best = []
    @ctypes.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)
    def cb(h, l):
        pid = wt.DWORD(); u32.GetWindowThreadProcessId(h, ctypes.byref(pid))
        if pid.value in pids and u32.IsWindowVisible(h):
            r = wt.RECT(); u32.GetWindowRect(h, ctypes.byref(r)); best.append(((r.right - r.left) * (r.bottom - r.top), h))
        return True
    u32.EnumWindows(cb, 0)
    hw = max(best)[1]; u32.ShowWindow(hw, 9); u32.MoveWindow(hw, 100, 60, 1100, 520, True); time.sleep(1.5)
    bridge({'type': 'open', 'url': 'odyssey://settings'}); time.sleep(1)
    ui("[...document.querySelectorAll('.page-internal *')].filter(e=>e.children.length===0&&e.textContent==='Appearance')[0].click(); 1"); time.sleep(0.6)
    ui("document.querySelector('.settings-content').scrollTop = 300; 1"); time.sleep(0.3)
    before = V("document.querySelector('.settings-content').scrollTop")
    ui("document.querySelector('.settings-content [role=switch], .settings-content .switch, .settings-content button[aria-checked]').click(); 1"); time.sleep(0.8)
    after = V("document.querySelector('.settings-content').scrollTop")
    check('toggling a switch keeps the scroll position', before > 100 and abs(after - before) < 5, (before, after))
    check('the settings page has no scroll bar', V("getComputedStyle(document.querySelector('.settings-content')).scrollbarWidth") == 'none' and V("getComputedStyle(document.querySelector('.page-internal')).overflowY") == 'hidden')
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
