#!/usr/bin/env python3
"""Root-only, finite real nextest reporter pilot; no product tests."""
from pathlib import Path
import argparse, datetime, fcntl, hashlib, importlib.util, json, os, re, shutil, subprocess, sys
BASE=Path('/tmp/ccg-nextest-real-pilot-v1')
ROOT=Path('/workspace/naming-migration-active/tcl-lsp')
def digest(p):
    with Path(p).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def unique(pairs):
    value={}
    for key,item in pairs:
        if key in value:raise ValueError('duplicate JSON key '+key)
        value[key]=item
    return value
def read(p):return json.loads(Path(p).read_text(),object_pairs_hook=unique)
def write(p,v):Path(p).write_text(json.dumps(v,indent=2,sort_keys=True)+'\n')
def utc():return datetime.datetime.now(datetime.timezone.utc).isoformat()
def source_manifest():
    records={}
    for item in subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard','-z'],cwd=ROOT).split(b'\0'):
        if not item:continue
        path=Path(os.fsdecode(item))
        if (path.parts[0] in {'rust','runtime','tests','experiments','spec','specs','data','resources','.cargo'} or path.suffix in {'.rs','.c','.h','.tcl','.tclspec'} or path.name in {'Cargo.toml','Cargo.lock','rust-toolchain.toml','rustfmt.toml','clippy.toml'}):
            records[str(path)]=digest(ROOT/path) if (ROOT/path).is_file() else None
    return records
parser=argparse.ArgumentParser();parser.add_argument('--go',type=Path,required=True);args=parser.parse_args()
if not args.go.is_file():raise SystemExit('STOP: root GO absent; no lock/Cargo/queue')
plan=read(BASE/'plan.json');manifest=read(BASE/'runner-input-manifest.json');go=read(args.go)
if go.get('purpose')!='nextest-real-pilot-v1' or go.get('plan_sha256')!=digest(BASE/'plan.json') or go.get('runner_input_manifest_sha256')!=digest(BASE/'runner-input-manifest.json'):raise SystemExit('STOP: exact root GO plan/runner pins required')
go_sha=digest(args.go)
def pins():
    if digest(args.go)!=go_sha:raise SystemExit('STOP: GO changed')
    selected=read('/tmp/ccg-selected-actual-tool-pins-20261022.json')
    override_names=('CC','AR','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER','CARGO_TARGET_WASM32_WASIP1_LINKER')
    if {k:os.environ[k] for k in override_names if k in os.environ}!=selected['overrides']:raise SystemExit('STOP: selected actual compiler/linker overrides changed')
    if any(shutil.which(k)!=v for k,v in selected['programs'].items()):raise SystemExit('STOP: selected actual compiler/linker PATH changed')
    for path,expected in manifest['inputs'].items():
        if digest(path)!=expected:raise SystemExit('STOP: runner/tool/fixture/provider input changed '+path)
pins();config=read(BASE/'build-environment.json')
for key in os.environ:
    if key in config['refused_environment_exact'] or any(key.startswith(p['prefix']) and key.endswith(p['suffix']) for p in config['refused_environment_patterns']):raise SystemExit('STOP: unsupported build override '+key)
for key,value in config['required_environment'].items():
    if os.environ.get(key)!=value:raise SystemExit('STOP: required build environment '+key)
if not os.environ.get('PATH','').startswith(config['path_prefix']+':'):raise SystemExit('STOP: tool PATH prefix')
if Path(sys.executable).resolve()!=Path(config['python_path']).resolve() or digest(sys.executable)!=config['python_sha256']:raise SystemExit('STOP: actual Python interpreter changed')
for name in ['cargo','rustc']:
    if digest(config[name+'_path'])!=config[name+'_sha256']:raise SystemExit('STOP: actual build tool changed')
lock=open('/tmp/tcl-lsp-naming-build.lock','w')
try:fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
except BlockingIOError:raise SystemExit('STOP: global build lock occupied; no queue')
receipt=BASE/'runs'/datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ');receipt.mkdir(parents=True)
write(receipt/'plan.json',plan);write(receipt/'go.json',go)
spec=importlib.util.spec_from_file_location('pilot_guarded_exec',BASE/'guarded-exec.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
def execute(label,argv,artifact,seconds):
    pins();prefix=receipt/label
    result=module.run(argv,cwd=ROOT,env=os.environ.copy(),stdout_path=str(prefix)+'.stdout',stderr_path=str(prefix)+'.stderr',receipt_path=str(prefix)+'.process.json',artifact_path=artifact,deadline_seconds=seconds,cap_bytes=20*1024**2,disk_floor_bytes=2*1024**3,disk_paths=('/workspace','/tmp',BASE/'target'))
    if not result.get('complete') or result.get('stop_reason') is not None or not result.get('group_empty_before_parent_reap') or not result.get('timer_armed_before_spawn'):raise RuntimeError('owned command lifecycle failed '+label)
    pins();return result,prefix
before=source_manifest();write(receipt/'source-before.json',before)
provider='/tmp/compiler-c6-provider-manifest.py';subprocess.run([sys.executable,provider,str(receipt/'providers-before.json')],check=True)
providers=read(receipt/'providers-before.json');seal=read(BASE/'provider-contract.json')
if digest(seal['path'])!=seal['sha256'] or providers!=read(seal['path']):raise SystemExit('STOP: actual provider seal differs')
head_before=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip();artifacts=[];final={'status':'FAIL','scope':'real nextest reporter tool pilot only','receipt':str(receipt)}
try:
    build,prefix=execute('01-actual-nextest-list',plan['commands'][0]['argv'],plan['tool']['path'],900)
    if build['exit_code']!=0:raise RuntimeError('nextest actual list/build failed')
    if re.search(r'^warning:',Path(str(prefix)+'.stderr').read_text(errors='replace'),re.M):raise RuntimeError('nextest/Cargo list warning')
    raw=Path(str(prefix)+'.stdout').read_bytes();selected=BASE/'actual-selected.json';selected.write_bytes(raw);(receipt/'actual-selected.json').write_bytes(raw)
    if selected.stat().st_size>16*1024**2:raise RuntimeError('actual roster cap exceeded')
    listing=read(selected);suites=listing.get('rust-suites',{});seen=[]
    for key,suite in suites.items():
        matches=[name for name,case in suite.get('testcases',{}).items() if case.get('filter-match',{}).get('status')=='matches' and case.get('ignored') is False]
        if not matches:continue
        if suite.get('package-name')!='nextest-capture-pilot' or suite.get('binary-name')!='nextest_capture_pilot' or suite.get('kind')!='lib' or suite.get('binary-id')!=key:raise RuntimeError('actual roster identity mismatch')
        seen.extend((key,name) for name in matches)
        exe=Path(suite['binary-path']).resolve()
        if not exe.is_relative_to((BASE/'target').resolve()) or not exe.is_file():raise RuntimeError('actual roster artifact containment')
        stat=exe.stat();artifacts.append({'binary_id':key,'path':str(exe),'sha256':digest(exe),'mtime_ns':stat.st_mtime_ns,'captured_from_actual_nextest_listing':True})
    if len(seen)!=2 or sorted(name for _,name in seen)!=plan['expected_roster']['selected'] or len(artifacts)!=1:raise RuntimeError('exact2 actual roster mismatch')
    write(receipt/'actual-artifacts.json',artifacts)
    # This is the original wrapper command, with only its output location made unique.
    argv=list(plan['commands'][1]['argv']);argv[0]=sys.executable;argv[argv.index('--output-dir')+1]=str(receipt/'actual-capture')
    run,prefix=execute('02-actual-reporter',argv,sys.executable,180)
    summary=read(receipt/'actual-capture/summary.json');terminal=[json.loads(line,object_pairs_hook=unique) for line in (receipt/'actual-capture/terminal.jsonl').read_text().splitlines() if line]
    if run['exit_code']==0 or summary.get('nextest_exit')!=run['exit_code']:raise RuntimeError('intentional failure rawexit not preserved')
    if summary.get('capture_errors') or summary.get('missing_terminal') or summary.get('selected_count')!=2 or summary.get('terminal_count')!=2:raise RuntimeError('actual capture terminal completeness')
    life=summary.get('lifecycle') or {}
    if not life.get('group_empty_before_parent_reap') or life.get('cleanup_signals'):raise RuntimeError('actual nested nextest lifecycle incomplete')
    if sorted((x['binary_id'],x['test']) for x in terminal)!=sorted(seen):raise RuntimeError('actual terminal identity mismatch')
    statuses={x['test']:x['status'] for x in terminal}
    if statuses!={'tests::actual_reporter_pass':'ok','tests::actual_reporter_failure':'failed'}:raise RuntimeError('actual pass/fail terminal mismatch')
    if summary.get('stderr_truncated'):raise RuntimeError('actual diagnostic cap exceeded')
    for item in artifacts:
        if digest(item['path'])!=item['sha256']:raise RuntimeError('artifact changed')
    final.update({'status':'PASS','tool_pilot_passed':True,'intentional_fixture_failure_exit':run['exit_code'],'selected':2,'terminal':2,'statuses':statuses,'nested_lifecycle':life,'artifacts':artifacts})
except Exception as error:
    final['failure']=str(error)
finally:
    after=source_manifest();write(receipt/'source-after.json',after)
    subprocess.run([sys.executable,provider,str(receipt/'providers-after.json')],check=True)
    final.update({'finished_utc':utc(),'source_stable':before==after,'providers_stable':providers==read(receipt/'providers-after.json'),'root_go_stable':digest(args.go)==go_sha,'head_before':head_before,'head_after':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'artifacts_retained':True})
    pins()
    if not final['source_stable'] or not final['providers_stable'] or not final['root_go_stable']:final['status']='FAIL'
    write(receipt/'final.json',final);print(json.dumps(final,sort_keys=True),flush=True)
    fcntl.flock(lock,fcntl.LOCK_UN)
raise SystemExit(0 if final['status']=='PASS' else 1)
