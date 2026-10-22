# Review triage evidence (22 Oct)

Evidence for the replay of the external review ([../naming-migration-review-20261022.md](../naming-migration-review-20261022.md)) on the current tree. Status per finding: [../naming-migration-review-20261022-triage.md](../naming-migration-review-20261022-triage.md); live progress: [../../outstanding/review-triage.md](../../outstanding/review-triage.md).

| Item | What it is |
| --- | --- |
| `run10-progress.log`, `run10-source-audit.log`, `run10-startup.log`, `run10-source.rs.txt` | Interim all-finding replay on `60ce7009`: closed prefix through N16 and static audit; active main log/output replay not yet complete. Startup wrapper correction is separately recorded. |
| `summarize-oracles.py` | Checks known generated outputs on matching engines and requires manual review for new fixture names; reproduces earlier 126/60 check counts. |
| `run7-pending.log`, `run7-progress.log`, `run7-pending-oracles.log`, `run7-pending-oracle-summary.json`, `run7-source-audit.log` | Complete bounded follow-up on `0434e7e7`, including P25/50/100 lenses; 180 engine executions / 60 matched generated checks, no unexpected result; static seam audit. |
| `run-focused.sh`, `run8-n09*` | Focused current-tree runner; initial N09 attempt failed before execution for a private helper. `run9-n09*` is the successful public-ID retry on `128e1bb4`: precise type/call facts, conservatively widened taint dependencies. |
| `run-final.sh`, `final-probe.rs.txt` | Prepared latest-work replay of every finding and seam, including a direct N09 taint-dependency projection and the existing fold e2e control. |
| `run-pending.sh`, `pending-probe.rs.txt` | Fresh build, seams first, then bounded R1/parser/source-chain/lens cases; deletes source after build and executable at exit. |
| `run6.log`, `run6-oracles.log`, `run6-oracle-summary.json`, `run6-source-audit.log` | Completed main snapshot (cap reached during scaling), full 576-execution replay, 126 matched checks with no unexpected result, and later static audit. |
| `run-r1-transport.sh`, `r1-transport-probe.rs.txt` | Prepared focused R1 transport/bootstrap and N03 parser-debt diagnostic; run after the active full replay. |
| `run6.sh`, `replay-oracles.sh`, `source-audit.py` | Prepared full replay, six-engine execution, and static seam evidence runners. |
| `cloud-baseline-oracles.log` | Fresh 19-file / 114-execution baseline replay; full output and exit codes. |
| `cloud-source-audit.log` | Current static excerpts; no server protocol claim. |
| `run5-e2e-repeat.log` | Existing live-fold e2e failure: DSL 1 expected ["5"], got []; stable repeat after the first active log lost diagnostics. |
| `run5-source.rs.txt` | Exact source launched by the initial cloud run5 (before the expanded follow-up). |
| `cloud-setup.log` | Cloud checkout, compiler and pinned engine versions; no new probe results. |
| `zz_review_probe.rs.txt` | Copy of the probe test (`rust/tcl-spectcl/tests/zz_review_probe.rs`); restore it under that path to re-run. |
| `part2.rs.txt` | Second probe batch draft. |
| `run5.sh` | **Resume batch**: restores the probe, builds+runs+deletes, logs to `run5.log` here (run from repo root). |
| `build.sh`, `buildrun.sh`, `run4.sh`, `oracle.sh` | Build/run scripts (build, run and delete in one command) and the tclsh/Jim oracle runner. |
| `run1.log` … `run4.log` | Filtered probe output per run. |
| `out/` | Edited sources and pack files produced by the probes (each run on tclsh 8.4–9.1 and Jim). |
| `tcl/` | Tcl repro scripts from the review. |

Evidence stays on `work` (decision 2026-10-21T12:05Z); it is not carried into the PR to `rust`.
