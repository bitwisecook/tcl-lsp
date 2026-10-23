#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Build the remaining suites after the large report-control executable is ready."""
from pathlib import Path
import subprocess,os,time,json
root=Path('/workspace/tcl-lsp-perf-latest');perf=Path('/workspace/tcl-lsp-publish/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf');base=Path('/workspace/tcl-lsp-work/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/evidence');controls=base/'current-b6ea6dc5-controls';out=base/'current-b6ea6dc5';handles=[];children=[]
while not (controls/'controls-binary.json').exists():
 if (controls/'final.json').exists():raise RuntimeError('control build failed; remaining suites not started on broken source')
 time.sleep(5)
try:
 for stage,profile,target,flag in [('server','0',root/'target','server'),('spectcl','1',root/'target-spectcl-opt1','spec')]:
  log=Path('/tmp/perf-b6-'+stage+'.log').open('w');handles.append(log)
  env=dict(os.environ,CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true',CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true',TCL_LSP_JIMSH=str(root/'tmp/jimtcl-oracle/jimsh'))
  args=['python3',str(perf/'tools/run_crate_proofs.py'),str(root),'--stage',stage,'--opt-level',profile,'--target-dir',str(target),'--evidence-dir',str(out/('crate-tests-opt1' if profile=='1' else 'crate-tests-default')),'--wait-before-tests','/tmp/perf-b6-'+flag+'-quiet-ready']
  child=subprocess.Popen(args,cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT);children.append(child)
 (out/'remaining-suite-drivers.json').write_text(json.dumps({'started_unix':time.time(),'drivers':dict(zip(['server','spectcl'],[c.pid for c in children])),'readiness':'held until all other runtime work completes naturally'},indent=2)+'\n')
 for child in children:child.wait()
finally:
 for handle in handles:handle.close()
