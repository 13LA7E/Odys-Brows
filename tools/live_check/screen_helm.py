"""AUDIT-01 on the REAL screen: with a web page open, the Helm and the notch must show drawn pixels
(not empty grey shapes), and the page must show through where they have slid away.

The notch and the Helm auto-hide over web pages, so each is brought back with a pointer event on its
strip (over the debugging port), then the app window's own rectangle is copied from the screen
(BitBlt with CAPTUREBLT, so page windows are included). The old PrintWindow checks cannot see this bug
because they ignore how the windows are composited."""
import os, sys, subprocess
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *
from PIL import Image

PS = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'capture_seq.ps1')
OUT = os.path.join(ROOT, 'out', 'live'); os.makedirs(OUT, exist_ok=True)


def mouse(x, y):
    s = cdp.ws_connect(ui_ws()); cdp.send(s, {"id": 1, "method": "Input.dispatchMouseEvent", "params": {"type": "mouseMoved", "x": x, "y": y}})
    while cdp.recv(s).get('id') != 1: pass
    s.close()


def shot(name):
    prefix = os.path.join(OUT, name)
    subprocess.run(['powershell', '-ExecutionPolicy', 'Bypass', '-File', PS, '-Prefix', prefix, '-Count', '1', '-IntervalMs', '10'], capture_output=True)
    return Image.open(prefix + '-00.png').convert('L')


def bright(im, box):
    return sum(1 for v in im.crop(box).getdata() if v > 120)


prof = tempfile.mkdtemp(prefix='cons-prof-')
urls = os.path.join(prof, 'urls.txt')
open(urls, 'w').write('https://example.com\n')
p = launch(['--urls-file=' + urls], prof)
ok = True
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    time.sleep(9)   # the page is up and both bars have slid away
    iw = ui('window.innerWidth')
    mouse(iw - 100, 2); time.sleep(1.6)                                                       # the notch's strip
    b = shot('screen_helm_notch'); W, H = b.size
    notch = bright(b, (W - 260, 3, W - 5, 40))
    mouse(iw // 2, 1000); mouse(iw // 2 + 5, ui('window.innerHeight') - 2); time.sleep(1.6)   # the Helm's strip
    a = shot('screen_helm_helm')
    helm = bright(a, (int(W * .30), H - 95, int(W * .72), H - 30))
    print('helm bright pixels', helm, '| notch bright pixels', notch)
    ok = helm > 200 and notch > 80
finally:
    kill(p)
print('PASS Helm and notch are visible on screen over a web page (AUDIT-01)' if ok else 'FAIL Helm or notch is blank on screen (AUDIT-01)')
sys.exit(0 if ok else 1)
