# Actual C8.6 real naming suites

Twenty fresh bounded processes measure unchanged real C8.6.18 init.tcl, Tcltest 2.5.11, eight complete upstream naming files, and the pinned current Native C8.6 runner. Actual C controls complete all eight files with **1182 passes / 85 real harness skips / 0 failures**. Native completes proc and rename with **40 passes / 17 skips / 0 failures**; five files stop at host refusals and ooNext2 exits with an ordinary script error. Four incomplete Native files retain 34 assertion-failure IDs. There is no full Native total or pass rate.

| Full-file question | C passed / skipped / failed | Native evidence | Native completion and last started ID |
| --- | --- | --- | --- |
| [var](questions/var.md) | 90 / 20 / 0 | No final summary; 0 observed assertion failures | host-refusal; `var-1.15` |
| [proc](questions/proc.md) | 29 / 9 / 0 | 29 / 9 / 0 | completed-suite; `proc-7.6` |
| [rename](questions/rename.md) | 11 / 8 / 0 | 11 / 8 / 0 | completed-suite; `rename-6.1` |
| [upvar](questions/upvar.md) | 57 / 8 / 0 | No final summary; 0 observed assertion failures | host-refusal; `upvar-1.1` |
| [namespace](questions/namespace.md) | 310 / 3 / 0 | No final summary; 12 observed assertion failures | host-refusal; `namespace-51.13` |
| [interp](questions/interp.md) | 339 / 15 / 0 | No final summary; 9 observed assertion failures | host-refusal; `interp-11.4` |
| [oo](questions/oo.md) | 289 / 17 / 0 | No final summary; 10 observed assertion failures | host-refusal; `oo-7.10` |
| [ooNext2](questions/ooNext2.md) | 57 / 5 / 0 | No final summary; 3 observed assertion failures | process-error; `next-tailcall-simple-1` |

Both separate real-init controls reach exact `NH_INIT\n`, exit 0 and empty stderr. The unchanged actual all.tcl single-file orchestration probe completes C var with 90/20/0. Native reaches real runAllTests but errors at `invalid command name "ParseFormatArgs"` before any test, while formatting the start timestamp. [All.tcl question/result](questions/all-tcl.md), [Native all.tcl record](runtime/all-tcl-var/record.json) and [stderr](runtime/all-tcl-var/stderr.txt) establish that boundary; no all.tcl pass claim follows.

To reach actual suite questions independently of that clock orchestration gap, the eight paired wrappers directly source unchanged matching Tcltest and complete unchanged files, with real `configure -singleproc 1` and `interp debug {} -frame 1`, following upstream [tests/all.tcl](originals/tests/all.tcl.gz). Each stream verifies `singleProcess=1 singleTestInterp=1`. This standard upstream in-process constraint causes the two interp default-frame cases to skip; it is not a backend overlay. No case-ID filtering, mock package, source patch, procedure injection or Logical adapter applies.

[Provenance](provenance.json), [per-process records](records.json), [ordered failures/refusal boundaries/skips](observations.json), [validation](validation.json), [capture script](capture.py) and [Native runner source snapshot](native-runner-source.rs) retain exact hashes and invocations. Compressed originals retain upstream copyrights and licence, including actual all.tcl and the package index. [Library inventory](library-source-hashes.json.gz) pins other upstream library bytes. The runner SHA-256 is `fb1ae94b86854a4f56476011dd50ead92ec7c94fac5aa721d678e72bb15885c6`; frozen special_vars/cmd_mathfunc owner bytes match before and after.

Processes are serial, fresh, in temporary working directories, with 60-second timeout/5-second kill grace, 1 GiB address-space bound and disabled core dumps. No process reaches timeout, crash, OOM or stream review threshold. No build, source repair, Git/network operation or F5 query occurs. Startup, harness entry, assertion failures, host refusals and suite summaries are kept distinct. Other releases, Jim, F5, Logical and VM provide no answer to these exact files.

Two retained diagnostic attempts establish upstream harness coupling: [frame-enabled without singleproc](../20261012T192755Z/README.md), then [default-frame direct source](../20261012T193042Z/README.md). Neither is used for final conformance totals.
