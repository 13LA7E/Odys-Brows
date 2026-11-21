"""Extensions page: the Chrome Web Store link, pinning an extension to the notch, and the notch growing
(also the hole the page windows leave for it)."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

S = lambda e: ui('JSON.stringify(%s)' % e)
V = lambda e: json.loads(S(e))


def mk(name):
    d = tempfile.mkdtemp(prefix='cons-ext-')
    open(os.path.join(d, 'manifest.json'), 'w').write(json.dumps({"manifest_version": 3, "name": name, "version": "1.0", "description": "x"}))
    return d


def notch_w(): return V("document.querySelector('.notch').getBoundingClientRect().width")


def click_text(txt, idx=0):
    ui("[...document.querySelectorAll('.extensions-page button')].filter(b=>b.textContent===%s)[%d].click(); 1" % (json.dumps(txt), idx)); time.sleep(0.6)


prof = tempfile.mkdtemp(prefix='cons-prof-')
open(os.path.join(prof, 'urls.txt'), 'w').write('http://127.0.0.1:%d/a.html\n' % SRV)
p = launch(['--urls-file=' + os.path.join(prof, 'urls.txt')], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(4)
    bridge({'type': 'load-unpacked-extension', 'path': mk('Alpha Tool')}); time.sleep(1.2)
    bridge({'type': 'load-unpacked-extension', 'path': mk('Beta Tool')}); time.sleep(1.2)
    bridge({'type': 'open', 'url': 'odyssey://extensions'}); time.sleep(1.5)
    base = notch_w()
    check('the Extensions page has a Chrome Web Store button', ui("!![...document.querySelectorAll('.extensions-page button')].find(b=>b.textContent==='Chrome Web Store')") is True)
    n0 = V('window.__uiState.tabs.length')
    click_text('Chrome Web Store'); time.sleep(2.5)
    urls = V("window.__uiState.tabs.map(t=>t.url)")
    check('it opens the Chrome Web Store in a tab', any('chromewebstore.google.com' in (u or '') for u in urls), urls)
    bridge({'type': 'open', 'url': 'odyssey://extensions'}); time.sleep(1.2)
    check('nothing is pinned at first: no pinned buttons', V("document.querySelectorAll('.notch-pinned').length") == 0)
    click_text('Pin', 0)
    w1 = notch_w()
    check('pinning one adds a button and the notch grows', V("document.querySelectorAll('.notch-pinned').length") == 1 and w1 > base + 10, (base, w1))
    click_text('Pin', 0)
    w2 = notch_w()
    check('pinning a second grows it again', V("document.querySelectorAll('.notch-pinned').length") == 2 and w2 > w1 + 10, (w1, w2))
    saved = json.load(open(os.path.join(prof, 'Constellations', 'settings.json')))['pinnedExtensions']
    check('the pins are saved', len(saved) == 2, saved)
    check('the page-window hole follows the notch width (reported)', V("document.documentElement.style.getPropertyValue('--notch-w')") == '%dpx' % round(w2), (w2, V("document.documentElement.style.getPropertyValue('--notch-w')")))
    click_text('Unpin', 0)
    check('unpinning shrinks it back', V("document.querySelectorAll('.notch-pinned').length") == 1 and notch_w() < w2 - 10)
    # a disabled extension is not shown in the notch
    eid = V("window.__uiState.settings.pinnedExtensions[0]")
    bridge({'type': 'toggle-extension', 'id': eid, 'enabled': False}); time.sleep(1.2)
    check('a disabled extension leaves the notch', V("document.querySelectorAll('.notch-pinned').length") == 0)
    bridge({'type': 'toggle-extension', 'id': eid, 'enabled': True}); time.sleep(1.2)
    PS = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'capture_seq.ps1')
    subprocess.run(['powershell', '-ExecutionPolicy', 'Bypass', '-File', PS, '-Prefix', os.path.join(ROOT, 'out', 'live', 'ext-pinned'), '-Count', '1', '-IntervalMs', '10'], capture_output=True)
finally:
    kill(p)
p = launch([], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(5)
    check('after a restart the pinned extension is still in the notch', V("document.querySelectorAll('.notch-pinned').length") == 1)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
