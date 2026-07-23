# Recursive-descent depth limits (issue #996)

Every stage of the pipeline that walks nested Tcl control-flow bodies
(`if`/`while`/`foreach`/`switch`/`try`/`catch`/`dict for`, `apply` lambdas,
`namespace eval`, …) does it by recursive descent: one Rust stack-frame
group per source-nesting level. That shape is simple and matches the
source structure, but it means the *native* call-stack depth scales
directly with how deeply the input nests — and unlike a Tcl-level
`proc`-call stack (bounded by `interp recursionlimit`, a catchable error),
nothing in Rust stops a native stack overflow from being an uncatchable
process abort (`SIGABRT`).

Issue #996 was exactly this: `Analyser::analyse()` crashed the whole
process on Tcl source nested ~100-150 levels deep — a real DoS for any
consumer that analyses untrusted, generated, or minified Tcl. Chasing the
same root cause turned up sibling instances of the same bug class across
most of the recursive-descent walkers in the workspace — a WASM-hosted Tcl
interpreter crashing on ordinary recursive `proc` calls, an optimiser pass
tree, a formatter, a minifier, an iRules reference walker, and a
second-order recursion (`elseif`-chain length) distinct from body nesting.
This doc records the root cause, the fixes, and which walkers are covered.

## Root cause: two distinct problems, not one

**Problem 1 — the depth cap existed, the stack budget didn't.**
`tcl_compiler::analyser::commands::MAX_BODY_DEPTH` already capped
`analyse_body`'s recursion at 256 levels before this issue — the cap
itself was correct and was checked on every recursive entry. The crash
happened anyway because **256 real Rust stack frames of that recursion
chain need more stack than the thread actually running it provides**, and
that budget is not a fixed, portable quantity:

- The native LSP server runs analysis inside `tokio::spawn`ed tasks, which
  land on Tokio's worker-thread pool. Tokio's default worker-thread stack
  is 2 MiB.
- `cargo test` runs each `#[test]` on its own thread with the same ~2 MiB
  platform default — which is why several of the regression tests below
  spawn their own big-stack thread rather than relying on the harness's.
- The `tcl` CLI's main thread inherits whatever the OS/`ulimit -s` gives
  it — 8 MiB by default on Linux, but far less guaranteed on other
  platforms or in a constrained container.
- Several crates (`tcl_runtime` — the WASM Tcl runtime, `tcl_lsp_core`,
  `tcl_irules`, `tcl_vm`) are also compiled to WASM and consumed by a host
  (`bigip-query-wasm`, `tcl-vm-wasm`) whose stack budget is **entirely
  outside this repo's control** — commonly far smaller than 2 MiB.

Reproduced and measured directly: `ulimit -s 2048` (2 MiB) against the
*unfixed* analyser binary crashes at nesting depth 130-140 — an exact
match for the issue's reported range — while the same input is fine up to
the 256-level cap on an 8 MiB stack. The cap was never the problem; the
ambient stack size the cap's own frames were allowed to run on was.

This is also why raising the magic number alone was rejected as a fix: a
bigger cap just needs proportionally more stack, and the *actual* frame
cost of the recursion chain silently grows every time a hot function in it
gains a local variable — which is plausibly how "256 was safe" drifted
into "256 isn't," with nobody touching the constant.

**Problem 2 — some walkers had no cap at all**, independent of any stack
budget. An uncapped walker is not made safe by a bigger stack in
principle — only in practice, for however deep real input happens to
nest. Several of these turned out to be reachable in ways the first pass
missed: a second, unguarded recursion running *before* an existing guard
(`lowering/mod.rs`); a guard that could be *bypassed* by resetting its own
counter (`param_traits.rs`'s `apply` re-entry, and structurally the same
bug as the pre-existing #997 finding); an `elseif`-chain that recurses
independently of body-nesting depth (`codegen::structured`); and an
interpreter's own `RECURSION_LIMIT` (matching real Tcl's default) that
was itself never actually a safe native-stack backstop, because it only
bounded proc-call nesting, not the *general* eval-recursion nesting that a
tree-walking interpreter also pays a native stack frame for.

## The fix

Three complementary strategies, applied per walker according to what that
walker's actual runtime environment can guarantee:

1. **Guarantee a generous, explicit stack budget at every process entry
   point**, rather than depending on the ambient thread's stack. Used for
   every walker that is *only* ever reachable from a binary this repo
   controls the entry point of:
   - `tcl-lsp-server`/`tcl-mcp`: `main` builds its own
     `tokio::runtime::Builder::new_multi_thread()` with
     `.thread_stack_size(64 MiB)` instead of relying on `#[tokio::main]`'s
     (2 MiB) default.
   - `tcl` CLI / `f5-cli`: `run` spawns every verb's dispatch on a
     dedicated `std::thread::Builder::new().stack_size(64 MiB)` thread,
     decoupling crash behaviour from the OS/`ulimit` default.
   - `tcl-debugger`: `VmBackend::record` (the compile-and-run step behind
     both the CLI's `launch` and the DAP server's `launch` request) runs
     on its own dedicated 64 MiB thread rather than whatever thread
     `launch` was called on.

   64 MiB is deliberately generous — the measured need is a few MiB even
   in an unoptimised debug build — so it also comfortably covers deeper
   nesting than the current 256-level caps allow, future frame-size
   growth, and multiple guarded walkers running on the same call stack
   at once.

2. **A conservative, small depth cap for anything reachable from a WASM
   host** (`tcl_runtime`, `tcl_lsp_core`'s formatter/minifier,
   `tcl_irules`'s walker, `tcl_vm`'s `cmd_control.rs` fallback) — strategy 1
   doesn't apply here because this repo does not control the WASM host's
   stack. Each of these was calibrated empirically against a 2 MiB native
   thread (the same class of ambient budget that made the original crash
   reproducible), then set well under the measured crash floor to leave
   real margin for a meaningfully smaller WASM stack. Values differ
   because the measured per-level native-frame cost differs by walker
   (see the table below for each one's specific number and reasoning).

3. **Every independently-recursive walker still needs its own depth cap
   (or, for `codegen::structured`'s `elseif` chains, its own *chain*
   cap)**, regardless of which stack-budget strategy applies. `lowering.rs`
   (`Lowerer::lower_script`/`lower_body`) had a cap-shaped gap: it runs
   *before* `cfg_builder`'s own guard (`MAX_LOWER_DEPTH`, already
   present), so an unguarded lowering pass crashed first and made the
   downstream guard moot regardless of stack size. `param_traits.rs`'s
   deep pass had a bypass: its `apply` (`ArgRole::LambdaLiteral`) handling
   re-entered the public, depth-0 entry point instead of threading its own
   depth forward, so alternating `if {…} { apply {x {…}} … }` nesting
   reset the logical `MAX_DEPTH` counter on every `apply` boundary while
   the native call stack kept growing regardless. `codegen::structured`'s
   `emit_if` recurses once per `elseif` link via a self-call — a
   *different* recursion shape from nested bodies, unbounded by
   `MAX_LOWER_DEPTH`-style caps entirely, so a pathologically long
   `elseif` chain needed its own guard threaded through the same `depth`
   budget.

4. **The depth cap trips as a diagnostic or an explicit fallback, not
   silent truncation or a miscompile.** Before this fix, `analyse_body`
   hitting `MAX_BODY_DEPTH` silently stopped descending with no signal to
   the user — diagnostics for the excess nesting just never appeared, and
   there was nothing to tell an editor "I gave up here." Real `tclsh`
   doesn't behave this way either: exceeding `interp recursionlimit`
   raises a catchable `"too many nested evaluations (infinite loop?)"`
   error. `analyse_body` now emits
   **[`E207`](../../kcs/codes/kcs-diagnostic-e207-nesting-depth-exceeds-limit.md)**
   once per analysis run when the cap trips, anchored on the body where
   descent stopped. Every other walker's fallback is chosen for
   soundness in its own domain: `lowering` emits a `Statement::Barrier`
   (unknown-effect, not dead code) past the cap; `codegen::structured`
   degrades to the same whole-construct eval-fallback every other
   unstructured statement kind already uses (and, for the `elseif`-chain
   cap specifically, re-runs the *entire* original `if`/`elseif`/`else`
   construct as one eval-fallback — sound because that branch only runs
   when every earlier condition was already false, so re-testing them is
   simply redundant, not wrong); `formatting`/`minify` leave the
   over-deep body unformatted/unminified rather than corrupting it;
   `tcl_irules::walker` stops collecting references past the cap (the
   references found up to that point still stand); `tcl_runtime` and
   `tcl_vm` both raise the same catchable `"too many nested evaluations
   (infinite loop?)"` error real `tclsh` uses for the conceptually
   identical failure (too much nesting) — just caught earlier, for
   native-safety reasons independent of either interpreter's
   user-configurable `interp recursionlimit`.

## Guarded walkers

| Walker | Cap constant / value | Stack strategy | Notes |
|---|---|---|---|
| `tcl_compiler::analyser::commands::analyse_body` | `MAX_BODY_DEPTH` = 256 | Big-stack entry points | Emits `E207` once per run when tripped. |
| `tcl_compiler::cfg_builder::CfgBuilder::lower_script` | `MAX_LOWER_DEPTH` = 256 | Big-stack entry points | Pre-existing; stops descending, truncated-but-valid CFG. |
| `tcl_compiler::lowering::Lowerer::lower_script`/`lower_body` | `MAX_LOWER_NEST_DEPTH` = 256 | Big-stack entry points | Added by this fix. Emits a `Statement::Barrier` past the cap. |
| `tcl_compiler::analyser::param_traits::scan_deep` / `infer_param_traits_deep_at_depth` | `MAX_DEPTH` = 8 | Big-stack entry points | `apply`-reset bypass fixed by this change. |
| `tcl_compiler::optimiser::{propagation, expr_simplify, pattern_recognition, structure_elimination, code_sinking}` | `MAX_OPTIMISER_WALK_DEPTH` = 256 (shared, `optimiser/mod.rs`) | Big-stack entry points | Defence in depth: `lowering`'s cap already bounds the IR these passes see in the normal pipeline (source → lowering → optimiser) to 256 levels before they run; this cap protects any future/test-only caller that builds a `Script` directly. `code_sinking` additionally caps three more mutually-recursive query families (`find_deepest_targets`/`try_deeper_sink`, `script_redefines_sink_read`/`stmt_redefines_sink_read`, `script_uses_var`/`statement_uses_var`) — each answers conservatively (`true` / stop descending) past the cap, biasing toward *not* applying an optimisation rather than risking an unsound one. |
| `tcl_compiler::codegen::structured::{walk_stmt, emit_if, emit_loop}` | `MAX_STRUCTURED_DEPTH` = 256 | Big-stack entry points (when wired up — see below) | Not currently reachable from any wired-up production path (`structured::walk` has no caller yet — WASM-backend infrastructure ahead of its integration), guarded anyway. Covers both nested-body depth *and*, independently, `emit_if`'s own `elseif`-chain self-recursion (a distinct recursion shape `MAX_LOWER_DEPTH`-style caps don't bound at all) — a long chain re-runs the whole original `if`/`elseif`/`else` as one eval-fallback past the cap rather than recursing further. |
| `tcl_lsp_core::references` (`scan_my_method_region`, `scan_obj_method_region`, `scan_next_dispatch_region`) | `MAX_DISPATCH_SCAN_DEPTH` | Big-stack entry points | Audited for this issue: every recursive call site correctly threads `depth + 1`; no bypass found. |
| `tcl_lsp_core::folding` | `MAX_FOLD_DEPTH` | Big-stack entry points | Pre-existing. |
| `tcl_lsp_core::declaration` | `MAX_BODY_DEPTH` (local copy) | Big-stack entry points | Pre-existing. |
| `tcl_lsp_core::refactor::MAX_COMMAND_SEARCH_DEPTH` | `MAX_COMMAND_SEARCH_DEPTH` | Big-stack entry points | Pre-existing. |
| `tcl_lsp_core::semantic_tokens` (`collect_lambda_literal` family) | `MAX_TOKEN_RECURSION` = 32 | Big-stack entry points | Audited for this issue: every call site correctly threads depth; no bypass found. |
| `tcl_lsp_core::formatting::engine::format_body`/`format_switch_body` | `MAX_FORMAT_DEPTH` = 128 | **Conservative cap** (WASM host: `bigip-query-wasm`) | Reuses the existing `indent_level` parameter as the depth signal. Empirically measured 2 MiB-stack crash range: depth 800-1200. Past the cap, leaves the body unformatted rather than recursing. |
| `tcl_lsp_core::minify::minify_body` (+ `render_command`, `reconstruct_arg`/`reconstruct_raw`, `minify_switch_case_list`, `minify_lambda_literal`, `compress_expr`/`strip_expr_whitespace`/`shrink_expr_ast`/`tokenise_expr`) | `MAX_MINIFY_DEPTH` = 128 | **Conservative cap** (WASM host: `bigip-query-wasm`) | Same value/reasoning as `MAX_FORMAT_DEPTH` (near-identical per-level shape); not separately calibrated. |
| `tcl_irules::walker::{walk, recurse_token}` | `MAX_WALK_DEPTH` = 128 | **Conservative cap** (WASM host: `bigip-query-wasm`, via `tcl-bigip`) | Same value/reasoning as the two above. Past the cap, stops descending — references collected up to that point still stand. |
| `tcl_runtime::interp::eval_script_mode` (the WASM Tcl runtime's execution engine) | `NATIVE_EVAL_DEPTH_LIMIT` = 128 | **Conservative cap** (WASM host: this crate's own eventual embedding target) | The most severe of the pre-existing gaps: fires at *run time* on any deployed binary, not only during editing/linting, and — unlike the other walkers — this bug reached ordinary recursive **`proc` calls**, not just pathological control-flow nesting. `eval_script_mode` is the single choke point every script-body evaluation shares (control-flow bodies, proc bodies, `eval`/`uplevel`/`source`, command substitution); the pre-existing `RECURSION_LIMIT` (1000, matching real tclsh) only ever incremented for proc calls specifically, and — empirically confirmed via `probe_proc_recursion_unbounded` during investigation — was itself never a safe native-stack backstop: unbounded recursive `proc r {} { r }` overflowed the stack (SIGABRT) *before* reaching 1000 on a 2 MiB thread. This is a tree-walking interpreter (unlike C Tcl's bytecode-compiled control structures, which execute via a flat instruction loop with no per-nesting-level native recursion), so every nested body costs one more native stack-frame group regardless of Tcl-level semantics. `NATIVE_EVAL_DEPTH_LIMIT` is independent of the user-configurable `RECURSION_LIMIT`/`interp recursionlimit` (preserving its get/set contract exactly), checked first, and — like the other two — raises the same `"too many nested evaluations (infinite loop?)"` error. |
| `tcl_vm::cmd_control::eval_body` (the `if`/`while`/`for`/`foreach`/`lmap` runtime-command fallback for a computed command name or dynamic body) | `CONTROL_FALLBACK_DEPTH_LIMIT` = 24 | **Conservative cap** (WASM host: `tcl-vm-wasm`) | A more mature architecture than `tcl_runtime`: ordinary proc-to-proc calls are trampolined (no native recursion), so the existing `RECURSION_LIMIT` (1000) *is* a safe bound for them. `cmd_control.rs`'s fallback (driven through a computed command name — `set c if; $c ...` — defeating the compiled fast path) is different: invoking a *registered command* (full argument-processing machinery) on every recursive level genuinely recurses on the host stack, and empirically overflowed (SIGABRT) between depth 50 and 60 on a 2 MiB thread. **Deliberately not a cap on `Vm::eval_source` itself** (`Self::compile_cached` + `Self::run_module`), which this fallback calls into: an earlier version of this fix capped `eval_source` directly, since it's also the mechanism behind ordinary `[…]` command substitution (`subst.rs`), `switch`/`try`/`dict with`/OO-method/namespace-eval bodies, event dispatch, and `source` — and broke ordinary iRule execution, because that routine usage needs more depth than a conservative cap allows, nowhere near the actual danger (pure nested command substitution measured safe to at least depth 1000 on the same 2 MiB thread). The real risk is narrower — `cmd_control.rs`'s fallback specifically, not `eval_source` in general — so the fix moved to a dedicated `Vm::control_fallback_depth` counter, checked and incremented only around `cmd_control.rs`'s own body evaluation (`eval_body`, the single choke point all five of that module's fallback commands share). Unlike `recursion_depth` (which models `info level`/`interp recursionlimit` and must survive a coroutine suspend/resume via `swap_flow`), this new counter is pure native-stack bookkeeping with no Tcl-visible meaning, so it is deliberately *not* threaded through `swap_flow`/`eval_at_level`. |

Also fixed as part of this issue, not a recursion cap: `tcl_compiler::analyser`'s ~14 call sites that rebuilt a full-document `SourceMap`/`LineIndex` from scratch on every command at every nesting level — a genuine, severe (non-crashing) `O(document size × nesting depth)` DoS found while investigating. Now cached once per analysis run (`Analyser::cached_line_index`, self-invalidating on a source-length mismatch) and reused via `Analyser::source_map`.

Also fixed, mechanically identical to the big-stack entry points above but discovered in a follow-up audit: `f5-cli` (`irule minify` calls straight into the analyser on caller-supplied Tcl) and `tcl-debugger` (both the CLI file-load path and the DAP server's `launch` request) were missed by the first pass and ran on the unmodified default stack.

Confirmed **not** affected: `tcl-lexer` and `tcl-syntax`'s CST/segmenter
(iterative, brace/bracket counters in a loop), `ssa.rs`'s dominator-tree
rename walk (explicit worklist, by design — see the comment at its
construction site), and `sccp.rs` (worklist/fixpoint over CFG blocks, no
tree recursion).

## Testing

- `tcl_compiler::analyser::commands::tests` — TP/FP/TN coverage for
  `E207` at the exact `MAX_BODY_DEPTH` boundary.
- `tcl_compiler::lowering::tests::deeply_nested_if_past_max_lower_depth_barriers_not_crashes`
  / `shallow_nested_if_lowers_with_no_barrier` — the lowering cap, both
  sides.
- `tcl_compiler::analyser::param_traits::tests::deep_pass_bounds_alternating_if_apply_nesting`
  — the `apply`-reset bypass fix; reverting it reliably overflows the
  stack on the test harness's own default-sized thread.
- `tcl_compiler::optimiser::manager::tests::deeply_nested_if_survives_full_optimiser_pipeline`
  — end-to-end (source → lowering → all 5 optimiser passes) survival,
  spawned on its own 64 MiB thread since `lowering`'s cap barriers the
  input before the optimiser-level caps can be isolated by a source-text
  test alone.
- `tcl_compiler::codegen::structured::tests::deeply_nested_if_survives_structured_walk`
  / `very_long_elseif_chain_survives_structured_walk` — nested-body depth
  and `elseif`-chain length, as two *separate* recursion shapes.
- `tcl_lsp_core::formatting::engine::tests::deeply_nested_if_survives_formatting`
  / `moderately_nested_if_still_formats` — the formatter cap, both sides.
- `tcl_lsp_core::minify::tests::deeply_nested_if_survives_minify` /
  `moderately_nested_if_still_minifies` — the minifier cap, both sides.
- `tcl_irules::walker::tests::deeply_nested_command_substitution_does_not_crash`
  — the iRules walker cap.
- `tcl_runtime::interp::tests::deeply_nested_foreach_errors_instead_of_crashing`
  / `moderately_nested_foreach_still_runs` /
  `unbounded_proc_recursion_errors_instead_of_crashing` — both the
  control-flow-nesting gap and the proc-recursion gap this fix closes.
- `tcl_vm`'s `tests/cmd_control_e2e.rs`:
  `deeply_nested_dynamic_if_errors_instead_of_crashing` /
  `shallow_dynamic_if_still_runs` — the `cmd_control.rs` fallback cap, both
  sides, driven through the same computed-command-name technique the rest
  of that file already uses to reach the runtime fallback.
  `deeply_nested_command_substitution_is_unaffected` — proves the cap's
  narrow scope: ordinary nested `[…]` command substitution (which also
  routes through `Vm::eval_source`, but not through `cmd_control.rs`) is
  unaffected — regression coverage for the wrongly-scoped `eval_source`-wide
  cap this replaced, which broke exactly this.
- `rust/tcl-lsp-server/tests/e2e/issue996_stack_overflow.rs` — drives the
  real, packaged native server (not the analyser library function
  directly) with pathological input at the exact reported crash depth and
  well past the analyser's cap, and proves the *same server process*
  answers unrelated follow-up work afterwards. Also covers the LSP-reachable
  surface of three of the other fixed walkers against that same real
  server: `formatting_survives_deep_nesting` (`textDocument/formatting`),
  `minify_survives_deep_nesting` (the `tcl-lsp.minifyDocument` workspace
  command), and `irules_semantic_tokens_survive_deep_command_substitution`
  (`textDocument/semanticTokens/full` on an iRules document, exercising
  `tcl_irules::walker` via `tcl_lsp_core::semantic_tokens`).
- `editors/vscode/src/test/issue996.test.ts` — the same three LSP-reachable
  cases (diagnostics, formatting, `tcl-lsp.minifyDocument`) driven through
  a real VS Code extension host session against the packaged release
  server binary, against a committed fixture
  (`testFixture/issue996DeepNesting.tcl`, 300 levels) — proves the fix
  arrives through the actual editor integration, not just the raw LSP
  wire protocol.
- `rust/tcl-debugger/src/backend.rs`'s
  `launch_survives_deeply_nested_control_flow` — the debugger's `launch`
  path (both CLI and, transitively, DAP) survives deep nesting.
- `rust/f5-cli/tests/irule.rs`'s
  `minify_aggressive_survives_deeply_nested_irule` — drives the real,
  packaged `f5-query` binary (not a library call) with a deeply nested
  `.irule` file through `irule minify --aggressive`, the exact path a
  follow-up audit found was still running on the unmodified default stack
  after the first pass of this fix.

## Scope note

An initial audit for this issue found several recursive-descent locations
with no depth cap at all beyond the analyser itself: the WASM Tcl
runtime's execution engine, the five optimiser passes, the formatter, the
minifier, and the iRules reference walker. Continued empirical testing
while fixing those turned up two more of the same bug class:
`codegen::structured`'s `elseif`-chain recursion (a distinct shape from
body nesting) and `tcl_vm::cmd_control.rs`'s runtime fallback (which
turned out to overflow the stack at depth 50-60 — far below any of the
other walkers' thresholds). All are now fixed — see the table above; none
were judged acceptable to leave unguarded.

Three things worth remembering if this area changes again:

- The optimiser passes' cap (`MAX_OPTIMISER_WALK_DEPTH`) is defence in
  depth, not the primary mitigation for the normal pipeline — `lowering`'s
  cap upstream already bounds the IR these passes see to 256 levels
  before they run. If a future change ever lets one of these passes run on
  IR built by a path other than `lowering`, re-verify the cap actually
  matters for that path.
- `tcl_vm::cmd_control`'s *ordinary* static nested control flow (which
  compiles to flat bytecode via `exec.rs`) was never a concern and stays
  that way by construction — no per-nesting-level native recursion exists
  on that path at all. Only the runtime-command fallback needed a cap.
- **`Vm::eval_source` itself must never gain a uniform depth cap.** It is
  the shared mechanism behind ordinary command substitution, several
  compound-statement bodies, event dispatch, and `source` — not just
  `cmd_control.rs`'s fallback. A first attempt at this exact fix capped
  `eval_source` directly and broke ordinary iRule execution in CI
  (`live_session_class_match_ends_with_honours_operator`) before it was
  caught and re-scoped to `cmd_control.rs::eval_body` specifically. Any
  future native-stack-safety work on `tcl_vm` needs its own
  per-call-site counter the same way, not a change to `eval_source`.
