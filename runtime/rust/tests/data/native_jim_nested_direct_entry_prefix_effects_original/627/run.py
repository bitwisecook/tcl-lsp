from pathlib import Path
import hashlib,json,subprocess,time,os,concurrent.futures
p=Path(__file__).parent;repo=Path('/workspace/tcl-lsp');assert not (p/'results.json').exists();probe=p/'probe.tcl';sha=lambda x:hashlib.sha256(x.read_bytes()).hexdigest();providers=[]
for ver in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0']:
 root=repo/'tmp'/('tcl'+ver);providers.append((ver,root/'unix/tclsh',root/'library'))
providers.append(('Jim',Path('/workspace/.proofs/native-providers/jimtcl/jimsh'),None))
common={'PATH':'/usr/bin:/bin','LC_ALL':'C','LANG':'C'}
def run(row):
 name,exe,library=row;folder=p/name;folder.mkdir();env=dict(common)
 if library:env['TCL_LIBRARY']=str(library)
 inputs={str(probe):sha(probe),str(exe):sha(exe)}
 if library:
  inputs.update({str(x):sha(x) for x in sorted(library.rglob('*.tcl')) if x.is_file()})
 start=time.time();out=subprocess.run([str(exe),str(probe)],cwd=p,env=env,capture_output=True,timeout=30);elapsed=time.time()-start;(folder/'stdout').write_bytes(out.stdout);(folder/'stderr').write_bytes(out.stderr);unchanged=all(sha(Path(x))==h for x,h in inputs.items());record={'provider':name,'argv':[str(exe),str(probe)],'cwd':str(p),'environment':env,'exit':out.returncode,'seconds':elapsed,'before_source_and_executable_pins':inputs,'all_after_pins_match':unchanged,'stdout_sha256':sha(folder/'stdout'),'stderr_sha256':sha(folder/'stderr'),'limitations':'Measures info complete and actual Direct command-at-time evaluation of these seven counted byte source values only. No bytecode identity, Native lifetime/teardown, current Rust assertions, F5 or opaque-source inference.'};(folder/'receipt.json').write_text(json.dumps(record,indent=2)+'\n');assert unchanged;return {**{k:record[k] for k in ['provider','exit','seconds','all_after_pins_match']},'stdout':out.stdout.decode('utf-8'),'stderr':out.stderr.decode('utf-8')}
with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:results=list(pool.map(run,providers))
(p/'results.json').write_text(json.dumps({'question':'When the exact counted source has malformed or bare-brace nested scripts, does selected Direct evaluation enter its outer and child prefixes, and is info complete a distinct observation?','results':results,'probe_sha256':sha(probe),'cases_sha256':sha(p/'cases.json'),'limitations':'Direct observed queries/effects only; results cannot grant command binding/compiler purpose or Native callback-state equivalence.'},indent=2)+'\n')
for r in results:print(r['provider'],'exit',r['exit'],r['stdout'].strip().replace('\n',' | '),'stderr',r['stderr'])
