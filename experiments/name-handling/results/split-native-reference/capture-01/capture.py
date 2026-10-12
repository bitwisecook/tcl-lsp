import hashlib,json,os,resource,subprocess,tempfile,time
from pathlib import Path
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[3]
VERSIONS=('8.4.20','8.5.19','8.6.18','9.0.4','9.1.0')
RUNNER=ROOT/'tmp/name-target-runtime/debug/examples/run_script';JIM=ROOT/'tmp/jimtcl-oracle'
OUT=HERE/'capture-01';OUT.mkdir(exist_ok=False)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def limits():
 resource.setrlimit(resource.RLIMIT_AS,(512*1024**2,512*1024**2));resource.setrlimit(resource.RLIMIT_CORE,(0,0))
cases=json.loads((HERE/'cases.json').read_text());records=[];binaries={}
paths=[HERE/'control.c',HERE/'capture.py',HERE/'cases.json',RUNNER,JIM/'jimsh',JIM/'jim.c',JIM/'jimsh.c',JIM/'utf8.c',JIM/'jimautoconf.h',ROOT/'rust/tcl-test-support/jim-reference.txt']
paths += [ROOT/p for p in ('rust/tcl-syntax/src/native_tcl_utf.rs','rust/tcl-syntax/src/value.rs','rust/tcl-cmd-core/src/list.rs')]
for version in VERSIONS:
 tree=ROOT/('tmp/tcl'+version)
 paths += [tree/p for p in ('generic/tclCmdMZ.c','generic/tclUtf.c','library/init.tcl','unix/tclsh')]
 paths.append(tree/('unix/libtcl'+'.'.join(version.split('.')[:2])+'.a'))
before={str(p):sha(p) for p in paths}
for name in ('control.c','capture.py','cases.json'):(OUT/name).write_bytes((HERE/name).read_bytes())
env=dict(os.environb)
for name in (b'TCLLIBPATH',b'TCL_BACKEND_CONSTRAINTS',b'TCL_TEST_VERBOSE',b'TCL_TEST_CONSTRAINTS'):env.pop(name,None)
def execute(name,argv,library=None):
 dest=OUT/name;dest.mkdir(parents=True);child=dict(env)
 if library:child[b'TCL_LIBRARY']=str(library).encode()
 start=time.monotonic()
 with tempfile.TemporaryDirectory(prefix='split-reference-') as cwd,(dest/'stdout.txt').open('wb') as stdout,(dest/'stderr.txt').open('wb') as stderr:
  cmd=['timeout','--kill-after=2','15',*map(str,argv)]
  result=subprocess.run(cmd,env=child,cwd=cwd,stdout=stdout,stderr=stderr,timeout=19,preexec_fn=limits())
 records.append({'name':name,'argv':cmd,'library':str(library) if library else None,'exit_code':result.returncode,'seconds':round(time.monotonic()-start,3),'stdout_sha256':sha(dest/'stdout.txt'),'stderr_sha256':sha(dest/'stderr.txt')})
 return result.returncode
engines={}
for version in VERSIONS:
 tree=ROOT/('tmp/tcl'+version);binary=OUT/('control-'+version)
 cmd=['cc','-DHAVE_UNISTD_H=1','-I'+str(tree/'unix'),'-I'+str(tree/'generic'),HERE/'control.c',tree/('unix/libtcl'+'.'.join(version.split('.')[:2])+'.a'),'-lm','-ldl','-lpthread','-lz','-o',binary]
 if execute(version+'/compile',cmd)==0:
  binaries[str(binary)]=sha(binary);engines[version]=([binary],tree/'library')
engines['jim-pinned']=([JIM/'jimsh'],None)
engines['native-c86']=([RUNNER,'--quiet','--tcl-version','8.6','--init'],ROOT/'tmp/tcl8.6.18/library')
inputs={}
for case in cases:
 def literal(field):return b'{'+bytes.fromhex(case[field])+b'}'
 script=b'puts [list RELEASE [info patchlevel]]\nset input '+(case['input_expr'].encode() if 'input_expr' in case else literal('input'))+b'\n'
 if case.get('delimiter') is None and 'delimiter_expr' not in case:command=b'split $input'
 else:
  script+=b'set delimiter '+(case['delimiter_expr'].encode() if 'delimiter_expr' in case else literal('delimiter'))+b'\n'
  command=b'split $input $delimiter'
 script+=b'puts SPLIT_ENTER\nset r ['+command+b']\nputs [list COUNT [llength $r]]\nforeach part $r {binary scan $part H* hex; puts [list PROJECTED_HEX $hex]}\nset r\n'
 p=OUT/(case['id']+'.tcl');p.write_bytes(script);inputs[p.name]=sha(p)
 for engine,(cmd,library) in engines.items():execute(engine+'/'+case['id'],[*cmd,p],library)
after={str(p):sha(p) for p in paths}
(OUT/'provenance.json').write_text(json.dumps({'source_and_binary_before':before,'source_and_binary_after':after,'binary_hashes':binaries,'inputs':inputs,'records':records,'scope':'Synthetic only; real C Tcl_Init and counted Tcl_EvalEx, unadapted pinned Jim shell, retained Native C86 runner with --init. Shared projected binary hex is distinct from C direct raw element bytes. No F5, Rust edits or Cargo.','environment':'Inherited environment not enumerated or logged; matching C/native TCL_LIBRARY and four named Tcl-test removals only.'},indent=2)+'\n')
print('source changes',sum(before[p]!=after[p] for p in before),'records',len(records),'capture',OUT)
for c in cases:
 row=[c['id']]
 for engine in engines:
  dest=OUT/engine/c['id'];stdout=(dest/'stdout.txt').read_text(errors='backslashreplace');record=next(r for r in records if r['name']==engine+'/'+c['id']);count=next((s[6:] for s in stdout.splitlines() if s.startswith('COUNT ')),'-');row.append(str(record['exit_code'])+'/'+count)
 print('\t'.join(row))
