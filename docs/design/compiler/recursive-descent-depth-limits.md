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
consumer that analyses untrusted, generated, or minified Tcl (the LSP
server, `tcl diag`/`lint`/`validate`, the MCP server). This doc records the
root cause, the fix, and which recursive walkers are covered.

## Root cause: the depth cap existed, the stack budget didn't

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

Reproduced and measured directly: `ulimit -s 2048` (2 MiB) against the
*unfixed* binary crashes at nesting depth 130-140 — an exact match for the
issue's reported range — while the same input is fine up to the 256-level
cap on an 8 MiB stack. The cap was never the problem; the ambient stack
size the cap's own frames were allowed to run on was.

This is also why raising the magic number alone was rejected as a fix: a
bigger cap just needs proportionally more stack, and the *actual* frame
cost of the recursion chain silently grows every time a hot function in it
gains a local variable — which is plausibly how "256 was safe" drifted
into "256 isn't," with nobody touching the constant.

## The fix

Two independent, complementary changes, both required:

1. **Guarantee a generous, explicit stack budget at every process entry
   point**, rather than depending on the ambient thread's stack:
   - `tcl-lsp-server`/`tcl-mcp`: `main` builds its own
     `tokio::runtime::Builder::new_multi_thread()` with
     `.thread_stack_size(64 MiB)` instead of relying on `#[tokio::main]`'s
     (2 MiB) default.
   - `tcl` CLI: `tcl_cli::run` spawns every verb's dispatch on a dedicated
     `std::thread::Builder::new().stack_size(64 MiB)` thread, decoupling
     its crash behaviour from the OS/`ulimit` default.

   64 MiB is deliberately generous — the measured need is a few MiB even
   in an unoptimised debug build — so it also comfortably covers deeper
   nesting than the current 256-level caps allow, future frame-size
   growth, and multiple guarded walkers running on the same call stack
   at once.

2. **Every independently-recursive walker still needs its own depth cap.**
   The big stack makes a *bounded* recursion safe; it does not make an
   *unbounded* one safe — a walker with no cap at all will eventually
   outrun any fixed stack size given deep enough input. `lowering/mod.rs`
   (`Lowerer::lower_script`/`lower_body`) had exactly this gap: it runs
   *before* `cfg_builder`'s own guard (`MAX_LOWER_DEPTH`, already
   present), so an unguarded lowering pass crashed first and made the
   downstream guard moot regardless of stack size. It is now capped the
   same way, at `MAX_LOWER_NEST_DEPTH = 256`, matching the analyser's and
   CFG builder's cap.

   A guard that exists but can be *bypassed* is the same bug in a
   different shape. `param_traits.rs`'s deep pass had one: its `apply`
   (`ArgRole::LambdaLiteral`) handling re-entered the public,
   depth-0 entry point instead of threading its own depth forward, so
   alternating `if {…} { apply {x {…}} … }` nesting reset the logical
   `MAX_DEPTH` counter on every `apply` boundary while the native call
   stack kept growing regardless — unboundedly, for however deep the
   input alternates. Fixed by adding an internal
   `infer_param_traits_deep_at_depth` entry point that threads `depth + 1`
   through the `apply` re-entry instead of resetting to `0`.

3. **The depth cap trips as a diagnostic, not silent truncation.** Before
   this fix, `analyse_body` hitting `MAX_BODY_DEPTH` silently stopped
   descending with no signal to the user — diagnostics for the excess
   nesting just never appeared, and there was nothing to tell an editor
   "I gave up here." Real `tclsh` doesn't behave this way either: exceeding
   `interp recursionlimit` raises a catchable `"too many nested
   evaluations (infinite loop?)"` error. `analyse_body` now emits
   **[`E207`](../../kcs/codes/kcs-diagnostic-e207-nesting-depth-exceeds-limit.md)**
   once per analysis run when the cap trips, anchored on the body where
   descent stopped — visible, not a quiet gap in coverage.

## Guarded walkers (cap = 256 unless noted)

| Walker | Cap constant | Notes |
|---|---|---|
| `tcl_compiler::analyser::commands::analyse_body` | `MAX_BODY_DEPTH` | Emits `E207` once per run when tripped (see above). |
| `tcl_compiler::cfg_builder::CfgBuilder::lower_script` | `MAX_LOWER_DEPTH` | Pre-existing; stops descending, truncated-but-valid CFG. |
| `tcl_compiler::lowering::Lowerer::lower_script`/`lower_body` | `MAX_LOWER_NEST_DEPTH` | Added by this fix. Emits a `Statement::Barrier` ("nesting depth exceeds analysis limit") past the cap so downstream passes (SCCP, DCE, …) treat the unanalysed region as unknown-effect, not dead code. |
| `tcl_compiler::analyser::param_traits::scan_deep` / `infer_param_traits_deep_at_depth` | `MAX_DEPTH` (8) | Bypass fixed by this change — see above. |
| `tcl_lsp_core::references` (`scan_my_method_region`, `scan_obj_method_region`, `scan_next_dispatch_region`) | `MAX_DISPATCH_SCAN_DEPTH` | Audited for this issue: every recursive call site correctly threads `depth + 1`; no bypass found. |
| `tcl_lsp_core::folding` | `MAX_FOLD_DEPTH` | Pre-existing. |
| `tcl_lsp_core::declaration` | `MAX_BODY_DEPTH` (local copy) | Pre-existing. |
| `tcl_lsp_core::refactor::MAX_COMMAND_SEARCH_DEPTH` | `MAX_COMMAND_SEARCH_DEPTH` | Pre-existing. |
| `tcl_lsp_core::semantic_tokens` (`collect_lambda_literal` family) | `MAX_TOKEN_RECURSION` (32) | Audited for this issue: every call site correctly threads depth; no bypass found. |

## Known gaps — not covered by this fix

An audit for this issue found several other recursive-descent walkers with
**no depth cap at all**. The big-stack entry points (above) push their
crash threshold far out, but an uncapped walker is not made safe by a
bigger stack in principle — only in practice, for however deep real input
happens to nest. Recorded here rather than silently left unaudited:

- `runtime/rust` (`cmd_control.rs` + `interp.rs`'s `eval_control_body`
  family) — the WASM runtime's *execution* engine, not just static
  analysis; the most severe of these because it fires at run time on any
  deployed binary, not only during editing/linting.
- `tcl_compiler::optimiser::{propagation, expr_simplify,
  pattern_recognition, structure_elimination, code_sinking}` — each walks
  `Script`/`Statement::If`/`While`/`For`/`Foreach`/`Try`/`Switch` bodies
  recursively with no cap. Partially mitigated in practice by the
  `lowering` cap above: since `Statement::Barrier` now stands in for
  anything past 256 levels of *source* nesting, these passes never see
  IR nested deeper than that in the first place.
- `tcl_lsp_core::formatting::engine` (`format_body`) and
  `tcl_lsp_core::minify` (`minify_body`) — both uncapped.
- `tcl_irules::walker` — uncapped.
- `tcl_compiler::codegen::structured` (used by the WASM backend for
  `if`/`while`/`for`; `foreach`/`switch`/`try`/`catch`/`dict for` already
  fall back to an eval barrier, so this is narrower in scope) — uncapped.
- `tcl_vm::cmd_control` — uncapped on its fallback path only (a computed
  command name, dynamic body, or malformed-grammar barrier); ordinary
  static nested control flow compiles to flat bytecode via `exec.rs`,
  which does not recurse per nesting level.

Confirmed **not** affected: `tcl-lexer` and `tcl-syntax`'s CST/segmenter
(iterative, brace/bracket counters in a loop), `ssa.rs`'s dominator-tree
rename walk (explicit worklist, by design — see the comment at its
construction site), and `sccp.rs` (worklist/fixpoint over CFG blocks, no
tree recursion).

## Testing

- `tcl_compiler::analyser::commands::tests` — TP/FP/TN coverage for
  `E207` at the exact `MAX_BODY_DEPTH` boundary (`depth_exactly_at_cap_emits_no_e207`,
  `depth_one_past_cap_emits_e207_exactly_once`,
  `depth_far_past_cap_still_emits_e207_exactly_once`,
  `shallow_nesting_never_emits_e207`).
- `tcl_compiler::lowering::tests::deeply_nested_if_past_max_lower_depth_barriers_not_crashes`
  / `shallow_nested_if_lowers_with_no_barrier` — the lowering cap, both
  sides.
- `tcl_compiler::analyser::param_traits::tests::deep_pass_bounds_alternating_if_apply_nesting`
  — the `apply`-reset bypass fix; reverting it reliably overflows the
  stack on the test harness's own default-sized thread.
- `rust/tcl-lsp-server/tests/e2e/issue996_stack_overflow.rs` — drives the
  real, packaged native server (not the analyser library function
  directly) with pathological input at the exact reported crash depth and
  well past the analyser's cap, and proves the *same server process*
  answers unrelated follow-up work afterwards.
