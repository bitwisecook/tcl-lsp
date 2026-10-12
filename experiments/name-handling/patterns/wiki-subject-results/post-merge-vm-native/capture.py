from pathlib import Path
import hashlib,json,os,re,subprocess,time
root=Path('/workspace/tcl-lsp');base=root/'experiments/name-handling/patterns';out=base/'wiki-subject-results/post-merge-vm-native';out.mkdir(parents=True,exist_ok=True)
probe=base/'wiki-probes.tcl';binary=Path('/tmp/name-native-run-test-018ac8741cd2');receipt=root/'experiments/name-handling/results/vm-native-post-merge-baseline/build-receipt.json'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
ref=json.loads(receipt.read_text());expected='018ac8741cd251e32bb8786795e69310d72e5c1c06d5e5952b37a2fcc7593137';assert sha(binary)==expected
env=os.environ.copy();env['TCL_LIBRARY']=str(root/'tmp/tcl9.0.4/library');env.pop('TCL_BACKEND_CONSTRAINTS',None)
cmd=[str(binary),str(probe),'--tcl-version','9.0'];start=time.time();r=subprocess.run(cmd,cwd=root,env=env,capture_output=True,timeout=60)
(out/'stdout.txt').write_bytes(r.stdout);(out/'stderr.txt').write_bytes(r.stderr);(out/'capture.py').write_bytes(Path(__file__).read_bytes())
match=re.search(rb'Total\s+(\d+)\s+Passed\s+(\d+)\s+Skipped\s+(\d+)\s+Failed\s+(\d+)',r.stdout)
meta={'engine':'vm','subject_commit':ref.get('subject_commit',ref.get('head','not recorded')),'build_receipt':str(receipt.relative_to(root)),'build_receipt_sha256':sha(receipt),'build_currency_note':'Actual native-bootstrap driver and release selector; complete build source receipt retained separately. No pristine HEAD claim.','input':str(probe.relative_to(root)),'input_sha256':sha(probe),'command':cmd,'binary_sha256':sha(binary),'started_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime(start)),'TCL_LIBRARY':env['TCL_LIBRARY'],'init_sha256':sha(Path(env['TCL_LIBRARY'])/'init.tcl'),'tcltest_sha256':sha(Path(env['TCL_LIBRARY'])/'tcltest/tcltest.tcl'),'backend_constraints_applied':False,'timeout_seconds':60,'exit_code':r.returncode,'stdout_sha256':sha(out/'stdout.txt'),'stderr_sha256':sha(out/'stderr.txt'),'counts':dict(zip(['total','passed','skipped','failed'],map(int,match.groups()))) if match else None}
assert sha(binary)==expected;(out/'provenance.json').write_text(json.dumps(meta,indent=2)+'\n');print(meta['counts']);print(r.stderr.decode()[:1500])
