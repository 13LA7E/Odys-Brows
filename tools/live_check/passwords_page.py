"""The Passwords settings page: no descriptive text, Add/Import/Export, and import through the real
file input (DOM.setFileInputFiles stands in for the native file dialog)."""
import os, sys, subprocess
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

S = lambda e: ui('JSON.stringify(%s)' % e)
PS = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'capture_seq.ps1')
prof = tempfile.mkdtemp(prefix='cons-prof-')
p = launch([], prof)
csv_path = os.path.join(tempfile.mkdtemp(prefix='cons-csv-'), 'pw.csv')
open(csv_path, 'w', encoding='utf8').write('name,url,username,password,note\r\nexample.com,https://example.com/,me@example.com,s3cret,\r\ngithub.com,https://github.com/login,octocat,"p,w",\r\n')
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30)
    time.sleep(3)
    bridge({'type': 'open', 'url': 'odyssey://settings'})
    time.sleep(1)
    ui("window.__uiState.settingsSection='Passwords'; 1")
    ev(page_ws('x') or ui_ws(), "1")
    bridge({'type': 'open', 'url': 'odyssey://settings'})
    time.sleep(0.8)
    # click the Passwords nav entry
    ui("[...document.querySelectorAll('.page-internal *')].filter(e=>e.children.length===0&&e.textContent==='Passwords')[0].click(); 1")
    time.sleep(0.8)
    txt = ui("document.querySelector('.settings-content').innerText")
    print('page text:', repr(txt))
    check('Passwords page has no descriptive sentences', 'DPAPI' not in txt and 'No passwords saved' not in txt and 'encrypted' not in txt, txt)
    check('it has the two switches and Add / Import / Export', all(w in txt for w in ('Offer to save passwords', 'Autofill passwords', 'Add', 'Import', 'Export')), txt)
    # import through the real file input
    s = cdp.ws_connect(ui_ws())
    cdp.send(s, {"id": 1, "method": "DOM.getDocument", "params": {}})
    root = None
    while True:
        m = cdp.recv(s)
        if m.get('id') == 1:
            root = m['result']['root']['nodeId']
            break
    cdp.send(s, {"id": 2, "method": "DOM.querySelector", "params": {"nodeId": root, "selector": "input[type=file]"}})
    while True:
        m = cdp.recv(s)
        if m.get('id') == 2:
            node = m['result']['nodeId']
            break
    cdp.send(s, {"id": 3, "method": "DOM.setFileInputFiles", "params": {"nodeId": node, "files": [csv_path]}})
    while cdp.recv(s).get('id') != 3:
        pass
    s.close()
    time.sleep(1.5)
    ents = json.loads(S('window.__uiState.vaultEntries||[]'))
    check('choosing a CSV in the file input imports it', len(ents) == 2, ents)
    txt = ui("document.querySelector('.settings-content').innerText")
    check('the page shows the imported logins and a short result', 'example.com' in txt and 'Imported 2' in txt, txt)
    out = os.path.join(ROOT, 'out', 'live', 'passwords-page')
    subprocess.run(['powershell', '-ExecutionPolicy', 'Bypass', '-File', PS, '-Prefix', out, '-Count', '1', '-IntervalMs', '10'], capture_output=True)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
