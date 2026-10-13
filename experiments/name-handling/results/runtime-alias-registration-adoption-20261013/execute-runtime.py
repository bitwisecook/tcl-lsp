from pathlib import Path
import gzip,hashlib,json,shlex,subprocess,time
root=Path('/workspace/name-implementation-worktrees/runtime');out=Path('/tmp/runtime-naming-final-capture-20261013');target=root/'target'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def artifact(log,name):
 for l in (out/log).read_text().splitlines():
  try:r=json.loads(l)
  except ValueError:continue
  if r.get('reason')=='compiler-artifact' and r.get('target',{}).get('name')==name and r.get('profile',{}).get('test') and r.get('executable'):return Path(r['executable'])
 raise RuntimeError(name+' executable absent')
runtime=artifact('build-runtime-01.jsonl','tcl_runtime')
query=artifact('build-runtime-01.jsonl','native_namespace_command_query')
with gzip.open(out/'source-before-01.json.gz','rt') as f:sources=json.load(f)
inputs={};unresolved=[];uncovered=[];dep_info={}
for p in target.rglob('*.d'):
 dep_info[str(p)]=sha(p)
 for line in p.read_text().replace('\\\n','').splitlines():
  if ': ' not in line:continue
  for s in shlex.split(line.split(': ',1)[1]):
   q=Path(s)
   if not q.is_absolute():q=next((c for c in [root/'runtime/rust'/q,root/q] if c.is_file()),q)
   if not q.is_file():unresolved.append({'dep_info':str(p),'input':s});continue
   q=q.resolve();inputs[str(q)]=sha(q)
   if str(q) not in sources and not q.is_relative_to(target):uncovered.append(str(q))
result={'source_before':'source-before-01.json.gz','compiled_inputs':inputs,'dep_info':dep_info,'unresolved':unresolved,'uncovered':sorted(set(uncovered)),'artifacts':{str(p):sha(p) for p in [runtime,query]},'runs':[]}
(out/'compiled-inputs.json').write_text(json.dumps(result,indent=2)+'\n')
if unresolved or uncovered:raise SystemExit('compiled input coverage incomplete; no tests run')
selectors=json.loads((out/'exact-plan-proposed.json').read_text())
runtime_names=selectors['runtime'];query_names=selectors['query']
plan=[(runtime,n) for n in runtime_names]+[(query,n) for n in query_names]
for binary,n in plan:
 listing=subprocess.check_output([str(binary),'--list'],text=True)
 if n+': test' not in listing:raise SystemExit('Missing selector '+n)
(out/'exact-plan.json').write_text(json.dumps([{'binary':str(b),'selector':n} for b,n in plan],indent=2)+'\n')
for i,(binary,n) in enumerate(plan):
 command=[str(binary),n,'--exact','--nocapture','--test-threads=1'];log=out/f'test-{i+1:02}.log';t=time.monotonic()
 with log.open('w') as f:r=subprocess.run(command,cwd=root,stdout=f,stderr=subprocess.STDOUT,timeout=120)
 row={'selector':n,'binary':str(binary),'exit_code':r.returncode,'elapsed_seconds':time.monotonic()-t,'log':str(log),'sha256':sha(log),'exact_one_test':'running 1 test\n' in log.read_text(),'one_passed':'test result: ok. 1 passed; 0 failed;' in log.read_text()};result['runs'].append(row);(out/'result-in-progress.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(row),flush=True)
result['changed_compiled_inputs']={p:sha(Path(p)) for p,h in inputs.items() if sha(Path(p))!=h};result['changed_artifacts']={p:sha(Path(p)) for p,h in result['artifacts'].items() if sha(Path(p))!=h};(out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
