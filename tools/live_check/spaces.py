"""Spaces (Home, Work, Personal): switching shows only that space's tabs, an empty space opens a new tab,
switching back returns to the tab you had, and the spaces survive a restart."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

S = lambda e: ui('JSON.stringify(%s)' % e)
V = lambda e: json.loads(S(e))
A = 'http://127.0.0.1:%d/a.html' % SRV
B = 'http://127.0.0.1:%d/b.html' % SRV


def active_space_of_tab():
    return V("(()=>{const t=window.__uiState.tabs.find(t=>t.id===window.__uiState.activeTab); return t?t.space:null})()")


def rail_labels():
    return V("[...document.querySelectorAll('.rail button')].map(b=>b.textContent.trim()).filter(Boolean)")


prof = tempfile.mkdtemp(prefix='cons-prof-')
urls = os.path.join(prof, 'urls.txt'); open(urls, 'w').write(A + '\n')
p = launch(['--urls-file=' + urls], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(4)
    ui("window.__uiState.rail='expanded'; 1"); bridge({'type': 'load-settings'}); time.sleep(1)
    first = V('window.__uiState.activeTab')
    check('the first tab is in space 0 (Home)', active_space_of_tab() == 0)
    bridge({'type': 'switch-space', 'space': 1}); time.sleep(2)
    check('switching to the empty Work space opens a new tab there', V('window.__uiState.activeSpace') == 1 and active_space_of_tab() == 1 and V('window.__uiState.activeTab') != first, V('window.__uiState.tabs'))
    check('the Home tab is not listed in Work', all('Page A' not in x for x in rail_labels()), rail_labels())
    check('the page of Home is no longer shown (new tab page)', V('window.__uiState.page') == 'none')
    bridge({'type': 'open', 'url': B}); time.sleep(2.5)
    check('a page opened in Work belongs to Work', active_space_of_tab() == 1)
    bridge({'type': 'switch-space', 'space': 0}); time.sleep(2)
    check('back in Home you are on the tab you left', V('window.__uiState.activeTab') == first and V('window.__uiState.page') == 'web', V('window.__uiState.activeTab'))
    check('Home lists only its own tab', any('Page A' in x for x in rail_labels()) and all('Page B' not in x for x in rail_labels()), rail_labels())
    bridge({'type': 'switch-space', 'space': 2}); time.sleep(2)
    check('Personal also opens empty with a new tab', V('window.__uiState.activeSpace') == 2 and active_space_of_tab() == 2)
    bridge({'type': 'switch-space', 'space': 1}); time.sleep(2)
    check('Work still has its two tabs and shows the page you were on', V("window.__uiState.tabs.filter(t=>t.space===1).length") == 2 and V('window.__uiState.page') == 'web', V('window.__uiState.page'))
    bridge({'type': 'close-window'})
    try: p.wait(timeout=40)
    except Exception: pass
finally:
    subprocess.run(['taskkill', '/F', '/IM', 'constellations.exe'], capture_output=True); time.sleep(2)
p = launch([], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(4)
    check('after a restart you are back in Work', V('window.__uiState.activeSpace') == 1, V('window.__uiState.activeSpace'))
    sp = V("window.__uiState.tabs.map(t=>t.space).sort()")
    check('every tab kept its space after a restart', sp == [0, 1, 1, 2], sp)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
