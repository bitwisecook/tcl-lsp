from pathlib import Path
import json,hashlib,subprocess,datetime,time,os,signal,shutil,re,ast,tomllib,glob
repo=Path('/workspace/tcl-lsp');out=Path('/tmp/mcp-generator-current-validation-01');out.mkdir(exist_ok=False);target=repo/'target';assert os.environ.get('CARGO_TARGET_DIR')==str(target);roots=['tcl-mcp']
w=tomllib.loads((repo/'Cargo.toml').read_text())['workspace'];manifests={}
for entry in w['members']:
 for directory in glob.glob(str(repo/entry)):
  p=Path(directory)/'Cargo.toml'
  if p.is_file():manifests[tomllib.loads(p.read_text())['package']['name']]=p.resolve()
rootpaths={manifests[n] for n in roots};queue=list(rootpaths);seen={};shared=w.get('dependencies',{})
while queue:
 p=queue.pop().resolve()
 if p in seen:continue
 data=tomllib.loads(p.read_text());seen[p]=data['package']['name']
 for table in [data,*data.get('target',{}).values()]:
  for kind in ['dependencies','build-dependencies']+(['dev-dependencies'] if p in rootpaths else []):
   for name,d in table.get(kind,{}).items():
    if not isinstance(d,dict):continue
    parent=p.parent
    if d.get('workspace'):
     base=shared.get(name,{});d={**(base if isinstance(base,dict) else {}),**d};parent=repo
    if 'path' in d:queue.append((parent/d['path']/'Cargo.toml').resolve())
closure={'selected_roots':roots,'package_dirs':{name:str(p.parent.relative_to(repo)) for p,name in sorted(seen.items())},'scope':'Actual normal/build plus selected-MCP-root dev local manifest dependency paths including all target/optional tables; conservative, not compiled graph.'};assert len(closure['package_dirs'])==39
module=ast.parse(Path('/tmp/mcp-joint-six-lib-validation-run.py').read_text());nodes=[n for n in module.body if isinstance(n,ast.FunctionDef) and n.name in {'digest','save','capture','run'}];exec(compile(ast.Module(body=nodes,type_ignores=[]),'<retained helpers>','exec'))
before=capture();save('before-sources.json',before);save('manifest-closure.json',closure);owners=['rust/tcl-registry/src/source_configuration_reference.rs','rust/tcl-irules/src/source_object.rs'];save('changed-owner-images.json',{p:(repo/p).read_text() for p in owners});prior=repo/'experiments/name-handling/results/mcp-static-name-inventory/validation-01';oldbin=json.loads((prior/'binary.json').read_text());import gzip;oldsrc=json.loads(gzip.decompress((prior/'before-sources.json.gz').read_bytes()));save('baseline.json',{'package_count':39,'frozen_files':len(before),'changed_owner_images':owners,'recent_owner_sha256':{p:before[p] for p in owners},'original_mcp_generator_fixture_assertions_unchanged':True,'prior_failed_generator_binary_sha256':oldbin['sha256'],'default_binary_sha256_before_build':digest(Path(oldbin['target']['executable'])),'changes_from_prior_mcp_in_common_scope':[{'path':p,'prior':oldsrc[p],'current':before[p]} for p in before if p in oldsrc and oldsrc[p]!=before[p]],'explicit_target':str(target),'target_kind':'bin','disk_free_before_bytes':shutil.disk_usage(repo).free,'no_lib_target':'tcl-mcp declares src/main.rs binary only; actual selected --bin used rather than impossible --lib.'})
print('FROZEN',json.dumps({'packages':39,'files':len(before),'explicit_default_target':str(target),'changed_owner_images':2}),flush=True)
build=run('offline-build',['cargo','test','--offline','-j2','-p','tcl-mcp','--bin','tcl-mcp','--no-run','--message-format=json'],space_guard=True);print('BUILD',json.dumps(build),flush=True);records=[]
for line in (out/'offline-build.stdout').read_text().splitlines():
 try:r=json.loads(line)
 except json.JSONDecodeError:continue
 records.append(r)
a=[r for r in records if r.get('reason')=='compiler-artifact'];save('artifacts.json',a);save('diagnostics.json',[r for r in records if r.get('reason')=='compiler-message']);ownedmanifests={str(repo/d/'Cargo.toml') for d in closure['package_dirs'].values()};owned=[]
for x in a:
 if x['manifest_path'] in ownedmanifests:owned.append({'package_id':x['package_id'],'target':x['target'],'target_source_sha256':before.get(str(Path(x['target']['src_path']).relative_to(repo))),'manifest_path':x['manifest_path'],'manifest_sha256':before.get(str(Path(x['manifest_path']).relative_to(repo))),'features':x['features'],'fresh':x['fresh']})
save('observed-owned-artifacts.json',owned);selected=[x for x in a if x['manifest_path']==str(manifests['tcl-mcp']) and x['target']['name']=='tcl-mcp' and x.get('executable') and x['profile']['test']];abort=None;result=None;b=None
if build['exit_code'] or build['abort']:abort='selected MCP build failed; zero test execution'
elif len(selected)!=1:abort='actual selected test binary not uniquely emitted'
elif capture()!=before:abort='source changed during build'
else:
 binary=Path(selected[0]['executable']);b={'artifact':selected[0],'executable':str(binary),'sha256':digest(binary),'size':binary.stat().st_size};save('binary.json',b);test='irule_gen::tests::generator_builds_one_request_closure_for_every_execution_sensitive_output';result=run('generator',[str(binary),test,'--exact','--nocapture'],timeout=60);raw=(out/'generator.stdout').read_text();m=re.search(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out',raw);result.update({'test':test,'test_result':dict(zip(['status','passed','failed','ignored','measured','filtered_out'],[m[1],*map(int,m.groups()[1:])])) if m else None,'source_unchanged_after':capture()==before,'binary_sha256_after':digest(binary)});save('generator.invocation.json',result);print('CONTROL',json.dumps(result),flush=True);print(raw,flush=True);print((out/'generator.stderr').read_text(),flush=True)
after=capture();save('after-sources.json',after);v={'build':build,'abort':abort,'generator':result,'test_execution_count':0 if result is None else 1,'passed':result['test_result']['passed'] if result and result['test_result'] else 0,'failed':result['test_result']['failed'] if result and result['test_result'] else 0,'source_changes':[p for p in sorted(before.keys()|after.keys()) if before.get(p)!=after.get(p)],'conservative_local_packages':39,'frozen_files':len(before),'artifact_records':len(a),'observed_owned_packages':len(set(x['package_id'] for x in owned)),'prior_failed_generator_binary_sha256':oldbin['sha256'],'selected_current_binary_sha256':b['sha256'] if b else None,'selected_binary_sha256_after':digest(Path(b['executable'])) if b else None,'actual_selected_target_kind':'bin','explicit_default_target':str(target),'original_fixture_assertions_unchanged':True,'source_or_fixture_updates_during_run':False,'literal_inventory_helper_event_or_braced_controls_repeated':False,'oracle_execution':False,'full_suite_run':False,'cache_released':True};save('validation.json',v);print('FINAL',json.dumps(v),flush=True)
if build['exit_code']:print((out/'offline-build.stderr').read_text()[-12000:],flush=True)
