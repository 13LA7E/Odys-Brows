"""AUDIT-35: themes through the real Settings page: presets apply and persist, Default adds nothing,
the editor refuses unreadable or light colours, Import refuses junk, Export round-trips, bad themes
in settings.json are dropped."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

S = lambda e: ui('JSON.stringify(%s)' % e)


def sf(prof): return os.path.join(prof, 'Constellations', 'settings.json')


def start(prof):
    p = launch([], prof)
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    time.sleep(3)
    return p


def open_appearance():
    bridge({'type': 'open', 'url': 'odyssey://settings'}); time.sleep(0.8)
    ui("[...document.querySelectorAll('.page-internal *')].filter(e=>e.children.length===0&&e.textContent==='Appearance')[0].click(); 1"); time.sleep(0.5)


def btn(text):
    ui("[...document.querySelectorAll('.settings-content button')].find(b=>b.textContent===%s).click(); 1" % json.dumps(text)); time.sleep(0.4)


def card(name):
    ui("document.querySelector('.settings-content [aria-label=\"Theme %s\"]').click(); 1" % name); time.sleep(0.5)


def var(n): return ui("document.documentElement.style.getPropertyValue(%s)" % json.dumps(n))


def set_hex(label, v):
    ui("(()=>{const i=document.querySelector('.settings-content [aria-label=\"%s hex\"]'); i.value=%s; i.dispatchEvent(new Event('change',{bubbles:true})); return 1})()" % (label, json.dumps(v)))
    time.sleep(0.2)


def msg():
    return ui("(document.querySelector('.theme-message')||{}).textContent || ''")


# bad themes in the file are dropped, a good one is kept
prof = tempfile.mkdtemp(prefix='cons-prof-')
os.makedirs(os.path.join(prof, 'Constellations'), exist_ok=True)
good = {"name": "Dusk", "bg": "#000000", "surface": "#101018", "panel": "#08080e", "text": "#e8e8f0", "accent": "#ff8844", "star": "#ffeedd"}
json.dump({"theme": "dusk", "customThemes": {"dusk": good, "evil": dict(good, accent="red;x"), "Bad Id": good}}, open(sf(prof), 'w'))
p = start(prof)
try:
    st = json.loads(S('window.__uiState.settings'))
    check('only the valid custom theme survives loading', list(st['customThemes']) == ['dusk'], st['customThemes'])
    check('a saved custom theme is applied at start', var('--bg') == '#000000' and var('--accent') == '#ff8844' and var('--star') == '#ffeedd', var('--accent'))
    check('text variants are worked out', var('--text-strong').startswith('#') and var('--on-accent') == '#000000', (var('--text-strong'), var('--on-accent')))
finally:
    kill(p)

prof = tempfile.mkdtemp(prefix='cons-prof-')
p = start(prof)
try:
    check('Default adds no colour overrides', var('--bg') == '' and var('--panel-internal') == '' and var('--star') == '' and not var('--accent').startswith('#'), var('--accent'))
    open_appearance()
    card('Nebula')
    check('picking a preset paints it', var('--accent') == '#7aa2ff' and var('--panel-internal') == '#0e0828', var('--accent'))
    check('...and saves it', json.load(open(sf(prof)))['theme'] == 'Nebula')
    check('the real colours changed (computed)', ui("getComputedStyle(document.documentElement).getPropertyValue('--bg').trim()") == '#03000a')
    card('Default')
    check('back to Default removes every override', var('--bg') == '' and not var('--accent').startswith('#') and var('--star') == '')

    # editor
    btn('New theme')
    set_hex('Text', '#222222')
    check('unreadable text is explained', 'hard to read' in ui("document.querySelector('.settings-content').textContent"))
    btn('Save theme')
    check('...and saving is refused', 'hard to read' in msg() and json.load(open(sf(prof))).get('customThemes', {}) == {}, msg())
    set_hex('Text', '#eeeeee')
    set_hex('Background', '#ffffff')
    btn('Save theme')
    check('a light background is refused (dark only)', 'dark only' in msg(), msg())
    set_hex('Background', '#010203')
    set_hex('Accent', '#33ccaa')
    ui("document.querySelector('.settings-content [aria-label=\"Theme name\"]').value='Mine'; document.querySelector('.settings-content [aria-label=\"Theme name\"]').dispatchEvent(new Event('input',{bubbles:true})); 1")
    check('the editor previews live', var('--bg') == '#010203' and var('--accent') == '#33ccaa', var('--bg'))
    btn('Cancel')
    check('Cancel undoes the preview', var('--bg') == '' and not var('--accent').startswith('#'))
    btn('New theme')
    set_hex('Background', '#010203'); set_hex('Accent', '#33ccaa')
    ui("document.querySelector('.settings-content [aria-label=\"Theme name\"]').value='Mine'; document.querySelector('.settings-content [aria-label=\"Theme name\"]').dispatchEvent(new Event('input',{bubbles:true})); 1")
    btn('Save theme')
    disk = json.load(open(sf(prof)))
    tid = disk['theme']
    check('a good theme saves and becomes active', disk['customThemes'].get(tid, {}).get('name') == 'Mine' and var('--accent') == '#33ccaa', disk)

    # export / import
    btn('Export')
    text = ui("document.querySelector('.settings-content textarea').value")
    check('Export gives theme text', '"name": "Mine"' in text and '#33ccaa' in text, text[:80])
    btn('Import')
    for bad in ['not json', '{"name":"x"}', json.dumps(dict(json.loads(text), accent='url(javascript:1)'))]:
        ui("(()=>{const t=document.querySelector('.settings-content textarea'); t.value=%s; return 1})()" % json.dumps(bad))
        btn('Import now')
        check('Import refuses: ' + bad[:30], msg() != '' and len(json.load(open(sf(prof)))['customThemes']) == 1, msg())
    ui("(()=>{const t=document.querySelector('.settings-content textarea'); t.value=%s; return 1})()" % json.dumps(text.replace('Mine', 'Copy')))
    btn('Import now')
    disk = json.load(open(sf(prof)))
    check('Import accepts a good theme', len(disk['customThemes']) == 2 and disk['customThemes'][disk['theme']]['name'] == 'Copy', disk['customThemes'])
    btn('Delete')
    disk = json.load(open(sf(prof)))
    check('Delete removes it and returns to Default', len(disk['customThemes']) == 1 and disk['theme'] == 'Default' and not var('--accent').startswith('#'))
    card('Mine')
    # screenshot for the owner
    open_appearance()
    ui("[...document.querySelectorAll('.settings-content button')].find(b=>b.textContent==='Edit').click(); 1"); time.sleep(0.5)
    ui("document.querySelector('.settings-content').scrollTop=0; 1")
    subprocess.run(['powershell', '-ExecutionPolicy', 'Bypass', '-File', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'capture_seq.ps1'), '-Prefix', os.path.join(ROOT, 'out', 'live', 'settings-themes'), '-Count', '1', '-IntervalMs', '10'], capture_output=True)
finally:
    kill(p)

p = start(prof)
try:
    check('the chosen theme is still applied after a restart', var('--accent') == '#33ccaa', var('--accent'))
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
