#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
from pathlib import Path
import json,os,subprocess,time
root=Path('/workspace/tcl-lsp-diagnostic');base=Path('/workspace/tcl-lsp-work/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf');out=base/'evidence/current-b6ea6dc5-allocated-mini-acceptance';out.mkdir(exist_ok=False);probe=root/'rust/tcl-compiler/examples/zz_allocatedacceptanceprobe.rs';probe.parent.mkdir(exist_ok=True);assert not probe.exists();source=(base/'tools/authenticated_perfprobe.rs.txt').read_bytes();env=dict(os.environ,CARGO_INCREMENTAL='0',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_PROFILE_DEV_OPT_LEVEL='0',CARGO_PROFILE_TEST_OPT_LEVEL='0',CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true',CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true',CARGO_TARGET_DIR=str(root/'target'));env.pop('ZZ_OLD_ENV',None);exes=[];record={'started_unix':time.time(),'scope':'Candidate API diagnostic on compact original failing source; unchanged full triple in both old-only/mixed modes. Uses existing candidate opt0 normal library; not full corpus acceptance.','results':[]}
try:
 probe.write_bytes(source)
 with (out/'build.jsonl').open('w') as stdout,(out/'build.stderr').open('w') as stderr:code=subprocess.run(['cargo','build','--locked','-j2','-p','tcl-compiler','--example','zz_allocatedacceptanceprobe','--message-format=json'],cwd=root,env=env,stdout=stdout,stderr=stderr).returncode
 rows=[json.loads(line) for line in (out/'build.jsonl').read_text().splitlines()];warnings=[r for r in rows if r.get('reason')=='compiler-message' and r['message']['level']=='warning'];exes=[Path(r['executable']) for r in rows if r.get('reason')=='compiler-artifact' and r.get('executable')];record.update(build_exit=code,warnings=len(warnings));assert code==0 and not warnings and len(exes)==1
 src=base/'evidence/current-b6ea6dc5-no-comment-repro/source.tcl'
 for mode in ['old-only','mixed']:
  start=time.time()
  with (out/(mode+'.out')).open('w') as log:code=subprocess.run([str(exes[0]),'files',str(src)],cwd=root,env=env|({'ZZ_OLD_ENV':'1'} if mode=='old-only' else {}),stdout=log,stderr=subprocess.STDOUT).returncode
  text=(out/(mode+'.out')).read_text();record['results'].append({'mode':mode,'exit':code,'wall_s':time.time()-start,'valid':code==0 and 'REPORT_SUMMARY inputs=1 failed=0' in text and ' pass=true' in text});print(mode,record['results'][-1],flush=True)
finally:
 probe.unlink(missing_ok=True)
 for p in exes:
  p.unlink(missing_ok=True)
  for alias in p.parent.glob(p.name+'-*'):
   if alias.is_file() and os.access(alias,os.X_OK):alias.unlink()
 record.update(finished_unix=time.time(),executables_removed=all(not p.exists() for p in exes),probe_removed=not probe.exists());(out/'final.json').write_text(json.dumps(record,indent=2)+'\n')
