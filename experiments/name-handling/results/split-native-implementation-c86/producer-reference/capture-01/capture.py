from pathlib import Path
import hashlib,json,os,subprocess,tempfile
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[4];OUT=HERE/'capture-01';OUT.mkdir(exist_ok=False)
CONTROL=ROOT/'experiments/name-handling/results/split-native-reference/capture-02/control-8.6.18';records=[]
cases={'written-nul':b'set d \\0','written-unicode-nul':b'set d \\u0000','written-surrogate':b'set d \\uD800','literal-raw-nul':b'set d {\0}','literal-encoded-nul':b'set d {\xc0\x80}','braced-backslash':b'set d {\\0}','double-backslash':b'set d \\\\0','mixed':b'set v V; proc mark {} {return X}; set d "A\\0$v[mark]\\u0000B"'}
env=dict(os.environb);env[b'TCL_LIBRARY']=str(ROOT/'tmp/tcl8.6.18/library').encode();env.pop(b'TCLLIBPATH',None)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
for name,source in cases.items():
 p=OUT/(name+'.tcl');p.write_bytes(b'puts [list RELEASE [info patchlevel]]\n'+source+b'\nlist $d\n');d=OUT/name;d.mkdir();argv=['timeout','15',str(CONTROL),str(p)]
 with tempfile.TemporaryDirectory(prefix='source-producer-reference-') as cwd,(d/'stdout.txt').open('wb') as stdout,(d/'stderr.txt').open('wb') as stderr:run=subprocess.run(argv,cwd=cwd,env=env,stdout=stdout,stderr=stderr,timeout=19)
 records.append({'case':name,'argv':argv,'exit':run.returncode,'source_sha256':sha(p),'stdout_sha256':sha(d/'stdout.txt'),'stderr_sha256':sha(d/'stderr.txt')})
 print(name,run.returncode,(d/'stdout.txt').read_text().strip())
(OUT/'capture.py').write_bytes((HERE/'capture.py').read_bytes());(OUT/'provenance.json').write_text(json.dumps({'records':records,'control_sha256':sha(CONTROL),'actual_source_sha256':{p:sha(ROOT/p) for p in ['tmp/tcl8.6.18/generic/tclParse.c','tmp/tcl8.6.18/generic/tclUtf.c','tmp/tcl8.6.18/library/init.tcl']},'scope':'Eight benign direct C86 source controls only; actual real Tcl_Init/counting Tcl_EvalEx, no new oracle build, Native rebuilt comparisons pending focused checks, no C84/C85/C9/Jim/F5 producer execution inferred.'},indent=2)+'\n')
