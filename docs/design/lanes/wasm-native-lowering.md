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
| `p2-executable-ir` | P2 executable IR total | `rust/tcl-compiler/src/executable_ir.rs` and its consumers | landed (see below) |
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

### The harness is red on the branch as landed — and that is its first find

`e422f4b0 wip(p2-executable-ir): structured control as executable edges`
landed between this lane's measured baseline and its commits, and it is not
green:

- `build_linear_executable_ir` trips its own
  `debug_assert!(executable.validate().is_ok(), …)` at
  `executable_ir.rs:2162` with
  `ValueNotAvailableOnAllPaths(ExecutableValueId { index: 15 })` while
  compiling a `samples/wasm` script, which panics all three sample-driven
  `wasm_tiers` tests (both plans and the budget regeneration);
- `wasm_real_link.rs`'s `canonical_generic_argv_runs_against_the_real_runtime`
  and `guarded_boxed_intrinsic_runs_and_falls_back_against_the_real_runtime`
  fail with `NoViablePlan { operation: Intrinsic(StringLength), … failure:
  Selector(InconsistentCompletionReturn) }` — the literal program no longer
  selects the generic-argv plan those cases exist to prove.

Both were reproduced on a clean worktree checked out at `e422f4b0` itself,
with this lane's files absent — so neither is caused by anything here, and the
`wasm_real_link` pair reproduces against that file's *unmodified* pre-move
form. `p2-executable-ir` owns `executable_ir.rs` and owns the fix; this lane
must not edit it.

That the breakage is visible at all is the point of P0. Before `wasm_tiers.rs`
existed, a compatibility-builder regression that silently stops compiling
`samples/wasm` scripts had nothing pointed at it: `wasm_real_link.rs` compiles
six hand-written snippets, not the corpus. The budgets golden cannot be
regenerated, and this lane's ledger cannot be re-measured, until `e422f4b0`'s
`debug_assert` is fixed — `samples/wasm/budgets.tsv` as committed is the
`4a0b9d58` measurement and will need a reviewed regeneration once P2 is green
again.

## p1-runtime-abi

Status: **all five deliverables landed.** Every commit kept
`make runtime-rust-test` and `make runtime-rust-lint` green, plus
`cargo test -p tcl-runtime-api` for the descriptor changes.

### Done

1. **Activation accounting** (`2d2aec37`) — fixes §2.2's live defect.
   `tcl_invoke_argv` / `tcl_intrinsic_invoke_argv` dispatched at
   `eval_depth == 0`, so a dispatched `catch` had the outermost-eval rule fire
   *inside* its body and lost its `-errorcode`. Both now hold an activation
   across dispatch (`AbiActivation`), so today's generated code is correct
   without emitter changes; the guard's `Drop` subsumes the ad-hoc
   `publish_error_if_uncaught` call, which is removed.
2. **Numeric write-back, typed value ABI, boolean owner** (landed inside
   `4a0b9d58`, see *Notes* below) — `bignum::read` caches the parsed rep back
   onto the object exactly as `TclParseNumber` does (in place, string rep
   kept); `obj::may_cache_parsed_rep` owns the "may we" question (a plain
   string, or any unshared object). New `runtime/rust/src/typed_value.rs` is
   the one owner of "read this object as an integer / double / boolean", with
   C's messages and `-errorcode`s. Closes #1425's runtime half: the four
   private boolean word tables (`value_ops::as_bool`,
   `cmd_namespace::parse_bool`, `interp::parse_truth`, `cmd_dict`'s tower-less
   `dict_filter_bool`) are gone, all routed through `tcl_syntax::boolean`.
3. **Expression AST cache** (`ea486a3e`) — `TCL_EXPR_TYPE` holds the parsed,
   validated `ExprNode` behind an `Rc` plus the release it was admitted under.
   A twenty-iteration `while` now parses its condition once.
4. **Real indexed slots** (`baeb9634`) — `VarTable` cells live in a
   slot-indexed array with an ordered name → slot side table; cells are
   reserved, never removed, so an `unset` and re-created variable refills the
   same cell. `compiled_slots_and_named_access_share_one_cell` stays green.
5. **Per-cell trace bit** (`d6cd19f7`) — derived from the trace set and cached
   on the cell against a variable-trace epoch bumped in the existing
   `invalidate_guard_domain(VariableTrace)` chokepoint.

### New ABI (descriptor in `tcl-runtime-api`, export in `runtime/rust`, test through the C ABI)

```text
tcl_codegen_activation_enter()                          -> i32   ActivationEnter
tcl_codegen_activation_leave(code: i32)                          ActivationLeave
tcl_value_new_double(value: f64)                        -> i32   ValueNewDouble
tcl_value_new_bool(value: i32)                          -> i32   ValueNewBool
tcl_value_get_wide_int(obj: i32, out: i32)              -> i32   ValueGetWideInt
tcl_value_get_double(obj: i32, out: i32)                -> i32   ValueGetDouble
tcl_value_get_bool(obj: i32, out: i32)                  -> i32   ValueGetBool
tcl_codegen_slot_bind(slot, name_ptr, name_len, value)  -> i32   SlotBind
tcl_codegen_slot_set(slot: i32, value: i32)             -> i32   SlotSet
tcl_codegen_slot_get(slot: i32)                         -> i32   SlotGet
tcl_codegen_slot_incr_i64(slot: i32, delta: i64, out)   -> i32   SlotIncrI64
tcl_codegen_slot_append(slot: i32, value: i32)          -> i32   SlotAppend
tcl_codegen_slot_lappend(slot: i32, value: i32)         -> i32   SlotLappend
tcl_codegen_var_traced(name_ptr: i32, name_len: i32)    -> i32   VarTraced
tcl_codegen_slot_traced(slot: i32)                      -> i32   SlotTraced
```

`CodegenAbiValueType` gains `F64` for `tcl_value_new_double`; the one-line arm
this needs in `codegen/wasm/backend.rs`'s `abi_value_type` is the lane's only
edit outside its own files. Status constants: `TCL_VALUE_GET_OK` = 0,
`TCL_VALUE_GET_ERROR` = 1 (the interpreter carries the Tcl error, the out
storage is untouched). The `tcl_codegen_local_bind/set/get` spellings keep
working and address the same indexed cell.

### Decisions

- **Write-back caches on a plain string or any unshared object.** C converts in
  place regardless of refcount because the string rep is kept; that reasoning
  holds for those two shapes. A *shared* object already carrying a list/dict
  rep is left alone rather than having that rep freed under its other holders.
- **The ABI contributes addressing, never semantics.** `slot_incr_i64`,
  `slot_append` and `slot_lappend` run the runtime's own `incr`/`append`/
  `lappend` over a prebuilt argv, so bignum promotion, copy-on-write growth,
  `const`, write traces and every error message are interpreted Tcl's.
- **The trace bit is derived and epoch-validated, not hand-maintained.** Seven
  sites mutate the trace list; a bit set and cleared at each could drift, and a
  wrong "untraced" is a silently missed trace.
- **The expression cache only shimmers a plain string** that already carries its
  spelling, and a reader takes an owning `Rc` before evaluating (a `[cmd]`
  substitution can shimmer the very object the AST hangs on).
- `eval_expr_obj` keeps its `&[u8]` signature: its only caller is `lseq`'s
  shared-core `decode` callback, which hands over bytes, so there is no object
  to cache on.

### Notes and remaining

- **Deliverable 2's diff is inside commit `4a0b9d58`**, not a `wip(p1-…)`
  commit: it was staged in the index when that commit was made during a
  session interruption. The code is intact and green; the history just
  mislabels it.
- Two stale test expectations (`parse`/`subst` nested array index) were red at
  the branch head and are fixed in `4af373e7` — the scanner already matched
  `tclsh9.0`; the expectations predated #1741.
- **Found, not fixed (out of lane):** `fire_var_trace` resolves its identity
  from the access spelling, so a write through an `upvar` alias does not fire
  the target's trace — `tclsh9.0` counts 2 firings for the
  `upvar 0 loc alias; trace add variable loc write …` probe, the runtime 1.
  `vars::trace_home` (added here) is the resolution a fix would use.
- Not attempted, deferred to a later P1 slice: the small-int cache and
  `tcl_codegen_literal_table` (§4.1), the intrinsic table (§4.2),
  `tcl_codegen_proc_define_native` and the shared function table (§4.4), and
  `CmdArena`-backed command handles (§4.4).

## p2-executable-ir

**Goal.** Make the executable semantic IR total for structured control flow so
P3's native lowering has real edges to work with, without changing any
consumer's observable output where it already had a precise answer.

**Owner files.** `rust/tcl-compiler/src/executable_ir.rs` and its consumers
(`semantic_analysis.rs`, `world_state_ssa.rs`, `mixed_region_plan.rs`,
`dispatch_proof.rs`, `codegen/wasm/{semantic_plan,pipeline}.rs`),
`rust/bpf-tcl-ir/src/semantic_bridge.rs`, `rust/tcl-explorer/src/serialise.rs`,
`rust/tcl-registry/src/completion.rs`.

### Design decisions

- **The builder is block-oriented, not stage-oriented.** `FunctionBuilder`
  allocates empty blocks in ID order (which is the deterministic vector
  position validation requires) and `dispatch()` terminates a block on one
  completion, returning the block where normal completion continues. Every
  instruction is still the last in its own block, which is what
  `require_normal_availability` needs to prove a value available.
- **`CompletionSwitch` *is* `DispatchCompletion`.** The plan named a
  `DispatchCompletion { ok, break_target, continue_target, unwind }`
  terminator; the existing `CompletionSwitch` expresses exactly that and every
  consumer already understands it, so a `ControlContext` decides which arms a
  switch gets instead. `Break`/`Continue` arms exist only inside a loop body;
  the default joins a `catch`/`try` handler or leaves the function. "Any non-OK
  code unwinds" is a graph fact rather than a backend convention.
- **`break`/`continue` are never recognised by name.** They arrive as ordinary
  invocations whose completion code the graph routes, so nothing in this lane
  matches a command spelling.
- **`CompleteStructuredRegion` sits at the region's *exit*, not its entry.** A
  region's completion becomes available where its interior edges join, so that
  is where the instruction that defines it belongs. It doubles as the
  completion-φ for the region and as its stable identity for
  `RegionPlan::Structured`, provenance, and Explorer serialisation.
- **`JoinCompletion` is the completion-φ for a handler.** `catch` and `try`
  join many abrupt edges carrying different completion IDs; without it a
  handler could not name the triple it received.
- **Handler and arm grammar is never re-parsed here.** `Statement::Try`
  already carries the registry-owned `kind`/`match_arg`/`trap_pattern`
  decomposition and `Statement::Switch` its `arms`/`mode`/`nocase`/
  `patterns_braced`; the projection consumes those. The one new registry
  helper is `tcl_registry::completion::completion_code_selector`, which owns
  the `ok`/`error`/`return`/`break`/`continue`/integer selector spelling.
- **`CellReference`, not `Place`.** Binding a name to a `crate::place::Place`
  needs the scope declarations a `ResolveContext` carries, and the executable
  builder is handed a `Script` and a registry context only. `CellReference`
  is the exact retained name split by the shared `tcl_syntax` array rule, with
  the base name as the world subject so an element write is seen by a
  whole-array read. A consumer that owns a scope context binds it to a `Place`
  itself.
- **A footprint must be recomputable from the statement.** `validate()`
  recomputes `LoweredFootprint` and rejects a retained one that disagrees, so
  no consumer can be handed a footprint the IR does not prove. That rules out
  using the registry variable scanner (which needs a `CommandRegistry`) inside
  the footprint, so a substituted operand held as exact text reports unbounded
  reads rather than a guessed list; a parsed `ExprNode` still yields its exact
  read set through `ExprNode::vars()`.

### What became precise

| Construct | Projection |
|---|---|
| `if`/`elseif`/`else` | `EvaluateExpr` condition per clause, `Branch` per decision, all arms join at the region completion |
| `while` | header with `EvaluateExpr`, `Branch` to body or exit, explicit back edge; `break` → exit, `continue` → header |
| `for` | `init` before the header, `continue` → the `next` script, which flows to the header |
| `foreach`/`lmap` | `EvaluateExpr` per iterator list, then an `IterateLists` header declaring each group's per-iteration loop-variable cell writes and producing the boolean the `Branch` tests |
| `catch` | body emitted with every abrupt code joining one handler; handler is `JoinCompletion` + `WriteCompletionCell` for the result/options cells, then continues at the region join with code 0 |
| `try` | `JoinCompletion` then a completion-class `CompletionSwitch` to handler entries; a literal `trap` selector adds an `EvaluateExpr { TrapPrefix }` `-errorcode` test; `finally` runs on the normal, handled, and unhandled edges alike |
| `switch` | `EvaluateExpr` subject, one `MatchPattern` per arm honouring `-exact`/`-glob`/`-regexp`/`--` and `patterns_braced`, real body blocks, `-` fallthrough arms branching to one shared body |
| `set`/`incr`/`expr`/`return` | exact `LoweredFootprint` — written cells, read cells, unbounded-read and runs-commands flags, completion set (`{Ok}` for a constant assignment) — projected by world SSA into `VariableStore`-scoped intents |

### What stayed opaque, and why

- `Block` and `UpFrame` (inlined `eval`/`uplevel` bodies): splicing them is only
  sound when the body is straight-line and frame-shift-free, which is a
  separate analysis, so they keep their world barrier.
- `dict for`/`dict map` and Tcl 9's `array for`: their cursors iterate a
  dictionary or an array, not the Tcl list `IterateLists` models.
- Any structured statement the source-faithful lowering could not decompose
  (a dynamic handler body, a malformed clause) already arrives as a `Barrier`
  or keeps its opaque region.
- Trace callbacks and fallback invocation remain non-edges: a cell write
  records a *use* of the variable-trace domain and the contents/absence
  lattice decides whether a callback runs there, exactly as it already does
  for a variable read.

### `return -code`

`return -code error oops` never reaches this IR as a `Statement::Return`: the
`return` lowering hook emits a `Barrier` for any option-bearing form, so it
arrives as a generic invocation. Its exact completion set is already the
registry's (`ExactReturnCompletion` on the invocation's `CompletionDescriptor`),
and the graph dispatches whatever that set contains — a `return -code break`
inside a loop therefore reaches the loop's break target through the ordinary
completion switch, with no command-name recognition anywhere. A plain retained
`Statement::Return` carries no `-code`/`-level` field to project; its footprint
records the completion set `{Return}`, or `{Error, Return}` when its operand can
fail.

### Known follow-ups

- **Dispatch narrowing.** A completion switch currently names `Ok`, the loop
  control codes where a loop encloses the site, and a default. The registry's
  `CompletionDescriptor` on a resolved invocation already bounds the codes it
  can produce, so an edge for a code the callee cannot produce could be pruned.
  That is P7's `Dispatch` narrowing, deliberately not done here.
- **`try` handler fallthrough duplicates its body.** A `-` handler shares the
  next clause's body; the shared body is emitted once per sharing clause under
  a distinct node path, so one source script maps to more than one executable
  node. That is honest (the code really does run on each path) but it means a
  node path is no longer a bijection with a source statement inside a `try`.
- **`lmap`'s accumulated result is not a value in this IR.** A region's
  completion is a first-class triple, but no consumer reads a region's *result*
  today, so the per-iteration accumulation `lmap` performs is described only by
  the region's `LoweringHookId::Lmap` identity. The cursor, the back edge, the
  loop-variable writes, and the completion routing are all exact; the
  accumulation is not modelled.
- **`Block`/`UpFrame` straight-line splice.** Deliberately not attempted: it
  needs a frame-shift and scope analysis this lane does not own.
