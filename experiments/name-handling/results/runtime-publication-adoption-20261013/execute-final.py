from pathlib import Path
import gzip,hashlib,json,shlex,subprocess,time
root=Path('/workspace/name-implementation-worktrees/runtime');base=Path('/tmp/runtime-publication-adoption-20261013');out=base/'final-control-capture';out.mkdir(exist_ok=True);target=root/'target'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def artifact(log,name):
 for l in (base/log).read_text().splitlines():
  try:r=json.loads(l)
  except ValueError:continue
  if r.get('reason')=='compiler-artifact' and r.get('target',{}).get('name')==name and r.get('profile',{}).get('test') and r.get('executable'):return Path(r['executable'])
 raise RuntimeError(name+' executable absent')
owner=artifact('build-owner-final.jsonl','tcl_syntax');runtime=artifact('build-runtime-final.jsonl','tcl_runtime');query=artifact('build-runtime-final.jsonl','native_namespace_query_bases')
with gzip.open(base/'source-before-final.json.gz','rt') as f:sources=json.load(f)
inputs={};unresolved=[];uncovered=[];dep_info={}
for p in target.rglob('*.d'):
 dep_info[str(p)]=sha(p)
 for line in p.read_text().replace('\\\n','').splitlines():
  if ': ' not in line:continue
  for s in shlex.split(line.split(': ',1)[1]):
   q=Path(s)
   if not q.is_absolute():q=next((c for c in [root/'runtime/rust'/q,root/q] if c.is_file()),q)
   if not q.is_file():unresolved.append({'dep_info':str(p),'input':s});continue
   q=q.resolve();inputs[str(q)]=sha(q)
   if str(q) not in sources and not q.is_relative_to(target):uncovered.append(str(q))
result={'source_before':'../source-before-final.json.gz','compiled_inputs':inputs,'dep_info':dep_info,'unresolved':unresolved,'uncovered':sorted(set(uncovered)),'artifacts':{str(p):sha(p) for p in [owner,runtime,query]},'runs':[]}
(out/'compiled-inputs.json').write_text(json.dumps(result,indent=2)+'\n')
if unresolved or uncovered:raise SystemExit('compiled input coverage incomplete; no tests run')
owner_names=['naming::shared_boundary_tests::written_command_parts_keep_empty_tail_and_counted_components','naming::oo_variables::explicit_variable_tests::varname_lookup_plan_retains_absolute_original_and_separates_constructed_inputs','naming::oo_variables::explicit_variable_tests::explicit_local_and_varname_lookup_keep_their_independent_byte_extents','naming::native::routing::tests::native_lookup_bases_select_release_path_and_original_extent','naming::native::routing::tests::purpose_and_selected_extent_control_alias_and_c_api_routes','naming::native::routing::tests::equal_constructed_paths_retain_distinct_root_and_current_routes','naming::native::routing::tests::ensemble_default_parent_and_explicit_current_routing_remain_distinct']
runtime_names=['namespace::tests::bootstrap_registration_and_restore_keep_the_last_nonempty_component','namespace::tests::qualified_registration_and_resolution','namespace::tests::unqualified_resolves_in_global','namespace::tests::generation_location_projects_actual_constructed_slots_and_declines_replacement','namespace::tests::compiler_and_resolver_epochs_match_original_native_mutation_rows','namespace::tests::constructed_path_and_lookup_stay_on_detached_original_token','cmd_oo::native_explicit_variable_tests::explicit_variable_links_and_varname_match_original_counted_controls','cmd_oo::native_original_holder_tests::original_oo_publication_preserves_constructed_holder_and_collision_results','cmd_oo::original_publication_tests::private_dispatcher_refuses_retired_namespace_without_adopting_recreated_holder','cmd_namespace::tests::oo_created_by_delete_trace_joins_dying_generation','cmd_namespace::tests::retained_and_recreated_import_sources_stay_generation_distinct','cmd_namespace::tests::retained_import_origin_chain_ignores_a_recreated_same_fqn_command','cmd_namespace::tests::import_follows_source_rename_but_not_delete','cmd_alias::original_alias_key_tests::original_alias_replacement_keeps_measured_name_key_and_query_roles','cmd_alias::original_alias_key_tests::original_alias_rename_keeps_measured_record_and_destination_key_roles','cmd_alias::original_alias_key_tests::original_jim_rename_qualification_and_table_key_keep_distinct_native_references','cmd_alias::original_rename_operation_tests::original_rename_observers_keep_reached_effects_and_stop_later_result_publication','cmd_alias::original_operand_tests::original_child_handle_alias_reads_the_selected_operand_before_publication','cmd_alias::tests::a_rename_that_would_close_a_loop_is_refused_and_rolled_back','cmd_alias::tests::a_refused_rename_leaves_the_destination_command_intact','cmd_alias::tests::visibility_failures_match_direct_and_child_oracles','cmd_alias::tests::child_shorthand_visibility_ops_are_leak_free']
plan=[(owner,n) for n in owner_names]+[(runtime,n) for n in runtime_names]+[(query,'namespace_queries_match_original_source_reports')]
for binary,names in [(owner,owner_names),(runtime,runtime_names),(query,['namespace_queries_match_original_source_reports'])]:
 listing=subprocess.check_output([str(binary),'--list'],text=True);(out/(binary.name+'-list.txt')).write_text(listing)
 for n in names:
  if n+': test' not in listing:raise SystemExit('Missing selector '+n)
(out/'exact-plan.json').write_text(json.dumps([{'binary':str(b),'selector':n} for b,n in plan],indent=2)+'\n')
for i,(binary,n) in enumerate(plan):
 command=[str(binary),n,'--exact','--nocapture','--test-threads=1'];log=out/f'test-{i+1:02}.log';t=time.monotonic()
 with log.open('w') as f:r=subprocess.run(command,cwd=root,stdout=f,stderr=subprocess.STDOUT,timeout=240)
 row={'selector':n,'binary':str(binary),'exit_code':r.returncode,'elapsed_seconds':time.monotonic()-t,'log':str(log),'sha256':sha(log),'exact_one_test':'running 1 test\n' in log.read_text(),'one_passed':'test result: ok. 1 passed; 0 failed;' in log.read_text()};result['runs'].append(row);(out/'result-in-progress.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(row),flush=True)
result['changed_compiled_inputs']={p:sha(Path(p)) for p,h in inputs.items() if sha(Path(p))!=h};result['changed_artifacts']={p:sha(Path(p)) for p,h in result['artifacts'].items() if sha(Path(p))!=h};(out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
