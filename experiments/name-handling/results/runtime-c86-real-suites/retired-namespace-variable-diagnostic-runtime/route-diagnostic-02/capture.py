import gzip,hashlib,json,os,pathlib,resource,shutil,signal,subprocess,tempfile,time,re
ROOT=pathlib.Path('/workspace/tcl-lsp');OUT=ROOT/'experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-runtime/route-diagnostic-02';OUT.mkdir(exist_ok=False)
PROP=OUT.parent/'proposed-passive-failure-route';PREV=OUT.parent/'orchestration-03';sha=lambda b:hashlib.sha256(b).hexdigest();file_sha=lambda p:sha(p.read_bytes());ready=json.loads((PROP/'source-ready.json').read_text());assert file_sha(PROP/'source.patch')==ready['patch_sha256'];originals={rel:(ROOT/rel).read_bytes() for rel in ready['sources']};temp={rel:pathlib.Path(item['scratch_path']).read_bytes() for rel,item in ready['sources'].items()}
for rel,item in ready['sources'].items():assert sha(originals[rel])==item['preimage_sha256'];assert sha(temp[rel])==item['scratch_sha256']
inputs=json.loads((PREV/'orchestration-inputs.json').read_text());assert all(file_sha(pathlib.Path(p))==h for p,h in inputs.items());(OUT/'orchestration-inputs.json').write_text(json.dumps(inputs,indent=2)+'\n');(OUT/'temp-source-ready.json').write_text(json.dumps(ready,indent=2)+'\n');shutil.copyfile(PROP/'source.patch',OUT/'temp-source.patch')
with gzip.open(PREV/'production-source-before.json.gz','rt') as f:paths=set(json.load(f))
def snapshot(label):
 current={p:file_sha(ROOT/p) for p in sorted(paths)}
 with gzip.open(OUT/f'production-source-{label}.json.gz','wt') as f:json.dump(current,f,indent=2)
 return current
before=snapshot('before');scope=json.loads((PREV/'scope.json').read_text());scope.update(count=len(before),scope='One TEMP selected production example; Runtime14 actual metadata reused from original current lib build and orchestration03. Conservative local sources plus unchanged original C86 library/tests/libtommath. Compiler/IRules/VM/LSP/MCP excluded. Temporary exact2 owners are inventoried separately; not every retained input is compiled.');(OUT/'scope.json').write_text(json.dumps(scope,indent=2)+'\n');shutil.copyfile(PREV/'runtime-cargo-metadata.json.gz',OUT/'runtime-cargo-metadata.json.gz')
for rel in originals:
 for label,data in [('production-before',originals[rel]),('TEMP',temp[rel])]:(OUT/(rel.replace('/','--')+'.'+label+'.gz')).write_bytes(gzip.compress(data,mtime=0))
env=dict(os.environ);env['CARGO_TARGET_DIR']=str(ROOT/'tmp/name-target-runtime');cargo=ROOT/'tmp/name-tools/cargo/bin/cargo';buildargv=[str(cargo),'build','--offline','-j2','--manifest-path','runtime/rust/Cargo.toml','--example','run_script','--message-format=json'];record={'status':'prepared','scope':'One original var17 passive route capture only. Prior ordinary/vector/dynamic controls not rerun; their causes not claimed measured. No production repair. TEMP logging cap64 state observations plus bounded typed fields; later events may be unreached/cap-suppressed.','build':{'argv':buildargv,'initial_free_bytes':shutil.disk_usage(env['CARGO_TARGET_DIR']).free,'minimum_free_bytes':shutil.disk_usage(env['CARGO_TARGET_DIR']).free,'disk_abort_threshold_bytes':192*1024**2},'original_inputs_before':inputs,'source_preimages':{p:sha(b) for p,b in originals.items()},'temp_sources':{p:sha(b) for p,b in temp.items()}}
def save():(OUT/'record.json').write_text(json.dumps(record,indent=2)+'\n')
save()
try:
 for rel,data in temp.items():(ROOT/rel).write_bytes(data)
 during=snapshot('TEMP-before-build');changed={p for p in paths if before[p]!=during[p]};assert changed==set(temp),changed;record['status']='TEMP applied, building once';save();print('TEMP BUILD START',len(before),'source inputs',flush=True)
 with (OUT/'build-messages.jsonl').open('wb') as stdout,(OUT/'build.stderr.txt').open('wb') as stderr:
  proc=subprocess.Popen(buildargv,cwd=ROOT,env=env,stdout=stdout,stderr=stderr,start_new_session=True)
  while proc.poll() is None:
   free=shutil.disk_usage(env['CARGO_TARGET_DIR']).free;record['build']['minimum_free_bytes']=min(record['build']['minimum_free_bytes'],free)
   if free<192*1024**2:record['build']['disk_abort_triggered']=True;os.killpg(proc.pid,signal.SIGTERM);break
   time.sleep(1)
  code=proc.wait()
 record['build'].update(exit_code=code,messages_sha256=file_sha(OUT/'build-messages.jsonl'),stderr_sha256=file_sha(OUT/'build.stderr.txt'));save();print('BUILD EXIT',code,flush=True)
 afterbuild=snapshot('TEMP-after-build');record['changed_source_during_build']=[p for p in sorted(paths) if during[p]!=afterbuild[p]]
 if code==0 and not record['changed_source_during_build']:
  messages=[json.loads(l) for l in (OUT/'build-messages.jsonl').read_text().splitlines() if l.startswith('{')];arts=[a for a in messages if a.get('reason')=='compiler-artifact' and a['target']['name']=='run_script' and a.get('executable')];assert len(arts)==1;a=arts[0];runner=pathlib.Path(a['executable']);assert runner==ROOT/'tmp/name-target-runtime/debug/examples/run_script';record['build'].update(binary=str(runner),TEMP_binary_sha256=file_sha(runner),artifact_fresh=a.get('fresh'),cargo_profile=a['profile']);save()
  runenv=dict(os.environb)
  for key in [b'NH_CLOCK_GETENV_MISSING',b'TCL_BACKEND_CONSTRAINTS',b'TCLLIBPATH',b'TCL_TEST_VERBOSE',b'TCL_TEST_CONSTRAINTS']:runenv.pop(key,None)
  runenv.update({b'TCL_LIBRARY':str(ROOT/'tmp/tcl8.6.18/library').encode(),b'NH_CLOCK_GETENV_TEST':b'host-value',b'NH_CLOCK_GETENV_\xff':b'raw-name-value',b'NH_CLOCK_GETENV_RAW_VALUE':b'\xff'})
  wrapper=ROOT/'experiments/name-handling/results/split-native-implementation-c86/capture-01/all-tcl-var.tcl';runargv=['timeout','--kill-after=5','60',str(runner),'--quiet','--tcl-version','8.6','--init',str(wrapper)]
  def limits():resource.setrlimit(resource.RLIMIT_AS,(1024**3,1024**3));resource.setrlimit(resource.RLIMIT_CORE,(0,0))
  start=time.monotonic();record['status']='Executing original wrapper once';save();print('ORIGINAL WRAPPER START',flush=True)
  with tempfile.TemporaryDirectory(prefix='split-c86-current-') as cwd,(OUT/'stdout.txt').open('wb') as stdout,(OUT/'stderr.txt').open('wb') as stderr:run=subprocess.run(runargv,cwd=cwd,env=runenv,stdout=stdout,stderr=stderr,preexec_fn=limits,timeout=67)
  out=(OUT/'stdout.txt').read_text(errors='replace');err=(OUT/'stderr.txt').read_text(errors='replace');lines=out.splitlines();starts=re.findall(r'^---- (\S+) start',out,re.M);counts=[l for l in lines if re.search(r'Total\s+\d+.*Passed\s+\d+',l)]
  record.update(status='Original wrapper process completed once',argv=runargv,exit_code=run.returncode,seconds=round(time.monotonic()-start,3),limits={'address_space_bytes':1024**3,'timeout_seconds':60,'kill_after_seconds':5,'core_bytes':0},environment='Same prior named synthetic NH_CLOCK_GETENV controls and TCL_LIBRARY, same five removed test/overlay selectors. Inherited host environment not dumped. No TZ override or environment_sync.',TEMP_runner_sha256_after=file_sha(runner),stdout_sha256=file_sha(OUT/'stdout.txt'),stderr_sha256=file_sha(OUT/'stderr.txt'),actual_progress={'init_marker':'NH_INIT' in lines,'tcltest_marker':'NH_HARNESS' in lines,'all_tcl_enter_marker':'NH_ORCHESTRATION_ENTER' in lines,'all_tcl_return_marker':'NH_ORCHESTRATION_RETURN' in lines,'test_starts':starts,'last_reached_test_start':starts[-1] if starts else None,'final_suite_count_lines':counts},passive_markers=[l for l in err.splitlines() if l.startswith('NATIVE_VARIABLE_ROUTE ')],first_refusal=[l for l in err.splitlines() if l.startswith('run_script:')]);save();print('WRAPPER EXIT',run.returncode,'starts',len(starts),'last',starts[-1] if starts else None,flush=True);print(err[:6500],flush=True)
  dep=runner.with_suffix('.d')
  if dep.is_file():(OUT/'TEMP-runner-rustc-depinfo.d.gz').write_bytes(gzip.compress(dep.read_bytes(),mtime=0))
 else:record['status']='Selected TEMP build failed/source changed; original wrapper not executed'
except BaseException as error:
 record['capture_exception_type']=type(error).__name__;record['status']='Capture raised; byteexact restoration in finally';save();raise
finally:
 before_restore=snapshot('TEMP-before-restore')
 for rel,data in originals.items():(ROOT/rel).write_bytes(data)
 restored=snapshot('restored');record['source_restore']={'restored_owner_sha256':{p:file_sha(ROOT/p) for p in originals},'owners_byteexact':all((ROOT/p).read_bytes()==data for p,data in originals.items()),'changed_from_production_before':[p for p in sorted(paths) if before[p]!=restored[p]],'unexpected_changes_while_TEMP':[p for p in sorted(paths) if p not in temp and before[p]!=before_restore[p]]};record['original_inputs_after']={p:file_sha(pathlib.Path(p)) for p in inputs};record['TEMP_binary_currentness']='NONCURRENT after byte-exact production source restoration; do not reuse as production validation.';save();shutil.copyfile('/tmp/name-detached-runtime-route-diagnostic.py',OUT/'capture.py')
 if (OUT/'build-messages.jsonl').is_file():(OUT/'build-messages.jsonl.gz').write_bytes(gzip.compress((OUT/'build-messages.jsonl').read_bytes(),mtime=0))
 print('RESTORED',record['source_restore'],flush=True)
