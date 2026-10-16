import subprocess,pathlib,re,json,hashlib,time
root=pathlib.Path('/workspace/tcl-lsp');out=pathlib.Path('/workspace/naming-validation/five-library-cd0')
binding=json.loads((out/'source-binding.json').read_text())
spec={
 'tcl_vm':['cmd_info::','cmd_namespace::','jim_result_publication_tests::','interp::native_original_lookup::tests'],
 'tcl_compiler':['original_formal_reference_','scalar_source_name_roles_keep_original_declaration_scope_and_currency','original_command_words_keep_actual_module_source_and_vector_owners','param_traits_inferred','used_in_condition_detected','forwarded_to_callee_detected','readonly_authored_coverage_','original_logical_parameter_roles_','conditional_parameter_source_prefix_','declaration_variable_operands_','declaration_mapped_source_operands_','declaration_configured_scalar_operands_','declaration_scalar_trace_exclusions_'],
 'tcl_registry':['model::context::tests::source_math_catalogue_retains_core_release_build_and_unknown','resolved_invocation::tests::source_completion_options_','source_name_ownership::tests::trace_registration_source_exclusion_'],
 'tcl_lsp_core':['semantic_tokens::original_definition_vocabulary_tests::','completion::original_context::tests::','math_function_symbol::tests::'],
 'tcl_cli_support':['input::tests::']}
records=[]
for package,selectors in spec.items():
 b=binding['binaries'][package];binary=out/'frozen-binaries'/b['name'];assert hashlib.sha256(binary.read_bytes()).hexdigest()==b['sha256']
 for selector in selectors:
  log=out/(package+'-'+re.sub('[^a-zA-Z0-9_-]','-',selector)+'.log');start=time.monotonic()
  with log.open('w') as f:p=subprocess.run([str(binary),selector,'--nocapture','--test-threads=1'],cwd=root,stdout=f,stderr=subprocess.STDOUT)
  m=re.search(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;',log.read_text())
  row={'package':package,'selector':selector,'source_head':binding['source_head'],'source_diff_sha256':binding['source_diff_sha256'],'binary_sha256':b['sha256'],'exit':p.returncode,'seconds':time.monotonic()-start,'log':log.name,'log_sha256':hashlib.sha256(log.read_bytes()).hexdigest(),'status':'UNACCEPTED'}
  if m:
   row.update(passed=int(m[2]),failed=int(m[3]),ignored=int(m[4]));row['status']='NO_TESTS' if int(m[2])+int(m[3])==0 else 'PASS' if p.returncode==0 and int(m[3])==0 else 'FAIL'
  records.append(row);(out/'records.json').write_text(json.dumps(records,indent=2)+'\n');print(json.dumps(row),flush=True)
