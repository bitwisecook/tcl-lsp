from pathlib import Path
import subprocess,hashlib,json,os,sys
r=Path('/home/user/tcl-lsp'); facts={};env=os.environ.copy();env.pop('TCL_LIBRARY',None)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
for version in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0']:
 tree=r/'tmp'/('tcl'+version);binary=tree/'unix/tclsh';wrapper=Path('/root/.local/bin')/('tclsh'+'.'.join(version.split('.')[:2]))
 run=subprocess.run([str(wrapper)],input='puts [info patchlevel]\nputs [info library]\n',text=True,capture_output=True,env=env,timeout=30,check=True)
 lines=run.stdout.splitlines();assert lines[0]==version and len(lines)==2,(version,run.stdout)
 library=tree/'library' if lines[1].startswith('//zipfs:') else Path(lines[1]).resolve();assert library.is_relative_to(tree.resolve())
 paths=[binary,wrapper]+sorted(p for p in library.rglob('*') if p.is_file())
 # Retain dynamic library alongside executable if this build uses one.
 paths+=sorted(p for p in (tree/'unix').glob('libtcl*') if p.is_file())
 facts[version]={'binary':str(binary),'wrapper':str(wrapper),'patchlevel':lines[0],'library_report':lines[1],'source_library':str(library),'library_owner':'embedded zipfs authenticated by executable' if lines[1].startswith('//zipfs:') else 'source tree files','inputs':{str(p):sha(p) for p in paths},'stdout':run.stdout,'stderr':run.stderr}
jim=r/'tmp/jimtcl-oracle/jimsh'
run=subprocess.run([str(jim)],input='puts [info patchlevel]\n',text=True,capture_output=True,env=env,timeout=30,check=True)
facts['Jim']={'binary':str(jim),'inputs':{str(jim):sha(jim)},'stdout':run.stdout,'stderr':run.stderr,'scope':'Pinned Jim executable; no semantics inferred from setup.'}
out=Path(sys.argv[1]);out.write_text(json.dumps(facts,sort_keys=True,indent=2)+'\n');print(json.dumps({'snapshot':str(out),'sha256':sha(out),'provider_count':len(facts)}))
