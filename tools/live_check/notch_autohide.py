import os, sys, ctypes, ctypes.wintypes as wt
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *
u32 = ctypes.windll.user32; g32 = ctypes.windll.gdi32; u32.SetProcessDPIAware()

def covered(x_dip, y_dip, dpi):
    """True if a page window covers this main-window DIP point (no hole there)."""
    pids = set(int(l.split(',')[1].strip('"')) for l in subprocess.run(['tasklist', '/FI', 'IMAGENAME eq constellations.exe', '/FO', 'CSV', '/NH'], capture_output=True, text=True).stdout.splitlines() if l.strip())
    res = []
    @ctypes.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)
    def cb(h, l):
        pid = wt.DWORD(); u32.GetWindowThreadProcessId(h, ctypes.byref(pid))
        if pid.value in pids and u32.IsWindowVisible(h):
            rgn = g32.CreateRectRgn(0, 0, 0, 0)
            if u32.GetWindowRgn(h, rgn) >= 2: res.append(bool(g32.PtInRegion(rgn, int((x_dip - 56) * dpi), int(y_dip * dpi))))
        return True
    u32.EnumWindows(cb, 0)
    return res[0] if res else None

def mouse(x, y):
    s = cdp.ws_connect(ui_ws()); cdp.send(s, {"id": 1, "method": "Input.dispatchMouseEvent", "params": {"type": "mouseMoved", "x": x, "y": y}})
    while cdp.recv(s).get('id') != 1: pass
    s.close()

prof = tempfile.mkdtemp(prefix='cons-prof-')
open(os.path.join(prof, 'urls.txt'), 'w').write(f'http://127.0.0.1:{SRV}/a.html\n')
p = launch([f'--urls-file={os.path.join(prof, "urls.txt")}'], prof)
state = lambda: ui("document.getElementById('app').dataset.notch")
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); wait(lambda: ui("document.getElementById('app').dataset.notch") == 'shown', 20, 0.1)
    dpi = ui('window.devicePixelRatio'); iw = ui('window.innerWidth')
    cx, cy = iw - 110, 17
    check('notch is shown for a moment when the page appears', state() == 'shown', state())
    check('...its hole is in the page window', covered(cx, cy, dpi) is False)
    time.sleep(4.5)
    check('notch slides away after a few seconds over a web page', state() == 'hidden', state())
    check('...the page window now covers the notch area (page controls usable)', covered(cx, cy, dpi) is True)
    check('...and there is no strip over the page: the corner is covered too', covered(iw - 30, 2, dpi) is True)
    real_cursor(-30, 3)
    check('pointer at the top-right strip brings the notch back', state() == 'shown', state())
    check('...and its hole returns', covered(cx, cy, dpi) is False)
    real_cursor(500, 300); time.sleep(1.6)
    check('moving away hides it again', state() == 'hidden', state())
    bridge({'type': 'open', 'url': ''}); time.sleep(1.5)
    check('on the new-tab page the notch stays visible', state() == 'shown' and ui("!document.querySelector('.notch-grip')") is True, state())
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
