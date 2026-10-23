#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
from pathlib import Path
import json,os,subprocess,time
root=Path('/workspace/tcl-lsp-perf-latest');out=Path('/workspace/tcl-lsp-work/docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/evidence/current-b6ea6dc5-native35-refusal');out.mkdir(exist_ok=False)
source='''// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
use std::sync::atomic::Ordering;
use tcl_compiler::analyser::Analyser;
fn main() {
    let path = std::env::args().nth(1).expect("fixture path");
    let source = std::fs::read_to_string(path).expect("complete UTF-8 fixture");
    tcl_compiler::realm::ZZ_NO_MEMO.store(true, Ordering::Relaxed);
    tcl_lexer::ZZ_OLD.store(true, Ordering::Relaxed);
    let mut reports = Vec::new();
    for index in 0..3 {
        let start = std::time::Instant::now();
        let report = Analyser::new().analyse_for_report(&source, "tcl8.6");
        println!("CAPTURE index={index} ms={} state={report:?} self={:?} procs={} classes={}", start.elapsed().as_millis(), report.compare(&report), report.analysis().all_procs.len(), report.analysis().all_classes.len());
        reports.push(report);
        for (prior, report) in reports.iter().enumerate() {
            println!("AFTER capture={index} prior={prior} state={report:?} self={:?}", report.compare(report));
        }
    }
    for (left, right) in [(0, 1), (1, 2), (0, 2)] {
        println!("PAIR left={left} right={right} result={:?}", reports[left].compare(&reports[right]));
    }
}
'''
probe=root/'rust/tcl-compiler/examples/zz_capturefailureprobe.rs';assert not probe.exists();(out/'probe-source.rs.txt').write_text(source);before=subprocess.check_output(['git','diff','--','rust'],cwd=root);executables=[];record={'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),'started_unix':time.time(),'fixture':'samples/wasm/t3-procs/35_native_dispatch_matrix.tcl','scope':'Diagnostic only: three forced-old actual reports, immediate and later self comparisons; no guard or assertion changed.'}
env=dict(os.environ,CARGO_INCREMENTAL='0',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_PROFILE_DEV_OPT_LEVEL='1',CARGO_PROFILE_TEST_OPT_LEVEL='1',CARGO_PROFILE_DEV_DEBUG_ASSERTIONS='true',CARGO_PROFILE_DEV_OVERFLOW_CHECKS='true',CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true',CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true',CARGO_TARGET_DIR=str(root/'target-spectcl-opt1'),ZZ_OLD_ENV='1')
try:
 probe.write_text(source)
 with (out/'build.jsonl').open('w') as stdout,(out/'build.stderr').open('w') as stderr:code=subprocess.run(['cargo','build','--locked','-j2','-p','tcl-compiler','--example','zz_capturefailureprobe','--message-format=json'],cwd=root,env=env,stdout=stdout,stderr=stderr).returncode
 rows=[json.loads(line) for line in (out/'build.jsonl').read_text().splitlines()];warnings=[r for r in rows if r.get('reason')=='compiler-message' and r['message']['level']=='warning'];executables=[Path(r['executable']) for r in rows if r.get('reason')=='compiler-artifact' and r.get('executable')];record.update(build_exit=code,warnings=len(warnings));assert code==0 and not warnings and len(executables)==1
 with (out/'diagnostic.out').open('w') as log:record['run_exit']=subprocess.run([str(executables[0]),record['fixture']],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT).returncode
finally:
 probe.unlink(missing_ok=True)
 for exe in executables:
  exe.unlink(missing_ok=True)
  for alias in exe.parent.glob(exe.name+'-*'):
   if alias.is_file() and os.access(alias,os.X_OK):alias.unlink()
 record.update(finished_unix=time.time(),rust_diff_unchanged=before==subprocess.check_output(['git','diff','--','rust'],cwd=root),executables_removed=all(not p.exists() for p in executables),probe_removed=not probe.exists());(out/'final.json').write_text(json.dumps(record,indent=2)+'\n')
