import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

prof = tempfile.mkdtemp(prefix='cons-prof-')
open(os.path.join(prof, 'urls.txt'), 'w').write(f'http://127.0.0.1:{SRV}/a.html\n')
p = launch([f'--urls-file={os.path.join(prof, "urls.txt")}'], prof)
try:
    ok = wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    check('UI loads in live mode', bool(ok))
    st = lambda: ui('JSON.stringify({tabs:window.__uiState.tabs.map(t=>({id:t.id,url:t.url,title:t.title,parent:t.parent})),active:window.__uiState.activeTab,page:window.__uiState.page,shields:window.__uiState.shields,helm:window.__uiState.helm})')
    time.sleep(3)
    s0 = json.loads(st()); print(s0)
    check('urls-file tab opened and is a web page', s0['page'] == 'web' and len(s0['tabs']) == 1, s0['page'])
    check('Shields starts at 0 (AUDIT-25)', s0['shields'] == 0, s0['shields'])

    # AUDIT-02: popups
    pw = wait(lambda: page_ws('/a.html'), 10)
    check('page target reachable', bool(pw))
    ev(pw, "document.getElementById('blank').click()", gesture=True)
    time.sleep(2)
    s1 = json.loads(st()); print(s1)
    check('target=_blank opens a tab (AUDIT-02)', len(s1['tabs']) == 2, len(s1['tabs']))
    if len(s1['tabs']) >= 2:
        check('...as child of opener', s1['tabs'][1]['parent'] == s1['tabs'][0]['id'], s1['tabs'][1]['parent'])
    pw = page_ws('/a.html')
    ev(pw, "document.getElementById('wopen').click()", gesture=True)
    time.sleep(2)
    s2 = json.loads(st())
    check('window.open with gesture opens a tab', len(s2['tabs']) == 3, len(s2['tabs']))
    check('window.open without gesture (timer) is blocked', not any('spam' in (t['url'] or '') for t in s2['tabs']), [t['url'] for t in s2['tabs']])

    # AUDIT-09: history
    bridge({'type': 'open', 'url': 'odyssey://history'}); time.sleep(2)
    h = ui('JSON.stringify({page:window.__uiState.page,name:window.__uiState.pageName,groups:(window.__uiState.history||[]).length,rows:document.querySelectorAll(".history-title").length})')
    print('history', h)
    hist = json.loads(h)
    check('History page opens and loads groups (AUDIT-09)', hist['name'] == 'history' and hist['groups'] >= 1, hist)

    # AUDIT-27: settings reuse
    bridge({'type': 'open', 'url': 'odyssey://settings'}); time.sleep(1)
    n1 = len(json.loads(st())['tabs'])
    bridge({'type': 'open', 'url': 'odyssey://settings'}); time.sleep(1)
    s3 = json.loads(st())
    check('Settings reuses its tab (AUDIT-27)', len(s3['tabs']) == n1, (n1, len(s3['tabs'])))
    check('internal tab has a title', any(t['title'] == 'Settings' for t in s3['tabs']), [t['title'] for t in s3['tabs']])

    # session.json write count while loading (AUDIT-03)
    sj = os.path.join(prof, 'ConstellationsBrowser') if False else None
finally:
    kill(p)
    print(open(os.path.join(prof, 'stderr.txt')).read().replace('\r', '')[-1500:] if False else '')
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
