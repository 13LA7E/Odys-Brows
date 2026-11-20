import json, os, subprocess, sys, time, threading, http.server, socketserver, urllib.request, shutil, tempfile
sys.path.insert(0, os.path.dirname(__file__))
import cdp

ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
EXE = os.path.join(ROOT, 'target', 'debug', 'constellations.exe')
PORT = 9333
SRV = 8765

# ---- local site -------------------------------------------------------------------
site = tempfile.mkdtemp(prefix='cons-site-')
open(os.path.join(site, 'a.html'), 'w').write('''<!doctype html><title>Page A</title>
<a id=blank href="/b.html" target=_blank>b</a>
<button id=wopen onclick="window.open('/c.html')">open</button>
<script>setTimeout(()=>{ try{window.open('/spam.html')}catch(e){} }, 1500)</script>
<p>Findme findme FINDME</p><div style="height:2000px"></div>''')
for n in ('b', 'c', 'spam'):
    open(os.path.join(site, n + '.html'), 'w').write(f'<!doctype html><title>Page {n.upper()}</title><p>{n}</p>')
with open(os.path.join(site, 'big.bin'), 'wb') as f:
    f.truncate(120 * 1024 * 1024)

class H(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *a, **k): super().__init__(*a, directory=site, **k)
    def log_message(self, *a): pass
    def copyfile(self, src, dst):  # throttle the big file so progress is observable
        if self.path.endswith('big.bin'):
            try:
                while True:
                    b = src.read(256 * 1024)
                    if not b: break
                    dst.write(b); time.sleep(0.02)
            except Exception: pass
        else:
            super().copyfile(src, dst)
class TS(socketserver.ThreadingMixIn, http.server.HTTPServer): daemon_threads = True
srv = TS(('127.0.0.1', SRV), H); threading.Thread(target=srv.serve_forever, daemon=True).start()

# ---- helpers -----------------------------------------------------------------------
results = []
def check(name, ok, info=''):
    results.append((name, ok, info)); print(('PASS ' if ok else 'FAIL ') + name + (' :: ' + str(info) if info else ''), flush=True)

def targets():
    return json.load(urllib.request.urlopen(f'http://127.0.0.1:{PORT}/json'))
def ui_ws():
    for t in targets():
        if 'odyssey-ui' in t.get('url', ''): return t['webSocketDebuggerUrl']
def page_ws(sub):
    for t in targets():
        if t.get('type') == 'page' and sub in t.get('url', '') and 'odyssey-ui' not in t.get('url', ''): return t['webSocketDebuggerUrl']
def ev(ws, expr, gesture=False):
    s = cdp.ws_connect(ws)
    cdp.send(s, {"id": 1, "method": "Runtime.evaluate", "params": {"expression": expr, "awaitPromise": True, "returnByValue": True, "userGesture": gesture}})
    while True:
        m = cdp.recv(s)
        if m.get('id') == 1:
            s.close(); r = m.get('result', {}); return r.get('result', {}).get('value', r)
def ui(expr): return ev(ui_ws(), expr)
def bridge(obj): return ui(f"window.odysseyBridge({json.dumps(json.dumps(obj))}); 1")
def wait(fn, t=15, step=0.5):
    end = time.time() + t
    while time.time() < end:
        try:
            v = fn()
            if v: return v
        except Exception: pass
        time.sleep(step)
    return None

def launch(args, profile):
    env = dict(os.environ); env['LOCALAPPDATA'] = profile
    libdir = None
    for dp, dn, fn in os.walk(os.path.join(ROOT, 'target', 'debug', 'build')):
        if 'libcef.dll' in fn and 'cef_windows' in dp: libdir = dp; break
    env['PATH'] = libdir + os.pathsep + env['PATH']
    return subprocess.Popen([EXE, f'--remote-debugging-port={PORT}'] + args, env=env, stdout=subprocess.DEVNULL, stderr=open(os.path.join(profile, 'stderr.txt'), 'w'))

def kill(p):
    p.kill(); subprocess.run(['taskkill', '/F', '/IM', 'constellations.exe'], capture_output=True); time.sleep(2)

STATE = "(()=>{const t=[...document.querySelectorAll('*')];return 0})()"
