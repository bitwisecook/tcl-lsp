# audit session scratch tools (WIP, not reviewed)

Kept from the `audit` session's scratchpad on 18 November 2026 so nothing lives only in a container.

- `sweep.sh`, `sweep-probe.sh`: the iRules false-positive sweep used for #1712's F5 measurement (before/after firing counts; 5,432 both). The firing lists themselves were deleted to free disk; rerun to regenerate.
- `run-crates.sh`, `run-reg.sh`, `run-groups.sh`: baseline/tip nextest runners writing `<name>.fail` / `.full` lists for diffing.
- `rd.py`: helper that inserts `retained_dispatch:` rows into registry files (used for `issues/audit-22`).
- `rm_pure.py`, `pure.tsv`, `purefiles.txt`, `*.patch`: tooling and intermediate data from retiring `Traits::PURE` and the C-evidence backfill (#1712, landed).
- `rule.irule`: the sweep's control probe.
- `dispatch3.tcl`: the menu `invoke` probe (also in `rust/tcl-registry/tests/data/tk_callback_dispatch/menu_invoke.tcl`).

wish builds: see "How to measure" in `docs/work/issue-sessions/handover/audit.md`. The wish evidence scripts and outputs are under `rust/tcl-registry/tests/data/tk_callback_dispatch/` on `issues/audit-22`.
