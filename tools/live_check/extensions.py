import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

ext = tempfile.mkdtemp(prefix='cons-ext-')
open(os.path.join(ext, 'manifest.json'), 'w').write(json.dumps({"manifest_version": 3, "name": "Live Check Ext", "version": "1.0", "description": "adds a marker",
    "content_scripts": [{"matches": ["http://127.0.0.1/*"], "js": ["c.js"]}]}))
open(os.path.join(ext, 'c.js'), 'w').write("document.documentElement.setAttribute('data-live-ext','yes')")
prof = tempfile.mkdtemp(prefix='cons-prof-')
open(os.path.join(prof, 'urls.txt'), 'w').write(f'http://127.0.0.1:{SRV}/a.html\n')
S = lambda e: ui(f'JSON.stringify({e})')

p = launch([f'--urls-file={os.path.join(prof, "urls.txt")}'], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(3)
    check('extension not active before it is loaded', ev(page_ws('/a.html'), "document.documentElement.getAttribute('data-live-ext')") in (None, ''))
    bridge({'type': 'load-unpacked-extension', 'path': ext}); time.sleep(1)
    lst = json.loads(S('window.__uiState.extensions'))
    print(lst)
    check('extension listed, no invented memory/CPU numbers (AUDIT-22)', len(lst) == 1 and lst[0]['memory_mb'] == 0 and lst[0]['cpu_pct'] == 0, lst)
finally:
    kill(p)
p = launch([f'--urls-file={os.path.join(prof, "urls.txt")}'], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(5)
    v = wait(lambda: ev(page_ws('/a.html'), "document.documentElement.getAttribute('data-live-ext')"), 10)
    check('enabled extension runs after restart (AUDIT-22)', v == 'yes', v)
    eid = json.loads(S('window.__uiState.extensions[0].id')) if json.loads(S('(window.__uiState.extensions||[]).length')) else None
    bridge({'type': 'get-extensions'}); time.sleep(1)
    eid = json.loads(S('window.__uiState.extensions[0].id'))
    bridge({'type': 'toggle-extension', 'id': eid, 'enabled': False}); time.sleep(1)
finally:
    kill(p)
p = launch([f'--urls-file={os.path.join(prof, "urls.txt")}'], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(5)
    v = ev(page_ws('/a.html'), "document.documentElement.getAttribute('data-live-ext')")
    check('disabled extension does not run after restart', v in (None, ''), v)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
