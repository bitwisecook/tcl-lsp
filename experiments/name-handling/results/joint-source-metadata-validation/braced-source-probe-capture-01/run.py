from pathlib import Path
import json,hashlib,gzip,subprocess,datetime,time,os,signal,shutil,re,ast,tomllib,glob
prefix=Path('/tmp/mcp-reference-refinement-validation-run.py').read_text().split('before=capture();')[0]
prefix=prefix.replace("out=Path('/tmp/mcp-reference-refinement-validation-01')","out=Path('/tmp/braced-original-source-probe-capture-02')")
exec(prefix)
closure['actual_selected_build_root']=['tcl-irules'];closure['baseline_extra_root_dev_dependencies']=['tcl-registry'];closure['scope']+='; conservative broader baseline includes Registry root dev dependencies, actual selected Cargo root is only iRules.'
prepared=Path('/tmp/braced-original-source-probe-01');ready=json.loads((prepared/'readiness.json').read_text());owners=list(ready['preimages']);baseline=capture();save('baseline-sources.json',baseline);save('manifest-closure.json',closure);save('source-readiness.json',ready);saved={p:(repo/p).read_bytes() for p in owners};assert all(digest(repo/p)==h for p,h in ready['preimages'].items())
assert len(closure['package_dirs']) in (25,26)
save('production-source-images.json',{p:saved[p].decode() for p in owners})
shutil.copyfile(prepared/'probe.patch',out/'probe.patch')
build=None;test=None;selected=None;artifacts=[];owned=[];abort=None;restored=False;instrumented=None
try:
 for p in owners:(repo/p).write_bytes((prepared/Path(p).name).read_bytes())
 instrumented=capture();save('instrumented-sources.json',instrumented);changes=[p for p in baseline if baseline[p]!=instrumented.get(p)];assert sorted(changes)==sorted(owners)
 save('probe-images.json',{p:(repo/p).read_text() for p in owners})
 print('FROZEN',json.dumps({'packages':len(closure['package_dirs']),'files':len(baseline),'temporary_deltas':changes,'syntax_source_sha256':baseline.get('rust/tcl-syntax/src/semantic/operators.rs'),'target':str(target)}),flush=True)
 build=run('offline-build',['cargo','test','--offline','-j2','-p','tcl-irules','--lib','--no-run','--message-format=json'],space_guard=True);print('BUILD',json.dumps(build),flush=True)
 records=[]
 for line in (out/'offline-build.stdout').read_text().splitlines():
  try:records.append(json.loads(line))
  except json.JSONDecodeError:pass
 artifacts=[r for r in records if r.get('reason')=='compiler-artifact'];save('artifacts.json',artifacts);save('diagnostics.json',[r for r in records if r.get('reason')=='compiler-message'])
 owned_manifests={str(repo/d/'Cargo.toml') for d in closure['package_dirs'].values()};owned=[a for a in artifacts if a['manifest_path'] in owned_manifests];save('observed-owned-artifacts.json',owned)
 for a in artifacts:
  if a.get('executable') and a['profile']['test'] and a['target']['kind']==['lib'] and a['manifest_path']==str(repo/'rust/tcl-irules/Cargo.toml'):selected={'artifact':a,'executable':a['executable'],'sha256':digest(Path(a['executable']))}
 save('binary.json',selected)
 if build['exit_code'] or build['abort']:abort='first compilation failure; zero assertion controls'
 elif not selected:abort='selected library binary absent'
 elif capture()!=instrumented:abort='source changed during compile'
 else:
  selector=ready['original_control'];listing=run('exact-selector-list',[selected['executable'],selector,'--exact','--list'],timeout=60);listed=(out/'exact-selector-list.stdout').read_text();assert listing['exit_code']==0 and listed.count(selector+': test')==1
  assert capture()==instrumented and digest(Path(selected['executable']))==selected['sha256']
  os.environ[ready['environment_gate']]='1'
  try:test=run('braced-original-control',[selected['executable'],selector,'--exact','--nocapture'],timeout=60)
  finally:os.environ.pop(ready['environment_gate'],None)
  raw=(out/'braced-original-control.stdout').read_text();match=re.search(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out',raw)
  test.update({'test':selector,'test_result':dict(zip(['status','passed','failed','ignored','measured','filtered_out'],[match[1],*map(int,match.groups()[1:])])) if match else None,'environment_gate':ready['environment_gate'],'instrumented_source_unchanged':capture()==instrumented,'binary_sha256_after':digest(Path(selected['executable']))});save('braced-original-control.invocation.json',test);print('CONTROL',json.dumps(test),flush=True);print(raw,flush=True);print((out/'braced-original-control.stderr').read_text(),flush=True)
except BaseException as e:
 abort=f'runner exception {type(e).__name__}: {e}';save('runner-exception.json',{'abort':abort});raise
finally:
 save('before-restore-sources.json',capture())
 for p,data in saved.items():(repo/p).write_bytes(data)
 after=capture();save('restored-sources.json',after);restored=all((repo/p).read_bytes()==data for p,data in saved.items());assert restored
 validation={'build':build,'control':test,'abort':abort,'conservative_packages':len(closure['package_dirs']),'baseline_files':len(baseline),'instrumented_source_changes':[] if instrumented is None else [p for p in baseline if baseline[p]!=instrumented.get(p)],'restored_byte_exact':restored,'changes_after_restore':[p for p in sorted(baseline.keys()|after.keys()) if baseline.get(p)!=after.get(p)],'binary_current_after_restore':False if selected else None,'binary_sha256':selected['sha256'] if selected else None,'artifacts':len(artifacts),'observed_owned_packages':len(set(a['package_id'] for a in owned)),'assertion_controls_executed':1 if test else 0,'passive_expr_issuer_newly_called':True,'expr_children_newly_enqueued':False,'original_fixture_assertions_unchanged':True,'semantic_fix':False,'cache_released':True,'full_suite_or_oracle':False};save('validation.json',validation);print('RESTORED_RELEASE',json.dumps(validation),flush=True)
