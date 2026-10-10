from pathlib import Path
import json,hashlib,subprocess,os,time
root=Path('/workspace/tcl-lsp');queue=Path('/workspace/.proofs/recovered-configurable-bootstrap64/queue.json');plan=json.loads(queue.read_text());source=Path(plan['source']);out=Path('/workspace/.proofs/captured-configurable-bootstrap064');out.mkdir(exist_ok=False)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
if sha(source)!=plan['source_sha256']:raise RuntimeError('original source changed')
captures=[]
for provider,version in [('tcl8.4','8.4.20'),('tcl8.5','8.5.19'),('tcl8.6','8.6.18'),('tcl9.0','9.0.4'),('tcl9.1','9.1.0'),('jim','jim')]:
 d=out/provider;d.mkdir();env=os.environ.copy()
 if provider=='jim':
  base=Path('/workspace/.proofs/native-providers/jimtcl');exe=base/'jimsh';header=base/'jim.h';lib=base/'libjim.a';makefile=base/'Makefile';owner=base/'jim.c'
 else:
  base=root/'tmp'/('tcl'+version);exe=base/'unix/tclsh';header=base/'generic/tcl.h';lib=base/'unix'/('libtcl'+'.'.join(version.split('.')[:2])+'.a');makefile=base/'unix/Makefile';owner=base/'generic'/('tclOO.c' if version not in ['8.4.20','8.5.19'] else 'tclBasic.c');env['TCL_LIBRARY']=str(base/'library')
 version_input=b'puts [info patchlevel]\n';started=time.time();startup=subprocess.run([str(exe)],input=version_input,capture_output=True,env=env,timeout=30)
 for name,value in [('startup.input',version_input),('startup.stdout',startup.stdout),('startup.stderr',startup.stderr)]: (d/name).write_bytes(value)
 if startup.returncode or startup.stderr:raise RuntimeError('startup failure '+provider)
 command=[str(exe),str(source)];result=subprocess.run(command,capture_output=True,env=env,timeout=30)
 (d/'stdout').write_bytes(result.stdout);(d/'stderr').write_bytes(result.stderr)
 receipt=dict(provider=provider,reported_version=startup.stdout.decode('ascii').strip(),source=str(source),source_sha256=sha(source),input_channel='Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.',command=command,executable=str(exe),executable_sha256=sha(exe),header_sha256=sha(header),library_sha256=sha(lib),makefile_sha256=sha(makefile),source_owner=str(owner),source_owner_sha256=sha(owner),startup_command=[str(exe)],startup_exit=startup.returncode,startup_input_sha256=sha(d/'startup.input'),startup_stdout_sha256=sha(d/'startup.stdout'),startup_stderr_sha256=sha(d/'startup.stderr'),process_exit=result.returncode,stdout_sha256=sha(d/'stdout'),stderr_sha256=sha(d/'stderr'),started=started,finished=time.time())
 (d/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');captures.append(receipt);(out/'receipt.json').write_text(json.dumps(captures,indent=2)+'\n')
 print(provider,receipt['reported_version'],'process',result.returncode,'stderrbytes',len(result.stderr),flush=True);print(result.stdout.decode('ascii'),flush=True)
 if result.returncode or result.stderr:raise RuntimeError('native script process failure '+provider)
