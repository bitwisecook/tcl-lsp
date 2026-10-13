import importlib.util,json,pathlib,subprocess,time,os,shutil,hashlib
spec=importlib.util.spec_from_file_location('capture','/tmp/codegen-original-argv-capture-02.py');c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
ROOT=c.ROOT
out=ROOT/'experiments/name-handling/results/codegen-root-colon-publication/compiler-wasm-validation-01';out.mkdir(exist_ok=False)
meta=['/tmp/codegen-original-argv-metadata-preparation-03/workspace.stdout','/tmp/codegen-original-argv-metadata-preparation-02/runtime.stdout']
packages=c.local_packages(meta)
targets=[str(ROOT/'tmp/name-target-names'),str(ROOT/'target/tcl_reserved_runtime')]
record={'started_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'git_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'scope':'Frozen resolved local package files, original literal external include inputs, existing compiled dep-info inputs and libtommath; post-build dep-info audited separately. Not a full-suite or all-consumer-green claim.','environment':{k:os.environ.get(k) for k in ['CARGO_TARGET_DIR','CARGO_NET_OFFLINE','CARGO_INCREMENTAL','CARGO_BUILD_JOBS','CARGO_PROFILE_DEV_DEBUG','TCL_REQUIRE_WASM_LINK','WASI_SDK_PATH','TCL_TOMMATH_DIR','TCL_LSP_REQUIRE_ALL_TCL_ORACLES','TCL_LSP_REQUIRE_JIM_ORACLE','CARGO_HOME','RUSTUP_HOME','RUSTFLAGS']},'tools':{},'builds':[],'tests':[]}
for name in ['rustc','cargo','wasmtime','wasm-merge','cc']:
 path=pathlib.Path(shutil.which(name)).resolve()
 if name in ['rustc','cargo']:
  actual=subprocess.check_output(['rustup','which',name],text=True).strip();path=pathlib.Path(actual).resolve()
 r=subprocess.run([str(path),'--version'],capture_output=True)
 record['tools'][name]={'path':str(path),'sha256':c.digest(path),'exit_code':r.returncode,'stdout':r.stdout.decode(errors='replace'),'stderr':r.stderr.decode(errors='replace')}
clang=pathlib.Path(os.environ['WASI_SDK_PATH'])/'bin/clang';record['tools']['wasi_clang']={'path':str(clang.resolve()),'sha256':c.digest(clang.resolve()),'version':subprocess.check_output([str(clang),'--version'],text=True)}
(out/'packages.json').write_text(json.dumps([{'name':p['name'],'manifest_path':p['manifest_path'],'version':p['version']} for p in packages.values()],indent=2)+'\n')
for i,p in enumerate(meta):shutil.copyfile(p,out/f'metadata-{i}.json')
shutil.copyfile('/tmp/codegen-original-argv-capture-02.py',out/'capture.py');shutil.copyfile(__file__,out/'run.py')
print('snapshotting frozen inputs',flush=True)
before=c.static_inputs(packages,targets);(out/'inputs-before.json').write_text(json.dumps(before,indent=2)+'\n');print('frozen inputs',len(before),flush=True)
plans=[('api',['cargo','test','--offline','-p','tcl-runtime-api','--lib','--no-run','--message-format=json']),('compiler',['cargo','test','--offline','-p','tcl-compiler','--lib','--test','wasm_codegen','--test','wasm_real_link','--no-run','--message-format=json']),('cli',['cargo','test','--offline','-p','tcl-cli','--test','cli','--no-run','--message-format=json'])]
artifacts=[];binaries={}
for label,args in plans:
 print('building',label,flush=True);br=c.command_record(args,out,'build-'+label);record['builds'].append(br);print('build',label,'exit',br['exit_code'],flush=True)
 if br['exit_code']:
  if label=='compiler':record['compiler_dependent_builds_stopped_after_failure']=True;break
  continue
 for line in (out/br['stdout']).read_text().splitlines():
  try:a=json.loads(line)
  except json.JSONDecodeError:continue
  if a.get('reason')=='compiler-artifact':
   artifacts.append(a)
   if a.get('executable'):binaries[a['target']['name']]=a['executable']
 if label=='api':
  b=binaries['tcl_runtime_api'];selectors=[('api-descriptors',b,'codegen_abi::tests::',120)]
 elif label=='compiler':
  b=binaries['tcl_compiler'];selectors=[('compiler-original-definition',b,'native_lowering::tests::original_',300),('compiler-admission',b,'codegen::wasm::pipeline::tests::compiler_admission_evaluates_one_original_chunk_before_any_stores_or_exports',180),('compiler-missing-source',b,'codegen::wasm::pipeline::tests::missing_original_compilation_source_is_a_typed_host_refusal',180)]
  for i,s in enumerate(['optimiser::tail_call::tests::original_allocation_and_grouped_names_control_tail_rewrites','optimiser::tail_call::tests::unrelated_short_name_is_not_the_original_procedure','optimiser::tail_call::tests::produced_expression_commands_retain_readonly_self_identity','command_binding::declaration_layout::tests::original_lookup_completeness_keeps_unknown_absent_and_namespace_alternatives','command_binding::original_command_table::tests::original_catalogue_advice_stops_at_earlier_occupied_namespace_cells','command_binding::original_command_table::tests::original_mutation_selector_stops_at_the_first_occupied_cell','command_binding::original_command_table::tests::closed_original_lookup_retains_missing_holder_absence_without_a_slot','command_binding::original_command_table::tests::original_metadata_index_keeps_collisions_and_complete_baseline_keys']):selectors.append(('compiler-base-'+str(i),b,s,180))
  for i,s in enumerate(['top_installs_one_table_entry_per_bound_body','only_the_definition_whose_body_was_compiled_binds_an_entry','general_host_refusal_releases_the_current_frame_and_stops_before_later_effects']):selectors.append(('wasm-plan-'+str(i),binaries['wasm_codegen'],s,300))
  for i,s in enumerate(['root_colon_publication_runs_its_own_compiled_table_entry','original_argv_descriptor_requires_current_manifest_and_rejects_prior_fingerprint','general_original_definition_materialises_its_body_once','general_original_definition_keeps_braced_body_text_inert','general_original_definition_evaluates_its_name_once','general_original_definition_preserves_unknown_name_error','general_original_definition_resolves_a_replaced_definer','native_original_definition_preserves_a_replaced_definer']):selectors.append(('real-wasm-'+str(i),binaries['wasm_real_link'],s,1200))
 else:selectors=[('cli-exact-names',binaries['cli'],'command_info_preserves_exact_command_names',180)]
 for name,b,sel,timeout in selectors:
  print('running',name,sel,flush=True)
  args=[b,sel,'--nocapture','--test-threads=1']
  tr=c.command_record(args,out,name,timeout);tr.update(selector=sel,binary_sha256=c.digest(pathlib.Path(b)),classification='genuine runtime-linked WASM required' if name.startswith('real-wasm') else 'software/compiler control')
  text=(out/tr['stdout']).read_text(errors='replace');tr['executed_test_count']=sum(int(x) for x in __import__('re').findall(r'^running (\d+) tests?$',text,__import__('re').M));tr['vacuous_selector']=tr['executed_test_count']==0
  record['tests'].append(tr);(out/'validation.partial.json').write_text(json.dumps(record,indent=2)+'\n');print('result',name,tr['exit_code'],'tests',tr['executed_test_count'],flush=True)
(out/'artifacts.json').write_text(json.dumps(artifacts,indent=2)+'\n')
record['executables']={name:{'path':p,'sha256_after':c.digest(pathlib.Path(p))} for name,p in binaries.items()}
wasm=ROOT/'target/tcl_reserved_runtime/wasm32-wasip1/debug/tcl_runtime.wasm'
if wasm.is_file():record['reserved_wasm_runtime']={'path':str(wasm),'sha256':c.digest(wasm)}
print('snapshotting final inputs and dep-info',flush=True)
after=c.static_inputs(packages,targets);(out/'inputs-after.json').write_text(json.dumps(after,indent=2)+'\n')
record['preexisting_inputs_unchanged']=all(after.get(k)==v for k,v in before.items());record['changed_preexisting_inputs']={k:{'before':v,'after':after.get(k)} for k,v in before.items() if after.get(k)!=v};record['before_input_count']=len(before);record['after_input_count']=len(after)
(out/'compiled-input-coverage.json').write_text(json.dumps(c.coverage(before,after,targets),indent=2)+'\n')
record['finished_utc']=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime());(out/'validation.json').write_text(json.dumps(record,indent=2)+'\n')
print('capture complete',record['preexisting_inputs_unchanged'],flush=True)
