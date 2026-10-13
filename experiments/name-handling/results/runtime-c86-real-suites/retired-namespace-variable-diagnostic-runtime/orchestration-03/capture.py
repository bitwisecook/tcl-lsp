import gzip,hashlib,json,os,pathlib,resource,shutil,signal,subprocess,tempfile,time,re
ROOT=pathlib.Path('/workspace/tcl-lsp');OUT=ROOT/'experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-runtime/orchestration-03';OUT.mkdir(exist_ok=False)
PRIOR=OUT.parent/'validation-01';ORCH=ROOT/'experiments/name-handling/results/runtime-c86-real-suites/glob-type-union-runtime/orchestration-02';sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
inputs=json.loads((ORCH/'orchestration-inputs.json').read_text());assert all(sha(pathlib.Path(p))==h for p,h in inputs.items());(OUT/'orchestration-inputs.json').write_text(json.dumps(inputs,indent=2)+'\n')
with gzip.open(PRIOR/'production-source-before.json.gz','rt') as f:paths=set(json.load(f))
for directory in [ROOT/'tmp/tcl9.0.4/libtommath',ROOT/'tmp/tcl8.6.18/library',ROOT/'tmp/tcl8.6.18/tests']:
 paths.update(str(p.relative_to(ROOT)) for p in directory.rglob('*') if p.is_file() and p.suffix in ('.c','.h','.tcl','.test'))
paths.update(str(pathlib.Path(p).relative_to(ROOT)) for p in inputs)
validation=json.loads((PRIOR/'validation.json').read_text());owners=validation['source_comparison']['actual_owner_sha256'];assert all(sha(ROOT/p)==h for p,h in owners.items())
def snapshot(label):
 current={p:sha(ROOT/p) for p in sorted(paths)}
 with gzip.open(OUT/f'production-source-{label}.json.gz','wt') as f:json.dump(current,f,indent=2)
 return current
before=snapshot('before');scope=json.loads((PRIOR/'scope.json').read_text());scope.update(count=len(before),metadata_reference=str((PRIOR/'runtime-cargo-metadata.json.gz').relative_to(ROOT)),scope='One production run_script example, actual latest Runtime14 metadata closure reused. Conservative local-source inventory adds original C86 library/tests and default libtommath sources/headers. Not every retained input is compiled. No Compiler/IRules/VM/LSP/MCP inclusion or all-workspace freshness claim.');(OUT/'scope.json').write_text(json.dumps(scope,indent=2)+'\n');shutil.copyfile(PRIOR/'runtime-cargo-metadata.json.gz',OUT/'runtime-cargo-metadata.json.gz')
for p in list(owners)+['runtime/rust/examples/run_script.rs']:(OUT/(p.replace('/','--')+'.gz')).write_bytes(gzip.compress((ROOT/p).read_bytes(),mtime=0))
env=dict(os.environ);env['CARGO_TARGET_DIR']=str(ROOT/'tmp/name-target-runtime');cargo=ROOT/'tmp/name-tools/cargo/bin/cargo';argv=[str(cargo),'build','--offline','-j2','--manifest-path','runtime/rust/Cargo.toml','--example','run_script','--message-format=json'];record={'status':'running build','acceptance_scope':'Independent original operational orchestration study; prior Runtime3FAIL and Syntax2PASS remain separate. No acceptance/full-suite claim.','build':{'argv':argv,'initial_free_bytes':shutil.disk_usage(env['CARGO_TARGET_DIR']).free,'minimum_free_bytes':shutil.disk_usage(env['CARGO_TARGET_DIR']).free,'disk_abort_threshold_bytes':192*1024**2},'original_inputs_before':inputs}
def save():(OUT/'record.json').write_text(json.dumps(record,indent=2)+'\n')
save();print('BUILD START',len(before),'source inputs',flush=True)
with (OUT/'build-messages.jsonl').open('wb') as stdout,(OUT/'build.stderr.txt').open('wb') as stderr:
 proc=subprocess.Popen(argv,cwd=ROOT,env=env,stdout=stdout,stderr=stderr,start_new_session=True)
 while proc.poll() is None:
  free=shutil.disk_usage(env['CARGO_TARGET_DIR']).free;record['build']['minimum_free_bytes']=min(record['build']['minimum_free_bytes'],free)
  if free<192*1024**2:record['build']['disk_abort_triggered']=True;os.killpg(proc.pid,signal.SIGTERM);break
  time.sleep(1)
 code=proc.wait()
record['build'].update(exit_code=code,messages_sha256=sha(OUT/'build-messages.jsonl'),stderr_sha256=sha(OUT/'build.stderr.txt'));save();print('BUILD EXIT',code,flush=True)
afterbuild=snapshot('after-build');changed=[p for p in paths if before[p]!=afterbuild[p]];record['changed_source_during_build']=changed
if code==0 and not changed:
 items=[json.loads(l) for l in (OUT/'build-messages.jsonl').read_text().splitlines() if l.startswith('{')];arts=[a for a in items if a.get('reason')=='compiler-artifact' and a['target']['name']=='run_script' and a.get('executable')];assert len(arts)==1;a=arts[0];runner=pathlib.Path(a['executable']);assert runner==ROOT/'tmp/name-target-runtime/debug/examples/run_script';record['build'].update(binary=str(runner),binary_sha256=sha(runner),artifact_fresh=a.get('fresh'),cargo_profile=a['profile']);save()
 runenv=dict(os.environb)
 for key in [b'NH_CLOCK_GETENV_MISSING',b'TCL_BACKEND_CONSTRAINTS',b'TCLLIBPATH',b'TCL_TEST_VERBOSE',b'TCL_TEST_CONSTRAINTS']:runenv.pop(key,None)
 runenv.update({b'TCL_LIBRARY':str(ROOT/'tmp/tcl8.6.18/library').encode(),b'NH_CLOCK_GETENV_TEST':b'host-value',b'NH_CLOCK_GETENV_\xff':b'raw-name-value',b'NH_CLOCK_GETENV_RAW_VALUE':b'\xff'})
 wrapper=ROOT/'experiments/name-handling/results/split-native-implementation-c86/capture-01/all-tcl-var.tcl';runargv=['timeout','--kill-after=5','60',str(runner),'--quiet','--tcl-version','8.6','--init',str(wrapper)]
 def limits():resource.setrlimit(resource.RLIMIT_AS,(1024**3,1024**3));resource.setrlimit(resource.RLIMIT_CORE,(0,0))
 start=time.monotonic();print('ORIGINAL WRAPPER START',flush=True)
 with tempfile.TemporaryDirectory(prefix='split-c86-current-') as cwd,(OUT/'stdout.txt').open('wb') as stdout,(OUT/'stderr.txt').open('wb') as stderr:run=subprocess.run(runargv,cwd=cwd,env=runenv,stdout=stdout,stderr=stderr,preexec_fn=limits,timeout=67)
 out=(OUT/'stdout.txt').read_text(errors='replace');err=(OUT/'stderr.txt').read_text(errors='replace');starts=re.findall(r'^---- (\S+) start',out,re.M);counts=[l for l in out.splitlines() if re.search(r'Total\s+\d+.*Passed\s+\d+',l)]
 record.update(status='original wrapper completed process once',argv=runargv,exit_code=run.returncode,seconds=round(time.monotonic()-start,3),limits={'address_space_bytes':1024**3,'timeout_seconds':60,'kill_after_seconds':5,'core_bytes':0},environment='Exact prior named synthetic NH_CLOCK_GETENV controls and TCL_LIBRARY. Same five removed test/overlay selectors. Inherited host data not dumped; no TZ override/environment_sync.',runner_sha256_after=sha(runner),stdout_sha256=sha(OUT/'stdout.txt'),stderr_sha256=sha(OUT/'stderr.txt'),actual_progress={'init_marker':'NH_INIT 8.6 OK' in out,'tcltest_marker':'NH_HARNESS ACTUAL_TCLTEST_LOADED' in out,'all_tcl_enter_marker':'NH_ORCHESTRATION_ENTER all.tcl selected=var.test' in out,'all_tcl_return_marker':'NH_ORCHESTRATION_RETURN' in out,'actual_selected_file':'var.test' if 'var.test' in out else None,'test_starts':starts,'last_reached_test_start':starts[-1] if starts else None,'final_suite_count_lines':counts},first_stderr=err)
 print('WRAPPER EXIT',run.returncode,'seconds',record['seconds'],'test starts',len(starts),'last',starts[-1] if starts else None,flush=True);print(err[:1800],flush=True)
 dep=runner.with_suffix('.d')
 if dep.is_file():(OUT/'runner-rustc-depinfo.d.gz').write_bytes(gzip.compress(dep.read_bytes(),mtime=0))
else:record['status']='selected build failed/source changed; original wrapper not executed'
after=snapshot('after');record['source_comparison']={'changed':[p for p in sorted(paths) if before[p]!=after[p]],'actual_owner_sha256':{p:sha(ROOT/p) for p in owners}};record['original_inputs_after']={p:sha(pathlib.Path(p)) for p in inputs};save();shutil.copyfile('/tmp/name-retired-orchestration03.py',OUT/'capture.py');(OUT/'build-messages.jsonl.gz').write_bytes(gzip.compress((OUT/'build-messages.jsonl').read_bytes(),mtime=0));print('FINAL source changes',record['source_comparison']['changed'],flush=True)
