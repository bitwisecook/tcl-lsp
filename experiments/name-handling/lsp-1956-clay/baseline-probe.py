# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
import hashlib,json,os,pathlib,queue,subprocess,tempfile,threading,time
binary=pathlib.Path('/workspace/tcl-lsp/tmp/lsp-values-clean-target/debug/tcl-lsp-server')
expected='e4e4153afadf5906975a94fe661d81ef1040ecce0c630416c4cde9c0742d2ab7'
assert hashlib.sha256(binary.read_bytes()).hexdigest()==expected
out=pathlib.Path('/workspace/tcl-lsp/tmp/lsp-verified-artifacts/1956'); out.mkdir(parents=True,exist_ok=True)
root=pathlib.Path(tempfile.mkdtemp(prefix='lsp-1956-baseline-'))
for d in ['config','cache','state']: (root/d).mkdir()
env=dict(os.environ,XDG_CONFIG_HOME=str(root/'config'),XDG_CACHE_HOME=str(root/'cache'),XDG_STATE_HOME=str(root/'state'))
p=subprocess.Popen([str(binary)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=(out/'server.stderr').open('wb'),env=env)
messages=queue.Queue(); lock=threading.Lock(); log=(out/'server-protocol.jsonl').open('w'); next_id=0

def send(msg):
 data=json.dumps(msg).encode()
 with lock:
  p.stdin.write(f'Content-Length: {len(data)}\r\n\r\n'.encode()+data);p.stdin.flush()

def reader():
 try:
  while True:
   headers={}
   while True:
    line=p.stdout.readline()
    if not line:return
    if line in [b'\r\n',b'\n']:break
    k,v=line.decode().split(':',1);headers[k.lower()]=v.strip()
   msg=json.loads(p.stdout.read(int(headers['content-length'])))
   log.write(json.dumps(msg)+'\n');log.flush()
   if 'method' in msg and 'id' in msg:
    result=[{'lineLength':117} if item.get('section')=='tclLsp' else None for item in msg.get('params',{}).get('items',[])] if msg['method']=='workspace/configuration' else None
    send({'jsonrpc':'2.0','id':msg['id'],'result':result})
   else:messages.put(msg)
 except BaseException as error:messages.put({'reader_error':repr(error)})
threading.Thread(target=reader,daemon=True).start()

def request(method,params):
 global next_id
 next_id+=1;rid=next_id; send({'jsonrpc':'2.0','id':rid,'method':method,'params':params}); end=time.monotonic()+50
 while time.monotonic()<end:
  msg=messages.get(timeout=end-time.monotonic())
  if msg.get('id')==rid:return msg
 raise TimeoutError(method)

def position(source,needle):
 i=source.index(needle);prefix=source[:i];return {'line':prefix.count('\n'),'character':len(prefix.rsplit('\n',1)[-1].encode('utf-16-le'))//2}

cases=[('tcloo','oo::class create ::Probe {method connect {} {}; method use {} {my connect}}\n','connect}}'),('clay','::clay::define ::Probe {method connect {} {}; method use {} {my connect}}\n','connect}}'),('unknown','::clayish::define ::Probe {method connect {} {}; method use {} {my connect}}\n','connect}}'),('httpd',pathlib.Path('/workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/httpd/httpd.tcl').read_text(),'connect]]')]
rows=[]
try:
 init=request('initialize',{'processId':os.getpid(),'rootUri':None,'capabilities':{'textDocument':{'documentSymbol':{'hierarchicalDocumentSymbolSupport':True}}}})
 send({'jsonrpc':'2.0','method':'initialized','params':{}})
 for name,source,needle in cases:
  uri=(root/(name+'.tcl')).as_uri();send({'jsonrpc':'2.0','method':'textDocument/didOpen','params':{'textDocument':{'uri':uri,'languageId':'tcl','version':1,'text':source}}})
  symbols=request('textDocument/documentSymbol',{'textDocument':{'uri':uri}})
  definition=request('textDocument/definition',{'textDocument':{'uri':uri},'position':position(source,needle)})
  rows.append(dict(name=name,source_sha256=hashlib.sha256(source.encode()).hexdigest(),source_bytes=len(source.encode()),symbols=symbols,definition=definition))
  (out/'server-rows.json').write_text(json.dumps(rows,indent=2)+'\n')
  print(name,'symbols',len(symbols.get('result') or []),'definition',definition.get('result'),flush=True)
 request('shutdown',None);send({'jsonrpc':'2.0','method':'exit','params':None});p.stdin.close();p.wait(timeout=10)
finally:
 if p.poll() is None:p.kill();p.wait()
 log.close()
 receipt=dict(source_commit='cab14d1a4fe76da7e88eb5225b9533408d35e95b',binary=str(binary),binary_sha256=expected,binary_hash_unchanged=hashlib.sha256(binary.read_bytes()).hexdigest()==expected,process_exit=p.returncode,rows=rows)
 (out/'server-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
