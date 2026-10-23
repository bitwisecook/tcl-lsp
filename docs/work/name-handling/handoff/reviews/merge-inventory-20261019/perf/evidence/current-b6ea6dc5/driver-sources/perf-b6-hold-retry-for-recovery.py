#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
from pathlib import Path
import json,os,signal,time
base=Path('/workspace/tcl-lsp-work/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/evidence');recovery=json.loads((base/'current-b6ea6dc5-filter-resource-recovery/inputs.json').read_text())['pid'];waiter=47490
def process(pid):
 try:
  p=Path('/proc')/str(pid);s=(p/'stat').read_text();v=s[s.rfind(')')+2:].split();return {'state':v[0],'start':v[19],'args':(p/'cmdline').read_bytes().replace(b'\0',b' ').decode()}
 except OSError:return None
w=process(waiter);r=process(recovery);assert w and r and 'perf-b6-clean6-after-memory.py' in w['args'] and 'perf-b6-filter-recovery.py' in r['args'];record={'started_unix':time.time(),'pid':os.getpid(),'waiting_script':waiter,'recovery_driver':recovery,'policy':'Hold only the idle clean6 retry script until missing-mode resource recovery ends naturally. No Cargo compiler or runtime test is signalled. Finally always resumes the same waiting script.'};out=base/'current-b6ea6dc5/retry-recovery-ordering.json'
try:
 os.kill(waiter,signal.SIGSTOP);record['held_unix']=time.time();out.write_text(json.dumps(record,indent=2)+'\n')
 while True:
  now=process(recovery)
  if not now or now['state']=='Z' or now['start']!=r['start']:break
  time.sleep(5)
finally:
 now=process(waiter)
 if now and now['state']!='Z' and now['start']==w['start']:os.kill(waiter,signal.SIGCONT);record['resumed_unix']=time.time()
 record['finished_unix']=time.time();out.write_text(json.dumps(record,indent=2)+'\n')
