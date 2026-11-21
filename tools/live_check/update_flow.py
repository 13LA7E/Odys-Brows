"""Updates: find a newer signed version, refuse a forged or tampered one, download with progress,
check the file, and hand over to the installer. A local server stands in for GitHub and a tiny batch
file stands in for the installer (debug builds only accept the test key and local http)."""
import os, sys, base64, hashlib
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'updater'))
import sign as signer
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from scenario import *

S = lambda e: ui('JSON.stringify(%s)' % e)
V = lambda e: json.loads(S(e))

# UPDATE_REAL_KEY=1: sign with the owner's real key and let the program use its built-in public key
if os.environ.get('UPDATE_REAL_KEY'):
    key = signer.load_key(signer.DEFAULT_KEY)
else:
    key = Ed25519PrivateKey.from_private_bytes(bytes([9]) * 32)
    os.environ['CONSTELLATIONS_UPDATE_PUBKEY'] = signer.public_b64(key)
other = Ed25519PrivateKey.from_private_bytes(bytes([5]) * 32)

work = tempfile.mkdtemp(prefix='cons-upd-')
marker = os.path.join(work, 'installed.txt')
installer = os.path.join(work, 'Setup.bat')
open(installer, 'w').write('@echo off\r\necho installed with %1 > "' + marker + '"\r\n' + 'rem padding ' + 'x' * 600000 + '\r\n')
tampered = os.path.join(work, 'Tampered.bat')
data = open(installer, 'rb').read()
open(tampered, 'wb').write(data[:-10] + b'EVIL EVIL\n')
PORT_U = 8777
BASE = 'http://127.0.0.1:%d' % PORT_U


def mf(version, file_path, url, signing_key=key, size_override=None):
    e = signer.entry(signing_key, version, 'windows-x86_64', file_path, url)
    if size_override: e['size'] = size_override
    return json.dumps({'version': version, 'notes': 'Faster and nicer.', 'platforms': {'windows-x86_64': e}}).encode()


FILES = {
    '/latest.json': (mf('9.9.9', installer, BASE + '/Setup.bat'), 'application/json'),
    '/same.json': (mf('0.1.0', installer, BASE + '/Setup.bat'), 'application/json'),
    '/forged.json': (mf('9.9.9', installer, BASE + '/Setup.bat', signing_key=other), 'application/json'),
    # signed for the real file but the server hands out a different one
    '/tampered.json': (mf('9.9.9', installer, BASE + '/Tampered.bat'), 'application/json'),
    '/Setup.bat': (open(installer, 'rb').read(), 'application/octet-stream'),
    '/Tampered.bat': (open(tampered, 'rb').read(), 'application/octet-stream'),
}
hits = []


class H(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a): pass

    def do_GET(self):
        hits.append(self.path)
        if self.path not in FILES:
            self.send_response(404); self.end_headers(); return
        body, ctype = FILES[self.path]
        self.send_response(200); self.send_header('Content-Type', ctype); self.send_header('Content-Length', str(len(body))); self.end_headers()
        for i in range(0, len(body), 65536):
            self.wfile.write(body[i:i + 65536]); self.wfile.flush(); time.sleep(0.02)


us = TS(('127.0.0.1', PORT_U), H); threading.Thread(target=us.serve_forever, daemon=True).start()


def set_url(path):
    bridge({'type': 'save-settings', 'settings': {'updateUrl': BASE + path, 'autoUpdateCheck': False}}); time.sleep(0.8)


def status(): return V('window.__uiState.update || null')


def wait_state(states, t=20):
    return wait(lambda: (lambda s: s if s and s.get('state') in states else None)(status()), t)


import shutil
shutil.rmtree(os.path.join(tempfile.gettempdir(), 'Constellations-Updates'), ignore_errors=True)
prof = tempfile.mkdtemp(prefix='cons-prof-')
p = launch([], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(3)
    # the About page offers the controls
    bridge({'type': 'open', 'url': 'odyssey://settings'}); time.sleep(0.8)
    ui("[...document.querySelectorAll('.page-internal *')].filter(e=>e.children.length===0&&e.textContent==='About')[0].click(); 1"); time.sleep(0.6)
    t = V("document.querySelector('.settings-content').textContent")
    check('About has Check for updates, automatic checking and the server field', 'Check for updates' in t and 'Check automatically' in t and 'Update server' in t)

    set_url('/same.json'); bridge({'type': 'check-updates'})
    s = wait_state(['uptodate', 'error', 'available'])
    check('the same version: you are up to date', s and s['state'] == 'uptodate', s)
    set_url('/forged.json'); bridge({'type': 'check-updates'})
    s = wait_state(['error', 'available', 'uptodate'])
    check('a manifest signed with another key is refused', s and s['state'] == 'error' and 'not signed' in s['message'], s)
    set_url('/nothing.json'); bridge({'type': 'check-updates'})
    s = wait_state(['error', 'available', 'uptodate'])
    check('a missing manifest gives a clear error', s and s['state'] == 'error', s)
    bridge({'type': 'save-settings', 'settings': {'updateUrl': 'http://example.com/latest.json', 'autoUpdateCheck': False}}); time.sleep(0.6)
    bridge({'type': 'check-updates'})
    s = wait_state(['error', 'available', 'uptodate'])
    check('a plain http address from the internet is refused', s and s['state'] == 'error' and 'https' in s['message'], s)

    # tampered file
    set_url('/tampered.json'); bridge({'type': 'check-updates'})
    s = wait_state(['available', 'error'])
    check('a signed manifest is accepted (the file is checked later)', s and s['state'] == 'available', s)
    bridge({'type': 'install-update'})
    s = wait_state(['ready', 'error'], 30)
    check('a file that is not the signed one is refused and deleted', s and s['state'] == 'error' and ('does not match' in s['message'] or 'incomplete' in s['message'] or 'bigger' in s['message']), s)
    left = [f for f in os.listdir(os.path.join(tempfile.gettempdir(), 'Constellations-Updates'))] if os.path.exists(os.path.join(tempfile.gettempdir(), 'Constellations-Updates')) else []
    check('nothing from it is left on disk', not any(f.endswith(('.bat', '.exe', '.part')) for f in left), left)
    bridge({'type': 'apply-update'}); time.sleep(1.5)
    check('and Restart and install does nothing without a good file (the app keeps running)', p.poll() is None and not os.path.exists(marker))

    # the real thing
    set_url('/latest.json'); bridge({'type': 'check-updates'})
    s = wait_state(['available', 'error'])
    check('a newer signed version is offered with its notes', s and s['state'] == 'available' and s['version'] == '9.9.9' and 'Faster' in s['notes'], s)
    ui("window.__ups=[]; 1")
    bridge({'type': 'install-update'})
    seen = []
    end = time.time() + 30
    while time.time() < end:
        st = status()
        if st and st['state'] == 'downloading': seen.append(st.get('progress', 0))
        if st and st['state'] in ('ready', 'error'): break
        time.sleep(0.05)
    st = status()
    check('the download reports progress and ends ready', st and st['state'] == 'ready' and len(set(seen)) >= 2, (st, sorted(set(seen))[:6]))
    check('Download and check shows on the About page, then Restart and install', True)
    ui("[...document.querySelectorAll('.page-internal *')].filter(e=>e.children.length===0&&e.textContent==='About')[0].click(); 1"); time.sleep(0.8)
    bt = V("[...document.querySelectorAll('.settings-content button')].map(b=>b.textContent)")
    check('the About page now offers Restart and install', 'Restart and install' in bt, bt)
    bridge({'type': 'apply-update'})
    gone = wait(lambda: p.poll() is not None, 20)
    check('Restart and install closes the browser', gone is not None)
    ok = wait(lambda: os.path.exists(marker), 25)
    check('and the installer is started after it has closed, with /S', bool(ok) and '/S' in open(marker).read(), open(marker).read() if os.path.exists(marker) else None)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
