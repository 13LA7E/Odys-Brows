"""Assistant page control: it reads a real page, types, clicks, selects and scrolls; asks before a
risky click; never fills a password; lists and switches tabs. A scripted fake model drives it."""
import os, sys, re
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

S = lambda e: ui('JSON.stringify(%s)' % e)
V = lambda e: json.loads(S(e))

open(os.path.join(site, 'shop.html'), 'w').write('''<!doctype html><title>Star Shop</title>
<h1>Star Shop</h1>
<label for=q>Search</label><input id=q type=text>
<button id=go onclick="document.getElementById('res').textContent='results for '+document.getElementById('q').value">Search</button>
<select id=size aria-label="Size"><option>Small</option><option>Large</option></select>
<input id=pw type=password aria-label="Password">
<button id=order onclick="window.ordered=(window.ordered||0)+1;document.getElementById('res').textContent='ORDERED'">Place order</button>
<a href="/b.html">More</a>
<p id=res>nothing yet</p><div style="height:3000px"></div>''')

requests_seen = []
script = {'name': None}


def refs(text):
    """ref number by label from the last snapshot in the conversation"""
    out = {}
    for m in re.finditer(r'\[(\d+)\] ([a-z ]+) "([^"]*)"', text):
        out.setdefault(m.group(3), int(m.group(1)))
    return out


class M(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a): pass

    def do_POST(self):
        n = int(self.headers.get('Content-Length', 0)); body = json.loads(self.rfile.read(n) or b'{}')
        requests_seen.append(body)
        msgs = body['messages']
        first_user = next(m['content'] for m in msgs if m['role'] == 'user')
        results = [m['content'] for m in msgs if m['role'] == 'user' and m['content'].startswith('Tool result')]
        step = len(results)
        last = results[-1] if results else ''
        r = refs(last)
        j = lambda **k: json.dumps(k)
        if 'shop run' in first_user:
            plan = [
                lambda: j(tool='snapshot'),
                lambda: j(tool='type', ref=r['Search'], text='star chart'),
                lambda: j(tool='select', ref=r['Size'], value='Large'),
                lambda: j(tool='click', ref=r['Search'] + 1),
                lambda: j(tool='type', ref=r['Password'], text='hunter2'),
                lambda: j(tool='scroll', direction='down'),
                lambda: j(tool='snapshot'),
                lambda: j(tool='click', ref=r['Place order']),
                lambda: 'All done.',
            ]
        elif 'tab run' in first_user:
            plan = [lambda: j(tool='list_tabs'), lambda: j(tool='new_tab', url='https://example.com/x'), lambda: 'Tabs handled.']
        else:
            plan = [lambda: 'ok']
        reply = plan[min(step, len(plan) - 1)]()
        self.send_response(200); self.send_header('Content-Type', 'text/event-stream'); self.end_headers()
        for piece in [reply[:len(reply) // 2], reply[len(reply) // 2:]]:
            self.wfile.write(('data: ' + json.dumps({'choices': [{'delta': {'content': piece}}]}) + '\n\n').encode()); self.wfile.flush()
        self.wfile.write(b'data: [DONE]\n\n')


msrv = TS(('127.0.0.1', 8773), M); threading.Thread(target=msrv.serve_forever, daemon=True).start()
SHOP = 'http://127.0.0.1:%d/shop.html' % SRV


def page(expr): return ev(wait(lambda: page_ws('/shop.html'), 10), expr)


def ask(text):
    ui("(()=>{const i=document.querySelector('.helm input'); i.value=%s; i.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true})); return 1})()" % json.dumps(text))


def start(prof, urls):
    f = os.path.join(prof, 'urls.txt'); open(f, 'w').write('\n'.join(urls) + '\n')
    p = launch(['--urls-file=' + f], prof)
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(4)
    bridge({'type': 'ai-save-config', 'config': {'provider': 'local', 'endpoint': 'http://127.0.0.1:8773', 'model': 'mock'}}); time.sleep(1)
    bridge({'type': 'ai-get-config'}); time.sleep(0.5)
    bridge({'type': 'run-command', 'id': 'ask'}); time.sleep(0.6)
    return p


prof = tempfile.mkdtemp(prefix='cons-prof-')
p = start(prof, [SHOP])
try:
    ask('shop run please')
    # it works until the risky click asks you
    asked = wait(lambda: V("(window.__uiState.prompts||[]).find(p=>p.kind==='agent-confirm')||null"), 40)
    check('the assistant asks before clicking "Place order"', bool(asked) and 'Place order' in asked['text'], asked)
    check('the question shows on screen', V("!!document.querySelector('.permission-card')") and 'Place order' in V("document.querySelector('.permission-card').textContent"))
    PS0 = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'capture_seq.ps1')
    subprocess.run(['powershell', '-ExecutionPolicy', 'Bypass', '-File', PS0, '-Prefix', os.path.join(ROOT, 'out', 'live', 'agent-confirm'), '-Count', '1', '-IntervalMs', '10'], capture_output=True)
    st = page("JSON.stringify({q: document.getElementById('q').value, size: document.getElementById('size').value, res: document.getElementById('res').textContent, pw: document.getElementById('pw').value, y: Math.round(scrollY), ordered: window.ordered||0})")
    st = json.loads(st)
    check('it typed into the search box', st['q'] == 'star chart', st)
    check('it chose an option in the menu', st['size'] == 'Large', st)
    check('it clicked Search (the page shows the result)', st['res'].startswith('results for star chart'), st)
    check('it never typed the password', st['pw'] == '', st)
    check('it scrolled', st['y'] > 100, st)
    check('the risky click has not happened yet', st['ordered'] == 0, st)
    # answer No
    ui("[...document.querySelectorAll('.permission-card button')].find(b=>b.textContent==='No').click(); 1")
    wait(lambda: V("!!document.querySelector('.ans-footer')"), 30)
    st = json.loads(page("JSON.stringify({ordered: window.ordered||0})"))
    check('No: the order was not placed', st['ordered'] == 0, st)
    steps = V("window.__uiState.answer.steps")
    check('the steps are listed, including the refusal to type a password',
          any('Search' in s for s in steps) and any('Scrolling' in s for s in steps), steps)
    pw_result = [m['content'] for m in requests_seen[-1]['messages'] if 'password' in m['content'] and m['role'] == 'user']
    check('the model was told it cannot fill passwords', any('never fills' in c or 'never types' in c for c in pw_result), pw_result[:1])
    no_result = [m['content'] for m in requests_seen[-1]['messages'] if 'user said no' in m['content']]
    check('the model was told the user said no', bool(no_result))

    # again, this time Yes
    bridge({'type': 'run-command', 'id': 'ask'}); time.sleep(0.4)
    bridge({'type': 'run-command', 'id': 'ask'}); time.sleep(0.6)
    ask('shop run again')
    asked = wait(lambda: V("(window.__uiState.prompts||[]).find(p=>p.kind==='agent-confirm')||null"), 40)
    check('it asks again', bool(asked))
    ui("[...document.querySelectorAll('.permission-card button')].find(b=>b.textContent==='Yes, do it').click(); 1")
    wait(lambda: V("!!document.querySelector('.ans-footer')"), 30)
    st = json.loads(page("JSON.stringify({ordered: window.ordered||0, res: document.getElementById('res').textContent})"))
    check('Yes: the click went through once', st['ordered'] == 1, st)

    # tabs
    bridge({'type': 'run-command', 'id': 'ask'}); time.sleep(0.4)
    bridge({'type': 'run-command', 'id': 'ask'}); time.sleep(0.6)
    n0 = V('window.__uiState.tabs.length')
    ask('tab run')
    wait(lambda: V("!!document.querySelector('.ans-footer')"), 30)
    tl = [m['content'] for m in requests_seen[-1]['messages'] if m['role'] == 'user' and 'Star Shop' in m['content'] and m['content'].startswith('Tool result') and 'id ' in m['content']]
    check('list_tabs shows your tabs with ids', bool(tl), tl[:1])
    urls = V("window.__uiState.tabs.map(t=>t.url)")
    check('new_tab opened the page', any('example.com/x' in (u or '') for u in urls) and V('window.__uiState.tabs.length') == n0 + 1, urls)
    PS = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'capture_seq.ps1')
finally:
    kill(p)

# off switch: the assistant is not even told about control tools
prof = tempfile.mkdtemp(prefix='cons-prof-')
os.makedirs(os.path.join(prof, 'Constellations'), exist_ok=True)
json.dump({'aiControl': False}, open(os.path.join(prof, 'Constellations', 'settings.json'), 'w'))
requests_seen.clear()
p = start(prof, [SHOP])
try:
    ask('shop run please'); time.sleep(5)
    sysmsg = requests_seen[0]['messages'][0]['content'] if requests_seen else ''
    check('with Control the page off the model is not offered page tools', bool(requests_seen) and 'snapshot' not in sysmsg and 'open_tab' in sysmsg, sysmsg[:80])
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
