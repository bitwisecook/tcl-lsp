from pathlib import Path
import argparse,hashlib,json
p=Path(__file__).resolve().parent
h=lambda f:hashlib.sha256(f.read_bytes()).hexdigest()
a=argparse.ArgumentParser();a.add_argument('--current-provider',action='store_true');a=a.parse_args()
r=json.loads((p/'archive-receipt.json').read_text())
for row in r['originals']:
 assert h(p/row['path'])==row['sha256'],row['path']
m=json.loads((p/'manifest.json').read_text());assert h(p/'probe.tcl')==m['probe_sha256']
for row in m['rows']:
 assert row['exit_code']==0 and row['provider_before']==row['provider_after']
 assert h(p/(row['version']+'.stdout'))==row['stdout_sha256']
 assert h(p/(row['version']+'.stderr'))==row['stderr_sha256']
 if a.current_provider:
  for path,digest in row['provider_before'].items():assert h(Path(path))==digest,path
print('PASS',len(r['originals']),'retained',len(m['rows']),'provider receipts')
