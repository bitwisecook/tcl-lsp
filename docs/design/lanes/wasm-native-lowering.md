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

## r4-parser-gaps

Status: **done** — all three assigned issues fixed, oracle-pinned, one
commit each. Owner files: `runtime/rust/src/{parse,cmd_list,cmd_error,
cmd_regex,cmd_scan,cmd_binary,cmd_control,cmd_var}.rs`,
`runtime/rust/tests/parser_gaps.rs`.

### Done

1. **#1576** (`34005140`) — an unterminated `{` word tokenized best-effort
   instead of raising `missing close-brace`. `parse.rs::build_word` checks
   every brace-delimited (`Str`) token against `tcl_lexer::word_closer_offset`
   before treating its content as a literal, in both the single-token fast
   path and the rarer multi-token case (a brace fragment glued onto a prior
   word, e.g. `{a}{b` with no close — necessarily the *last* token, since
   nothing follows end of input). An unterminated one carries the failure as
   `WordPart::ParseError("missing close-brace")`, the evaluator's existing
   raise-on-reach convention for `${…}` (see #1586 below).
2. **#1586** (folded into `5a74eb43` by a same-worktree staging race — see
   *Note* below) — `build_word`'s `${name}` arm trusted the lexer's lenient
   unterminated-`${` token boundary (a name read to end-of-input) as the
   actual variable name. It now calls `tcl_lexer::braced_var_name_end`
   directly and, on `BracedVarEnd::Unterminated`, emits
   `WordPart::ParseError(MISSING_CLOSE_BRACE_FOR_VAR)` instead of ever
   reaching `parse_var_ref` on lenient content — fixing both the script-word
   case (`set x "${a{"`) and the nested case (`${a{` inside a command
   substitution reached through `subst`). `parse_var_ref`/
   `array_index_parse_error` dropped their now-always-false `braced`
   parameter (the braced form never reaches them).
3. **#1577** (`9a0d0ab9`) — `lassign`, `catch` (result/options vars), `regexp`
   match vars, `scan`, `binary scan`, and `foreach`/`lmap` loop vars each
   called `interp.var_set(name, …)` on the raw argument bytes, so `arr(a)`
   became a literal scalar named `arr(a)` instead of element `a` of array
   `arr`. All six sites (`cmd_list.rs::lassign`, `cmd_error.rs::catch_cmd` +
   `bind_handler_vars` — `try`'s handler clause shares the identical bug and
   now a shared `set_var_or_elem` helper, `cmd_regex.rs`'s `regexp` match-var
   loop, `cmd_scan.rs::scan_cmd`, `cmd_binary.rs::binary_scan`,
   `cmd_control.rs::each_loop`) now split the name with the same
   `crate::frame::split_array_ref` + `var_set`/`var_set_elem` routing `set`/
   `lset`/`ledit` already use, and surface the real `VarError` through
   `crate::builtins::var_error` on failure instead of a fixed "variable is
   array" guess. The zero-length-array-name spelling `(k)` comes along for
   free since `split_array_ref` already resolves it (#1458 tracks that
   owner's own correctness, separately).

### Decisions

- **No second name parser anywhere.** Every fix routes through an owner that
  already existed for a sibling command (`word_closer_offset` and
  `braced_var_name_end` from `tcl-lexer`; `split_array_ref` +
  `var_set`/`var_set_elem` from `frame.rs`/`interp.rs`) rather than
  reimplementing brace-balance or array-ref parsing locally.
- **#1576/#1586 share one representation.** Both are "the eval-facing parser
  must fail closed where the shared lexer stays lenient", and both fail the
  same way: a `WordPart::ParseError` the evaluator already knows to raise
  left-to-right (documented on `WordPart::ParseError` itself, extended from
  #1586's pre-existing `${…}` case to the general brace case).
- **Oracle-pinned, not just "doesn't crash".** Every test in
  `runtime/rust/tests/parser_gaps.rs` asserts the exact `tclsh` message text
  (and `-errorcode` where the issue gave it), derived by running the sheet
  through `tclsh9.0`/`tclsh8.6` directly — several draft sheets initially
  mis-stated the oracle (unbalanced literal braces in a test's own Tcl
  source is a live trap for exactly this bug class) and were caught by
  re-running them, not by trusting the first draft.
- **Adjacent, unlisted sites were left alone.** `regsub`'s target variable
  has the identical `arr(a)`-as-literal-scalar bug (verified against
  `tclsh9.0`) but was not in #1577's explicit site list, so it was not
  touched here — flagged as a follow-up instead of silently expanding scope.

### Note: a commit landed under another lane's name

`5a74eb43 wip(r5-trace-semantics): …` carries this lane's #1586 diff
(`parse.rs` + `parser_gaps.rs`) alongside r5's own trace-semantics work: both
lanes' changes were staged in the shared worktree's single git index at the
same moment, and r5's commit swept up everything staged, not only its own
paths — the AGENTS.md "stage only your own files, by explicit path" rule
was followed *here* (`git add` was scoped to this lane's two files
immediately before committing) but the race is symmetric — another lane's
broader `git add`/`git commit` can still sweep up whatever this lane has
staged in the same window. The code is correct, tested, and present at HEAD
either way; only the commit attribution is wrong. No history rewrite was
attempted (out of policy for a shared branch). Every commit after this one
used `git commit -m … -- <explicit paths>` with nothing pre-staged, to
close the window as tightly as possible — but the underlying race is
cross-lane and cannot be fully closed from one lane's side alone.

### Remaining / not done

- **`regsub`'s target variable** (`cmd_regex.rs::regsub_cmd`) has the same
  #1577-shaped bug (`regsub {a} xax b arr(k)` writes a literal scalar, not
  element `k`) but was outside the issue's explicit site list — flagged as a
  follow-up task rather than fixed inline.
- The `quote`/`bracket` siblings of #1576 (`missing "` for an unterminated
  `"..."` word, `missing close-bracket` for an unterminated `[...]`) are
  **also** currently lenient in the eval path — verified against `tclsh9.0`
  during investigation, e.g. `eval {list a "b}` returns `a b` instead of
  raising `missing "`. #1576 named only the brace case; the quote/bracket
  gap is real but tracked separately (no filed issue found for it at the
  time of writing — worth filing one).

### Verification

`make runtime-rust-test` (589 lib + 16 `parser_gaps` + suite tests, 0
failed) and `make runtime-rust-lint` (`cargo fmt --check` + `cargo clippy
--locked --all-targets -- -D warnings`) both green before every commit in
this lane, on top of a tree containing other lanes' concurrent in-flight
edits outside this lane's files.

## r6a-rename-interp

Status: **done** — all seven items of #1412 verified against tclsh9.0.4
(8.6.16 cross-checked where the two releases could plausibly differ); four
needed a fix, three (item 4, the two halves of item 6, and all three stale
doc comments the issue named) were already correct at this lane's start.
Owner files: `runtime/rust/src/namespace.rs`, `runtime/rust/src/cmd_alias.rs`,
plus small additive `runtime/rust/src/interp.rs` hunks (see *Note on
attribution* below), and the new
`runtime/rust/tests/rename_interp_semantics.rs`.

### Done

1. **Item 1 — `rename` onto an occupied destination.**
   `Namespaces::rename` (`namespace.rs`) now checks the destination before
   touching the source, matching C's `TclRenameCommand` (`tclBasic.c`), which
   checks `newNsPtr->cmdTable` before removing `old`'s hash entry. Refuses
   with `can't rename to "X": command already exists`
   (`RenameOutcome::TargetExists`), leaving both commands intact. This also
   makes a same-slot self-rename (`rename foo foo`) refuse — tclsh 9.0.4
   does too, since the source is still occupying the slot when the check
   runs; the old doc comment claiming self-rename was a harmless no-op was
   wrong and is gone. The occupancy check is gate-hidden-root-aware
   (`Interp::is_gate_hidden_object_root`) via `Namespaces::
   destination_occupant_fqn`, so renaming onto an engine-installed TclOO
   root this release doesn't carry (`::oo::configurable` on an 8.6 surface)
   still succeeds, per that mechanism's existing "must read as free"
   contract — two pre-existing `cmd_namespace.rs` tests exercise exactly
   this and would have failed otherwise.
2. **Item 2 — `rename` across namespaces re-homes a proc.**
   `Namespaces::rehome_proc` rewrites a moved `Command::Proc`'s `ns`/`fqn`
   to the destination when they differ, so `namespace current` inside the
   body reports the new namespace, mirroring C's `cmdPtr->nsPtr`
   reassignment. A fresh `Rc<ProcDef>` is built rather than mutating through
   the shared one, so any frame still on the call stack from before the
   rename keeps its old snapshot.
3. **Item 3 — `interp`'s bad-option list advertised undispatchable
   subcommands.** `target` is cheap given this runtime's two supported alias
   shapes (same-interp `Command::Alias`, child-to-immediate-parent
   `Command::ParentAlias`) — `Interp::alias_target_path` computes the
   interp-path directly from the shape rather than a general interp-tree
   walk, and `interp target path alias` now dispatches. `cancel` (script
   cancellation) and `share`/`transfer` (cross-interp channel sharing) need
   infrastructure this runtime has none of, so they were dropped from the
   advertised list rather than left advertised-but-undispatchable — this
   runtime's list now names only what it dispatches, diverging from tclsh's
   on purpose for those three names.
4. **Item 5 — `invokehidden`'s `-global`/`-namespace` were parsed and
   discarded.** They now set the current-namespace context for the hidden
   call (`Interp::set_current_ns`, saved/restored around it — no frame push
   needed for one command rather than a script body), with `-namespace`'s
   name resolved from the **global** namespace regardless of the caller's
   current one (`Interp::ensure_global_namespace`, C's `TCL_GLOBAL_ONLY`;
   tclsh-pinned: `-namespace bar` from inside `::foo` still names `::bar`).
   An unrecognized option is now a hard `bad option "-x": must be -global,
   -namespace, or --` error instead of a silent skip. **Correction to the
   issue's own item 5**: the claimed `cannot use -global option and
   -namespace option together` error does not exist on either tclsh 8.6.16
   or 9.0.4 — C's `ChildInvokeHidden` just takes the last of the two given;
   no mutual-exclusion refusal was added. Simplification: unlike C's
   `Tcl_GetIndexFromObj`, this does not accept an abbreviated option name
   (`-g` for `-global`).
5. **Item 7 — inconsistent `bad option` shape between `interp` and
   `$child`.** `Interp::dispatch_child`'s fallthrough said `interp
   subcommand "X" is not supported in this runtime` — not a tclsh shape at
   all. It now reports the same `bad option "X": must be ...` shape the
   `interp` ensemble uses, with the child command object's own (shorter)
   list — `NRChildCmd` (tclInterp.c) never dispatches
   `children`/`create`/`delete`/`exists` there, those are only ever spelled
   `interp <op> path` from the parent.
6. **Items 4 and 6 (already correct).** `rename name ""`'s missing-source
   error already said `can't delete` (item 4); `interp hide`/`expose`
   misses already raised and `expose` already refused an occupied
   destination (item 6). No code change; both got regression tests anyway.
7. **Three stale doc comments (already fixed before this lane started).**
   `cmd_alias.rs:28`/`:75`'s "single-interp scope only" / "other subcommands
   … trap here" claims and `interp.rs`'s `Command` enum doc's "`Builtin` and
   `Alias` … the next variants" are all gone from the current text —
   verified by grep, no diff needed.

### Note on attribution (shared-worktree staging race)

Two of this lane's fixes — item 1/2's `namespace.rs`/`interp.rs` edits, and
the `Interp::alias_target_path` addition item 3's commit message cites —
physically landed inside two `wip(r5-trace-semantics):` commits (`5a74eb43`,
`605989af`) instead of a `wip(r6a-rename-interp):` one. This lane staged its
own files with explicit paths and used `git add -p` to pick only its own
hunks out of `interp.rs` (which had, and kept having, other lanes' unrelated
in-flight edits sitting in the same working copy throughout), but between
staging and running `git commit`, another lane's broader `git add`/`git
commit` in the same shared index swept up what this lane had already staged
before this lane's own `git commit` ran — which then found nothing left to
commit and aborted, invisibly, with output that reads like an ordinary
post-commit status listing rather than a failure (worth watching for: `git
commit`'s "no changes added to commit" line, easy to miss when skimming for
just the "files changed" summary). r4-parser-gaps hit the same race from the
other side (see its *Note* above) and reached the same conclusion: the code
is correct and tested either way (see `rename_interp_semantics.rs`), only
the commit attribution is wrong, and rewriting shared branch history to fix
attribution is out of policy. Every commit after the first used `git add
<explicit paths>` immediately followed by `git commit` with no gap for
another lane's commit to intervene, and a `git diff --cached` sanity check
immediately before each one — closing the window as tightly as one lane can
from its own side, though the race is inherently cross-lane.

### Verification

`make runtime-rust-test` and `make runtime-rust-lint` both green before
every commit, re-run immediately before each one to absorb other lanes'
concurrent in-flight breakage in files outside this lane's ownership
(`interp.rs`/`cmd_error.rs`/`parse.rs`/etc. were each observed transiently
broken mid-edit by other lanes during this lane's work; every one resolved
itself within a minute without action from this lane). `runtime/rust/tests/
rename_interp_semantics.rs` pins all seven items against real `tclsh9.0.4`
transcripts quoted in each test's doc comment.

## r5-trace-semantics

Status: **all five buckets landed.** §8's R5 row — #1633's runtime rows, #1574,
#1575's two runtime gaps, #1569 — plus the `upvar`-alias defect the
`p1-runtime-abi` lane found and left behind `vars::trace_home` for.

Owner files: `runtime/rust/src/{cmd_trace,interp,frame,vars,cmd_array}.rs` and
the new `runtime/rust/tests/trace_semantics.rs`.

### Delivered

Eight commits, each with `make runtime-rust-test` and `make runtime-rust-lint`
green immediately before it.

1. **`528a87c5` — traces fire on the resolved cell.** `fire_var_trace` derived
   its identity from the *access spelling*: `local_trace_level` refused a frame
   level whenever the accessed name was a link, and `home_namespace_and_base`
   discarded the level for a frame home, so an `upvar` alias and its target had
   different identities and a write through the alias matched nothing.
   Registration, firing, `trace info variable`, `trace remove variable` and
   `trace vinfo` now share one `Interp::trace_identity` over `vars::trace_home`.
   Separately, the callback's `name1` is the caller's spelling, not the resolved
   base — C passes `part1` through untouched.
2. **`5a74eb43` — `incr` fires the read trace** (#1633 row 3), through
   `Interp::read_for_update`, which is `lappend_read` generalised: both commands
   are C callers that treat a `NULL` fetch as "no current value", so an erroring
   read trace leaves `incr` counting from 0 rather than failing.
3. **`605989af` — a trace that aborts an access keeps its errorInfo chain**
   (#1633 row 2), with the `(<type> trace on "…")` frame and the
   `TCL WRITE|READ VARNAME` `-errorcode`.
4. **`c9c393e6` — an array-element alias carries its element** (#1633 rows 6-7).
   `vars::trace_home` returns a `TraceHome` with the element the resolution
   ended on, and `Interp::trace_access` is the one place that decides what a
   given access matches, reports, and whether the array's traces take part.
5. **`33e00cbb` — what a callback changes mid-firing is honoured** (#1633 rows
   8-9): a trace removed during a firing no longer runs in that pass, and a
   command-delete trace that re-creates its command leaves the new command
   standing.
6. **`5c3b582d` — re-entrancy suppression per `Var` cell** (#1574), with the
   array's own cell keeping the separate gate C gives `arrayPtr`.
7. **`eb650422` — the two missing unset-trace sites** (#1575 rows 1 and 3):
   proc-frame teardown, and per-element firing on a whole-array `unset`.
8. **`0d3b125a` — `array` traces dispatched** (#1569), from one `LocateArray`
   equivalent at the top of the `array` command.

### Decisions

- **One resolution, five consumers.** Registration, firing, `trace info`,
  `trace remove` and the per-cell trace bit all go through
  `Interp::trace_identity`. The bug class this closes is a trace that registers
  against one identity and fires against another; keeping the two on separate
  code paths is what let #1328 and #1633's `upvar` row exist at all.
- **Transcripts, not counts.** Every test in `trace_semantics.rs` asserts the
  whole firing transcript — argument lists and order — and quotes its sheet
  verbatim so it can be pasted into a real `tclsh`. A count-only test passes on
  both sides of almost every bug in this bucket.
- **Both releases where they differ, both releases where they agree.** The
  release-split rows (`upvar #0 a(k) e`, `unset a(k)`) are pinned at 9.0 *and*
  8.6; so is the half of the element-alias rule that is release-independent
  (registration lands on the element), so a future change cannot quietly make
  the invariant part release-dependent too.
- **The new release axis is derived in `interp.rs`, and says so.**
  `traces_recover_the_linked_element` is `>= V9_0`, from the two 9.0-only C
  blocks it names (`tclTrace.c`:2560-2565 and `tclVar.c`:2634-2640, absent in
  8.4/8.5/8.6). It belongs in `tcl-dialect` beside
  `namespace_var_global_fallback`; that crate is outside this lane, and the
  method carries a comment saying where it should move.
- **No command-name branches.** The `array`-trace hook is
  `Interp::fire_array_trace` in the variable owner; `cmd_array.rs` calls it once
  before dispatch, and the only per-subcommand fact it needs — where the array
  name sits — is a table it shares with the unknown-subcommand message, so a new
  subcommand cannot be added without becoming trace-visible.
- **`VarTrace` gained an id, not a snapshot.** C identifies a trace by pointer
  and rewrites `nextTracePtr` when one is removed mid-walk. Ids reproduce that
  with a fixed walk order and a liveness re-read per step; they are never
  reused, so a trace removed and re-added during one firing is a different
  trace, as it is a different allocation in C.
- **The per-cell trace bit stays correct.** `var_is_traced` answers from the
  same resolved identity and remains conservative in the safe direction (an
  element alias reports its array as traced); the only trace-list mutations
  added are the existing ones, so the `VariableTrace` epoch chokepoint is
  unchanged.

### Notes and remaining

- **One `#1633` row is left, and is not this lane's file.** `set x orig` whose
  write trace mutates or unsets `x` must return the value read back *after* the
  traces (`tclVar.c` 9.0.4:2050-2065 — `TclPtrSetVarIdx` re-reads `varPtr` and
  yields an empty object if a trace changed the variable "in some gross way").
  The runtime still returns `orig`. The fix belongs in the `set` path in
  `cmd_var.rs`, which another lane owns this cycle. The other three rows the
  issue lists as runtime/rust divergences (`lappend` read traces,
  `trace info command` on a nonexistent command, an unset trace reviving its
  variable) were already correct and are re-verified here.
- **#1575's row 2 is tcl-vm's** (namespace teardown), not runtime/rust's.
- **One edit outside this lane's files.** `cmd_var.rs`'s
  `traces_fire_on_the_resolved_variable_not_the_spelling` asserted
  `a2:write` for a `set ::a2 X` access where tclsh 8.6.16 and 9.0.4 both print
  `::a2:write`. The expectation was wrong; commit 1 corrects that one line and
  the new test module carries the sheet.
- **`samples/wasm/t7-dynamic/70_var_traces.tcl` now matches its golden**
  byte for byte through `examples/run_script`. `p0-harness` retired the
  corresponding `Plan::Default` ledger row in `f9830ca5` while this lane was
  running, so there is no stale entry left; the remaining `Plan::Analysis` row
  for that sample is #1772, a codegen defect P3 owns.
- **Not attempted:** `array` subcommand prefix matching. `array nam arr` works
  in both tclsh releases and errors here, which is why the `array`-trace hook
  can match subcommand names exactly. That is a pre-existing gap in the `array`
  ensemble, unrelated to traces, and belongs with whoever owns the subcommand
  table.

### Verification

`make runtime-rust-test` (587 lib + 62 integration) and `make runtime-rust-lint`
green before every commit. Every expectation in `trace_semantics.rs` was
produced by running its own sheet through `tclsh9.0` (9.0.4), and through
`tclsh8.6` (8.6.16) for the release-split rows and for the rows asserted to be
release-independent. Other lanes' in-flight edits to `bignum.rs`, `expr.rs`,
`cmd_error.rs`, `cmd_alias.rs`, `namespace.rs`, `parse.rs` and `tcl-registry`
were each observed transiently non-compiling or red during this lane's work; the
gates were re-run until those cleared, and no commit here includes another
lane's file.

## r10-word-parts

Bucket R10 of `docs/design/compiler/wasm-native-lowering-plan.md` §8: give
"split a Tcl word into its substitution components" one owner. The walk had
four implementations — `runtime/rust/src/parse.rs`'s `scan_parts`, that
crate's `subst.rs` mirror, `tcl-vm`'s `subst.rs`, and the compiler's
`segmenter.rs` / `ir.rs` `WordExpr` builder — and they had drifted apart in
ways users could see.

### Delivered

1. **`rust/tcl-lexer/src/word_parts.rs`** — the owner. `decompose` splits a
   word's content (or a `subst` template) into text runs with their backslash
   escapes folded in, `$name` / `${name}` / `$arr(index)` references and
   `[script]` substitutions, parameterised by `LexerConfig`. `scan_var_ref`,
   `command_subst_close` and `quoted_word_close` are the piecewise entry
   points, and the six parse-error messages C names live here as constants.
   It sits in `tcl-lexer` because `braced_var_name_end`, `scan_array_index`,
   `command_substitution_end` and `close_quote_offset` already do, and because
   the crate is below `tcl-syntax`, both runtimes and the compiler.

   Contractual: the model is borrow-based (the literal fast path is a
   sub-slice, which is what `parse_cache` / MM-B.6 rests on), parse failures
   travel as `WordPart::ParseError` parts so C's incremental `subst` order is
   preserved, and an unterminated `[` reports the error found *inside* the
   bracket because C recurses into `Tcl_ParseCommand` there.

2. **`runtime/rust` consumes it.** `scan_parts` is a three-boolean adapter;
   `scan_var_name`, `skip_command_subst`, `parse_var_ref` and
   `array_index_parse_error` are deleted; `subst.rs`'s `scan` calls the same
   owner. `parse.rs` now owns only the script level — commands, words, and
   the word-delimiter rules.

3. **`tcl-vm` consumes it** for `subst_command`, the resumable
   `subst_scan_step`, and `subst_word`'s general scan.

4. **Owner map**: a `word substitution components` row in
   `docs/design/contracts/shared-utility-contracts-rust.md`'s machine-checked
   manifest (so a fourth copy fails `cargo xtask owner-resolution`), the
   matching prose under the `tcl-lexer` heading, and the summary row in
   `AGENTS.md`.

### What this closed

The two lenient cases r4-parser-gaps left, plus a third of the same shape.
All are lexer recoveries the eval-facing parser was trusting — the lexer is
shared with the LSP and must keep tokenizing half-typed source, so each
delimited word now re-derives its close from the owner, as the braced word
already did for #1576. Oracle-pinned in `runtime/rust/tests/parser_gaps.rs`:

| source | was | now (= `tclsh` 8.6.16 / 9.0.4) |
|---|---|---|
| `eval {list a "b}` | the two-word list `a b` | `missing "` |
| `eval {list a [b}` | `invalid command name "b"` | `missing close-bracket` |
| `eval {list a $x(}` | `can't read "x("` | `missing )` |

And two the owner brought with it, in both engines: the `]` search is now
brace-, quote- **and** comment-aware (`subst {[list "a]b"]}` no longer stops
at the quoted bracket), and an unterminated `[` reports the error inside it
(`subst $t` with `t` = `[set y ${a{b]` is `missing close-brace for variable
name`).

### Follow-ups, both deliberate

- **The compiler's segmenter is the remaining adopter.** `segmenter.rs` /
  `ir.rs::CommandTokens::from_segmented` still build `WordExpr` / `WordPart`
  themselves. The owner's API is shaped so the adoption needs no change to
  `WordExpr`'s public shape — `decompose` takes the word's content span plus
  a `LexerConfig` and returns parts whose byte extents are recoverable from
  the borrows, so `from_segmented` maps part for part, and the segmenter keeps
  owning command and word boundaries. Not done here: `rust/tcl-compiler/` is
  the native-lowering lane's this cycle, and doing it blind against a moving
  file is how the four copies happened in the first place.

- **Issue #1646 is not a decomposition bug.** `string length "x\$y"` is 3 on
  both oracles; the VM answers 3 at the top level and 4 one bracket deep, and
  `list "x\$y"` gives `{x\$y}` where the oracles give `{x$y}`. The same word
  decomposing correctly at one nesting level and wrongly at the next is not a
  decomposition difference: `tcl explore --show asm` shows the compiler
  pushing the literal `x\$y` for a word whose value is `x$y`. A blanket decode
  in the VM makes those vectors pass only by breaking
  `set body "list e\\n} f\\$} "`, whose emitted literal legitimately does
  contain backslashes (15 characters on both oracles, 13 after a second
  decode). The VM's rule — an emitted `PUSH` literal is already its value — is
  the right one; the emission is not, and the fix belongs in
  `rust/tcl-compiler`'s literal emission for a word nested in a bracket word.
  `rust/tcl-vm/tests/word_parts_owner_e2e.rs` records the measurement, and
  fails loudly with instructions if either vector starts answering the oracle
  value.

- **Word-parse ordering in the runtime's eval loop.** C parses every word of a
  command before evaluating any, so `eval {list [side] [b}` raises without
  running `side`; this runtime substitutes word by word and runs it first. The
  message and completion code are right, only the ordering is not. The fix is
  a pre-pass in `interp.rs`'s eval loop, another lane's file this cycle.

### Verification

`make runtime-rust-test` (587 lib + integration) and `make runtime-rust-lint`
green before every commit. `cargo test -p tcl-lexer` and `cargo test -p tcl-vm`
green, and clippy clean for both crates, apart from
`mathfunc_int_wide_are_the_64bit_window` and the `tcl-syntax` warnings, which
come from the concurrent math lane's in-flight edits to
`expr/mathfunc.rs` / `number_tower.rs` and are untouched here.
`cargo xtask owner-resolution` reports 25 rows resolving.

Every expectation was produced by running its own sheet through `tclsh9.0`
(9.0.4) and `tclsh8.6` (8.6.16); the sheets are recorded in the test modules,
so a reader can re-derive them by pasting into a real interpreter.

## r5b-leftovers

Status: **done** — the two follow-ups r5-trace-semantics and r4-parser-gaps
each left behind for another lane. Owner files: `runtime/rust/src/{cmd_var,
cmd_regex}.rs` and `runtime/rust/tests/{trace_semantics,parser_gaps}.rs`.

1. **#1633 row 1 — return-after-trace.** `TclPtrSetVarIdx` (`tclVar.c`
   9.0.4:2050-2065) stores, fires the write traces, and only *then* decides
   what to return: the cell's current value if a trace rewrote it (still a
   defined scalar), or an empty-string object if a trace changed the variable
   "in some gross way" (unset it, or made it an array). `set`/`incr` — both
   in `builtins.rs`, not this lane's file — instead echoed back the value
   they had just stored. That is also a use-after-free once a write trace
   replaces the same cell: the stored object's only reference is dropped out
   from under the handler before it reads `sum`/`value` back (reproduced live
   — `incr z` on a `z` whose write trace does `set ::z mangled` printed the
   empty string, not `1` or `mangled`, i.e. read freed memory). Fixed without
   touching `builtins.rs`: `cmd_var::install` re-registers `set` and (under
   `#[cfg(have_tommath)]`) `incr`, the same override pattern `builtins.rs`
   already documents for TclOO's `variable` — installed later in the same
   `install()` sweep, so it wins, and nothing downstream re-registers either
   name. Both new bodies are structurally the original ones with their store
   tail replaced by `interp.store_var_result` (`interp.rs`, already used by
   `append`/`lappend`/`string insert` for this identical reason — a
   protective reference held across the store, then a trace-free read-back).
   `incr`'s small `incr_constant_error` guard is duplicated locally (it was
   module-private in `builtins.rs`); everything else is reused, not
   reimplemented. Pinned (`tclsh9.0`/`tclsh8.6`, identical on both):
   `proc w {n1 n2 op} {set ::x mangled}; trace add variable x write w; set x
   orig` → `mangled`; the same with a trace that `unset`s its target → `""`;
   both repeated for `incr`.
2. **`regsub`'s target variable — element write.** `regsub -all a $s b
   arr(k)` wrote a literal scalar named `arr(k)` instead of element `k` of
   `arr` — the #1577 shape R4 fixed at its six explicit sites and flagged
   this one as a follow-up rather than touch out-of-scope code. `cmd_regex.rs
   ::regsub_cmd`'s target-variable arm now runs the same `split_array_ref` +
   `var_set`/`var_set_elem` routing `set`, `regexp`'s match-var loop, and
   every #1577 site already share. Pinned: `set s xax; regsub -all a $s b
   arr(k)` → `array get arr` gives `k xbx` on both oracles.

### Decisions

- **The override-registration pattern, not a `builtins.rs` edit.** This
  lane's file list is `cmd_var.rs`/`cmd_regex.rs` plus the two test files —
  `set`/`incr` live in `builtins.rs`, another lane's file this cycle (a
  concurrent worktree, shared with several other in-flight lanes). Rather
  than either touching a file outside the assignment or leaving a confirmed,
  oracle-pinned bug (including the use-after-free above) unfixed,
  `cmd_var::install` re-registers both names with corrected bodies, using the
  last-registration-wins override `builtins.rs` itself already relies on for
  TclOO's `variable`. No other file changed.
- **`store_var_result`, not a hand-rolled read-back.** It already exists
  (`interp.rs`, r4/r5-era) for exactly this shape and is already exercised by
  `append`/`lappend`/`string insert`; reusing it instead of re-deriving the
  read-back logic keeps the "gross change → empty string" rule in one place.
- **`append`/`lappend` needed no change.** Both already route through
  `store_var_result` (`cmd_list.rs`) and were re-verified against the same
  oracle sheets rather than assumed correct.

### Verification

`make runtime-rust-test` (587 lib + 23 `parser_gaps` + 22 `trace_semantics` +
suite tests, 0 failed) and `make runtime-rust-lint` both green before each
commit, on a tree carrying other lanes' concurrent in-flight edits outside
this lane's files (a `runtime/rust/src/expr.rs` formatting diff from another
lane was transiently red mid-session and cleared before commit, per the
r4/r5 lanes' own note on this same trap). Every new expectation was produced
by running its sheet through `tclsh9.0` (9.0.4) and `tclsh8.6` (8.6.16)
directly.
