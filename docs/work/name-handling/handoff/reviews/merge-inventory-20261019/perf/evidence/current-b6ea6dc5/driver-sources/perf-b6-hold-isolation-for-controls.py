#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
from pathlib import Path
import json,os,signal,time
base=Path('/workspace/tcl-lsp-work/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/evidence');pid=json.loads((base/'current-b6ea6dc5-clean6-isolated-recovery/inputs.json').read_text())['pid'];p=Path('/proc')/str(pid)
def info():
 try:
  s=(p/'stat').read_text();v=s[s.rfind(')')+2:].split();return v[0],v[19]
 except OSError:return None
old=info();assert old and b'perf-b6-compiler-isolated-recovery.py' in (p/'cmdline').read_bytes();record={'started_unix':time.time(),'pid':os.getpid(),'held_orchestrator':pid,'policy':'Hold only isolation orchestrator until candidate control retry completes; existing child test may finish naturally. No runtime child signalled. Finally resumes the same orchestrator.'};out=base/'current-b6ea6dc5/isolation-control-retry-ordering.json'
try:
 os.kill(pid,signal.SIGSTOP);record['held_unix']=time.time();out.write_text(json.dumps(record,indent=2)+'\n')
 while not (base/'current-b6ea6dc5-empty-allocated-controls-retry/final.json').exists():time.sleep(5)
finally:
 now=info()
 if now and now[0]!='Z' and now[1]==old[1]:os.kill(pid,signal.SIGCONT);record['resumed_unix']=time.time()
 record['finished_unix']=time.time();out.write_text(json.dumps(record,indent=2)+'\n')
