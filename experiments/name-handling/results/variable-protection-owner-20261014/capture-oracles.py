from pathlib import Path
import hashlib,json,os,subprocess,time
out=Path('/tmp/name-variable-protection-owner-20261014/oracles');probe=out/'literal-observer-targets.tcl';rows=[]
for key in ['TCL_LSP_TCLSH84','TCL_LSP_TCLSH85','TCL_LSP_TCLSH86','TCL_LSP_TCLSH90','TCL_LSP_TCLSH91','TCL_LSP_JIMSH']:
 exe=Path(os.environ[key]).resolve();version=exe.parent.parent;inputs={str(probe):probe}
 for path in [exe,*exe.parent.glob('libtcl*.so*')]:
  if path.is_file():inputs[str(path)]=path
 library=version/'library'
 if library.is_dir():
  for path in library.rglob('*'):
   if path.is_file():inputs[str(path)]=path
 def snap():return {name:hashlib.sha256(path.read_bytes()).hexdigest() for name,path in sorted(inputs.items())}
 before=snap();start=time.time();run=subprocess.run([str(exe),str(probe)],cwd=out,capture_output=True,timeout=30);after=snap();label=key.removeprefix('TCL_LSP_');(out/(label+'.stdout')).write_bytes(run.stdout);(out/(label+'.stderr')).write_bytes(run.stderr);(out/(label+'.inputs-before.json')).write_text(json.dumps(before,indent=2)+'\n');(out/(label+'.inputs-after.json')).write_text(json.dumps(after,indent=2)+'\n');rows.append({'provider':key,'executable':str(exe),'arguments':[str(exe),str(probe)],'input_channel':'Original Tcl source file','exit_code':run.returncode,'seconds':time.time()-start,'changed_inputs':{name:{'before':value,'after':after.get(name)} for name,value in before.items() if after.get(name)!=value},'input_scope':'Probe, selected interpreter executable, selected adjacent Tcl shared libraries and complete original matching library tree when present; unrecorded system dynamic dependencies are not a complete host closure. No tcltest was invoked.','stdout':label+'.stdout','stderr':label+'.stderr'})
(out/'observations.json').write_text(json.dumps({'scope':'Original interpreter observations for literal variable names and observer-target grammar only. No compiler source/cell/Native-entry permission or F5 answer follows.','rows':rows},indent=2)+'\n');print(json.dumps([{'provider':r['provider'],'exit_code':r['exit_code'],'changed_inputs':len(r['changed_inputs'])} for r in rows]))
