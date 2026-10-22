# Review triage evidence (22 Oct)

Evidence for the replay of the external review ([../naming-migration-review-20261022.md](../naming-migration-review-20261022.md)) on the current tree. Status per finding: [../naming-migration-review-20261022-triage.md](../naming-migration-review-20261022-triage.md); live progress: [../../outstanding/review-triage.md](../../outstanding/review-triage.md).

| Item | What it is |
| --- | --- |
| `zz_review_probe.rs.txt` | Copy of the probe test (`rust/tcl-spectcl/tests/zz_review_probe.rs`); restore it under that path to re-run. |
| `part2.rs.txt` | Second probe batch draft. |
| `build.sh`, `buildrun.sh`, `run4.sh`, `oracle.sh` | Build/run scripts (build, run and delete in one command) and the tclsh/Jim oracle runner. |
| `run1.log` … `run4.log` | Filtered probe output per run. |
| `out/` | Edited sources and pack files produced by the probes (each run on tclsh 8.4–9.1 and Jim). |
| `tcl/` | Tcl repro scripts from the review. |

Evidence stays on `work` (decision 2026-10-21T12:05Z); it is not carried into the PR to `rust`.
