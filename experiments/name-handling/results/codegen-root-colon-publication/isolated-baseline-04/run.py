import pathlib,json,hashlib,os,subprocess,time,shutil,importlib.util,re
ROOT=pathlib.Path('/workspace/name-implementation-worktrees/codegen')
spec=importlib.util.spec_from_file_location('capture','/tmp/codegen-original-argv-capture-02.py');c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c);c.ROOT=ROOT
out=ROOT/'experiments/name-handling/results/codegen-root-colon-publication/isolated-baseline-04';out.mkdir(parents=True,exist_ok=False)
packages=c.local_packages(['/tmp/name-codegen-isolated-metadata.json','/tmp/name-codegen-isolated-runtime-metadata.json']);targets=['/tmp/name-agent-target-codegen',str(ROOT/'target/tcl_reserved_runtime')]
# The helper's c.local_packages originally selected main-checkout manifests;
# its ROOT is replaced above, so package roots are this isolated checkout.
record={'git_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'started_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'scope':'Independent isolated base compilation and genuine Runtime-linked general-WASM positives. Initial sparse-input failures and memory interruption are prerequisite results, not accepted test captures. Full local files plus compiled dep-info/external include data, with new generated/third-party inputs separately classified.','environment':{k:os.environ.get(k) for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS','CARGO_PROFILE_DEV_DEBUG','CARGO_PROFILE_TEST_DEBUG','CARGO_INCREMENTAL','CARGO_NET_OFFLINE','TCL_REQUIRE_WASM_LINK','XDG_CACHE_HOME','WASI_SDK_PATH','TCL_TOMMATH_DIR']},'tools':{},'tests':[]}
for tool in ['rustc','cargo','wasmtime','wasm-merge']:
 path=pathlib.Path(subprocess.check_output(['rustup','which',tool],text=True).strip()) if tool in ['cargo','rustc'] else pathlib.Path(shutil.which(tool));r=subprocess.run([str(path),'--version'],capture_output=True);record['tools'][tool]={'path':str(path.resolve()),'sha256':c.digest(path.resolve()),'exit_code':r.returncode,'stdout':r.stdout.decode(errors='replace'),'stderr':r.stderr.decode(errors='replace')}
for n in ['01','02','03']:
 for suffix in ['jsonl','stderr']:
  p=pathlib.Path('/tmp/name-codegen-isolated-build-'+n+'.'+suffix)
  if p.exists():shutil.copyfile(p,out/f'prerequisite-build-{n}.{suffix}')
for n in ['01','02']:shutil.copyfile('/tmp/name-codegen-sparse-fixtures-restored-'+n+'.json',out/f'sparse-input-restoration-{n}.json')
for name,p in [('workspace-metadata.json','/tmp/name-codegen-isolated-metadata.json'),('runtime-metadata.json','/tmp/name-codegen-isolated-runtime-metadata.json'),('capture.py','/tmp/codegen-original-argv-capture-02.py'),('run.py',__file__)]:shutil.copyfile(p,out/name)
before=c.static_inputs(packages,targets);(out/'inputs-before.json').write_text(json.dumps(before,indent=2)+'\n');print('before',len(before),'local packages',len(packages),flush=True)
args=['cargo','test','--offline','-p','tcl-compiler','--lib','--test','wasm_real_link','--no-run','--message-format=json'];record['build']=c.command_record(args,out,'build-04');print('build exit',record['build']['exit_code'],flush=True)
artifacts=[];binaries={}
if record['build']['exit_code']==0:
 for line in (out/'build-04.stdout').read_text().splitlines():
  try:a=json.loads(line)
  except json.JSONDecodeError:continue
  if a.get('reason')=='compiler-artifact':
   artifacts.append(a)
   if a.get('executable'):binaries[a['target']['name']]=a['executable']
 (out/'artifacts.json').write_text(json.dumps(artifacts,indent=2)+'\n');record['executables']={name:{'path':p,'sha256_before':c.digest(pathlib.Path(p))} for name,p in binaries.items()}
 print('HEAVY LINK COMPLETE',flush=True)
 for i,selector in enumerate(['general_original_definition_preserves_a_renamed_stock_definer','general_original_definition_preserves_a_materialised_name_word']):
  b=binaries['wasm_real_link'];print('running',selector,flush=True);r=c.command_record([b,selector,'--nocapture','--test-threads=1'],out,f'test-{i}',1800);r['selector']=selector;r['executed_test_count']=sum(int(x) for x in re.findall(r'^running (\d+) tests?$',(out/r['stdout']).read_text(),re.M));record['tests'].append(r);print('result',r['exit_code'],flush=True)
 for name,info in record['executables'].items():info['sha256_after']=c.digest(pathlib.Path(info['path']))
wasm=ROOT/'target/tcl_reserved_runtime/wasm32-wasip1/debug/tcl_runtime.wasm'
if wasm.exists():record['reserved_runtime']={'path':str(wasm),'sha256':c.digest(wasm)}
after=c.static_inputs(packages,targets);(out/'inputs-after.json').write_text(json.dumps(after,indent=2)+'\n');record['unchanged_preexisting_inputs']=all(after.get(k)==v for k,v in before.items());record['changed_preexisting_inputs']={k:{'before':v,'after':after.get(k)} for k,v in before.items() if after.get(k)!=v};(out/'compiled-input-coverage.json').write_text(json.dumps(c.coverage(before,after,targets),indent=2)+'\n');record['finished_utc']=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime());(out/'validation.json').write_text(json.dumps(record,indent=2)+'\n');print('capture complete',record['unchanged_preexisting_inputs'],flush=True)
