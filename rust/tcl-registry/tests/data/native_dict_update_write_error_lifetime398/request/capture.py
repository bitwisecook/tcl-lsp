from pathlib import Path
import concurrent.futures,hashlib,json,shutil,subprocess,time
request_dir=Path(__file__).resolve().parent
request_path=request_dir/'request.json';request=json.loads(request_path.read_bytes())
output=Path(request['output_directory']);output.mkdir(exist_ok=False)
sha=lambda b:hashlib.sha256(b).hexdigest()
launcher_sha=sha(Path(__file__).read_bytes());request_sha=sha(request_path.read_bytes())
def verify():
 for p,s in request['required_sha256'].items():
  if sha(Path(p).read_bytes())!=s:raise RuntimeError('changed required input '+p)
verify()
def provider(item):
 version=item['version'];sdk=Path(item['sdk']);archive=Path(item['archive']);r=output/version;r.mkdir();receipts=[]
 def run(label,command,cwd=None,timeout=60,required_success=True):
  verify();start=time.monotonic()
  try:
   process=subprocess.run(command,cwd=cwd,capture_output=True,timeout=timeout)
   code=process.returncode;stdout=process.stdout;stderr=process.stderr;timed_out=False
  except subprocess.TimeoutExpired as exc:
   code=None;stdout=exc.stdout or b'';stderr=exc.stderr or b'';timed_out=True
  (r/(label+'.stdout')).write_bytes(stdout);(r/(label+'.stderr')).write_bytes(stderr)
  row={'label':label,'version':version,'command':command,'cwd':str(cwd) if cwd else None,'exit':code,'timeout':timed_out,'seconds':time.monotonic()-start,'stdout_sha256':sha(stdout),'stderr_sha256':sha(stderr),'request_sha256':request_sha,'launcher_sha256':launcher_sha,'required_sha256':request['required_sha256'],'scope':request['purpose']}
  row['artifacts']={str(p):sha(p.read_bytes()) for p in r.iterdir() if p.is_file() and (p.suffix in ['.o','.a'] or p.name in ['baseline-probe','observed-probe','version-probe'])}
  (r/(label+'.receipt.json')).write_text(json.dumps(row,indent=2)+'\n');receipts.append(row)
  print(version,label,'exit',code,'stdout',len(stdout),'stderr',len(stderr),flush=True)
  verify()
  if required_success and (code!=0 or timed_out):raise RuntimeError(version+' '+label+' failed; original streams retained')
  return row
 compiler=['cc','-I'+str(sdk/'unix'),'-I'+str(sdk/'generic')]
 libraries=['-lm','-ldl','-lpthread','-lz']
 version_source=r/'version.c';version_source.write_text('#include <stdio.h>\n#include "tcl.h"\nint main(void){Tcl_FindExecutable("lifetime398");Tcl_Interp *i=Tcl_CreateInterp();int c=Tcl_Eval(i,"info patchlevel");printf("%d\\t%s\\n",c,Tcl_GetStringResult(i));Tcl_DeleteInterp(i);return c;}\n')
 run('version-compile',compiler+[str(version_source),str(archive)]+libraries+['-o',str(r/'version-probe')])
 version_row=run('version',[str(r/'version-probe')]);assert (r/'version.stdout').read_bytes()==('0\t'+version+'\n').encode()
 run('baseline-compile',compiler+[str(request_dir/'original-probe.c'),str(archive)]+libraries+['-o',str(r/'baseline-probe')])
 for index in request['cases']:run('baseline-case'+str(index),[str(r/'baseline-probe'),str(index)],required_success=False)
 makefile=r/'Makefile.observer';makefile.write_text('include '+str(sdk/'unix/Makefile')+'\nBUILD_DIR := '+str(sdk/'unix')+'\n.PHONY: observed-execute observed-object\nobserved-execute:\n\t$(CC) -c $(CC_SWITCHES) '+str(request_dir/version/'observed-tclExecute.c')+' -o '+str(r/'tclExecute.o')+'\nobserved-object:\n\t$(CC) -c $(CC_SWITCHES) '+str(request_dir/version/'observed-tclObj.c')+' -o '+str(r/'tclObj.o')+'\n')
 run('build-execute',['make','--no-print-directory','-f',str(makefile),'observed-execute'],cwd=sdk/'unix',timeout=120)
 run('build-object',['make','--no-print-directory','-f',str(makefile),'observed-object'],cwd=sdk/'unix',timeout=120)
 copied=r/archive.name;shutil.copyfile(archive,copied)
 (r/'archive-copy.json').write_text(json.dumps({'source':str(archive),'destination':str(copied),'source_sha256':sha(archive.read_bytes()),'copied_sha256':sha(copied.read_bytes())},indent=2)+'\n')
 run('build-archive',['ar','rcs',str(copied),str(r/'tclExecute.o'),str(r/'tclObj.o')])
 run('observed-compile',compiler+[str(request_dir/'observed-probe.c'),str(copied)]+libraries+['-o',str(r/'observed-probe')])
 for index in request['cases']:run('observed-case'+str(index),[str(r/'observed-probe'),str(index)],required_success=False)
 (r/'summary.json').write_text(json.dumps({'version':version,'request_sha256':request_sha,'launcher_sha256':launcher_sha,'commands':len(receipts),'original_source_unchanged':True,'separate_instrumented_archive':True,'exits':[x['exit'] for x in receipts]},indent=2)+'\n')
 return version
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as workers:
 futures=[workers.submit(provider,item) for item in request['versions']]
 failures=[]
 for future in concurrent.futures.as_completed(futures):
  try: print('provider complete',future.result(),flush=True)
  except Exception as exc:failures.append(str(exc));print('provider setup failure',str(exc),flush=True)
verify()
(output/'summary.json').write_text(json.dumps({'id':request['id'],'request_sha256':request_sha,'launcher_sha256':launcher_sha,'question':request['question'],'scope':request['purpose'],'setup_failures':failures},indent=2)+'\n')
if failures:raise SystemExit(1)
