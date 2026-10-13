import pathlib,json,hashlib,os,subprocess,time,shutil,importlib.util
ROOT=pathlib.Path('/workspace/name-implementation-worktrees/codegen')
def module(name,path):
 spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
c=module('capture','/tmp/name-codegen-isolated-capture.py');c.ROOT=ROOT
audit=module('audit','/tmp/name-codegen-selected-output-audit.py')
out=ROOT/'experiments/name-handling/results/codegen-structured-procedure-slots/typecheck-06';out.mkdir(parents=True,exist_ok=False)
packages=c.local_packages(['/tmp/name-codegen-isolated-metadata.json','/tmp/name-codegen-isolated-runtime-metadata.json']);targets=[os.environ['CARGO_TARGET_DIR']]
record={'git_head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'started_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'scope':'Production typecheck of Compiler lib and genuine selected local dependency inputs; no executed Compiler tests, Native entry, Runtime/WASM or oracle acceptance implied. No temporary instrumentation.','environment':{k:os.environ.get(k) for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS','CARGO_PROFILE_DEV_DEBUG','CARGO_PROFILE_TEST_DEBUG','CARGO_INCREMENTAL','CARGO_NET_OFFLINE']},'tools':{}}
for tool in ['rustc','cargo']:
 path=pathlib.Path(subprocess.check_output(['rustup','which',tool],text=True).strip());r=subprocess.run([str(path),'--version'],capture_output=True);record['tools'][tool]={'path':str(path.resolve()),'sha256':c.digest(path.resolve()),'exit_code':r.returncode,'stdout':r.stdout.decode(),'stderr':r.stderr.decode()}
for name,path in [('capture.py','/tmp/name-codegen-isolated-capture.py'),('selected-output-audit.py','/tmp/name-codegen-selected-output-audit.py'),('run.py',__file__),('workspace-metadata.json','/tmp/name-codegen-isolated-metadata.json'),('runtime-metadata.json','/tmp/name-codegen-isolated-runtime-metadata.json')]:shutil.copyfile(path,out/name)
before=c.static_inputs(packages,targets);(out/'inputs-before.json').write_text(json.dumps(before,indent=2)+'\n');print('before',len(before),flush=True)
record['build']=c.command_record(['cargo','check','--offline','-p','tcl-compiler','--lib','--message-format=json'],out,'build');print('build exit',record['build']['exit_code'],flush=True)
artifacts=[]
for line in (out/'build.stdout').read_text().splitlines():
 try:a=json.loads(line)
 except json.JSONDecodeError:continue
 if a.get('reason')=='compiler-artifact':artifacts.append(a)
(out/'artifacts.json').write_text(json.dumps(artifacts,indent=2)+'\n')
after=c.static_inputs(packages,targets);(out/'inputs-after.json').write_text(json.dumps(after,indent=2)+'\n');record['unchanged_preexisting_inputs']=all(after.get(k)==v for k,v in before.items());record['changed_preexisting_inputs']={k:{'before':v,'after':after.get(k)} for k,v in before.items() if after.get(k)!=v};(out/'selected-local-output-coverage.json').write_text(json.dumps(audit.selected_output_coverage(c,artifacts,packages,before,after),indent=2)+'\n');record['finished_utc']=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime());(out/'validation.json').write_text(json.dumps(record,indent=2)+'\n');print('capture complete',record['unchanged_preexisting_inputs'],flush=True)
