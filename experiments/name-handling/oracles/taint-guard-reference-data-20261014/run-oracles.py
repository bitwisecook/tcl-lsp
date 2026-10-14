from pathlib import Path
import os,hashlib,subprocess,json
base=Path('/workspace/tcl-lsp/tmp'); r=Path('/tmp/name-taint-guard-original-reference-20261014/oracles');script=r/'guard-reference-data.tcl'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
rows=[]
for release in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0','Jim']:
 root=base/('jimtcl-oracle' if release=='Jim' else 'tcl'+release)
 exe=root/('jimsh' if release=='Jim' else 'unix/tclsh'); init=None if release=='Jim' else root/'library/init.tcl'
 env=os.environ.copy();env.pop('TCL_LIBRARY',None);env.pop('TCLLIBPATH',None)
 if init:env['TCL_LIBRARY']=str(init.parent);env['LD_LIBRARY_PATH']=str(exe.parent)
 libraries={str(p.resolve()):p.resolve() for p in (root/'unix').glob('libtcl*.so*') if p.is_file()}
 before={'executable':sha(exe),'provider_libraries':{name:sha(path) for name,path in libraries.items()}}
 if init:before['init']=sha(init)
 run=subprocess.run([str(exe),str(script)],env=env,capture_output=True,timeout=15)
 (r/(release+'.stdout')).write_bytes(run.stdout);(r/(release+'.stderr')).write_bytes(run.stderr)
 after={'executable':sha(exe),'provider_libraries':{name:sha(path) for name,path in libraries.items()}}
 if init:after['init']=sha(init)
 rows.append({'provider':release,'executable':str(exe),'init':str(init) if init else None,'before':before,'after':after,'script_sha256':sha(script),'returncode':run.returncode,'stdout':run.stdout.decode(),'stderr':run.stderr.decode(),'unchanged':before==after})
(r/'results.json').write_text(json.dumps(rows,indent=2)+'\n')
print(json.dumps([{'provider':x['provider'],'returncode':x['returncode'],'stdout':x['stdout'],'unchanged':x['unchanged']} for x in rows],indent=2))
