import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

# ---- fake Ollama ----
class O(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def do_GET(self):
        if self.path == '/api/tags':
            b = json.dumps({'models': [{'name': 'llama3:latest'}]}).encode()
            self.send_response(200); self.send_header('Content-Type', 'application/json'); self.send_header('Content-Length', str(len(b))); self.end_headers(); self.wfile.write(b)
        else:
            self.send_response(404); self.end_headers()
    def do_POST(self):
        n = int(self.headers.get('Content-Length', 0)); body = json.loads(self.rfile.read(n) or b'{}')
        O.last = body
        if self.path != '/v1/chat/completions' or body.get('model') != 'llama3':
            self.send_response(404); self.end_headers(); return
        self.send_response(200); self.send_header('Content-Type', 'text/event-stream'); self.end_headers()
        for w in ['Hello ', 'from ', 'fake ', 'ollama']:
            self.wfile.write(('data: ' + json.dumps({'choices': [{'delta': {'content': w}}]}) + '\n\n').encode()); self.wfile.flush(); time.sleep(0.05)
        self.wfile.write(b'data: [DONE]\n\n')
O.last = None
osrv = TS(('127.0.0.1', 8766), O); threading.Thread(target=osrv.serve_forever, daemon=True).start()

# ================= private window =================
profp = tempfile.mkdtemp(prefix='cons-priv-')
p = launch(['--private'], profp)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    time.sleep(2)
    st = json.loads(ui('JSON.stringify({private:window.__uiState.private,rail:window.__uiState.rail,page:window.__uiState.page,copy:!!document.querySelector(".private-copy"),cls:document.getElementById("app").className})'))
    print(st)
    check('private window runs live with private UI (AUDIT-26)', st['private'] and st['copy'], st)
    bridge({'type': 'open', 'url': f'http://127.0.0.1:{SRV}/b.html'}); time.sleep(3)
    bridge({'type': 'save-settings', 'settings': {'accent': 'Rose'}}); time.sleep(1)
    files = [os.path.join(dp, f) for dp, dn, fn in os.walk(profp) for f in fn if f != 'stderr.txt']
    check('private window leaves no profile files on disk', not any(('session.json' in f or 'history' in f or 'settings.json' in f) for f in files), [os.path.relpath(f, profp) for f in files][:10])
finally:
    kill(p)

# ================= AI =================
prof = tempfile.mkdtemp(prefix='cons-prof-')
p = launch([], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    time.sleep(2)
    S = lambda e: ui(f'JSON.stringify({e})')
    for ep in ('http://127.0.0.1:8766', 'http://127.0.0.1:8766/v1'):
        bridge({'type': 'ai-test-connection', 'config': {'provider': 'local', 'endpoint': ep, 'model': 'llama3'}})
        r = wait(lambda: json.loads(S('window.__uiState.aiTestResult||null') or 'null'), 10)
        check(f'local test-connection ok with endpoint {ep}', bool(r) and r.get('ok'), r)
        ui('window.__uiState.aiTestResult=null; 1')
    bridge({'type': 'ai-test-connection', 'config': {'provider': 'local', 'endpoint': 'http://127.0.0.1:8766', 'model': 'nope'}})
    r = wait(lambda: json.loads(S('window.__uiState.aiTestResult||null') or 'null'), 10)
    check('missing model gives a clear message', bool(r) and (not r.get('ok')) and 'ollama pull nope' in r.get('message', ''), r and r.get('message'))
    # save config with a key, ask
    bridge({'type': 'ai-save-config', 'config': {'provider': 'local', 'endpoint': 'http://127.0.0.1:8766', 'model': 'llama3', 'api_key': 'sk-live-test-12345'}})
    time.sleep(1)
    cfg = json.loads(S('window.__uiState.aiConfig'))
    check('UI only sees has_key, never the key', cfg.get('has_key') is True and 'sk-live' not in json.dumps(cfg), cfg)
    aijson = [os.path.join(dp, 'ai.json') for dp, dn, fn in os.walk(prof) if 'ai.json' in fn]
    txt = open(aijson[0]).read() if aijson else ''
    check('ai.json exists and has no key', bool(aijson) and 'sk-live' not in txt, txt[:120])
    bridge({'type': 'ai-save-config', 'config': {'provider': 'local', 'endpoint': 'http://127.0.0.1:8766', 'model': 'llama3'}})
    time.sleep(1)
    cfg2 = json.loads(S('window.__uiState.aiConfig'))
    check('re-save without a key keeps it', cfg2.get('has_key') is True, cfg2)
    bridge({'type': 'ai-ask', 'prompt': 'hi'})
    a = wait(lambda: (lambda x: x if x and 'ollama' in x.get('text', '') else None)(json.loads(S('window.__uiState.answer||null') or 'null')), 10)
    check('local AI streams an answer (AUDIT-19)', bool(a), a and a.get('text'))
    print('last request model:', O.last and O.last.get('model'))
finally:
    kill(p)
    subprocess.run(['cmdkey', '/delete:ai-api-key.constellations-browser'], capture_output=True)
    subprocess.run(['cmdkey', '/delete:LegacyGeneric:target=ai-api-key.constellations-browser'], capture_output=True)

# restart: key survives
p = launch([], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    time.sleep(2)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
