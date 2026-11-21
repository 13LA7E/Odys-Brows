"""AUDIT-38 and the settings storage (STORE): every engine through the real Settings control, the
custom engine, restore tabs vs new tab, link target, and junk in settings.json."""
import os, sys, subprocess
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

S = lambda e: ui('JSON.stringify(%s)' % e)


def settings_file(prof):
    return os.path.join(prof, 'Constellations', 'settings.json')


def start(prof, urls=None):
    args = []
    if urls:
        f = os.path.join(prof, 'urls.txt')
        open(f, 'w').write('\n'.join(urls) + '\n')
        args = ['--urls-file=' + f]
    p = launch(args, prof)
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    time.sleep(3)
    return p


def open_general():
    bridge({'type': 'open', 'url': 'odyssey://settings'})
    time.sleep(0.8)
    ui("[...document.querySelectorAll('.page-internal *')].filter(e=>e.children.length===0&&e.textContent==='General')[0].click(); 1")
    time.sleep(0.5)


def pick_engine(name):
    label = 'Custom…' if name == 'Custom' else name
    ui("document.querySelector('.settings-content [aria-label=\"Default search engine\"]').click(); 1")
    time.sleep(0.2)
    ui("[...document.querySelectorAll('.settings-content button')].find(b=>b.textContent===%s).click(); 1" % json.dumps(label))
    time.sleep(0.4)


def search(text):
    bridge({'type': 'open', 'url': ''})
    time.sleep(1)
    ui("document.querySelector('.helm-address').click(); 1")
    ui("(()=>{const i=document.querySelector('.helm-input'); i.value=%s; i.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true})); return 1})()" % json.dumps(text))
    return wait(lambda: (lambda u: u if u and u.startswith('http') and 'hello' in u else None)(ui("(window.__uiState.tabs.find(t=>t.id===window.__uiState.activeTab)||{}).url")), 8)


# ---------- junk in settings.json is cleaned, never crashes ----------
prof = tempfile.mkdtemp(prefix='cons-prof-')
os.makedirs(os.path.join(prof, 'Constellations'), exist_ok=True)
json.dump({"stars": "Sparkly", "accent": 7, "connections": 3, "searchEngine": "Nope", "startup": "NewTab", "evil": "<script>", "shootingStars": True}, open(settings_file(prof), 'w'))
p = start(prof)
try:
    st = json.loads(S('window.__uiState.settings'))
    check('invalid saved values load as defaults, valid ones are kept, unknown keys are dropped',
          st.get('stars') == 'Still' and st.get('accent') == 'Sea' and st.get('connections') == 8 and st.get('searchEngine') == 'DuckDuckGo'
          and st.get('startup') == 'Restore tabs' and st.get('shootingStars') is True and 'evil' not in st, st)
    bridge({'type': 'save-settings', 'settings': {"stars": "Drifting", "connections": 99, "searchEngine": "Custom", "customEngine": {"name": "x", "url": "javascript:alert(1)%s"}, "junk": 1}})
    time.sleep(1)
    disk = json.load(open(settings_file(prof)))
    check('saving junk writes a complete, valid file', disk['stars'] == 'Drifting' and disk['connections'] == 8 and disk['searchEngine'] == 'DuckDuckGo' and disk['customEngine']['url'] == '' and 'junk' not in disk and len(disk) >= 13, disk)
finally:
    kill(p)

# ---------- engines through the real control ----------
prof = tempfile.mkdtemp(prefix='cons-prof-')
p = start(prof)
try:
    expect = {
        'DuckDuckGo': 'https://duckduckgo.com/?q=hello', 'Google': 'https://www.google.com/search?q=hello', 'Brave': 'https://search.brave.com/search?q=hello',
        'Bing': 'https://www.bing.com/search?q=hello', 'Startpage': 'https://www.startpage.com/do/search?q=hello', 'Ecosia': 'https://www.ecosia.org/search?q=hello',
    }
    for name, prefix in expect.items():
        open_general()
        pick_engine(name)
        u = search('hello')
        check('engine %s is used by the Helm' % name, bool(u) and u.startswith(prefix), u)
    # custom engine
    open_general()
    pick_engine('Custom')
    wait(lambda: ui("document.querySelectorAll('.settings-content input').length") == 2, 5, 0.1)
    ui("(()=>{const i=[...document.querySelectorAll('.settings-content input')]; i[1].value='javascript:alert(1)%s'; i[1].dispatchEvent(new Event('input',{bubbles:true}))})()")
    note = wait(lambda: (lambda n: n if isinstance(n, str) and n.startswith('"Use http') else None)(S("[...document.querySelectorAll('.settings-content div')].map(d=>d.textContent).find(t=>t.startsWith('Use http'))")), 4, 0.2)
    check('a bad custom URL is refused with a message', isinstance(note, str) and 'Use http or https' in note, note)
    ui("(()=>{const i=[...document.querySelectorAll('.settings-content input')]; i[0].value='Mine'; i[0].dispatchEvent(new Event('input',{bubbles:true})); i[1].value='https://example.com/find?term=%s&x=1'; i[1].dispatchEvent(new Event('input',{bubbles:true}))})()")
    time.sleep(0.5)
    u = search('hello world')
    check('a custom engine with %s is used', bool(u) and u.startswith('https://example.com/find?term=hello%20world&x=1'), u)
    # link target
    open_general()
    ui("[...document.querySelectorAll('.settings-content button')].find(b=>b.textContent==='Same tab').click(); 1")
    time.sleep(0.4)
    check('link target setting is saved', json.loads(S('window.__uiState.settings.linkTarget')) == 'Same tab')
    disk = json.load(open(settings_file(prof)))
    check('...and written to settings.json', disk['linkTarget'] == 'Same tab' and disk['searchEngine'] == 'Custom' and disk['customEngine']['url'].startswith('https://example.com/find'), disk)
    # screenshot for the owner
    PS = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'capture_seq.ps1')
    ui("window.__uiState.settingsSection='General'; 1")
    open_general()
    subprocess.run(['powershell', '-ExecutionPolicy', 'Bypass', '-File', PS, '-Prefix', os.path.join(ROOT, 'out', 'live', 'settings-general'), '-Count', '1', '-IntervalMs', '10'], capture_output=True)
finally:
    kill(p)

# ---------- link target from History ----------
prof = tempfile.mkdtemp(prefix='cons-prof-')
os.makedirs(os.path.join(prof, 'Constellations'), exist_ok=True)
json.dump({"linkTarget": "Same tab"}, open(settings_file(prof), 'w'))
p = start(prof, ['http://127.0.0.1:%d/a.html' % SRV])
try:
    bridge({'type': 'open', 'url': 'odyssey://history'})
    time.sleep(1.5)
    n0 = len(json.loads(S('window.__uiState.tabs')))
    ui("(()=>{const r=document.querySelector('.history-row'); r.dispatchEvent(new MouseEvent('mouseover',{bubbles:true})); const b=[...r.querySelectorAll('span')].find(x=>x.textContent==='Open'); b.click()})()")
    time.sleep(2)
    tabs = json.loads(S('window.__uiState.tabs'))
    act = [t for t in tabs if t['id'] == json.loads(S('window.__uiState.activeTab'))][0]
    check('"Same tab": History opens the link in the current tab (no new tab)', len(tabs) == n0 and act['url'].startswith('http'), (n0, len(tabs), act['url']))
finally:
    kill(p)

# ---------- startup: restore tabs vs new tab ----------
prof = tempfile.mkdtemp(prefix='cons-prof-')
p = start(prof, ['http://127.0.0.1:%d/a.html' % SRV, 'http://127.0.0.1:%d/b.html' % SRV])
try:
    time.sleep(2)
finally:
    kill(p)
p = start(prof)
try:
    n = len(json.loads(S('window.__uiState.tabs')))
    check('startup "Restore tabs" brings the tabs back', n == 2, n)
    open_general()
    ui("[...document.querySelectorAll('.settings-content button')].find(b=>b.textContent==='New tab' && b.parentElement.children.length===2 && b.parentElement.textContent.startsWith('Restore')).click(); 1")
    time.sleep(0.6)
    disk = json.load(open(settings_file(prof)))
    check('"New tab" is saved', disk['startup'] == 'New tab', disk['startup'])
finally:
    kill(p)
p = start(prof)
try:
    n = len(json.loads(S('window.__uiState.tabs')))
    check('startup "New tab" does not restore the last tabs', n == 0, n)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
