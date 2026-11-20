"""Tiny CDP client: python cdp.py <port> <expr> [titlesubstr]. Prints the evaluated value of each target matching."""
import json, sys, urllib.request, socket, base64, os, struct

def ws_connect(url):
    host, rest = url[5:].split('/', 1)
    h, p = host.split(':')
    s = socket.create_connection((h, int(p)), timeout=10)
    key = base64.b64encode(os.urandom(16)).decode()
    s.send((f"GET /{rest} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n").encode())
    buf = b''
    while b'\r\n\r\n' not in buf:
        buf += s.recv(1024)
    return s

def send(s, obj):
    data = json.dumps(obj).encode()
    mask = os.urandom(4)
    hdr = bytearray([0x81])
    n = len(data)
    if n < 126: hdr.append(0x80 | n)
    elif n < 65536: hdr.append(0x80 | 126); hdr += struct.pack('>H', n)
    else: hdr.append(0x80 | 127); hdr += struct.pack('>Q', n)
    s.send(bytes(hdr) + mask + bytes(b ^ mask[i % 4] for i, b in enumerate(data)))

def recv(s):
    def rn(n):
        b = b''
        while len(b) < n:
            c = s.recv(n - len(b))
            if not c: raise EOFError
            b += c
        return b
    out = b''
    while True:
        h = rn(2)
        fin = h[0] & 0x80
        n = h[1] & 0x7f
        if n == 126: n = struct.unpack('>H', rn(2))[0]
        elif n == 127: n = struct.unpack('>Q', rn(8))[0]
        out += rn(n)
        if fin: break
    return json.loads(out)

def evaluate(ws_url, expr):
    s = ws_connect(ws_url)
    send(s, {"id": 1, "method": "Runtime.evaluate", "params": {"expression": expr, "awaitPromise": True, "returnByValue": True}})
    while True:
        m = recv(s)
        if m.get('id') == 1:
            s.close()
            r = m.get('result', {})
            return r.get('result', {}).get('value', r)

if __name__ == '__main__':
    port, expr = sys.argv[1], sys.argv[2]
    sub = sys.argv[3] if len(sys.argv) > 3 else ''
    targets = json.load(urllib.request.urlopen(f'http://127.0.0.1:{port}/json'))
    for t in targets:
        if sub in (t.get('url', '') + t.get('title', '')):
            print(t.get('url', '')[:60], '=>', json.dumps(evaluate(t['webSocketDebuggerUrl'], expr)))
