import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *
prof = tempfile.mkdtemp(prefix='cons-prof-')
p = launch([], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(2)
    # new-tab page, Drifting
    bridge({'type': 'save-settings', 'settings': {'stars': 'Drifting', 'accent': 'Sea', 'trail': 'Narrow', 'connections': 8}})
    ui("window.__uiState.starsMode='drifting'; window.__uiState.settings={stars:'Drifting'}; window.dispatchEvent(new Event('resize')); 1"); time.sleep(1.5)
    check('drifting sky runs on the new-tab page', ui('window.__skyLoopCount()') == 1, ui('window.__skyLoopCount()'))
    ui("window.__skyAge && 0; 1")
    # age the activity past the limit by shifting the clock reference through the sky handle
    ui("(()=>{ for (const c of document.querySelectorAll('canvas.sky')) {} return 1 })()")
    time.sleep(34)
    check('sky loop stops after ~30 s without activity (AUDIT-34)', ui('window.__skyLoopCount()') == 0, ui('window.__skyLoopCount()'))
    ui("window.dispatchEvent(new Event('pointermove')); 1"); time.sleep(0.5)
    check('sky resumes on pointer activity', ui('window.__skyLoopCount()') == 1, ui('window.__skyLoopCount()'))
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
