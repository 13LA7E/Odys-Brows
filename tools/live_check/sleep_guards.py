import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

open(os.path.join(site, 'audio.html'), 'w').write('''<!doctype html><title>Audio</title><input id=i>
<button id=play onclick="const c=new AudioContext();const o=c.createOscillator();const g=c.createGain();g.gain.value=0.02;o.connect(g);g.connect(c.destination);o.start();c.resume();window.ctx=c">play</button>''')
open(os.path.join(site, 'form.html'), 'w').write('<!doctype html><title>Form</title><input id=i><p>x</p>')
prof = tempfile.mkdtemp(prefix='cons-prof-')
open(os.path.join(prof, 'urls.txt'), 'w').write(f'http://127.0.0.1:{SRV}/audio.html\nhttp://127.0.0.1:{SRV}/form.html\nhttp://127.0.0.1:{SRV}/b.html\n')
p = launch([f'--urls-file={os.path.join(prof, "urls.txt")}'], prof)
S = lambda e: ui(f'JSON.stringify({e})')
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(4)
    tabs = json.loads(S('window.__uiState.tabs'))
    aid = [t for t in tabs if 'audio.html' in t['url']][0]['id']
    fid = [t for t in tabs if 'form.html' in t['url']][0]['id']
    bid = [t for t in tabs if 'b.html' in t['url']][0]['id']
    bridge({'type': 'show', 'view': bid}); time.sleep(1)
    pa = wait(lambda: page_ws('/audio.html'), 10)
    ev(pa, "document.getElementById('play').click()", gesture=True); time.sleep(3)
    pf = wait(lambda: page_ws('/form.html'), 10)
    ev(pf, "(()=>{const i=document.getElementById('i'); i.value='draft'; i.dispatchEvent(new Event('input',{bubbles:true}))})()"); time.sleep(1)
    asleep = lambda v: json.loads(S('!!window.__uiState.tabs.find(t=>t.id==%d).sleeping' % v))
    bridge({'type': 'set-tab-sleeping', 'view': aid, 'sleeping': True}); time.sleep(1.5)
    check('tab playing audio does not sleep (AUDIT-15)', not asleep(aid))
    bridge({'type': 'set-tab-sleeping', 'view': fid, 'sleeping': True}); time.sleep(1.5)
    check('tab with typed text does not sleep (AUDIT-15)', not asleep(fid))
    ev(pa, "window.ctx && window.ctx.close()"); time.sleep(4)
    bridge({'type': 'set-tab-sleeping', 'view': aid, 'sleeping': True}); time.sleep(1.5)
    check('tab sleeps once audio has stopped', asleep(aid))
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))

# ---- scroll restored after sleep (AUDIT-14) and duplicate Url / internal-page checks
prof = tempfile.mkdtemp(prefix='cons-prof-')
open(os.path.join(prof, 'urls.txt'), 'w').write(f'http://127.0.0.1:{SRV}/a.html\nhttp://127.0.0.1:{SRV}/b.html\n')
p = launch([f'--urls-file={os.path.join(prof, "urls.txt")}'], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(3)
    tabs = json.loads(ui('JSON.stringify(window.__uiState.tabs)'))
    a = [t for t in tabs if 'a.html' in t['url']][0]['id']; b = [t for t in tabs if 'b.html' in t['url']][0]['id']
    bridge({'type': 'show', 'view': a}); time.sleep(1)
    pa = page_ws('/a.html'); ev(pa, "scrollTo(0,900)"); time.sleep(1.5)
    bridge({'type': 'show', 'view': b}); time.sleep(1)
    bridge({'type': 'set-tab-sleeping', 'view': a, 'sleeping': True}); time.sleep(2)
    bridge({'type': 'show', 'view': a}); time.sleep(4)
    y = ev(page_ws('/a.html'), "window.scrollY")
    check('scroll position restored after wake (AUDIT-14)', isinstance(y, (int, float)) and abs(y - 900) < 5, y)
    # web tab navigated to an internal page stops running (AUDIT-29)
    bridge({'type': 'navigate', 'view': b, 'url': 'odyssey://settings'}); time.sleep(2)
    live_b = [t for t in targets() if t.get('type') == 'page' and 'b.html' in t.get('url', '')]
    check('web tab sent to odyssey:// stops its page', not live_b, [t['url'] for t in live_b])
    tb = json.loads(ui('JSON.stringify(window.__uiState.tabs.find(t=>t.id==%d))' % b))
    check('...and keeps the odyssey:// address', tb['url'] == 'odyssey://settings', tb['url'])
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
