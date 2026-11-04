import json, os, pathlib, queue, subprocess, tempfile, threading, time

binary = os.environ['LSP_940_SERVER']
positive = 'set x 0; foreach i {1 2 3} {incr x; lindex $x 0}\n'
fixtures = {
    'positive': positive,
    'invalid': 'set x abc; foreach i {1 2 3} {incr x; lindex $x 0}\n',
    'one_trip': 'set x 0; foreach i {1} {incr x; lindex $x 0}\n',
    'original': 'set ys {}; set pts {}; foreach pt {{1 2} {3 4} {5 6}} {set y [lindex $pt 1]; if {$y > 2} {lappend ys $y; lappend pts $pt}}\n',
    'unconditional': 'set ys {}; set pts {}; foreach pt {{1 2} {3 4} {5 6}} {set y [lindex $pt 1]; lappend ys $y; lappend pts $pt}\n',
    'font_braced': 'set fontSizes {10.0 12.0 16.0 24.0}\nforeach size $fontSizes { puts $size }\n',
    'font_list': 'set fontSizes [list 10.0 12.0 16.0 24.0]\nforeach size $fontSizes { puts $size }\n',
    'suppressed': '# noqa: S102\n' + positive,
    'unrelated_noqa': '# noqa: W999\n' + positive,
}

def run(name, configuration, inputs):
    root = pathlib.Path(tempfile.mkdtemp(prefix='s102-native-'))
    for directory in ('config','cache','state'):
        (root/directory).mkdir()
    env = dict(os.environ, XDG_CONFIG_HOME=str(root/'config'), XDG_CACHE_HOME=str(root/'cache'), XDG_STATE_HOME=str(root/'state'))
    config = {'lineLength':117, **configuration}
    proc = subprocess.Popen([binary], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
    messages=queue.Queue(); write_lock=threading.Lock(); start=time.monotonic(); next_id=0
    log=open(f'/tmp/lsp-s102-auth-{name}-messages.jsonl','w')
    err=open(f'/tmp/lsp-s102-auth-{name}-stderr.log','w')
    def send(msg):
        data=json.dumps(msg).encode()
        with write_lock:
            proc.stdin.write(f'Content-Length: {len(data)}\r\n\r\n'.encode()+data);proc.stdin.flush()
    def stderr():
        for line in iter(proc.stderr.readline,b''):
            err.write(line.decode(errors='replace'));err.flush()
    def read():
        while True:
            headers={}
            while True:
                line=proc.stdout.readline()
                if not line:return
                if line in (b'\r\n',b'\n'):break
                key,value=line.decode().split(':',1);headers[key.lower()]=value.strip()
            msg=json.loads(proc.stdout.read(int(headers['content-length'])))
            log.write(json.dumps({'time':time.monotonic()-start,'message':msg})+'\n');log.flush()
            if 'id' in msg and 'method' in msg:
                result=[config if item.get('section')=='tclLsp' else None for item in msg.get('params',{}).get('items',[])] if msg['method']=='workspace/configuration' else None
                send({'jsonrpc':'2.0','id':msg['id'],'result':result})
            else:messages.put(msg)
    threading.Thread(target=read,daemon=True).start();threading.Thread(target=stderr,daemon=True).start()
    def wait(predicate,timeout):
        deadline=time.monotonic()+timeout
        while True:
            left=deadline-time.monotonic()
            if left<=0:raise TimeoutError('waiting for protocol event')
            msg=messages.get(timeout=left)
            if predicate(msg):return msg
    def request(method,params,timeout=30):
        nonlocal next_id
        next_id+=1; rid=next_id
        send({'jsonrpc':'2.0','id':rid,'method':method,'params':params})
        msg=wait(lambda m:m.get('id')==rid,timeout)
        if 'error' in msg:raise RuntimeError(msg)
        return msg.get('result')
    try:
        request('initialize',{'processId':os.getpid(),'rootUri':None,'capabilities':{}})
        send({'jsonrpc':'2.0','method':'initialized','params':{}})
        deadline=time.monotonic()+30
        while True:
            effective=request('workspace/executeCommand',{'command':'tcl-lsp.getEffectiveConfig','arguments':['']})
            if effective.get('line_length')==117:break
            if time.monotonic()>deadline:raise TimeoutError('config barrier')
            time.sleep(.1)
        print(f'{name} CONFIG {json.dumps(config)} effective_line_length=117',flush=True)
        for label,source in inputs.items():
            uri=(root/(label+'.tcl')).as_uri(); latest=None
            send({'jsonrpc':'2.0','method':'textDocument/didOpen','params':{'textDocument':{'uri':uri,'languageId':'tcl','version':1,'text':source}}})
            deadline=time.monotonic()+90
            while True:
                remaining=deadline-time.monotonic()
                if remaining<=0:raise TimeoutError(f'{name}/{label} deep')
                msg=messages.get(timeout=remaining)
                if msg.get('method')=='textDocument/publishDiagnostics' and msg.get('params',{}).get('uri')==uri:
                    latest=msg['params']['diagnostics']
                if msg.get('method')=='window/logMessage' and 'deep diagnostics' in msg.get('params',{}).get('message','') and uri in msg['params']['message']:
                    break
            if latest is None:raise AssertionError(f'{name}/{label}: no diagnostic publication before deep marker')
            deadline=time.monotonic()+90
            while True:
                report=request('workspace/executeCommand', {'command':'tcl-lsp.currentDiagnostics','arguments':[{'uri':uri,'version':1,'source':source}]}, 30)
                if report.get('status') in ('complete','suppressed'):break
                if report.get('status')!='unavailable' or report.get('reason') not in ('pending','stale') or time.monotonic()>=deadline:
                    raise AssertionError(f'{name}/{label} diagnostic completion unavailable: {report}')
                time.sleep(.05)
            assert (report.get('uri'),report.get('version'),report.get('source'))==(uri,1,source),report
            assert report.get('resultId') and report.get('revision') and report.get('inputsEpoch'),report
            print(f'{name} {label} RECEIPT {json.dumps(report)}',flush=True)
            if name!='off':
                assert report['status']=='complete',report
                identity={k:report[k] for k in ('uri','version','source','dialectId','revision','inputsEpoch','resultId')}
                verified=request('workspace/executeCommand', {'command':'tcl-lsp.validateDiagnosticPublication','arguments':[identity]},30)
                assert verified.get('status')=='current',verified
                print(f'{name} {label} REBORROW {json.dumps(verified)}',flush=True)
            s102=[d for d in report['diagnostics'] if d.get('code')=='S102']
            print(f'{name} {label} S102={json.dumps(s102)}',flush=True)
            expected=1 if label in ('positive','unrelated_noqa') and name!='off' else 0
            assert len(s102)==expected,(name,label,s102)
            if expected:
                d=s102[0];line=1 if label=='unrelated_noqa' else 0
                assert d['range']['start']=={'line':line,'character':43},d
                assert d['range']['end']=={'line':line,'character':45},d
                if name=='error':assert d.get('severity')==1,d
                actions=request('textDocument/codeAction', {'textDocument':{'uri':uri}, 'range':d['range'], 'context':{'diagnostics':[d]}}, 30)
                print(f'{name} {label} ACTIONS {json.dumps(actions)}', flush=True)
                assert not any(a.get('diagnostics') and any(x.get('code')=='S102' for x in a['diagnostics']) for a in (actions or [])), actions
    finally:
        proc.stdin.close()
        try:proc.wait(timeout=5)
        except subprocess.TimeoutExpired:proc.kill();proc.wait()
        log.close();err.close()

run('default',{},fixtures)
run('off',{'shimmer':{'enabled':False}},{'positive':positive})
run('error',{'diagnosticSeverity':{'S102':'error'}},{'positive':positive})
print('PASS conditional S102, negative/source/config controls; original accumulator coverage remains incomplete',flush=True)
