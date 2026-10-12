#!/usr/bin/env python3
"""Verify immutable receipts and streams only; never build or launch Tcl."""
from pathlib import Path
import hashlib,json
root=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
checks=0
for suffix in ['', 'v2']:
 base=root/suffix
 aggregate=json.loads((base/'receipt.json').read_text())
 for provider in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0','jim']:
  r=json.loads((base/provider/'receipt.json').read_text())
  assert r['compile_exit']==0 and r['process_exit']==0
  assert next(x for x in aggregate if x['provider']==provider)==r
  for name,key in [('stdout.tsv','stdout_sha256'),('stderr','stderr_sha256'),('compile.stdout','compile_stdout_sha256'),('compile.stderr','compile_stderr_sha256')]:
   assert sha(base/provider/name)==r[key],(suffix,provider,name)
  for name,key in [('probe.c','probe_sha256'),('inputs.json','inputs_sha256'),('capture.py','runner_sha256'),('queue.json','queue_sha256')]:
   assert sha(base/name)==r[key],(suffix,provider,name)
  assert (base/provider/'stdout.tsv').read_text().splitlines()==r['rows']
  checks+=1
print(f'{checks} provider/variant stream and input associations verified; no launches')
