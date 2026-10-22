# Outstanding work: vm-runtime agent

Area: tcl-vm, runtime/rust, tcl-spectcl, tcl-spec-studio: SpecTcl execution and acceptance.

Last updated: 2026-10-22T12:22Z

Maintained by the vm-runtime agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## Resume

The evidence is in the repository under
`docs/work/name-handling/handoff/reviews/merge-inventory-20261019/vm-runtime/`:

- `seq.sh`: the runner. It builds, runs and deletes in one invocation, waits for ≥2 GB
  free disk, writes `runs/<TAG>.<test>.full.log`, `.status` and `<TAG>.summary`, and
  passes `$FILTER` to the binary.
  Usage: `TMO=3000 seq.sh TAG crate:test …`.
  The script hard-codes `S` (its log dir) to the old scratchpad; edit it to a writable
  directory first.
- `runs/*.status`, `runs/*.summary`: per-test pass/fail for every run this session.
  `before.codegen_stamps.log` holds the codegen_stamps panics before my changes.
- `sol48-baseline/`: the previous agent's "before" per-test statuses. Compare
  `before.tcl-vm---lib.status` (1107/1387) with a new run to list every test that passed
  before and fails now.
- `zz_vmrt_probe.rs.txt`: the golden-render probe. To regenerate goldens:
  1. Copy it to `rust/tcl-spectcl/tests/zz_vmrt_probe.rs`.
  2. Build and run it with `ZZ_OUT=<dir>`.
  3. Diff `<dir>/*.snap` against `rust/tcl-spectcl/tests/golden/`.
  4. Classify the differences with `gold/reconstruct.py`, which needs the `xxhash` Python
     module (I ran it with `/usr/bin/python3`, i.e. 3.13) and the `old.json`/`new.json`
     files from `gold/structs.py <rev>`. Use `gold/nested_cont.py` for the fold set.
  5. Copy only the accepted files.
  6. Delete the probe.

Current step: the thread-exit abort fix. Re-run it with:

```
cd /home/user/tcl-lsp && CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 \
  cargo test --locked -p tcl-vm --test thread_exit_retirement -j 2 -- --test-threads 1
```

To check that the test reproduces the bug, temporarily make `native_retirement::release`
use `PENDING.with` again; the test binary should then die with SIGABRT.

Then continue, in this order:

1. The Jim/EDA targets:
   `seq.sh after8 tcl-spectcl:jim_document tcl-spectcl:jim_own_surface tcl-spectcl:jim_surface_roster tcl-spectcl:jim_generation_zero tcl-spectcl:jim_seed_after_publish tcl-spectcl:eda_loadables tcl-spectcl:eda_environment_packs`
2. `spec_corpus`:
   `FILTER="--exact every_shipped_tclspec_loads_installs_and_analyses_against_corpus" THREADS=1 seq.sh after9 tcl-spectcl:spec_corpus`
3. `codegen_stamps`:
   `FILTER="--skip every_shape" seq.sh after9 tcl-spectcl:codegen_stamps`
4. The runtime, with libtommath:
   `cd runtime/rust && TCL_TOMMATH_DIR=../../tmp/tcl9.0.4/libtommath cargo test --locked --features engine`
   Run one binary at a time where possible.
5. The runtime without libtommath:
   `unshare -m sh -c 'mount --bind <emptydir> tmp/tcl9.0.4/libtommath && cd runtime/rust && cargo test --locked'`
6. tcl-vm, tcl-engine-tclvm and embed:
   `seq.sh after10 tcl-vm:embed_api_e2e`, plus `cargo test -p tcl-vm --lib` and
   `cargo test -p tcl-engine-tclvm --lib`, each built and run in one command.
7. Clippy.

## In progress

- **PRIORITY (from the lsp agent): process abort when a VM is dropped at thread exit.**
  - The `pack_hooks` `HOST` thread-local drops a `HookHost<TclVmEngine>` → `Vm` →
    `Value` retirement, which touches an already-destroyed thread-local, and the process
    aborts (SIGABRT).
  - Fix in place (`rust/tcl-vm/src/value/native_retirement.rs`, `value/native_jim_pool.rs`,
    `value.rs`): use `try_with` throughout and leak rather than panic.
  - New regression test: `rust/tcl-vm/tests/thread_exit_retirement.rs`.
  - First run: 1/2, but it failed as a test rather than aborting. Diagnosing now.

## Outstanding (in priority order)

0. **Review C.2 Jim/EDA acceptance targets.** Paused for the priority bug. Run:
   - `jim_document`
   - `jim_own_surface`
   - `jim_surface_roster`
   - `jim_generation_zero`
   - `jim_seed_after_publish`
   - `eda_loadables`
   - `eda_environment_packs`
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
