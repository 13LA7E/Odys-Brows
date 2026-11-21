"""Extensions run at once, without restarting: load a folder, install a .crx, see it work on a page;
turn it off and remove it and it is gone from pages too. Needs the private DevTools pipe."""
import os, sys, io, zipfile, struct
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *
S = lambda e: ui('JSON.stringify(%s)' % e)
V = lambda e: json.loads(S(e))
MANI = json.dumps({'manifest_version': 3, 'name': 'Demo Live', 'version': '1.0', 'content_scripts': [{'matches': ['http://127.0.0.1/*'], 'js': ['c.js'], 'run_at': 'document_start'}]})
work = tempfile.mkdtemp(prefix='cons-ext-')
os.makedirs(os.path.join(work, 'unpacked'))
open(os.path.join(work, 'unpacked', 'manifest.json'), 'w').write(MANI)
open(os.path.join(work, 'unpacked', 'c.js'), 'w').write("document.documentElement.dataset.demoExt='yes';")
z = io.BytesIO()
with zipfile.ZipFile(z, 'w') as f:
    f.writestr('manifest.json', MANI.replace('Demo Live', 'Demo Packed'))
    f.writestr('c.js', "document.documentElement.dataset.demoPacked='yes';")
open(os.path.join(site, 'packed.crx'), 'wb').write(b'Cr24' + struct.pack('<II', 3, 4) + b'\0' * 4 + z.getvalue())
prof = tempfile.mkdtemp(prefix='cons-prof-')
open(os.path.join(prof, 'urls.txt'), 'w').write('http://127.0.0.1:%d/a.html\n' % SRV)
dl_dir = os.path.join(os.environ['USERPROFILE'], 'Downloads')
before = set(os.listdir(dl_dir))
p = launch(['--urls-file=' + os.path.join(prof, 'urls.txt')], prof)


def marker(attr):
    bridge({'type': 'navigate', 'view': V('window.__uiState.activeTab'), 'url': 'http://127.0.0.1:%d/b.html' % SRV}); time.sleep(2)
    pw = wait(lambda: page_ws('/b.html'), 10)
    return ev(pw, "document.documentElement.dataset.%s || 'no'" % attr)


try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(3)
    bridge({'type': 'load-unpacked-extension', 'path': os.path.join(work, 'unpacked')}); time.sleep(3)
    exts = V('window.__uiState.extensions||[]')
    check('a loaded folder is listed and running without a restart', len(exts) == 1 and exts[0].get('live') is True, exts)
    check('...and it really runs on a page', marker('demoExt') == 'yes')
    eid = exts[0]['id'] if exts else ''
    bridge({'type': 'toggle-extension', 'id': eid, 'enabled': False}); time.sleep(2)
    check('turned off: it stops at once', marker('demoExt') == 'no')
    bridge({'type': 'toggle-extension', 'id': eid, 'enabled': True}); time.sleep(3)
    check('turned on again: it runs again', marker('demoExt') == 'yes')
    bridge({'type': 'remove-extension', 'id': eid}); time.sleep(2)
    check('removed: gone from the list and from pages', V('(window.__uiState.extensions||[]).length') == 0 and marker('demoExt') == 'no')
    # a .crx
    bridge({'type': 'navigate', 'view': V('window.__uiState.activeTab'), 'url': 'http://127.0.0.1:%d/a.html' % SRV}); time.sleep(2)
    pw = wait(lambda: page_ws('/a.html'), 10)
    ev(pw, "location.href='http://127.0.0.1:%d/packed.crx'" % SRV, gesture=True)
    ok = wait(lambda: V('(window.__uiState.downloads||[]).some(d=>/ready to use/.test(d.detail||""))') is True, 20)
    check('an installed .crx says it is ready to use', bool(ok), V('(window.__uiState.downloads||[]).map(d=>d.detail)'))
    check('...and works on the next page at once', marker('demoPacked') == 'yes')
    # an extension that opens its welcome page when installed (like most store extensions)
    os.makedirs(os.path.join(work, 'welcome'))
    open(os.path.join(work, 'welcome', 'manifest.json'), 'w').write(json.dumps({'manifest_version': 3, 'name': 'Welcomer', 'version': '1.0', 'background': {'service_worker': 'bg.js'}, 'permissions': []}))
    open(os.path.join(work, 'welcome', 'bg.js'), 'w').write("chrome.runtime.onInstalled.addListener(() => chrome.tabs.create({url: 'http://127.0.0.1:%d/b.html?welcome=1'}));" % SRV)
    n0 = V('window.__uiState.tabs.length')
    bridge({'type': 'load-unpacked-extension', 'path': os.path.join(work, 'welcome')}); time.sleep(4)
    urls = V('window.__uiState.tabs.map(t=>t.url||"")')
    check('its welcome page opens as a tab in Constellations', any('welcome=1' in u for u in urls) and V('window.__uiState.tabs.length') == n0 + 1, urls)
    wins = subprocess.run(['powershell', '-NoProfile', '-Command', "(Get-Process constellations | Where-Object {$_.MainWindowTitle} | Measure-Object).Count"], capture_output=True, text=True).stdout.strip()
    print('visible top-level windows with a title:', wins)
    # icon, options page, pinning, and the Remove button in the page
    import base64
    os.makedirs(os.path.join(work, 'iconic'))
    png = base64.b64decode('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/q842iQAAAABJRU5ErkJggg==')
    open(os.path.join(work, 'iconic', 'icon.png'), 'wb').write(png)
    open(os.path.join(work, 'iconic', 'o.html'), 'w').write('<title>Iconic options</title><h1 id=x>options page</h1>')
    open(os.path.join(work, 'iconic', 'manifest.json'), 'w').write(json.dumps({'manifest_version': 3, 'name': 'Iconic', 'version': '1.0', 'icons': {'16': 'icon.png', '48': 'icon.png'}, 'options_page': 'o.html'}))
    bridge({'type': 'load-unpacked-extension', 'path': os.path.join(work, 'iconic')}); time.sleep(3)
    ex = [e for e in V('window.__uiState.extensions') if e['name'] == 'Iconic']
    check('its icon and options page are read from the manifest', len(ex) == 1 and (ex[0].get('icon') or '').startswith('data:image/png;base64,') and ex[0].get('options') == 'o.html', ex and {k: ex[0].get(k) for k in ('icon', 'options', 'chrome_id')})
    bridge({'type': 'save-settings', 'settings': {'pinnedExtensions': [ex[0]['id']]}}); time.sleep(0.8); bridge({'type': 'load-settings'}); time.sleep(1.5)
    check('a pinned extension shows its own icon in the pill', ui("!!document.querySelector('.notch-pinned img')") is True)
    n0 = V('window.__uiState.tabs.length')
    ui("document.querySelector('.notch-pinned').click(); 1"); time.sleep(2)
    pw = wait(lambda: page_ws('/o.html'), 10)
    check('clicking it opens its options page in a new tab', V('window.__uiState.tabs.length') == n0 + 1 and bool(pw) and ev(pw, "document.getElementById('x') && document.getElementById('x').textContent") == 'options page')
    bridge({'type': 'open', 'url': 'odyssey://extensions'}); time.sleep(1.5)
    RM = "[...document.querySelectorAll('.extension-card')].find(c=>c.textContent.includes('Iconic')).querySelector('.extension-card > div:last-child > button:last-child').click(); 1"
    cnt = V('(window.__uiState.extensions||[]).length')
    ui(RM); time.sleep(0.4)
    hgt = V("(()=>{const c=[...document.querySelectorAll('.extension-card')].find(c=>c.textContent.includes('Iconic')); const b=c.querySelector(':scope > div:last-child > button:last-child'); const r=b.getBoundingClientRect(); return [Math.round(r.height), Math.round(r.width), b.textContent, getComputedStyle(b).outlineStyle]})()")
    check('the confirm button stays on one line and has no focus box', hgt[0] < 40 and hgt[3] == 'none', hgt)
    check('Remove asks for a second click first', V('(window.__uiState.extensions||[]).length') == cnt)
    ui(RM); time.sleep(1.5)
    check('...then removes it', V('(window.__uiState.extensions||[]).length') == cnt - 1 and not any(e['name'] == 'Iconic' for e in V('window.__uiState.extensions')))
finally:
    kill(p)
    for f in set(os.listdir(dl_dir)) - before:
        try: os.remove(os.path.join(dl_dir, f))
        except OSError: pass
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
