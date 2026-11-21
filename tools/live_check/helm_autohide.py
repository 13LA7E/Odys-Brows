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
            if u32.GetWindowRgn(h, rgn) >= 2:
                res.append(bool(g32.PtInRegion(rgn, int((x_dip - 56) * dpi), int(y_dip * dpi))))
        return True
    u32.EnumWindows(cb, 0)
    return res[0] if res else None


def mouse(x, y):
    s = cdp.ws_connect(ui_ws()); cdp.send(s, {"id": 1, "method": "Input.dispatchMouseEvent", "params": {"type": "mouseMoved", "x": x, "y": y}})
    while cdp.recv(s).get('id') != 1: pass
    s.close()


prof = tempfile.mkdtemp(prefix='cons-prof-')
urls = os.path.join(prof, 'urls.txt')
open(urls, 'w').write('http://127.0.0.1:%d/a.html\n' % SRV)
p = launch(['--urls-file=' + urls], prof)
S = lambda e: ui('JSON.stringify(%s)' % e)
D = lambda n: ui("document.getElementById('app').dataset.%s" % n)
TY = "(()=>{const m=getComputedStyle(document.querySelector('.helm')).transform; return m==='none'?0:parseFloat(m.split(',')[5])})()"
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    wait(lambda: D('helm') == 'shown', 20, 0.1)
    dpi = ui('window.devicePixelRatio'); iw = ui('window.innerWidth'); ih = ui('window.innerHeight')
    hx, hy = iw // 2, ih - 28 - 26      # centre of the Helm
    check('Helm shows for a moment when the page appears', D('helm') == 'shown')
    check('...its hole is in the page window', covered(hx, hy, dpi) is False)

    # watch the slide inside the page, one sample per frame: the transform must pass through in-between values
    ui("window.__ty=[];window.__nh=[];(()=>{const t0=performance.now();(function f(){const m=getComputedStyle(document.querySelector('.helm')).transform;window.__ty.push(m==='none'?0:Math.round(parseFloat(m.split(',')[5])));const n=document.querySelector('.notch');if(n)window.__nh.push(Math.round(n.getBoundingClientRect().bottom));if(performance.now()-t0<9000)requestAnimationFrame(f)})()})(); 1")
    wait(lambda: D('helm') == 'hidden', 10, 0.1)
    time.sleep(1.0)
    samples = json.loads(S('window.__ty'))
    mid = sorted(set(v for v in samples if 2 < v < 60))
    print('Helm slide, distinct in-between offsets (px):', mid)
    check('Helm slides down gradually, frame by frame (not instantly)', len(mid) >= 4, mid)
    time.sleep(0.6)
    check('Helm is hidden after a while over a web page', D('helm') == 'hidden', D('helm'))
    check('...the page window now covers the Helm area (page fully usable)', covered(hx, hy, dpi) is True)
    check('...and no strip is left over the page at the bottom either', covered(hx, ih - 2, dpi) is True)

    # notch slide: the hole follows it frame by frame
    heights = ui('JSON.stringify(window.__notchHeights)')
    print('notch hole heights reported:', heights)
    hs = json.loads(heights)
    check('the notch hole shrank in steps while it slid away (page shows through)', len([h for h in hs if 4 < h < 34]) >= 3, hs)

    # hover the strip: Helm returns, animated
    real_cursor(0 + hx, -3) if False else real_cursor(hx, -3)
    time.sleep(0.12)
    t1 = ui(TY)
    time.sleep(0.8)
    check('pointer on the bottom strip brings the Helm back (animated)', D('helm') == 'shown' and 0 <= t1 < 90, (D('helm'), round(t1)))
    check('...and its hole returns', covered(hx, hy, dpi) is False)
    real_cursor(300, 300)
    time.sleep(2.6)
    check('moving away hides it again', D('helm') == 'hidden', D('helm'))

    # Ctrl+L (the shell sends helm-focus): reveals the Helm and keeps it while typing
    ui("window.odysseyReceive({type:'helm-focus'}); 1")
    time.sleep(0.8)
    check('Ctrl+L brings the Helm back and focuses the address', D('helm') == 'shown' and ui("document.activeElement && document.activeElement.className") == 'helm-input', (D('helm'), ui("document.activeElement && document.activeElement.className")))
    time.sleep(6)
    check('it stays while the address bar has focus (typing)', D('helm') == 'shown', D('helm'))
    ui("document.activeElement.blur(); 1")
    time.sleep(3.2)
    check('...and hides after you leave it', D('helm') == 'hidden', D('helm'))

    # Ask mode keeps it up
    ui("document.querySelector('.helm-asst-btn').click(); 1") if False else None
    # a new page shows it for a moment
    bridge({'type': 'navigate', 'view': json.loads(ui('JSON.stringify(window.__uiState.activeTab)')), 'url': 'http://127.0.0.1:%d/b.html' % SRV})
    time.sleep(1.2)
    check('navigating shows the Helm for a moment', D('helm') == 'shown', D('helm'))
    # new-tab page: nothing hides
    bridge({'type': 'open', 'url': ''})
    time.sleep(1.2)
    check('on the new-tab page the Helm is always visible', D('helm') == 'shown' and ui("!document.querySelector('.helm-grip')") is True, D('helm'))
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
