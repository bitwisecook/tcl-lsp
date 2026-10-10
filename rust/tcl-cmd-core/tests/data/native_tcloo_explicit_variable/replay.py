import argparse,hashlib,json,pathlib
p=argparse.ArgumentParser();p.add_argument('--verify-only',action='store_true',required=True);p.parse_args()
root=pathlib.Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
for variant in ['v1','v2']:
 d=root/variant;queue=json.loads((d/'queue.json').read_text());aggregate=json.loads((d/'receipt.json').read_text());total=0
 for path,digest in queue['input_sha256'].items():assert sha(d/pathlib.Path(path).name)==digest,path
 for spec in queue['providers']:
  provider=d/spec['id'];receipt=json.loads((provider/'receipt.json').read_text())
  assert receipt==next(r for r in aggregate if r['provider']==spec['id'])
  assert receipt['compile_exit']==0 and receipt['process_exit']==0
  assert receipt['required_sha256']==spec['required_sha256']
  assert receipt['probe_sha256']==sha(d/'probe.c') and receipt['inputs_sha256']==sha(d/'inputs.json')
  assert receipt['runner_sha256']==sha(d/'capture.py') and receipt['queue_sha256']==sha(d/'queue.json')
  for file,key in [('stdout.tsv','stdout_sha256'),('stderr','stderr_sha256'),('compile.stdout','compile_stdout_sha256'),('compile.stderr','compile_stderr_sha256')]:assert sha(provider/file)==receipt[key]
  rows=(provider/'stdout.tsv').read_text().splitlines();assert rows==receipt['rows'];total+=len(rows)
  for line in rows:
   label,code,payload=line.split('|',2);assert code in ['0','1']
   if not label.endswith(('NOT_ATTEMPTED','OPTIONS_NOT_TESTED')):bytes.fromhex(payload)
 print(variant,total,'protocol rows verified; zero native/compiler/Rust launches')
