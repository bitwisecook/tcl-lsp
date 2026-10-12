from pathlib import Path
import hashlib,json,os,resource,subprocess,tempfile
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[4];OUT=HERE/'capture-02';OUT.mkdir(exist_ok=False)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
records=[];env=dict(os.environb);env[b'TZ']=b'UTC0';env.pop(b'TCLLIBPATH',None)
def run(name,argv):
 dest=OUT/name;dest.mkdir(parents=True)
 with tempfile.TemporaryDirectory(prefix='getdatefields-c9-') as cwd,(dest/'stdout.txt').open('wb') as stdout,(dest/'stderr.txt').open('wb') as stderr:
  r=subprocess.run(['timeout','--kill-after=2','20',*argv],cwd=cwd,env=env,stdout=stdout,stderr=stderr,timeout=24)
 records.append({'name':name,'argv':argv,'exit':r.returncode,'stdout_sha256':sha(dest/'stdout.txt'),'stderr_sha256':sha(dest/'stderr.txt')});print(name,r.returncode)
 return r.returncode
before={str(HERE/p):sha(HERE/p) for p in ['control.c','c9-input.tcl','capture-c9.py']};binaries={}
for version in ['9.0.4','9.1.0']:
 tree=ROOT/('tmp/tcl'+version);env[b'TCL_LIBRARY']=str(tree/'library').encode();binary=OUT/('control-'+version)
 if run(version+'/compile',['cc','-DHAVE_UNISTD_H=1','-I'+str(tree/'unix'),'-I'+str(tree/'generic'),str(HERE/'control.c'),str(tree/('unix/libtcl'+'.'.join(version.split('.')[:2])+'.a')),'-lm','-ldl','-lpthread','-lz','-o',str(binary)])!=0:continue
 binaries[str(binary)]=sha(binary)
 for p in [tree/'generic/tclClock.c',tree/'library/clock.tcl',tree/'library/init.tcl',tree/('unix/libtcl'+'.'.join(version.split('.')[:2])+'.a')]:before[str(p)]=sha(p)
 run(version+'/values',[str(binary),str(HERE/'c9-input.tcl')])
for p in ['control.c','c9-input.tcl','capture-c9.py']:(OUT/p).write_bytes((HERE/p).read_bytes())
(OUT/'provenance.json').write_text(json.dumps({'source_before':before,'source_after':{p:sha(Path(p)) for p in before},'binary_hashes':binaries,'records':records,'scope':'Three separate C9 version questions after genuine clock format library warmup. Original32 C86-table rows, C90 classic-trace refusal, C91 first compile failure and pinned Jim trace refusal remain capture-01 unchanged. Native target remains C86 only; no overlay/emulation/Rust/Cargo.'},indent=2)+'\n')
