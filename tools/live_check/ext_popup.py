"""Extension popups: a pinned extension's button opens its own popup (action.default_popup) in a small
window under the notch; the id Chromium gives the extension is worked out correctly; the popup closes
when you click elsewhere; an extension without a popup opens the Extensions page."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

S = lambda e: ui('JSON.stringify(%s)' % e)
V = lambda e: json.loads(S(e))


def mk(name, popup):
    d = tempfile.mkdtemp(prefix='cons-ext-')
    man = {"manifest_version": 3, "name": name, "version": "1.0", "description": "x"}
    if popup:
        man["action"] = {"default_popup": "popup.html"}
        open(os.path.join(d, 'popup.html'), 'w').write('<!doctype html><title>Popup OK</title><body style="width:300px;height:200px"><h1 id=h>hello popup</h1><script src="popup.js"></script>')
        open(os.path.join(d, 'popup.js'), 'w').write('document.getElementById("h").textContent="popup script ran"')
    open(os.path.join(d, 'manifest.json'), 'w').write(json.dumps(man))
    return d


def popup_targets():
    return [t for t in targets() if t.get('type') == 'page' and t.get('url', '').startswith('chrome-extension://')]


prof = tempfile.mkdtemp(prefix='cons-prof-')
a = mk('Popup Tool', True)
b = mk('Plain Tool', False)
p = launch([], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(3)
    bridge({'type': 'load-unpacked-extension', 'path': a}); time.sleep(1)
    bridge({'type': 'load-unpacked-extension', 'path': b}); time.sleep(1)
finally:
    kill(p)

p = launch([], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(5)
    bridge({'type': 'get-extensions'}); time.sleep(1.2)
    exts = V('window.__uiState.extensions')
    pop = next(e for e in exts if e['name'] == 'Popup Tool')
    plain = next(e for e in exts if e['name'] == 'Plain Tool')
    check('the list says which extension has a popup', pop.get('popup') == 'popup.html' and not plain.get('popup'), [(e['name'], e.get('popup')) for e in exts])
    check('each extension has a Chromium id (32 letters a to p)', len(pop.get('chrome_id', '')) == 32 and set(pop['chrome_id']) <= set('abcdefghijklmnop'), pop.get('chrome_id'))
    bridge({'type': 'save-settings', 'settings': {'pinnedExtensions': [pop['id'], plain['id']]}}); time.sleep(0.8)
    bridge({'type': 'load-settings'}); time.sleep(1.5)
    check('both pinned buttons are in the notch', V("document.querySelectorAll('.notch-pinned').length") == 2)
    ui("[...document.querySelectorAll('.notch-pinned')].find(b=>b.getAttribute('aria-label')==='Popup Tool').click(); 1")
    t = wait(lambda: popup_targets(), 15)
    check('clicking it opens the extension popup page', bool(t), [x['url'] for x in targets()])
    if t:
        check('the popup lives under the id we computed (so the id is right)', t[0]['url'].startswith('chrome-extension://%s/popup.html' % pop['chrome_id']), t[0]['url'])
        time.sleep(1)
        title = ev(t[0]['webSocketDebuggerUrl'], 'document.title + "|" + document.getElementById("h").textContent')
        check('its page runs (title and script)', title == 'Popup OK|popup script ran', title)
        # it is a small window under the notch, not a full tab
        import ctypes
        from ctypes import wintypes
        u = ctypes.windll.user32
        found = []
        pid = wintypes.DWORD()
        cb = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)(lambda h, _l: (found.append(h) if u.IsWindowVisible(h) and u.GetWindow(h, 4) and u.GetWindowThreadProcessId(h, ctypes.byref(pid)) and pid.value == p.pid else None) or True)
        u.EnumWindows(cb, 0)
        sizes = []
        for h in found:
            r = wintypes.RECT(); u.GetWindowRect(h, ctypes.byref(r)); sizes.append((r.right - r.left, r.bottom - r.top))
        check('the popup is a small window (about 380 x 540)', any(300 < w < 700 and 400 < h < 900 for w, h in sizes), sizes)
        check('opening it did not add a tab', all('chrome-extension' not in (x or '') for x in V("window.__uiState.tabs.map(t=>t.url)")), V("window.__uiState.tabs.map(t=>t.url)"))
        # clicking elsewhere (the main window gets focus) closes it
        bridge({'type': 'focus-window'}); ui("window.focus(); 1")
        ctypes.windll.user32.SetForegroundWindow(found[-1] if False else ctypes.windll.user32.GetWindow(found[0], 4))
        gone = wait(lambda: not popup_targets(), 8)
        check('clicking elsewhere closes the popup', bool(gone), [x['url'] for x in targets()])
    # a second click opens it again, and an extension without a popup opens the Extensions page
    ui("[...document.querySelectorAll('.notch-pinned')].find(b=>b.getAttribute('aria-label')==='Popup Tool').click(); 1")
    check('it opens again', bool(wait(lambda: popup_targets(), 15)))
    bridge({'type': 'close-extension-popup'}); time.sleep(1.5)
    check('the app can close it too', not popup_targets())
    ui("[...document.querySelectorAll('.notch-pinned')].find(b=>b.getAttribute('aria-label')==='Plain Tool').click(); 1"); time.sleep(1.5)
    check('an extension without a popup opens the Extensions page', V('window.__uiState.pageName') == 'extensions' and not popup_targets())
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
