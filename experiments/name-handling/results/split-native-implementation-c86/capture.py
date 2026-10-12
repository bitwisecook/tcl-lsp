#!/usr/bin/env python3
"""Current Native C86 against retained references; one original orchestration."""
from pathlib import Path
import argparse, gzip, hashlib, json, os, resource, subprocess, tempfile, time
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[3]
OUT=HERE/'capture-01'
REFERENCE=ROOT/'experiments/name-handling/results/split-native-reference/capture-02'
RUNNER=ROOT/'tmp/name-target-runtime/debug/examples/run_script'
TREE=ROOT/'tmp/tcl8.6.18'
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def limits():
 resource.setrlimit(resource.RLIMIT_AS,(1024**3,1024**3))
 resource.setrlimit(resource.RLIMIT_CORE,(0,0))
def snapshot():
 keys=json.loads(gzip.decompress((HERE/'validation-03/standalone-source-before.json.gz').read_bytes()))
 return {k:sha(ROOT/k) for k in keys}
def execute(name,script,quiet=True):
 dest=OUT/name; dest.mkdir(parents=True,exist_ok=False)
 env=dict(os.environb)
 for key in [b'NH_CLOCK_GETENV_MISSING',b'TCL_BACKEND_CONSTRAINTS',b'TCLLIBPATH',b'TCL_TEST_VERBOSE',b'TCL_TEST_CONSTRAINTS']: env.pop(key,None)
 env.update({b'TCL_LIBRARY':str(TREE/'library').encode(),b'NH_CLOCK_GETENV_TEST':b'host-value',b'NH_CLOCK_GETENV_\xff':b'raw-name-value',b'NH_CLOCK_GETENV_RAW_VALUE':b'\xff'})
 argv=['timeout','--kill-after=5','60',str(RUNNER),*(['--quiet'] if quiet else []),'--tcl-version','8.6','--init',str(script)]
 start=time.monotonic()
 with tempfile.TemporaryDirectory(prefix='split-c86-current-') as cwd,(dest/'stdout.txt').open('wb') as stdout,(dest/'stderr.txt').open('wb') as stderr:
  result=subprocess.run(argv,cwd=cwd,env=env,stdout=stdout,stderr=stderr,preexec_fn=limits,timeout=67)
 record={'name':name,'argv':argv,'exit_code':result.returncode,'seconds':round(time.monotonic()-start,3),'input_sha256':sha(script),'stdout_sha256':sha(dest/'stdout.txt'),'stderr_sha256':sha(dest/'stderr.txt')}
 (dest/'record.json').write_text(json.dumps(record,indent=2)+'\n')
 return record
if __name__=='__main__':
 parser=argparse.ArgumentParser();parser.add_argument('phase',choices=['split','producer','all']);args=parser.parse_args()
 OUT.mkdir(exist_ok=True)
 (OUT/'capture.py').write_bytes((HERE/'capture.py').read_bytes())
 before=snapshot();binary_before=sha(RUNNER); records=[]
 if args.phase=='split':
  old=json.loads((REFERENCE/'provenance.json').read_text()); comparisons=[]
  for case in json.loads((REFERENCE.parent/'cases.json').read_text()):
   name=case['id']; script=REFERENCE/(name+'.tcl');record=execute('native-c86/'+name,script);records.append(record)
   current=(OUT/'native-c86'/name/'stdout.txt').read_bytes()
   axes={}
   for engine in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0','jim-pinned','native-c86']:
    path=REFERENCE/engine/name/'stdout.txt';raw=path.read_bytes();count=[line.decode('ascii') for line in raw.splitlines() if line.startswith(b'COUNT ')]
    axes[engine]={'retained_stdout_sha256':sha(path),'count':count,'status':'retained reference, not rerun'}
   c=(REFERENCE/'8.6.18'/name/'stdout.txt').read_bytes()
   c_guest=b''.join(line+b'\n' for line in c.splitlines() if not line.startswith(b'C_'))
   comparisons.append({'case':name,'current_count':[line.decode('ascii') for line in current.splitlines() if line.startswith(b'COUNT ')],'same_c86_guest_stdout':current==c_guest,'retained_axes':axes})
   print(name,record['exit_code'],comparisons[-1]['current_count'],comparisons[-1]['same_c86_guest_stdout'],flush=True)
   if record['exit_code']!=0: break
  (OUT/'split-comparison.json').write_text(json.dumps({'cases':comparisons,'retained_reference_provenance_sha256':sha(REFERENCE/'provenance.json'),'old_native_binary_sha256':old['source_and_binary_before'][str(RUNNER)],'note':'Fresh Native only. C raw element/object identity channels are independently validated by focused tests; guest stdout comparison excludes C_* control reports. C5/Jim are retained actual reference receipts with their old source/binary provenance.'},indent=2)+'\n')
 elif args.phase=='producer':
  expected={'written-nul':'c080','written-unicode-nul':'c080','written-surrogate':'eda080','literal-raw-nul':'00','literal-encoded-nul':'c080','braced-backslash':'5c30','double-backslash':'5c30','mixed':'41c0805658c08042'}
  comparisons=[]
  for name,value in expected.items():
   source=HERE/'producer-reference/capture-01'/(name+'.tcl');data=source.read_bytes();assert data.endswith(b'list $d\n')
   script=OUT/(name+'.tcl');script.write_bytes(data[:-len(b'list $d\n')]+b'set d\n')
   record=execute('native-producer/'+name,script,quiet=False);records.append(record)
   stdout=(OUT/'native-producer'/name/'stdout.txt').read_bytes();result=stdout.split(b'\n',1)[1] if b'\n' in stdout else b''
   raw=(HERE/'producer-reference/capture-01'/name/'stdout.txt').read_bytes()
   c_line=[line for line in raw.splitlines() if line.startswith(b'C_RAW_ELEMENT ')];assert len(c_line)==1
   c_hex=c_line[0].split(b' ',2)[2].decode('ascii');assert c_hex==value
   comparisons.append({'case':name,'current_raw_result_hex':result[:-1].hex() if result.endswith(b'\n') else result.hex(),'matches_retained_c86_raw_element':result==bytes.fromhex(value)+b'\n','retained_c86_stdout_sha256':sha(HERE/'producer-reference/capture-01'/name/'stdout.txt'),'retained_c86_input_sha256':sha(source),'reporting':'Same body; Native final set d uses counted completion bytes, retained C final list $d uses counted raw element.'})
   print(name,record['exit_code'],comparisons[-1]['matches_retained_c86_raw_element'],flush=True)
   if record['exit_code']!=0:break
  (OUT/'producer-comparison.json').write_text(json.dumps({'cases':comparisons,'other_producer_oracles':'C84/C85/C9/Jim/F5 not executed; no producer parity inferred.'},indent=2)+'\n')
 else:
  source=ROOT/'experiments/name-handling/results/runtime-c86-real-suites/getenv-runtime/capture-01/all-tcl-var.tcl'
  script=OUT/'all-tcl-var.tcl';script.write_bytes(source.read_bytes());records.append(execute('native-original-all-tcl',script))
  print('original-all-tcl exit',records[-1]['exit_code'],flush=True)
  (OUT/'orchestration-inputs.json').write_text(json.dumps({str(p):sha(p) for p in [source,script,TREE/'library/init.tcl',TREE/'library/tcltest/tcltest.tcl',TREE/'tests/all.tcl',TREE/'tests/var.test']},indent=2)+'\n')
 after=snapshot()
 (OUT/(args.phase+'-source-before.json.gz')).write_bytes(gzip.compress(json.dumps(before,sort_keys=True).encode(),mtime=0))
 (OUT/(args.phase+'-source-after.json.gz')).write_bytes(gzip.compress(json.dumps(after,sort_keys=True).encode(),mtime=0))
 (OUT/(args.phase+'-provenance.json')).write_text(json.dumps({'records':records,'runner_sha256_before':binary_before,'runner_sha256_after':sha(RUNNER),'source_inventory_count':len(before),'changed_sources':[k for k in before if before[k]!=after[k]],'scope':'Conservative standalone source/fixture inventory, not every file compiled. Compiler/VM/LSP/xtask absent from standalone graph. Frozen seven owners and focused fixture hashes are separately recorded in source-ready-v4.json.','limits':{'address_space_bytes':1024**3,'timeout_seconds':60,'core_bytes':0},'environment':'Inherited host data never logged or dumped. Only named benign NH_CLOCK_GETENV controls added; overlays/Tcl test environment switches removed.','version_axes':'Current Native C86 only; retained C5/Jim receipts preserve separate old provenance. No F5 native context executed.'},indent=2)+'\n')
