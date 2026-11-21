import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

open(os.path.join(site, 'login.html'), 'w').write('<!doctype html><title>Login</title><form method=post action=/done.html><input id=u type=text name=u><br><input id=p type=password name=p><br><button id=go type=submit>Go</button></form><div style="height:600px"></div>')
open(os.path.join(site, 'done.html'), 'w').write('<!doctype html><title>Done</title><p>ok</p>')

prof = tempfile.mkdtemp(prefix='cons-prof-')
os.makedirs(os.path.join(prof, 'Constellations'), exist_ok=True)
S = lambda e: ui(f'JSON.stringify({e})')


def trusted_click(pw, selector):
    r = json.loads(ev(pw, "JSON.stringify((()=>{const r=document.querySelector('%s').getBoundingClientRect();return {x:r.left+r.width/2,y:r.top+r.height/2}})())" % selector))
    s = cdp.ws_connect(pw)
    for i, t in enumerate(('mousePressed', 'mouseReleased')):
        cdp.send(s, {"id": i + 1, "method": "Input.dispatchMouseEvent", "params": {"type": t, "x": r['x'], "y": r['y'], "button": "left", "clickCount": 1}})
        while cdp.recv(s).get('id') != i + 1:
            pass
    s.close()


def form(pw):
    return json.loads(ev(pw, "JSON.stringify({u:document.getElementById('u').value,p:document.getElementById('p').value})"))


def fill_and_submit(pw, user, pwd):
    js = ("(()=>{const u=document.getElementById('u'),p=document.getElementById('p');u.value=%s;p.value=%s;"
          "u.dispatchEvent(new Event('input',{bubbles:true}));p.dispatchEvent(new Event('input',{bubbles:true}));"
          "document.getElementById('go').click()})()") % (json.dumps(user), json.dumps(pwd))
    ev(pw, js, gesture=True)


def prompt(kind, t=6):
    return wait(lambda: json.loads(S('(window.__uiState.prompts||[]).find(p=>p.kind=="%s")||null' % kind)) or None, t)


def card_click(label):
    ui("[...document.querySelectorAll('.permission-card button')].find(b=>b.textContent===%s).click(); 1" % json.dumps(label))


def vault():
    return json.loads(S('window.__uiState.vaultEntries||[]'))


def open_login(host='127.0.0.1'):
    bridge({'type': 'navigate', 'view': json.loads(S('window.__uiState.activeTab')), 'url': 'http://%s:%d/login.html' % (host, SRV)})
    time.sleep(2.5)
    return wait(lambda: page_ws('/login.html'), 8)


urls = os.path.join(prof, 'urls.txt')
open(urls, 'w').write('http://127.0.0.1:%d/login.html\n' % SRV)
p = launch(['--urls-file=' + urls], prof)
csv = ''
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    time.sleep(3)
    bridge({'type': 'vault-list'})
    time.sleep(0.5)
    pw = wait(lambda: page_ws('/login.html'), 10)

    # 1. detect + prompt; the password never reaches the UI
    fill_and_submit(pw, 'alice', 'hunter2')
    pr = prompt('save-login')
    check('submitting a login offers to save it (like Brave)', bool(pr) and pr['username'] == 'alice' and pr['host'] == '127.0.0.1:8765' and pr['update'] is False, pr)
    check('the password is never sent to the UI', 'hunter2' not in ui('JSON.stringify(window.__uiState)'))
    # 2. save
    time.sleep(0.8)
    card_click('Save')
    time.sleep(1.2)
    v = vault()
    check('Save stores it in the vault', len(v) == 1 and v[0]['username'] == 'alice', v)

    # 3. autofill on a real click
    pw = open_login()
    trusted_click(pw, '#u')
    time.sleep(1.2)
    f1 = form(pw)
    check('a real click on the login field autofills the saved login', f1 == {'u': 'alice', 'p': 'hunter2'}, f1)
    # 4. fake clicks / focus do not
    pw = open_login()
    ev(pw, "document.getElementById('u').click(); document.getElementById('u').focus(); document.getElementById('p').click()", gesture=True)
    time.sleep(1.2)
    check('script-made clicks and focus do NOT autofill (pages cannot pull passwords)', form(pw) == {'u': '', 'p': ''}, form(pw))

    # 5. update prompt when the password changed
    fill_and_submit(pw, 'alice', 'new-pass-9')
    pr = prompt('save-login')
    check('a changed password offers to update', bool(pr) and pr['update'] is True, pr)
    time.sleep(0.6)
    card_click('Update')
    time.sleep(1.2)
    pw = open_login()
    trusted_click(pw, '#u')
    time.sleep(1.2)
    check('autofill uses the updated password', form(pw) == {'u': 'alice', 'p': 'new-pass-9'}, form(pw))

    # 6. second account -> chooser
    ui("window.__uiState.prompts=[]; 1")
    fill_and_submit(pw, 'bob', 'bobpw')
    prompt('save-login')
    time.sleep(0.6)
    card_click('Save')
    time.sleep(1.2)
    pw = open_login()
    trusted_click(pw, '#u')
    ch = prompt('fill-choice')
    check('two saved logins show a chooser (usernames only)', bool(ch) and sorted(l['username'] for l in ch['logins']) == ['alice', 'bob'] and 'bobpw' not in json.dumps(ch), ch)
    time.sleep(0.6)
    card_click('bob')
    time.sleep(1.2)
    check('choosing fills that login', form(pw) == {'u': 'bob', 'p': 'bobpw'}, form(pw))

    # 7. Never
    pw = open_login('localhost')
    fill_and_submit(pw, 'carol', 'cpw')
    prompt('save-login')
    time.sleep(0.6)
    card_click('Never')
    time.sleep(1.0)
    nv = json.loads(S('window.__uiState.vaultNever'))
    check('Never remembers the site', nv == ['localhost:8765'], nv)
    ui("window.__uiState.prompts=[]; 1")
    pw = open_login('localhost')
    fill_and_submit(pw, 'carol', 'cpw')
    time.sleep(1.5)
    check('no prompt again for a Never site', prompt('save-login', 2) is None)
    bridge({'type': 'vault-remove-never', 'host': 'localhost:8765'})
    time.sleep(0.8)
    pw = open_login('localhost')
    fill_and_submit(pw, 'carol', 'cpw')
    check('removing it from the Never list brings the prompt back', bool(prompt('save-login')))
    ui("window.__uiState.prompts=[]; 1")

    # 8. settings switches
    off = {'stars': 'Still', 'accent': 'Sea', 'trail': 'Narrow', 'connections': 8, 'offerSavePasswords': False, 'autofillPasswords': False}
    bridge({'type': 'save-settings', 'settings': off})
    time.sleep(0.8)
    pw = open_login()
    fill_and_submit(pw, 'dave', 'dpw')
    time.sleep(1.5)
    check('"Offer to save passwords" off: no prompt', prompt('save-login', 2) is None)
    pw = open_login()
    trusted_click(pw, '#u')
    time.sleep(1.2)
    check('"Autofill passwords" off: nothing is filled', form(pw) == {'u': '', 'p': ''}, form(pw))
    on = dict(off, offerSavePasswords=True, autofillPasswords=True)
    bridge({'type': 'save-settings', 'settings': on})
    time.sleep(0.5)

    # 9. export
    bridge({'type': 'vault-export'})
    time.sleep(1.5)
    ex = json.loads(S('window.__uiState.vaultExport'))
    csv = open(ex['path'], encoding='utf8').read() if ex and ex.get('ok') else ''
    print('exported to', ex and ex.get('path'))
    check('export writes a CSV with the logins', 'alice' in csv and 'new-pass-9' in csv and csv.startswith('name,url,username,password'), csv[:80])
    try:
        os.remove(ex['path'])
    except Exception:
        pass
finally:
    kill(p)

# 10. import into a fresh profile
prof2 = tempfile.mkdtemp(prefix='cons-prof-')
p = launch([], prof2)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    time.sleep(2)
    bridge({'type': 'vault-import', 'csv': csv})
    time.sleep(1.5)
    v = vault()
    imp = json.loads(S('window.__uiState.vaultImport'))
    check('import adds the logins from the CSV', len(v) == 2 and imp['added'] == 2, (len(v), imp))
    bridge({'type': 'vault-import', 'csv': csv})
    time.sleep(1.2)
    imp = json.loads(S('window.__uiState.vaultImport'))
    check('importing again skips duplicates', imp['added'] == 0 and imp['skipped'] == 2, imp)
    bridge({'type': 'vault-import', 'csv': 'not,a,passwords,file\n1,2,3,4'})
    time.sleep(1.0)
    imp = json.loads(S('window.__uiState.vaultImport'))
    check('a wrong file gives an error and adds nothing', bool(imp['error']) and len(vault()) == 2, imp)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
