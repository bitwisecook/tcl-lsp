#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
from pathlib import Path
import json,os,subprocess,time
root=Path('/workspace/tcl-lsp-perf-latest');perf=Path('/workspace/tcl-lsp-publish/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf');base=Path('/workspace/tcl-lsp-work/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/evidence/current-b6ea6dc5');pid=47490
def process(pid):
 try:
  s=(Path('/proc')/str(pid)/'stat').read_text();v=s[s.rfind(')')+2:].split();return v[0],v[19]
 except OSError:return None
old=process(pid);mapping=base/'remaining-suite-drivers.json';r=json.loads(mapping.read_text());r['original_server_driver']=r['drivers']['server'];r['drivers']['server']=os.getpid();r['server_retry']={'started_unix':time.time(),'wait_for_clean6':pid,'log':'/tmp/perf-b6-server-after-memory.log','evidence':'crate-tests-default-after-memory','reason':'Serialize server build after clean6 compiler runtime to preserve memory and original server wall budgets.'};mapping.write_text(json.dumps(r,indent=2)+'\n');print('Waiting for clean6 compiler build and both runtime checks to finish naturally',flush=True)
while old:
 now=process(pid)
 if not now or now[0]=='Z' or now[1]!=old[1]:break
 time.sleep(5)
argv=['python3',str(perf/'tools/run_crate_proofs.py'),str(root),'--stage','server','--opt-level','0','--target-dir',str(root/'target'),'--evidence-dir',str(base/'crate-tests-default-after-memory'),'--wait-before-tests','/tmp/perf-b6-server-quiet-ready'];code=subprocess.run(argv,cwd=root).returncode
(base/'server-memory-retry-final.json').write_text(json.dumps({'pid':os.getpid(),'exit':code,'finished_unix':time.time()},indent=2)+'\n');raise SystemExit(code)
