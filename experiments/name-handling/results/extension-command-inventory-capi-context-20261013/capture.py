from pathlib import Path
import datetime,hashlib,json,os,re,subprocess
out=Path('/tmp/name-extension-command-inventory-20261013/original-capi-qualified-context')
root=Path('/workspace/tcl-lsp')
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
providers=[('c84','8.4.20'),('c85','8.5.19'),('c86','8.6.18'),('c90','9.0.4'),('c91','9.1.0')]
def seal(binary,library):
 p=subprocess.run(['ldd',str(binary)],capture_output=True,timeout=15)
 paths=set(re.findall(r'(/[A-Za-z0-9_./+-]+)',p.stdout.decode(errors='replace')))
 return {'binary_path':str(binary),'binary_sha256':sha(binary),'ldd_exit':p.returncode,'ldd_stdout':p.stdout.decode(errors='replace'),'ldd_stderr':p.stderr.decode(errors='replace'),'linked_files':{v:sha(v) for v in sorted(paths) if Path(v).is_file()},'library_root':str(library) if library else None,'library_files':{str(v.relative_to(library)):sha(v) for v in sorted(library.rglob('*')) if v.is_file()} if library else {}}
records=[]
for name,version in providers:
 src=root/'tmp'/('tcl'+version if name!='jim' else version)
 binary=src/('unix/tclsh' if name!='jim' else 'jimsh')
 library=src/'library' if name!='jim' else None
 env=os.environ.copy()
 for key in ['TCL_LIBRARY','TCLLIBPATH','LD_LIBRARY_PATH']:env.pop(key,None)
 selected={}
 if library:
  selected={'TCL_LIBRARY':str(library),'LD_LIBRARY_PATH':str(src/'unix')};env.update(selected)
 dest=out/name;dest.mkdir(exist_ok=False)
 cc=Path('/usr/bin/cc').resolve()
 cc_before=sha(cc)
 command=[str(cc),'-shared','-fPIC','-I'+str(src/'generic'),str(out/'probe.c'),'-o',str(dest/'selected.so')]
 built=subprocess.run(command,capture_output=True,timeout=30)
 (dest/'build.stdout').write_bytes(built.stdout);(dest/'build.stderr').write_bytes(built.stderr)
 assert built.returncode==0, built.stderr.decode()
 (dest/'input.tcl').write_bytes((out/'input.tcl').read_bytes())
 artifact_before=sha(dest/'selected.so')
 before=seal(binary,library);(dest/'seal-before.json').write_text(json.dumps(before,indent=2)+'\n')
 run=subprocess.run([str(binary),str(dest/'input.tcl')],cwd=dest,env=env,capture_output=True,timeout=20)
 (dest/'stdout.bin').write_bytes(run.stdout);(dest/'stderr.bin').write_bytes(run.stderr)
 after=seal(binary,library); assert sha(cc)==cc_before and sha(dest/'selected.so')==artifact_before;(dest/'seal-after.json').write_text(json.dumps(after,indent=2)+'\n')
 changes={key:[before[key],after[key]] for key in ['binary_sha256','linked_files','library_files'] if before[key]!=after[key]}
 rec={'provider':name,'expected_release':version if name!='jim' else 'selected latest Jim source checkout','captured_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'command':[str(binary),str(out/'input.tcl')],'cwd':str(out),'selected_environment':selected,'unset_environment':['TCL_LIBRARY','TCLLIBPATH','LD_LIBRARY_PATH'] if name=='jim' else ['TCLLIBPATH'],'input_sha256':sha(out/'input.tcl'),'runner_sha256':sha(out/'capture.py'),'exit_code':run.returncode,'stdout_sha256':sha(dest/'stdout.bin'),'stderr_sha256':sha(dest/'stderr.bin'),'seal_changes':changes,'original_CAPI_artifact':{'build_command':command,'compiler_path':str(cc),'compiler_sha256':cc_before,'source_sha256':sha(out/'probe.c'),'artifact_sha256':artifact_before,'build_exit':built.returncode},'scope':'Original C5 API callback publishes original bare x inside actual N: context; Rust Runtime/Engine controls remain authored/unexecuted. No Jim/F5/tcltest identity claim.'}
 (dest/'provenance.json').write_text(json.dumps(rec,indent=2)+'\n');records.append(rec)
 print(name,'exit',run.returncode,'seal changes',len(changes));print('raw stdout bytes',len(run.stdout),'stderr bytes',len(run.stderr))
(out/'records.json').write_text(json.dumps(records,indent=2)+'\n')
