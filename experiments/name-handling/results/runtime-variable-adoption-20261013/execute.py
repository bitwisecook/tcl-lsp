from pathlib import Path
import gzip,hashlib,json,os,shlex,subprocess,sys,time
root=Path('/workspace/name-implementation-worktrees/runtime'); out=Path('/tmp/runtime-variable-adoption-20261013'); target=root/'target'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
label=sys.argv[1]; build=sys.argv[2]; before=sys.argv[3]
rows=[]
for line in (out/build).read_text().splitlines():
 try:r=json.loads(line)
 except ValueError:continue
 if r.get('reason')=='compiler-artifact' and r.get('executable'): rows.append(r)
lib=next(Path(r['executable']) for r in rows if r['target']['name']=='tcl_runtime' and r['profile']['test']); query=next(Path(r['executable']) for r in rows if r['target']['name']=='native_namespace_query_bases')
with gzip.open(out/(before+'.json.gz'),'rt') as f: sources=json.load(f)
inputs={};unresolved=[];uncovered=[];d_files={}
for p in target.rglob('*.d'):
 d_files[str(p)]=sha(p)
 for line in p.read_text().replace('\\\n','').splitlines():
  if ': ' not in line: continue
  for s in shlex.split(line.split(': ',1)[1]):
   q=Path(s)
   if not q.is_absolute():
    candidates=[root/'runtime/rust'/q,root/q]
    q=next((c for c in candidates if c.is_file()),q)
   if not q.is_file():unresolved.append({'dep_info':str(p),'input':s});continue
   q=q.resolve(); inputs[str(q)]=sha(q)
   if str(q) not in sources and not q.is_relative_to(target):uncovered.append(str(q))
result={'source_before':before,'compiled_inputs':inputs,'dep_info':d_files,'unresolved':unresolved,'uncovered':sorted(set(uncovered)),'artifacts':{str(p):sha(p) for p in [lib,query]},'runs':[]}
(out/(label+'-compiled-inputs.json')).write_text(json.dumps(result,indent=2)+'\n')
if unresolved or uncovered:raise SystemExit('compiled input coverage incomplete; no tests run')
selectors=['cmd_var::tests::qualified_variable_fallback_depends_on_cell_and_release','cmd_var::tests::ns_scope_unqualified_falls_back_to_global_only_under_8x','cmd_var::tests::the_fallback_is_selected_by_the_emulated_release','cmd_var::tests::the_fallback_never_walks_intermediate_parents','vars::tests::plain_and_global_qualified_alias_the_same_var','vars::tests::unqualified_in_proc_is_frame_local','vars::tests::variable_in_proc_links_to_namespace_var','vars::tests::qualified_set_into_missing_namespace_errors','cmd_alias::tests::rename_moves_a_command','cmd_alias::tests::rename_and_hide_error_shapes_and_lifecycle']
if label=='final':selectors+=['cmd_var::tests::jim_flat_absolute_variable_names','cmd_var::tests::jim_relative_qualified_proc_variable','cmd_var::tests::global_namespace_destinations_follow_real_engine_activation_rules','interp::native_jim_lookup::tests::original_jim_lookup_primaries_match_native_key_and_cache_ownership','interp::native_jim_lookup::tests::jim_original_variable_frame_miss_and_unrelated_unset_match_native_controls']
listing=subprocess.check_output([str(lib),'--list'],text=True); (out/(label+'-test-list.txt')).write_text(listing)
for n in selectors:
 if n+': test' not in listing:raise SystemExit('Missing selector '+n)
for i,(binary,n) in enumerate([(lib,n) for n in selectors]+[(query,'namespace_queries_match_original_source_reports')]):
 command=[str(binary),n,'--exact','--nocapture','--test-threads=1']; log=out/(label+f'-test-{i+1:02}.log');t=time.monotonic()
 with log.open('w') as f:r=subprocess.run(command,cwd=root/'runtime/rust',stdout=f,stderr=subprocess.STDOUT,timeout=180)
 row={'selector':n,'exit_code':r.returncode,'elapsed_seconds':time.monotonic()-t,'log':str(log),'sha256':sha(log),'exact_one_test':'running 1 test\n' in log.read_text(),'one_passed':'test result: ok. 1 passed; 0 failed;' in log.read_text()};result['runs'].append(row);print(json.dumps(row),flush=True)
result['changed_compiled_inputs']={p:sha(Path(p)) for p,h in inputs.items() if sha(Path(p))!=h}; result['changed_artifacts']={p:sha(Path(p)) for p,h in result['artifacts'].items() if sha(Path(p))!=h}
(out/(label+'-result.json')).write_text(json.dumps(result,indent=2)+'\n')
