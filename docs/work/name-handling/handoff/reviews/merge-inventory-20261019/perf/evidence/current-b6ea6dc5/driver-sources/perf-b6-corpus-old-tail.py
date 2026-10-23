#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
from pathlib import Path
import hashlib,json,os,re,shutil,subprocess,tempfile,time
root=Path('/workspace/tcl-lsp-perf-latest');base=Path('/workspace/tcl-lsp-work/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/evidence');main=base/'current-b6ea6dc5-full-corpus';out=base/'current-b6ea6dc5-old-corpus-tail-recovery';out.mkdir(exist_ok=False)
r=json.loads((main/'inputs.json').read_text());roster=r['rosters']['corpus.txt'];completed=[line for line in (main/'old-only-tcl.out').read_text().splitlines() if line.startswith('REPORT label=')];assert len(completed)==77;paths=roster[77:];assert len(paths)==25
for path in paths:assert hashlib.sha256((root/path).read_bytes()).hexdigest()==r['fixture_sha256'][path]
artifacts=[json.loads(line) for line in (main/'build.jsonl').read_text().splitlines()];matches=[row for row in artifacts if row.get('reason')=='compiler-artifact' and row['target']['name']=='zz_authenticated_perfprobe' and row.get('executable')];assert len(matches)==1;original=Path(matches[0]['executable'])
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  while data:=f.read(1024*1024):h.update(data)
 return h.hexdigest()
record={'started_unix':time.time(),'head':r['head'],'main_inputs':str(main/'inputs.json'),'main_all_old_exit':-9,'original_completed':77,'scope':'Recover ONLY original25 uncompleted all-old Tcl inputs with the exact executable and unchanged triples. Preserve original failed one-process run and all Unavailable results. This is coverage recovery, not full acceptance.','paths':paths,'source_executable':str(original),'binary_sha256':sha(original)}
with tempfile.TemporaryDirectory(prefix='perf-b6-corpus-tail-') as td:
 exe=Path(td)/original.name;shutil.copy2(original,exe);assert sha(exe)==record['binary_sha256'];(out/'inputs.json').write_text(json.dumps(record,indent=2)+'\n')
 env=dict(os.environ,ZZ_OLD_ENV='1');start=time.time()
 with (out/'old-only-tcl-tail.out').open('w') as log:code=subprocess.run([str(exe),'files',*paths],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT).returncode
 text=(out/'old-only-tcl-tail.out').read_text();rows=[line for line in text.splitlines() if line.startswith('REPORT label=')];record.update(exit=code,wall_s=time.time()-start,completed=len(rows),equal=sum(line.endswith('pass=true') for line in rows),failures=[line for line in rows if not line.endswith('pass=true')],valid=code==0 and len(rows)==25 and all(line.endswith('pass=true') for line in rows) and 'REPORT_SUMMARY inputs=25 failed=0' in text,copy_removed_on_exit=True,finished_unix=time.time());(out/'final.json').write_text(json.dumps(record,indent=2)+'\n')
print('Recovery complete; exact copied executable removed',flush=True)
