from pathlib import Path
import hashlib,json,os,subprocess,time
b=Path('/workspace/.proofs'); request=b/'native-child-alias-inventory298b-request'; f=b/'native-child-alias-inventory298-capture'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(request/'request.json')=='32264d2d29b77612d556009096085e7805676cec7135251e104c1f67df5b06cb'
assert not f.exists();f.mkdir()
for p in request.iterdir():
 if p.is_file(): (f/p.name).write_bytes(p.read_bytes())
(f/'capture.py').write_bytes(Path(__file__).read_bytes())
req=json.loads((f/'request.json').read_text()); probe=f/'probe.c'; assert sha(probe)==req['driver_sha256']
summary=[]
for provider in req['providers']:
 version=provider['version'];oldpath=request/provider['original_compile_receipt_file'];assert sha(oldpath)==provider['original_compile_receipt_sha256'];old=json.loads(oldpath.read_text())
 directory=f/version;directory.mkdir();exe=directory/'probe.elf';pins=dict(provider['required_sha256']);pins[str(probe)]=req['driver_sha256']
 for case in req['cases']:
  p=f/Path(case['source_file']).name;pins[str(p)]=case['source_sha256']
 for p,digest in pins.items():assert sha(Path(p))==digest,p
 command=list(old['command']);command=[str(probe) if x==str(b/'native-info-original-dispatch292-capture/probe.c') else x for x in command];command[-1]=str(exe)
 compile_start=time.monotonic();compiled=subprocess.run(command,capture_output=True,timeout=60)
 (directory/'compile.stdout').write_bytes(compiled.stdout);(directory/'compile.stderr').write_bytes(compiled.stderr)
 record={'command':command,'exit':compiled.returncode,'seconds':time.monotonic()-compile_start,'required_sha256':pins,'original_compile_receipt_sha256':sha(oldpath),'source_sha256':sha(probe),'stdout_sha256':sha(directory/'compile.stdout'),'stderr_sha256':sha(directory/'compile.stderr'),'request_sha256':sha(f/'request.json')}
 if compiled.returncode==0:record['executable_sha256']=sha(exe)
 (directory/'compile.receipt.json').write_text(json.dumps(record,indent=2)+'\n');assert compiled.returncode==0,(version,compiled.stderr)
 env=os.environ.copy()
 if version!='jim':env['TCL_LIBRARY']=str(Path('/workspace/tcl-lsp/tmp')/('tcl'+version)/'library')
 for case in req['cases']:
  destination=directory/case['id'];destination.mkdir();source=f/Path(case['source_file']).name;assert sha(source)==case['source_sha256']
  command=[str(exe),'-',str(source),'-'];start=time.monotonic();result=subprocess.run(command,env=env,capture_output=True,timeout=30)
  (destination/'stdout').write_bytes(result.stdout);(destination/'stderr').write_bytes(result.stderr)
  rows=result.stdout.decode('ascii').splitlines()
  receipt={'provider':version,'command':command,'exit':result.returncode,'seconds':time.monotonic()-start,'rows':rows,'executable_sha256':sha(exe),'compile_receipt_sha256':sha(directory/'compile.receipt.json'),'source_sha256':{'original':sha(source)},'required_sha256':pins,'stdout_sha256':sha(destination/'stdout'),'stderr_sha256':sha(destination/'stderr'),'environment':{name:env[name] for name in ['TCL_LIBRARY'] if name in env},'request_sha256':sha(f/'request.json'),'provider_version_row':rows[0] if rows else None,'channel':'External byte-identical original-counted C API fullInit driver. Fresh interpreter/process per unchanged source, no outer eval/catch/puts wrapper. C Tcl_EvalEx flags0 or Jim_EvalObj actual counted source; whole public result bytes hex. Explicit original source catches individual operation APIs independently.'}
  for p,digest in pins.items():assert sha(Path(p))==digest,p
  (destination/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
  original=[row for row in rows if row.startswith('ORIGINAL|')];assert len(original)==1,rows
  detail={'provider':version,'case':case['id'],'exit':result.returncode,'original_code':int(original[0].split('|')[1]),'stderr_bytes':len(result.stderr),'rows':len(rows)};summary.append(detail);print(json.dumps(detail),flush=True)
(f/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
