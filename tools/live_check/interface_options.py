"""AUDIT-39: corner roundness, interface size, Helm buttons and answer card options."""
import os, sys, ctypes
from ctypes import wintypes
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

S = lambda e: ui('JSON.stringify(%s)' % e)
V = lambda e: json.loads(S(e))


def save(d):
    bridge({'type': 'save-settings', 'settings': d}); time.sleep(0.8); bridge({'type': 'load-settings'}); time.sleep(1.2)


def rootvar(n): return ui("document.documentElement.style.getPropertyValue(%s)" % json.dumps(n))


prof = tempfile.mkdtemp(prefix='cons-prof-')
p = launch([], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(3)
    check('default: no radius, size or answer overrides', all(rootvar(n) == '' for n in ['--r-helm', '--r-answer', '--answer-font', '--answer-size', '--answer-w']) and ui("document.getElementById('app').style.zoom") == '')
    check('default: nav and reload buttons show', ui("getComputedStyle(document.querySelector('.helm-nav')).display") != 'none' and ui("getComputedStyle(document.querySelector('.helm-reload')).display") != 'none')
    base = V("getComputedStyle(document.querySelector('.helm')).borderTopLeftRadius")
    check('default Helm radius is the design 26px', base == '26px', base)
    save({'radius': 'Round'})
    check('Round makes the Helm 39px', V("getComputedStyle(document.querySelector('.helm')).borderTopLeftRadius") == '39px')
    save({'radius': 'Sharp'})
    check('Sharp makes it 4px', V("getComputedStyle(document.querySelector('.helm')).borderTopLeftRadius") == '4px')
    save({'radius': 'Default'})
    check('back to Default removes the override', rootvar('--r-helm') == '' and V("getComputedStyle(document.querySelector('.helm')).borderTopLeftRadius") == '26px')
    save({'showNavButtons': False})
    check('back and forward can be hidden', ui("getComputedStyle(document.querySelector('.helm-nav')).display") == 'none' and ui("getComputedStyle(document.querySelector('.helm-reload')).display") != 'none')
    save({'showReload': False})
    check('reload can be hidden', ui("getComputedStyle(document.querySelector('.helm-reload')).display") == 'none')
    w0 = V("document.querySelector('.helm').getBoundingClientRect().width")
    check('the Helm keeps its size', w0 == 560, w0)
    # answer card
    save({'answerFont': 'Mono', 'answerSize': 'Large', 'answerWidth': 'Wide'})
    ui("window.__uiState.answer = {text: 'Hello there', sources: [], footer: 'Answered locally', action: ''}; 1")
    bridge({'type': 'load-settings'}); time.sleep(1.2)
    c = V("(()=>{const c=document.querySelector('.answer-card'); const cs=getComputedStyle(c); return {w:c.getBoundingClientRect().width, font:cs.fontFamily, fs:getComputedStyle(c.firstChild).fontSize}})()")
    check('answer card: Wide, Mono, Large', c['w'] == 680 and 'Cascadia' in c['font'] and c['fs'] == '17.5px', c)
    save({})
    c = V("(()=>{const c=document.querySelector('.answer-card'); return {w:c.getBoundingClientRect().width, fs:getComputedStyle(c.firstChild).fontSize}})()")
    check('answer card back to the design (560px, 15.5px)', c['w'] == 560 and c['fs'] == '15.5px', c)
    # interface size
    ui("window.__sent=[]; const o=window.odysseyBridge; window.odysseyBridge=(m)=>{window.__sent.push(m); return o(m)}; 1")
    bridge({'type': 'open', 'url': 'http://127.0.0.1:%d/a.html' % SRV}); time.sleep(2.5)
    save({'uiScale': '125%'}); time.sleep(1.5)
    r = V("(()=>{const e=document.querySelector('.helm').getBoundingClientRect(); return {w:e.width,l:e.left,t:e.top,b:e.bottom,iw:innerWidth,ih:innerHeight, bw:document.documentElement.scrollWidth}})()")
    check('125%: the Helm is 700px wide on screen', abs(r['w'] - 700) < 1, r)
    check('125%: nothing overflows the window', r['bw'] <= r['iw'] + 1 and r['b'] <= r['ih'] + 1, r)
    rects = V("window.__overlayRects()")
    r = V("(()=>{const e=document.querySelector('.helm, .helm-grip').getBoundingClientRect(); return {w:e.width,l:e.left,t:e.top,b:e.bottom,iw:innerWidth,ih:innerHeight,bw:document.documentElement.scrollWidth}})()")
    check('125%: the hole reported for the page window matches the Helm on screen', rects and any(abs(x['width'] - r['w']) <= 1 and abs(x['x'] - r['l']) <= 1 and abs(x['y'] - r['t']) <= 1 for x in rects), (rects, r))
    # the page window starts right of the rail as drawn (56 css px x 1.25 = 70 on screen)
    u = ctypes.windll.user32
    pid = wintypes.DWORD(); found = []
    cb = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)(lambda h, _l: (found.append(h) if u.IsWindowVisible(h) and u.GetWindow(h, 4) and u.GetWindowThreadProcessId(h, ctypes.byref(pid)) and pid.value == p.pid else None) or True)
    u.EnumWindows(cb, 0)
    parent = u.GetWindow(found[0], 4) if found else 0
    pr, ur = wintypes.RECT(), wintypes.RECT()
    if found:
        u.GetWindowRect(found[0], ctypes.byref(pr)); u.GetWindowRect(parent, ctypes.byref(ur))
    off = pr.left - ur.left
    check('125%: the page window starts right of the scaled rail (70px)', found and 68 <= off <= 72, off)
    save({'uiScale': '90%'})
    r = V("document.querySelector('.helm').getBoundingClientRect().width")
    check('90%: the Helm is 504px', abs(r - 504) < 1, r)
    save({'uiScale': '100%'})
    check('100% again: no zoom left over', ui("document.getElementById('app').style.zoom") == '' and ui("document.getElementById('app').style.width") == '')
    # settings page rows
    bridge({'type': 'open', 'url': 'odyssey://settings'}); time.sleep(0.8)
    ui("[...document.querySelectorAll('.page-internal *')].filter(e=>e.children.length===0&&e.textContent==='Appearance')[0].click(); 1"); time.sleep(0.5)
    t = ui("document.querySelector('.settings-content').textContent")
    check('Settings lists the interface options', all(x in t for x in ['Corner roundness', 'Interface size', 'Reload button', 'Answer font', 'Answer width']))
    save({'radius': 'Round', 'uiScale': '110%'})
    PS = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'capture_seq.ps1')
    ui("[...document.querySelectorAll('.settings-content *')].find(e=>e.textContent==='Interface'&&e.children.length===0).scrollIntoView(); 1"); time.sleep(0.5)
    subprocess.run(['powershell', '-ExecutionPolicy', 'Bypass', '-File', PS, '-Prefix', os.path.join(ROOT, 'out', 'live', 'settings-interface'), '-Count', '1', '-IntervalMs', '10'], capture_output=True)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
