"""An extension that opens its welcome page when installed must not make a window flash on screen:
the page becomes a tab and no other top-level window is ever visible (not even for a moment)."""
import os, sys, ctypes, ctypes.wintypes as wt, threading
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *
u32 = ctypes.windll.user32; u32.SetProcessDPIAware()
S = lambda e: ui('JSON.stringify(%s)' % e)
V = lambda e: json.loads(S(e))

work = tempfile.mkdtemp(prefix='cons-ext-')
os.makedirs(os.path.join(work, 'welcome'))
open(os.path.join(work, 'welcome', 'manifest.json'), 'w').write(json.dumps({'manifest_version': 3, 'name': 'Welcomer', 'version': '1.0', 'background': {'service_worker': 'bg.js'}}))
open(os.path.join(work, 'welcome', 'bg.js'), 'w').write("chrome.runtime.onInstalled.addListener(() => chrome.tabs.create({url: 'http://127.0.0.1:%d/b.html?welcome=1'}));" % SRV)
prof = tempfile.mkdtemp(prefix='cons-prof-')
open(os.path.join(prof, 'urls.txt'), 'w').write('http://127.0.0.1:%d/a.html\n' % SRV)
p = launch(['--urls-file=' + os.path.join(prof, 'urls.txt')], prof)
seen = {}
stop = False


def pids():
    return set(int(l.split(',')[1].strip('"')) for l in subprocess.run(['tasklist', '/FI', 'IMAGENAME eq constellations.exe', '/FO', 'CSV', '/NH'], capture_output=True, text=True).stdout.splitlines() if l.strip())


def poll(pidset):
    @ctypes.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)
    def cb(h, l):
        pid = wt.DWORD(); u32.GetWindowThreadProcessId(h, ctypes.byref(pid))
        if pid.value in pidset and u32.IsWindowVisible(h):
            r = wt.RECT(); u32.GetWindowRect(h, ctypes.byref(r))
            key = wt.COLORREF(); alpha = ctypes.c_ubyte(255); fl = wt.DWORD()
            layered = u32.GetLayeredWindowAttributes(h, ctypes.byref(key), ctypes.byref(alpha), ctypes.byref(fl))
            # something a person could see: on screen and not fully transparent
            if r.left > -10000 and not (layered and alpha.value == 0):
                seen[h] = (r.right - r.left, r.bottom - r.top)
        return True
    while not stop:
        u32.EnumWindows(cb, 0)
        time.sleep(0.002)


try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(3)
    t = threading.Thread(target=poll, args=(pids(),), daemon=True); t.start()
    time.sleep(0.5)
    n0 = V('window.__uiState.tabs.length')
    bridge({'type': 'load-unpacked-extension', 'path': os.path.join(work, 'welcome')}); time.sleep(5)
    stop = True; t.join(2)
    urls = V('window.__uiState.tabs.map(t=>t.url||"")')
    check('the welcome page opened as a tab', any('welcome=1' in u for u in urls), urls)
    gone = {h: sz for h, sz in seen.items() if not u32.IsWindow(h)}
    print('windows seen visible that no longer exist:', gone)
    check('no other window was ever visible, not even for a moment', len(gone) == 0, gone)
finally:
    stop = True
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
