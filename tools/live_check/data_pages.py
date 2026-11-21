import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *
os.environ['CONSTELLATIONS_TEST_AUTH'] = 'allow'  # Reveal waits for Windows verification; debug builds only

prof = tempfile.mkdtemp(prefix='cons-prof-')
open(os.path.join(prof, 'urls.txt'), 'w').write(f'http://127.0.0.1:{SRV}/a.html\n')
p = launch([f'--urls-file={os.path.join(prof, "urls.txt")}'], prof)
S = lambda e: ui(f'JSON.stringify({e})')
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    time.sleep(3)
    # history rows after 3 visits
    for n in 'bc':
        bridge({'type': 'open', 'url': f'http://127.0.0.1:{SRV}/{n}.html'}); time.sleep(2)
    bridge({'type': 'query-history'}); time.sleep(1)
    g = json.loads(S('window.__uiState.history'))
    rows = [v for d in g for v in d['visits']]
    print('history rows', [(v['url'][-6:], v.get('opener_id')) for v in rows])
    check('history records each visit once (P4-02)', len(rows) == 3, len(rows))
    bridge({'type': 'search-history', 'query': 'b.html'}); time.sleep(1)
    g2 = json.loads(S('window.__uiState.history')); r2 = [v for d in g2 for v in d['visits']]
    check('history search filters', len(r2) == 1 and 'b.html' in r2[0]['url'], len(r2))
    bridge({'type': 'forget-visit', 'id': r2[0]['id']}); time.sleep(1)
    g3 = json.loads(S('window.__uiState.history')); r3 = [v for d in g3 for v in d['visits']]
    check('forget removes the visit', all('b.html' not in v['url'] for v in r3), len(r3))
    bridge({'type': 'clear-history-day', 'day_start_secs': g3[0]['day_start_secs']}) if g3 else None; time.sleep(1)
    g4 = json.loads(S('window.__uiState.history'))
    check('clear this day empties it', sum(len(d['visits']) for d in g4) == 0, g4)

    # closed tabs
    tabs = json.loads(S('window.__uiState.tabs'))
    cid = [t for t in tabs if t['url'].endswith('c.html')][0]['id']
    bridge({'type': 'close', 'view': cid}); time.sleep(2)
    closed = json.loads(S('window.__uiState.closed||[]'))
    check('closed tab appears in "closed earlier" (P4-04)', len(closed) == 1 and closed[0]['url'].endswith('c.html'), closed)
    n_before = len(json.loads(S('window.__uiState.tabs')))
    bridge({'type': 'relight-closed-tab'}); time.sleep(2)
    t_after = json.loads(S('window.__uiState.tabs'))
    check('relight reopens it', len(t_after) == n_before + 1 and t_after[-1]['url'].endswith('c.html'), [t['url'][-6:] for t in t_after])
    check('closed list emptied after relight', json.loads(S('(window.__uiState.closed||[]).length')) == 0)

    # spaces + pinned
    bridge({'type': 'switch-space', 'space': 1}); time.sleep(1)
    check('space switch reported', json.loads(S('window.__uiState.activeSpace')) == 1)
    bridge({'type': 'switch-space', 'space': 0}); time.sleep(1)

    # vault
    bridge({'type': 'vault-add', 'origin': 'example.test', 'username': 'user1', 'password': 'p@ss&word'}); time.sleep(1)
    v = json.loads(S('window.__uiState.vaultEntries||[]'))
    check('vault add lists entry without password (P8-02)', len(v) == 1 and 'secret' not in json.dumps(v) and 'p@ss' not in json.dumps(v), v)
    vjson = [os.path.join(dp, 'vault.json') for dp, dn, fn in os.walk(prof) if 'vault.json' in fn]
    txt = open(vjson[0]).read() if vjson else ''
    check('vault.json does not contain the plaintext', bool(vjson) and 'p@ss&word' not in txt)
    bridge({'type': 'vault-reveal', 'id': v[0]['id']}); time.sleep(1)
    rv = json.loads(S('window.__uiState.revealedPassword||null') or 'null')
    check('vault reveal round-trips', rv and rv['password'] == 'p@ss&word', rv and rv.get('username'))
    bridge({'type': 'vault-delete', 'id': v[0]['id']}); time.sleep(1)
    check('vault delete', json.loads(S('(window.__uiState.vaultEntries||[]).length')) == 0)

    # shields toggle + extensions
    bridge({'type': 'get-shields-status'}); time.sleep(1)
    bridge({'type': 'get-extensions'}); time.sleep(1)
    ex = json.loads(S('window.__uiState.extensions||null'))
    print('extensions:', ex)
    check('extensions list returns (P6-01)', ex is not None)
    # pages
    for pg in ('settings', 'history', 'extensions'):
        bridge({'type': 'open', 'url': f'odyssey://{pg}'}); time.sleep(1)
        txt = ui('document.querySelector(".page-internal")?.innerText.slice(0,40)||""')
        check(f'odyssey://{pg} renders', bool(txt), txt)
    # expanded rail
    bridge({'type': 'set-rail-width', 'width': 264}); ui("window.__uiState.rail='expanded'; 1"); time.sleep(0.5)
    bridge({'type': 'show', 'view': json.loads(S('window.__uiState.tabs[0].id'))}); time.sleep(1)
    check('app still alive', p.poll() is None)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
