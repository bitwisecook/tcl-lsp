import subprocess,pathlib,re,json,hashlib,time
root=pathlib.Path('/workspace/tcl-lsp');out=pathlib.Path('/workspace/naming-validation/hosts-forwarding-class');binding=json.loads((out/'source-binding.json').read_text())
spec=[('tests/cli.rs:cli',''),('tests/grep.rs:grep',''),('tests/query_input.rs:query_input',''),('tests/tmsh.rs:tmsh',''),('tests/remote.rs:remote',''),('unittests src/lib.rs:f5_cli','commands::remote::auth::tests::opaque_environment_is_present_but_explicit_user_wins'),('unittests src/lib.rs:tcl_registry','source_configuration_reference::tests::'),('unittests src/lib.rs:tcl_spectcl','catalogue::tests::'),('unittests src/lib.rs:tcl_compiler','original_authored_call_reports_keep_actual_source_target_formals_and_debts'),('unittests src/lib.rs:tcl_compiler','forwarded_to_callee_detected'),('unittests src/lib.rs:tcl_compiler','param_traits_inferred'),('unittests src/main.rs:tcl_mcp','tools::')]
records=[]
for key,selector in spec:
 b=binding['binaries'][key];binary=out/'frozen-binaries'/b['name'];assert hashlib.sha256(binary.read_bytes()).hexdigest()==b['sha256']
 log=out/(key.rsplit(':',1)[-1]+'-'+re.sub('[^a-zA-Z0-9_-]','-',selector or 'all')+'.log');start=time.monotonic()
 prod=pathlib.Path(binding['production_f5_query']['path']);assert hashlib.sha256(prod.read_bytes()).hexdigest()==binding['production_f5_query']['sha256']
 args=[str(binary)]+([selector] if selector else [])+['--nocapture','--test-threads=1']
 with log.open('w') as f:p=subprocess.run(args,cwd=root,stdout=f,stderr=subprocess.STDOUT)
 assert hashlib.sha256(prod.read_bytes()).hexdigest()==binding['production_f5_query']['sha256']
 m=re.search(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;',log.read_text())
 row={'target':key,'selector':selector,'source_head':binding['source_head'],'source_diff_sha256':binding['source_diff_sha256'],'binary_sha256':b['sha256'],'exit':p.returncode,'seconds':time.monotonic()-start,'log':log.name,'log_sha256':hashlib.sha256(log.read_bytes()).hexdigest(),'status':'UNACCEPTED'}
 if m:
  row.update(passed=int(m[2]),failed=int(m[3]),ignored=int(m[4]));row['status']='NO_TESTS' if int(m[2])+int(m[3])==0 else 'PASS' if p.returncode==0 and int(m[3])==0 else 'FAIL'
 records.append(row);(out/'records.json').write_text(json.dumps(records,indent=2)+'\n');print(json.dumps(row),flush=True)
(out/'preview-status.json').write_text(json.dumps({'status':'NOT_RERUN_KNOWN_VM_HOST_BOOTSTRAP_FAILURE','prior_capture':'cli-mcp-server-889','reason':'Current cd0 VM controls confirm same host bootstrap blocker; reviewed Host repair is not in this binary. Original first failure/shutdown stop and23 unexecuted controls remain open. No test/assertion/budget changed.'},indent=2)+'\n')
