from pathlib import Path
import hashlib,json,os,re,subprocess,time
root=Path('/workspace/tcl-lsp');base=root/'experiments/name-handling/patterns';out=base/'wiki-subject-results/post-merge-runtime-native';out.mkdir(parents=True,exist_ok=True)
probe=base/'wiki-probes.tcl';binary=Path('/tmp/name-runtime-native-lifecycle');receipt=root/'experiments/name-handling/results/runtime-lifecycle-fix/native-runner/receipt.json'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
ref=json.loads(receipt.read_text());assert sha(binary)==ref['binary_sha256']
env=os.environ.copy();env['TCL_LIBRARY']=str(root/'tmp/tcl9.0.4/library');env.pop('TCL_BACKEND_CONSTRAINTS',None)
cmd=[str(binary),'--init','--quiet','--tcl-version','9.0',str(probe)];start=time.time();r=subprocess.run(cmd,cwd=root,env=env,capture_output=True,timeout=60)
(out/'stdout.txt').write_bytes(r.stdout);(out/'stderr.txt').write_bytes(r.stderr);(out/'capture.py').write_bytes(Path(__file__).read_bytes())
match=re.search(rb'Total\s+(\d+)\s+Passed\s+(\d+)\s+Skipped\s+(\d+)\s+Failed\s+(\d+)',r.stdout)
meta={'engine':'runtime','subject_commit':ref['subject_commit'],'build_receipt':str(receipt.relative_to(root)),'build_receipt_sha256':sha(receipt),'build_currency_note':ref.get('currency_note','Actual native-bootstrap runner source patch and build receipt retained; no pristine HEAD claim.'),'input':str(probe.relative_to(root)),'input_sha256':sha(probe),'command':cmd,'binary_sha256':sha(binary),'started_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime(start)),'TCL_LIBRARY':env['TCL_LIBRARY'],'init_sha256':sha(Path(env['TCL_LIBRARY'])/'init.tcl'),'tcltest_sha256':sha(Path(env['TCL_LIBRARY'])/'tcltest/tcltest.tcl'),'backend_constraints_applied':False,'timeout_seconds':60,'exit_code':r.returncode,'stdout_sha256':sha(out/'stdout.txt'),'stderr_sha256':sha(out/'stderr.txt'),'counts':dict(zip(['total','passed','skipped','failed'],map(int,match.groups()))) if match else None}
assert sha(binary)==ref['binary_sha256'];(out/'provenance.json').write_text(json.dumps(meta,indent=2)+'\n');print(meta['counts']);print(r.stderr.decode()[:1500])
