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
most of the recursive-descent walkers in the workspace. A first pass fixed
the analyser and a handful of siblings (an optimiser pass tree, a
formatter, a minifier, an iRules reference walker, a WASM Tcl runtime, a
second-order recursion in `codegen::structured`, and `tcl_vm`'s runtime
control-flow fallback). A second, systematic workspace-wide sweep — five
parallel research agents, one per major subsystem — then found the bug
class was far more widespread than the first pass's reactive, ad-hoc
discovery had caught: dozens more unguarded walkers across `tcl-lexer`,
`tcl-lsp-core`, `tcl-compiler`, `tcl-regex`, `tcl-bigip-io`/
`tcl-bigip-query`, `tcl-vm`, and `runtime/rust`, plus two more native
binaries (`bpf-tcl`, `bigip-report-gen`) missing the same big-stack
entry-point guard the first pass added elsewhere. That sweep is also what
motivated **[the unified mechanism](#unified-mechanism-recursionlimit--recursionguard)**
below: fixing this many call sites with one-off `const MAX_X_DEPTH: u32`
copies at every site was already becoming inconsistent (some compared
`depth > LIMIT`, others `depth >= LIMIT`, with the off-by-one worked out
independently every time) before the sweep even started finding the rest.
This doc records the root cause, the shared mechanism, the fixes, and
which walkers are covered.

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
  `tcl_irules`, `tcl_vm`, `tcl-regex`, `tcl-bigip-query`) are also compiled
  to WASM and consumed by a host (`bigip-query-wasm`, `tcl-vm-wasm`,
  `tcl-explorer-wasm`, `bigip-report-gen/wasm`) whose stack budget is
  **entirely outside this repo's control** — commonly far smaller than
  2 MiB.

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
nest. The reactive first pass found several of these by chasing where the
first fix's own testing led; the systematic sweep found dozens more by
deliberately searching every crate for the pattern rather than waiting for
a crash to point at it. Recurring shapes worth naming, because they each
defeated an *existing* guard elsewhere rather than simply lacking one:

- **A second, unguarded recursion running *before* an existing guard**
  (`lowering/mod.rs` before the first pass's fix).
- **A guard that could be *bypassed* by resetting its own counter**
  (`param_traits.rs`'s `apply` re-entry).
- **A recursion shape the existing cap doesn't bound at all**, because it
  isn't the same axis as the thing the cap counts: `codegen::structured`'s
  `elseif`-chain length (independent of body-nesting depth);
  `ExprNode`-tree recursion inside a *single* `expr {...}` (independent of
  surrounding Statement/Script nesting, and not bounded by `lowering`'s
  cap at all, since one expression's own operator tree is parsed once);
  raw nested `[cmd [cmd …]]` command-substitution *text* inside one word
  (independent of Statement-tree nesting for the same reason); `tcl-regex`'s
  *execution-time* dissection/backtracking recursion against a long
  subject string (independent of `MAX_PARSE_DEPTH`, which only bounds the
  *pattern's* parsed AST depth, not how many times a repeat quantifier
  matches against the input).
- **An interpreter's own `RECURSION_LIMIT`** (matching real Tcl's default)
  that was itself never actually a safe native-stack backstop, because it
  only bounded proc-call nesting, not the *general* eval-recursion nesting
  a tree-walking interpreter also pays a native stack frame for — or, in
  `tcl-vm`'s case, because it only applies to the proc-to-proc trampoline
  and TclOO method dispatch turned out to bypass that trampoline entirely
  (see the `tcl-vm` entries below).
- **Construction/drop of a deeply-nested value**, independent of any
  operation performed *on* it: several `tcl-vm`/`tcl-bigip-query` fixes'
  own regression tests had to work around the fact that a `Value` with no
  custom `Drop` implementation recurses through the compiler-generated
  drop glue once per nesting level when it goes out of scope — a related
  but *out-of-scope* native-stack risk in those types' representations,
  noted in the relevant test doc comments rather than fixed here.

## Unified mechanism: `RecursionLimit`/`RecursionGuard`

`tcl_core_types::{RecursionLimit, RecursionGuard}`
(`rust/tcl-core-types/src/recursion.rs`) is the shared, dependency-free,
`no_std`, `unsafe`-free primitive every guarded walker in the workspace now
builds on, added once the sweep above made clear how many near-identical,
independently-invented `const MAX_X_DEPTH: u32` copies already existed.
It intentionally centralises only the *bookkeeping* — compare, increment,
decrement, tested thoroughly once — not the "what happens when the limit
trips" behaviour, which stays domain-specific by design (see point 4 under
[The fix](#the-fix), unchanged by this refactor).

```rust
pub struct RecursionLimit(pub u32);
impl RecursionLimit {
    /// `depth > self.0` — the nesting level about to run, *including* it.
    pub const fn exceeded(self, depth: u32) -> bool { depth > self.0 }
}
```

Two call-site shapes cover every walker in the workspace, both built on
the same `depth > limit` comparison:

1. **An explicit `depth: u32` parameter threaded through recursive
   calls** — the common case, and every walker fixed by the original
   pass and the sweep that isn't `tcl-vm`'s two counters below. Each call
   already carries a `depth` (the nesting level of the node it's about to
   process), so the guard is a direct
   `if MAX_X_DEPTH.exceeded(depth) { <domain-specific fallback>; }` at the
   top of the function, incrementing `depth + 1` at each recursive call —
   no state to own. See `rust/tcl-lexer/src/lexer.rs`
   (`MAX_ARRAY_INDEX_DEPTH`) for a compact worked example.
2. **A counter stored on a long-lived struct**, incremented before a
   nested call and decremented after. Two sub-shapes exist here:
   - When the recursive call does **not** need to re-borrow the owning
     struct mutably through several other function calls,
     [`RecursionGuard`] wraps the counter in RAII: `enter()` checks and
     increments, returning a guard whose `Drop` decrements again —
     including on an early return via `?`, an early `break`/`return`, or
     an unexpected panic unwind, which a manual increment/decrement pair
     can silently get wrong on any of those paths.
   - When the recursive call **does** pass back through several other
     `&mut Self`-taking methods before recursing again — `tcl-vm`'s
     `Vm::control_fallback_depth`/`oo_dispatch_depth`, where the call
     chain runs back out through the engine's dispatch loop before
     re-entering — `RecursionGuard`'s borrow can't span that gap, so
     these two use a manual `enter_*`/`exit_*` method pair on `Vm`
     instead, with the same `LIMIT.exceeded(counter + 1)` check
     (algebraically identical to the more familiar `counter >= LIMIT`
     phrasing — see [`RecursionLimit::exceeded`]'s own doc comment for
     the derivation) and `counter.saturating_sub(1)` on the way out. This
     is a deliberate, narrow exception to "always use `RecursionGuard`
     for the counter shape," not an oversight.

Every already-fixed call site — from the original pass and the sweep
alike — was migrated to this type (`RecursionLimit` for the constant,
`.exceeded()` for the comparison) as part of closing out the sweep, so
searching the workspace for `RecursionLimit` now finds every guarded
walker in one query, and a boundary bug in the comparison itself only
needs fixing (and testing) once, in `tcl-core-types`, rather than
re-litigated at each of the ~90 call sites it's used from.

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
   - `tcl` CLI / `f5-cli` / `bpf-tcl` / `bigip-report-gen`: each binary's
     entry point spawns its dispatch on a dedicated
     `std::thread::Builder::new().stack_size(64 MiB)` thread, decoupling
     crash behaviour from the OS/`ulimit` default. `bpf-tcl` and
     `bigip-report-gen` were missed by the original pass (which covered
     `f5-cli`/`tcl-debugger` in a follow-up audit but not these two) and
     added by the sweep — both call straight into the same
     depth-capped-but-stack-hungry compiler/parser pipeline on an
     otherwise-unguarded main-thread stack.
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
   `tcl_irules`'s walker, `tcl_vm`'s `cmd_control.rs` fallback and TclOO
   dispatch, `tcl-lexer`, `tcl-regex`, `tcl-bigip-io`, `tcl-bigip-query`)
   — strategy 1 doesn't apply here because this repo does not control the
   WASM host's stack. Each of these was calibrated empirically against a
   2 MiB native thread (the same class of ambient budget that made the
   original crash reproducible) where practical, then set well under the
   measured crash floor to leave real margin for a meaningfully smaller
   WASM stack; a handful (documented individually below) used a clearly
   conservative estimate instead, where empirical measurement wasn't
   practical in the time available. Values differ because the measured
   per-level native-frame cost differs by walker — see the tables below
   for each one's specific number and reasoning.

3. **Every independently-recursive walker still needs its own depth cap
   (or, for a genuinely distinct recursion *axis*, its own separate
   cap)**, regardless of which stack-budget strategy applies, and
   regardless of whether another cap already bounds a *different* axis of
   the same input. `lowering.rs` (`Lowerer::lower_script`/`lower_body`)
   had a cap-shaped gap: it runs *before* `cfg_builder`'s own guard
   (`MAX_LOWER_DEPTH`, already present), so an unguarded lowering pass
   crashed first and made the downstream guard moot regardless of stack
   size. `param_traits.rs`'s deep pass had a bypass: its `apply`
   (`ArgRole::LambdaLiteral`) handling re-entered the public, depth-0
   entry point instead of threading its own depth forward, so alternating
   `if {…} { apply {x {…}} … }` nesting reset the logical `MAX_DEPTH`
   counter on every `apply` boundary while the native call stack kept
   growing regardless. `codegen::structured`'s `emit_if` recurses once
   per `elseif` link via a self-call — a *different* recursion shape from
   nested bodies, unbounded by `MAX_LOWER_DEPTH`-style caps entirely, so a
   pathologically long `elseif` chain needed its own guard threaded
   through the same `depth` budget. The sweep found the same "different
   axis, same walker" shape twice more, at much larger scale: `expr`'s
   `ExprNode` operator-tree recursion (bounded by neither `MAX_BODY_DEPTH`
   nor `MAX_LOWER_NEST_DEPTH` — one expression's own nesting is parsed
   once, independent of the surrounding block structure) and raw nested
   `[cmd [cmd …]]` command-substitution text embedded in a single argument
   word (same reasoning) each turned out to need their own cap across
   dozens of `tcl-compiler` functions that all walk one or the other.

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
   unstructured statement kind already uses; `formatting`/`minify` leave
   the over-deep body unformatted/unminified rather than corrupting it;
   `tcl_irules::walker` stops collecting references past the cap (the
   references found up to that point still stand); `tcl_runtime` and
   `tcl_vm` both raise the same catchable `"too many nested evaluations
   (infinite loop?)"` error real `tclsh` uses for the conceptually
   identical failure. The sweep's `tcl-compiler` fixes generalise this
   principle to non-diagnostic return types: a boolean "does this have an
   observable side effect / could this equal something else" helper
   answers conservatively (`true`) past the cap, biasing toward *not*
   applying an optimisation rather than risking an unsound one; a
   collector simply stops collecting and returns what it already has; a
   tree-rewriting pass (`inlining::rename::rewrite_expr`) passes the
   over-deep subtree through unchanged rather than attempting a
   structurally-truncated rewrite. `tcl-lexer`'s and `runtime/rust`'s
   array-index/bracket-text fixes take a third shape again: past the cap,
   the walker stops treating the nested construct specially and falls
   through to scanning it as an ordinary character, so the rest of the
   parse degrades gracefully (a slightly-imprecise span for the
   pathological tail) instead of erroring the whole parse out.

## Guarded walkers — original pass

| Walker | Cap constant / value | Stack strategy | Notes |
|---|---|---|---|
| `tcl_compiler::analyser::commands::analyse_body` | `MAX_BODY_DEPTH` = 256 | Big-stack entry points | Emits `E207` once per run when tripped. |
| `tcl_compiler::cfg_builder::CfgBuilder::lower_script` | `MAX_LOWER_DEPTH` = 256 | Big-stack entry points | Pre-existing; stops descending, truncated-but-valid CFG. |
| `tcl_compiler::lowering::Lowerer::lower_script`/`lower_body` | `MAX_LOWER_NEST_DEPTH` = 256 | Big-stack entry points | Emits a `Statement::Barrier` past the cap. |
| `tcl_compiler::analyser::param_traits::scan_deep` / `infer_param_traits_deep_at_depth` | `MAX_DEPTH` = 8 | Big-stack entry points | `apply`-reset bypass fixed by this change. |
| `tcl_compiler::optimiser::{propagation, expr_simplify, pattern_recognition, structure_elimination, code_sinking}` | `MAX_OPTIMISER_WALK_DEPTH` = 256 (shared, `optimiser/mod.rs`) | Big-stack entry points | Defence in depth: `lowering`'s cap already bounds the IR these passes see in the normal pipeline to 256 levels before they run. `code_sinking` additionally caps three more mutually-recursive query families, each answering conservatively past the cap. |
| `tcl_compiler::codegen::structured::{walk_stmt, emit_if, emit_loop}` | `MAX_STRUCTURED_DEPTH` = 256 | Big-stack entry points | Covers both nested-body depth *and*, independently, `emit_if`'s own `elseif`-chain self-recursion. |
| `tcl_lsp_core::references` (`scan_my_method_region`, `scan_obj_method_region`, `scan_next_dispatch_region`) | `MAX_DISPATCH_SCAN_DEPTH` = 256 | Big-stack entry points | Pre-existing; audited, no bypass found. |
| `tcl_lsp_core::folding` | `MAX_FOLD_DEPTH` = 256 | Big-stack entry points | Pre-existing. |
| `tcl_lsp_core::declaration` | `MAX_BODY_DEPTH` = 256 (local copy) | Big-stack entry points | Pre-existing. |
| `tcl_lsp_core::refactor` | `MAX_COMMAND_SEARCH_DEPTH` = 256 | Big-stack entry points | Pre-existing. |
| `tcl_lsp_core::semantic_tokens` (`collect_lambda_literal` family) | `MAX_TOKEN_RECURSION` = 32 | Big-stack entry points | Pre-existing; audited, no bypass found. |
| `tcl_lsp_core::formatting::engine::format_body`/`format_switch_body` | `MAX_FORMAT_DEPTH` = 128 | **Conservative cap** (WASM host: `bigip-query-wasm`) | Empirically measured 2 MiB-stack crash range: depth 800-1200. |
| `tcl_lsp_core::minify::minify_body` (+ siblings) | `MAX_MINIFY_DEPTH` = 128 | **Conservative cap** | Same value/reasoning as `MAX_FORMAT_DEPTH`. |
| `tcl_irules::walker::{walk, recurse_token}` | `MAX_WALK_DEPTH` = 128 | **Conservative cap** (WASM host: `tcl-bigip`) | Same value/reasoning as the two above. |
| `tcl_runtime::interp::eval_script_mode` | `NATIVE_EVAL_DEPTH_LIMIT` = 128 | **Conservative cap** | The most severe of the original gaps: reachable via ordinary recursive `proc` calls, not just pathological nesting — `RECURSION_LIMIT` (1000, matching tclsh) was itself never a safe native-stack backstop, empirically overflowing before reaching 1000 on a 2 MiB thread. |
| `tcl_vm::cmd_control::eval_body` | `CONTROL_FALLBACK_DEPTH_LIMIT` = 24 | **Conservative cap** (WASM host: `tcl-vm-wasm`) | Ordinary proc-to-proc calls are trampolined (native-stack-free); this fallback (driven through a computed command name) genuinely recurses on the host stack, empirically overflowing between depth 50-60 on a 2 MiB thread. **Deliberately not a cap on `Vm::eval_source` itself** — see the [scope note](#scope-note). |

Also fixed as part of the original pass, not a recursion cap:
`tcl_compiler::analyser`'s ~14 call sites that rebuilt a full-document
`SourceMap`/`LineIndex` from scratch on every command at every nesting
level — a genuine, severe (non-crashing) `O(document size × nesting
depth)` DoS. Now cached once per analysis run.

Also fixed, mechanically identical to the big-stack entry points above but
discovered in a follow-up audit: `f5-cli` (`irule minify` on
caller-supplied Tcl) and `tcl-debugger` (both the CLI file-load path and
the DAP server's `launch` request) were missed by the first pass and ran
on the unmodified default stack.

## Guarded walkers — the sweep

Five parallel agents, one per major subsystem, re-audited the whole
workspace once the original pass's own testing kept turning up more
instances than it fixed — see [Scope note](#scope-note) for why a
reactive, crash-driven discovery process was judged insufficient on its
own. Grouped by crate; every entry uses the shared
`RecursionLimit`/`RecursionGuard` mechanism above.

**`tcl-lexer`** — the lowest-level, most widely-depended-on crate in the
sweep, so these three were the highest-value fixes found: all three are
reachable directly on raw, untrusted document text, before any
higher-level guard ever runs.

| Walker | Cap | Notes |
|---|---|---|
| `Lexer::scan_array_index_body`/`skip_var_in_index` (nested `$a($b($c(...)))`) | `MAX_ARRAY_INDEX_DEPTH` = 64 | Empirically: SIGABRT depth 20,000-25,000 on a 2 MiB thread. Past the cap, a nested `$` is scanned as an ordinary character. |
| `structural_index.rs`'s three scanners (`BracketIndex::scan_cmd_sub`; `BraceIndex`'s `scan_script`/`scan_quoted`; `scan_complete`/`scan_complete_quoted` behind `script_is_complete`/`command_boundaries`) | `MAX_NESTED_BRACKET_DEPTH` = 128 | The last of these backs `script_is_complete`, called directly on raw document text from `tcl-lsp-server::compute_base_analysis`. Empirically: SIGABRT depth 10,000-50,000 depending on the scanner. |
| `expr_lexer.rs`'s `Inner::scan_array_index` (nested array index inside an `expr`) | `MAX_EXPR_ARRAY_INDEX_DEPTH` = 64 | Bypassed `tcl-syntax`'s already-capped Pratt parser entirely — the whole nested chain is swallowed into one `Variable` token during lexing, before the parser ever sees per-level tokens to count. Empirically: SIGABRT depth 100,000-200,000. |

**`tcl-lsp-core`** — seven Scope-tree/word-nesting walkers, all sharing
one new crate-wide constant (`crate::MAX_SCOPE_WALK_DEPTH` = 256, defined
in `lib.rs`) for the Scope-tree ones, since a `Scope` node is only ever
created for a `namespace`/`proc`/`method` body and the analyser already
caps that at the same 256 — this cap is defence-in-depth against a scope
tree built or received some other way, not a currently-reproducible crash
via the public analyser API.

| Walker | Notes |
|---|---|
| `irules_context::scan_when_context`/`collect_when_events` | Own local `MAX_WHEN_SCAN_DEPTH` = 256 (word-nesting, not Scope-tree). Reachable from completion/code-actions on essentially every keystroke. |
| `document_symbols::scope_symbols`/`proc_symbol` | Powers `textDocument/documentSymbol`. |
| `definition`'s `innermost` (nested in `variable_scope_extent`), `collect_alias_spans`, `collect_shared_span_refs`, `visit` (nested in `cross_namespace_qualified_vars`) | Go-to-definition / variable-linking / cross-namespace completion. |
| `graphs::scope_to_value`, `count_namespaces`, `count_variables` | Feeds the `symbol_graph` JSON payload. |
| `inlay_hints::walk_scope_type_hints` | Type-hint emission. |
| `refactor::inline_variable::walk_scopes` | Own local wrapper (`walk_scopes_at_depth`) around the shared crate constant. |
| `package_resolver::collect_source_targets` | Own local `MAX_SOURCE_TARGET_SCAN_DEPTH` = 256 (word-nesting, not Scope-tree) — nested `[...]`/`{...}`/`"..."` wrapper words in a `pkgIndex.tcl` `package ifneeded` body. |

**`tcl-compiler`** — by far the largest single chunk found: ~55 functions
across ~30 files, split into three tiers by how directly exploitable each
was.

*Tier 1A — `ExprNode` operator-tree recursion* (genuinely unbounded; a
single `expr {((((...))))}` or a long `1+1+1+…` chain directly controls
native stack depth, independent of any Statement/Script-tree cap). 29
functions across `type_infer.rs`, `codegen/expressions.rs`, `taint.rs`,
`interprocedural.rs`, `optimiser/helpers/expr_simplify.rs`,
`optimiser/elimination.rs`, `optimiser/code_sinking.rs`, `uri_split.rs`,
`shimmer/{expr,commit}.rs`, `ir_helpers.rs`, `intervals.rs`,
`interval_bounds.rs`, `inlining/rename.rs`,
`analyser/diagnostics/{usage,dataflow}.rs`, `connection_scope.rs`. Shared
cap: `depth_guard::MAX_EXPR_NODE_DEPTH` = 256.

*Tier 1B — raw nested `[cmd …]` command-substitution text* (recurses on
bracket nesting inside a single word's raw text, independent of
Statement-tree nesting for the same reason as Tier 1A). 6 functions across
`taint.rs`, `optimiser/elimination.rs`, `optimiser/end_offset.rs`,
`interprocedural.rs`, `analyser/commands.rs`. Shared cap:
`depth_guard::MAX_BRACKET_TEXT_DEPTH` = 256.

*Tier 2 — Script/Statement-tree walkers with no cap of their own*
(defence-in-depth: transitively bounded to 256 today by `lowering`'s
existing `MAX_LOWER_NEST_DEPTH`, so not currently reproducible as a crash
via the normal pipeline, but inconsistent with every other full-tree
walker in the crate having its own explicit cap — see point 3 under
[The fix](#the-fix)). ~20 functions across `ir.rs`, `ir_helpers.rs`,
`ssa.rs`, `interprocedural.rs`, `lowering/mod.rs`,
`optimiser/{chain_fold,tail_call,end_offset}.rs`, `command_binding.rs`,
`var_escape/{walker,slot_resolution}.rs`, `analyser/oo.rs`,
`cfg_builder/mod.rs`, `inlining/mod.rs`. Each reuses a sibling constant
already in its own file where one existed (e.g. `optimiser::mod`'s
`MAX_OPTIMISER_WALK_DEPTH`), or declares a small local one otherwise — no
new shared module for this tier.

Both Tier 1 constants live in the new `tcl-compiler::depth_guard` module,
the one new shared module this sweep added.

**`tcl-vm`** (see also the [`RecursionGuard` exception](#unified-mechanism-recursionlimit--recursionguard)
above for why the two counter-based ones here don't use `RecursionGuard`):

| Walker | Cap | Notes |
|---|---|---|
| `cmd_oo.rs::run_step` (TclOO method dispatch — `$obj method`/`my method`/`next`/`nextto`) | `OO_DISPATCH_DEPTH_LIMIT` = 20 | **The most severe finding of the entire sweep.** Method dispatch bypasses the proc-call trampoline entirely — every nested method call is a genuine native `run_activation` call, not a trampoline push. Empirically: SIGABRT at depth 45-48 on a 2 MiB thread, *20x below* the existing (irrelevant, for this path) `RECURSION_LIMIT` of 1000. Guarded at `run_step` specifically (not `oo_dispatch`) because `my method` and `next`/`nextto` — the two most common ways a method recurses — reach `run_step` directly, bypassing `oo_dispatch`. A real, deliberate compatibility gap vs. tclsh (a recursive method now errors at depth 20, not 1000); the architecturally correct fix is routing method dispatch through the same trampoline proc calls use, which is a substantially larger change than this mitigation. |
| `value.rs::Value::to_str` (nested list/dict stringification) | `MAX_LIST_TO_STR_DEPTH` = 256 | Empirically: SIGABRT depth 1200-1250. Reachable via a plain loop (`for {...} {set v [list $v]}`), no `{*}` needed. |
| `cmd_dict.rs::set_path`/`unset_path`, `cmd_list.rs::lpop_remove`, `exec.rs::lset_descend` | *(none — rewritten iteratively)* | Multi-key/multi-index path-walking; rewritten as an explicit work-stack instead of one native call per key/index, eliminating the native-stack risk entirely rather than bounding it — mirroring this file's own pre-existing iterative `get_path`. |

**`tcl-regex`** (Finding A1 — execution-time recursion, distinct from the
pre-existing `MAX_PARSE_DEPTH` which only bounds the *pattern's* parsed
AST):

| Walker | Cap | Notes |
|---|---|---|
| `Matcher::dissect`/`dissect_seq`/`dissect_repeat` (POSIX leftmost-longest dissection, runs after every repeat-quantifier match) | `MAX_DISSECT_DEPTH` = 256 | Reachable from ordinary `regexp`/`regsub` on a repeat-heavy pattern (`.*`, `a+`) against a long subject — depth scales with subject length, not pattern complexity. Empirically: SIGABRT depth ~2400. |
| `Bt::m`/`m_seq`/`m_repeat`/`m_star`/`m_backref` (backtracking path, backreference patterns) | `MAX_BT_DEPTH` = 256 | Empirically: SIGABRT depth ~2200-2500. |

**`tcl-bigip-io`/`tcl-bigip-query`** (Findings A2/A3):

| Walker | Cap | Notes |
|---|---|---|
| `tcl-bigip-io::openpgp::extract_literal` | `MAX_COMPRESSED_PACKET_DEPTH` = 16 | Nested OpenPGP Compressed Data packets in an encrypted `.ucs`/`.scf` archive — fully attacker-controlled via a crafted archive. A legitimate archive never nests more than one level. |
| `tcl-bigip-query`'s six `Value`-tree walkers (`special::walk`, `builtins/mod.rs`'s `to_jsonable`/`walk_paths`/`set_at_path`/`delete_at_path`/`flatten_go`, `value::py_eq`, `edit_plan::format_value`, `builtins/encoding::json_to_value`) | `value::MAX_VALUE_WALK_DEPTH` = 64 | Mirrors this crate's own pre-existing `parser::MAX_PARSE_DEPTH` (also 64) — left uncapped despite the crate's parser/eval already being hardened against this exact bug class. |

**`runtime/rust`** (the standalone WASM Tcl runtime):

| Walker | Cap | Notes |
|---|---|---|
| `cmd_dict.rs::dict_path_set`/`dict_path_unset` | *(none — rewritten iteratively)* | Same treatment as `tcl-vm`'s dict-path functions above. |
| `cmd_oo.rs::linearize_class`/`gather_class_props` | `MAX_MRO_DEPTH` = 1024, `MAX_MRO_VISITS` = 200,000 | Ports `tcl_syntax::mro`'s already-hardened two-cap approach (depth *and* total-visit budget, since a mixin/superclass graph can be wide as well as deep) — this runtime's own MRO walk was the one place that port had been missed. Empirically: SIGABRT depth 100-150 on a 256 KiB stack via a deep `mixin` chain. |
| `parse.rs::scan_parts`, `subst.rs::resolve_parts` | `MAX_ARRAY_INDEX_DEPTH` = 64 (each file's own copy) | Mirrors `tcl-lexer`'s array-index fix exactly, including the "fall through to an ordinary character past the cap" fallback. Empirically: SIGABRT depth 100-150 on a 256 KiB stack. |

**Native binaries** — `bpf-tcl` and `bigip-report-gen` gained the same
64 MiB dedicated-thread stack wrapper described under strategy 1 above;
see that section rather than repeating it here.

Confirmed **not** affected, checked during the sweep: `tcl-syntax`'s
Pratt expression parser (already capped, `MAX_EXPR_DEPTH` = 256, and the
model several sweep fixes above cite); `tcl-syntax::mro`'s TclOO
linearisation (already capped, the model `runtime/rust`'s MRO fix ports);
`tcl-explorer::cst.rs` (already capped, `MAX_DEPTH` = 256); `ssa.rs`'s
dominator-tree rename walk and `sccp.rs` (explicit worklist/fixpoint, no
tree recursion, by design). Also checked and found to be a *different*,
out-of-scope concern rather than this bug class: `cmd_oo.rs::self_reachable`
(`runtime/rust`, an O(n²)-ish algorithmic-complexity cost in
`superclass`-only MRO construction, not a native-stack recursion depth
problem) and `Value`'s lack of a custom `Drop` implementation in both
`tcl-vm` and `tcl-bigip-query` (see [Root cause](#root-cause-two-distinct-problems-not-one)'s
last bullet).

## Testing

Every guarded walker above has at least one regression test proving deep
or adversarial input survives (typically well past the empirical crash
floor, or — where the crash floor wasn't independently measured — well
past the new cap) and at least one proving realistic/moderate-depth input
is exactly unaffected, following one consistent doc-comment convention
throughout the codebase:

```rust
/// Regression coverage for issue #996: `<function>` recurses once per
/// <nesting unit>, with no depth cap before this fix. Empirically,
/// unguarded input overflowed the native stack (SIGABRT) around depth
/// <D> on a 2 MiB thread (`cargo test`'s per-test default). <N> is
/// comfortably past both that crash range and `MAX_X_DEPTH` (<cap>); the
/// assertion is that <call> returns at all, not what it returns.
```

A handful of tests need to spawn their own dedicated large-stack thread
(`std::thread::Builder::new().stack_size(64 * 1024 * 1024)`) rather than
asserting on the test harness's own ~2 MiB default thread, when the input
needed to exercise the fix would itself overflow that default independent
of the fix (e.g. `document_symbols`'s namespace-nesting tests — building
and dropping the deeply-nested `Scope`/`Value` fixture costs real native
stack on its own, separate from whatever the test is actually proving).
Follow whichever sibling test in the same file already does this as the
template rather than reinventing the pattern.

Standout tests worth knowing about specifically:

- `tcl_compiler::analyser::commands::tests` — TP/FP/TN coverage for
  `E207` at the exact `MAX_BODY_DEPTH` boundary.
- `tcl_compiler::optimiser::manager::tests::deeply_nested_if_survives_full_optimiser_pipeline`
  — end-to-end (source → lowering → all 5 optimiser passes) survival,
  spawned on its own 64 MiB thread since `lowering`'s cap barriers the
  input before the optimiser-level caps can be isolated by a source-text
  test alone. The same pattern is reused by several of the sweep's own
  `tcl-compiler` Tier 2 tests for the same reason.
- `tcl_compiler::codegen::structured::tests::deeply_nested_if_survives_structured_walk`
  / `very_long_elseif_chain_survives_structured_walk` — nested-body depth
  and `elseif`-chain length, as two *separate* recursion shapes.
- `tcl_vm`'s `tests/cmd_oo_e2e.rs::deeply_nested_method_errors_instead_of_crashing`
  / `moderately_recursive_method_still_runs` — the TclOO dispatch fix,
  both sides, through the real compiled VM rather than a unit-level call.
- `tcl_vm`'s `tests/cmd_control_e2e.rs`:
  `deeply_nested_dynamic_if_errors_instead_of_crashing` /
  `shallow_dynamic_if_still_runs` — the `cmd_control.rs` fallback cap, both
  sides, driven through the same computed-command-name technique the rest
  of that file already uses to reach the runtime fallback.
  `deeply_nested_command_substitution_is_unaffected` — proves the cap's
  narrow scope: ordinary nested `[…]` command substitution (which also
  routes through `Vm::eval_source`, but not through `cmd_control.rs`) is
  unaffected.
- `rust/tcl-lexer/tests/lexer_depth.rs` — every `tcl-lexer` fix from the
  sweep, including the two structural-index scanners reachable directly
  on raw document text.
- `rust/tcl-lsp-server/tests/e2e/issue996_stack_overflow.rs` — drives the
  real, packaged native server (not a library function directly) with
  pathological input at the exact reported crash depth and well past the
  analyser's cap, and proves the *same server process* answers unrelated
  follow-up work afterwards. Also covers the LSP-reachable surface of
  three of the other original-pass fixes against that same real server:
  `formatting_survives_deep_nesting`, `minify_survives_deep_nesting`, and
  `irules_semantic_tokens_survive_deep_command_substitution`.
- `editors/vscode/src/test/issue996.test.ts` — the same three
  LSP-reachable cases (diagnostics, formatting, `tcl-lsp.minifyDocument`)
  driven through a real VS Code extension host session against the
  packaged release server binary, against a committed fixture
  (`testFixture/issue996DeepNesting.tcl`, 300 levels).
- `rust/tcl-debugger/src/backend.rs`'s
  `launch_survives_deeply_nested_control_flow` — the debugger's `launch`
  path (both CLI and, transitively, DAP) survives deep nesting.
- `rust/f5-cli/tests/irule.rs`'s
  `minify_aggressive_survives_deeply_nested_irule` — drives the real,
  packaged `f5-query` binary (not a library call) with a deeply nested
  `.irule` file through `irule minify --aggressive`.

## Scope note

An initial audit for this issue found several recursive-descent locations
with no depth cap at all beyond the analyser itself, and continued
empirical testing while fixing those turned up two more of the same bug
class (`codegen::structured`'s `elseif`-chain recursion and
`tcl_vm::cmd_control.rs`'s runtime fallback, the latter overflowing the
stack at depth 50-60 — far below any of the other walkers' thresholds at
the time). That pattern — fixing one instance's own test coverage keeps
surfacing another instance — was the signal that reactive, crash-driven
discovery was not going to find every instance of this bug class on its
own. It found the loud ones (the ones some existing test path already
exercised deeply enough to crash) but had no way to find the quiet ones
(correct today only because nothing has yet nested input deep enough
through that particular path to prove otherwise).

The follow-up sweep addressed this directly: five parallel research
agents, each assigned a disjoint slice of the workspace (`tcl-compiler`;
`tcl-lsp-core`; `tcl-syntax`/`tcl-lexer` verification; `tcl-vm` ancillary
commands + `runtime/rust`; every remaining crate plus every binary
entry-point's stack-size posture), searched for the *pattern* — native
recursive descent with no depth parameter and no cap check — rather than
waiting for a crash to point at a specific function. This is what found
the ~55-function `tcl-compiler` `ExprNode`/bracket-text gap (a
structurally different recursion axis from anything the original pass had
tested against), `tcl-vm`'s TclOO dispatch crash (at depth 45-48 — the
single most severe finding across both passes, and invisible to the
original pass's testing because none of it happened to write a deeply
*recursive TclOO method*, as opposed to deeply nested control flow), and
the `tcl-regex`/`tcl-bigip-io`/`tcl-bigip-query`/`runtime/rust` findings
in crates the original pass never looked at because nothing had crashed
there yet.

All findings from both passes are now fixed — see the two tables above;
none were judged acceptable to leave unguarded. Everything fixed by the
sweep also migrated to the shared `RecursionLimit`/`RecursionGuard`
mechanism described above, and every pre-existing ad-hoc `MAX_X_DEPTH`
constant from the original pass (and from before issue #996 entirely, in
crates the sweep touched) was migrated to the same type for consistency —
searching the workspace for `RecursionLimit` now finds the complete set.

Four things worth remembering if this area changes again:

- **A reactive fix is not a complete fix for this bug class.** If a
  future change adds a new recursive-descent walker anywhere in this
  workspace, it needs its own `RecursionLimit` from the start — do not
  wait for a crash report to justify adding one. The sweep's own
  discipline (search for the *pattern*, not for a specific known-bad
  input) is the right model to repeat, not "fix it when someone hits it."
- **One cap does not necessarily bound every recursion axis a walker
  has.** `codegen::structured`'s `elseif` chains, `tcl-compiler`'s
  `ExprNode`/bracket-text recursions, and `tcl-vm`'s TclOO dispatch (which
  looked "safe" because `RECURSION_LIMIT` existed, just for the wrong
  code path) are three different flavours of this same mistake. When
  adding a cap, ask explicitly which axis of the input it bounds, and
  whether the same function has another axis that isn't the same one.
- The optimiser passes' cap (`MAX_OPTIMISER_WALK_DEPTH`) and the sweep's
  Tier 2 `tcl-compiler` caps are defence in depth, not the primary
  mitigation for the normal pipeline — `lowering`'s cap upstream already
  bounds the IR these passes see to 256 levels before they run. If a
  future change ever lets one of these passes run on IR built by a path
  other than `lowering`, re-verify the cap actually matters for that
  path.
- **`Vm::eval_source` itself must never gain a uniform depth cap**, and
  the same now applies to `runtime/rust`'s `Value`/`TclObj` Drop
  recursion and `cmd_oo.rs::self_reachable`'s complexity cost noted
  above — these are real, adjacent risks that were deliberately left
  unfixed here because they are a *different* problem (or, for
  `eval_source`, because a uniform cap on it specifically was tried once
  already and broke ordinary iRule execution in CI before being caught
  and re-scoped to `cmd_control.rs::eval_body`). Fixing them needs its
  own investigation, not a drive-by cap borrowed from this one.
