#!/usr/bin/env python3
"""Fixed GetDateFields source/value/version questions; no runtime build."""
from pathlib import Path
import hashlib,json,os,resource,subprocess,tempfile,time
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[4];OUT=HERE/'capture-01';OUT.mkdir(exist_ok=False)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def limits():
 resource.setrlimit(resource.RLIMIT_AS,(1024**3,1024**3));resource.setrlimit(resource.RLIMIT_CORE,(0,0))
records=[];paths=[HERE/p for p in ['control.c','object-control.c','input.tcl','cases.json','capture.py']]
for version in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0']:
 tree=ROOT/('tmp/tcl'+version)
 paths.extend([tree/'generic/tclClock.c',tree/'library/init.tcl',tree/'unix/tclsh',tree/('unix/libtcl'+'.'.join(version.split('.')[:2])+'.a')])
 if (tree/'library/clock.tcl').exists():paths.append(tree/'library/clock.tcl')
runner=ROOT/'tmp/name-target-runtime/debug/examples/run_script';jim=ROOT/'tmp/jimtcl-oracle/jimsh'
paths.extend([runner,jim,ROOT/'rust/tcl-test-support/jim-reference.txt',ROOT/'rust/tcl-syntax/src/native_string_trim.rs'])
before={str(p):sha(p) for p in paths}
def execute(name,argv,library=None):
 dest=OUT/name;dest.mkdir(parents=True);env=dict(os.environb)
 for key in [b'TCLLIBPATH',b'TCL_BACKEND_CONSTRAINTS',b'TCL_TEST_VERBOSE',b'TCL_TEST_CONSTRAINTS']:env.pop(key,None)
 if library:env[b'TCL_LIBRARY']=str(library).encode()
 else:env.pop(b'TCL_LIBRARY',None)
 env[b'TZ']=b'UTC0'
 start=time.monotonic()
 with tempfile.TemporaryDirectory(prefix='get-date-fields-') as cwd,(dest/'stdout.txt').open('wb') as stdout,(dest/'stderr.txt').open('wb') as stderr:
  result=subprocess.run(['timeout','--kill-after=5','60',*argv],cwd=cwd,env=env,stdout=stdout,stderr=stderr,preexec_fn=limits,timeout=67)
 rec={'name':name,'argv':argv,'library':str(library) if library else None,'exit_code':result.returncode,'seconds':round(time.monotonic()-start,3),'stdout_sha256':sha(dest/'stdout.txt'),'stderr_sha256':sha(dest/'stderr.txt')};records.append(rec);print(name,result.returncode,flush=True);return result.returncode
binaries={}
for version in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0']:
 tree=ROOT/('tmp/tcl'+version);binary=OUT/('control-'+version)
 argv=['cc','-DHAVE_UNISTD_H=1','-I'+str(tree/'unix'),'-I'+str(tree/'generic'),str(HERE/'control.c'),str(tree/('unix/libtcl'+'.'.join(version.split('.')[:2])+'.a')),'-lm','-ldl','-lpthread','-lz','-o',str(binary)]
 if execute(version+'/compile',argv)!=0:continue
 binaries[str(binary)]=sha(binary);execute(version+'/values',[str(binary),str(HERE/'input.tcl')],tree/'library')
tree=ROOT/'tmp/tcl8.6.18';binary=OUT/'control-c86-objects'
if execute('c86-objects/compile',['cc','-DHAVE_UNISTD_H=1','-I'+str(tree/'unix'),'-I'+str(tree/'generic'),str(HERE/'object-control.c'),str(tree/'unix/libtcl8.6.a'),'-lm','-ldl','-lpthread','-lz','-o',str(binary)])==0:
 binaries[str(binary)]=sha(binary);execute('c86-objects/values',[str(binary)],tree/'library')
execute('jim-pinned/values',[str(jim),str(HERE/'input.tcl')])
execute('native-c86/values',[str(runner),'--quiet','--tcl-version','8.6','--init',str(HERE/'input.tcl')],tree/'library')
for p in [HERE/'capture.py',HERE/'control.c',HERE/'object-control.c',HERE/'input.tcl',HERE/'cases.json']:(OUT/p.name).write_bytes(p.read_bytes())
(OUT/'provenance.json').write_text(json.dumps({'source_binary_before':before,'source_binary_after':{str(p):sha(p) for p in paths},'control_binary_sha256':binaries,'records':records,'scope':'Fresh C5 real Tcl_Init/counting Tcl_EvalEx and selected C86 original-object control; actual pinned Jim unadapted Jim setup and existing current Native C86 runner, no Rust/Cargo/build change.','environment':'Only synthetic TZ=UTC0 selected for genuine empty-table localtime question; no environment dump/private names or values queried.','source_channel':'Original counted C Tcl_EvalEx; Native unchanged file byte ingress; Jim unadapted script ingress.','F5':'TMM/MCPD native contexts unanswered; no Tcltest equivalent.'},indent=2)+'\n')
