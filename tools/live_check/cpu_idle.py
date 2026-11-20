"""CPU use of the whole browser (all processes) in idle situations, on the release build.
Usage: python tools/live_check/cpu_idle.py [path-to-constellations.exe]"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import scenario
from scenario import *
rel = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, 'target', 'release', 'constellations.exe')
scenario.EXE = rel
def cpu_seconds():
    out = subprocess.run(['powershell', '-NoProfile', '-Command', "(Get-Process constellations -ErrorAction SilentlyContinue | ForEach-Object { $_.TotalProcessorTime.TotalSeconds } | Measure-Object -Sum).Sum"], capture_output=True, text=True).stdout.strip()
    return float(out.replace(',', '.') or 0)
def measure(label, secs=10):
    a = cpu_seconds(); t = time.time(); time.sleep(secs); b = cpu_seconds()
    pct = (b - a) / (time.time() - t) * 100
    print(f'{label:58s} {pct:5.2f}% of one core  ({pct / os.cpu_count():.2f}% of the machine)')
    return pct

def run(stars, url=None):
    prof = tempfile.mkdtemp(prefix='cons-prof-')
    os.makedirs(os.path.join(prof, 'Constellations'), exist_ok=True)
    json.dump({"stars": stars, "accent": "Sea", "trail": "Narrow", "connections": 8}, open(os.path.join(prof, 'Constellations', 'settings.json'), 'w'))
    args = []
    if url:
        open(os.path.join(prof, 'urls.txt'), 'w').write(url + '\n'); args = ['--urls-file=' + os.path.join(prof, 'urls.txt')]
    p = launch(args, prof)
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    return p

def mouse(x, y):
    s = cdp.ws_connect(ui_ws()); cdp.send(s, {"id": 1, "method": "Input.dispatchMouseEvent", "params": {"type": "mouseMoved", "x": x, "y": y}})
    while cdp.recv(s).get('id') != 1: pass
    s.close()

print('exe:', rel, '| logical cores:', os.cpu_count())
p = run('Still'); time.sleep(15)
measure('Still, new-tab page, nothing moving')
kill(p)
p = run('Drifting'); time.sleep(15)
measure('Drifting, new-tab page, pointer still (first 10 s)')
th = threading.Event()
def wiggle():
    i = 0
    while not th.is_set():
        mouse(300 + (i * 37) % 900, 300 + (i * 23) % 400); i += 1; time.sleep(0.016)
t = threading.Thread(target=wiggle); t.start()
measure('Drifting, pointer moving all the time (cursor stars)')
th.set(); t.join()
time.sleep(34)
measure('Drifting, after 30 s without activity (sky paused)')
kill(p)
p = run('Drifting', f'http://127.0.0.1:{SRV}/a.html'); time.sleep(15)
measure('Drifting, a web page shown (sky paused)')
kill(p)
