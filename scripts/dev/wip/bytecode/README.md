# bytecode issue session: working files

Kept from the paused session's scratchpad; see docs/work/issue-sessions/handover/bytecode.md.

- `suite.sh <label>`: bytecode/codegen/vm/registry/explorer/cli suites; writes `<label>.log` and `<label>.fail` under `$OUT` (default `target/bytecode-wip`).
- `csuite.sh <label>`: tcl-compiler + tcl-codegen, incremental off, 3 jobs.
- `corpus.py <out.json>`: run from the repo root with `python3 -I`, after `cargo build -p tcl-cli --bin tcl` and `scripts/capture/bytecode.sh`; prints the per-release scoreboard and dumps both listings per snippet.
- `baseline-vm.fail`: 230 failing names from `suite.sh` at a50d3b1e4 (18 Nov).
- `baseline-compiler.fail`: 2,289 failing names from `csuite.sh` (17 Nov, before e21d3aa1a).
- `corpus-wip.json`: `corpus.py` output at 7a7f1434a (62/65/66/65).
- No 8.4 debug-build notes exist beyond the hand-over's "8.4 plan"; the build was not attempted.

Compare a run with a baseline: `comm -13 baseline-vm.fail new.fail` (new failures), `comm -23` (fixed).
