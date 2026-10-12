import hashlib,json,os,subprocess,tempfile
from pathlib import Path
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[4];OUT=HERE/'capture-01';OUT.mkdir(exist_ok=False)
C=ROOT/'experiments/name-handling/results/split-native-reference/capture-02/control-8.6.18'
NATIVE=ROOT/'tmp/name-target-runtime/debug/examples/run_script';LIB=ROOT/'tmp/tcl8.6.18/library'
paths=[C,NATIVE,HERE/'capture.py',ROOT/'experiments/name-handling/results/split-native-reference/control.c',ROOT/'runtime/rust/src/parse.rs',ROOT/'runtime/rust/src/interp.rs',ROOT/'rust/tcl-syntax/src/backslash.rs']
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
before={str(p):sha(p) for p in paths};records=[]
env=dict(os.environb);env[b'TCL_LIBRARY']=str(LIB).encode();env.pop(b'TCLLIBPATH',None)
for label,source in [('written-nul',b'\\0'),('literal-nul',b'{\0}'),('literal-encoded-nul',b'{\xc0\x80}')]:
 script=OUT/(label+'.tcl');script.write_bytes(b'puts [list RELEASE [info patchlevel]]\nset d '+source+b'\nputs [list EXTENT [string bytelength $d]]\nlist $d\n')
 for engine,cmd in [('c86',[str(C)]),('native-c86',[str(NATIVE),'--quiet','--tcl-version','8.6','--init'])]:
  dest=OUT/(engine+'-'+label);dest.mkdir()
  argv=['timeout','--kill-after=2','15',*cmd,str(script)]
  with tempfile.TemporaryDirectory(prefix='split-producer-') as cwd,(dest/'stdout.txt').open('wb') as stdout,(dest/'stderr.txt').open('wb') as stderr:
   result=subprocess.run(argv,cwd=cwd,env=env,stdout=stdout,stderr=stderr,timeout=19)
  records.append({'label':label,'engine':engine,'argv':argv,'exit_code':result.returncode,'stdout_sha256':sha(dest/'stdout.txt'),'stderr_sha256':sha(dest/'stderr.txt')})
  print(engine,label,result.returncode,(dest/'stdout.txt').read_text().strip(),(dest/'stderr.txt').read_text().strip())
after={str(p):sha(p) for p in paths}
(OUT/'capture.py').write_bytes((HERE/'capture.py').read_bytes())
(OUT/'provenance.json').write_text(json.dumps({'source_and_binary_before':before,'source_and_binary_after':after,'records':records,'input_hashes':{p.name:sha(p) for p in OUT.glob('*.tcl')},'scope':'Benign source producer extents only; counted real C86 Tcl_Init/Tcl_EvalEx control from frozen split reference; retained Native C86 runner with real --init. No Rust build/source changes or environment values queried.'},indent=2)+'\n')
