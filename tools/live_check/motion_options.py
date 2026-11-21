"""AUDIT-42: Reduce motion Auto / On / Off through real settings."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

S = lambda e: ui('JSON.stringify(%s)' % e)


def save(d):
    bridge({'type': 'save-settings', 'settings': d}); time.sleep(0.8); bridge({'type': 'load-settings'}); time.sleep(1.2)


PROBE = "(()=>{const e=document.createElement('div'); e.className='enter'; document.getElementById('app').append(e); const n=getComputedStyle(e).animationName; e.remove(); return n})()"


def probe_emulated(reduce):
    """Probe the entering animation while the page is told the system prefers (or not) reduced motion.
    The emulation lives only as long as this debugging connection, so ask in the same one."""
    s = cdp.ws_connect(ui_ws())
    def call(i, method, params):
        cdp.send(s, {"id": i, "method": method, "params": params})
        while True:
            m = cdp.recv(s)
            if m.get('id') == i: return m
    call(1, 'Emulation.setEmulatedMedia', {"features": [{"name": "prefers-reduced-motion", "value": "reduce" if reduce else "no-preference"}]})
    time.sleep(0.5)
    r = call(2, 'Runtime.evaluate', {"expression": PROBE, "returnByValue": True})['result']['result']['value']
    s.close()
    return r


def enter_anim():
    """Animation name of a card entering (cheap probe: a throwaway .enter element)."""
    return json.loads(S("(()=>{const e=document.createElement('div'); e.className='enter'; document.getElementById('app').append(e); const n=getComputedStyle(e).animationName; e.remove(); return n})()"))


prof = tempfile.mkdtemp(prefix='cons-prof-')
p = launch([], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(3)
    check('default Auto: no data-reduce attribute', ui("String(document.documentElement.dataset.reduce)") == 'undefined')
    check('Auto with a normal system: cards rise (full motion)', enter_anim() == 'ody-rise', enter_anim())
    save({'stars': 'Drifting'})
    check('Drifting runs a loop when motion is allowed', ui('window.__skyLoopCount()') == 1)
    save({'stars': 'Drifting', 'reduceMotion': 'On'})
    check('On: data-reduce is on, cards only fade', ui("String(document.documentElement.dataset.reduce)") == 'on' and enter_anim() == 'ody-fade', enter_anim())
    check('On: the drifting sky is still and no loop runs', ui('window.__skyLoopCount()') == 0 and json.loads(S('window.__skyInfo()'))['mode'] == 'still')
    save({'stars': 'Drifting', 'reduceMotion': 'Auto'})
    a = probe_emulated(True)
    check('Auto follows a system that asks for reduced motion (cards only fade)', a == 'ody-fade', a)
    save({'stars': 'Drifting', 'reduceMotion': 'Off'})
    a = probe_emulated(True)
    check('Off: motion stays even if the system asks to reduce it', ui("String(document.documentElement.dataset.reduce)") == 'off' and a == 'ody-rise', a)
    check('Off: the sky drifts', ui('window.__skyLoopCount()') == 1 and json.loads(S('window.__skyInfo()'))['mode'] == 'drifting')
    save({'stars': 'Drifting', 'reduceMotion': 'Auto'})
    check('back to Auto: full motion again', enter_anim() == 'ody-rise' and ui('window.__skyLoopCount()') == 1)
    bridge({'type': 'open', 'url': 'odyssey://settings'}); time.sleep(0.8)
    ui("[...document.querySelectorAll('.page-internal *')].filter(e=>e.children.length===0&&e.textContent==='Appearance')[0].click(); 1"); time.sleep(0.5)
    check('Settings offers Reduce motion', 'Reduce motion' in ui("document.querySelector('.settings-content').textContent"))
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
