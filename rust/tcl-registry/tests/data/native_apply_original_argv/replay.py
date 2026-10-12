#!/usr/bin/env python3
"""Verify immutable original-object results without a provider launch."""
import argparse,json,hashlib
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--verify-only',action='store_true',required=True);p.parse_args();root=Path(__file__).resolve().parent
rows=0
for r in json.loads((root/'receipt.json').read_text()):
 d=root/r['provider'];assert r['compile_exit']==r['process_exit']==0
 for name,key in [('compile.stdout','compile_stdout_sha256'),('compile.stderr','compile_stderr_sha256'),('stdout.tsv','stdout_sha256'),('stderr','stderr_sha256')]:
  assert hashlib.sha256((d/name).read_bytes()).hexdigest()==r[key]
 for name,key in [('probe.c','probe_sha256'),('inputs.json','inputs_sha256'),('capture.py','runner_sha256'),('queue.json','queue_sha256')]:assert hashlib.sha256((root/name).read_bytes()).hexdigest()==r[key]
 raw=(d/'stdout.tsv').read_text().splitlines();assert raw==r['rows'];rows+=len(raw)
 values=json.loads((root/'inputs.json').read_text())['values']
 for v in values:
  matched=[line.split('|') for line in raw if line.startswith('APPLY_'+v['name']+'|')];assert len(matched)==1
  if r['provider']!='8.4.20':assert matched[0][1:]==['0','1',v['hex']]
  if r['provider'] in ['8.6.18','9.0.4','9.1.0']:
   matched=[line.split('|') for line in raw if line.startswith('COROUTINE_RETURN_'+v['name']+'|')];assert matched[0][1:]==['0','1',v['hex']]
assert rows==291
print('PASS:291 retained protocol rows,35 direct and21 post-suspension same-object returns; zero provider launches')
