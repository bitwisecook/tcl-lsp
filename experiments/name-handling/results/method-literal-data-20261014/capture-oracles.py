from pathlib import Path
import json,hashlib,os,subprocess,datetime
out=Path('/tmp/name-method-literal-data-20261014/oracles');root=Path('/workspace/tcl-lsp');probe=out/'literal-namespace-and-method-names.tcl'
interpreters={f'C{v}':root/f'tmp/tcl{v}/unix/tclsh' for v in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0']}
jim=os.environ.get('TCL_LSP_JIMSH')
if jim:interpreters['Jim']=Path(jim)
else:
 for p in [root/'tmp/jimtcl/jimsh',Path('/workspace/.codex/oracle-bin/jimsh'),Path('/workspace/oracle-bin/jimsh')]:
  if p.exists():interpreters['Jim']=p;break
records=[]
def snapshot(exe):
 inputs=[probe,exe.resolve()];inputs+=list(exe.parent.glob('libtcl*.so'))
 library=exe.parent.parent/'library'
 if library.is_dir():inputs+=sorted(p for p in library.rglob('*') if p.is_file())
 return {str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in inputs}
for label,exe in interpreters.items():
 before=snapshot(exe);(out/(label+'.inputs-before.json')).write_text(json.dumps(before,indent=2)+'\n')
 result=subprocess.run([str(exe),str(probe)],cwd=root,capture_output=True,timeout=30)
 (out/(label+'.stdout')).write_bytes(result.stdout);(out/(label+'.stderr')).write_bytes(result.stderr)
 after=snapshot(exe);(out/(label+'.inputs-after.json')).write_text(json.dumps(after,indent=2)+'\n')
 records.append({'provider':label,'executable':str(exe),'exit_code':result.returncode,'input_drift':[p for p in before if before.get(p)!=after.get(p)],'scope':'Original executable, probe, adjacent Tcl library shared object and matching release library captured where present. Host/loader dynamic dependency closure not certified. Direct interpreter observations only; no compiler or Native admission claim.'})
 (out/'observations.json').write_text(json.dumps({'captured_at_utc':datetime.datetime.now(datetime.UTC).isoformat(),'records':records,'f5':'not observed','tcltest':'not used; direct Tcl program'},indent=2)+'\n')
 print(label,'exit',result.returncode,'input_drift',len(records[-1]['input_drift']))
