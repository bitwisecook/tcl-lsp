# Lane: WASM native lowering programme

Tracking document for the phased plan in
[`docs/design/compiler/wasm-native-lowering-plan.md`](../compiler/wasm-native-lowering-plan.md).
One sub-lane per phase; each keeps its own section here. Protocol: AGENTS.md
"Long-running agent lanes" (compile before every commit, stage by explicit
path, `wip(<lane>):` prefix, lanes commit locally, the orchestrator pushes).

Branch: `claude/wasm-codegen-architecture-5exvpu`.

## Sub-lanes

| Lane | Phase | Owner files | Status |
|---|---|---|---|
| `p0-harness` | P0 tier harness + runtime unit suite in CI | `rust/tcl-compiler/tests/wasm_tiers.rs`, `rust/tcl-compiler/tests/common/wasm_link.rs`, `samples/wasm/budgets.tsv`, `.github/workflows/ci.yml`, `scripts/dev/runtime-rust-path.sh`, `runtime/rust/examples/run_script.rs`, `runtime/rust/tests/run_script_builtin_surface.rs` | **done** — see below |
| `p1-runtime-abi` | P1 runtime ABI v2 groundwork | `runtime/rust/src/{codegen_abi,frame,vars,interp,obj,bignum,builtins,expr}.rs`, `rust/tcl-runtime-api/src/codegen_abi.rs` | open |
| `p2-executable-ir` | P2 executable IR total | `rust/tcl-compiler/src/executable_ir.rs` and its consumers | open |
| `p3-native-lowering` | P3 NLIR + native T0/T1 | new `rust/tcl-compiler/src/native_lowering/`, `codegen/wasm/backend.rs` | blocked on P1, P2 |

## Decisions

- Compiled-code activations count as eval-loop activations: the runtime's
  "outermost eval" rule (`interp.rs` eval loop, depth 0) must never fire
  inside a `tcl_invoke_argv` dispatched from generated code. The fix is an
  activation record the ABI enters and leaves, not a special case in `catch`.
- Compiled procs are reached from the runtime through the shared wasm
  function table: the user module imports the runtime's table, grows it,
  installs its functions, and registers `(name, params, table index)`; the
  runtime treats the index as an `extern "C"` function pointer. Falls back to
  the source body when the native entry declines.
- No emitter reads a compatibility string. `whole_var_reference` and the
  `name.contains('(')` gates in `codegen/wasm/backend.rs` are retired by P3;
  word shapes come from `WordExpr` only.

## Site inventory and status

Filled in by each sub-lane as it lands.

## p0-harness

Status: **done**. P0's acceptance harness, the framing goldens, and the two
CI/infrastructure issues (#1768, #1589) are landed.

### Delivered

- **`rust/tcl-compiler/tests/wasm_tiers.rs`** — every `samples/wasm/t*/*.tcl`
  compiled with `WasmCompileOptions::standalone(true)` and again with
  `SemanticOptimisationPassId::LegacyAnalysisSpecialisation`, linked against
  the real `tcl_runtime.wasm` (`wasmtime run --preload tcl=…`), stdout diffed
  byte for byte against `samples/wasm/expected/<tier>/<name>.out`. 72 runs.
  Gated exactly as `wasm_real_link.rs`: loud, dimension-naming skip when the
  toolchain is absent; hard failure under `TCL_REQUIRE_WASM_LINK=1`.
- **`rust/tcl-compiler/tests/common/wasm_link.rs`** — the gate
  (`missing_requirements`/`real_link_runtime`), the `--global-base=0x200000`
  reserved-runtime build, and the per-checkout/per-process `scratch` paths,
  *moved* out of `wasm_real_link.rs` into the existing `tests/common/`
  structure and consumed by both suites. Not duplicated: a divergent copy is
  how a suite ends up linking a foreign `tcl_runtime.wasm` (#1590) or skipping
  silently in CI (#1542).
- **`samples/wasm/budgets.tsv`** — committed golden, one row per sample per
  plan: `call` sites reaching `tcl_eval_code` / `tcl_expr_bool` /
  `tcl_invoke_argv`, plus native 64-bit numeric instruction count. Computed by
  walking the emitted `WasmModule`'s `functions[].body` and resolving `call`
  operands against import indices — never by regexing the WAT. Drift fails;
  `UPDATE_WASM_BUDGETS=1 cargo test -p tcl-compiler --test wasm_tiers
  framing_budgets` regenerates. The budgets test needs no wasm toolchain, so
  framing is measured in every partition that builds `tcl-compiler`.
- **#1768** — `runtime-rust-tests` CI job running `make runtime-rust-test`
  (now `--locked`, with `TCL_TOMMATH_DIR` passed explicitly). Step-level skip
  on a new `runtime_rust_changed` channel output; the job always reports.
  `scripts/dev/runtime-rust-path.sh` carries the closure,
  `scripts/dev/test-runtime-rust-paths.sh` re-derives it from
  `runtime/rust/Cargo.lock` and asserts the CI wiring
  (`make check-runtime-rust-paths`, wired into `xtask-check`). Documented in
  AGENTS.md's deep tier and CI redundancy contract.
- **#1589 (second half)** — `runtime/rust/examples/run_script.rs` already
  bootstraps through `Interp::new()`, which runs `builtins::install` and so
  registers `if`/`catch`/`while`/`foreach`/… The reported gap is closed; what
  was missing was anything *keeping* it closed.
  `runtime/rust/tests/run_script_builtin_surface.rs` now pins it three ways:
  the control-flow commands are registered, a plain `if`/`catch` sheet
  evaluates, and the example must use `Interp::new()` and must not call
  `register_builtin` itself. The example's module doc says so and names the
  one remaining conditional gap (`expr` is `have_tommath`-gated).

### Decisions

- **The expected-divergence ledger fails in both directions.** A sample that
  starts diverging fails (regression); a listed sample that *stops* diverging
  also fails, with "delete its entry in the same commit that fixed it". An
  xfail list that silently absorbs fixes rots into a list of things nobody
  remembers were broken, and the phase that fixed one gets no gate.
- **Every ledger entry names a defect**, and a well-formedness test rejects an
  empty reason. The table holds filed bugs, not tolerances.
- **The budget walks the IR, not the WAT.** A regex over `to_wat()` counts
  calls named in data strings and silently starts counting nothing the day the
  formatter changes shape. Import *names* come from `CodegenAbiImportId`
  descriptors, so renaming one in the shared ABI is a compile error here
  rather than a zeroed column.
- **`native_i64_f64` is a prefix rule** over `WasmOp::wat_name` (`i64.`/`f64.`
  minus `const`/`load`/`store`/`extend_i32_s`), so the `f64` arithmetic P3 adds
  is counted the day it is emitted, with no second edit to forget.
- **CI runs the shared make target**, not an inline cargo line, so a
  contributor reproducing a failure runs the identical command.
- `TCL_TOMMATH_DIR` is passed explicitly by `make runtime-rust-test`, and the
  contract test enforces it: `runtime/rust`'s `build.rs` degrades *silently* to
  a bignum-less build that un-registers `expr` entirely, so a green run without
  it would be far weaker than it looks — #1542's shape, on a different gate.

### Measured baseline (at `4a0b9d58`)

| Plan | byte-identical | divergences |
|---|---|---|
| default | 34 / 36 | `70_var_traces` (#1633 row 3), `73_coroutine` (no wasm coroutines — P9) |
| analysis | 30 / 36 | those two plus `11_while_loop`, `20_lists`, `24_regex`, `41_upvar` — all one defect: §2.2's `puts` compatibility-text reparse (P3) |

**§2.2 of the plan document is now one row out of date.** It records 29/36 for
the analysis plan and lists `50_catch_error` among the divergences. The
`p1-runtime-abi` lane's "a compiled activation is an eval-loop activation"
commit closed §2.2's second defect, and this suite's stale-entry check caught
the ledger row going out of date the moment it did — the mechanism working as
designed on its first day. The plan document's table should be corrected to
30/36 when §2.2 is next touched; it was left alone here because another lane
holds that file.

### Remaining / not done

- The plan document's §2.2 table (above) — deliberately left to its owner.
- `wasm_tiers.rs` is not yet wired into a CI job. It belongs beside
  `wasm_real_link.rs` in the `wasm-real-link` job (same toolchain, same
  `TCL_REQUIRE_WASM_LINK=1`), but that job's step is currently a single
  `--test wasm_real_link` line and adding a second test target there is a
  change to a step another lane may be editing; it is a one-line follow-up.
- #1716 (wasm32-capable clang on macOS), listed against P0 in §7, is a
  developer-environment issue with no in-repo surface here and was not
  attempted.

### Verification (clean checkout at `4a0b9d58` + this lane's files only)

`TCL_REQUIRE_WASM_LINK=1 cargo test -p tcl-compiler --test wasm_tiers` 5/5,
`--test wasm_real_link` 8/8, `make runtime-rust-test` 582+3 pass,
`cargo clippy -p tcl-compiler --tests -- -D warnings` clean,
`cargo fmt --check` clean, `make runtime-rust-lint` clean,
`bash scripts/dev/test-runtime-rust-paths.sh` pass.

Those runs were made in a detached worktree pinned to `4a0b9d58`, because the
shared worktree carries other lanes' in-flight edits to
`rust/tcl-compiler/src/executable_ir.rs` and `runtime/rust/src/**` that did not
compile at the time. A worktree at HEAD plus this lane's own files is exactly
what this lane's commit produces, so it is the correct thing to measure.
