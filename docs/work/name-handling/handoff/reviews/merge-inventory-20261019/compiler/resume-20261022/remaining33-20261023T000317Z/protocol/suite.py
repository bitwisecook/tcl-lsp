from pathlib import Path
import datetime,fcntl,hashlib,json,os,re,shlex,shutil,subprocess,sys,time
root=Path('/workspace/naming-migration-active/tcl-lsp'); os.chdir(root)
plan_path=Path(sys.argv[1]); plan=json.loads(plan_path.read_text()); rows=plan['required_filters']
if len(rows)!=plan['expected_filters'] or sum(r['expected_selected'] for r in rows)!=plan['expected_selected']: raise SystemExit('STOP: plan count mismatch')
if not rows: raise SystemExit('STOP: explicit narrow compiler filters required')
lock=open('/tmp/tcl-lsp-naming-build.lock','w'); fcntl.flock(lock,fcntl.LOCK_EX)
tag=datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
receipt=Path('/tmp/compiler-postbatch-remaining33-v1-runs')/tag; receipt.mkdir(parents=True)
(receipt/'plan.json').write_text(json.dumps(plan,indent=2)+'\n')
def digest(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def source_manifest():
    records={}
    raw=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard','-z'])
    for item in raw.split(b'\0'):
        if not item:continue
        path=Path(os.fsdecode(item))
        if (path.parts[0] in {'rust','runtime','tests','experiments','spec','specs','data','resources','.cargo'}
            or path.suffix in {'.rs','.c','.h','.tcl','.tclspec'}
            or path.name in {'Cargo.toml','Cargo.lock','rust-toolchain.toml','rustfmt.toml','clippy.toml'}):
            records[str(path)]=digest(path) if path.is_file() else None
    return records
def write_json(path,data):path.write_text(json.dumps(data,sort_keys=True,indent=2)+'\n')
before=source_manifest(); write_json(receipt/'source-before.json',before)
subprocess.run([sys.executable,'/tmp/compiler-c6-provider-manifest.py',str(receipt/'providers-before.json')],check=True)
providers=json.loads((receipt/'providers-before.json').read_text())
(receipt/'head-before.txt').write_bytes(subprocess.check_output(['git','rev-parse','HEAD']))
(receipt/'current-rust-diff.patch').write_bytes(subprocess.check_output(['git','diff','--binary','HEAD','--','rust','Cargo.toml','Cargo.lock']))
write_json(receipt/'build-environment.json',{k:v for k,v in os.environ.items() if k.startswith(('CARGO_','RUSTUP_','TCL_LSP_')) or k in {'RUSTFLAGS','RUSTDOCFLAGS','RUSTC','RUSTDOC','WASI_SDK_PATH'}})
runner_files=['/tmp/tcl-lsp-naming-env.sh','/tmp/compiler-postbatch-remaining33-v1/run.sh','/tmp/compiler-postbatch-remaining33-v1/suite.py','/tmp/compiler-postbatch-remaining33-v1/guarded-cargo.py','/tmp/compiler-c6-provider-manifest.py',str(plan_path)]
write_json(receipt/'runner-inputs.json',{p:digest(Path(p)) for p in runner_files})
results=[]; overall=0; original_artifact=None
print('START',tag,'receipt='+str(receipt),'source_inputs='+str(len(before)),flush=True)
for row in rows:
    selector=row['selector']; expected=row['expected_selected']
    if not selector or selector=='--' or expected<1:raise SystemExit('STOP: invalid/zero test selector')
    if shutil.disk_usage('/workspace').free<2*1024**3:raise SystemExit('STOP: disk below2GiB')
    label=row['label']; log=receipt/(label+'.log')
    argv=['cargo','test','--locked','-p','tcl-compiler','--lib',selector,'--']
    if row.get('exact'):argv.append('--exact')
    argv+=['--nocapture','--test-threads=1']
    env=os.environ.copy(); env.update(row.get('env',{}))
    (Path(str(log)+'.command')).write_text(shlex.join(argv)+'\n')
    write_json(Path(str(log)+'.env.json'),row.get('env',{}))
    print('RUN',label,selector,flush=True)
    result=subprocess.run([sys.executable,'/tmp/compiler-postbatch-remaining33-v1/guarded-cargo.py',str(log),*argv],env=env,check=False)
    text=log.read_text(errors='replace')
    artifacts=[]
    for name in dict.fromkeys(re.findall(r'\(([^()\s]*/tcl_compiler-[^()\s]+)\)',text)):
        artifact=Path(name)
        if artifact.is_file():
            stat=artifact.stat()
            artifacts.append({'path':str(artifact.resolve()),'sha256':digest(artifact),'mtime_utc':datetime.datetime.fromtimestamp(stat.st_mtime,datetime.timezone.utc).isoformat(),'age_seconds':time.time()-stat.st_mtime,'retained':True})
    write_json(Path(str(log)+'.artifact.json'),artifacts)
    artifact_identity = [(item['path'], item['sha256']) for item in artifacts]
    if original_artifact is None:
        original_artifact = artifact_identity
    artifact_stable = len(artifact_identity) == 1 and artifact_identity == original_artifact
    after=source_manifest(); write_json(Path(str(log)+'.source-after.json'),after)
    subprocess.run([sys.executable,'/tmp/compiler-c6-provider-manifest.py',str(receipt/'providers-after.json')],check=True)
    provider_after=json.loads((receipt/'providers-after.json').read_text())
    source_stable=before==after; providers_stable=providers==provider_after
    (receipt/'head-after.txt').write_bytes(subprocess.check_output(['git','rev-parse','HEAD']))
    summaries=re.findall(r'^test result: (.+?)\. (\d+) passed; (\d+) failed; (\d+) ignored;',text,re.M)
    selected=sum(int(passed)+int(failed) for _,passed,failed,_ in summaries)
    warnings=bool(re.search(r'^warning',text,re.M)); at_cap=Path(str(log)+'.process.json').is_file() and json.loads(Path(str(log)+'.process.json').read_text())['log_at_cap']
    entry={'label':label,'selector':selector,'command':argv,'cargo_wrapper_exit':result.returncode,'expected_selected':expected,'actual_selected':selected,'summaries':summaries,'warnings':warnings,'source_stable':source_stable,'providers_stable':providers_stable,'log_at_cap':at_cap,'artifacts':artifacts,'artifact_stable':artifact_stable}
    if row.get('diagnostic_only'):
        entry['diagnostic_only']=True
        entry['zz_result_rows']=re.findall(r'^ZZ_RESULT.*',text,re.M)
        entry['report_observations']=re.findall(r'^REPORT_.*',text,re.M)
        entry['requires_separate_fact_review']=True
    write_json(Path(str(log)+'.result.json'),entry); results.append(entry)
    write_json(receipt/'results.json',results)
    write_json(Path(str(log)+'.sha256.json'),{str(p):digest(p) for p in receipt.glob(label+'.log*') if p.is_file() and p.name!=label+'.log.sha256.json'})
    print('RESULT',label,'cargo='+str(result.returncode),'selected='+str(selected),'expected='+str(expected),'source_stable='+str(source_stable),'providers_stable='+str(providers_stable),flush=True)
    for summary in summaries:print('TEST_RESULT',summary,flush=True)
    if not source_stable or not providers_stable:raise SystemExit('STOP: source/provider drift')
    if not artifact_stable:raise SystemExit('STOP: missing/multiple/changed compiler test artifact')
    if selected != expected:raise SystemExit('STOP: unexpected selected count')
    if at_cap:raise SystemExit('STOP: log at20MiB cap')
    if result.returncode or selected!=expected or warnings or at_cap:overall=1
    if not summaries:raise SystemExit('STOP: build/runner failed before a matching test summary')
if len(results)!=plan['expected_filters'] or sum(r['actual_selected'] for r in results)!=plan['expected_selected']: raise SystemExit('STOP: terminal count mismatch')
write_json(receipt/'final.json',{'overall_exit':overall,'results':len(results),'finished_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'source_count':len(before),'providers_stable':True,'source_stable':True,'all_artifacts_retained':True})
print('COMPLETE','receipt='+str(receipt),'overall='+str(overall),flush=True)
fcntl.flock(lock,fcntl.LOCK_UN); lock.close()
print('LOCK_RELEASED',datetime.datetime.now(datetime.timezone.utc).isoformat(),flush=True)
sys.exit(overall)
