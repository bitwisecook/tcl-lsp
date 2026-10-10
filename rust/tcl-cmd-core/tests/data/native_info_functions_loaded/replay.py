#!/usr/bin/env python3
"""Verify exact retained native info source/provider/stream associations."""
from pathlib import Path
import argparse,hashlib,json,re
def main():
 parser=argparse.ArgumentParser(description=__doc__)
 parser.add_argument('--verify-only',action='store_true',required=True)
 parser.parse_args()
 base=Path(__file__).resolve().parent;sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
 queue=json.loads((base/'queue.json').read_text());receipts=json.loads((base/'receipt.json').read_text())
 for receipt in receipts:
  directory=base/receipt['provider']
  assert json.loads((directory/'receipt.json').read_text())==receipt
  assert receipt['exit']==receipt['version_exit']==0 and receipt['rows']==12
  assert sha(base/'cases.tcl')==queue['source_sha256']==receipt['input_sha256']
  assert sha(base/'capture.py')==receipt['runner_sha256']
  assert sha(directory/'stdout')==receipt['stdout_sha256'] and sha(directory/'stderr')==receipt['stderr_sha256']
  assert (directory/'stderr').read_bytes()==b'' and (directory/'version.stderr').read_bytes()==b''
  assert (directory/'version.stdout').read_text().strip()==receipt['reported_version']
  lines=(directory/'stdout').read_text().splitlines()
  assert len(lines)==12 and [line.split(' ',1)[0] for line in lines]==queue['case_ids']
  for line in lines:
   match=re.fullmatch(r'(\S+) (\d+) (\d+) \{([\d ]*)\}',line);assert match,line
   assert int(match[3])==len(match[4].split())
 print(json.dumps({'providers':len(receipts),'controls':len(receipts)*12,'native_launches':0}))
if __name__=='__main__':main()
