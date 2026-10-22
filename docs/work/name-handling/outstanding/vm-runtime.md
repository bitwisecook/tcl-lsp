# Outstanding work: vm-runtime agent

Area: tcl-vm, runtime/rust, tcl-spectcl, tcl-spec-studio: SpecTcl execution and acceptance.

Last updated: 2026-10-22T15:52Z (test-only D4 case implemented, unexecuted; waiting for the VM build slot)

Maintained by the vm-runtime agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## Resume

Resumed at 14:21Z in `/workspace/naming-migration-active/tcl-lsp`; root rebased and
pushed the full WIP/report as `41e547bcb`. Waiting for root's explicit build release:
compiler-names, compiler-codegen, compiler, VM-runtime, coordination checks, then
lsp. No VM-runtime cargo process has started. First run remains the six Jim/EDA
targets (32 cases), then rerun and triage `jim_document` (26 cases). No current
result below is a guarded validation or audit admission.

Six-vendor EDA smoke is prepared in `spec_packs.rs` and included in the WIP, with
all eight pack names, own-vendor plus shared SDC/UPF availability and rival-vendor
exclusion checked in one server process. It is unmeasured; section D stays open.

Accepted D4 protocol case is now implemented as the child module
`rust/tcl-lsp-server/tests/e2e/spec_packs/environment_lifecycle.rs`, linked from
the owned `spec_packs.rs`. It changes folder configuration, the workspace-wide
custom pack and dialect sidecars through change/remove/restore while all three
open document versions remain 1 and their bytes plus the unopened provider stay
exactly fixed. It compares first warm/fresh naming, callback, class, signature,
role and live-hook answers. Readiness uses current configuration/publication
and raw read-only `currentDiagnostics`; exact old/new identities are checked
through `validateDiagnosticPublication` before and after the fixed provider
sequence. No naming-answer polling, new sleeps or production instrumentation.
LSP owns the complementary controlled stale-worker completion regression;
transport silence is not a retirement proof. Rustfmt parses/formats the test;
it has not compiled or run, and grants no section D admission.

New release blocker: external strict Clippy on `aeecf39743b` with Rust 1.99.1
reported **379 VM diagnostics across 72 paths**. Inventory:
`/tmp/naming-clippy-vm-runtime.json`; canonical external receipt is
`../handoff/reviews/merge-inventory-20261019/bigip-lsp/clippy-dependency-blockers-20261022.json`.
These are old-snapshot observations, not a current source measurement. Prepare the
cleanup off-tree during the source hold, then fix it after SpecTcl/Jim triage;
no new lint allowances. A selected `--no-deps` run does not clear the full gate.

Actual mechanical shadow patch is ready at
`/tmp/vm-runtime-lint-shadow/mechanical.patch`, SHA-256
`6d93623097aff7c10061ca9138c4eff0be15a271f30fd1a6724051fd567762c9`.
It contains **149 edit groups across 44 files**: identity conversions, docs,
unit patterns/semicolons, method closures, unnecessary borrows, declaration
placement and small equivalent combinators. The receipt beside it records
current base/shadow hashes and every edit. All 44 shadows parsed/formatted with
rustfmt; `git apply --check` passes and base drift is zero at 15:22Z. This is
off-tree only, with no typecheck, Clippy count, behavioural result or canonical
VM source edit. TLS retirement `try_with` guards are untouched. Refusal-sensitive
and structural helpers remain in the separate plan for review after acceptance.

The scratch runner is `/tmp/vm-runtime-preflight/seq-safe.sh`: one `cargo test`
build/run per target, draining filtered logs capped at 20 MB, recording exit and
warning counts, no deletion of fresh shared executables. Source
`/tmp/tcl-lsp-naming-env.sh` first. The old runner below is historical and has
hardcoded paths plus unsafe fresh-executable cleanup; do not execute it unchanged.

Narrow additional ownership agreed with lsp: extend
`rust/tcl-lsp-server/tests/e2e/spec_packs.rs` bundled EDA smoke to six vendor
environments. Server lib and general section D lifecycle seams stay with lsp.

**After8 (review C.2 Jim/EDA targets).**
- Finished: `jim_document` **9 passed / 17 failed / 26**. The panic summaries are in
  `…/vm-runtime/runs/after8.jim_document.failures.txt`. They are W123 "class" unknown in
  Jim documents, missing class/method facts and `all_classes` entries, i.e. Jim
  class/document analysis. No pre-change baseline exists. Not yet triaged: owner is
  probably the compiler/LSP side, VM to be ruled out.
- Not run: `jim_own_surface`, `jim_surface_roster`, `jim_generation_zero`,
  `jim_seed_after_publish`, `eda_loadables`, `eda_environment_packs`.

**Exact next command after root releases the VM slot.** Active logs stay under `/tmp`;
copy completed receipts into the evidence tree only after the target finishes:

```
source /tmp/tcl-lsp-naming-env.sh
TMO=1800 bash /tmp/vm-runtime-preflight/seq-safe.sh after8b-resumed tcl-spectcl:jim_own_surface tcl-spectcl:jim_surface_roster \
  tcl-spectcl:jim_generation_zero tcl-spectcl:jim_seed_after_publish tcl-spectcl:eda_loadables \
  tcl-spectcl:eda_environment_packs
```

Then:
1. Triage the `jim_document` failures.
2. `FILTER="--exact every_shipped_tclspec_loads_installs_and_analyses_against_corpus" THREADS=1 seq.sh after9 tcl-spectcl:spec_corpus`
3. `FILTER="--skip every_shape" seq.sh after9 tcl-spectcl:codegen_stamps`
4. The runtime with libtommath:
   `cd runtime/rust && TCL_TOMMATH_DIR=../../tmp/tcl9.0.4/libtommath cargo test --locked --features engine`
5. The runtime without libtommath:
   `unshare -m sh -c 'mount --bind <emptydir> tmp/tcl9.0.4/libtommath && cd runtime/rust && cargo test --locked'`
6. `cargo test -p tcl-vm --lib`, and compare it with
   `…/vm-runtime/sol48-baseline/before.tcl-vm---lib.status`.
7. `cargo test -p tcl-engine-tclvm --lib`
8. `seq.sh after10 tcl-vm:embed_api_e2e`
9. Clippy `-D warnings` on my crates.
10. Finish `vm-runtime-NOTES.md`, which has `RESULTS`/`COMPILER`/`REMAINING` placeholders.

Re-check the thread-exit fix with:
`cargo test --locked -p tcl-vm --test thread_exit_retirement -- --test-threads 1` (2/2).

**Files touched this session** (all uncommitted edits are now in the working tree for the
coordinator's commit):
- `rust/tcl-spectcl/src/loader/eval.rs`: `static_row`, the fast-path brace fold.
- `rust/tcl-spectcl/tests/`:
  - `codegen_stamps.rs`: `PlainCounting` forwarding.
  - `upf_hooks.rs`: new.
  - `spec_corpus.rs` and `spec_corpus_baseline.txt`: oo-class lines removed.
  - `golden/*.snap`: regenerated.
- `rust/tcl-vm/src/value/native_retirement.rs`, `value/native_jim_pool.rs`, `value.rs`:
  the thread-exit `try_with` changes.
- `rust/tcl-vm/tests/thread_exit_retirement.rs`: new.
- `rust/tcl-vm/src/cmd_package.rs`: `version_conflict` takes the protocol release.
- `rust/tcl-engine-tclvm/src/lib.rs`: a doc comment.
- `runtime/rust/src/{value_ops.rs,interp/native_variable_names.rs,interp/native_body_artifact.rs}`:
  redundant `#[path]` attributes removed. NOT yet compile-checked; that is runtime step 4.
- `docs/design/spec-dsl-examples/{oo-class,upvar,lsort,switch,foreach,return,string}.tclspec`
  and `README.md`.
- `rust/tcl-spec-studio/tests/{spectcl_ports,pack_store}.rs`.

**Evidence** is under `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/vm-runtime/`:
- `runs/*.summary`, `*.status` and `after8.jim_document.failures.txt`;
- `sol48-baseline/` (per-test before statuses);
- `gold/` (classification scripts and table);
- `zz_vmrt_probe.rs.txt` (the golden-render probe);
- `seq.sh`.

## In progress

- Waiting for compiler-names, compiler-codegen and compiler C6 build slots in that
  order; root owns sequencing. The initial waiting flock was cancelled before any
  VM-runtime cargo process started. Active logs will remain under `/tmp` and only
  completed receipts will be copied into evidence. Six-vendor EDA smoke is prepared;
  planning the external Clippy inventory off-tree without dependency source edits.
  The remaining Jim/EDA targets are queued. Earlier `jim_document` is 9/17/26;
  its failures include signature/static/body analysis and disabled-command diagnostics
  as well as class facts. This target does not execute VM code or external oracles.

## Outstanding (in priority order)

1. **`spec_corpus` `every_shipped_tclspec_loads_installs_and_analyses_against_corpus`.**
   - Rerun it alone, single-threaded. Its last run hit the 600 s watchdog under load.
   - It must confirm the `spec_corpus_baseline.txt` edit, which removed the 9 obsolete
     oo-class lines and their header paragraphs.
2. **`codegen_stamps`: collect the failure messages.**
   - Rerun with `--skip every_shape`. The last run was 5 ok / 23 failed / 1 still running
     at the 1500 s cap; the before figure was 1/28.
   - Wait for compiler-codegen's `install.rs` native_compilation change, then re-classify
     which failures are VM-side and which are compiler-side.
3. **runtime/rust.**
   - `cargo test --locked --features engine` with libtommath: the lib `string` tests,
     `integer_argument_reads`, `literal_index_encoding`, then the rest.
   - The no-libtommath configuration: hide the tree only in a private mount namespace
     (`unshare -m` + bind-mount an empty dir), never by moving the shared tree.
   - These also compile-check this session's redundant `#[path]` removals.
4. **tcl-vm lib** (Sol-48 baseline 1107/1387, per-test log retained),
   **tcl-engine-tclvm lib** and **embed_api_e2e**.
   - These check the Sol-48 commands-budget change and the `cmd_package`
     `version_conflict(release)` change.
   - Report every test that passed before and fails now.
5. **Clippy `-D warnings`** on changed lines in my crates (whole-crate where cheap), and
   `cargo fmt` (edition 2021 for runtime/rust).
6. **The vm-runtime handoff.**
   - `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/vm-runtime-NOTES.md`
     is drafted; fill in the results.
   - Write `vm-runtime.patch`.

## Blocked (on whom, and what is needed)

- **`pack_source_e2e` 3/6** (fold DSL 1, reference body, derived body), on compiler-codegen:
  - `optimise_raw` never invokes installed pack const_fold hooks. The host error log and
    crash log are empty, and there is no O129.
  - The TclVmEngine sandbox refuses `string cat` / `string is` bodies with
    `NativePreflightRequired` on the entry-aware route. `string length`, `lindex` and
    `expr` work.
  - Both are routed by the coordinator.
- **`spec_corpus` hostile-pack test** (hooks attempted 0/0): same root cause as the
  `optimise_raw` hook gap, on compiler-codegen.
- **`spec_corpus` `shared_unit_matches_legacy_for_representative_profiles`**: on the
  compiler agent. tcl8.6 `if {$value}` gives I230 from the legacy path, but the shared
  path gives nothing.
- **`codegen_stamps` remaining failures**, on compiler-codegen:
  - `BytecodeCompileService::default().compile()` refuses every source;
  - a whole-module compile emits no procedures or provenance and no pack
    specialisation/claims;
  - native_compilation is not derived for pack rows (`install.rs`, in progress there).
  - `every_shape_of_inlined_body…` takes more than 25 min, because every compile
    re-assembles `ContextRegistry` (compiler lowering cost).
- **N17** (codegen `$a:b` variable grammar → wrong registration): compiler-codegen.

## Done since the last update (with evidence: test counts, commit or snapshot)

- **Thread-exit abort fixed** (priority, from the lsp agent).
  - The drop paths in `tcl-vm` `value/native_retirement.rs`, `value/native_jim_pool.rs` and
    `value.rs` (`DoubleFormatContext`) now use `LocalKey::try_with`, and leak instead of
    panicking once a thread-local is gone.
  - New `tcl-vm` test `thread_exit_retirement` passes 2/2.
  - Negative control: with only `release` reverted to `PENDING.with`, the test binary
    aborts with the reported AccessError (exit 134).

The state is in WIP snapshot `56893780b9`, plus the working tree since.

- **`eval_loader` 28/28** (was 27/1/28).
  - Static fast path folds braced backslash-newline (`loader/eval.rs` `static_row`).
  - Checked against tclsh 8.4 to 9.1, which all fold.
- **`golden_packs` 3/3** (was 1/2/3).
  - The goldens were regenerated twice under the classification rule: 1367 digest-only
    commands (proven by hash), 148 fold commands, and the accepted foreach line plus the
    oo-class port.
  - After the port pass, only the intended fields changed: foreach, lsort, return,
    string, switch, upvar.
- **`pack_is_real_tcl` 1/1.**
- **`upf_hooks` 2/2 (new)**: all 17 UPF hook slots run on the production TclVM host with
  exact expected answers.
  - 16 option-arity hooks: optional `TRUE|FALSE` or name, consume 1/0.
  - The `define_power_model` resolver marks the trailing Body, and abstains for one word.
- **tcl-spec-studio `spectcl_ports` 12/12** (was 10/12) and **`pack_store` 7/7** (was 6/7).
  - The oo-class port uses the current vocabulary.
  - The upvar port uses `-level-word Upvar`: tclsh 8.4/8.5 give "wrong # args" for
    `upvar 1 b`, while 8.6+ alias it.
  - Ports aligned with their shipped specs: lsort (and moved to vocabulary 2.1), switch,
    foreach, return, string `is` refine.
  - Non-authorable fields are documented, never loosened.
- **`codegen_stamps` harness**: `PlainCounting` now forwards every compile entry. This is
  +4 passes, with no regressions.
- **Other code changes**:
  - `cmd_package` `version_conflict` takes the protocol release.
  - Redundant runtime `#[path]` attributes removed (owner gate).
  - The engine wall-clock test comment is corrected.
