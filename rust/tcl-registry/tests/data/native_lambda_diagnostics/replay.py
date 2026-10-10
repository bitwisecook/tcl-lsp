#!/usr/bin/env python3
"""Verify retained observations without compiling or launching a provider."""
import argparse, hashlib, json
from pathlib import Path
parser=argparse.ArgumentParser();parser.add_argument('--verify-only',action='store_true',required=True);parser.parse_args()
root=Path(__file__).resolve().parent
count=0
for variant in ['v1','v2']:
 packet=root/variant
 receipts=json.loads((packet/'receipt.json').read_text())
 assert len(receipts)==6
 for r in receipts:
  d=packet/r['provider']
  assert r['compile_exit']==0 and r['process_exit']==0
  for name,key in [('compile.stdout','compile_stdout_sha256'),('compile.stderr','compile_stderr_sha256'),('stdout.tsv','stdout_sha256'),('stderr','stderr_sha256')]:
   assert hashlib.sha256((d/name).read_bytes()).hexdigest()==r[key],str(d/name)
  assert hashlib.sha256((packet/'probe.c').read_bytes()).hexdigest()==r['probe_sha256']
  assert hashlib.sha256((packet/'inputs.json').read_bytes()).hexdigest()==r['inputs_sha256']
  assert hashlib.sha256((packet/'capture.py').read_bytes()).hexdigest()==r['runner_sha256']
  assert hashlib.sha256((packet/'queue.json').read_bytes()).hexdigest()==r['queue_sha256']
  rows=(d/'stdout.tsv').read_text().splitlines()
  assert rows==r['rows']
  assert len(rows)==(105 if variant=='v1' else 113)
  for row in rows:
   fields=row.split('|');assert len(fields)==3
   if not fields[0].endswith('NOT_TESTED'):
    assert fields[2]=='ABSENT' or len(bytes.fromhex(fields[2]))*2==len(fields[2])
  count+=len(rows)
assert count==1308
print(f'PASS: {count} retained rows, 12 provider captures; zero provider launches')
