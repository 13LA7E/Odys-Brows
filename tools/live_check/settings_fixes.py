"""Search engine setting is used by the Helm; the Passwords page scrolls; Reveal and Export wait for
Windows verification. The real Windows Hello / password dialog can't be clicked by a script, so
debug builds honour CONSTELLATIONS_TEST_AUTH=allow|deny (release builds never do)."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

S = lambda e: ui('JSON.stringify(%s)' % e)


def start(auth=None):
    if auth:
        os.environ['CONSTELLATIONS_TEST_AUTH'] = auth
    else:
        os.environ.pop('CONSTELLATIONS_TEST_AUTH', None)
    prof = tempfile.mkdtemp(prefix='cons-prof-')
    p = launch([], prof)
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    time.sleep(3)
    return p


def type_in_helm(text):
    ui("document.querySelector('.helm-address').click(); 1")
    ui("(()=>{const i=document.querySelector('.helm-input'); i.value=%s; i.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true})); return 1})()" % json.dumps(text))


def open_passwords():
    bridge({'type': 'open', 'url': 'odyssey://settings'})
    time.sleep(1)
    ui("[...document.querySelectorAll('.page-internal *')].filter(e=>e.children.length===0&&e.textContent==='Passwords')[0].click(); 1")
    time.sleep(0.8)


# ---------- search engine ----------
p = start()
try:
    for engine, prefix in (('Google', 'https://www.google.com/search?q=hello%20world'), ('Bing', 'https://www.bing.com/search?q=hello%20world'), ('DuckDuckGo', 'https://duckduckgo.com/?q=hello%20world')):
        # the real Settings control, then the real Helm
        bridge({'type': 'open', 'url': 'odyssey://settings'})
        time.sleep(0.8)
        ui("[...document.querySelectorAll('.page-internal *')].filter(e=>e.children.length===0&&e.textContent==='General')[0].click(); 1")
        time.sleep(0.5)
        ui("[...document.querySelectorAll('.settings-content button')].find(b=>b.textContent===%s).click(); 1" % json.dumps(engine))
        time.sleep(0.5)
        bridge({'type': 'open', 'url': ''})
        time.sleep(1)
        type_in_helm('hello world')
        url = wait(lambda: (lambda u: u if u and u.startswith('http') and 'hello' in u else None)(ui("(window.__uiState.tabs.find(t=>t.id===window.__uiState.activeTab)||{}).url")), 8)
        check('searching with %s in Settings goes to %s' % (engine, engine), bool(url) and url.startswith(prefix), url)
finally:
    kill(p)

# ---------- scrolling + verification ----------
rows = 'name,url,username,password,note\r\n' + ''.join('site%d.example,https://site%d.example/,user%d,pw%d,\r\n' % (i, i, i, i) for i in range(40))
p = start('deny')
try:
    bridge({'type': 'vault-import', 'csv': rows})
    time.sleep(1.5)
    open_passwords()
    sc = json.loads(S("(()=>{const c=document.querySelector('.settings-content');return {sh:c.scrollHeight,ch:c.clientHeight,ov:getComputedStyle(c).overflowY}})()"))
    check('Passwords content is taller than the window and scrollable (was clipped)', sc['sh'] > sc['ch'] + 100 and sc['ov'] == 'auto', sc)
    # wheel scroll with real wheel events over the content
    s = cdp.ws_connect(ui_ws())
    cdp.send(s, {"id": 1, "method": "Input.dispatchMouseEvent", "params": {"type": "mouseWheel", "x": 700, "y": 400, "deltaX": 0, "deltaY": 600}})
    while cdp.recv(s).get('id') != 1:
        pass
    s.close()
    time.sleep(1.5)
    top = ui("document.querySelector('.settings-content').scrollTop")
    check('mouse wheel scrolls it', top > 100, top)
    ui("document.querySelector('.settings-content').scrollTop=0; 1")

    # Reveal denied -> no password in the UI
    ui("[...document.querySelectorAll('.settings-content button')].find(b=>b.textContent==='Reveal').click(); 1")
    time.sleep(1.5)
    dump = ui('JSON.stringify(window.__uiState)')
    msg = json.loads(S('window.__uiState.vaultAuth'))
    check('Reveal without Windows verification shows no password', msg and msg['message'] and 'pw0' not in dump and 'pw1' not in dump, msg)
    # Export denied -> no file
    before = set(os.listdir(os.path.join(os.environ['USERPROFILE'], 'Downloads')))
    bridge({'type': 'vault-export'})
    time.sleep(1.5)
    ex = json.loads(S('window.__uiState.vaultExport'))
    after = set(os.listdir(os.path.join(os.environ['USERPROFILE'], 'Downloads')))
    check('Export without verification writes nothing', ex and ex['ok'] is False and not [f for f in after - before if f.startswith('constellations-passwords')], ex)
finally:
    kill(p)

p = start('allow')
try:
    bridge({'type': 'vault-import', 'csv': rows})
    time.sleep(1.5)
    open_passwords()
    ui("[...document.querySelectorAll('.settings-content button')].find(b=>b.textContent==='Reveal').click(); 1")
    time.sleep(1.5)
    rp = json.loads(S('window.__uiState.revealedPassword'))
    check('Reveal after verification shows the password', bool(rp) and rp['password'].startswith('pw'), rp and rp['username'])
    check('...and the button now says Hide', ui("[...document.querySelectorAll('.settings-content button')].some(b=>b.textContent==='Hide')") is True)
    ui("[...document.querySelectorAll('.settings-content button')].find(b=>b.textContent==='Hide').click(); 1")
    time.sleep(0.5)
    check('Hide removes it again', json.loads(S('window.__uiState.revealedPassword')) is None)
    bridge({'type': 'vault-export'})
    time.sleep(1.5)
    ex = json.loads(S('window.__uiState.vaultExport'))
    check('Export after verification writes the file', ex and ex['ok'] and os.path.exists(ex['path']), ex)
    try:
        os.remove(ex['path'])
    except Exception:
        pass
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
