import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

def mouse(x, y):
    s = cdp.ws_connect(ui_ws())
    cdp.send(s, {"id": 1, "method": "Input.dispatchMouseEvent", "params": {"type": "mouseMoved", "x": x, "y": y}})
    while cdp.recv(s).get('id') != 1: pass
    s.close()

prof = tempfile.mkdtemp(prefix='cons-prof-')
os.makedirs(os.path.join(prof, 'Constellations'), exist_ok=True)
json.dump({"stars": "Drifting", "accent": "Sea", "trail": "Narrow", "connections": 8}, open(os.path.join(prof, 'Constellations', 'settings.json'), 'w'))
p = launch([], prof)
S = lambda e: ui(f'JSON.stringify({e})')
LIT = """(()=>{const c=document.querySelector('canvas.sky');const d=window.devicePixelRatio||1;const x=%d,y=%d,r=%d;
 const g=c.getContext('2d').getImageData(Math.max(0,(x-r)*d),Math.max(0,(y-r)*d),2*r*d,2*r*d).data;let n=0;for(let i=3;i<g.length;i+=4){if(g[i]>0)n++}return n})()"""
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(3)
    st = json.loads(S('{mode:window.__uiState.settings&&window.__uiState.settings.stars, loops:window.__skyLoopCount(), page:window.__uiState.page}'))
    print(st)
    check('saved Stars=Drifting setting is applied to the sky after a restart', st['loops'] == 1, st)
    # densest spot of "near" stars (radius >= 0.9) in the sky data
    pt = json.loads(S("""(()=>{const s=window.__uiState.sky.filter(q=>q.r>=0.9);let best=null,bn=0;for(const a of s){let n=0;for(const b of s){if(Math.hypot(a.x-b.x,a.y-b.y)<110)n++}if(n>bn){bn=n;best=a}}return {x:best.x+20,y:best.y+20,n:bn}})()"""))
    print('spot', pt)
    x, y = int(pt['x']), int(pt['y'])
    mouse(1500, 20); time.sleep(0.8)   # far away, effect not near the spot
    base = ui(LIT % (x, y, 150))
    mouse(x, y); time.sleep(0.8)
    near = ui(LIT % (x, y, 150))
    print('lit pixels near spot: base', base, 'with cursor', near)
    check('cursor lights and links nearby stars (AUDIT-32)', near > base * 1.0 + 20, (base, near))
    # click burst on empty background
    ui("window.dispatchEvent(new Event('pointerleave'));1")
    # toggle off in settings -> effect gone
    ui("window.__uiState.settings.starsFollowCursor=false; document.querySelector('.helm-address').click(); document.querySelector('.helm-input').blur(); 1")
    ev(ui_ws(), "1")
    bridge({'type': 'open', 'url': ''}); time.sleep(1)  # forces a render with the new setting
    mouse(1500, 20); time.sleep(0.6); base2 = ui(LIT % (x, y, 150)); mouse(x, y); time.sleep(0.8); near2 = ui(LIT % (x, y, 150))
    check('toggle off removes the cursor effect', near2 <= base2 + 5, (base2, near2))
    check('exactly one sky loop after all that (AUDIT-04)', ui('window.__skyLoopCount()') <= 1, ui('window.__skyLoopCount()'))
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
