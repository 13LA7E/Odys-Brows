import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

prof = tempfile.mkdtemp(prefix='cons-prof-')
open(os.path.join(prof, 'urls.txt'), 'w').write(f'http://127.0.0.1:{SRV}/a.html\n')
p = launch([f'--urls-file={os.path.join(prof, "urls.txt")}'], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    time.sleep(3)
    S = lambda e: ui(f'JSON.stringify({e})')
    bridge({'type': 'open', 'url': f'http://127.0.0.1:{SRV}/b.html'})
    print('opened b', wait(lambda: len(json.loads(S('window.__uiState.tabs'))) == 2, 10))
    tabs = json.loads(S('window.__uiState.tabs'))
    first = tabs[0]['id']
    print('tabs', tabs)
    bridge({'type': 'set-tab-sleeping', 'view': first, 'sleeping': True}); time.sleep(3)
    print('after sleep targets:', [(t.get('type'), t.get('url')[:50]) for t in targets()])
    t0 = json.loads(S('window.__uiState.tabs.find(t=>t.id==%d)' % first))
    print('t0', t0)
    check('sleeping tab keeps real URL in UI (AUDIT-14)', 'a.html' in (t0.get('url') or ''), t0.get('url'))
    time.sleep(2)
    sess = None
    for dp, dn, fn in os.walk(prof):
        if 'session.json' in fn: sess = os.path.join(dp, 'session.json')
    if sess:
        sj = json.load(open(sess))
        check('session.json keeps sleeping tab URL', any('a.html' in t['url'] for t in sj['tabs']), [t['url'] for t in sj['tabs']])
    else:
        check('session.json exists', False)
    bridge({'type': 'show', 'view': first}); time.sleep(3)
    t1 = json.loads(S('window.__uiState.tabs.find(t=>t.id==%d)' % first))
    check('woken tab has URL', 'a.html' in (t1.get('url') or ''), t1.get('url'))
    pg = wait(lambda: page_ws('/a.html'), 10)
    check('woken tab actually reloaded its page', bool(pg))

    # session write throttle: count mtime changes during a page-load storm
    sess = [os.path.join(dp, 'session.json') for dp, dn, fn in os.walk(prof) if 'session.json' in fn][0]
    writes = set(); end = time.time() + 6
    pw = page_ws('/a.html')
    def storm():
        for i in range(60):
            try: ev(pw, f"document.title='S{i}'")
            except Exception: pass
    th = threading.Thread(target=storm); th.start()
    while time.time() < end:
        try: writes.add(os.stat(sess).st_mtime_ns)
        except Exception: pass
        time.sleep(0.05)
    th.join()
    check('session.json written at most ~1/s during title storm (AUDIT-03)', len(writes) <= 8, len(writes))

    # zoom via keyboard-equivalent events is engine-side; verify via bridge no crash
    bridge({'type': 'set-zoom', 'view': first, 'factor': 1.5}); time.sleep(0.5)
    check('app still alive after zoom', p.poll() is None)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
