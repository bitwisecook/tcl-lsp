import pathlib,json,os,subprocess,time,shutil,importlib.util,re
ROOT=pathlib.Path('/workspace/name-implementation-worktrees/codegen')
def module(name,path):
 spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
c=module('capture','/tmp/name-codegen-isolated-capture.py');c.ROOT=ROOT
audit=module('audit','/tmp/name-codegen-selected-output-audit.py')
out=ROOT/'experiments/name-handling/results/codegen-root-colon-publication/first-gate-diagnostic-04';out.mkdir(parents=True,exist_ok=False)
source=ROOT/'rust/tcl-compiler/src/native_lowering/tests.rs';original=source.read_bytes();(out/'native-lowering-tests-preimage.rs').write_bytes(original)
patch=pathlib.Path('/tmp/codegen-proc-entry-diag.patch');shutil.copyfile(patch,out/'temporary-diagnostic.patch')
subprocess.run(['git','apply','--check',str(patch)],cwd=ROOT,check=True);subprocess.run(['git','apply',str(patch)],cwd=ROOT,check=True)
packages=c.local_packages(['/tmp/name-codegen-isolated-metadata.json','/tmp/name-codegen-isolated-runtime-metadata.json']);targets=[os.environ['CARGO_TARGET_DIR']]
record={'git_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'started_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'scope':'Temporary test-only genuine native source gate observation; instrumented source is not a production green result. BPF source changes remain production inputs. No Runtime stub or native-table acceptance.','environment':{k:os.environ.get(k) for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS','CARGO_PROFILE_DEV_DEBUG','CARGO_PROFILE_TEST_DEBUG','CARGO_INCREMENTAL','CARGO_NET_OFFLINE']},'tools':{},'tests':[],'restoration':{'path':str(source),'preimage_sha256':__import__('hashlib').sha256(original).hexdigest(),'instrumented_sha256':c.digest(source)}}
artifacts=[];before={}
try:
 for tool in ['rustc','cargo']:
  path=pathlib.Path(subprocess.check_output(['rustup','which',tool],text=True).strip());r=subprocess.run([str(path),'--version'],capture_output=True);record['tools'][tool]={'path':str(path.resolve()),'sha256':c.digest(path.resolve()),'exit_code':r.returncode,'stdout':r.stdout.decode(),'stderr':r.stderr.decode()}
 for name,path in [('capture.py','/tmp/name-codegen-isolated-capture.py'),('selected-output-audit.py','/tmp/name-codegen-selected-output-audit.py'),('run.py',__file__),('workspace-metadata.json','/tmp/name-codegen-isolated-metadata.json'),('runtime-metadata.json','/tmp/name-codegen-isolated-runtime-metadata.json')]:shutil.copyfile(path,out/name)
 before=c.static_inputs(packages,targets);(out/'inputs-before.json').write_text(json.dumps(before,indent=2)+'\n');print('before',len(before),flush=True)
 record['build']=c.command_record(['cargo','test','--offline','-p','tcl-compiler','--lib','--no-run','--message-format=json'],out,'build');print('build exit',record['build']['exit_code'],flush=True)
 if record['build']['exit_code']==0:
  for line in (out/'build.stdout').read_text().splitlines():
   try:a=json.loads(line)
   except json.JSONDecodeError:continue
   if a.get('reason')=='compiler-artifact':artifacts.append(a)
  (out/'artifacts.json').write_text(json.dumps(artifacts,indent=2)+'\n');binary=next(a['executable'] for a in artifacts if a.get('executable') and a['target']['name']=='tcl_compiler');record['executable']={'path':binary,'sha256_before':c.digest(pathlib.Path(binary))};print('HEAVY LINK COMPLETE',flush=True)
  selector='native_lowering::tests::original_root_colon_definition_preserves_evaluated_publication_operand';os.environ['TCL_LSP_TRACE_PROC_ENTRY_GATES']='1';r=c.command_record([binary,selector,'--exact','--nocapture','--test-threads=1'],out,'test',900);os.environ.pop('TCL_LSP_TRACE_PROC_ENTRY_GATES');r['selector']=selector;r['environment_changes']={'TCL_LSP_TRACE_PROC_ENTRY_GATES':'1'};r['executed_test_count']=sum(int(x) for x in re.findall(r'^running (\d+) tests?$',(out/r['stdout']).read_text(),re.M));record['tests'].append(r);record['executable']['sha256_after']=c.digest(pathlib.Path(binary));print('result',r['exit_code'],'count',r['executed_test_count'],flush=True)
 after=c.static_inputs(packages,targets);(out/'inputs-after.json').write_text(json.dumps(after,indent=2)+'\n');record['unchanged_instrumented_inputs']=all(after.get(k)==v for k,v in before.items());record['changed_instrumented_inputs']={k:{'before':v,'after':after.get(k)} for k,v in before.items() if after.get(k)!=v};(out/'selected-local-output-coverage.json').write_text(json.dumps(audit.selected_output_coverage(c,artifacts,packages,before,after),indent=2)+'\n')
finally:
 source.write_bytes(original);record['restoration']['restored_sha256']=c.digest(source);record['restoration']['exact_preimage_restored']=source.read_bytes()==original;record['finished_utc']=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime());(out/'validation.json').write_text(json.dumps(record,indent=2)+'\n');print('restored',record['restoration']['exact_preimage_restored'],flush=True)
