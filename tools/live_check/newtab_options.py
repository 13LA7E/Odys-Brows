"""AUDIT-37: new-tab clock options and the wallpaper, through real settings and the real bridge."""
import os, sys, zlib, struct, base64
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

S = lambda e: ui('JSON.stringify(%s)' % e)


def png(w=64, h=48, rgb=(200, 60, 60)):
    raw = b''.join(b'\x00' + bytes(rgb) * w for _ in range(h))
    def chunk(t, d):
        c = struct.pack('>I', len(d)) + t + d
        return c + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(raw)) + chunk(b'IEND', b'')


def durl(mime, b): return 'data:%s;base64,%s' % (mime, base64.b64encode(b).decode())


def header(): return S("(document.querySelector('.newtab-header')||{}).innerText || ''")


def save(d):
    bridge({'type': 'save-settings', 'settings': d}); time.sleep(0.8); bridge({'type': 'load-settings'}); time.sleep(1.2)


def pick(path):
    """Choose a file in the real Settings file input (what a click on Choose image does)."""
    c = cdp.ws_connect(ui_ws())
    def call(i, method, params):
        cdp.send(c, {"id": i, "method": method, "params": params})
        while True:
            m = cdp.recv(c)
            if m.get('id') == i: return m
    doc = call(1, 'DOM.getDocument', {'depth': 0})['result']['root']['nodeId']
    n = call(2, 'DOM.querySelector', {'nodeId': doc, 'selector': 'input[aria-label="Wallpaper file"]'})['result']['nodeId']
    call(3, 'DOM.setFileInputFiles', {'files': [path], 'nodeId': n})
    c.close()


def wp_present(): return ui("!!document.querySelector('.newtab-wallpaper')")


prof = tempfile.mkdtemp(prefix='cons-prof-')
root = os.path.join(prof, 'Constellations')
p = launch([], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(3)
    h = json.loads(header())
    import re
    check('default: greeting, 24-hour clock and date are shown', bool(re.search(r'Good (morning|afternoon|evening|night)', h)) and bool(re.search(r'\b\d\d:\d\d\b', h)), h)
    check('default: hint is shown', ui("!!document.querySelector('.newtab-hint')") is True)
    check('default: no wallpaper element at all', wp_present() is False)
    save({'clockFormat': '12-hour'})
    check('12-hour clock', bool(re.search(r'\d{1,2}:\d\d (AM|PM)', json.loads(header()))), json.loads(header()))
    save({'greetingName': 'Sam'})
    check('the name is added to the greeting', ', Sam' in json.loads(header()), json.loads(header()))
    save({'greetingName': '<img src=x onerror=alert(1)>'})
    check('a name is shown as text, never as markup', ui("document.querySelector('.newtab-header img')") is None and '<img' in json.loads(header()), json.loads(header()))
    save({'showGreeting': False})
    check('greeting off removes the greeting line', not re.search(r'Good (morning|afternoon|evening|night)', json.loads(header())), json.loads(header()))
    save({'showGreeting': False, 'showClock': False})
    check('clock off and greeting off: no header at all', ui("document.querySelector('.newtab-header')") is None)
    save({'showClock': False, 'showGreeting': False, 'showTrailHint': False})
    check('hint off', ui("document.querySelector('.newtab-hint')") is None)
    save({})
    check('back to defaults shows everything again', 'Good' in json.loads(header()) and ui("!!document.querySelector('.newtab-hint')"))

    # wallpaper
    good = durl('image/png', png())
    bridge({'type': 'set-wallpaper', 'data': good}); time.sleep(1.5)
    files = [f for f in os.listdir(root) if f.startswith('wallpaper.')]
    check('a PNG is saved in the profile', files == ['wallpaper.png'], files)
    check('the new tab draws it', wp_present() is True)
    bg = json.loads(S("getComputedStyle(document.querySelector('.newtab-wallpaper')).backgroundImage"))
    check('...as the background image', bg.startswith('url("data:image/png'), bg[:40])
    dim = json.loads(S("getComputedStyle(document.querySelector('.newtab-wallpaper-dim')).backgroundColor"))
    check('default dim is 40%', dim == 'rgba(0, 0, 0, 0.4)', dim)
    save({'wallpaperDim': '80%', 'wallpaperBlur': 'Strong'})
    dim = json.loads(S("getComputedStyle(document.querySelector('.newtab-wallpaper-dim')).backgroundColor"))
    flt = json.loads(S("getComputedStyle(document.querySelector('.newtab-wallpaper')).filter"))
    check('dim 80% and blur Strong apply', dim == 'rgba(0, 0, 0, 0.8)' and 'blur(16px)' in flt, (dim, flt))

    # refused files
    for name, bad in [('html as png', durl('image/png', b'<html>hi</html>')), ('svg', durl('image/svg+xml', b'<svg/>')),
                      ('text', 'hello'), ('too big', durl('image/png', b'\x89PNG\r\n\x1a\n' + b'0' * (4 * 1024 * 1024 + 10)))]:
        bridge({'type': 'set-wallpaper', 'data': bad}); time.sleep(1.0)
        files = [f for f in os.listdir(root) if f.startswith('wallpaper.')]
        err = ui("(window.__uiState.wallpaper||{}).error")
        check('refused: ' + name, files == ['wallpaper.png'] and bool(err), (files, err))
    # settings page shows the controls and the message
    bridge({'type': 'open', 'url': 'odyssey://settings'}); time.sleep(0.8)
    ui("[...document.querySelectorAll('.page-internal *')].filter(e=>e.children.length===0&&e.textContent==='New tab')[0].click(); 1"); time.sleep(0.6)
    t = ui("document.querySelector('.settings-content').textContent")
    check('Settings offers Change image, Remove, Dim and Blur when a wallpaper exists', all(x in t for x in ['Change image', 'Remove', 'Dim', 'Blur']), t[:300])
    # the real file picker, with a big (but allowed) image and one that is too big
    f1 = os.path.join(prof, 'big.png'); open(f1, 'wb').write(png() + b'0' * 3_900_000)
    pick(f1); time.sleep(4)
    sz = os.path.getsize(os.path.join(root, 'wallpaper.png'))
    check('a 3.9 MB image chosen in Settings is saved', sz > 3_900_000, sz)
    f2 = os.path.join(prof, 'over.png'); open(f2, 'wb').write(png() + b'0' * 4_300_000)
    pick(f2); time.sleep(3)
    check('a 4.3 MB image is refused with a message and the old one stays', os.path.getsize(os.path.join(root, 'wallpaper.png')) == sz and 'too big' in ui("document.querySelector('.settings-content').textContent"))
    # a web page does not get it (wallpaper is new-tab only)
    bridge({'type': 'open', 'url': 'http://127.0.0.1:%d/a.html' % SRV}); time.sleep(2.5)
    check('on a web page the wallpaper is not drawn', wp_present() is False)
    bridge({'type': 'open', 'url': ''}); time.sleep(1.5)
    check('on a fresh new tab it is drawn again', wp_present() is True)
    ui("window.scrollTo(0,0); 1")
    PS = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'capture_seq.ps1')
    save({'wallpaperDim': '40%', 'wallpaperBlur': 'Off', 'greetingName': 'Sam', 'clockFormat': '24-hour'})
    time.sleep(0.5)
    subprocess.run(['powershell', '-ExecutionPolicy', 'Bypass', '-File', PS, '-Prefix', os.path.join(ROOT, 'out', 'live', 'newtab-wallpaper'), '-Count', '1', '-IntervalMs', '10'], capture_output=True)
finally:
    kill(p)

p = launch([], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(4)
    check('after a restart the wallpaper is still there', wait(lambda: wp_present() is True, 8) is not None)
    bridge({'type': 'clear-wallpaper'}); time.sleep(1.2)
    files = [f for f in os.listdir(root) if f.startswith('wallpaper.')]
    check('Remove deletes the file and the layer', files == [] and wp_present() is False, files)
finally:
    kill(p)
# a damaged file is ignored
open(os.path.join(root, 'wallpaper.png'), 'wb').write(b'not a png')
p = launch([], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(4)
    check('a damaged wallpaper file is ignored', wp_present() is False)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
