import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

prof = tempfile.mkdtemp(prefix='cons-prof-')
open(os.path.join(prof, 'urls.txt'), 'w').write(f'http://127.0.0.1:{SRV}/a.html\n')
p = launch([f'--urls-file={os.path.join(prof, "urls.txt")}'], prof)
dl_dir = os.path.join(os.environ['USERPROFILE'], 'Downloads')
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    time.sleep(3)
    S = lambda e: ui(f'JSON.stringify({e})')

    # ---- AUDIT-03: render count / session writes / typing survives events
    rc0 = ui('window.__renderCount')
    pw = wait(lambda: page_ws('/a.html'), 10)
    for i in range(40):
        ev(pw, f"document.title='T{i}'")
    time.sleep(1)
    rc1 = ui('window.__renderCount')
    print('renders during 40 title changes:', rc1 - rc0)

    # typing in the Helm survives page events
    ui("document.querySelector('.helm-address').click(); 1")
    ui("(()=>{const i=document.querySelector('.helm-input'); i.value='typed-by-user'; i.dispatchEvent(new Event('input')); return 1})()")
    for i in range(10):
        ev(pw, f"document.title='X{i}'")
    time.sleep(1)
    val = ui("(()=>{const i=document.querySelector('.helm-input'); return i? (i.style.display+'|'+i.value):'gone'})()")
    check('Helm text survives page events (AUDIT-03)', val.endswith('typed-by-user') and not val.startswith('none'), val)
    ui("document.querySelector('.helm-input').blur(); 1")

    # sky loops
    loops = ui('window.__skyLoopCount()')
    check('sky loop count <= 1 (AUDIT-04)', loops <= 1, loops)
    bridge({'type': 'save-settings', 'settings': {'stars': 'Drifting', 'accent': 'Sea', 'trail': 'Narrow', 'connections': 8}})
    ui("window.__uiState.starsMode='drifting'; 1")
    bridge({'type': 'open', 'url': ''}); time.sleep(1)   # new tab page -> sky visible
    ui("for(let i=0;i<200;i++){window.dispatchEvent(new Event('resize'))}; 1"); time.sleep(1)
    n = ui('window.__skyLoopCount()')
    check('after 200 resizes with Drifting on the new-tab page: exactly 1 sky loop', n == 1, n)
    bridge({'type': 'show', 'view': json.loads(S('window.__uiState.tabs[0].id'))}); time.sleep(1)
    n2 = ui('window.__skyLoopCount()')
    check('sky loop paused while a web page is shown (AUDIT-34)', n2 == 0, n2)

    # ---- AUDIT-07: find in page
    view = json.loads(S('window.__uiState.activeTab'))
    bridge({'type': 'find-in-page', 'view': view, 'text': 'findme', 'forward': True, 'match_case': False, 'next': False})
    time.sleep(1)
    bridge({'type': 'stop-finding', 'view': view, 'clear_selection': True})
    err = open(os.path.join(prof, 'stderr.txt')).read()
    check('find/stop-finding messages accepted by Rust (no malformed-message log)', 'malformed' not in err, [l for l in err.splitlines() if 'malformed' in l][:2])

    # ---- AUDIT-20 zoom via the bridge
    bridge({'type': 'set-zoom', 'view': view, 'factor': 1.25}); time.sleep(0.5)

    # ---- AUDIT-05: download
    before = set(os.listdir(dl_dir)) if os.path.isdir(dl_dir) else set()
    ev(pw, f"location.href='http://127.0.0.1:{SRV}/big.bin'", gesture=True)
    d = wait(lambda: (lambda x: x if x and x.get('active') else None)(json.loads(S('(window.__uiState.downloads||[])[0]||null') or 'null')), 20)
    print('download state:', d)
    check('download appears in UI with progress (AUDIT-05)', bool(d) and d.get('progress', 0) >= 0 and d.get('detail'), d and d.get('detail'))
    helm_ring = ui("!!document.querySelector('.helm-dl-btn')")
    check('Helm shows the download ring', helm_ring is True, helm_ring)
    pop = ui("!!document.querySelector('.downloads-popover')")
    check('downloads popover rendered', pop is True, pop)
    if d:
        bridge({'type': 'download-control', 'id': d['id'], 'action': 'pause'}); time.sleep(1.5)
        a1 = json.loads(S('window.__uiState.downloads[0].received')); time.sleep(1.5); a2 = json.loads(S('window.__uiState.downloads[0].received'))
        check('pause stops progress', a2 - a1 < 1024 * 1024, (a1, a2))
        bridge({'type': 'download-control', 'id': d['id'], 'action': 'resume'}); time.sleep(1.5)
        a3 = json.loads(S('window.__uiState.downloads[0].received'))
        check('resume continues progress', a3 > a2, (a2, a3))
        bridge({'type': 'download-control', 'id': d['id'], 'action': 'cancel'}); time.sleep(2)
        fin = json.loads(S('window.__uiState.downloads[0]'))
        check('cancel marks download cancelled', fin.get('failed') or not fin.get('active'), fin.get('detail'))
    new = set(os.listdir(dl_dir)) - before
    for f in new:
        if f.startswith('big'):
            try: os.remove(os.path.join(dl_dir, f))
            except Exception: pass

    # ---- sleeping tab keeps URL (AUDIT-14)
    bridge({'type': 'open', 'url': f'http://127.0.0.1:{SRV}/b.html'}); time.sleep(2)
    tabs = json.loads(S('window.__uiState.tabs'))
    first = tabs[0]['id']
    bridge({'type': 'set-tab-sleeping', 'view': first, 'sleeping': True}); time.sleep(3)
    t0 = json.loads(S('window.__uiState.tabs.find(t=>t.id==%d)' % first))
    check('sleeping tab keeps real URL in UI (AUDIT-14)', 'a.html' in (t0.get('url') or ''), t0.get('url'))
    sess = None
    for dp, dn, fn in os.walk(prof):
        if 'session.json' in fn: sess = os.path.join(dp, 'session.json')
    time.sleep(2)
    if sess:
        sj = json.load(open(sess))
        check('session.json keeps sleeping tab URL', any('a.html' in t['url'] for t in sj['tabs']), [t['url'] for t in sj['tabs']])
    else:
        check('session.json exists', False)
    bridge({'type': 'show', 'view': first}); time.sleep(3)
    t1 = json.loads(S('window.__uiState.tabs.find(t=>t.id==%d)' % first))
    check('woken tab is shown and has URL', 'a.html' in (t1.get('url') or ''), t1.get('url'))
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
