import gzip,hashlib,json,os,pathlib,shutil,signal,subprocess,time
ROOT=pathlib.Path('/workspace/tcl-lsp');OUT=ROOT/'experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-runtime/validation-01';OUT.mkdir(exist_ok=False);sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();ready=json.loads((OUT.parent/'applied-caller-context-repair/source-ready.json').read_text());owners=ready['sources'];assert all(sha(ROOT/p)==item['applied_sha256'] for p,item in owners.items());env=dict(os.environ);env['CARGO_TARGET_DIR']=str(ROOT/'tmp/name-target-runtime');cargo=ROOT/'tmp/name-tools/cargo/bin/cargo';margv=[str(cargo),'metadata','--offline','--manifest-path','runtime/rust/Cargo.toml','--format-version=1'];mr=subprocess.run(margv,cwd=ROOT,env=env,capture_output=True);(OUT/'metadata.stderr.txt').write_bytes(mr.stderr);(OUT/'runtime-cargo-metadata.json.gz').write_bytes(gzip.compress(mr.stdout,mtime=0));assert mr.returncode==0
m=json.loads(mr.stdout);pkgs={p['id']:p for p in m['packages']};nodes={p['id']:p for p in m['resolve']['nodes']};rid=m['resolve']['root'];todo=[rid];seen=set()
while todo:
 pid=todo.pop()
 if pid in seen:continue
 seen.add(pid);todo.extend(d['pkg'] for d in nodes[pid]['deps'] if pid==rid or any(k['kind'] in (None,'build') for k in d['dep_kinds']))
local=[pkgs[p] for p in seen if pathlib.Path(pkgs[p]['manifest_path']).is_relative_to(ROOT) and not pathlib.Path(pkgs[p]['manifest_path']).is_relative_to(ROOT/'tmp')];assert len(local)==14
fixtures=['experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/'+p for p in ['input.tcl','variable-head-input.tcl','capture-01/8.6.18/stdout.txt','capture-02/8.6.18/stdout.txt']]
def snapshot():
 selected={}
 for pkg in local:
  d=pathlib.Path(pkg['manifest_path']).parent
  for p in d.rglob('*'):
   if p.is_file() and (p.suffix in ('.rs','.c','.h','.tcl','.txt','.json') or p.name in ('Cargo.toml','Cargo.lock')) and not any(x in ('target','.git') for x in p.relative_to(d).parts):selected[str(p.relative_to(ROOT))]=sha(p)
 for rel in fixtures+['Cargo.toml','Cargo.lock','runtime/rust/Cargo.lock','.cargo/config.toml','rust-toolchain.toml']:
  if (ROOT/rel).is_file():selected[rel]=sha(ROOT/rel)
 return dict(sorted(selected.items()))
def save_snapshot(label,data):
 with gzip.open(OUT/f'production-source-{label}.json.gz','wt') as f:json.dump(data,f,indent=2)
before=snapshot();save_snapshot('before',before);scope={'packages':sorted(p['name'] for p in local),'count':len(before),'metadata_argv':margv,'metadata_sha256':hashlib.sha256(mr.stdout).hexdigest(),'owners':owners,'scope':'Actual standalone default-feature Runtime lib-test graph: root all dependency kinds, other packages normal/build; conservative local rs/c/h/tcl/txt/json/manifests and exact retained oracle fixtures. Syntax/Registry/Core included; Compiler/VM/LSP/IRules/MCP/xtask excluded. Not all-workspace freshness or every-file compilation.'};(OUT/'scope.json').write_text(json.dumps(scope,indent=2)+'\n')
for p in owners:
 with gzip.open(OUT/(p.replace('/','--')+'.gz'),'wb') as f:f.write((ROOT/p).read_bytes())
names=['interp::native_variable_names::tests::'+name for name in ready['tests']['runtime/rust/src/interp/native_variable_names.rs']];record={'status':'prepared','build':{},'checks':[],'unexecuted_names':names,'metadata_exit_code':mr.returncode}
def save():(OUT/'validation.json').write_text(json.dumps(record,indent=2)+'\n')
bargv=[str(cargo),'test','--offline','-j2','--manifest-path','runtime/rust/Cargo.toml','--lib','--no-run','--message-format=json'];record['build']={'argv':bargv,'started_utc':time.strftime('%Y-%m-%d %H:%M:%S UTC',time.gmtime()),'initial_free_bytes':shutil.disk_usage(env['CARGO_TARGET_DIR']).free,'minimum_free_bytes':shutil.disk_usage(env['CARGO_TARGET_DIR']).free,'disk_abort_threshold_bytes':192*1024**2,'status':'running'};save();print('BUILD START Runtime14',len(before),'source inputs',flush=True)
with (OUT/'build-messages.jsonl').open('wb') as stdout,(OUT/'build.stderr.txt').open('wb') as stderr:
 proc=subprocess.Popen(bargv,cwd=ROOT,env=env,stdout=stdout,stderr=stderr,start_new_session=True)
 while proc.poll() is None:
  free=shutil.disk_usage(env['CARGO_TARGET_DIR']).free;record['build']['minimum_free_bytes']=min(record['build']['minimum_free_bytes'],free)
  if free<record['build']['disk_abort_threshold_bytes']:
   record['build']['disk_aborted']=True;os.killpg(proc.pid,signal.SIGTERM);break
  time.sleep(1)
 code=proc.wait()
record['build'].update(exit_code=code,status='completed',finished_utc=time.strftime('%Y-%m-%d %H:%M:%S UTC',time.gmtime()),messages_sha256=sha(OUT/'build-messages.jsonl'),stderr_sha256=sha(OUT/'build.stderr.txt'));save();print('BUILD EXIT',code,flush=True)
after=snapshot();save_snapshot('after-build',after);changed=[p for p in sorted(before.keys()|after.keys()) if before.get(p)!=after.get(p)];record['changed_source_during_build']=changed;save()
if code==0 and not changed:
 messages=[json.loads(l) for l in (OUT/'build-messages.jsonl').read_text().splitlines() if l.startswith('{')];arts=[a for a in messages if a.get('reason')=='compiler-artifact' and a['target']['name']=='tcl_runtime' and a['profile']['test'] and a['executable']];assert len(arts)==1;artifact=arts[0];binary=pathlib.Path(artifact['executable']);digest=sha(binary);record['build'].update(binary=str(binary),binary_sha256=digest,artifact_fresh=artifact.get('fresh'),cargo_profile=artifact['profile']);listed=subprocess.run([str(binary),'--list'],cwd=ROOT,capture_output=True);(OUT/'test-list.stdout.txt').write_bytes(listed.stdout);(OUT/'test-list.stderr.txt').write_bytes(listed.stderr);assert listed.returncode==0
 for name in names:assert listed.stdout.decode().splitlines().count(name+': test')==1,name
 record['observed_exact_names']=names;save()
 for index,name in enumerate(names,1):
  argv=[str(binary),name,'--exact','--test-threads=1','--nocapture'];run=subprocess.run(argv,cwd=ROOT,capture_output=True);log=OUT/f'{index:02d}-{name.rsplit("::",1)[1]}.log';log.write_bytes(run.stdout+run.stderr);record['checks'].append({'name':name,'argv':argv,'exit_code':run.returncode,'log':log.name,'log_sha256':sha(log),'binary_sha256_before':digest,'binary_sha256_after':sha(binary)});record['unexecuted_names']=names[index:];save();print(index,name,'EXIT',run.returncode,flush=True)
  if run.returncode:
   record.setdefault('first_failure',name);save();print((run.stdout+run.stderr).decode(errors='replace')[:7000],flush=True)
 record['status']='All three exact Runtime controls executed once from the same current-source binary; inspect individual outcomes.'
else:record['status']='First selected Runtime build failed or frozen source changed; zero controls executed.'
after=snapshot();save_snapshot('after',after);record['source_comparison']={'changed':[p for p in sorted(before.keys()|after.keys()) if before.get(p)!=after.get(p)],'actual_owner_sha256':{p:sha(ROOT/p) for p in owners}};save();shutil.copyfile('/tmp/name-retired-runtime-validation.py',OUT/'capture.py');print('FINAL source changes',record['source_comparison']['changed'],'checks',len(record['checks']),flush=True)
