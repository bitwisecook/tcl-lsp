from pathlib import Path
import datetime,fcntl,hashlib,json,os,re,shlex,shutil,subprocess,sys,time
root=Path('/workspace/naming-migration-active/tcl-lsp'); os.chdir(root)
plan_path=Path(sys.argv[1]); plan=json.loads(plan_path.read_text()); rows=plan['jobs']
if not rows: raise SystemExit('STOP: explicit target jobs required')
lock=open('/tmp/tcl-lsp-naming-build.lock','w'); fcntl.flock(lock,fcntl.LOCK_EX)
tag=datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
receipt=Path('/tmp/ccg-postbatch-functional')/tag; receipt.mkdir(parents=True)
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
def check_runner_and_oracle_pins():
    for record in plan['runner_inputs']:
        if digest(Path(record['path']))!=record['sha256']:raise SystemExit('STOP: runner/provider setup input changed')
    oracle=json.loads(Path('/tmp/naming-oracle-setup.json').read_text())
    for row in [*oracle['tcl'],oracle['jim']]:
        binary=Path(row['interpreter'])
        if not binary.is_file() or digest(binary)!=row['interpreter_sha256']:raise SystemExit('STOP: pinned oracle binary mismatch')
check_runner_and_oracle_pins()
before=source_manifest(); write_json(receipt/'source-before.json',before)
subprocess.run([sys.executable,'/tmp/compiler-c6-provider-manifest.py',str(receipt/'providers-before.json')],check=True)
providers=json.loads((receipt/'providers-before.json').read_text())
(receipt/'head-before.txt').write_bytes(subprocess.check_output(['git','rev-parse','HEAD']))
(receipt/'current-rust-diff.patch').write_bytes(subprocess.check_output(['git','diff','--binary','HEAD','--','rust','Cargo.toml','Cargo.lock']))
safe_keys={'CARGO_TARGET_DIR','CARGO_BUILD_JOBS','CARGO_INCREMENTAL','CARGO_PROFILE_TEST_DEBUG','CARGO_PROFILE_DEV_DEBUG','CARGO_TERM_COLOR','RUSTUP_HOME','RUSTUP_TOOLCHAIN','TCL_LSP_REQUIRE_TCLSH','TCL_LSP_TCLSH84','TCL_LSP_TCLSH85','TCL_LSP_TCLSH86','TCL_LSP_TCLSH90','TCL_LSP_TCLSH91','TCL_LSP_JIMSH','WASI_SDK_PATH'}
write_json(receipt/'build-environment.json',{k:os.environ[k] for k in safe_keys if k in os.environ})
runner_files=['/tmp/tcl-lsp-naming-env.sh','/tmp/ccg-postbatch-functional-run.sh','/tmp/ccg-postbatch-functional-suite.py','/tmp/compiler-c6-guarded-cargo.py','/tmp/compiler-c6-provider-manifest.py','/tmp/naming-oracle-setup.json',str(plan_path)]
write_json(receipt/'runner-inputs.json',{p:digest(Path(p)) for p in runner_files})
results=[]; overall=0
print('START',tag,'receipt='+str(receipt),'source_inputs='+str(len(before)),flush=True)
for row in rows:
    check_runner_and_oracle_pins()
    selector=row.get('selector') or row['target']; expected=row['expected_selected']
    if expected<1:raise SystemExit('STOP: invalid/zero test selector')
    for source in plan['test_inventory_sources']:
        if digest(root/source['path'])!=source['sha256']:raise SystemExit('STOP: test inventory source changed; refresh plan')
    if shutil.disk_usage('/workspace').free<2*1024**3:raise SystemExit('STOP: disk below2GiB')
    label=row['label']; log=receipt/(label+'.log')
    argv=row['cargo_argv']
    if argv[:3]!=['cargo','test','--locked'] or row['package']=='tcl-compiler':raise SystemExit('STOP: unexpected target/whole compiler test command')
    env=os.environ.copy(); env.update(row.get('env',{}))
    (Path(str(log)+'.command')).write_text(shlex.join(argv)+'\n')
    write_json(Path(str(log)+'.env.json'),row.get('env',{}))
    print('RUN',label,selector,flush=True)
    result=subprocess.run([sys.executable,'/tmp/compiler-c6-guarded-cargo.py',str(log),*argv],env=env,check=False)
    text=log.read_text(errors='replace')
    artifacts=[]
    stem=re.escape(row['artifact_stem'])
    for name in dict.fromkeys(re.findall(r'\(([^()\s]*/'+stem+r'-[^()\s]+)\)',text)):
        artifact=Path(name)
        if artifact.is_file():
            stat=artifact.stat()
            artifacts.append({'path':str(artifact.resolve()),'sha256':digest(artifact),'mtime_utc':datetime.datetime.fromtimestamp(stat.st_mtime,datetime.timezone.utc).isoformat(),'age_seconds':time.time()-stat.st_mtime,'retained':True})
    write_json(Path(str(log)+'.artifact.json'),artifacts)
    after=source_manifest(); write_json(Path(str(log)+'.source-after.json'),after)
    subprocess.run([sys.executable,'/tmp/compiler-c6-provider-manifest.py',str(receipt/'providers-after.json')],check=True)
    provider_after=json.loads((receipt/'providers-after.json').read_text())
    check_runner_and_oracle_pins()
    source_stable=before==after; providers_stable=providers==provider_after
    selected_names=sorted(set(re.findall(r'^test ([A-Za-z0-9_:]+) \.\.\.',text,re.M)))
    selectors_match=selected_names==sorted(row['selected_tests'])
    (receipt/'head-after.txt').write_bytes(subprocess.check_output(['git','rev-parse','HEAD']))
    summaries=re.findall(r'^test result: (.+?)\. (\d+) passed; (\d+) failed; (\d+) ignored;',text,re.M)
    selected=sum(int(passed)+int(failed) for _,passed,failed,_ in summaries)
    warnings=bool(re.search(r'^warning',text,re.M)); at_cap=Path(str(log)+'.process.json').is_file() and json.loads(Path(str(log)+'.process.json').read_text())['log_at_cap']
    entry={'label':label,'selector':selector,'command':argv,'cargo_wrapper_exit':result.returncode,'expected_selected':expected,'actual_selected':selected,'summaries':summaries,'warnings':warnings,'source_stable':source_stable,'providers_stable':providers_stable,'log_at_cap':at_cap,'artifacts':artifacts,'selected_names':selected_names,'selector_inventory_matches':selectors_match,'package':row['package'],'target':row['target']}
    if row.get('diagnostic_only'):
        entry['diagnostic_only']=True
        entry['zz_result_rows']=re.findall(r'^ZZ_RESULT.*',text,re.M)
        entry['requires_separate_fact_review']=True
    write_json(Path(str(log)+'.result.json'),entry); results.append(entry)
    write_json(receipt/'results.json',results)
    write_json(Path(str(log)+'.sha256.json'),{str(p):digest(p) for p in receipt.glob(label+'.log*') if p.is_file() and p.name!=label+'.log.sha256.json'})
    print('RESULT',label,'cargo='+str(result.returncode),'selected='+str(selected),'expected='+str(expected),'source_stable='+str(source_stable),'providers_stable='+str(providers_stable),flush=True)
    for summary in summaries:print('TEST_RESULT',summary,flush=True)
    if not source_stable or not providers_stable:raise SystemExit('STOP: source/provider drift')
    if result.returncode or selected!=expected or warnings or at_cap or not selectors_match or not artifacts:overall=1
    process_receipt=Path(str(log)+'.process.json')
    if process_receipt.is_file() and json.loads(process_receipt.read_text()).get('disk_abort'):raise SystemExit('STOP: disk watchdog aborted own command')
    if not summaries:raise SystemExit('STOP: build/runner failed before a matching test summary')
write_json(receipt/'final.json',{'overall_exit':overall,'results':len(results),'finished_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'source_count':len(before),'providers_stable':True,'source_stable':True,'all_artifacts_retained':True})
print('COMPLETE_CCG_POSTBATCH','receipt='+str(receipt),'overall='+str(overall),flush=True)
sys.exit(overall)
