import argparse,hashlib,json,pathlib
p=argparse.ArgumentParser();p.add_argument('--verify-only',action='store_true',required=True);p.parse_args()
root=pathlib.Path(__file__).resolve().parent
receipts=json.loads((root/'receipt.json').read_text());rows=0;executed=0;not_attempted=0
for receipt in receipts:
 d=root/receipt['provider'];raw=(d/'stdout.tsv').read_bytes()
 assert hashlib.sha256(raw).hexdigest()==receipt['stdout_sha256']
 assert raw.decode('ascii').splitlines()==receipt['rows']
 assert receipt==json.loads((d/'receipt.json').read_text())
 for name,key in [('compile.stdout','compile_stdout_sha256'),('compile.stderr','compile_stderr_sha256'),('stderr','stderr_sha256')]:
  assert hashlib.sha256((d/name).read_bytes()).hexdigest()==receipt[key]
 for name,key in [('probe.c','probe_sha256'),('inputs.json','inputs_sha256'),('capture.py','runner_sha256'),('queue.json','queue_sha256')]:
  assert hashlib.sha256((root/name).read_bytes()).hexdigest()==receipt[key]
 for row in receipt['rows']:
  label,code,value=row.split('|',2);int(code)
  if label.endswith('_NOT_ATTEMPTED'):not_attempted+=1
  elif label.startswith(('DIRECT_','COUNTED_SOURCE_')) and not any(x in label for x in ['_SETUP','_ARG','_INPUT','_OPTIONS']):bytes.fromhex(value);executed+=1
 rows+=len(receipt['rows'])
print(json.dumps({'providers':len(receipts),'protocol_rows':rows,'executed_controls':executed,'not_attempted_controls':not_attempted,'native_launches':0,'compiler_launches':0,'rust_launches':0}))
