# Deterministic protocol fixture, not a language analyzer. Used only by Rust tests.
import json, sys, time
mode = sys.argv[1]
def send(value):
    raw = json.dumps(dict(jsonrpc='2.0', **value)).encode()
    sys.stdout.buffer.write(b'Content-Length: %d\r\n\r\n' % len(raw) + raw)
    sys.stdout.buffer.flush()
def read():
    n = None
    while True:
        line = sys.stdin.buffer.readline()
        if not line: raise EOFError
        if line == b'\r\n': break
        k, v = line.decode().split(':', 1)
        if k.lower() == 'content-length': n = int(v)
    return json.loads(sys.stdin.buffer.read(n))
opened = []
while True:
    v = read(); method = v.get('method')
    if method == 'initialize':
        assert v['params']['capabilities']['general']['positionEncodings'] == ['utf-16']
        assert v['params']['initializationOptions']['cargo']['buildScripts']['enable'] is False
        if mode == 'hang': time.sleep(60)
        if mode == 'malformed':
            sys.stdout.buffer.write(b'Content-Length: 99999999\r\n\r\n'); sys.stdout.buffer.flush(); time.sleep(60)
        send(dict(id=v['id'], result=dict(capabilities=dict(definitionProvider=True, positionEncoding='utf-8' if mode == 'encoding' else 'utf-16'))))
    elif method == 'initialized':
        if mode != 'index-hang': send(dict(method='experimental/serverStatus', params=dict(quiescent=True)))
    elif method == 'textDocument/didOpen':
        d = v['params']['textDocument']; assert d['version'] == 1
        opened.append(d)
    elif method == 'textDocument/definition':
        assert opened and '// unsaved LSP overlay' in opened[0]['text']
        send(dict(id=900, method='workspace/applyEdit', params=dict(edit={})))
        reply = read(); assert reply['id'] == 900 and reply['error']['code'] == -32601
        if mode in ('error', 'large-error'): send(dict(id=v['id'], error=dict(code=-32001, message='X' * 2048 if mode == 'large-error' else 'Fixture failure')))
        else:
            send(dict(id=42, result=None))  # unrelated response must not complete the request
            send(dict(id=v['id'], result=None if mode == 'null' else dict(uri=opened[0]['uri'], range=dict(start=dict(line=1, character=0), end=dict(line=1, character=0)))))
    elif method == 'shutdown': send(dict(id=v['id'], result=None))
    elif method == 'exit': break
