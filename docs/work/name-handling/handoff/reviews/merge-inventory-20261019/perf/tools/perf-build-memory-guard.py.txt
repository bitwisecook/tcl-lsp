#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Serialize only the two large compiler unit-test builds; never pause tests."""
from pathlib import Path
import os,signal,time,json
controls=Path('/workspace/tcl-lsp-work/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/evidence/current-b6ea6dc5-controls')
out=controls.parent/'current-b6ea6dc5/build-memory-guard.json';record={'started_unix':time.time(),'policy':'Hold only owned-tree rustc --test until the full control executable is built or its build fails. Never signal runtime test executables.','events':[]};paused={}
def write():out.write_text(json.dumps(record,indent=2)+'\n')
def get(pid):
 p=Path('/proc')/str(pid)
 try:
  stat=(p/'stat').read_text();v=stat[stat.rfind(')')+2:].split();a=(p/'cmdline').read_bytes().split(b'\0');return {'pid':pid,'state':v[0],'start':v[19],'args':[s.decode() for s in a if s]}
 except (OSError,UnicodeError):return None
def units():
 found=[]
 for p in Path('/proc').iterdir():
  if not p.name.isdigit():continue
  row=get(int(p.name))
  if row and row['state']!='Z' and row['args'] and row['args'][0].endswith('/rustc') and '--test' in row['args'] and 'tcl_compiler' in row['args']:found.append(row)
 return found
def resume(pid,row):
 current=get(pid)
 if current and current['start']==row['start'] and current['state']!='Z':os.kill(pid,signal.SIGCONT);record['events'].append({'action':'resume','pid':pid,'time':time.time()});write()
try:
 write()
 while not (controls/'controls-binary.json').exists() and not (controls/'final.json').exists():
  active=units();full=True
  for row in active:
   if full and '/workspace/tcl-lsp-owned/' in ' '.join(row['args']) and row['pid'] not in paused:
    os.kill(row['pid'],signal.SIGSTOP);paused[row['pid']]=row;record['events'].append({'action':'pause','process':row,'time':time.time()});write()
  if not full:
   for pid,row in list(paused.items()):resume(pid,row);del paused[pid]
  time.sleep(5)
finally:
 for pid,row in paused.items():resume(pid,row)
 record['finished_unix']=time.time();write()
 print('Build memory guard finished; all rustc workers resumed',flush=True)
