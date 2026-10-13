from pathlib import Path
import argparse, gzip, hashlib, json, os, shlex, subprocess, time
parser=argparse.ArgumentParser();parser.add_argument('--root',default='/workspace/tcl-lsp');parser.add_argument('--out',required=True);parser.add_argument('--target',required=True);args=parser.parse_args()
root=Path(args.root).resolve();out=Path(args.out).resolve();target=Path(args.target).resolve();out.mkdir(parents=True,exist_ok=True);target.mkdir(parents=True,exist_ok=True)
env=os.environ.copy();env.update(CARGO_TARGET_DIR=str(target),CARGO_BUILD_JOBS='1',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0')
plan=json.loads(Path('/tmp/runtime-alias-query-adoption-20261013/rerun-plan.json').read_text())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def snapshot():
 paths=set()
 for family in ['rust','runtime/rust','experiments/name-handling','scripts','specs']:
  d=root/family
  paths.update(p.resolve() for p in d.rglob('*') if p.is_file() and 'target' not in p.parts and '.git' not in p.parts)
 for name in ['Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo/config.toml','runtime/rust/Cargo.lock']:
  p=root/name
  if p.is_file():paths.add(p.resolve())
 # Full dependency-source closure is captured before compilation, independent of old dep-info.
 registry=Path(env.get('CARGO_HOME','/workspace/.cargo'))/'registry/src'
 paths.update(p.resolve() for p in registry.rglob('*') if p.is_file())
 for key in ['TCL_LSP_TCLSH84','TCL_LSP_TCLSH85','TCL_LSP_TCLSH86','TCL_LSP_TCLSH90','TCL_LSP_TCLSH91','TCL_LSP_JIMSH','TCL_TOMMATH_DIR']:
  p=Path(env[key]).resolve()
  if p.is_file():
   paths.add(p)
   for line in subprocess.check_output(['ldd',str(p)],text=True).splitlines():
    paths.update(Path(word).resolve() for word in line.split() if word.startswith('/') and Path(word).is_file())
   tree=p.parent.parent if p.name=='tclsh' else p.parent
   paths.update(f.resolve() for f in tree.rglob('*') if f.is_file() and f.suffix in ['.tcl','.tm','.h','.c','.a','.sh'])
  else:paths.update(f.resolve() for f in p.rglob('*') if f.is_file())
 return {str(p):sha(p) for p in sorted(paths)}
before=snapshot()
with gzip.open(out/'source-before.json.gz','wt') as f:json.dump(before,f,sort_keys=True)
(out/'selected-plan.json').write_text(json.dumps(plan,indent=2)+'\n')
builds=[('runtime',['cargo','test','--offline','--manifest-path',str(root/'runtime/rust/Cargo.toml'),'--features','engine','--lib','--test','native_namespace_command_query','--no-run','--message-format=json','-j1']),('registry',['cargo','test','--offline','-p','tcl-registry','--lib','--no-run','--message-format=json','-j1'])]
artifacts={};build_rows=[]
for label,cmd in builds:
 start=time.monotonic()
 with (out/(label+'-build.jsonl')).open('w') as stdout,(out/(label+'-build.stderr')).open('w') as stderr:run=subprocess.run(cmd,cwd=root,env=env,stdout=stdout,stderr=stderr)
 build_rows.append({'label':label,'argv':cmd,'exit_code':run.returncode,'elapsed_seconds':time.monotonic()-start});(out/'builds.json').write_text(json.dumps(build_rows,indent=2)+'\n')
 if run.returncode:raise SystemExit('build failed: '+label)
 for line in (out/(label+'-build.jsonl')).read_text().splitlines():
  try:row=json.loads(line)
  except ValueError:continue
  if row.get('reason')=='compiler-artifact' and row.get('profile',{}).get('test') and row.get('executable'):artifacts[row['target']['name']]=Path(row['executable'])
inputs={};unresolved=[];uncovered=[];dep_info={}
for p in target.rglob('*.d'):
 dep_info[str(p)]=sha(p)
 for line in p.read_text(errors='strict').replace('\\\n','').splitlines():
  if ': ' not in line:continue
  for word in shlex.split(line.split(': ',1)[1]):
   q=Path(word)
   if not q.is_absolute():q=next((c for c in [root/'runtime/rust'/q,root/q] if c.is_file()),q)
   if not q.is_file():unresolved.append({'dep_info':str(p),'input':word});continue
   q=q.resolve();inputs[str(q)]=sha(q)
   if str(q) not in before and not q.is_relative_to(target):uncovered.append(str(q))
closure={'inputs':inputs,'dep_info':dep_info,'unresolved':unresolved,'uncovered':sorted(set(uncovered)),'artifacts':{str(p):sha(p) for p in artifacts.values()}}
(out/'compiled-closure.json').write_text(json.dumps(closure,indent=2)+'\n')
if unresolved or uncovered:raise SystemExit('compiled coverage incomplete; no tests attempted')
exact=[(artifacts['tcl_runtime'],name) for name in plan['runtime']]+[(artifacts['native_namespace_command_query'],name) for name in plan['query']]+[(artifacts['tcl_registry'],name) for name in plan['shared_registry']]
listings={}
for binary in set(binary for binary,_ in exact):
 listings[binary]=subprocess.check_output([str(binary),'--list'],text=True,env=env,cwd=root)
 (out/(binary.name+'.list')).write_text(listings[binary])
for binary,name in exact:
 if name+': test' not in listings[binary]:raise SystemExit('missing exact selector: '+name)
(out/'exact-plan.json').write_text(json.dumps([{'binary':str(binary),'selector':name} for binary,name in exact],indent=2)+'\n')
runs=[]
for i,(binary,name) in enumerate(exact):
 cmd=[str(binary),name,'--exact','--nocapture','--test-threads=1'];start=time.monotonic();log=out/f'test-{i+1:02}.log'
 with log.open('w') as f:
  try:run=subprocess.run(cmd,cwd=root,env=env,stdout=f,stderr=subprocess.STDOUT,timeout=180);code=run.returncode
  except subprocess.TimeoutExpired:code='timeout'
 runs.append({'selector':name,'argv':cmd,'exit_code':code,'elapsed_seconds':time.monotonic()-start,'log':log.name,'log_sha256':sha(log)})
 (out/'runs-in-progress.json').write_text(json.dumps(runs,indent=2)+'\n')
after=snapshot()
with gzip.open(out/'source-after.json.gz','wt') as f:json.dump(after,f,sort_keys=True)
result={'runs':runs,'attempted':len(runs),'passed':sum(row['exit_code']==0 for row in runs),'failed':sum(row['exit_code']!=0 for row in runs),'source_drift':{p:[before.get(p),after.get(p)] for p in set(before)|set(after) if before.get(p)!=after.get(p)},'compiled_input_drift':{p:[digest,sha(Path(p)) if Path(p).is_file() else None] for p,digest in inputs.items() if not Path(p).is_file() or sha(Path(p))!=digest},'artifact_drift':{p:[digest,sha(Path(p))] for p,digest in closure['artifacts'].items() if sha(Path(p))!=digest},'scope':'Exact Runtime75 and Registry2; Engine8 remains its separately required host/guest capture'}
(out/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:result[k] for k in ['attempted','passed','failed']}))
