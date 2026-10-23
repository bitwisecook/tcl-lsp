#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
from pathlib import Path
import json,os,subprocess,time
base=Path('/workspace/tcl-lsp-work/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/evidence');root=Path('/workspace/tcl-lsp-diagnostic');tool=Path('/workspace/tcl-lsp-publish/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/tools/run_report_controls.py');out=base/'current-b6ea6dc5-empty-allocated-controls-quiet-wait';out.mkdir(exist_ok=False);record={'pid':os.getpid(),'started_unix':time.time(),'policy':'Wait isolated unchanged compiler test recovery and private ELF cleanup before candidate rebuild. Preserve earlier type-error and resource-failed builds. No source/assertion/budget changes.'};(out/'inputs.json').write_text(json.dumps(record,indent=2)+'\n');print('Waiting for isolated compiler recovery final receipt',flush=True)
while not (base/'current-b6ea6dc5-clean6-isolated-recovery/final.json').exists():time.sleep(5)
record['build_started_unix']=time.time();(out/'inputs.json').write_text(json.dumps(record,indent=2)+'\n');code=subprocess.run(['python3',str(tool),str(root),'--opt-level','0','--target-dir',str(root/'target'),'--evidence',str(base/'current-b6ea6dc5-empty-allocated-controls-quiet')],cwd=root).returncode;(out/'final.json').write_text(json.dumps(record|{'exit':code,'finished_unix':time.time()},indent=2)+'\n');raise SystemExit(code)
