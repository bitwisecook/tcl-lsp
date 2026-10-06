import pathlib,subprocess,hashlib,json,time,fcntl,os,signal
root=pathlib.Path('/workspace/tcl-lsp'); out=pathlib.Path(__file__).parent
sha=lambda p:hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
def execute(command,native_env=None):
 slot=None
 while slot is None:
  for number in range(2):
   lock=open('/workspace/.proofs/2286-test-slot-'+str(number)+'.lock','a')
   try:fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
   except BlockingIOError:lock.close();continue
   slot=lock;break
  if slot is None:time.sleep(.1)
 start=time.monotonic();env=os.environ.copy();env.update(native_env or {});p=subprocess.Popen(command,cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True)
 try:stdout,stderr=p.communicate(timeout=60);code=p.returncode;timeout=False
 except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);stdout,stderr=p.communicate();code=p.returncode;timeout=True
 finally:slot.close()
 return dict(command=command,exit=code,timeout=timeout,seconds=time.monotonic()-start,stdout=stdout.decode(errors='backslashreplace'),stderr=stderr.decode(errors='backslashreplace'),original_budget=60,global_slots=2,explicit_environment=native_env or {})
manifest=[]
for version in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0','Jim']:
 if version=='Jim':
  base=pathlib.Path('/tmp/2286-oracles/jimtcl');source=out/'jim_enum_probe.c';header=base/'jim.h';library=base/'libjim.a';inputs=[header,base/'jim.c',library,source];flags=['-lm','-ldl','-lssl','-lcrypto','-lz']
 else:
  base=root/'tmp'/('tcl'+version);source=out/'interp_probe.c';header=base/'generic/tcl.h';library=base/'unix'/('libtcl'+'.'.join(version.split('.')[:2])+'.a');inputs=[header,base/'generic/tclInterp.c',base/'generic/tclIndexObj.c',library,source];flags=['-lm','-ldl','-lpthread','-lz'];
  if version=='9.1.0':inputs.append(base/'generic/tclOOProp.c')
 binary=out/('probe-'+version)
 command=['cc','-I'+str(header.parent),str(source),str(library),*flags,'-o',str(binary)]
 compiled=execute(command);entry=dict(version=version,compile=compiled,sha256={str(p):sha(p) for p in inputs})
 if compiled['exit']==0:
  entry['sha256'][str(binary)]=sha(binary);run=execute([str(binary)], None if version=='Jim' else {'TCL_LIBRARY':str(base/'library')});entry['run']=run
  log=out/(version+'.tsv');log.write_text(run['stdout']);entry['sha256'][str(log)]=sha(log)
 manifest.append(entry);(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
 print(version,'compile',compiled['exit'],'run',entry.get('run',{}).get('exit'),'rows',entry.get('run',{}).get('stdout','').count('\n'),flush=True)
