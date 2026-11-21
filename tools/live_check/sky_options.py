"""AUDIT-36: star density, drift speed, comet rate and Low power through real settings; comets
actually cross (they had stopped spawning since AUDIT-32); CPU/loop stops in Low power."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

S = lambda e: ui('JSON.stringify(%s)' % e)
info = lambda: json.loads(S('window.__skyInfo()'))


def save(d):
    bridge({'type': 'save-settings', 'settings': d}); time.sleep(0.8); bridge({'type': 'load-settings'}); time.sleep(1.0)


prof = tempfile.mkdtemp(prefix='cons-prof-')
p = launch([], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(3)
    base = info()
    check('default sky: all stars, still', base['mode'] == 'still' and base['stars'] > 100, base)
    save({'stars': 'Drifting', 'skyDensity': '50%'})
    half = info()
    check('50% density keeps about half the stars', 0.4 * base['stars'] < half['stars'] < 0.6 * base['stars'], (base['stars'], half['stars']))
    save({'stars': 'Drifting', 'skyDensity': '25%'})
    check('25% keeps about a quarter', 0.15 * base['stars'] < info()['stars'] < 0.35 * base['stars'], info())
    save({'stars': 'Drifting', 'skyDensity': '100%'})
    check('100% is the full design again', info()['stars'] == base['stars'], info())
    save({'stars': 'Drifting', 'skySpeed': 'Fast'})
    check('Fast doubles the drift, Slow halves it', info()['drift'] == 2, info())
    save({'stars': 'Drifting', 'skySpeed': 'Slow'})
    check('...Slow', info()['drift'] == 0.5, info())
    check('Drifting runs a loop', ui('window.__skyLoopCount()') == 1)
    save({'stars': 'Drifting', 'lowPower': True, 'shootingStars': True})
    i = info()
    check('Low power: still, no comets, no cursor lines', i['mode'] == 'still' and not i['comets'] and not i['cursor'], i)
    check('Low power: no animation loop runs at all', ui('window.__skyLoopCount()') == 0, ui('window.__skyLoopCount()'))
    save({'stars': 'Off'})
    check('Stars Off still works', info()['mode'] == 'off', info())
    # comets really cross
    save({'stars': 'Still', 'shootingStars': True, 'cometRate': 'Often', 'lowPower': False})
    seen = wait(lambda: info()['comet'], 40, 0.25)
    check('a comet crosses the sky (Often)', bool(seen), info())
    check('the sky loop runs for comets', ui('window.__skyLoopCount()') == 1)
    # UI rows only when they can do something
    bridge({'type': 'open', 'url': 'odyssey://settings'}); time.sleep(0.8)
    ui("[...document.querySelectorAll('.page-internal *')].filter(e=>e.children.length===0&&e.textContent==='Appearance')[0].click(); 1"); time.sleep(0.5)
    text = ui("document.querySelector('.settings-content').textContent")
    check('rate row shows with shooting stars; speed row hidden unless Drifting', 'Shooting star rate' in text and 'Drift speed' not in text, text[:200])
    save({'stars': 'Off'}); time.sleep(0.5)
    text = ui("document.querySelector('.settings-content').textContent")
    check('with Stars off no sky options are offered', 'Star density' not in text and 'Low power' not in text)
    save({'stars': 'Drifting', 'lowPower': False, 'shootingStars': False, 'skyDensity': '75%'}); time.sleep(0.5)
    PS = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'capture_seq.ps1')
    subprocess.run(['powershell', '-ExecutionPolicy', 'Bypass', '-File', PS, '-Prefix', os.path.join(ROOT, 'out', 'live', 'settings-sky'), '-Count', '1', '-IntervalMs', '10'], capture_output=True)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
