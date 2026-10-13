import gzip,hashlib,json,os,pathlib,resource,shutil,subprocess,tempfile,time
ROOT=pathlib.Path('/workspace/tcl-lsp');SCRATCH=pathlib.Path('/tmp/retired-namespace-route-scratch');OUT=ROOT/'experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-runtime/route-diagnostic-01';OUT.mkdir(parents=True,exist_ok=False);sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();ready=json.loads((SCRATCH/'scratch-ready.json').read_text());owners=ready['sources'];record={'status':'prepared','scratch_ready':ready,'original_orchestration_executed':False};(OUT/'record.json').write_text(json.dumps(record,indent=2)+'\n')
for rel,item in owners.items():assert sha(ROOT/rel)==item['preimage_sha256'];assert sha(SCRATCH/rel)==item['scratch_sha256']
inputs=json.loads((ROOT/'experiments/name-handling/results/runtime-c86-real-suites/glob-type-union-runtime/orchestration-02/orchestration-inputs.json').read_text());assert all(sha(pathlib.Path(p))==h for p,h in inputs.items());(OUT/'original-inputs.json').write_text(json.dumps(inputs,indent=2)+'\n')
env=dict(os.environ);env['CARGO_HOME']=str(ROOT/'tmp/name-tools/cargo');env['RUSTUP_HOME']=str(ROOT/'tmp/name-tools/rustup');env['CARGO_TARGET_DIR']=str(ROOT/'tmp/name-target-runtime');env['PATH']=str(ROOT/'tmp/name-tools/cargo/bin')+':'+str(ROOT/'tmp/name-tools/bin')+':'+env.get('PATH','');cargo=ROOT/'tmp/name-tools/cargo/bin/cargo';record['initial_disk_free_bytes']=shutil.disk_usage(env['CARGO_TARGET_DIR']).free
shutil.copyfile(SCRATCH/'source.patch',OUT/'temporary-source.patch');local=[];before=None
def save(): (OUT/'record.json').write_text(json.dumps(record,indent=2)+'\n')
def snapshot():
 selected={}
 for pkg in local:
  directory=pathlib.Path(pkg['manifest_path']).parent
  for p in directory.rglob('*'):
   if p.is_file() and (p.suffix in ('.rs','.c','.h','.tcl','.txt','.json') or p.name in ('Cargo.toml','Cargo.lock')) and not any(part in ('target','.git') for part in p.relative_to(directory).parts):selected[str(p.relative_to(ROOT))]=sha(p)
 for rel in ['Cargo.toml','Cargo.lock','runtime/rust/Cargo.lock','.cargo/config.toml','rust-toolchain.toml']:
  if (ROOT/rel).is_file():selected[rel]=sha(ROOT/rel)
 for directory in [ROOT/'tmp/tcl9.0.4/libtommath',ROOT/'tmp/tcl8.6.18/library',ROOT/'tmp/tcl8.6.18/tests']:
  for p in directory.rglob('*'):
   if p.is_file() and p.suffix in ('.c','.h','.tcl','.test'):selected[str(p.relative_to(ROOT))]=sha(p)
 for path in inputs:selected[str(pathlib.Path(path).relative_to(ROOT))]=sha(pathlib.Path(path))
 return dict(sorted(selected.items()))
def write_snapshot(label,data):
 with gzip.open(OUT/f'source-{label}.json.gz','wt') as f:json.dump(data,f,indent=2)
try:
 for rel in owners:
  with gzip.open(OUT/(rel.replace('/','--')+'-production.gz'),'wb') as f:f.write((ROOT/rel).read_bytes())
  (ROOT/rel).write_bytes((SCRATCH/rel).read_bytes())
  with gzip.open(OUT/(rel.replace('/','--')+'-diagnostic.gz'),'wb') as f:f.write((ROOT/rel).read_bytes())
 metadata_argv=[str(cargo),'metadata','--offline','--manifest-path','runtime/rust/Cargo.toml','--format-version=1'];mr=subprocess.run(metadata_argv,cwd=ROOT,env=env,capture_output=True);(OUT/'metadata.stderr.txt').write_bytes(mr.stderr);(OUT/'runtime-cargo-metadata.json.gz').write_bytes(gzip.compress(mr.stdout,mtime=0));record['metadata']={'argv':metadata_argv,'exit_code':mr.returncode,'sha256':hashlib.sha256(mr.stdout).hexdigest()};save();assert mr.returncode==0
 m=json.loads(mr.stdout);pkgs={p['id']:p for p in m['packages']};nodes={p['id']:p for p in m['resolve']['nodes']};rid=m['resolve']['root'];todo=[rid];seen=set()
 while todo:
  pid=todo.pop()
  if pid in seen:continue
  seen.add(pid);todo.extend(d['pkg'] for d in nodes[pid]['deps'] if pid==rid or any(k['kind'] in (None,'build') for k in d['dep_kinds']))
 local=[pkgs[p] for p in seen if pathlib.Path(pkgs[p]['manifest_path']).is_relative_to(ROOT) and not pathlib.Path(pkgs[p]['manifest_path']).is_relative_to(ROOT/'tmp')];assert len(local)==14
 before=snapshot();write_snapshot('diagnostic-before-build',before);production=dict(before)
 for rel,item in owners.items():production[rel]=item['preimage_sha256']
 write_snapshot('production-before',production);record['source_scope']={'packages':sorted(p['name'] for p in local),'count':len(before),'scope':'Actual standalone production example local Cargo dependency closure, root all dependency kinds and nonroot normal/build, conservatively local rs/c/h/tcl/txt/json/manifests plus unchanged actual provider/header/input hashes. Compiler/VM/LSP/MCP/IRules/xtask excluded; not all-workspace or every-file-compiled evidence.'};save()
 argv=[str(cargo),'build','--offline','-j2','--manifest-path','runtime/rust/Cargo.toml','--example','run_script','--message-format=json'];record['build']={'argv':argv,'status':'running'};save();print('BUILD START diagnostic Runtime14',flush=True)
 with (OUT/'build-messages.jsonl').open('wb') as stdout,(OUT/'build.stderr.txt').open('wb') as stderr:build=subprocess.run(argv,cwd=ROOT,env=env,stdout=stdout,stderr=stderr)
 record['build'].update(status='completed',exit_code=build.returncode,messages_sha256=sha(OUT/'build-messages.jsonl'),stderr_sha256=sha(OUT/'build.stderr.txt'));save();print('BUILD EXIT',build.returncode,flush=True)
 after=snapshot();write_snapshot('diagnostic-after-build',after);record['source_changed_during_build']=[p for p in sorted(before.keys()|after.keys()) if before.get(p)!=after.get(p)];save()
 if build.returncode==0 and not record['source_changed_during_build']:
  messages=[json.loads(l) for l in (OUT/'build-messages.jsonl').read_text().splitlines() if l.startswith('{')];arts=[a for a in messages if a.get('reason')=='compiler-artifact' and a['target']['name']=='run_script' and a['executable']];assert len(arts)==1;runner=pathlib.Path(arts[0]['executable']);assert runner==ROOT/'tmp/name-target-runtime/debug/examples/run_script';record['build'].update(binary=str(runner),binary_sha256=sha(runner),artifact_fresh=arts[0].get('fresh'))
  tenv=dict(os.environb)
  for key in [b'NH_CLOCK_GETENV_MISSING',b'TCL_BACKEND_CONSTRAINTS',b'TCLLIBPATH',b'TCL_TEST_VERBOSE',b'TCL_TEST_CONSTRAINTS']:tenv.pop(key,None)
  tree=ROOT/'tmp/tcl8.6.18';tenv.update({b'TCL_LIBRARY':str(tree/'library').encode(),b'NH_CLOCK_GETENV_TEST':b'host-value',b'NH_CLOCK_GETENV_\xff':b'raw-name-value',b'NH_CLOCK_GETENV_RAW_VALUE':b'\xff'});wrapper=ROOT/'experiments/name-handling/results/split-native-implementation-c86/capture-01/all-tcl-var.tcl'
  run_argv=['timeout','--kill-after=5','60',str(runner),'--quiet','--tcl-version','8.6','--init',str(wrapper)]
  def limits():resource.setrlimit(resource.RLIMIT_AS,(1024**3,1024**3));resource.setrlimit(resource.RLIMIT_CORE,(0,0))
  start=time.monotonic();print('ORIGINAL WRAPPER START',flush=True)
  with tempfile.TemporaryDirectory(prefix='split-c86-current-') as cwd,(OUT/'stdout.txt').open('wb') as stdout,(OUT/'stderr.txt').open('wb') as stderr:run=subprocess.run(run_argv,cwd=cwd,env=tenv,stdout=stdout,stderr=stderr,preexec_fn=limits,timeout=67)
  record['original_orchestration_executed']=True;record['orchestration']={'argv':run_argv,'exit_code':run.returncode,'seconds':round(time.monotonic()-start,3),'stdout_sha256':sha(OUT/'stdout.txt'),'stderr_sha256':sha(OUT/'stderr.txt'),'binary_sha256_after':sha(runner),'limits':{'address_space_bytes':1024**3,'core_bytes':0,'timeout_seconds':60},'environment':'Same prior named benign NH_CLOCK_GETENV synthetic controls/TCL_LIBRARY and overlay/test-selector removals; inherited host/environment data never logged or dumped.'};save();print('ORIGINAL WRAPPER EXIT',run.returncode,flush=True)
  print((OUT/'stderr.txt').read_text(errors='replace')[:4000],flush=True)
 else:record['unexecuted']='Original wrapper not executed because selected example build failed or frozen dependency source changed.'
except BaseException as error:
 record['driver_exception_type']=type(error).__name__;record['status']='Diagnostic driver interrupted/failed; mandatory exact source restoration runs.';save();raise
finally:
 for rel,item in owners.items():
  (ROOT/rel).write_bytes((SCRATCH/'preimages'/rel).read_bytes());assert sha(ROOT/rel)==item['preimage_sha256']
 record['restoration']={'actual_sha256':{rel:sha(ROOT/rel) for rel in owners},'byte_exact':True,'diagnostic_binary_current':False,'scope':'Only two passive observation owners restored; binary built against temporary source is not current production.'}
 if before is not None:
  restored=snapshot();write_snapshot('production-after-restore',restored);expected=dict(before)
  for rel,item in owners.items():expected[rel]=item['preimage_sha256']
  record['production_source_changed_after_restore']=[p for p in sorted(expected.keys()|restored.keys()) if expected.get(p)!=restored.get(p)]
 record['status']='Mandatory two-source restoration complete; diagnostic binary marked noncurrent.';save();shutil.copyfile('/tmp/name-retired-runtime-route-diagnostic.py',OUT/'capture.py');print('RESTORED BOTH OWNERS byte exact; cache lease releasable',flush=True)
