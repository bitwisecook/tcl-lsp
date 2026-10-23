#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
from pathlib import Path
import json,os,signal,time
base=Path('/workspace/tcl-lsp-work/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/evidence')
def info(pid):
 try:
  p=Path('/proc')/str(pid);s=(p/'stat').read_text();v=s[s.rfind(')')+2:].split();return {'pid':pid,'state':v[0],'start':v[19],'args':(p/'cmdline').read_bytes().split(b'\0')}
 except OSError:return None
def live(row):
 now=info(row['pid']);return now and now['state']!='Z' and now['start']==row['start']
processes=[row for p in Path('/proc').iterdir() if p.name.isdigit() and (row:=info(int(p.name)))]
drivers=[r for r in processes if len(r['args'])>2 and r['args'][1].endswith(b'/run_crate_proofs.py') and r['args'][2]==b'/workspace/tcl-lsp-owned' and b'compiler' in r['args'] and r['state']!='Z'];assert len(drivers)==1;driver=drivers[0]
assert any(r['args'] and r['args'][0].endswith(b'/rustc') and b'--test' in r['args'] and b'/workspace/tcl-lsp-owned/target/debug/deps' in r['args'] and r['state']!='Z' for r in processes),'owned unit is not still compiling'
diag_pid=json.loads((base/'current-b6ea6dc5-native35-unit-trace/inputs.json').read_text())['pid'];workers=[r for pid in [35120,diag_pid] if (r:=info(pid)) and r['state']!='Z'];workers += [r for r in processes if len(r['args'])>1 and r['args'][1]==b'/tmp/perf-b6-corpus-old-tail.py' and r['state']!='Z'];out=base/'current-b6ea6dc5/compiler-runtime-window.json';record={'started_unix':time.time(),'pid':os.getpid(),'held_python_driver':driver['pid'],'workers':[dict(pid=r['pid'],start=r['start']) for r in workers],'policy':'Hold only Python compiler-test driver while its Cargo/rustc children continue building. Resume after full corpus, tail recovery and isolated diagnostic finish naturally, so no runtime test competes with those jobs for memory. Never signal a test executable. Build wall time is not latency evidence.'}
try:
 os.kill(driver['pid'],signal.SIGSTOP);record['held_unix']=time.time();out.write_text(json.dumps(record,indent=2)+'\n')
 while any(live(row) for row in workers):time.sleep(5)
finally:
 if live(driver):os.kill(driver['pid'],signal.SIGCONT);record['resumed_unix']=time.time()
 record['finished_unix']=time.time();out.write_text(json.dumps(record,indent=2)+'\n')
