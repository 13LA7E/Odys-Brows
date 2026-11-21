"""AUDIT-41: keyboard shortcuts. Real key presses (SendInput) in a page window go through the engine's
key hook; presses in our own window go through ui/shortcuts.js. Checks defaults, rebinding, the
rules (conflicts, reserved keys), Reset, and that changes survive a restart."""
import os, sys, ctypes
from ctypes import wintypes
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

S = lambda e: ui('JSON.stringify(%s)' % e)
user32 = ctypes.windll.user32
VK = {'CTRL': 0x11, 'SHIFT': 0x10, 'ALT': 0x12, 'T': 0x54, 'W': 0x57, 'K': 0x4B, 'F5': 0x74, 'R': 0x52, 'F9': 0x78, 'C': 0x43}


PID = [0]


def page_hwnd():
    """The visible page window: an untitled top-level window owned by the UI window."""
    found = []
    pid = wintypes.DWORD()
    cb = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)(
        lambda h, _l: (found.append(h) if user32.IsWindowVisible(h) and user32.GetWindow(h, 4)
                       and user32.GetWindowThreadProcessId(h, ctypes.byref(pid)) and pid.value == PID[0] else None) or True)
    user32.EnumWindows(cb, 0)
    return found[0] if found else None


def focus(h):
    user32.keybd_event(0x12, 0, 0, 0); user32.keybd_event(0x12, 0, 2, 0)   # Alt tap lets us take the foreground
    user32.ShowWindow(h, 9)
    user32.SetForegroundWindow(h)
    time.sleep(0.5)


def press(combo):
    keys = [VK[k] for k in combo]
    for k in keys:
        user32.keybd_event(k, 0, 0, 0); time.sleep(0.03)
    for k in reversed(keys):
        user32.keybd_event(k, 0, 2, 0); time.sleep(0.03)
    time.sleep(0.8)


hits = []


class Count(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *a, **k): super().__init__(*a, directory=site, **k)
    def log_message(self, *a): pass
    def do_GET(self):
        if self.path.startswith('/count'):
            hits.append(time.time())
            b = b'<!doctype html><title>Counter</title><p>hello</p>'
            self.send_response(200); self.send_header('Content-Type', 'text/html'); self.send_header('Content-Length', str(len(b))); self.end_headers(); self.wfile.write(b)
        else:
            super().do_GET()


cs = TS(('127.0.0.1', 8771), Count)
threading.Thread(target=cs.serve_forever, daemon=True).start()
URL = 'http://127.0.0.1:8771/count'


def tabs():
    return len(json.loads(S('window.__uiState.tabs')))


def result():
    return json.loads(S('window.__uiState.shortcutResult || null'))


def keys_of(cid):
    return next((c['keys'] for c in json.loads(S('window.__uiState.shortcutList')) if c['id'] == cid), None)


def start(prof):
    p = launch(['--urls-file=' + os.path.join(prof, 'urls.txt')] if os.path.exists(os.path.join(prof, 'urls.txt')) else [], prof)
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    wait(lambda: ui('(window.__uiState.shortcutList||[]).length') > 5, 10)
    time.sleep(2)
    return p


prof = tempfile.mkdtemp(prefix='cons-prof-')
open(os.path.join(prof, 'urls.txt'), 'w').write(URL + '\n')
p = start(prof); PID[0] = p.pid
try:
    check('the shell sends the list of commands with their default keys', keys_of('reload') == ['Ctrl+R', 'F5'] and keys_of('newTab') == ['Ctrl+T'], keys_of('reload'))
    h = wait(page_hwnd, 10)
    check('found the page window', bool(h))
    focus(h)

    # --- defaults, real key presses in the page window
    n0 = tabs(); press(['CTRL', 'T'])
    check('page window: Ctrl+T opens a tab', wait(lambda: tabs() == n0 + 1, 5) is not None, tabs())
    h = page_hwnd() or h
    # --- rebind reload to Ctrl+K
    bridge({'type': 'set-shortcut', 'id': 'reload', 'combo': 'Ctrl+K'})
    check('rebinding works', wait(lambda: keys_of('reload') == ['Ctrl+K'], 5) is not None, keys_of('reload'))
    check('...and the result says ok', (result() or {}).get('ok') is True, result())
    bridge({'type': 'navigate', 'view': json.loads(S('window.__uiState.activeTab')), 'url': URL + '?2'})
    time.sleep(1.5)
    h = page_hwnd() or h
    focus(h)
    b = len(hits); press(['CTRL', 'K'])
    check('page window: the new key reloads the page', len(hits) > b, (b, len(hits)))
    b = len(hits); press(['F5'])
    check('page window: the old key no longer reloads (swallowed, not left to Chromium)', len(hits) == b, (b, len(hits)))
    b = len(hits); press(['CTRL', 'R'])
    check('...Ctrl+R too', len(hits) == b, (b, len(hits)))
    # --- rules
    bridge({'type': 'set-shortcut', 'id': 'find', 'combo': 'Ctrl+K'}); time.sleep(0.6)
    r = result()
    check('a key another command uses is refused, naming that command', r and r['ok'] is False and 'Reload' in r['message'], r)
    check('...and nothing changed', keys_of('find') == ['Ctrl+F'])
    bridge({'type': 'set-shortcut', 'id': 'find', 'combo': 'Ctrl+C'}); time.sleep(0.6)
    r = result()
    check('Ctrl+C (editing) is refused', r and r['ok'] is False and 'editing' in r['message'], r)
    bridge({'type': 'set-shortcut', 'id': 'find', 'combo': 'K'}); time.sleep(0.6)
    r = result()
    check('a bare letter is refused', r and r['ok'] is False, r)
    bridge({'type': 'set-shortcut', 'id': 'find', 'combo': 'Ctrl+T'}); time.sleep(0.6)
    r = result()
    check('a default of another command is refused too', r and r['ok'] is False and 'New tab' in r['message'], r)
    # --- our own window
    n1 = tabs()
    ui("window.dispatchEvent(new KeyboardEvent('keydown',{key:'t',code:'KeyT',ctrlKey:true,bubbles:true})); 1")
    check('our own window: Ctrl+T opens a tab', wait(lambda: tabs() == n1 + 1, 5) is not None, (n1, tabs()))
    # --- Settings page: Change records a key press
    bridge({'type': 'open', 'url': 'odyssey://settings'}); time.sleep(0.8)
    ui("[...document.querySelectorAll('.page-internal *')].filter(e=>e.children.length===0&&e.textContent==='Shortcuts')[0].click(); 1"); time.sleep(0.5)
    labels = json.loads(ui("JSON.stringify([...document.querySelectorAll('.settings-content kbd')].map(k=>k.textContent))"))
    check('Settings lists the keys', 'Ctrl + K' in labels and 'Ctrl + T' in labels, labels)
    ui("[...document.querySelectorAll('.settings-content button')].find(b=>b.textContent==='Change').click(); 1"); time.sleep(0.4)
    check('Change enters recording', 'Press the new keys' in ui("document.querySelector('.settings-content').textContent"))
    ui("window.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',code:'Escape',bubbles:true})); 1"); time.sleep(0.4)
    check('Esc cancels recording', 'Press the new keys' not in ui("document.querySelector('.settings-content').textContent"))
    # a conflict through the real Change flow shows a message under that row
    ui("""[...document.querySelectorAll('.settings-content')[0].children].length; (()=>{const rows=[...document.querySelectorAll('.settings-content div')].filter(d=>d.children.length===2&&d.textContent.startsWith('Find in page')); rows[0].querySelector('button').click(); return 1})()"""); time.sleep(0.4)
    ui("window.dispatchEvent(new KeyboardEvent('keydown',{key:'k',code:'KeyK',ctrlKey:true,bubbles:true})); 1"); time.sleep(1.0)
    note = ui("(document.querySelector('.shortcut-note')||{}).textContent")
    check('Change + a taken key shows the message under the row', isinstance(note, str) and 'Reload' in note, note)
    # --- persistence
    st = json.load(open(os.path.join(prof, 'Constellations', 'settings.json')))
    check('the change is saved in settings.json', st.get('shortcuts') == {'reload': ['Ctrl+K']}, st.get('shortcuts'))
finally:
    kill(p)

# restart: the saved binding is active in page windows from the start
p = start(prof); PID[0] = p.pid
try:
    check('after a restart the binding is still there', keys_of('reload') == ['Ctrl+K'], keys_of('reload'))
    h = wait(page_hwnd, 10)
    if h:
        focus(h)
        b = len(hits); press(['CTRL', 'K'])
        check('after a restart the saved key works in a page window straight away', len(hits) > b, (b, len(hits)))
    bridge({'type': 'reset-shortcuts'})
    check('Reset all brings the defaults back', wait(lambda: keys_of('reload') == ['Ctrl+R', 'F5'], 5) is not None, keys_of('reload'))
    st = json.load(open(os.path.join(prof, 'Constellations', 'settings.json')))
    check('...and saves them', st.get('shortcuts') == {}, st.get('shortcuts'))
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
