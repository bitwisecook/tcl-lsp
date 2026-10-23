#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
from pathlib import Path
import subprocess,json,time
base=Path('/workspace/tcl-lsp-work/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/evidence')
controls=base/'current-b6ea6dc5-controls'
while not (controls/'final.json').exists():time.sleep(5)
final=json.loads((controls/'final.json').read_text());rows={r['label']:r for r in final['results']}
assert final['source_unchanged']
assert all(rows[label]['exit']==0 for label in ['build-controls','controls-new','controls-old']), 'current complete controls have not passed; corpus not started'
root=Path('/workspace/tcl-lsp-perf-latest');tool=Path('/workspace/tcl-lsp-publish/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/tools/run_authenticated_corpus.py')
subprocess.run(['python3',str(tool),str(root),'--controls',str(controls),'--opt-level','1','--target-dir',str(root/'target-spectcl-opt1'),'--evidence',str(base/'current-b6ea6dc5-full-corpus')],check=True)
