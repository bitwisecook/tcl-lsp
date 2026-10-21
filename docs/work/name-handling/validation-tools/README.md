# Validation tools

Scripts used to validate the LSP naming migration and to publish progress. They find the repository root with git and write working files to `tmp/name-handling-validation/` (git-ignored; override with `NH_VALIDATION_DIR`).

| Script | Purpose |
| --- | --- |
| `capture.sh <N> [name]` | Full direct-LSP capture: guarded build number `N` of the six LSP library test targets, then the 1,085 selectors (original 850, additive 230, five controls), one process per test, 180 s per test. Result: `$W/<name>/summary.json`. |
| `capture-inputs.py <out>` | Hash every tracked and untracked source file (excluding `docs/work/name-handling/`). Used as the source guard. |
| `capture-providers.py <out>` | Hash the tclsh 8.4–9.1 and Jim oracles and their libraries. Used as the provider guard. |
| `finalize-build.py <N>` | Check build `N` (logs in `tmp/logs/build<N>.*`) against the guards and record its test artifacts. |
| `run-lsp.py <name> lsp-build-<N>` | Run the capture against a finalized build. Selection manifests are extracted from the checkpoint09 archive and checked against pinned hashes. |
| `run-targeted.py lsp-build-<N> <out> <listing>` | Run an explicit `target selector` list against a finalized build, 180 s per test. |
| `write-report.py <delta.json>` | Write `progress-report-<UTC>.json` from the audit counts plus a delta file. |
| `ci-jobs.py <jobs.json>` | Summarise a GitHub Actions job listing: failing jobs and their failing steps. |

The guards fail by design if any source file changes during a build or capture. Run captures only while no other agent is editing the tree.

The latest full capture is capture48 (`experiments/name-handling/results/owner-adoption-20261018/lsp-generic-consumers-14/capture.json`): 870 pass, 215 fail, 2 timeouts. Compare a new capture against it per selector and report pass-to-fail and fail-to-pass rows.
