import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

prof = tempfile.mkdtemp(prefix='cons-prof-')
def start(args=()):
    p = launch(list(args), prof)
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    time.sleep(3)
    return p
S = lambda e: ui(f'JSON.stringify({e})')

p = start()
try:
    bridge({'type': 'open', 'url': f'http://127.0.0.1:{SRV}/b.html'}); time.sleep(2)
    bridge({'type': 'open', 'url': 'odyssey://settings'}); time.sleep(1)
    bridge({'type': 'open', 'url': f'http://127.0.0.1:{SRV}/c.html'}); time.sleep(2)
    bridge({'type': 'save-settings', 'settings': {'stars': 'Drifting', 'shootingStars': True, 'accent': 'Rose', 'trail': 'Wide', 'connections': 12}}); time.sleep(1)
    # Settings page UI controls: click Appearance accent swatches etc. exist?
    bridge({'type': 'show', 'view': json.loads(S('window.__uiState.tabs.find(t=>t.url=="odyssey://settings").id'))}); time.sleep(1)
    controls = json.loads(ui('JSON.stringify({buttons:document.querySelectorAll(".page-internal button").length, inputs:document.querySelectorAll(".page-internal input").length, text:document.querySelector(".page-internal")?.innerText.slice(0,200)})'))
    print('settings page:', controls)
    check('Settings page renders controls', controls['buttons'] > 3, controls['buttons'])
    bridge({'type': 'show', 'view': json.loads(S('window.__uiState.tabs.find(t=>t.url.includes("c.html")).id'))}); time.sleep(2)
finally:
    kill(p)

p = start()
try:
    s = json.loads(S('{tabs:window.__uiState.tabs.map(t=>({id:t.id,url:t.url,title:t.title})),settings:window.__uiState.settings,active:window.__uiState.activeTab,page:window.__uiState.page}'))
    print(s)
    check('settings survive restart (P2-07)', s['settings'] and s['settings'].get('accent') == 'Rose' and s['settings'].get('stars') == 'Drifting', s['settings'])
    urls = [t['url'] for t in s['tabs']]
    check('tabs restored after restart (P4-05)', any('b.html' in (u or '') for u in urls) and any('c.html' in (u or '') for u in urls), urls)
    check('internal page restored as internal (AUDIT-18)', 'odyssey://settings' in urls, urls)
    time.sleep(3)
    ws = len([t for t in targets() if t.get('type') == 'page' and t.get('url','').startswith('http')])
    check('UI active tab matches shown tab after restore', s['active'] == 3 and s['page'] == 'web', (s['active'], s['page']))
    print('engine page targets after restore:', ws, 'of', len(urls), 'tabs')
    check('restore is lazy: only the active tab loaded (EDGE_CASES 6)', ws == 1, ws)
    bidx = [t for t in s['tabs'] if 'b.html' in t['url']][0]['id']
    bridge({'type': 'show', 'view': bidx}); time.sleep(3)
    ws2 = [t['url'] for t in targets() if t.get('type') == 'page' and t.get('url','').startswith('http')]
    check('showing a restored tab loads it', any('b.html' in u for u in ws2), ws2)
    t0 = json.loads(S('window.__uiState.tabs.find(t=>t.id==%d)' % bidx))
    check('restored tab keeps URL and title in UI', 'b.html' in t0['url'] and t0['title'], t0)
    accent = ui("getComputedStyle(document.documentElement).getPropertyValue('--accent')")
    print('accent css var:', accent)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
