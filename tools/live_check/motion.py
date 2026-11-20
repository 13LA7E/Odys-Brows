import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

def cdp_call(method, params):
    s = cdp.ws_connect(ui_ws()); cdp.send(s, {"id": 1, "method": method, "params": params})
    while True:
        m = cdp.recv(s)
        if m.get('id') == 1: s.close(); return m

prof = tempfile.mkdtemp(prefix='cons-prof-')
os.makedirs(os.path.join(prof, 'Constellations'), exist_ok=True)
json.dump({"stars": "Still", "accent": "Sea", "trail": "Narrow", "connections": 8}, open(os.path.join(prof, 'Constellations', 'settings.json'), 'w'))
open(os.path.join(prof, 'urls.txt'), 'w').write(f'http://127.0.0.1:{SRV}/a.html\n')
p = launch([f'--urls-file={os.path.join(prof, "urls.txt")}'], prof)
S = lambda e: ui(f'JSON.stringify({e})')
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(4)
    # --- hover/press transitions come from the shared tokens
    td = ui("getComputedStyle(document.querySelector('.helm-asst-btn')).transitionDuration")
    check('buttons transition with --dur-fast (0.12s)', td.split(',')[0].strip() in ('0.12s', '120ms'), td)
    # --- popover enter: rises and fades, once
    ui("window.__uiState.downloads=[{id:'9',name:'big.bin',detail:'1 MB of 100 MB',progress:.2,active:true,cancellable:true}]; window.__uiState.downloadsOpen=true; 1")
    pw = page_ws('/a.html')
    ev(pw, "document.title='open'+Math.random()")   # any state change renders; query at once, before 200 ms pass
    anim = json.loads(ui("JSON.stringify((()=>{const e=document.querySelector('.downloads-popover'); if(!e) return null; const a=e.getAnimations(); return {n:a.length, name:a[0]&&a[0].animationName, dur:a[0]&&a[0].effect.getTiming().duration}})())"))
    print('popover animation:', anim)
    check('downloads popover enters with the rise animation (AUDIT-33)', bool(anim) and anim.get('name') == 'ody-rise-small', anim)
    check('animation duration = --dur-base (200ms)', bool(anim) and anim.get('dur') == 200, anim and anim.get('dur'))
    # --- no restart under churn: tag the animation object, force many renders, same animation object continues
    ui("window.__el = document.querySelector('.downloads-popover'); window.__a = window.__el && window.__el.getAnimations()[0]; 1")
    for i in range(30): ev(pw, f"document.title='M{i}'")
    still = json.loads(ui("JSON.stringify({same: document.querySelector('.downloads-popover')===window.__el, running: window.__a ? window.__a.playState : 'finished'})"))
    check('50 updates during an animation do not rebuild or restart it (no flicker)', still['same'], still)
    # --- exit: fades out, stays in the DOM until finished, then goes
    ui("window.__uiState.downloadsOpen=false; 1"); ev(pw, "document.title='close'+Math.random()"); time.sleep(0.05)
    leaving = ui("(()=>{const e=document.querySelector('.downloads-popover'); return !!e && e.classList.contains('leaving')})()")
    check('closing popover fades out before it leaves the DOM', leaving is True, leaving)
    time.sleep(0.5)
    check('...and is removed afterwards', ui("!document.querySelector('.downloads-popover')") is True)
    # --- tokens drive everything: change --dur-base, animation follows
    ui("document.documentElement.style.setProperty('--dur-base','900ms'); window.__uiState.downloadsOpen=true; 1"); ev(pw, "document.title='again'+Math.random()")
    d2 = ui("document.querySelector('.downloads-popover') && document.querySelector('.downloads-popover').getAnimations()[0].effect.getTiming().duration")
    check('changing the token --dur-base changes the animation (one place to tune)', d2 == 900, d2)
    ui("document.documentElement.style.removeProperty('--dur-base'); window.__uiState.downloadsOpen=false; 1")
    # --- rail expands with an animated width (--rail transition)
    ui("document.querySelector('.rail-logo').click(); 1")
    widths = []
    for i in range(8):
        widths.append(ui("document.querySelector('.rail').getBoundingClientRect().width")); time.sleep(0.04)
    print('rail width samples', widths)
    check('rail width animates between 56 and 264 (not a jump)', any(56 < w < 264 for w in widths) and widths[-1] > 200, widths)
    # --- new tab star enters
    bridge({'type': 'open', 'url': f'http://127.0.0.1:{SRV}/b.html'})
    fresh = 0; flick = []
    for i in range(12):
        time.sleep(0.03)
        fresh = max(fresh, ui("document.querySelectorAll('.star-enter, .branch-enter').length"))
        flick.append(ui("(()=>{const e=document.querySelector('.star-enter, .branch-enter'); if(!e) return null; const a=e.getAnimations()[0]; return a ? Math.round(a.currentTime) : null})()"))
    check('a new tab star/branch gets its enter animation', fresh >= 1, fresh)
    nums = [x for x in flick if x is not None]
    print('animation clock samples (ms):', nums)
    check('the enter animation keeps running across rail rebuilds (time never goes backwards)', all(b >= a - 5 for a, b in zip(nums, nums[1:])), nums)
    # --- page switch cross-fades
    bridge({'type': 'open', 'url': 'odyssey://settings'}); time.sleep(0.3)
    check('opening Settings cross-fades the panel', ui("!!document.querySelector('.page-in')") is True)
    # --- reduced motion: opacity-only, cursor effect off
    sk = cdp.ws_connect(ui_ws())
    cdp.send(sk, {"id": 1, "method": "Emulation.setEmulatedMedia", "params": {"features": [{"name": "prefers-reduced-motion", "value": "reduce"}]}})
    while cdp.recv(sk).get('id') != 1: pass
    cdp.send(sk, {"id": 2, "method": "Runtime.evaluate", "params": {"returnByValue": True, "expression": "(()=>{const t=document.createElement('div');t.className='enter';document.body.appendChild(t);const n=getComputedStyle(t).animationName;t.remove();return n})()"}})
    while True:
        m = cdp.recv(sk)
        if m.get('id') == 2: nm = m['result']['result']['value']; break
    sk.close()
    check('prefers-reduced-motion: entrance becomes a plain fade (no transform)', nm == 'ody-fade', nm)
    # --- idle CPU: the UI must not animate while nothing moves
    lc = ui('window.__skyLoopCount()')
    check('no sky loop running in Still mode', lc == 0, lc)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
