import pathlib,json,hashlib,os,subprocess,time,shutil,importlib.util,re
ROOT=pathlib.Path('/workspace/name-implementation-worktrees/codegen')
def module(name,path):
 spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
c=module('capture','/tmp/name-codegen-isolated-capture.py');c.ROOT=ROOT
audit=module('audit','/tmp/name-codegen-selected-output-audit.py')
out=ROOT/'experiments/name-handling/results/codegen-bpf-source-selection/validation-03';out.mkdir(parents=True,exist_ok=False)
packages=c.local_packages(['/tmp/name-codegen-isolated-metadata.json','/tmp/name-codegen-isolated-runtime-metadata.json']);targets=[os.environ['CARGO_TARGET_DIR']]
record={'git_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'started_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'scope':'Production BPF source/descriptor consumers, typed IR and eBPF lib tests. No temporary instrumentation, Runtime/WASM/native-entry or C/Jim/F5 execution.','environment':{k:os.environ.get(k) for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS','CARGO_PROFILE_DEV_DEBUG','CARGO_PROFILE_TEST_DEBUG','CARGO_INCREMENTAL','CARGO_NET_OFFLINE']},'tools':{},'tests':[]}
for tool in ['rustc','cargo']:
 path=pathlib.Path(subprocess.check_output(['rustup','which',tool],text=True).strip());r=subprocess.run([str(path),'--version'],capture_output=True);record['tools'][tool]={'path':str(path.resolve()),'sha256':c.digest(path.resolve()),'exit_code':r.returncode,'stdout':r.stdout.decode(),'stderr':r.stderr.decode()}
for name,path in [('capture.py','/tmp/name-codegen-isolated-capture.py'),('selected-output-audit.py','/tmp/name-codegen-selected-output-audit.py'),('run.py',__file__),('source.patch','/tmp/name-codegen-bpf-source-adapters-03.patch'),('workspace-metadata.json','/tmp/name-codegen-isolated-metadata.json'),('runtime-metadata.json','/tmp/name-codegen-isolated-runtime-metadata.json')]:shutil.copyfile(path,out/name)
before=c.static_inputs(packages,targets);(out/'inputs-before.json').write_text(json.dumps(before,indent=2)+'\n');print('before',len(before),flush=True)
record['build']=c.command_record(['cargo','test','--offline','-p','bpf-tcl-ir','-p','bpf-tcl-codegen','--lib','--no-run','--message-format=json'],out,'build');print('build exit',record['build']['exit_code'],flush=True)
artifacts=[];binaries={}
if record['build']['exit_code']==0:
 for line in (out/'build.stdout').read_text().splitlines():
  try:a=json.loads(line)
  except json.JSONDecodeError:continue
  if a.get('reason')=='compiler-artifact':
   artifacts.append(a)
   if a.get('executable'):binaries[a['target']['name']]=a['executable']
 (out/'artifacts.json').write_text(json.dumps(artifacts,indent=2)+'\n');record['executables']={name:{'path':p,'sha256_before':c.digest(pathlib.Path(p))} for name,p in binaries.items()};print('HEAVY LINK COMPLETE',flush=True)
 for name,binary in binaries.items():
  print('running',name,flush=True);r=c.command_record([binary,'--nocapture','--test-threads=1'],out,name,900);r['executed_test_count']=sum(int(x) for x in re.findall(r'^running (\d+) tests?$',(out/r['stdout']).read_text(),re.M));record['tests'].append(r);print(name,'exit',r['exit_code'],'count',r['executed_test_count'],flush=True)
 for info in record['executables'].values():info['sha256_after']=c.digest(pathlib.Path(info['path']))
after=c.static_inputs(packages,targets);(out/'inputs-after.json').write_text(json.dumps(after,indent=2)+'\n');record['unchanged_preexisting_inputs']=all(after.get(k)==v for k,v in before.items());record['changed_preexisting_inputs']={k:{'before':v,'after':after.get(k)} for k,v in before.items() if after.get(k)!=v};(out/'selected-local-output-coverage.json').write_text(json.dumps(audit.selected_output_coverage(c,artifacts,packages,before,after),indent=2)+'\n');record['finished_utc']=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime());(out/'validation.json').write_text(json.dumps(record,indent=2)+'\n');print('capture complete',record['unchanged_preexisting_inputs'],flush=True)
