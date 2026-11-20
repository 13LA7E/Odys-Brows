"""AUDIT-01: what is really on screen. Launches the app on a web page, makes ITS window topmost for
a moment and copies only that window's rectangle (BitBlt with CAPTUREBLT, so the page windows are
included), then checks that the Helm and the notch contain drawn pixels, not empty grey shapes.
This is a screen copy (limited to the app's own rectangle while it covers it); the old PrintWindow
checks cannot see this bug because they ignore how the windows are composited."""
import os, subprocess, sys, tempfile
from PIL import Image
ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
exe = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, 'target', 'debug', 'constellations.exe')
prof = tempfile.mkdtemp(prefix='cons-prof-')
open(os.path.join(prof, 'urls.txt'), 'w').write('https://example.com\n')
out = os.path.join(ROOT, 'out', 'live', 'screen_helm.png'); os.makedirs(os.path.dirname(out), exist_ok=True)
env = dict(os.environ)
for dp, dn, fn in os.walk(os.path.join(ROOT, 'target', 'debug', 'build')):
    if 'libcef.dll' in fn and 'cef_windows' in dp: env['PATH'] = dp + os.pathsep + env['PATH']; break
subprocess.run(['powershell', '-ExecutionPolicy', 'Bypass', '-File', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'screen_capture.ps1'),
                '-Exe', exe, '-Out', out, '-ArgList', '--urls-file=' + os.path.join(prof, 'urls.txt'), '-ProfileDir', prof, '-Wait', '14'], env=env)
im = Image.open(out).convert('L'); W, H = im.size
def bright(box):
    c = im.crop(box); return sum(1 for v in c.getdata() if v > 120)
helm = bright((int(W * .30), H - 95, int(W * .72), H - 30)); notch = bright((W - 260, 3, W - 5, 40))
print('helm bright pixels', helm, '| notch bright pixels', notch)
ok = helm > 200 and notch > 80
print('PASS Helm and notch are visible on screen over a web page (AUDIT-01)' if ok else 'FAIL Helm or notch is blank on screen (AUDIT-01)')
sys.exit(0 if ok else 1)
