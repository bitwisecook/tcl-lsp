from pathlib import Path
import hashlib,json,gzip,subprocess,datetime,time,re
repo=Path('/workspace/tcl-lsp');out=Path('/tmp/mcp-static-name-inventory-validation-01');out.mkdir(exist_ok=False)
prior=repo/'experiments/name-handling/results/mcp-exact-command-name/validation-02'
metadata=json.loads((prior/'baseline.json').read_text());old=json.loads(gzip.decompress((prior/'after-sources.json.gz').read_bytes()))
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest() if p.is_file() else None
def save(name,d): (out/name).write_text(json.dumps(d,indent=2)+'\n')
def capture():
 paths={'.cargo/config.toml','Cargo.lock','Cargo.toml','rust-toolchain.toml'}
 for directory in metadata['package_dirs'].values():
  paths.update(str(p.relative_to(repo)) for p in (repo/directory).rglob('*') if p.is_file() and (p.suffix in {'.rs','.c','.h'} or p.name=='Cargo.toml') and 'target' not in p.parts)
 return {p:digest(repo/p) for p in sorted(paths)}
before=capture();save('before-sources.json',before)
base={'conservative_packages':39,'package_dirs':metadata['package_dirs'],'frozen_file_count':len(before),'source_scope':'Conservative local manifest path closure of selected MCP package (normal/root-dev/build, all target and optional entries), Rust/C/header/Cargo.toml files plus root config/lock/toolchain. Not every frozen file claimed compiled. Standalone Runtime excluded.','changes_since_validation_02':[{'path':p,'validation_02':old.get(p),'current':before.get(p)} for p in sorted(old.keys()|before.keys()) if old.get(p)!=before.get(p)],'irule_gen_sha256':before['rust/tcl-mcp/src/irule_gen.rs'],'tools_sha256':before['rust/tcl-mcp/src/tools.rs'],'original_fixtures_and_assertions_byte_unchanged':True,'test_helper_transport_changed':True}
save('baseline.json',base)
def run(label,argv):
 start=datetime.datetime.now(datetime.timezone.utc).isoformat();t=time.monotonic()
 with (out/(label+'.stdout')).open('wb') as stdout,(out/(label+'.stderr')).open('wb') as stderr:p=subprocess.run(argv,cwd=repo,stdout=stdout,stderr=stderr)
 info={'argv':argv,'start_utc':start,'end_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'elapsed_seconds':time.monotonic()-t,'exit_code':p.returncode};save(label+'.invocation.json',info);return info
build=run('offline-build',['cargo','test','--offline','-j2','-p','tcl-mcp','--bin','tcl-mcp','--no-run','--message-format=json']);print('BUILD',json.dumps(build),flush=True)
records=[]
for line in (out/'offline-build.stdout').read_text().splitlines():
 try:record=json.loads(line)
 except json.JSONDecodeError:continue
 records.append(record)
artifacts=[r for r in records if r.get('reason')=='compiler-artifact'];save('artifacts.json',artifacts)
diagnostics=[r for r in records if r.get('reason')=='compiler-message'];save('diagnostics.json',diagnostics)
workspace=[]
for a in artifacts:
 src=Path(a['target']['src_path']);manifest=Path(a['manifest_path'])
 if not str(manifest).startswith(str(repo)+'/'):continue
 workspace.append({'package_id':a['package_id'],'manifest_path':str(manifest),'manifest_sha256':before.get(str(manifest.relative_to(repo))),'target':a['target'],'target_source_sha256':before.get(str(src.relative_to(repo))) if str(src).startswith(str(repo)+'/') else None,'features':a['features'],'fresh':a['fresh']})
save('observed-workspace-artifacts.json',workspace)
selected=[a for a in artifacts if a['target']['name']=='tcl-mcp' and a.get('executable') and a['profile']['test']]
if build['exit_code'] or len(selected)!=1:
 after=capture();save('after-sources.json',after);save('validation.json',{'build':build,'test_execution_count':0,'source_changes':[p for p in sorted(before.keys()|after.keys()) if before.get(p)!=after.get(p)],'oracle_execution':False,'selected_binary_count':len(selected),'cache_released':True});print((out/'offline-build.stderr').read_text(),flush=True);raise SystemExit(build['exit_code'] or 2)
binary=Path(selected[0]['executable']);binary_hash=digest(binary);save('binary.json',{'target':selected[0],'sha256':binary_hash,'size':binary.stat().st_size});print('BINARY',binary_hash,flush=True)
if capture()!=before:raise RuntimeError('source changed during build')
tests=[('inventory','irule_gen::tests::executable_inventory_ignores_comments_and_data_but_keeps_live_substitutions'),('helper','irule_gen::tests::called_helper_static_write_preserves_hot_event_for_multi_tmm'),('single-capture','irule_gen::tests::generator_builds_one_request_closure_for_every_execution_sensitive_output')];results=[]
for label,test in tests:
 if capture()!=before or digest(binary)!=binary_hash:raise RuntimeError('source/binary changed before '+test)
 info=run(label,[str(binary),test,'--exact','--nocapture']);raw=(out/(label+'.stdout')).read_text();match=re.search(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out',raw)
 info['test_result']=dict(zip(['status','passed','failed','ignored','measured','filtered_out'],[match[1],*map(int,match.groups()[1:])])) if match else None
 info['binary_sha256_after']=digest(binary);info['source_unchanged_after']=capture()==before;save(label+'.invocation.json',info);results.append(info);print(label,json.dumps(info),flush=True);print(raw,flush=True);print((out/(label+'.stderr')).read_text(),flush=True)
after=capture();save('after-sources.json',after)
validation={'build':build,'tests':results,'test_execution_count':sum(r.get('test_result',{}).get('passed',0)+r.get('test_result',{}).get('failed',0) for r in results),'passed':sum(r.get('test_result',{}).get('passed',0) for r in results),'failed':sum(r.get('test_result',{}).get('failed',0) for r in results),'source_changes':[p for p in sorted(before.keys()|after.keys()) if before.get(p)!=after.get(p)],'binary_sha256_before':binary_hash,'binary_sha256_after':digest(binary),'conservative_packages':39,'frozen_files':len(before),'observed_workspace_packages':len(set(a['package_id'] for a in workspace)),'cargo_artifact_records':len(artifacts),'oracle_execution':False,'original_fixture_and_assertions_byte_unchanged':True,'source_or_test_updates_during_validation':False,'literal_or_event_controls_repeated':False,'full_suite_run':False,'cache_released':True}
save('validation.json',validation);print('COMPLETE',json.dumps(validation),flush=True)
