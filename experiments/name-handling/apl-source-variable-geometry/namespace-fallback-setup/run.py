from pathlib import Path
import hashlib,json,subprocess,os
out=Path(__file__).parent; script=out/'source-variable-geometry.tcl'
old=Path('/tmp/name-method-typed-owners-20261013/oo-frame-followon/oracles')
results=[]
for inv in sorted(old.glob('*.before.json')):
 provider=inv.name.removesuffix('.before.json'); paths=list(json.loads(inv.read_text()))
 exe=next(p for p in paths if p.endswith('/tclsh') or p.endswith('/jimsh'))
 def seal():return {p:hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in paths}
 before=seal(); (out/(provider+'.before.json')).write_text(json.dumps(before,indent=2)+'\n')
 env=os.environ.copy()
 library=next((str(Path(p).parent) for p in paths if p.endswith('/library/init.tcl')),None)
 if library:env['TCL_LIBRARY']=library
 result=subprocess.run([exe,str(script)],env=env,capture_output=True,timeout=30)
 after=seal(); (out/(provider+'.after.json')).write_text(json.dumps(after,indent=2)+'\n')
 (out/(provider+'.stdout')).write_bytes(result.stdout);(out/(provider+'.stderr')).write_bytes(result.stderr)
 results.append({'provider':provider,'executable':exe,'library':library,'exit':result.returncode,'files':len(paths),'drift':before!=after,'input_sha256':hashlib.sha256(script.read_bytes()).hexdigest(),'stdout_sha256':hashlib.sha256(result.stdout).hexdigest(),'stderr_sha256':hashlib.sha256(result.stderr).hexdigest(),'stdout':result.stdout.decode(),'stderr':result.stderr.decode()})
(out/'results.json').write_text(json.dumps(results,indent=2)+'\n')
for row in results: print(row['provider'],row['exit'],row['drift'],repr(row['stdout']))
