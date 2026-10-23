#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Run original-budget server then SpecTcl suites after other jobs finish naturally."""
from pathlib import Path
import json,time
base=Path('/workspace/tcl-lsp-work/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/evidence/current-b6ea6dc5')
record={'started_unix':time.time(),'policy':'Wait for other owned compiler/corpus/lexer jobs to finish naturally. Never pause runtime tests; retain all original budgets.'}
def write():(base/'quiet-test-window.json').write_text(json.dumps(record,indent=2)+'\n')
def info(pid):
 p=Path('/proc')/str(pid)
 try:
  s=(p/'stat').read_text();v=s[s.rfind(')')+2:].split();return {'pid':pid,'state':v[0],'start':v[19],'argv':(p/'cmdline').read_bytes().replace(b'\0',b' ').decode()}
 except (OSError,UnicodeError):return None
def live(old):
 r=info(old['pid']);return r and r['start']==old['start'] and r['state']!='Z'
workers=[]
for pid,needle in [(34692,'perf-b6-clean6-checks.py'),(34585,'run_report_controls.py'),(41020,'perf-watch-compiler-b6.py'),(35120,'perf-corpus-b6-after-controls.py'),(36947,'--stage lexer')]:
 row=info(pid)
 if row and row['state']!='Z':assert needle in row['argv'],row;workers.append(row)
record['workers']=workers;write()
def builds_active():
 for p in Path('/proc').iterdir():
  if not p.name.isdigit():continue
  try:comm=(p/'comm').read_text().strip()
  except OSError:continue
  if comm in ['rustc','clippy-driver','cargo-clippy']:
   row=info(int(p.name))
   if row and row['state']!='Z':return True
 return False
while any(live(row) for row in workers) or builds_active():time.sleep(5)
record['other_work_finished_unix']=time.time();write()
path=base/'remaining-suite-drivers.json'
if not path.exists():
 record['remaining_suites']='not built: inspect control build failure';write();raise SystemExit('No remaining suite drivers; original failure retained')
drivers=json.loads(path.read_text())['drivers']
for stage,profile,flag in [('server','default','server'),('spectcl','opt1','spec')]:
 driver=info(drivers[stage]);log=Path('/tmp/perf-b6-'+stage+'.log').read_text()
 if not driver or driver['state']=='Z' or 'WAIT for quiet test resources:' not in log:
  record[stage]={'not_released':'builder failed; inspect original output','time':time.time()};write();continue
 record[stage]={'released_unix':time.time(),'driver':driver};write();Path('/tmp/perf-b6-'+flag+'-quiet-ready').touch()
 summary=base/('crate-tests-'+profile)/('tests-'+stage+'-summary.json')
 while True:
  try:state=json.loads(summary.read_text())
  except (OSError,ValueError):state={}
  exes=[]
  build=summary.parent/('tests-'+stage+'-build.jsonl')
  if build.exists():
   for line in build.read_text().splitlines():
    row=json.loads(line)
    if row.get('reason')=='compiler-artifact' and row.get('executable'):exes.append(Path(row['executable']))
  complete=len(state)==4 and all('returncode' in row for row in state.values())
  if complete and exes and all(not p.exists() for p in exes):break
  if not live(driver):record[stage]['incomplete_driver_exit']=not complete;break
  time.sleep(5)
 record[stage].update(summary=state,executables_removed=bool(exes) and all(not p.exists() for p in exes),finished_unix=time.time());write()
record['finished_unix']=time.time();write();print('Quiet comparisons complete; inspect actual outcomes',flush=True)
