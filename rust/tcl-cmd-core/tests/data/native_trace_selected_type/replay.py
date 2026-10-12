import hashlib,json,pathlib
root=pathlib.Path(__file__).resolve().parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
inputs=json.loads((root/'inputs.json').read_text());aggregate=json.loads((root/'receipt.json').read_text());checked=0
for r in aggregate:
 d=root/r['provider'];assert json.loads((d/'receipt.json').read_text())==r
 assert r['probe_sha256']==sha(root/'probe.c') and r['inputs_sha256']==sha(root/'inputs.json')
 assert r['runner_sha256']==sha(root/'capture.py') and r['queue_sha256']==sha(root/'queue.json')
 assert r['compile_exit']==r['process_exit']==0
 for name,key in [('stdout.tsv','stdout_sha256'),('stderr','stderr_sha256'),('compile.stdout','compile_stdout_sha256'),('compile.stderr','compile_stderr_sha256')]:assert sha(d/name)==r[key]
 rows=(d/'stdout.tsv').read_text().splitlines();assert rows==r['rows'] and len(rows)==91
 fields={line.split('|',2)[0]:line.split('|',2)[1:] for line in rows};assert len(fields)==91
 assert bytes.fromhex(fields['VERSION'][1]).decode('ascii')==r['reported_version']
 for case in inputs:
  label=case['label'];expected=case.get('source_hex',case['argv_hex'][2]);assert fields[label+'_INPUT']==['0',expected]
  assert fields[label][0]=='1';bytes.fromhex(fields[label][1])
  assert label+'_OPTIONS' in fields or label+'_OPTIONS_NOT_TESTED' in fields
  checked+=1
print(json.dumps({'provider_receipts':len(aggregate),'input_result_controls':checked,'protocol_rows':sum(len(r['rows']) for r in aggregate),'native_launches':0}))
