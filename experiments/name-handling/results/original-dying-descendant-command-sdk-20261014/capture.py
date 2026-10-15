from pathlib import Path
import hashlib,json,os,subprocess,concurrent.futures,datetime
root=Path(__file__).resolve().parent
prior=Path('/tmp/name-namespace-trailing-parent-sdk-20261014')
source=root/'probe.c'
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def run(version):
 original=json.loads((prior/version/'receipt.json').read_text());out=root/version;out.mkdir(exist_ok=True)
 command=list(original['compile_command']);command[4]=str(source);command[-1]=str(out/'probe')
 build=subprocess.run(command,capture_output=True,timeout=45)
 (out/'compile.stdout').write_bytes(build.stdout);(out/'compile.stderr').write_bytes(build.stderr)
 record={'version':version,'compile_command':command,'compile_exit':build.returncode,'library':original['library'],'library_sha256':sha(original['library']),'original_init':original['original_init'],'original_init_sha256':sha(original['original_init']),'source_sha256':sha(source)}
 if build.returncode==0:
  result=subprocess.run([str(out/'probe')],env={**os.environ,'TCL_LIBRARY':str(Path(original['original_init']).parent)},capture_output=True,timeout=30)
  (out/'stdout.tsv').write_bytes(result.stdout);(out/'stderr').write_bytes(result.stderr)
  record.update(process_exit=result.returncode,stdout_sha256=sha(out/'stdout.tsv'),stderr_sha256=sha(out/'stderr'),executable_sha256=sha(out/'probe'))
 (out/'receipt.json').write_text(json.dumps(record,indent=2)+'\n');return record
with concurrent.futures.ThreadPoolExecutor(max_workers=5) as pool:records=list(pool.map(run,['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0']))
manifest={'captured_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'scope':'Original C SDK namespace-eval missing descendant physical publication under actual namespace unset and command-delete callback windows. Native FindNamespace parent/child visibility and ancestor pointer equality distinguish genuine retained old-parent child from same-spelled recreated ancestor. Contents, reentry and final cleanup inspected. No implementation or tcltest result claimed.','providers':records}
(root/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
for record in records:print(record['version'],record['compile_exit'],record.get('process_exit'))
assert all(r.get('process_exit')==0 for r in records)
