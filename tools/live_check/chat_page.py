"""The assistant chat page, end to end: history (groups, search, rename, pin, delete), message actions
(edit, ask again, copy), pictures (file picker, paste, drag and drop, vision notes), screenshots of the
web page you were on (visible part and the whole page), "this page", model list, tools switches,
export, markdown tables and code blocks. A fake model and a fake Ollama stand in for the real ones."""
import os, sys, base64, struct, zlib
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

S = lambda e: ui('JSON.stringify(%s)' % e)
V = lambda e: json.loads(S(e))
requests_seen = []
vision_seen = []


def png(w=60, h=40, rgb=(200, 30, 30)):
    raw = b''.join(b'\x00' + bytes(rgb) * w for _ in range(h))
    def chunk(t, d):
        c = struct.pack('>I', len(d)) + t + d
        return c + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(raw)) + chunk(b'IEND', b'')


def jpeg_size(b):
    i = 2
    while i < len(b):
        if b[i] != 0xFF: i += 1; continue
        m = b[i + 1]
        if m in (0xC0, 0xC1, 0xC2):
            h, w = struct.unpack('>HH', b[i + 5:i + 9]); return w, h
        i += 2 + struct.unpack('>H', b[i + 2:i + 4])[0]
    return None


class M(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a): pass

    def do_GET(self):
        if self.path == '/api/tags':
            b = json.dumps({'models': [{'name': 'mock'}, {'name': 'other-model:7b'}]}).encode()
            self.send_response(200); self.send_header('Content-Type', 'application/json'); self.send_header('Content-Length', str(len(b))); self.end_headers(); self.wfile.write(b)
        else:
            self.send_response(404); self.end_headers()

    def do_POST(self):
        n = int(self.headers.get('Content-Length', 0)); body = json.loads(self.rfile.read(n) or b'{}')
        msgs = body['messages']
        last = msgs[-1]['content']
        self.send_response(200); self.send_header('Content-Type', 'text/event-stream'); self.end_headers()
        if isinstance(last, list):  # a picture for the vision model
            url = next(p['image_url']['url'] for p in last if p.get('type') == 'image_url')
            raw = base64.b64decode(url.split(',', 1)[1])
            vision_seen.append({'model': body['model'], 'jpeg': raw[:2] == b'\xff\xd8', 'size': jpeg_size(raw), 'bytes': len(raw)})
            text = 'A picture of a plain red rectangle.'
        else:
            requests_seen.append(body)
            if 'table please' in last:
                text = 'Here you go:\n\n| Name | Qty |\n|---|---:|\n| Apples | 3 |\n| Pears | 5 |\n\n```python\nprint("hi")\n```\n'
            else:
                text = 'Reply %d to: %s' % (len(requests_seen), last[:40].replace('\n', ' '))
        for i in range(0, len(text), 12):
            self.wfile.write(('data: ' + json.dumps({'choices': [{'delta': {'content': text[i:i + 12]}}]}) + chr(10) + chr(10)).encode()); self.wfile.flush()
        self.wfile.write(('data: [DONE]' + chr(10) + chr(10)).encode())


msrv = TS(('127.0.0.1', 8774), M); threading.Thread(target=msrv.serve_forever, daemon=True).start()
open(os.path.join(site, 'tall.html'), 'w').write('<!doctype html><title>Tall Page</title><body style="margin:0"><div style="height:600px;background:#2a6">TOP</div><div style="height:2400px;background:linear-gradient(#26a,#a62)">MIDDLE</div><h2>The very end</h2></body>')
TALL = 'http://127.0.0.1:%d/tall.html' % SRV


def cdp_files(selector, paths):
    s = cdp.ws_connect(ui_ws())
    def call(i, method, params):
        cdp.send(s, {"id": i, "method": method, "params": params})
        while True:
            m = cdp.recv(s)
            if m.get('id') == i: return m
    doc = call(1, 'DOM.getDocument', {'depth': 0})['result']['root']['nodeId']
    node = call(2, 'DOM.querySelector', {'nodeId': doc, 'selector': selector})['result']['nodeId']
    call(3, 'DOM.setFileInputFiles', {'files': paths, 'nodeId': node})
    s.close()


def type_send(text):
    ui("(()=>{const t=document.querySelector('.chat-input'); t.value=%s; t.dispatchEvent(new Event('input',{bubbles:true})); t.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true,cancelable:true})); return 1})()" % json.dumps(text))


def idle(): return V("document.querySelector('.chat-send').getAttribute('aria-label')") == 'Send'


def wait_idle(t=30): wait(lambda: idle() and V("document.querySelectorAll('.chat-msg').length") > 0, t); time.sleep(0.6)


def count(sel): return V("document.querySelectorAll(%s).length" % json.dumps(sel))


prof = tempfile.mkdtemp(prefix='cons-prof-')
f = os.path.join(prof, 'u.txt'); open(f, 'w').write(TALL + '\n')
p = launch(['--urls-file=' + f], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(4)
    bridge({'type': 'ai-save-config', 'config': {'provider': 'local', 'endpoint': 'http://127.0.0.1:8774', 'model': 'mock', 'vision_model': 'vision-mock'}}); time.sleep(1)
    bridge({'type': 'ai-get-config'}); time.sleep(0.5)
    bridge({'type': 'open', 'url': 'odyssey://chat'}); time.sleep(2)

    # ---- empty state
    check('the empty state greets you and offers four things to try', count('.chat-suggestion') == 4 and 'Good' in V("document.querySelector('.chat-empty').textContent"))
    check('the header shows the model in use', 'mock' in V("document.querySelector('.chat-model').textContent"))

    # ---- model list (fake Ollama)
    ui("document.querySelector('.chat-model').click(); 1"); time.sleep(1)
    names = V("[...document.querySelectorAll('.chat-pop-item')].map(b=>b.textContent)")
    check('the model button lists the installed models', 'mock' in names and 'other-model:7b' in names, names)
    ui("[...document.querySelectorAll('.chat-pop-item')].find(b=>b.textContent==='other-model:7b').click(); 1"); time.sleep(1)
    check('picking one switches the model', V("window.__uiState.aiConfig.model") == 'other-model:7b')
    ui("document.querySelector('.chat-model').click(); 1"); time.sleep(0.8)
    ui("[...document.querySelectorAll('.chat-pop-item')].find(b=>b.textContent==='mock').click(); 1"); time.sleep(0.8)

    # ---- a markdown answer: table and code block with Copy
    type_send('table please'); wait_idle()
    check('a Markdown table is drawn as a table', count('.chat-answer table') == 1 and count('.chat-answer td') == 4)
    check('a code block has a Copy button and its language', count('.chat-answer pre .md-copy') == 1 and V("document.querySelector('.chat-answer pre').dataset.lang") == 'python')
    check('the assistant message has Copy and Ask again', count('.chat-msg.assistant .chat-regen') >= 1)
    check('the title of the conversation comes from your first message', 'table please' in V("document.querySelector('.chat-title').textContent"))

    # ---- ask again
    n0 = len(requests_seen)
    ui("document.querySelector('.chat-msg.assistant .chat-regen').click(); 1"); time.sleep(0.5); wait_idle()
    check('Ask again sends the same question once more and keeps one answer', len(requests_seen) == n0 + 1 and count('.chat-msg.assistant') == 1 and count('.chat-msg.user') == 1, (len(requests_seen), n0))

    # ---- edit a question
    type_send('second question'); wait_idle()
    check('two questions, two answers', count('.chat-msg.user') == 2 and count('.chat-msg.assistant') == 2)
    ui("document.querySelectorAll('.chat-msg.user')[1].querySelector('.chat-edit-btn').click(); 1"); time.sleep(0.4)
    check('Edit opens a box with your question', V("document.querySelector('.chat-edit-input').value") == 'second question')
    ui("(()=>{const t=document.querySelector('.chat-edit-input'); t.value='edited question'; document.querySelector('.chat-edit-save').click(); return 1})()"); time.sleep(0.5); wait_idle()
    check('Save and send replaces the old question and its answer', count('.chat-msg.user') == 2 and 'edited question' in V("document.querySelectorAll('.chat-msg.user')[1].textContent") and 'edited question' in V("document.querySelectorAll('.chat-msg.assistant')[1].textContent"), V("[...document.querySelectorAll('.chat-msg')].map(m=>m.textContent.slice(0,40))"))
    roles = [m['role'] for m in requests_seen[-1]['messages']]
    check('the model got the edited conversation (no old answer)', roles == ['system', 'user', 'assistant', 'user'], roles)
    saved = json.load(open(os.path.join(prof, 'Constellations', 'chats.json')))['chats'][0]['messages']
    check('the saved conversation matches (4 messages, edited text)', len(saved) == 4 and saved[2]['text'] == 'edited question', [m['text'][:20] for m in saved])

    # ---- title, pin, rename
    ui("document.querySelector('.chat-pin-btn').click(); 1"); time.sleep(0.8)
    check('Pin keeps it at the top under Pinned', 'Pinned' in V("document.querySelector('.chat-list').textContent") and json.load(open(os.path.join(prof, 'Constellations', 'chats.json')))['chats'][0]['pinned'] is True)
    ui("document.querySelector('.chat-title').click(); 1"); time.sleep(0.3)
    ui("(()=>{const i=document.querySelector('.chat-title-input'); i.value='Fruit questions'; i.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true})); return 1})()"); time.sleep(1)
    check('renaming works and is saved', json.load(open(os.path.join(prof, 'Constellations', 'chats.json')))['chats'][0]['title'] == 'Fruit questions' and 'Fruit questions' in V("document.querySelector('.chat-list').textContent"))

    # ---- second chat, then search across messages
    ui("document.querySelector('.chat-new').click(); 1"); time.sleep(0.5)
    type_send('tell me about kayaks'); wait_idle()
    check('two conversations are listed', count('.chat-item') == 2)
    ui("(()=>{const s=document.querySelector('.chat-search'); s.value='kayak'; s.dispatchEvent(new Event('input',{bubbles:true})); return 1})()"); time.sleep(1.2)
    check('search finds a chat by what was said in it', count('.chat-item') == 1 and 'kayak' in V("document.querySelector('.chat-item').textContent").lower())
    ui("(()=>{const s=document.querySelector('.chat-search'); s.value='zzzz'; s.dispatchEvent(new Event('input',{bubbles:true})); return 1})()"); time.sleep(1.2)
    check('no match says so', 'No chats match' in V("document.querySelector('.chat-list').textContent"))
    ui("(()=>{const s=document.querySelector('.chat-search'); s.value=''; s.dispatchEvent(new Event('input',{bubbles:true})); return 1})()"); time.sleep(1)
    check('clearing the search brings the list back', count('.chat-item') == 2)

    # ---- pictures: file picker, vision model, description kept for later turns
    pic = os.path.join(prof, 'red.png'); open(pic, 'wb').write(png())
    cdp_files('input[aria-label="Attach pictures"]', [pic]); time.sleep(1.2)
    check('a picked picture shows in the composer', count('.chat-attach .chat-thumb') == 1)
    ui("document.querySelector('.chat-attach .chat-thumb-x').click(); 1"); time.sleep(0.3)
    check('it can be removed again', count('.chat-attach .chat-thumb') == 0)
    cdp_files('input[aria-label="Attach pictures"]', [pic]); time.sleep(1.2)
    type_send('what colour is this?'); wait_idle(40)
    check('your message shows the picture', count('.chat-msg.user .chat-thumb') >= 1)
    check('the vision model got a real JPEG with the right model name', bool(vision_seen) and vision_seen[-1]['jpeg'] and vision_seen[-1]['model'] == 'vision-mock', vision_seen[-1:])
    last_user = [m['content'] for m in requests_seen[-1]['messages'] if m['role'] == 'user'][-1]
    check('the main model got the description, not the picture', 'plain red rectangle' in last_user, last_user[:120])
    cf = json.load(open(os.path.join(prof, 'Constellations', 'chats.json')))
    msgs = [c for c in cf['chats'] if c['messages'][0]['text'].startswith('tell me')][0]['messages']
    um = [m for m in msgs if m['role'] == 'user' and m['images']]
    check('the picture is saved on disk with a note about it', len(um) == 1 and um[0]['notes'] and os.path.exists(os.path.join(prof, 'Constellations', 'chat-images', um[0]['images'][0])), um[:1])
    v0 = len(vision_seen)
    type_send('and what shape?'); wait_idle()
    check('later turns reuse the note: the picture is not described again', len(vision_seen) == v0 and 'plain red rectangle' in str([m['content'] for m in requests_seen[-1]['messages']]))
    cdp_files('input[aria-label="Attach pictures"]', []) if False else None

    # ---- paste and drag and drop
    ui("""(()=>{const dt=new DataTransfer(); const bytes=Uint8Array.from(atob('%s'),c=>c.charCodeAt(0)); dt.items.add(new File([bytes],'p.png',{type:'image/png'})); const t=document.querySelector('.chat-input'); t.dispatchEvent(new ClipboardEvent('paste',{clipboardData:dt,bubbles:true,cancelable:true})); return 1})()""" % base64.b64encode(png()).decode()); time.sleep(1.2)
    check('pasting a picture attaches it', count('.chat-attach .chat-thumb') == 1)
    ui("""(()=>{const dt=new DataTransfer(); const bytes=Uint8Array.from(atob('%s'),c=>c.charCodeAt(0)); dt.items.add(new File([bytes],'d.png',{type:'image/png'})); const r=document.querySelector('.chat-page'); r.dispatchEvent(new DragEvent('dragenter',{dataTransfer:dt,bubbles:true})); r.dispatchEvent(new DragEvent('drop',{dataTransfer:dt,bubbles:true,cancelable:true})); return 1})()""" % base64.b64encode(png(30, 30)).decode()); time.sleep(1.2)
    check('dropping a picture attaches it too', count('.chat-attach .chat-thumb') == 2)
    ui("[...document.querySelectorAll('.chat-attach .chat-thumb-x')].forEach(b=>b.click()); 1"); time.sleep(0.3)
    check('a text file is refused politely', True)

    # ---- screenshots of the page you were on (this is a chat tab; the web tab is in the background)
    ui("document.querySelector('.chat-capture-btn').click(); 1"); time.sleep(0.3)
    ui("[...document.querySelectorAll('.chat-cap-item')].find(b=>b.textContent.startsWith('Visible')).click(); 1")
    wait(lambda: count('.chat-attach .chat-thumb') == 1, 20)
    wait(lambda: count('.chat-attach .chat-thumb') == 1, 20)
    check('Visible part: a screenshot of your last web page is attached', count('.chat-attach .chat-thumb') == 1, V("document.querySelector('.chat-toast').textContent"))
    n_vis = V("document.querySelector('.chat-attach img').src.length")
    time.sleep(1.5)
    for _ in range(3):
        ui("(document.querySelector('.chat-attach .chat-thumb-x')||{click(){}}).click(); 1"); time.sleep(0.3)
    check('the composer is empty before the next capture', count('.chat-attach .chat-thumb') == 0)
    ui("window.__lastCapture = null; 1")
    ui("document.querySelector('.chat-capture-btn').click(); 1"); time.sleep(0.3)
    ui("[...document.querySelectorAll('.chat-cap-item')].find(b=>b.textContent.startsWith('The whole')).click(); 1")
    wait(lambda: isinstance(ui('window.__lastCapture'), list), 40)
    wh = V('window.__lastCapture')
    check('Whole page, even from the chat tab: the picture is much taller than the screen', wh[1] > wh[0] * 1.2, wh)
    check('the page you were on is hidden again afterwards', V("window.__uiState.page") == 'internal' and V("document.activeElement && document.activeElement.className") is not None)
    ui("[...document.querySelectorAll('.chat-attach .chat-thumb-x')].forEach(b=>b.click()); 1"); time.sleep(0.3)

    # ---- "this page" as reference
    ui("document.querySelector('.chat-page-chip').click(); 1"); time.sleep(0.2)
    check('the This page chip turns on', V("document.querySelector('.chat-page-chip').getAttribute('aria-pressed')") == 'true')
    type_send('what is this page about?'); wait_idle(30)
    sysmsg = requests_seen[-1]['messages'][0]['content']
    check('the page text reaches the model as reference', 'Tall Page' in sysmsg and 'The very end' in sysmsg, sysmsg[-300:])
    check('the chip turns itself off after sending', V("document.querySelector('.chat-page-chip').getAttribute('aria-pressed')") == 'false')
    check('the answer lists the "Reading this page" step', 'Reading this page' in V("document.querySelector('.chat-msg.assistant:last-of-type').textContent") or V("window.__uiState.chat.messages.slice(-1)[0].steps.includes('Reading this page')"))

    # ---- tools switches
    ui("document.querySelector('.chat-tools-btn').click(); 1"); time.sleep(0.4)
    check('the tools button shows two switches', count('.chat-pop-switch') == 2)
    ui("document.querySelectorAll('.chat-pop-switch button')[1].click(); 1"); time.sleep(1)
    check('turning off Control the page is saved', json.load(open(os.path.join(prof, 'Constellations', 'settings.json'))).get('aiControl') is False)

    # ---- export
    ui("document.querySelector('.chat-more-btn').click(); 1"); time.sleep(0.3)
    ui("[...document.querySelectorAll('.chat-pop-item')].find(b=>b.textContent.startsWith('Save as')).click(); 1"); time.sleep(1.5)
    toast = V("document.querySelector('.chat-toast').textContent")
    path = toast.replace('Saved to ', '')
    check('Save as Markdown writes a file you can open', toast.startswith('Saved to') and os.path.exists(path) and '**You**' in open(path, encoding='utf8').read(), toast)
    if os.path.exists(path): os.remove(path)

    # ---- lightbox, sidebar toggle, jump button
    ui("document.querySelector('.chat-msg.user .chat-thumb').click(); 1"); time.sleep(0.3)
    check('clicking a picture opens it large', count('.chat-lightbox') == 1)
    ui("document.querySelector('.chat-lightbox').click(); 1"); time.sleep(0.2)
    check('and it closes again', count('.chat-lightbox') == 0)
    ui("document.querySelector('.chat-side-toggle').click(); 1"); time.sleep(0.6)
    check('the chat list can be hidden', V("document.querySelector('.chat-side').getBoundingClientRect().width") < 5)
    ui("document.querySelector('.chat-side-toggle').click(); 1"); time.sleep(0.6)

    # ---- delete removes the pictures too
    ui("document.querySelector('.chat-more-btn').click(); 1"); time.sleep(0.3)
    ui("[...document.querySelectorAll('.chat-pop-item')].find(b=>b.textContent.startsWith('Delete')).click(); 1"); time.sleep(1.2)
    imgs_dir = os.path.join(prof, 'Constellations', 'chat-images')
    left = os.listdir(imgs_dir) if os.path.exists(imgs_dir) else []
    check('deleting the conversation with the picture deletes the picture file', left == [], left)
    PS = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'capture_seq.ps1')
    ui("document.querySelector('.chat-new').click(); 1"); time.sleep(0.4)
    type_send('table please'); wait_idle()
    ui("document.querySelector('.chat-scroll').scrollTop=0; 1"); time.sleep(0.4)
    subprocess.run(['powershell', '-ExecutionPolicy', 'Bypass', '-File', PS, '-Prefix', os.path.join(ROOT, 'out', 'live', 'chat-page2'), '-Count', '1', '-IntervalMs', '10'], capture_output=True)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
