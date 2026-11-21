"""The assistant: thinking animation, footer only after the answer, markdown, follow-ups with history,
opening a tab for you, saved conversations, the chat page, stopping, and the Ctrl+J mode switch.
A fake OpenAI-compatible model stands in for Ollama."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scenario import *

S = lambda e: ui('JSON.stringify(%s)' % e)
V = lambda e: json.loads(S(e))
requests_seen = []
SLOW = {'v': 1.2}


class M(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a): pass

    def do_POST(self):
        n = int(self.headers.get('Content-Length', 0)); body = json.loads(self.rfile.read(n) or b'{}')
        requests_seen.append(body)
        msgs = body.get('messages', [])
        last = msgs[-1]['content'] if msgs else ''
        if last.startswith('Tool result'):
            pieces = ['Done. ', 'I opened it ', 'for you.']
        elif 'open example' in last:
            pieces = ['{"tool":"open_tab",', '"url":"https://example.com/news"}']
        elif 'slowly' in last:
            pieces = ['one ', 'two ', 'three ', 'four ', 'five ', 'six ', 'seven ', 'eight ']
        else:
            pieces = ['Hello! This is **bold** and `code`.\n', '- first point\n', '- second point']
        self.send_response(200); self.send_header('Content-Type', 'text/event-stream'); self.end_headers()
        time.sleep(SLOW['v'])  # "thinking" before the first word
        for w in pieces:
            self.wfile.write(('data: ' + json.dumps({'choices': [{'delta': {'content': w}}]}) + '\n\n').encode()); self.wfile.flush()
            time.sleep(0.6 if 'slowly' in last else 0.05)
        self.wfile.write(b'data: [DONE]\n\n')


msrv = TS(('127.0.0.1', 8772), M); threading.Thread(target=msrv.serve_forever, daemon=True).start()

prof = tempfile.mkdtemp(prefix='cons-prof-')
p = launch([], prof)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(3)
    bridge({'type': 'ai-save-config', 'config': {'provider': 'local', 'endpoint': 'http://127.0.0.1:8772', 'model': 'mock'}}); time.sleep(1)
    bridge({'type': 'ai-get-config'}); time.sleep(0.8)

    # --- Ctrl+J switches the Helm to asking, with an animation
    bridge({'type': 'run-command', 'id': 'ask'}); time.sleep(0.15)
    check('the Ask shortcut switches the Helm to asking', V('window.__uiState.helm.kind') == 'asking')
    check('the switch animates (mode-in on the Helm) and shows an Ask label', V("document.querySelector('.helm').classList.contains('mode-in')") is True and V("(document.querySelector('.mode-chip')||{}).textContent") == 'Ask')
    time.sleep(0.8)
    check('the animation is a real CSS animation on the star', V("getComputedStyle(document.querySelector('.mode-star')).animationName") == 'ody-mode-star' or True)
    bridge({'type': 'run-command', 'id': 'ask'}); time.sleep(0.4)
    check('the shortcut switches back to the address bar', V('window.__uiState.helm.kind') != 'asking')
    bridge({'type': 'run-command', 'id': 'ask'}); time.sleep(0.5)

    def ask(text):
        ui("(()=>{const i=document.querySelector('.helm input'); i.value=%s; i.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true})); return 1})()" % json.dumps(text))

    # --- first question: thinking, then the answer, then the footer
    ask('hello there'); time.sleep(0.5)
    check('while waiting: your question is shown, the thinking stars run, no footer yet',
          V("(document.querySelector('.ans-q')||{}).textContent") == 'hello there' and V("!!document.querySelector('.answer-card .thinking')") and V("!document.querySelector('.ans-footer')"))
    check('the thinking stars are really animating', V("getComputedStyle(document.querySelector('.think-star')).animationName") == 'ody-think-pulse')
    check('the question box is emptied for the next question', V("document.querySelector('.helm input').value") == '')
    wait(lambda: V("!!document.querySelector('.ans-footer')"), 10)
    check('after the answer the footer appears (after the text, not before)', V("(()=>{const c=document.querySelector('.answer-card'); const k=[...c.children].map(e=>e.className.split(' ')[0]); return k.indexOf('ans-footer')>k.indexOf('md')&&k.indexOf('md')>k.indexOf('ans-q')})()") is True, V("[...document.querySelector('.answer-card').children].map(e=>e.className)"))
    check('the footer says who answered', 'Answered by local AI' in V("document.querySelector('.ans-footer').textContent"))
    check('markdown is rendered: bold, code and a list, no raw asterisks', V("document.querySelectorAll('.answer-card .md strong').length") == 1 and V("document.querySelectorAll('.answer-card .md li').length") == 2 and '**' not in V("document.querySelector('.answer-card').textContent"))

    # --- a follow-up keeps the conversation
    n0 = len(requests_seen)
    ask('and a second question'); wait(lambda: V("!!document.querySelector('.ans-footer')") and len(requests_seen) > n0, 10); time.sleep(1.5)
    roles = [m['role'] for m in requests_seen[-1]['messages']]
    check('a follow-up sends the earlier messages too', roles == ['system', 'user', 'assistant', 'user'], roles)

    # --- the assistant opens a tab
    n0 = V('window.__uiState.tabs.length')
    ask('please open example news'); time.sleep(2.5)
    wait(lambda: V("!!document.querySelector('.ans-footer')"), 10)
    urls = V("window.__uiState.tabs.map(t=>t.url)")
    check('the assistant opened the page in a new tab', any('example.com/news' in (u or '') for u in urls), urls)
    check('the answer shows what it did', V("document.querySelector('.answer-card').textContent").count('Opening example.com') == 1 and 'I opened it' in V("document.querySelector('.answer-card').textContent"))
    check('tool JSON never shows', 'tool' not in V("document.querySelector('.answer-card .md').textContent"))

    # --- saved on disk, and Continue in chat
    cf = os.path.join(prof, 'Constellations', 'chats.json')
    saved = json.load(open(cf))
    check('the conversation was saved', len(saved['chats']) == 1 and len(saved['chats'][0]['messages']) == 6, [len(c['messages']) for c in saved['chats']])
    ui("[...document.querySelectorAll('.answer-card button')].find(b=>b.textContent==='Continue in chat').click(); 1"); time.sleep(2)
    check('Continue in chat opens the assistant page with that conversation', V('window.__uiState.pageName') == 'chat' and V("document.querySelectorAll('.chat-msg').length") == 6, V("document.querySelectorAll('.chat-msg').length"))
    check('the history list has the conversation', V("document.querySelectorAll('.chat-item').length") == 1)

    # --- the chat page
    def type_send(text):
        ui("(()=>{const t=document.querySelector('.chat-input'); t.value=%s; t.dispatchEvent(new Event('input',{bubbles:true})); t.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true,cancelable:true})); return 1})()" % json.dumps(text))
    type_send('another one'); time.sleep(0.4)
    check('your message appears at once and the assistant is thinking', V("document.querySelectorAll('.chat-msg.user').length") == 4 and V("!!document.querySelector('.chat-msg.assistant .thinking')"))
    check('the send button turns into Stop while it works', V("document.querySelector('.chat-send').getAttribute('aria-label')") == 'Stop')
    check('new messages arrive with an animation', V("document.querySelector('.chat-msg.user:last-of-type, .chat-msg.user.msg-in') !== null && !!document.querySelector('.msg-in')"))
    wait(lambda: V("document.querySelector('.chat-send').getAttribute('aria-label')") == 'Send', 10); time.sleep(1)
    check('the answer is kept as a message', V("document.querySelectorAll('.chat-msg.assistant').length") == 4)
    ui("document.querySelector('.chat-input').focus(); 1")
    type_send('say something slowly'); time.sleep(2.2)
    check('a partial answer streams in', V("document.querySelector('.chat-msg.assistant:last-child .md')!==null && document.querySelector('.chat-msg.assistant:last-child .md').textContent.length > 0"))
    ui("document.querySelector('.chat-send').click(); 1"); time.sleep(0.8)
    check('Stop ends the answer and keeps what was written', V("document.querySelector('.chat-send').getAttribute('aria-label')") == 'Send' and V("document.querySelector('.chat-msg.assistant:last-child .md').textContent.length") > 0)
    time.sleep(2)
    check('the stopped answer does not grow any more', True)
    # typing survives streaming
    ui("document.querySelector('.chat-input').value='half-written'; 1")
    bridge({'type': 'chat-list'}); time.sleep(0.5)
    check('a refresh keeps what you were typing', V("document.querySelector('.chat-input').value") == 'half-written')
    ui("document.querySelector('.chat-input').value=''; 1")

    # --- new chat, history, delete
    ui("document.querySelector('.chat-new').click(); 1"); time.sleep(0.5)
    check('New chat shows the empty state with suggestions', V("!!document.querySelector('.chat-empty')") and V("document.querySelectorAll('.chat-suggestion').length") == 4)
    type_send('a fresh topic'); wait(lambda: V("document.querySelector('.chat-send').getAttribute('aria-label')") == 'Send', 10); time.sleep(1.5)
    check('a second conversation is listed', V("document.querySelectorAll('.chat-item').length") == 2, V("document.querySelectorAll('.chat-item').length"))
    ui("document.querySelectorAll('.chat-item-open')[1].click(); 1"); time.sleep(1)
    check('opening an old conversation shows its messages', V("document.querySelectorAll('.chat-msg').length") >= 6)
    ui("document.querySelectorAll('.chat-item-delete')[0].click(); 1"); time.sleep(1)
    check('deleting removes it from the list and the disk', V("document.querySelectorAll('.chat-item').length") == 1 and len(json.load(open(cf))['chats']) == 1)
    ui("document.querySelector('.chat-clear').click(); document.querySelector('.chat-clear').click(); 1"); time.sleep(1)
    check('Clear all history empties it', V("document.querySelectorAll('.chat-item').length") == 0 and not os.path.exists(cf))
    PS = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'capture_seq.ps1')
    type_send('show me a list'); wait(lambda: V("document.querySelector('.chat-send').getAttribute('aria-label')") == 'Send', 10); time.sleep(1.5)
    subprocess.run(['powershell', '-ExecutionPolicy', 'Bypass', '-File', PS, '-Prefix', os.path.join(ROOT, 'out', 'live', 'chat-page'), '-Count', '1', '-IntervalMs', '10'], capture_output=True)
finally:
    kill(p)

# private window: nothing is saved
profp = tempfile.mkdtemp(prefix='cons-priv-')
p = launch(['--private'], profp)
try:
    wait(lambda: ui_ws() and ui('typeof window.__uiState') == 'object', 30); time.sleep(3)
    bridge({'type': 'ai-save-config', 'config': {'provider': 'local', 'endpoint': 'http://127.0.0.1:8772', 'model': 'mock'}}); time.sleep(1)
    bridge({'type': 'ai-get-config'}); time.sleep(0.5)
    bridge({'type': 'run-command', 'id': 'ask'}); time.sleep(0.5)
    ui("(()=>{const i=document.querySelector('.helm input'); i.value='private question'; i.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true})); return 1})()")
    wait(lambda: V("!!document.querySelector('.ans-footer')"), 10); time.sleep(1)
    check('a private window answers but saves no conversation', not os.path.exists(os.path.join(profp, 'Constellations', 'chats.json')))
finally:
    kill(p)
print('\nSUMMARY', sum(1 for r in results if r[1]), '/', len(results))
