#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
from pathlib import Path
import subprocess,json,os,time
r=Path('/workspace/tcl-lsp-owned');p=Path('/workspace/tcl-lsp-publish/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf');e=Path('/workspace/tcl-lsp-work/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/evidence/current-b6ea6dc5-clean6');meta=json.loads((p/'patch-inputs.json').read_text());paths=meta['owned']
assert all((r/path).read_bytes()==(Path('/tmp/perf-b6-clean')/path).read_bytes() for path in paths)
assert set(subprocess.check_output(['git','diff','--name-only'],cwd=r,text=True).splitlines())==set(paths)
env=dict(os.environ,CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true',CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true');env.pop('ZZ_OLD_ENV',None)
results=[]
for stage in ['lexer','compiler']:
 code=subprocess.run(['python3',str(p/'tools/run_crate_proofs.py'),str(r),'--stage',stage,'--opt-level','1','--target-dir',str(r/'target'),'--evidence-dir',str(e/'crate-tests-opt1')],cwd=r,env=env).returncode
 results.append({'stage':stage,'exit':code})
 (e/'progress.json').write_text(json.dumps(results,indent=2)+'\n')
(e/'final.json').write_text(json.dumps({'results':results,'finished_unix':time.time(),'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=r,text=True).strip(),'switch_absent':True},indent=2)+'\n')
