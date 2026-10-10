import argparse,hashlib,json,pathlib
p=argparse.ArgumentParser();p.add_argument('--verify-only',action='store_true',required=True);p.parse_args()
root=pathlib.Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
queue=json.loads((root/'queue.json').read_text());inputs=json.loads((root/'inputs.json').read_text())
for original,digest in queue['input_sha256'].items():
 assert sha(root/pathlib.Path(original).name)==digest,original
aggregate=json.loads((root/'receipt.json').read_text());controls=0;rows_count=0
for provider in queue['providers']:
 d=root/provider['id'];receipt=json.loads((d/'receipt.json').read_text())
 assert receipt['compile_exit']==0 and receipt['process_exit']==0
 assert receipt==next(r for r in aggregate if r['provider']==provider['id'])
 assert receipt['required_sha256']==provider['required_sha256']
 assert receipt['probe_sha256']==sha(root/'probe.c') and receipt['inputs_sha256']==sha(root/'inputs.json')
 assert receipt['runner_sha256']==sha(root/'capture.py') and receipt['queue_sha256']==sha(root/'queue.json')
 for file,key in [('stdout.tsv','stdout_sha256'),('stderr','stderr_sha256'),('compile.stdout','compile_stdout_sha256'),('compile.stderr','compile_stderr_sha256')]:assert sha(d/file)==receipt[key],(provider['id'],file)
 rows=(d/'stdout.tsv').read_text().splitlines();assert rows==receipt['rows'];rows_count+=len(rows)
 parsed={row.split('|',2)[0]:row.split('|',2)[1:] for row in rows};assert len(parsed)==len(rows)
 for mode in ['DIRECT','COUNTED_SOURCE']:
  for kind in inputs['kinds']:
   for case in inputs['cases']:
    label=f'{mode}_{kind}_{case}';assert label in parsed and label+'_INPUT' in parsed
    assert parsed[label][0] in ['0','1'];bytes.fromhex(parsed[label][1]);bytes.fromhex(parsed[label+'_INPUT'][1]);controls+=1
 print(provider['id'],receipt['reported_version'],len(rows),'rows: retained bytes verified')
print(controls,'controls',rows_count,'rows; zero native/compiler/Rust launches')
