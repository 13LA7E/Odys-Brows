import os, sys, ssl, ctypes, ctypes.wintypes as wt, subprocess
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

# self-signed https site on 8767
cdir = tempfile.mkdtemp(prefix='cons-cert-')
subprocess.run(['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-keyout', os.path.join(cdir, 'k.pem'), '-out', os.path.join(cdir, 'c.pem'),
                '-days', '2', '-subj', '/CN=localhost'], capture_output=True)
class SH(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *a, **k): super().__init__(*a, directory=site, **k)
    def log_message(self, *a): pass
hs = TS(('127.0.0.1', 8767), SH)
ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER); ctx.load_cert_chain(os.path.join(cdir, 'c.pem'), os.path.join(cdir, 'k.pem'))
hs.socket = ctx.wrap_socket(hs.socket, server_side=True); threading.Thread(target=hs.serve_forever, daemon=True).start()

u32 = ctypes.windll.user32; g32 = ctypes.windll.gdi32
u32.SetProcessDPIAware()
def page_regions(pid_names=('constellations.exe',)):
    out = []
    pids = set(int(l.split(',')[1].strip('"')) for l in subprocess.run(['tasklist', '/FI', 'IMAGENAME eq constellations.exe', '/FO', 'CSV', '/NH'], capture_output=True, text=True).stdout.splitlines() if l.strip())
    @ctypes.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)
    def cb(h, l):
        pid = wt.DWORD(); u32.GetWindowThreadProcessId(h, ctypes.byref(pid))
        if pid.value in pids and u32.IsWindowVisible(h):
            rgn = g32.CreateRectRgn(0, 0, 0, 0)
            kind = u32.GetWindowRgn(h, rgn)
            r = wt.RECT(); u32.GetWindowRect(h, ctypes.byref(r))
            if kind >= 2: out.append((h, rgn, r))
        return True
    u32.EnumWindows(cb, 0)
    return out
def visible_page_pixel(x_dip, y_dip, dpi_scale):
    """True if the page window covers this main-window DIP point (no hole there)."""
    for h, rgn, r in page_regions():
        px = int((x_dip - 56) * dpi_scale); py = int(y_dip * dpi_scale)   # collapsed rail = 56 DIP
        return bool(g32.PtInRegion(rgn, px, py))
    return None

prof = tempfile.mkdtemp(prefix='cons-prof-')
open(os.path.join(prof, 'urls.txt'), 'w').write(f'http://127.0.0.1:{SRV}/a.html\n')
p = launch([f'--urls-file={os.path.join(prof, "urls.txt")}'], prof)
S = lambda e: ui(f'JSON.stringify({e})')
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(4)
    dpi = ui('window.devicePixelRatio')
    pw = wait(lambda: page_ws('/a.html'), 10)
    # --- permission prompt
    ev(pw, "window.geoResult='pending'; navigator.geolocation.getCurrentPosition(p=>window.geoResult='ok',e=>window.geoResult='err'+e.code)", gesture=True)
    pr = wait(lambda: (lambda x: x if x else None)(json.loads(S('(window.__uiState.prompts||[])[0]||null'))), 8)
    print('prompt:', pr)
    check('geolocation request reaches the UI as a prompt (AUDIT-21)', bool(pr) and pr['kind'] == 'permission' and pr['what'] == 'location', pr)
    time.sleep(1)
    card = json.loads(ui("JSON.stringify((()=>{const e=document.querySelector('.permission-card');if(!e)return null;const r=e.getBoundingClientRect();return {x:r.left,y:r.top,w:r.width,h:r.height}})())"))
    check('prompt card is drawn', bool(card), card)
    if card:
        cx, cy = card['x'] + card['w'] / 2, card['y'] + card['h'] / 2
        rects = json.loads(ui('JSON.stringify(window.__overlayRects())'))
        check('UI reports the card as an overlay rect (AUDIT-06)', any(abs(r['x'] - round(card['x'])) <= 1 for r in rects), rects)
        covered = visible_page_pixel(cx, cy, dpi)
        check('page window has a hole under the card (AUDIT-06)', covered is False, f'covered={covered}')
        outside = visible_page_pixel(cx, 200, dpi)
        check('page window still covers the page elsewhere', outside is True, outside)
    ui("document.querySelectorAll('.permission-card button')[0].click(); 1"); time.sleep(1.5)   # Block
    res = ev(pw, "window.geoResult")
    check('Block denies the permission (page sees PERMISSION_DENIED)', res == 'err1', res)
    time.sleep(0.5)
    check('hole removed after the card is gone (AUDIT-06)', visible_page_pixel(cx, cy, dpi) is True if card else False)

    # --- certificate error
    bridge({'type': 'navigate', 'view': json.loads(S('window.__uiState.activeTab')), 'url': 'https://127.0.0.1:8767/b.html'})
    cp = wait(lambda: (lambda x: x if x else None)(json.loads(S('(window.__uiState.prompts||[]).find(p=>p.kind=="certificate")||null'))), 10)
    print('cert prompt:', cp)
    check('certificate error reaches the UI (AUDIT-21)', bool(cp) and cp.get('error'), cp and cp.get('error'))
    ui("[...document.querySelectorAll('.permission-card button')].find(b=>b.textContent==='Continue anyway').click(); 1"); time.sleep(3)
    t = [x for x in targets() if x.get('type') == 'page' and '8767' in x.get('url', '')]
    check('Continue anyway loads the page', bool(t) and 'Page B' in t[0].get('title', ''), t and t[0].get('title'))
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
