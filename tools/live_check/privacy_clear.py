"""AUDIT-40: clear-on-exit really clears (cookies, local storage, history, closed tabs), only what was
asked for, after a clean exit and after a crash; with the switches off nothing is touched; and the
Global Privacy Control header is sent when it is on."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

S = lambda e: ui('JSON.stringify(%s)' % e)
PAGE = 'http://127.0.0.1:%d/a.html' % SRV


def start(prof):
    urls = os.path.join(prof, 'urls.txt')
    open(urls, 'w').write(PAGE + '\n')
    p = launch(['--urls-file=' + urls] if not os.path.exists(os.path.join(prof, 'Constellations', 'session.json')) else [], prof)
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    time.sleep(3)
    return p


def root(prof):
    return os.path.join(prof, 'Constellations')


def write_data(clear_history, clear_site):
    pw = wait(lambda: page_ws('/a.html'), 10)
    ev(pw, "localStorage.setItem('k','v'); document.cookie='c=1; max-age=86400; path=/'; 1")
    bridge({'type': 'save-settings', 'settings': {'clearHistoryOnExit': clear_history, 'clearSiteDataOnExit': clear_site}})
    time.sleep(1.5)
    bridge({'type': 'open', 'url': 'http://127.0.0.1:%d/b.html' % SRV})
    time.sleep(2)
    bridge({'type': 'close', 'view': json.loads(S('window.__uiState.activeTab'))})   # a closed tab for the closed list
    time.sleep(2.5)


def read_data():
    pw = wait(lambda: page_ws('/a.html'), 12)
    return json.loads(ev(pw, "JSON.stringify({ls: localStorage.getItem('k'), cookie: document.cookie})"))


def clean_exit(p):
    bridge({'type': 'close-window'})
    try:
        p.wait(timeout=40)
    except Exception:
        pass
    subprocess.run(['taskkill', '/F', '/IM', 'constellations.exe'], capture_output=True)
    time.sleep(2)


def history_rows(prof):
    import sqlite3
    f = os.path.join(root(prof), 'history.db')
    if not os.path.exists(f):
        return 0
    try:
        return sqlite3.connect(f).execute('select count(*) from visits').fetchone()[0]
    except Exception:
        return -1


def closed_count(prof):
    try:
        return len(json.load(open(os.path.join(root(prof), 'session.json'))).get('closed', []))
    except Exception:
        return 0


# ---------- control: switches off, nothing is touched ----------
prof = tempfile.mkdtemp(prefix='cons-prof-')
p = start(prof)
write_data(False, False)
clean_exit(p)
check('switches off: history stays', history_rows(prof) >= 2, history_rows(prof))
check('switches off: closed tabs stay', closed_count(prof) >= 1, closed_count(prof))
check('switches off: the lock file is removed after a clean exit', not os.path.exists(os.path.join(root(prof), 'running.lock')))
p = start(prof)
d = read_data()
check('switches off: cookie and local storage survive a restart', d == {'ls': 'v', 'cookie': 'c=1'}, d)
kill(p)

# ---------- clean exit with both on ----------
prof = tempfile.mkdtemp(prefix='cons-prof-')
p = start(prof)
write_data(True, True)
clean_exit(p)
check('on: history database is gone after closing', history_rows(prof) == 0, history_rows(prof))
check('on: closed-tabs list is gone', closed_count(prof) == 0, closed_count(prof))
check('on: settings.json is kept', os.path.exists(os.path.join(root(prof), 'settings.json')))
check('on: Chromium cookie database is gone', not os.path.exists(os.path.join(root(prof), 'Default', 'Network', 'Cookies')))
p = start(prof)
d = read_data()
check('on: cookie and local storage are gone after a restart', d == {'ls': None, 'cookie': ''}, d)
check('on: the open tab is still restored (only history and site data were cleared)', len(json.loads(S('window.__uiState.tabs'))) >= 1)
kill(p)

# ---------- only site data on ----------
prof = tempfile.mkdtemp(prefix='cons-prof-')
p = start(prof)
write_data(False, True)
clean_exit(p)
check('site data only: history stays', history_rows(prof) >= 2, history_rows(prof))
p = start(prof)
d = read_data()
check('site data only: cookie and storage are cleared', d == {'ls': None, 'cookie': ''}, d)
kill(p)

# ---------- crash (killed, no clean exit): cleared at the next start ----------
prof = tempfile.mkdtemp(prefix='cons-prof-')
p = start(prof)
write_data(True, True)
subprocess.run(['taskkill', '/F', '/IM', 'constellations.exe'], capture_output=True)
time.sleep(2)
check('after a crash the lock file is left behind', os.path.exists(os.path.join(root(prof), 'running.lock')))
p = start(prof)
d = read_data()
check('after a crash the next start clears site data', d == {'ls': None, 'cookie': ''}, d)
check('...and history', history_rows(prof) <= 1, history_rows(prof))   # the page just opened may add one visit
kill(p)

# ---------- Global Privacy Control header ----------
seen = []


class H(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a): pass

    def do_GET(self):
        seen.append((self.path, self.headers.get('Sec-GPC')))
        b = b'<!doctype html><title>gpc</title>'
        self.send_response(200); self.send_header('Content-Type', 'text/html'); self.send_header('Content-Length', str(len(b))); self.end_headers(); self.wfile.write(b)


gs = TS(('127.0.0.1', 8769), H)
threading.Thread(target=gs.serve_forever, daemon=True).start()
prof = tempfile.mkdtemp(prefix='cons-prof-')
os.makedirs(root(prof), exist_ok=True)
p = launch([], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    time.sleep(3)
    bridge({'type': 'open', 'url': 'http://127.0.0.1:8769/on'})
    time.sleep(2.5)
    check('GPC on (default): Sec-GPC: 1 is sent', ('/on', '1') in seen, seen)
    bridge({'type': 'save-settings', 'settings': {'globalPrivacyControl': False}})
    time.sleep(0.8)
    bridge({'type': 'open', 'url': 'http://127.0.0.1:8769/off'})
    time.sleep(2.5)
    check('GPC off: the header is not sent', ('/off', None) in seen, seen)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
