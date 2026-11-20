import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *
prof = tempfile.mkdtemp(prefix='cons-prof-')
p = launch([], prof)
RIGHT = """(()=>{const c=document.querySelector('canvas.sky');const d=window.devicePixelRatio||1;const w=c.width/d;const g=c.getContext('2d').getImageData(Math.floor((w*0.8)*d),0,Math.floor(w*0.2*d),c.height).data;let n=0;for(let i=3;i<g.length;i+=4){if(g[i]>0)n++}return n})()"""
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(3)
    bridge({'type': 'maximize-window'}); time.sleep(3)
    w = ui('window.innerWidth'); cw = ui("document.querySelector('canvas.sky').getBoundingClientRect().width")
    check('sky canvas is as wide as the window after maximising (was left at the old size)', abs(cw - w) <= 2, (w, cw))
    n = ui(RIGHT)
    check('stars are drawn across the right 20% of the window', n > 150, n)
    bridge({'type': 'maximize-window'}); time.sleep(3)
    w2 = ui('window.innerWidth'); cw2 = ui("document.querySelector('canvas.sky').getBoundingClientRect().width")
    check('...and again after restoring', abs(cw2 - w2) <= 2, (w2, cw2))
    check('no click effect left in the sky code', ui("typeof document.querySelector('canvas.sky')") == 'object')
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
