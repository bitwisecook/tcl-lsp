from pathlib import Path
import datetime, hashlib, json, gzip, os, shutil, signal, subprocess, tarfile, time, traceback
scratch=Path('/tmp/name-vm-same-position-diagnostic-20261013')
plan=json.loads((scratch/'plan.json').read_text());root=Path(plan['root'])
out=root/'experiments/name-handling/results/vm-oo-root-namespace-creation/questions/same-position-first-cause-probe/capture-01'
out.mkdir(parents=True,exist_ok=False)
def utc():return datetime.datetime.now(datetime.timezone.utc).isoformat()
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def write(name,data): (out/name).write_text(json.dumps(data,indent=2)+'\n')
crate_dirs=[(root/p).parent for p in plan['conservative_path_dependency_manifests']]
paths=set()
for d in crate_dirs:
 for p in d.rglob('*'):
  if p.is_file() and not {'target','.git','node_modules','__pycache__'}.intersection(p.relative_to(d).parts):paths.add(p)
for rel in ['Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo/config.toml']:
 p=root/rel
 if p.is_file():paths.add(p)
paths=sorted(paths)
def snapshot():return {str(p.relative_to(root)):digest(p) for p in paths}
def archive(name):
 with tarfile.open(out/name,'w:gz') as tar:
  for p in paths:tar.add(p,arcname=str(p.relative_to(root)),recursive=False)
def execute(argv,stem,env=None,timeout=1800):
 started=utc();print('Started '+stem,flush=True)
 with (out/(stem+'.stdout')).open('wb') as stdout,(out/(stem+'.stderr')).open('wb') as stderr:
  proc=subprocess.Popen(argv,cwd=root,env=env,stdout=stdout,stderr=stderr,start_new_session=True)
  try:code=proc.wait(timeout=timeout)
  except BaseException:
   os.killpg(proc.pid,signal.SIGTERM)
   try:proc.wait(timeout=5)
   except subprocess.TimeoutExpired:os.killpg(proc.pid,signal.SIGKILL);proc.wait()
   raise
 record={'argv':argv,'started':started,'finished':utc(),'exit_code':code,'stdout_sha256':digest(out/(stem+'.stdout')),'stderr_sha256':digest(out/(stem+'.stderr'))}
 write(stem+'.json',record);print('Finished '+stem+' exit '+str(code),flush=True);return record
record={'started':utc(),'status':'PREPARING','plan':plan,'environment':'Inherited authorised environment from /tmp/name-env.sh; explicit issues target and named synthetic diagnostic gate only; no environment dump.','disk_before':shutil.disk_usage(root)._asdict(),'source_scope':'15 recursively authored path manifest packages plus root build configs. Conservative whole-package file envelope, not compiler-read trace. Standalone Runtime Clock/fs are excluded. Actual Cargo artifacts and VM rustc dep-info retained separately.'}
write('execution.json',record)
before=snapshot();write('source-before.json',before)
for o in plan['owners']:
 if digest(root/o['path'])!=o['before_sha256']:raise RuntimeError('Owner preimage changed: '+o['path'])
for rel,h in plan['conservative_path_dependency_manifests'].items():
 if digest(root/rel)!=h:raise RuntimeError('Manifest changed: '+rel)
for name in ['plan.json','instrumentation.patch','run.py']:shutil.copyfile(scratch/name,out/name)
applied=[]
try:
 for o in plan['owners']:
  gzip.open(out/(o['path'].replace('/','--')+'.original.gz'),'wb').write((root/o['path']).read_bytes())
  (root/o['path']).write_bytes(Path(o['instrumented_path']).read_bytes());applied.append(o)
 instrumented=snapshot();write('source-instrumented-before.json',instrumented);archive('conservative-instrumented-source.tar.gz')
 record['disk_after_source_capture']=shutil.disk_usage(root)._asdict()
 print('Source capture '+str(len(paths))+' files; free '+str(shutil.disk_usage(root).free),flush=True)
 build=execute(plan['build_argv'],'vm-build',timeout=2400);record['build']=build
 current=snapshot();write('source-instrumented-after-build.json',current)
 record['during_build_changed_sources']=[p for p in instrumented if instrumented[p]!=current[p]]
 if build['exit_code']!=0:
  text=(out/'vm-build.stdout').read_text(errors='replace')+(out/'vm-build.stderr').read_text(errors='replace')
  record['status']='BUILD_FAILED';record['enospc_observed']='No space left on device' in text
 elif record['during_build_changed_sources']:
  record['status']='SOURCE_CURRENCY_CHANGED_NO_CONTROL_RUN'
 else:
  artifacts=[];binaries=[]
  for line in (out/'vm-build.stdout').read_text().splitlines():
   try:msg=json.loads(line)
   except json.JSONDecodeError:continue
   if msg.get('reason')=='compiler-artifact':
    artifacts.append(msg)
    if msg.get('target',{}).get('name')=='tcl_vm' and msg.get('profile',{}).get('test') and msg.get('executable'):binaries.append(msg['executable'])
  write('artifact-graph.json',artifacts)
  if len(set(binaries))!=1:raise RuntimeError('Expected exactly one emitted VM test binary: '+repr(binaries))
  binary=Path(binaries[0]);binaryhash=digest(binary)
  record['binary']={'path':str(binary),'before_sha256':binaryhash}
  info=binary.with_suffix('.d')
  if info.exists():shutil.copyfile(info,out/'vm-rustc-depinfo.d')
  else:record['depinfo_missing']=str(info)
  listing=execute([str(binary),'--exact',plan['control'],'--list'],'control-list',timeout=60);record['list']=listing
  lines=(out/'control-list.stdout').read_text().splitlines()
  if listing['exit_code']!=0 or lines.count(plan['control']+': test')!=1:raise RuntimeError('Exact native name not confirmed once')
  frozen=snapshot();write('source-before-control.json',frozen)
  if frozen!=instrumented:raise RuntimeError('Source changed before exact control')
  env=os.environ.copy();env[plan['environment_gate']]='1';env['RUST_BACKTRACE']='1'
  test=execute([str(binary),'--exact',plan['control'],'--test-threads=1','--nocapture'],'control',env=env,timeout=90);record['control']=test
  record['binary']['after_sha256']=digest(binary)
  after=snapshot();write('source-instrumented-after-control.json',after)
  record['during_control_changed_sources']=[p for p in instrumented if instrumented[p]!=after[p]]
  with binary.open('rb') as src,gzip.open(out/'vm-test-binary.gz','wb') as dest:shutil.copyfileobj(src,dest)
  record['status']='DIAGNOSTIC_COMPLETED_EXPECTED_ORIGINAL_ASSERTION_FAILURE' if test['exit_code']==101 else 'DIAGNOSTIC_COMPLETED_EXIT_'+str(test['exit_code'])
except BaseException as e:
 record['exception']=repr(e);record['traceback']=traceback.format_exc();record['status']='CAPTURE_FAILED'
 print('Capture failure '+repr(e),flush=True)
finally:
 restored={}
 for o in reversed(applied):
  p=root/o['path'];restored[o['path']]={'instrumented_before_restore_sha256':digest(p)}
  if digest(p)!=o['instrumented_sha256']:
   restored[o['path']]['concurrent_edit_refusal']=True
   print('Refusing to overwrite concurrent owner edit '+o['path'],flush=True)
  else:
   p.write_bytes(Path(o['before_path']).read_bytes());restored[o['path']]['restored_sha256']=digest(p);restored[o['path']]['matches_original']=digest(p)==o['before_sha256']
 record['restoration']=restored;record['finished']=utc();record['disk_after']=shutil.disk_usage(root)._asdict();record['diagnostic_binary_currency']='NONCURRENT_FOR_RESTORED_SOURCE; no reuse allowed'
 final=snapshot();write('source-restored-after.json',final);record['original_to_restored_changed_sources']=[p for p in before if before[p]!=final[p]]
 write('execution.json',record)
 print(json.dumps({'status':record['status'],'restored':restored,'changed_sources':record['original_to_restored_changed_sources'],'result_dir':str(out),'disk_free':record['disk_after']['free']},indent=2),flush=True)
