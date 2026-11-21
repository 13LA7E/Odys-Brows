"""Chat page, part two: files (text, PDF, Word), versions after Edit and Ask again, voice input with a
speech-to-text server, read aloud, and the speech settings. A fake model and a fake Whisper server."""
import os, sys, base64, struct, zlib, io, zipfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

S = lambda e: ui('JSON.stringify(%s)' % e)
V = lambda e: json.loads(S(e))
requests_seen = []
speech_seen = []


class M(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a): pass

    def do_POST(self):
        n = int(self.headers.get('Content-Length', 0)); raw = self.rfile.read(n)
        if self.path.endswith('/audio/transcriptions'):
            speech_seen.append({'bytes': len(raw), 'ctype': self.headers.get('Content-Type', ''), 'has_model': b'whisper-test' in raw, 'has_audio': b'filename="voice.' in raw})
            body = json.dumps({'text': 'hello from my voice'}).encode()
            self.send_response(200); self.send_header('Content-Type', 'application/json'); self.send_header('Content-Length', str(len(body))); self.end_headers(); self.wfile.write(body)
            return
        body = json.loads(raw or b'{}')
        requests_seen.append(body)
        last = body['messages'][-1]['content']
        self.send_response(200); self.send_header('Content-Type', 'text/event-stream'); self.end_headers()
        text = 'Answer %d to: %s' % (len(requests_seen), str(last)[:60].replace(chr(10), ' '))
        for i in range(0, len(text), 14):
            self.wfile.write(('data: ' + json.dumps({'choices': [{'delta': {'content': text[i:i + 14]}}]}) + chr(10) + chr(10)).encode()); self.wfile.flush()
        self.wfile.write(('data: [DONE]' + chr(10) + chr(10)).encode())


msrv = TS(('127.0.0.1', 8775), M); threading.Thread(target=msrv.serve_forever, daemon=True).start()


def make_pdf(text):
    objs = ['<</Type/Catalog/Pages 2 0 R>>', '<</Type/Pages/Kids[3 0 R]/Count 1>>',
            '<</Type/Page/Parent 2 0 R/MediaBox[0 0 300 100]/Contents 4 0 R/Resources<</Font<</F1 5 0 R>>>>>>',
            None, '<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>']
    c = 'BT /F1 18 Tf 20 50 Td (%s) Tj ET' % text
    objs[3] = '<</Length %d>>\nstream\n%s\nendstream' % (len(c), c)
    pdf = b'%PDF-1.4\n'; offs = []
    for i, o in enumerate(objs):
        offs.append(len(pdf)); pdf += ('%d 0 obj\n%s\nendobj\n' % (i + 1, o)).encode()
    x = len(pdf)
    pdf += ('xref\n0 %d\n0000000000 65535 f \n' % (len(objs) + 1)).encode()
    for o in offs: pdf += ('%010d 00000 n \n' % o).encode()
    pdf += ('trailer\n<</Size %d/Root 1 0 R>>\nstartxref\n%d\n%%%%EOF\n' % (len(objs) + 1, x)).encode()
    return pdf


def make_docx(paragraphs):
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, 'w', zipfile.ZIP_DEFLATED) as z:
        body = ''.join('<w:p><w:r><w:t>%s</w:t></w:r></w:p>' % p for p in paragraphs)
        z.writestr('word/document.xml', '<w:document><w:body>%s</w:body></w:document>' % body)
    return buf.getvalue()


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
def wait_idle(t=30): wait(lambda: idle() and V("document.querySelectorAll('.chat-msg').length") > 0, t); time.sleep(0.8)
def count(sel): return V("document.querySelectorAll(%s).length" % json.dumps(sel))
def texts(): return V("[...document.querySelectorAll('.chat-msg')].map(m=>m.querySelector('.chat-bubble,.chat-answer')?.textContent||'')")


prof = tempfile.mkdtemp(prefix='cons-prof-')
os.makedirs(os.path.join(prof, 'files'))
txt = os.path.join(prof, 'files', 'notes.txt'); open(txt, 'w').write('The launch date is 14 March. Budget: 4200 euro.')
pdf = os.path.join(prof, 'files', 'report.pdf'); open(pdf, 'wb').write(make_pdf('Quarterly sales rose by seven percent'))
docx = os.path.join(prof, 'files', 'memo.docx'); open(docx, 'wb').write(make_docx(['Meeting moved to Thursday', 'Bring the red folder']))
junk = os.path.join(prof, 'files', 'program.exe'); open(junk, 'wb').write(b'MZ\x90\x00\x03\x00\x00\x00')
p = launch(['--use-fake-device-for-media-stream'], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(3)
    bridge({'type': 'ai-save-config', 'config': {'provider': 'local', 'endpoint': 'http://127.0.0.1:8775', 'model': 'mock', 'speech_endpoint': 'http://127.0.0.1:8775/v1', 'speech_model': 'whisper-test'}}); time.sleep(1)
    bridge({'type': 'ai-get-config'}); time.sleep(0.5)
    bridge({'type': 'open', 'url': 'odyssey://chat'}); time.sleep(2)

    # ---- files
    cdp_files('input[aria-label="Attach files"]', [txt, pdf, docx]); time.sleep(1.5)
    check('three files show as chips in the composer', count('.chat-attach .chat-file') == 3, V("[...document.querySelectorAll('.chat-attach .chat-file')].map(f=>f.textContent)"))
    ui("document.querySelector('.chat-file-x').click(); 1"); time.sleep(0.3)
    check('a file can be removed', count('.chat-attach .chat-file') == 2)
    cdp_files('input[aria-label="Attach files"]', [txt]); time.sleep(1.2)
    type_send('summarise the files'); wait_idle(30)
    last = [m['content'] for m in requests_seen[-1]['messages'] if m['role'] == 'user'][-1]
    check('the text of the PDF reaches the model', 'Quarterly sales rose by seven percent' in last, last[:200])
    check('the text of the Word file reaches the model', 'Bring the red folder' in last and 'Meeting moved to Thursday' in last)
    check('the text file reaches the model', 'launch date is 14 March' in last)
    check('each file is introduced by its name', '[Attached file: report.pdf]' in last and '[Attached file: memo.docx]' in last)
    check('your message shows the file chips', count('.chat-msg.user .chat-file') == 3)
    saved = json.load(open(os.path.join(prof, 'Constellations', 'chats.json')))['chats'][0]['messages'][0]
    pdf_att = [a for a in saved['attachments'] if a['name'] == 'report.pdf']
    check('the files are saved with the message (names and text)', len(saved['attachments']) == 3 and bool(pdf_att) and 'seven percent' in pdf_att[0]['text'], [a['name'] for a in saved['attachments']])
    type_send('what was the budget?'); wait_idle()
    last2 = str(requests_seen[-1]['messages'])
    check('a later turn still has the files', 'Budget: 4200 euro' in last2 and 'Bring the red folder' in last2)
    n_before = count('.chat-msg')
    cdp_files('input[aria-label="Attach files"]', [junk]); time.sleep(1.2)
    type_send('and this one?'); wait_idle()
    last3 = [m['content'] for m in requests_seen[-1]['messages'] if m['role'] == 'user'][-1]
    check('a file that cannot be read is reported and the question still goes through', 'program.exe' not in last3 and V("window.__uiState.chat.messages.slice(-1)[0].steps.some(s=>s.includes('program.exe'))"), V("window.__uiState.chat.messages.slice(-1)[0].steps"))

    # ---- versions: ask again
    ui("document.querySelector('.chat-new').click(); 1"); time.sleep(0.5)
    type_send('give me a number'); wait_idle()
    first_answer = texts()[1]
    ui("document.querySelector('.chat-msg.assistant .chat-regen').click(); 1"); time.sleep(0.6); wait_idle(); time.sleep(1.5)
    second_answer = texts()[1]
    check('Ask again gives a different answer', second_answer != first_answer and count('.chat-msg.assistant') == 1, (first_answer, second_answer))
    check('and shows 2/2 with arrows', V("document.querySelector('.chat-msg.assistant .chat-version-label').textContent") == '2/2')
    ui("document.querySelector('.chat-msg.assistant .chat-prev').click(); 1"); time.sleep(1.2)
    check('the arrow shows the first answer again', texts()[1] == first_answer and V("document.querySelector('.chat-msg.assistant .chat-version-label').textContent") == '1/2', texts()[1])
    ui("document.querySelector('.chat-msg.assistant .chat-next').click(); 1"); time.sleep(1.2)
    check('and the second again', texts()[1] == second_answer)

    # ---- versions: edit
    type_send('second question'); wait_idle()
    ui("document.querySelectorAll('.chat-msg.user')[1].querySelector('.chat-edit-btn').click(); 1"); time.sleep(0.4)
    ui("(()=>{const t=document.querySelector('.chat-edit-input'); t.value='second question, reworded'; document.querySelector('.chat-edit-save').click(); return 1})()"); time.sleep(0.6); wait_idle(); time.sleep(1.5)
    check('after Edit the message shows 2/2', V("document.querySelectorAll('.chat-msg.user')[1].querySelector('.chat-version-label').textContent") == '2/2' and 'reworded' in texts()[2], texts())
    ui("document.querySelectorAll('.chat-msg.user')[1].querySelector('.chat-prev').click(); 1"); time.sleep(1.2)
    check('the arrow brings back the question you edited, with its answer', texts()[2] == 'second question' and 'second question' in texts()[3] and 'reworded' not in ''.join(texts()), texts())
    ui("document.querySelectorAll('.chat-msg.user')[1].querySelector('.chat-next').click(); 1"); time.sleep(1.2)
    check('and forward again to the edited one', 'reworded' in texts()[2])
    cf = json.load(open(os.path.join(prof, 'Constellations', 'chats.json')))
    ch = [c for c in cf['chats'] if c['messages'][0]['text'] == 'give me a number'][0]
    check('both versions are saved on disk', ch['messages'][2]['alts']['pos'] == 1 and len(ch['messages'][2]['alts']['all']) == 2 and len(ch['messages'][2]['alts']['all'][0]) == 2)
    # reopen from the list: versions survive
    ui("document.querySelector('.chat-new').click(); 1"); time.sleep(0.4)
    ui("[...document.querySelectorAll('.chat-item-open')].find(b=>b.textContent.includes('give me a number')).click(); 1"); time.sleep(1.5)
    check('reopening the chat keeps the versions', V("!!document.querySelectorAll('.chat-msg.user')[1].querySelector('.chat-version-label')"))

    # ---- voice
    ui("document.querySelector('.chat-mic-btn').click(); 1"); time.sleep(2.2)
    check('the microphone starts recording (a fake device)', 'Listening' in V("document.querySelector('.chat-input').placeholder"), V("document.querySelector('.chat-input').placeholder"))
    ui("document.querySelector('.chat-mic-btn').click(); 1")
    wait(lambda: speech_seen, 20); time.sleep(1.5)
    check('the recording went to the speech server with the model name', bool(speech_seen) and speech_seen[0]['has_audio'] and speech_seen[0]['has_model'] and speech_seen[0]['bytes'] > 500, speech_seen[:1])
    check('the words appear in the message box', 'hello from my voice' in V("document.querySelector('.chat-input').value"), V("document.querySelector('.chat-input').value"))
    ui("document.querySelector('.chat-input').value=''; 1")
    # no server configured: a clear message
    bridge({'type': 'ai-save-config', 'config': {'provider': 'local', 'endpoint': 'http://127.0.0.1:8775', 'model': 'mock', 'speech_endpoint': '', 'speech_model': ''}}); time.sleep(1)
    bridge({'type': 'ai-get-config'}); time.sleep(0.5)
    ui("document.querySelector('.chat-mic-btn').click(); 1"); time.sleep(2); ui("document.querySelector('.chat-mic-btn').click(); 1"); time.sleep(2.5)
    check('without a speech server you are told how to set one up', 'speech-to-text server' in V("document.querySelector('.chat-toast').textContent"), V("document.querySelector('.chat-toast').textContent"))

    # ---- read aloud
    check('the speech engine exists', V("typeof window.speechSynthesis") == 'object')
    ui("window.__spoken=[]; const o=speechSynthesis.speak.bind(speechSynthesis); speechSynthesis.speak=(u)=>{window.__spoken.push(u.text); return o(u)}; speechSynthesis.cancel=()=>{}; 1")
    ui("document.querySelector('.chat-msg.assistant .chat-speak').click(); 1"); time.sleep(0.5)
    check('Read aloud speaks the answer text', len(V('window.__spoken')) == 1 and 'Answer' in V('window.__spoken')[0], V('window.__spoken'))

    # ---- settings
    bridge({'type': 'open', 'url': 'odyssey://settings'}); time.sleep(0.8)
    ui("[...document.querySelectorAll('.page-internal *')].filter(e=>e.children.length===0&&e.textContent==='Odysseus')[0].click(); 1"); time.sleep(0.6)
    t = V("document.querySelector('.settings-content').textContent")
    check('Settings has the speech server fields', 'Speech-to-text server' in t and 'Speech model' in t)
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
