import hashlib,json,os,re,subprocess,time
from pathlib import Path
root=Path('/workspace/tcl-lsp'); out=root/'experiments/name-handling/patterns/wiki-subject-results';out.mkdir(exist_ok=True)
probe=root/'experiments/name-handling/patterns/wiki-probes.tcl'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
tracked=subprocess.check_output(['git','ls-files','rust','runtime/rust','runtime/src'],cwd=root,text=True).splitlines()
inventory=[{'path':p,'sha256':sha(root/p)} for p in tracked if (root/p).is_file()]
(out/'source-inventory.json').write_text(json.dumps(inventory,indent=2)+'\n')
(out/'working-tree.diff').write_bytes(subprocess.check_output(['git','diff','--binary','HEAD','--','rust','runtime'],cwd=root))
env=os.environ.copy();env['TCL_LIBRARY']=str(root/'tmp/tcl9.0.4/library');env.pop('TCL_BACKEND_CONSTRAINTS',None)
engines={'vm':[str(root/'tmp/name-target-vm/debug/examples/run_test')], 'runtime':[str(root/'tmp/name-target-runtime/debug/examples/run_script'),'--init','--quiet','--tcl-version','9.0']}
records=[]
for engine,base in engines.items():
 command=base+[str(probe)];started=time.time(); result=subprocess.run(command,cwd=root,env=env,capture_output=True,timeout=60)
 for stream,data in [('stdout',result.stdout),('stderr',result.stderr)]: (out/(engine+'.'+stream)).write_bytes(data)
 match=re.search(rb'Total\s+(\d+)\s+Passed\s+(\d+)\s+Skipped\s+(\d+)\s+Failed\s+(\d+)',result.stdout)
 record={'engine':engine,'command':command,'binary_sha256':sha(Path(base[0])),'binary_mtime_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime(Path(base[0]).stat().st_mtime)), 'started_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime(started)), 'exit_code':result.returncode,'stdout_sha256':hashlib.sha256(result.stdout).hexdigest(),'stderr_sha256':hashlib.sha256(result.stderr).hexdigest(),'counts':dict(zip(['total','passed','skipped','failed'],map(int,match.groups()))) if match else None}
 records.append(record);print(engine,result.returncode,record['counts']);print(result.stderr.decode()[:1200])
provenance={'capture_date':'2026-10-12','subject_head':head,'build_source_correspondence':'Unverified: these are supplied existing binaries. Source inventory captures the working tree at invocation, not a proved build input receipt. Binary timestamps/hashes retained; do not claim coverage of later source changes.', 'source_inventory_sha256':sha(out/'source-inventory.json'),'working_tree_diff_sha256':sha(out/'working-tree.diff'),'input_sha256':sha(probe),'input':str(probe.relative_to(root)), 'TCL_LIBRARY':env['TCL_LIBRARY'],'init_sha256':sha(Path(env['TCL_LIBRARY'])/'init.tcl'),'tcltest_sha256':sha(Path(env['TCL_LIBRARY'])/'tcltest/tcltest.tcl'), 'timeout_seconds':60,'backend_constraints_applied':False,'records':records}
(out/'provenance.json').write_text(json.dumps(provenance,indent=2)+'\n')
(out/'capture.py').write_bytes(Path(__file__).read_bytes())
