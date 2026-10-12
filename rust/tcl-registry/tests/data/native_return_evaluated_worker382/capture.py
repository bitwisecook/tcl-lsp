from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import subprocess,json,hashlib,time,datetime
root=Path('/workspace/tcl-lsp');out=Path('/tmp/native-return-grammar382');probe=out/'probe.tcl'
versions=['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0','jim']
def run(version):
 binary=Path('/workspace/.proofs/native-providers/jimtcl/jimsh') if version=='jim' else root/'tmp'/('tcl'+version)/'unix/tclsh'
 argv=[str(binary),str(probe)]
 env=None
 if version!='jim':
  import os
  env=os.environ.copy();env['TCL_LIBRARY']=str(root/'tmp'/('tcl'+version)/'library')
 start=datetime.datetime.now(datetime.timezone.utc).isoformat();tick=time.monotonic()
 completed=subprocess.run(argv,cwd=out,env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=20)
 directory=out/version;directory.mkdir(exist_ok=True)
 (directory/'stdout').write_bytes(completed.stdout);(directory/'stderr').write_bytes(completed.stderr)
 lines=completed.stdout.splitlines();rows=[line for line in lines if line.startswith(b'ROW|')];meta=[line.decode() for line in lines if line.startswith(b'META|')]
 receipt={'provider':version,'argv':argv,'original_source_sha256':hashlib.sha256(probe.read_bytes()).hexdigest(),'executable_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'started_utc':start,'elapsed_seconds':time.monotonic()-tick,'exit_code':completed.returncode,'stdout_sha256':hashlib.sha256(completed.stdout).hexdigest(),'stderr_sha256':hashlib.sha256(completed.stderr).hexdigest(),'public_meta_rows':meta,'row_count':len(rows)}
 (directory/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
 assert completed.returncode==0 and not completed.stderr,(version,completed.stderr[:1000])
 assert len(rows)==76,(version,len(rows))
 assert len(meta)==1 and (version=='jim' or meta[0].split('|')[1]==version),(version,meta)
 return {'provider':version,'meta':meta,'rows':len(rows),'elapsed':receipt['elapsed_seconds']}
with ThreadPoolExecutor(max_workers=6) as pool:
 for result in pool.map(run,versions):print(json.dumps(result))
