import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *
prof = tempfile.mkdtemp(prefix='cons-prof-')
open(os.path.join(prof, 'urls.txt'), 'w').write(f'http://127.0.0.1:{SRV}/a.html\nhttp://127.0.0.1:{SRV}/b.html\n')
p = launch([f'--urls-file={os.path.join(prof, "urls.txt")}'], prof)
S = lambda e: ui(f'JSON.stringify({e})')
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(4)
    tabs = json.loads(S('window.__uiState.tabs')); a = [t for t in tabs if 'a.html' in t['url']][0]['id']; b = [t for t in tabs if 'b.html' in t['url']][0]['id']
    bridge({'type': 'show', 'view': a}); time.sleep(1.5)
    ui("window.__tag = {rail: document.querySelector('nav.rail'), helm: document.querySelector('.helm'), notch: document.querySelector('.notch'), sky: document.querySelector('canvas.sky'), panel: document.querySelector('section.page')}; for (const k in window.__tag) window.__tag[k] && (window.__tag[k].__t = k); 1")
    r0 = ui('window.__renderCount')
    # 50 unrelated updates: titles of the OTHER tab change (rail data) and shields counts change (notch only)
    for i in range(25):
        ev(page_ws('/b.html'), f"document.title='T{i}'")
    for i in range(25):
        ev(page_ws('/b.html'), f"document.title='V{i}'")
    time.sleep(1)
    same = json.loads(ui("JSON.stringify(Object.fromEntries(Object.entries(window.__tag).map(([k,v]) => [k, !!v && v.isConnected && document.querySelector(k==='rail'?'nav.rail':k==='sky'?'canvas.sky':k==='panel'?'section.page':'.'+k) === v])))"))
    print('same nodes still in the DOM:', same, '| renders:', ui('window.__renderCount') - r0)
    check('Helm node is never rebuilt by unrelated updates (AUDIT-03)', same['helm'])
    check('sky canvas persists', same['sky'])
    check('notch persists when only another tab changes', same['notch'])
    check('rail IS rebuilt when a tab title changes (data changed)', not same['rail'])
    check('page panel persists across unrelated updates', same['panel'])
    # typing in the Helm survives
    ui("document.querySelector('.helm-address').click(); const i=document.querySelector('.helm-input'); i.value='draft-text'; 1")
    for i in range(10): ev(page_ws('/b.html'), f"document.title='U{i}'")
    time.sleep(0.8)
    v = ui("(document.querySelector('.helm-input')||{}).value")
    check('text typed in the Helm survives page events', v == 'draft-text', v)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
