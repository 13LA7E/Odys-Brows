"""An extension added while running survives a restart exactly once, and after Remove it is gone for
good (also from Chromium's own record, which is what the Web Store looks at)."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *
S = lambda e: ui('JSON.stringify(%s)' % e)
V = lambda e: json.loads(S(e))
work = tempfile.mkdtemp(prefix='cons-ext-')
os.makedirs(os.path.join(work, 'e'))
open(os.path.join(work, 'e', 'manifest.json'), 'w').write(json.dumps({'manifest_version': 3, 'name': 'Persist', 'version': '1.0', 'content_scripts': [{'matches': ['http://127.0.0.1/*'], 'js': ['c.js'], 'run_at': 'document_start'}]}))
open(os.path.join(work, 'e', 'c.js'), 'w').write("document.documentElement.dataset.cnt = String(Number(document.documentElement.dataset.cnt||0)+1);")
prof = tempfile.mkdtemp(prefix='cons-prof-')
open(os.path.join(prof, 'urls.txt'), 'w').write('http://127.0.0.1:%d/a.html\n' % SRV)


def start():
    p = launch(['--urls-file=' + os.path.join(prof, 'urls.txt')], prof)
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(3)
    return p


def count():
    bridge({'type': 'navigate', 'view': V('window.__uiState.activeTab'), 'url': 'http://127.0.0.1:%d/b.html' % SRV}); time.sleep(2)
    pw = wait(lambda: page_ws('/b.html'), 10)
    return ev(pw, "document.documentElement.dataset.cnt || '0'")


p = start()
try:
    bridge({'type': 'load-unpacked-extension', 'path': os.path.join(work, 'e')}); time.sleep(3)
    check('running after adding', count() == '1')
finally:
    kill(p)
p = start()
try:
    check('after a restart it runs once (not twice)', count() == '1', count())
    eid = V('window.__uiState.extensions[0].id')
    bridge({'type': 'remove-extension', 'id': eid}); time.sleep(2.5); print('STDERR', [l for l in open(os.path.join(prof, 'stderr.txt')).read().splitlines() if 'xtension' in l and 'external_registry' not in l][-5:])
    check('removed while running: gone at once', count() == '0')
finally:
    kill(p)
p = start()
try:
    check('after another restart it is still gone', count() == '0' and V('(window.__uiState.extensions||[]).length') == 0, count())
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
