#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
from pathlib import Path
import hashlib,json,os,subprocess,time
root=Path('/workspace/tcl-lsp-owned')
perf=Path('/workspace/tcl-lsp-publish/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf')
out=Path('/workspace/tcl-lsp-work/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/evidence/current-b6ea6dc5-clean6-after-memory-deferral');out.mkdir(exist_ok=False)
def process(pid):
 try:
  s=(Path('/proc')/str(pid)/'stat').read_text();v=s[s.rfind(')')+2:].split();return {'start':v[19],'state':v[0]}
 except OSError:return None
old=process(41020)
record={'pid':os.getpid(),'started_unix':time.time(),'waiting_for':{'pid':41020,'original':old},'reason':'Allow full compiler new/old runtime to finish naturally before restarting only the resource-deferred clean6 unit build; completed library retained. No runtime paused.','head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),'rust_diff_sha256':hashlib.sha256(subprocess.check_output(['git','diff','--','rust'],cwd=root)).hexdigest()}
(out/'waiter.json').write_text(json.dumps(record,indent=2)+'\n');print('Waiting for full selected compiler new/old runtime',flush=True)
while old:
 now=process(41020)
 if not now or now['state']=='Z' or now['start']!=old['start']:break
 time.sleep(5)
record['build_started_unix']=time.time();(out/'waiter.json').write_text(json.dumps(record,indent=2)+'\n')
assert hashlib.sha256(subprocess.check_output(['git','diff','--','rust'],cwd=root)).hexdigest()==record['rust_diff_sha256']
argv=['python3',str(perf/'tools/run_crate_proofs.py'),str(root),'--stage','compiler','--opt-level','1','--target-dir',str(root/'target'),'--evidence-dir',str(out/'crate-tests-opt1')]
code=subprocess.run(argv,cwd=root).returncode
(out/'final.json').write_text(json.dumps(record|{'exit':code,'finished_unix':time.time(),'source_diff_unchanged':hashlib.sha256(subprocess.check_output(['git','diff','--','rust'],cwd=root)).hexdigest()==record['rust_diff_sha256']},indent=2)+'\n')
raise SystemExit(code)
