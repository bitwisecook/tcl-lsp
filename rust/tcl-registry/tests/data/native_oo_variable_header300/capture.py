from pathlib import Path
import hashlib,json,os,subprocess,time
b=Path('/workspace/.proofs'); f=b/'native-oo-variable-header300'; req=json.loads((f/'request.json').read_text()); probe=f/'probe.c'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
for version in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0','jim']:
 old=json.loads((b/'native-jim-class-source269'/version/'receipt.json').read_text())
 directory=f/version;directory.mkdir(); exe=directory/'probe.elf'; env={**os.environ,**old['environment']}; pins=dict(old['required_sha256']);pins[str(probe)]=sha(probe)
 if version=='jim':
  root=b/'native-providers/jimtcl';header=root/'jim.h';lib=root/'libjim.a';flags=['-DUSE_JIM','-I'+str(root)];libs=['-lm','-lssl','-lcrypto','-lz','-ldl']
  for name in ['_load-static-exts.c','stdlib.tcl','_stdlib.c','jim-interp.c','jim-namespace.c']:pins[str(root/name)]=sha(root/name)
 else:
  root=Path('/workspace/tcl-lsp/tmp')/('tcl'+version);header=root/'generic/tcl.h';lib=root/'unix'/('libtcl'+'.'.join(version.split('.')[:2])+'.a');flags=['-I'+str(header.parent),'-I'+str(root/'unix')];libs=['-lm','-ldl','-lpthread','-lz'];env['TCL_LIBRARY']=str(root/'library')
  for name in ['tclNamesp.c','tclBasic.c','tclInterp.c','tclProc.c']:pins[str(root/'generic'/name)]=sha(root/'generic'/name)
 for p in [header,lib]:pins[str(p)]=sha(p)
 for name,digest in pins.items():assert sha(Path(name))==digest,name
 command=['cc','-std=c99',*flags,str(probe),str(lib),*libs,'-o',str(exe)];start=time.monotonic();r=subprocess.run(command,capture_output=True,timeout=60)
 (directory/'compile.stdout').write_bytes(r.stdout);(directory/'compile.stderr').write_bytes(r.stderr)
 compile_record={'command':command,'exit':r.returncode,'seconds':time.monotonic()-start,'required_sha256':pins,'source_sha256':sha(probe),'stdout_sha256':sha(directory/'compile.stdout'),'stderr_sha256':sha(directory/'compile.stderr')};assert r.returncode==0,(version,r.stderr)
 compile_record['executable_sha256']=sha(exe);(directory/'compile.receipt.json').write_text(json.dumps(compile_record,indent=2)+'\n')
 command=[str(exe)];start=time.monotonic();result=subprocess.run(command,env=env,capture_output=True,timeout=30)
 (directory/'stdout').write_bytes(result.stdout);(directory/'stderr').write_bytes(result.stderr)
 receipt={'provider':version,'command':command,'exit':result.returncode,'seconds':time.monotonic()-start,'rows':result.stdout.decode().splitlines(),'executable_sha256':sha(exe),'compile_receipt_sha256':sha(directory/'compile.receipt.json'),'driver_sha256':sha(probe),'required_sha256':pins,'stdout_sha256':sha(directory/'stdout'),'stderr_sha256':sha(directory/'stderr'),'environment':{name:env[name] for name in ['TCL_LIBRARY'] if name in env},'reported_cli_provider_version_association':old['reported_version_row'],'scope':'Unchanged requested public CAPI original counted61 00 7a declaration and direct info class variables same-header Boolean plus native getter bytes. No addresses emitted. Unsupported SDK purposes are not observations of absent arbitrary extension APIs.'}
 for name,digest in pins.items():assert sha(Path(name))==digest,name
 (directory/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps({'provider':version,'exit':result.returncode,'rows':receipt['rows'],'stderr_bytes':len(result.stderr)}),flush=True)
 assert result.returncode==0 and not result.stderr
