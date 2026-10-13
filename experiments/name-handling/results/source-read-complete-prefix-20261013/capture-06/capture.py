from pathlib import Path
import subprocess,json,hashlib,gzip,datetime,time,os,re
root=Path('/workspace/name-implementation-worktrees/source_flow');out=Path(__file__).parent;os.chdir(root)
def write(name,value):(out/name).write_text(json.dumps(value,indent=2)+'\n')
def digest(f):return hashlib.sha256(f.read_bytes()).hexdigest()
def inventory():
 files=set(subprocess.check_output(['rg','--files','--hidden','rust','runtime/rust'],text=True).splitlines())
 files.update(['Cargo.toml','Cargo.lock','rust-toolchain.toml','experiments/name-handling/tcltest-shared-frame/probe.tcl','experiments/name-handling/patterns/composed-callback-prefix/fixtures/private-positive.tcl'])
 files.update(str(f.relative_to(root)) for f in (root/'.cargo').rglob('*') if f.is_file())
 return {f:digest(root/f) for f in sorted(files) if (root/f).is_file()}
start=inventory();gzip.open(out/'source-before.json.gz','wt').write(json.dumps(start,sort_keys=True))
command=['cargo','test','--offline','-j','1','-p','tcl-compiler','--lib','--no-run']
write('build-start.json',{'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'command':command,'source_paths':len(start),'env':{k:v for k,v in os.environ.items() if k.startswith(('TCL_LSP_','CARGO_PROFILE_','CARGO_TARGET_','CARGO_INCREMENTAL'))}})
t=time.monotonic()
with (out/'build.log').open('wb') as log:code=subprocess.call(command,stdout=log,stderr=subprocess.STDOUT)
write('build-result.json',{'exit_code':code,'seconds':time.monotonic()-t})
if code:print('build failed',code,flush=True);raise SystemExit(code)
matches=re.findall(r'Executable unittests [^\n(]+\(([^)]+)\)',(out/'build.log').read_text());assert len(matches)==1,matches;b=Path(matches[0]);b=b if b.is_absolute() else root/b
write('executed-binary.json',{'path':str(b),'sha256':digest(b),'bytes':b.stat().st_size})
write('compiled-dependencies.json',{str(f):digest(f) for f in b.parent.glob('*') if f.is_file() and f.suffix in ('.rlib','.rmeta','.so')})
oracles={k:{'path':v,'sha256':digest(Path(v))} for k,v in os.environ.items() if k.startswith('TCL_LSP_') and Path(v).is_file()}
write('oracle-seals.json',oracles)
libraries=json.loads((out/'actual-oracle-library-retention-02.json').read_text())
for key, record in libraries.items():
 if key != 'Jim': assert digest(Path(record['binary'])) == record['binary_sha256']
 for path,seal in record['retained_originals'].items(): assert digest(Path(path)) == seal['sha256']
write('oracle-library-seals.json',libraries)
results=[]
for i,n in enumerate(json.loads((out/'selectors.json').read_text())):
 command=[str(b),n,'--exact','--nocapture','--test-threads=1'];t=time.monotonic();timed=False
 with (out/f'{i:02}.log').open('wb') as log:
  proc=subprocess.Popen(command,stdout=log,stderr=subprocess.STDOUT)
  try:code=proc.wait(timeout=240)
  except subprocess.TimeoutExpired:timed=True;proc.terminate();code=proc.wait(timeout=20)
 result={'selector':n,'command':command,'exit_code':code,'seconds':time.monotonic()-t,'timed_out':timed};write(f'{i:02}.json',result);results.append(result);print(json.dumps(result),flush=True)
 end=inventory();gzip.open(out/'source-end.json.gz','wt').write(json.dumps(end,sort_keys=True));write('result.json',{'source_paths':len(start),'changed_sources':{f:{'before':start.get(f),'after':end.get(f)} for f in set(start)|set(end) if start.get(f)!=end.get(f)},'executed':len(results),'passed':sum(r['exit_code']==0 for r in results),'failed':sum(r['exit_code']!=0 for r in results),'controls':results})
