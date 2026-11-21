"""Downloads: the button in the address bar stays while there are downloads (count with several),
the card can be closed, clears, goes away by itself, and a downloaded .crx is installed as an
extension (not handed to another browser)."""
import os, sys, zipfile, struct, io
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *
S = lambda e: ui('JSON.stringify(%s)' % e)
V = lambda e: json.loads(S(e))

# a tiny extension as a CRX3 file, and an ordinary file
z = io.BytesIO()
with zipfile.ZipFile(z, 'w') as f:
    f.writestr('manifest.json', json.dumps({'manifest_version': 3, 'name': 'Demo Ext', 'version': '1.0'}))
crx = b'Cr24' + struct.pack('<II', 3, 4) + b'\0' * 4 + z.getvalue()
open(os.path.join(site, 'demo.crx'), 'wb').write(crx)
open(os.path.join(site, 'plain.bin'), 'wb').write(b'x' * 5000)
dl_dir = os.path.join(os.environ['USERPROFILE'], 'Downloads')
before = set(os.listdir(dl_dir)) if os.path.isdir(dl_dir) else set()

prof = tempfile.mkdtemp(prefix='cons-prof-')
open(os.path.join(prof, 'urls.txt'), 'w').write('http://127.0.0.1:%d/a.html\n' % SRV)
p = launch(['--urls-file=' + os.path.join(prof, 'urls.txt')], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(3)
    pw = wait(lambda: page_ws('/a.html'), 10)
    ev(pw, "location.href='http://127.0.0.1:%d/plain.bin'" % SRV, gesture=True)
    wait(lambda: V('(window.__uiState.downloads||[]).filter(d=>d.done).length') >= 1, 20)
    time.sleep(1.8)
    check('after the download ended the button is still in the address bar', ui("!!document.querySelector('.helm-dl-btn')") is True)
    check('the card is open with a close button', ui("!!document.querySelector('.downloads-popover .dl-x')") is True)
    ui("document.querySelector('.downloads-popover [aria-label=Close]').click(); 1"); time.sleep(0.5)
    check('Close closes the card', ui("!document.querySelector('.downloads-popover')") is True)
    check('...and the button stays', ui("!!document.querySelector('.helm-dl-btn')") is True)
    # the extension (a fresh page: Chromium holds back a second automatic download from the same page)
    bridge({'type': 'navigate', 'view': V('window.__uiState.activeTab'), 'url': 'http://127.0.0.1:%d/b.html' % SRV}); time.sleep(2.5)
    pw = wait(lambda: page_ws('/b.html'), 10)
    ev(pw, "location.href='http://127.0.0.1:%d/demo.crx'" % SRV, gesture=True)
    ok = wait(lambda: V('(window.__uiState.downloads||[]).some(d=>d.installed)') is True, 20)
    check('a downloaded extension file is installed', bool(ok), V('(window.__uiState.downloads||[]).map(d=>[d.name,d.detail])'))
    check('...the note says so and there is no Open button for it', ui("document.querySelector('.downloads-popover') && !!document.querySelector('.downloads-popover') && !([...document.querySelectorAll('.downloads-popover button')].some(b=>b.textContent==='Open' && b.closest('div').textContent.includes('Demo')))") is True)
    bridge({'type': 'get-extensions'}); time.sleep(0.8)
    found = [r for r, ds, fs in os.walk(prof) if 'manifest.json' in fs and os.path.basename(os.path.dirname(r)) == 'extensions']
    check('Demo Ext is unpacked in the profile extensions folder', len(found) == 1, found)
    check('two downloads: the button shows a count', ui("document.querySelector('.helm-dl-count') && document.querySelector('.helm-dl-count').textContent") == '2', ui("document.querySelector('.helm-dl-count') && document.querySelector('.helm-dl-count').textContent"))
    time.sleep(7)
    check('the card goes away by itself after a few seconds', ui("!document.querySelector('.downloads-popover')") is True)
    ui("document.querySelector('.helm-dl-btn').click(); 1"); time.sleep(0.5)
    check('clicking the button opens the card again', ui("!!document.querySelector('.downloads-popover')") is True)
    ui("document.querySelector('.downloads-popover .dl-clear').click(); 1"); time.sleep(0.5)
    check('Clear empties the list and the button goes', ui("!document.querySelector('.helm-dl-btn') && !document.querySelector('.downloads-popover')") is True)
finally:
    kill(p)
    for f in set(os.listdir(dl_dir)) - before:
        try: os.remove(os.path.join(dl_dir, f))
        except OSError: pass
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
