import subprocess,pathlib,re,json,hashlib,time,shutil
root=pathlib.Path('/workspace/tcl-lsp');out=pathlib.Path('/workspace/naming-validation/compiler-vm-889');out.mkdir(exist_ok=True,parents=True);frozen=out/'frozen-binaries';frozen.mkdir(exist_ok=True)
build=pathlib.Path('/tmp/naming-compiler-vm-formal-availability-build.log').read_text();names=re.findall(r'Executable unittests src/lib.rs \(target/debug/deps/([^)]*)\)',build);assert len(names)==2
bins={}
for name in names:
 package=name.rsplit('-',1)[0];dest=frozen/name;shutil.copy2(root/'target/debug/deps'/name,dest);bins[package]=dest
binding={'source_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),'source_diff_sha256':None,'ordinary':True,'binaries':{k:{'name':v.name,'sha256':hashlib.sha256(v.read_bytes()).hexdigest()} for k,v in bins.items()}}
diff=subprocess.check_output(['git','diff','HEAD','--','rust','runtime/rust'],cwd=root);(out/'source.diff').write_bytes(diff);binding['source_diff_sha256']=hashlib.sha256(diff).hexdigest();(out/'source-binding.json').write_text(json.dumps(binding,indent=2)+'\n');print('BINARY_FREEZE_COMPLETE',flush=True)
spec={'tcl_vm': ['cmd_info::', 'cmd_namespace::', 'jim_result_publication_tests::', 'interp::native_original_lookup::tests'], 'tcl_compiler': ['original_formal_reference_', 'original_command_words_keep_actual_module_source_and_vector_owners', 'param_traits_inferred', 'used_in_condition_detected', 'forwarded_to_callee_detected', 'readonly_authored_coverage_', 'original_logical_parameter_roles_', 'conditional_parameter_source_prefix_', 'declaration_variable_operands_', 'declaration_mapped_source_operands_', 'declaration_configured_scalar_operands_', 'declaration_scalar_trace_exclusions_']}
records=[]
for package,filters in spec.items():
 binary=bins[package]
 for selector in filters:
  log=out/(package+'-'+re.sub('[^a-zA-Z0-9_-]','-',selector)+'.log');start=time.monotonic()
  with log.open('w') as f:p=subprocess.run([str(binary),selector,'--nocapture','--test-threads=1'],cwd=root,stdout=f,stderr=subprocess.STDOUT)
  s=log.read_text();m=re.search(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;',s)
  r={'package':package,'selector':selector,'source_head':binding['source_head'],'source_diff_sha256':binding['source_diff_sha256'],'binary_sha256':binding['binaries'][package]['sha256'],'exit':p.returncode,'seconds':time.monotonic()-start,'log':log.name,'log_sha256':hashlib.sha256(log.read_bytes()).hexdigest(),'status':'UNACCEPTED'}
  if m:r.update(passed=int(m[2]),failed=int(m[3]),ignored=int(m[4]));r['status']='NO_TESTS' if int(m[2])+int(m[3])==0 else ('PASS' if p.returncode==0 and int(m[3])==0 else 'FAIL')
  records.append(r);(out/'records.json').write_text(json.dumps(records,indent=2)+'\n');print(json.dumps(r),flush=True)
