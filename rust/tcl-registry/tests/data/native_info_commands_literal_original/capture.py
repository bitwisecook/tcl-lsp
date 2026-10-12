from pathlib import Path
import hashlib,json,os,subprocess,time
base=Path(__file__).parent; home=Path('/workspace/.proofs');request=json.loads((base/'request.json').read_text());probe=base/'probe.c'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
summary=[]
for version in request['providers']:
 oldpath=home/'native-jim-class-source269'/version/'receipt.json';old=json.loads(oldpath.read_text());directory=base/'providers'/version;directory.mkdir(parents=True,exist_ok=True);env={**os.environ,**old['environment']};pins=dict(old['required_sha256']);pins[str(probe)]=sha(probe);pins[str(oldpath)]=sha(oldpath);pins[old['executable_path']]=old['executable_sha256']
 if version=='jim':
  root=home/'native-providers/jimtcl';lib=root/'libjim.a';flags=['-DJIM_PROBE','-I'+str(root)];libs=['-lm','-lssl','-lcrypto','-lz','-ldl'];names=['jim.h','jim.c','jim-namespace.c','jim-subcmd.c','nshelper.tcl','_nshelper.c','jimautoconf.h','_load-static-exts.c','stdlib.tcl','_stdlib.c']
  sources=[root/name for name in names]
 else:
  root=Path('/workspace/tcl-lsp/tmp')/('tcl'+version);lib=root/'unix'/('libtcl'+'.'.join(version.split('.')[:2])+'.a');flags=['-I'+str(root/'generic'),'-I'+str(root/'unix')];libs=['-lm','-ldl','-lpthread','-lz'];env['TCL_LIBRARY']=str(root/'library');names=['tcl.h','tclInt.h','tclIntDecls.h','tclCompile.h','tclCompile.c','tclCmdIL.c','tclCompCmdsGR.c','tclExecute.c','tclNamesp.c','tclBasic.c','tclUtil.c','tclCompCmds.c','tclProc.c','tclObj.c','tclStringObj.c','tclDisassemble.c'];sources=[root/'generic'/name for name in names if (root/'generic'/name).is_file()]
 for p in [lib,*sources]:pins[str(p)]=sha(p)
 for name,digest in pins.items():assert sha(Path(name))==digest,name
 exe=directory/'probe.elf';cmd=['cc','-std=c99',*flags,str(probe),str(lib),*libs,'-o',str(exe)];started=time.monotonic();r=subprocess.run(cmd,capture_output=True,timeout=60);(directory/'compile.stdout').write_bytes(r.stdout);(directory/'compile.stderr').write_bytes(r.stderr)
 cr={'provider':version,'command':cmd,'exit':r.returncode,'seconds':time.monotonic()-started,'required_sha256':pins,'stdout_sha256':sha(directory/'compile.stdout'),'stderr_sha256':sha(directory/'compile.stderr')}
 if r.returncode==0:cr['executable_sha256']=sha(exe)
 (directory/'compile.receipt.json').write_text(json.dumps(cr,indent=2)+'\n')
 if r.returncode:
  print(json.dumps({'provider':version,'compile_exit':r.returncode,'stderr':r.stderr.decode()[:3000]}),flush=True);summary.append({'provider':version,'availability':'harness_compile_failed','compile_receipt_sha256':sha(directory/'compile.receipt.json')});continue
 for case in request['cases']:
  if case['providers'] and version not in case['providers']:continue
  target=directory/case['id'];target.mkdir(exist_ok=True);source=Path(case['body_file']);prelude=Path(case['prelude_file']);assert sha(source)==case['body_sha256'];assert sha(prelude)==case['prelude_sha256'];arg=Path(case['argument_file']) if case['argument_file'] else None
  if arg:assert sha(arg)==case['argument_sha256']
  cmd=[str(exe),str(prelude),str(source),case['procedure'],case['parameters'],str(arg) if arg else '-','-'];started=time.monotonic();r=subprocess.run(cmd,env=env,capture_output=True,timeout=30);(target/'stdout').write_bytes(r.stdout);(target/'stderr').write_bytes(r.stderr)
  rows=r.stdout.decode('utf-8','backslashreplace').splitlines();rec={'provider':version,'question':request['question'],'case':case['id'],'command':cmd,'exit':r.returncode,'seconds':time.monotonic()-started,'source_sha256':{'original_body':sha(source),'independent_prelude':sha(prelude),**({'original_argument':sha(arg)} if arg else {})},'source_channels':case['source_channel'],'required_sha256':pins,'executable_sha256':sha(exe),'compile_receipt_sha256':sha(directory/'compile.receipt.json'),'stdout_sha256':sha(target/'stdout'),'stderr_sha256':sha(target/'stderr'),'rows':rows,'environment':{name:env[name] for name in ['TCL_LIBRARY'] if name in env},'original_cli_version_receipt_sha256':sha(oldpath),'original_cli_reported_version_row':old['reported_version_row'],'observation_scope':'Public original proc installation and direct original object argv invocation; pre-counted-string-getter native result header and retained compiled-body literal/token headers, decoded original opcode byte stream against original instruction table, followed by original native disassembly when available. Live retained headers only. Software backend was not run.'}
  for name,digest in pins.items():assert sha(Path(name))==digest,name
  (target/'receipt.json').write_text(json.dumps(rec,indent=2)+'\n');print(json.dumps({'provider':version,'case':case['id'],'exit':r.returncode,'completion':[row for row in rows if row.startswith('COMPLETION|ORIGINAL|')],'header':[row for row in rows if row.startswith('HEADER|ORIGINAL|')],'result':[row for row in rows if row.startswith('RESULT_BYTES|ORIGINAL|')],'recipe':any('|resolveCmd|' in row for row in rows),'disassembly_code':[row.split('|')[1] for row in rows if row.startswith('ORIGINAL_DISASSEMBLY|')],'stderr_bytes':len(r.stderr)}),flush=True)
 summary.append({'provider':version,'availability':'original_harness_executed','compile_receipt_sha256':sha(directory/'compile.receipt.json'),'case_count':len(list(directory.glob('*/receipt.json')))})
(base/'capture-summary.json').write_text(json.dumps({'question':request['question'],'request_sha256':sha(base/'request.json'),'providers':summary},indent=2)+'\n')
