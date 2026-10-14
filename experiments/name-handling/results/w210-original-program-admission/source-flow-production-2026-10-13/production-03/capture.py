from pathlib import Path
import subprocess,json,hashlib,gzip,datetime,re,time,os
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-w210-source-flow/production-03');os.chdir(root/'rust')
def now():return datetime.datetime.now(datetime.timezone.utc).isoformat()
def inventory():
 files=set(subprocess.check_output(['rg','--files','--hidden',*[p for p in ['rust','runtime/rust','data'] if (root/p).exists()]],cwd=root,text=True).splitlines())
 files.update(['Cargo.toml','Cargo.lock','rust-toolchain.toml','experiments/name-handling/tcltest-shared-frame/probe.tcl'])
 for path in (root/'.cargo').rglob('*'):
  if path.is_file():files.add(str(path.relative_to(root)))
 files={f for f in files if (root/f).is_file()}
 return {f:hashlib.sha256((root/f).read_bytes()).hexdigest() for f in sorted(files)}
filters=['tcltest_', 'selected_formal_read_scope_', 'native_read_scope_requires_matching_complete_metadata', 'original_procedure_read_scope_preserves_continuation_occurrences','declaration_read_advice_requires_original_unchanged_body_and_formals','provider_read_domain_requires_the_same_current_complete_input','command_binding::declaration_flow::tests::']
command=['cargo','test','--offline','-j','2','-p','tcl-compiler','--lib','--',*filters,'--nocapture','--test-threads=1']
start=inventory();gzip.open(out/'source-before.json.gz','wt').write(json.dumps(start,sort_keys=True))
(out/'start.json').write_text(json.dumps({'started_utc':now(),'command':command,'target':os.environ.get('CARGO_TARGET_DIR'),'source_paths':len(start),'diagnostics_enabled':False},indent=2)+'\n')
print('Source snapshot complete: '+str(len(start))+' paths; starting focused Compiler capture.',flush=True)
clock=time.monotonic()
with (out/'compiler.log').open('wb') as log:
 process=subprocess.Popen(command,stdout=log,stderr=subprocess.STDOUT);code=process.wait()
end=inventory();gzip.open(out/'source-after.json.gz','wt').write(json.dumps(end,sort_keys=True))
changed={f:{'before':start.get(f),'after':end.get(f)} for f in set(start)|set(end) if start.get(f)!=end.get(f)}
log=(out/'compiler.log').read_text();matches=re.findall(r'test result: (\w+)\. (\d+) passed; (\d+) failed;',log)
result={'finished_utc':now(),'exit_code':code,'seconds':time.monotonic()-clock,'summaries':matches,'changed_sources':changed,'running_test_counts':re.findall(r'running (\d+) tests?',log)}
(out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
binaries=[]
for path in (root/os.environ['CARGO_TARGET_DIR']/'debug/deps').glob('tcl_compiler-*'):
 if path.is_file() and path.suffix=='' and os.access(path,os.X_OK):binaries.append({'path':str(path.relative_to(root)),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'bytes':path.stat().st_size})
(out/'binary-inventory.json').write_text(json.dumps(binaries,indent=2)+'\n')
executed=[]
for name in re.findall(r'Running unittests [^\n(]+\(([^)]+)\)',log):
 path=Path(name)
 if not path.is_absolute():path=root/path
 if path.is_file():executed.append({'path':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'bytes':path.stat().st_size})
result['executed_test_binaries']=executed
result['tested']=bool(matches)
(out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
(out/'executed-binary.json').write_text(json.dumps(executed,indent=2)+'\n')
print(json.dumps(result),flush=True)
print(log[-8500:],flush=True)
