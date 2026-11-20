"""Makes the evidence for AUDIT-32/33: PNG frames and GIFs of the real app window in out/motion/.
Durations are slowed through the same CSS tokens (--dur-*) so each step is visible; that also
shows the tokens are the one place to tune. Needs the debug build and Pillow."""
import os, sys, glob, subprocess
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *
from PIL import Image
OUT = os.path.join(ROOT, 'out', 'motion'); os.makedirs(OUT, exist_ok=True)
PS = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'capture_seq.ps1')

def mouse(x, y):
    s = cdp.ws_connect(ui_ws()); cdp.send(s, {"id": 1, "method": "Input.dispatchMouseEvent", "params": {"type": "mouseMoved", "x": x, "y": y}})
    while cdp.recv(s).get('id') != 1: pass
    s.close()

def capture(name, count, interval, action=None, delay=1.2):
    prefix = os.path.join(OUT, name)
    for f in glob.glob(prefix + '-*.png'): os.remove(f)
    proc = subprocess.Popen(['powershell', '-ExecutionPolicy', 'Bypass', '-File', PS, '-Prefix', prefix, '-Count', str(count), '-IntervalMs', str(interval)], stdout=subprocess.DEVNULL)
    time.sleep(delay)
    if action: action()
    proc.wait()
    frames = [Image.open(f).convert('RGB') for f in sorted(glob.glob(prefix + '-*.png'))]
    small = [f.resize((f.width // 2, f.height // 2)) for f in frames]
    small[0].save(prefix + '.gif', save_all=True, append_images=small[1:], duration=max(60, interval), loop=0)
    print(name, len(frames), 'frames ->', prefix + '.gif')
    return frames

prof = tempfile.mkdtemp(prefix='cons-prof-')
os.makedirs(os.path.join(prof, 'Constellations'), exist_ok=True)
json.dump({"stars": "Drifting", "accent": "Sea", "trail": "Narrow", "connections": 8}, open(os.path.join(prof, 'Constellations', 'settings.json'), 'w'))
open(os.path.join(prof, 'urls.txt'), 'w').write(f'http://127.0.0.1:{SRV}/a.html\nhttp://127.0.0.1:{SRV}/b.html\n')
p = launch([f'--urls-file={os.path.join(prof, "urls.txt")}'], prof)
S = lambda e: ui(f'JSON.stringify({e})')
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(4)
    bridge({'type': 'open', 'url': ''}); time.sleep(1.5)       # new-tab page, sky visible
    ui("window.dispatchEvent(new Event('resize')); 1"); time.sleep(1)
    # 1. cursor-linked stars: pointer far away, then sweeping over the densest cluster
    pt = json.loads(S("""(()=>{const s=window.__uiState.sky.filter(q=>q.r>=0.9);let best=null,bn=0;for(const a of s){let n=0;for(const b of s){if(Math.hypot(a.x-b.x,a.y-b.y)<110)n++}if(n>bn){bn=n;best=a}}return {x:best.x,y:best.y}})()"""))
    x, y = int(pt['x']), int(pt['y'])
    def sweep():
        mouse(1500, 700); time.sleep(0.5)
        for i in range(10):
            mouse(x - 160 + i * 36, y + 10); time.sleep(0.12)
    capture('cursor-stars', 14, 150, sweep, delay=1.0)
    # 2. downloads popover enter/exit (slowed through the tokens)
    bridge({'type': 'show', 'view': json.loads(S('window.__uiState.tabs[0].id'))}); time.sleep(1.5)
    pw = page_ws('/a.html')
    ui("document.documentElement.style.setProperty('--dur-base','1200ms'); window.__uiState.downloads=[{id:'9',name:'northstar-star-chart.pdf',detail:'124 MB of 200 MB, 11.2 MB/s, 8 connections',progress:.62,active:true,cancellable:true}]; 1")
    def pop():
        ui("window.__uiState.downloadsOpen=true; 1"); ev(pw, "document.title='x1'"); time.sleep(1.6)
        ui("window.__uiState.downloadsOpen=false; 1"); ev(pw, "document.title='x2'")
    capture('popover', 12, 200, pop, delay=1.0)
    ui("document.documentElement.style.removeProperty('--dur-base'); 1")
    # 3. rail expand (slowed)
    ui("document.documentElement.style.setProperty('--dur-slow','1600ms'); 1")
    capture('rail-expand', 10, 200, lambda: ui("document.querySelector('.rail-logo').click(); 1"), delay=1.0)
    ui("document.documentElement.style.removeProperty('--dur-slow'); 1")
    # 4. new tab star + branch (expanded rail), slowed
    ui("document.documentElement.style.setProperty('--dur-slow','1600ms'); 1")
    capture('new-tab-star', 10, 200, lambda: bridge({'type': 'open', 'url': f'http://127.0.0.1:{SRV}/c.html', 'opener': json.loads(S('window.__uiState.activeTab'))}), delay=1.0)
    ui("document.documentElement.style.removeProperty('--dur-slow'); 1")
finally:
    kill(p)
