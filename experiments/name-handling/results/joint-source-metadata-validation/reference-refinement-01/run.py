from pathlib import Path
import json,hashlib,gzip,subprocess,datetime,time,os,signal,shutil,re,ast,tomllib,glob
repo=Path('/workspace/tcl-lsp');out=Path('/tmp/mcp-reference-refinement-validation-01');out.mkdir(exist_ok=False);roots=['tcl-registry','tcl-irules'];target=repo/'tmp/name-target-issues';assert os.environ.get('CARGO_TARGET_DIR')==str(target)
w=tomllib.loads((repo/'Cargo.toml').read_text())['workspace'];manifests={}
for entry in w['members']:
 for directory in glob.glob(str(repo/entry)):
  manifest=Path(directory)/'Cargo.toml'
  if manifest.is_file():manifests[tomllib.loads(manifest.read_text())['package']['name']]=manifest.resolve()
rootpaths={manifests[name] for name in roots};queue=list(rootpaths);seen={};shared=w.get('dependencies',{})
while queue:
 manifest=queue.pop().resolve()
 if manifest in seen:continue
 data=tomllib.loads(manifest.read_text());seen[manifest]=data['package']['name']
 for table in [data,*data.get('target',{}).values()]:
  for kind in ['dependencies','build-dependencies']+(['dev-dependencies'] if manifest in rootpaths else []):
   for name,dependency in table.get(kind,{}).items():
    if not isinstance(dependency,dict):continue
    parent=manifest.parent
    if dependency.get('workspace'):
     base=shared.get(name,{});dependency={**(base if isinstance(base,dict) else {}),**dependency};parent=repo
    if 'path' in dependency:queue.append((parent/dependency['path']/'Cargo.toml').resolve())
closure={'selected_roots':roots,'package_dirs':{name:str(path.parent.relative_to(repo)) for path,name in sorted(seen.items())},'scope':'Actual conservative local normal/build and selected-root dev, all target/optional manifest paths; not a compiled graph.'}
module=ast.parse(Path('/tmp/mcp-joint-six-lib-validation-run.py').read_text());nodes=[n for n in module.body if isinstance(n,ast.FunctionDef) and n.name in {'digest','save','capture','run'}];exec(compile(ast.Module(body=nodes,type_ignores=[]),'<retained helpers>','exec'))
before=capture();save('before-sources.json',before);save('manifest-closure.json',closure);prior=json.loads(Path('/tmp/mcp-joint-six-lib-validation-01/before-sources.json').read_text());owners=['rust/tcl-registry/src/source_configuration_reference.rs','rust/tcl-irules/src/source_object.rs'];save('changed-owner-images.json',{p:(repo/p).read_text() for p in owners});save('baseline.json',{'selected_packages':roots,'conservative_packages':len(closure['package_dirs']),'frozen_files':len(before),'changed_owner_images':owners,'current_owner_sha256':{p:before[p] for p in owners},'changes_from_joint_validation_01_in_common_scope':[{'path':p,'previous':prior.get(p),'current':before[p]} for p in before if p in prior and prior[p]!=before[p]],'explicit_target':str(target),'original_registry_expectations_and_irules_assertions_unchanged':True,'braced_expression_repaired_or_repeated':False,'disk_free_before_bytes':shutil.disk_usage(repo).free})
print('FROZEN',json.dumps({'packages':len(closure['package_dirs']),'files':len(before),'changed_owner_images':2,'explicit_target':str(target)}),flush=True)
argv=['cargo','test','--offline','-j2','--lib','--no-run','--message-format=json','-p','tcl-registry','-p','tcl-irules'];build=run('offline-build',argv,space_guard=True);print('BUILD',json.dumps(build),flush=True)
records=[]
for line in (out/'offline-build.stdout').read_text().splitlines():
 try:r=json.loads(line)
 except json.JSONDecodeError:continue
 records.append(r)
artifacts=[r for r in records if r.get('reason')=='compiler-artifact'];save('artifacts.json',artifacts);save('diagnostics.json',[r for r in records if r.get('reason')=='compiler-message']);owned_manifests={str(repo/d/'Cargo.toml') for d in closure['package_dirs'].values()};owned=[]
for a in artifacts:
 if a['manifest_path'] in owned_manifests:
  owned.append({'package_id':a['package_id'],'target':a['target'],'target_source_sha256':before.get(str(Path(a['target']['src_path']).relative_to(repo))),'manifest_path':a['manifest_path'],'manifest_sha256':before.get(str(Path(a['manifest_path']).relative_to(repo))),'features':a['features'],'fresh':a['fresh']})
save('observed-owned-artifacts.json',owned);selected={}
for a in artifacts:
 if a.get('executable') and a['profile']['test'] and a['target']['kind']==['lib']:
  for name in roots:
   if a['manifest_path']==str(repo/closure['package_dirs'][name]/'Cargo.toml'):selected[name]={'artifact':a,'executable':a['executable'],'sha256':digest(Path(a['executable']))}
save('binaries.json',selected);tests=[('registry-positive','tcl-registry','source_configuration_reference::tests::selected_configuration_layout_keeps_unknown_item_and_original_ordinal'),('registry-adverse','tcl-registry','source_configuration_reference::tests::selected_configuration_layout_declines_unknown_control_and_cardinality'),('irules-positive','tcl-irules','source_object::tests::original_data_group_operands_keep_unknown_item_and_refuse_unknown_reference'),('irules-literal-adverse','tcl-irules','source_object::tests::typed_configuration_decline_does_not_reenter_literal_compatibility')];save('planned-controls.json',tests);results=[];abort=None
if build['exit_code'] or build['abort']:abort='selected two-package build failed; zero controls'
elif len(selected)!=2:abort='selected binary set incomplete'
elif capture()!=before:abort='source changed during build'
if not abort:
 for label,name,test in tests:
  if capture()!=before or digest(Path(selected[name]['executable']))!=selected[name]['sha256']:abort='source/binary drift';break
  info=run(label,[selected[name]['executable'],test,'--exact','--nocapture'],timeout=60);raw=(out/(label+'.stdout')).read_text();match=re.search(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out',raw)
  info.update({'package':name,'test':test,'test_result':dict(zip(['status','passed','failed','ignored','measured','filtered_out'],[match[1],*map(int,match.groups()[1:])])) if match else None,'source_unchanged_after':capture()==before,'binary_sha256_after':digest(Path(selected[name]['executable']))});save(label+'.invocation.json',info);results.append(info);print('CONTROL',label,json.dumps(info),flush=True);print(raw,flush=True);print((out/(label+'.stderr')).read_text(),flush=True)
after=capture();save('after-sources.json',after);validation={'build':build,'abort':abort,'controls_executed':len(results),'results':results,'passed':sum(r.get('test_result',{}).get('passed',0) for r in results),'failed':sum(r.get('test_result',{}).get('failed',0) for r in results),'source_changes':[p for p in sorted(before.keys()|after.keys()) if before.get(p)!=after.get(p)],'conservative_packages':len(closure['package_dirs']),'frozen_files':len(before),'artifact_records':len(artifacts),'observed_owned_packages':len(set(a['package_id'] for a in owned)),'binary_hashes_after':{n:digest(Path(b['executable'])) for n,b in selected.items()},'original_expectations_unchanged':True,'source_or_fixture_changes_during_run':False,'braced_expression_control_or_mcp_repeated':False,'oracle_execution':False,'full_suite_run':False,'cache_released':True};save('validation.json',validation);print('FINAL',json.dumps(validation),flush=True)
if build['exit_code']:print((out/'offline-build.stderr').read_text()[-12000:],flush=True)
