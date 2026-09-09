# Recursive-descent depth limits

Every stage of the pipeline that walks nested Tcl bodies
(`if`/`while`/`foreach`/`switch`/`try`/`catch`/`dict for`, `apply` lambdas,
`namespace eval`, …) does it by recursive descent: one Rust stack-frame group
per source-nesting level. Native call-stack depth therefore scales with how
deeply the input nests, and — unlike a Tcl-level `proc`-call stack, bounded by
`interp recursionlimit` as a catchable error — a native stack overflow is an
uncatchable process abort (`SIGABRT`). For any consumer that analyses
untrusted, generated, or minified Tcl that is a denial of service (issue
#996). This doc records the model every walker follows, the shared mechanism,
and the inventory of guarded walkers.

## Two distinct problems

**A cap without a stack budget.** A depth cap only contains what its own
frames fit in, and the ambient stack is not a portable quantity: Tokio worker
threads and `cargo test` threads get ~2 MiB, the CLI main thread gets
whatever `ulimit -s` gives, and a WASM host's stack is outside this repo's
control. Raising the number is not a fix — a bigger cap needs proportionally
more stack, and the per-level frame cost silently grows whenever a hot
function in the chain gains a local.

**Walkers with no cap at all.** A bigger stack does not make an uncapped
walker safe. Shapes that defeat an *existing* guard rather than merely lack
one:

- a second, unguarded recursion running *before* an existing guard;
- a guard that can be bypassed by resetting its own counter (an `apply`
  body re-entering the public depth-0 entry point);
- a recursion axis the existing cap does not count: an `elseif`-chain length
  (independent of body nesting), the `ExprNode` tree inside a *single*
  `expr {…}`, raw nested `[cmd [cmd …]]` text inside one word, a regex
  engine's execution-time backtracking against a long subject (independent
  of the pattern's parsed depth);
- an interpreter `RECURSION_LIMIT` that bounds proc-call nesting but not the
  general eval recursion a tree-walker also pays a native frame for, or that
  a dispatch path (TclOO method dispatch in `tcl-vm`) bypasses entirely;
- drop glue: a deeply nested `Value` with no custom `Drop` recurses once per
  level when it goes out of scope (`tcl-vm`, `tcl-bigip-query`) — a related
  risk that is deliberately out of scope here.

## Unified mechanism: `RecursionLimit` / `RecursionGuard`

`tcl_core_types::{RecursionLimit, RecursionGuard}`
(`rust/tcl-core-types/src/recursion.rs`) is the dependency-free, `no_std`,
`unsafe`-free primitive every guarded walker builds on. It centralises the
bookkeeping — compare, increment, decrement — not what happens when the limit
trips, which stays domain-specific.

```rust
pub struct RecursionLimit(pub u32);
impl RecursionLimit {
    /// `depth > self.0` — the nesting level about to run, *including* it.
    pub const fn exceeded(self, depth: u32) -> bool { depth > self.0 }
}
```

Two call-site shapes cover every walker:

1. **An explicit `depth: u32` parameter** threaded through recursive calls —
   the common case. Guard at the top of the function with
   `if MAX_X_DEPTH.exceeded(depth) { <domain-specific fallback>; }` and pass
   `depth + 1` to each recursive call.
2. **A counter on a long-lived struct.** When the recursive call does not
   re-borrow the struct mutably through other calls, `RecursionGuard` wraps
   the counter in RAII: `enter()` checks and increments, and its `Drop`
   decrements again on every exit path (`?`, early `return`, panic unwind).
   When the call chain runs back out through the engine before re-entering
   (`tcl-vm`'s `Vm::control_fallback_depth` / `oo_dispatch_depth`), the
   borrow cannot span the gap, so those two use a manual `enter_*`/`exit_*`
   pair with the same `LIMIT.exceeded(counter + 1)` check and
   `counter.saturating_sub(1)` on the way out. That is the one sanctioned
   exception.

Searching the workspace for `RecursionLimit` finds every guarded walker.

## The model

1. **A generous, explicit stack budget at every process entry point** this
   repo controls, instead of the ambient thread's stack. `tcl-lsp-server` and
   `tcl-mcp` build their own Tokio runtime with a 64 MiB
   `thread_stack_size`; the `tcl` CLI, `f5-query`, `bpf-tcl`, and
   `bigip-report-gen` run their dispatch on a dedicated 64 MiB thread; the
   debugger's `VmBackend::record` (behind both `launch` paths) does the same.
   64 MiB is deliberately generous — the measured need is a few MiB even in a
   debug build — so it also covers future frame growth and several guarded
   walkers on one call stack.

2. **A conservative, small cap for anything reachable from a WASM host**
   (`tcl_runtime`, `tcl_lsp_core`'s formatter and minifier, `tcl_irules`'s
   walker, `tcl_vm`'s runtime control-flow fallback and TclOO dispatch,
   `tcl-lexer`, `tcl-regex`, `tcl-bigip-io`, `tcl-bigip-query`), because the
   host's stack is not ours to size. Each is calibrated against a 2 MiB
   thread and set well under the measured crash floor; the values differ
   because the per-level frame cost differs by walker.

3. **Every independently recursive walker needs its own cap, and every
   distinct recursion axis its own separate cap**, whichever budget applies.
   A cap on body nesting does not bound an `elseif` chain, an `ExprNode`
   tree, or bracket-nested text inside one word.

4. **A tripped cap is a diagnostic or an explicit conservative fallback,
   never silent truncation or a miscompile.** The analyser emits
   [`E207`](../../kcs/codes/kcs-diagnostic-e207-nesting-depth-exceeds-limit.md)
   once per run, anchored on the body where descent stopped; lowering emits a
   `Statement::Barrier` (unknown effect, not dead code); `codegen::structured`
   degrades to its whole-construct eval fallback; the formatter and minifier
   leave the over-deep body untouched; `tcl_irules::walker` keeps the
   references found so far; `tcl_runtime` and `tcl_vm` raise tclsh's own
   catchable `too many nested evaluations (infinite loop?)`. A boolean
   "could this have an effect / could this alias" helper answers
   conservatively (`true`) past the cap; a collector returns what it has; a
   tree rewriter passes the over-deep subtree through unchanged; the lexer
   and runtime word scanners fall through to scanning the nested construct as
   ordinary characters.

5. **A cap over a source-nesting walk is arithmetic against a stack budget,
   not a convention number** (issue #1654). `tcl_compiler::depth_guard`
   states the three inputs — `MIN_SOURCE_WALK_STACK` (2 MiB, the platform
   default thread), `SOURCE_WALK_STACK_RESERVE` (a quarter of it, for
   everything on the stack that is not the descent), and
   `SOURCE_WALK_BYTES_PER_LEVEL` (24 KiB, the worst measured per-level cost
   of the three braced-body walks, rounded up) — and derives
   `MAX_SOURCE_NEST_DEPTH` from them. All three braced-body caps read it, so
   they stay in lockstep and the number can be re-derived from re-measured
   inputs. `depth_guard::tests::the_source_walk_cap_fits_its_stack_budget`
   analyses a document nested past the cap on a thread sized to
   `MIN_SOURCE_WALK_STACK - SOURCE_WALK_STACK_RESERVE`, so frame growth in any
   of the three walks fails a test instead of resurfacing as an abort.

## Guarded walkers

### `tcl-compiler`

| Walker | Cap | Trip behaviour |
|---|---|---|
| `analyser::commands::analyse_body` | `MAX_BODY_DEPTH` = `depth_guard::MAX_SOURCE_NEST_DEPTH` | `E207` once per run |
| `cfg_builder::CfgBuilder::lower_script` | `MAX_LOWER_DEPTH` = `MAX_SOURCE_NEST_DEPTH` | stops descending; truncated but valid CFG |
| `lowering::Lowerer::lower_script` / `lower_body` | `MAX_LOWER_NEST_DEPTH` = `MAX_SOURCE_NEST_DEPTH` | `Statement::Barrier`; the fattest of the three walks, so it sets the budget |
| `analyser::param_traits::scan_deep` / `infer_param_traits_deep_at_depth` | `MAX_DEPTH` = 8 | threads its own depth through `apply` bodies rather than re-entering at 0 |
| `analyser::scope` walkers | `MAX_SCOPE_WALK_DEPTH` = 256 | defence in depth |
| `optimiser::{propagation, expr_simplify, pattern_recognition, structure_elimination, code_sinking, tail_call, end_offset, chain_fold}` | `MAX_OPTIMISER_WALK_DEPTH` = 256 (`optimiser/mod.rs`) | defence in depth: lowering's cap already bounds the IR these passes see; `code_sinking`'s query families answer conservatively past the cap |
| `codegen::structured::{walk_stmt, emit_if, emit_loop}` | `MAX_STRUCTURED_DEPTH` = 256 | covers nested-body depth *and* `emit_if`'s `elseif`-chain self-recursion |
| every `ExprNode` operator-tree walker (`type_infer`, `codegen/expressions`, `taint`, `interprocedural`, `optimiser/helpers/expr_simplify`, `optimiser/elimination`, `optimiser/code_sinking`, `uri_split`, `shimmer/{expr,commit}`, `ir_helpers`, `intervals`, `interval_bounds`, `inlining/rename`, `analyser/diagnostics/{usage,dataflow}`, `connection_scope`) | `depth_guard::MAX_EXPR_NODE_DEPTH` = 256 | a single `expr {((((…))))}` or a long `1+1+…` chain controls this axis directly, independent of statement nesting |
| every raw nested `[cmd …]` text walker (`taint`, `optimiser/elimination`, `optimiser/end_offset`, `interprocedural`, `analyser/commands`) | `depth_guard::MAX_BRACKET_TEXT_DEPTH` = 256 | same reasoning: bracket nesting inside one word is its own axis |
| the remaining Script/Statement-tree walkers (`ir`, `ir_helpers`, `ssa`, `interprocedural`, `lowering`, `command_binding`, `var_escape/{walker,slot_resolution}`, `analyser/oo`, `cfg_builder`, `inlining`) | a sibling constant in the same file, or a small local one | defence in depth behind `MAX_LOWER_NEST_DEPTH` |

`ssa.rs`'s dominator-tree rename walk and `sccp.rs` are explicit worklists
and need no cap.

### `tcl-lexer`, `tcl-syntax`, `tcl-explorer`

These run directly on raw, untrusted document text, before any higher-level
guard.

| Walker | Cap | Notes |
|---|---|---|
| `word_parts` nested array index (`$a($b($c(…)))`) | `MAX_INDEX_DEPTH` = 64 | past the cap a nested `$` is scanned as an ordinary character |
| `structural_index`'s scanners (`BracketIndex::scan_cmd_sub`; `BraceIndex`'s `scan_script`/`scan_quoted`; `scan_complete`/`scan_complete_quoted` behind `script_is_complete`) | `MAX_NESTED_BRACKET_DEPTH` = 128 | `script_is_complete` is called on raw text from `tcl-lsp-server::compute_base_analysis`; `command_boundaries` needs no cap because its `[…]` skip (`ranges::command_substitution_end`) is iterative |
| `tcl-syntax` Pratt expression parser | `MAX_EXPR_DEPTH` = 256 | |
| `tcl-syntax::mro` TclOO linearisation | `MAX_MRO_DEPTH` = 1024, `MAX_MRO_VISITS` = 200,000 | depth *and* a total-visit budget: a mixin/superclass graph can be wide as well as deep |
| `tcl-explorer::cst` | `MAX_DEPTH` = 256 | |

### `tcl-lsp-core`

`crate::MAX_SCOPE_WALK_DEPTH` = 256 (`lib.rs`) guards every Scope-tree
walker — `document_symbols`, `definition` (`innermost`, `collect_alias_spans`,
`collect_shared_span_refs`, `cross_namespace_qualified_vars`), `graphs`
(`scope_to_value`, `count_namespaces`, `count_variables`), `inlay_hints`,
`refactor::inline_variable::walk_scopes_at_depth`. A `Scope` node only exists
for a `namespace`/`proc`/`method` body the analyser already caps, so this is
defence in depth against a scope tree built some other way.

| Walker | Cap | Notes |
|---|---|---|
| `references` (`scan_my_method_region`, `scan_obj_method_region`, `scan_next_dispatch_region`) | `MAX_DISPATCH_SCAN_DEPTH` = 256 | |
| `folding` | `MAX_FOLD_DEPTH` = 256 | |
| `declaration` | `MAX_BODY_DEPTH` = 256 (local) | not tied to the compiler's derived cap; defence in depth |
| `refactor` | `MAX_COMMAND_SEARCH_DEPTH` = 256 | |
| `semantic_tokens` (`collect_lambda_literal` family) | `MAX_TOKEN_RECURSION` = 32 | |
| `formatting::engine::format_body` / `format_switch_body` | `MAX_FORMAT_DEPTH` = 128 | **conservative cap** — reachable from the `bigip-query-wasm` host; 2 MiB crash floor measured at depth 800–1200 |
| `minify::minify_body` and siblings | `MAX_MINIFY_DEPTH` = 128 | same reasoning |
| `package_resolver::collect_source_targets` | `MAX_SOURCE_TARGET_SCAN_DEPTH` = 256 | nested wrapper words in a `pkgIndex.tcl` `package ifneeded` body |

### Other crates

| Walker | Cap | Notes |
|---|---|---|
| `tcl_irules::walker::{walk, recurse_token}` | `MAX_WALK_DEPTH` = 128 | **conservative cap** (WASM host: `tcl-bigip`) |
| `tcl-regex` `Matcher::dissect`/`dissect_seq`/`dissect_repeat` (POSIX leftmost-longest dissection after every repeat match) | `MAX_DISSECT_DEPTH` = 256 | execution-time recursion whose depth scales with the *subject* length, not the pattern; distinct from `MAX_PARSE_DEPTH` (1000), which bounds the parsed pattern |
| `tcl-regex` `Bt::m`/`m_seq`/`m_repeat`/`m_star`/`m_backref` (backtracking path) | `MAX_BT_DEPTH` = 256 | |
| `tcl-bigip-io::openpgp::extract_literal` | `MAX_COMPRESSED_PACKET_DEPTH` = 16 | nested Compressed Data packets in an encrypted `.ucs`/`.scf` — attacker-controlled; a legitimate archive nests one level |
| `tcl-bigip-query`'s `Value`-tree walkers (`special::walk`, `builtins/mod.rs`'s `to_jsonable`/`walk_paths`/`set_at_path`/`delete_at_path`/`flatten_go`, `value::py_eq`, `edit_plan::format_value`, `builtins/encoding::json_to_value`) | `value::MAX_VALUE_WALK_DEPTH` = 64 | mirrors the crate's `parser::MAX_PARSE_DEPTH` (64) |

### `tcl-vm`

| Walker | Cap | Notes |
|---|---|---|
| `cmd_control::eval_body` (runtime control-flow fallback, reached through a computed command name) | `CONTROL_FALLBACK_DEPTH_LIMIT` = 24 | **conservative cap** (WASM host: `tcl-vm-wasm`); ordinary proc-to-proc calls are trampolined and never touch the native stack. Deliberately *not* a cap on `Vm::eval_source` — see [Rules](#rules) |
| `cmd_oo::run_step` (TclOO dispatch — `$obj method` / `my method` / `next` / `nextto`) | `OO_DISPATCH_DEPTH_LIMIT` = 20 | method dispatch bypasses the proc-call trampoline, so every nested method call is a native `run_activation` call; guarded at `run_step`, which `my` and `next`/`nextto` reach directly. A deliberate compatibility gap against tclsh (a recursive method errors at depth 20, not 1000); the architectural fix is routing method dispatch through the trampoline |
| `value::Value::to_str` (nested list/dict stringification) | `MAX_LIST_TO_STR_DEPTH` = 256 | reachable via a plain loop, no `{*}` needed |
| `cmd_dict::set_path`/`unset_path`, `cmd_list::lpop_remove`, `exec::lset_descend` | none — iterative | explicit work-stack, mirroring `get_path` |

### `runtime/rust`

| Walker | Cap | Notes |
|---|---|---|
| `interp::eval_script_mode` | `NATIVE_EVAL_DEPTH_LIMIT` = 128 | **conservative cap**, reachable via ordinary recursive `proc` calls: `RECURSION_LIMIT` (1000, matching tclsh) overflows a 2 MiB thread long before 1000 |
| `cmd_oo::linearize_class` / `gather_class_props` | `MAX_MRO_DEPTH` = 64, `MAX_MRO_VISITS` = 200,000 | the `tcl_syntax::mro` two-cap approach |
| `subst::resolve_parts` (nested `$name(index)`) | `MAX_RESOLVE_PARTS_DEPTH` = 64 | falls through to an ordinary character past the cap |
| `parse` error scan | `MAX_PARSE_ERROR_SCAN_DEPTH` = 128 | |
| `cmd_dict::dict_path_set`/`dict_path_unset` | none — iterative | |

Known adjacent risks, deliberately not addressed by a cap: `cmd_oo::self_reachable`'s
O(n²)-ish cost in `superclass`-only MRO construction (an algorithmic-complexity
problem, not stack depth) and `Value`/`TclObj` drop recursion.

## Testing

Every guarded walker has at least one regression test proving deep or
adversarial input survives (well past the crash floor or the cap) and one
proving moderate-depth input is unaffected, under one doc-comment convention:

```rust
/// Regression coverage for issue #996: `<function>` recurses once per
/// <nesting unit>, with no depth cap before this fix. Empirically,
/// unguarded input overflowed the native stack (SIGABRT) around depth
/// <D> on a 2 MiB thread (`cargo test`'s per-test default). <N> is
/// comfortably past both that crash range and `MAX_X_DEPTH` (<cap>); the
/// assertion is that <call> returns at all, not what it returns.
```

A test whose *fixture* would itself overflow the harness's ~2 MiB thread
(building and dropping a deeply nested `Scope`/`Value`) spawns its own
64 MiB thread (`std::thread::Builder::new().stack_size(64 * 1024 * 1024)`);
follow the sibling in the same file that already does this.

Standout tests:

- `tcl_compiler::analyser::commands::tests` — TP/FP/TN coverage for `E207`
  at the exact `MAX_BODY_DEPTH` boundary.
- `tcl_compiler::depth_guard::tests::the_source_walk_cap_fits_its_stack_budget`
  — the derived cap fits its stated budget.
- `tcl_compiler::optimiser::manager::tests::deeply_nested_if_survives_full_optimiser_pipeline`
  — source → lowering → every optimiser pass, on its own 64 MiB thread.
- `tcl_compiler::codegen::structured::tests::deeply_nested_if_survives_structured_walk`
  / `very_long_elseif_chain_survives_structured_walk` — nested-body depth and
  `elseif`-chain length as two separate axes.
- `tcl_vm`'s `tests/cmd_oo_e2e.rs::deeply_recursive_method_errors_instead_of_crashing`
  / `moderately_recursive_method_still_runs`.
- `tcl_vm`'s `tests/cmd_control_e2e.rs`:
  `deeply_nested_dynamic_if_errors_instead_of_crashing` /
  `shallow_dynamic_if_still_runs`, and
  `deeply_nested_command_substitution_is_unaffected` (ordinary nested `[…]`
  also routes through `Vm::eval_source` but not through `cmd_control.rs`).
- `rust/tcl-lexer/tests/lexer_depth.rs` — every `tcl-lexer` cap, including
  the structural-index scanners reachable on raw document text.
- `rust/tcl-lsp-server/tests/e2e/issue996_stack_overflow.rs` — drives the
  packaged native server with input at the reported crash depth and past the
  analyser's cap, and proves the same process answers follow-up work; also
  `formatting_survives_deep_nesting`, `minify_survives_deep_nesting`, and
  `irules_semantic_tokens_survive_deep_command_substitution`.
- `editors/vscode/src/test/issue996.test.ts` — the same three cases through a
  real extension host against the packaged server, on the committed fixture
  `testFixture/issue996DeepNesting.tcl` (300 levels).
- `rust/tcl-debugger/src/backend.rs`'s `launch_survives_deeply_nested_control_flow`.
- `rust/f5-cli/tests/irule.rs`'s `minify_aggressive_survives_deeply_nested_irule`
  — the packaged `f5-query` binary on a deeply nested `.irule`.

## Rules

- **A new recursive-descent walker gets its own `RecursionLimit` from the
  start.** Search for the pattern — native recursive descent with no depth
  parameter and no cap check — rather than waiting for a crash to point at a
  function.
- **One cap does not bound every recursion axis a walker has.** When adding
  a cap, say which axis of the input it bounds, and check whether the same
  function has another.
- The optimiser caps and the compiler's Script/Statement-tree caps are
  defence in depth behind lowering's cap. If a pass ever runs on IR built by
  a path other than `lowering`, re-verify the cap matters for that path.
- **`Vm::eval_source` must never gain a uniform depth cap**: a uniform cap
  on it breaks ordinary iRule execution, which is why the cap sits on
  `cmd_control.rs::eval_body`. The adjacent drop-recursion and
  `self_reachable` complexity risks need their own investigation, not a
  drive-by cap borrowed from this one.
