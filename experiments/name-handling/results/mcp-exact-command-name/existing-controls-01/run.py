from pathlib import Path
import json,gzip,hashlib,subprocess,datetime,time,re
repo=Path('/workspace/tcl-lsp'); prior=repo/'experiments/name-handling/results/mcp-exact-command-name/validation-02'
out=Path('/tmp/mcp-existing-controls-resumed-02');out.mkdir(exist_ok=False)
base=json.loads(gzip.decompress((prior/'after-sources.json.gz').read_bytes())); metadata=json.loads((prior/'baseline.json').read_text())
binary=Path(json.loads((prior/'binary.json').read_text())['target']['executable']); expected=json.loads((prior/'binary.json').read_text())['sha256']
def digest(p): return hashlib.sha256(p.read_bytes()).hexdigest() if p.is_file() else None
def capture():
 paths=set(base)
 for directory in metadata['package_dirs'].values():
  paths.update(str(p.relative_to(repo)) for p in (repo/directory).rglob('*') if p.is_file() and (p.suffix in {'.rs','.c','.h'} or p.name=='Cargo.toml') and 'target' not in p.parts)
 return {f:digest(repo/f) for f in sorted(paths)}
def save(name,data): (out/name).write_text(json.dumps(data,indent=2)+'\n')
before=capture();save('before-sources.json',before)
drift=[{'path':f,'validation_02':base.get(f),'current':before.get(f)} for f in sorted(base.keys()|before.keys()) if base.get(f)!=before.get(f)]
save('currency.json',{'validation_02_files':len(base),'current_files':len(before),'drift':drift,'binary_sha256':digest(binary),'expected_binary_sha256':expected,'package_count':len(metadata['package_dirs']),'standalone_runtime_excluded':True})
if drift or digest(binary)!=expected: print('STOP source/binary drift',json.dumps(drift)); raise SystemExit(2)
tests=[('helper-static-write','irule_gen::tests::called_helper_static_write_preserves_hot_event_for_multi_tmm'),('executable-inventory','irule_gen::tests::executable_inventory_ignores_comments_and_data_but_keeps_live_substitutions'),('original-event-reports','tools::original_event_source_tests::original_mcp_event_reports_keep_requested_analysis_and_ignore_reporting_maps')]
results=[]
for key,test in tests:
 if digest(binary)!=expected or capture()!=before: raise RuntimeError('source/binary changed before '+test)
 argv=[str(binary),test,'--exact','--nocapture'];start=datetime.datetime.now(datetime.timezone.utc).isoformat();t=time.monotonic()
 with (out/(key+'.stdout')).open('wb') as stdout,(out/(key+'.stderr')).open('wb') as stderr:
  p=subprocess.run(argv,cwd=repo,stdout=stdout,stderr=stderr)
 info={'argv':argv,'start_utc':start,'end_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'elapsed_seconds':time.monotonic()-t,'exit_code':p.returncode,'binary_sha256':digest(binary),'source_unchanged_after_test':capture()==before}
 raw=(out/(key+'.stdout')).read_text();m=re.search(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out',raw)
 info['test_result']=dict(zip(['status','passed','failed','ignored','measured','filtered_out'],[m[1],*map(int,m.groups()[1:])])) if m else None
 save(key+'.invocation.json',info);results.append(info);print(json.dumps(info),flush=True)
 print(raw,flush=True);print((out/(key+'.stderr')).read_text(),flush=True)
after=capture();save('after-sources.json',after)
save('validation.json',{'binary_sha256_before':expected,'binary_sha256_after':digest(binary),'source_changes_during_capture':[f for f in sorted(before.keys()|after.keys()) if before.get(f)!=after.get(f)],'validation_02_source_drift':drift,'conservative_packages':39,'frozen_files':len(before),'tests':results,'oracle_execution':False,'no_build':True,'literal_control_repeated':False,'test_sources_or_assertions_changed':False,'cache_released':True})
