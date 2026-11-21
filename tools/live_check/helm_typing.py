"""Address bar: a new tab focuses it at once, history suggestions finish what you type, and in a
normal (not full screen) window the real mouse at the top/bottom edge brings the notch and Helm back."""
import os, sys, ctypes, ctypes.wintypes as wt
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *
u32 = ctypes.windll.user32; g32 = ctypes.windll.gdi32; u32.SetProcessDPIAware()

S = lambda e: ui('JSON.stringify(%s)' % e)
V = lambda e: json.loads(S(e))
D = lambda n: ui("document.getElementById('app').dataset.%s" % n)


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




def cdp_call(method, params):
    s = cdp.ws_connect(ui_ws()); cdp.send(s, {"id": 1, "method": method, "params": params})
    while cdp.recv(s).get('id') != 1: pass
    s.close()


def type_text(t):
    for ch in t:
        cdp_call('Input.dispatchKeyEvent', {'type': 'keyDown', 'text': ch, 'key': ch})
        cdp_call('Input.dispatchKeyEvent', {'type': 'keyUp', 'key': ch})


def key(k, code):
    cdp_call('Input.dispatchKeyEvent', {'type': 'rawKeyDown', 'key': k, 'code': k, 'windowsVirtualKeyCode': code, 'commands': ['deleteBackward'] if k == 'Backspace' else []})
    cdp_call('Input.dispatchKeyEvent', {'type': 'keyUp', 'key': k, 'code': k, 'windowsVirtualKeyCode': code})


def main_hwnd(pid_set):
    best = []
    @ctypes.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)
    def cb(h, l):
        pid = wt.DWORD(); u32.GetWindowThreadProcessId(h, ctypes.byref(pid))
        if pid.value in pid_set and u32.IsWindowVisible(h):
            r = wt.RECT(); u32.GetWindowRect(h, ctypes.byref(r))
            best.append(((r.right - r.left) * (r.bottom - r.top), h))
        return True
    u32.EnumWindows(cb, 0)
    return max(best)[1] if best else None


prof = tempfile.mkdtemp(prefix='cons-prof-')
urls = os.path.join(prof, 'urls.txt')
open(urls, 'w').write('http://127.0.0.1:%d/a.html\n' % SRV)
p = launch(['--urls-file=' + urls], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    wait(lambda: D('helm') == 'shown', 20, 0.1)
    time.sleep(3)

    # 1. a new tab puts the cursor in the address bar
    bridge({'type': 'open', 'url': ''}); time.sleep(1.5)
    check('a new tab: the address bar is ready to type in', ui("document.activeElement && document.activeElement.className") == 'helm-input',
          ui("document.activeElement && document.activeElement.className"))

    # 2. suggestions from history (the first page was visited)
    bridge({'type': 'open', 'url': 'http://127.0.0.1:%d/b.html' % SRV}); time.sleep(2.5)
    bridge({'type': 'open', 'url': ''}); time.sleep(1.5)
    type_text('127.0.0')
    time.sleep(1.2)
    rows = V("[...document.querySelectorAll('.helm-suggest-row')].map(r=>r.textContent)")
    check('typing shows pages you visited', len(rows) >= 1, rows)
    val = ui('document.activeElement.value')
    sel = V('[document.activeElement.selectionStart, document.activeElement.selectionEnd]')
    check('the rest of the address is filled in, selected', val.startswith('127.0.0') and len(val) > 7 and sel[0] == 7 and sel[1] == len(val), (val, sel))
    check('the suggestion list has a hole in the page window (it is clickable)', V("(()=>{const r=document.querySelector('.helm-suggest').getBoundingClientRect(); return r.width>0 && r.bottom<=document.querySelector('.helm').getBoundingClientRect().top})()"))
    key('ArrowDown', 40); time.sleep(0.3)
    check('Down picks a suggestion', V("document.querySelectorAll('.helm-suggest-row')[0].style.background.length>0") is True)
    key('Escape', 27); time.sleep(0.4)
    check('Escape closes the list', ui("!document.querySelector('.helm-suggest')") is True)
    # keep typing a word with a space: no completion
    ui("window.odysseyReceive({type:'helm-focus'}); 1"); time.sleep(0.5)
    type_text('zzzqqq nothing')
    time.sleep(0.8)
    check('no match: nothing is filled in and no list', ui("!document.querySelector('.helm-suggest')") is True and ui('document.activeElement.value') == 'zzzqqq nothing', ui('document.activeElement.value'))
    # backspace removes the filled-in part without it coming back
    ui("window.odysseyReceive({type:'helm-focus'}); 1"); time.sleep(0.5)
    type_text('127.0.0'); time.sleep(1.0)
    ui("(()=>{const i=document.activeElement; i.setRangeText('',i.selectionStart,i.selectionEnd,'end'); i.dispatchEvent(new InputEvent('input',{inputType:'deleteContentBackward',bubbles:true}))})(); 1"); time.sleep(0.8)
    check('Backspace deletes the completion and it stays gone', ui('document.activeElement.value') == '127.0.0', ui('document.activeElement.value'))

    # a redraw while typing keeps the text and the caret
    ui("window.odysseyReceive({type:'helm-focus'}); 1"); time.sleep(0.5)
    type_text('127.0.0.1:87'); time.sleep(0.8)
    ui("window.__oldHelm = document.querySelector('.helm'); window.__uiState.tabs[0].title = 'Renamed ' + Date.now(); window.odysseyReceive({type:'space-changed', space: window.__uiState.activeSpace}); 1"); time.sleep(0.8)
    check('a redraw of the window while typing keeps what you typed and the cursor in the bar', ui("document.activeElement.className") == 'helm-input' and ui('document.activeElement.value').startswith('127.0.0.1:87'), (ui("document.querySelector('.helm') !== window.__oldHelm"), ui('document.activeElement.value')))

    # 3. a normal window: the real mouse on the top and bottom edge
    pids = set(int(l.split(',')[1].strip('"')) for l in subprocess.run(['tasklist', '/FI', 'IMAGENAME eq constellations.exe', '/FO', 'CSV', '/NH'], capture_output=True, text=True).stdout.splitlines() if l.strip())
    hw = main_hwnd(pids)
    u32.ShowWindow(hw, 9)   # restore: not maximised
    u32.MoveWindow(hw, 120, 80, 1200, 800, True)
    time.sleep(1.5)
    bridge({'type': 'navigate', 'view': V('window.__uiState.activeTab'), 'url': 'http://127.0.0.1:%d/a.html' % SRV}); time.sleep(2.5)
    ui("document.activeElement.blur(); 1"); u32.SetCursorPos(hw and 400, 400); time.sleep(0.1); u32.SetCursorPos(402, 402)
    wait(lambda: D('notch') == 'hidden' and D('helm') == 'hidden', 15, 0.2)
    #print('DBG', ui("JSON.stringify({page:window.__uiState.page, ae:document.activeElement&&document.activeElement.className, grip:document.querySelector('.notch-grip')!==null, tabs:window.__uiState.tabs.length, prompts:(window.__uiState.prompts||[]).length, sh:window.__uiState.shieldsOpen, hov:[...document.querySelectorAll(':hover')].map(e=>e.className).join(',')})"))
    check('notch and Helm have slid away over the page', D('notch') == 'hidden' and D('helm') == 'hidden', (D('notch'), D('helm')))
    r = wt.RECT(); u32.GetWindowRect(hw, ctypes.byref(r))
    def real_move(x, y):
        u32.SetCursorPos(int(x), int(y)); time.sleep(0.05); u32.SetCursorPos(int(x) + 1, int(y)); time.sleep(0.6)
    ow = ui('window.outerWidth'); dpi = ui('window.devicePixelRatio')
    # top edge, inside the notch's width, 4 px down (a position the old 4 px strip could not give reliably)
    real_move(r.right - 100 * dpi, r.top + 8)
    check('real mouse at the top edge of a normal window brings the notch back', D('notch') == 'shown', D('notch'))
    real_move(r.left + 400, r.top + 500); time.sleep(1.5)
    cr = wt.RECT(); u32.GetClientRect(hw, ctypes.byref(cr)); pt = wt.POINT(cr.right // 2, cr.bottom); u32.ClientToScreen(hw, ctypes.byref(pt))
    real_move(pt.x, pt.y - 10)
    check('real mouse at the bottom edge brings the Helm back', D('helm') == 'shown', D('helm'))
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
