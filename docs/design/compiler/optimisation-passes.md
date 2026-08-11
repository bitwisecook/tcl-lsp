# KCS: Optimisation passes (O100–O130)

## Symptom

A contributor needs to understand which optimisation pass produces a specific
diagnostic code, how passes are orchestrated, or needs to add a new
optimisation.

## Context

The optimiser runs as Phase 7 of the pipeline, after CFG/SSA/SCCP analysis.
It is managed by `_CompilerOptimiser` in `_manager.py`, which orchestrates
individual passes and collects `Optimisation` objects.  Each pass produces
diagnostics with codes O100–O130.  Passes share state through `PassContext`.

Source: `compiler/optimiser/` —
`_manager.py`,
`_propagation.py`,
`_elimination.py`,
`_expr_simplify.py`,
`_code_sinking.py`,
`_tail_call.py`,
`_unused_procs.py`,
`compiler/gvn.py`

**Note:** the source-file column above and below predates the Rust
rewrite — Python has been fully retired (see `AGENTS.md`). The
equivalent Rust modules live under `rust/tcl-compiler/src/optimiser/`
(`manager.rs`, `propagation.rs`, `elimination.rs`, `expr_simplify.rs`,
`code_sinking.rs`, `tail_call.rs`, `unused_procs.rs`, and
`rust/tcl-compiler/src/gvn.rs`). The `O102` row has been verified and
corrected against the current Rust behaviour; the rest of this table
has not been re-audited row-by-row and may still describe
Python-era-only behaviour.

## Content

### Complete pass table

| Code | Name | Source file | Trigger |
|------|------|-------------|---------|
| O100 | Constant propagation | `_propagation.py` | Variable has known constant value |
| O101 | Fold constant expression | `_propagation.py` | All `expr` operands are constants |
| O102 | Forward single reaching literal load | `propagation.rs` (`run_load_forwarding`) | Variable has exactly one reaching literal definition, not trace-tainted or frame-aliased |
| O103 | ICIP (interprocedural constant fold) | `_propagation.py` | Pure proc called with all-constant args |
| O104 | String build chain | `_propagation.py` | `set` + `append` sequence detected |
| O105 | GVN/CSE redundancy | `gvn.rs` | Same computation appears twice with equal value, effect, result-stability, and live-dispatch proofs |
| O106 | Loop-invariant computation (LICM) | `gvn.rs` | Proven invariant computation inside a loop |
| O107 | Dead code elimination (DCE) | `_elimination.py` | Unreachable code after `return`/`break` |
| O108 | Aggressive DCE (ADCE) | `_elimination.py` | Statement result never used, no side effects |
| O109 | Dead store elimination (DSE) | `_elimination.py` | Variable set but never read |
| O110 | InstCombine | `_expr_simplify.py` | Algebraic simplification opportunity |
| O111 | Brace expression hints | `_propagation.py` | Unbraced expr body in expression context (readability, paired with W100) |
| O112 | Constant condition (SCCP) | `_propagation.py` | Branch condition is compile-time constant |
| O113 | Strength reduction | `_propagation.py` | Power/modulo with small constants |
| O114 | Incr idiom | `_propagation.py` | `set x [expr {$x + N}]` pattern |
| O115 | Nested expr unwrap | `_propagation.py` | `expr {expr {…}}` double wrapping |
| O116 | List folding | `_propagation.py` | `[list a b c]` with all-constant args |
| O117 | String length zero-check | `_propagation.py` | `[string length $s] == 0` |
| O118 | Lindex folding | `_propagation.py` | `[lindex {a b c} N]` with constant list and index |
| O119 | Multi-set packing | `_propagation.py` | Consecutive `set` commands with related values |
| O120 | String compare eq/ne | `_propagation.py` | `==`/`!=` on string-typed operands |
| O121 | Tail-call detection | `_tail_call.py` | Self-recursive call in tail position |
| O122 | Tail-recursion to loop | `_tail_call.py` | Fully tail-recursive proc |
| O123 | Accumulator introduction | `_tail_call.py` | Non-tail recursion with associative op |
| O124 | Unused proc elimination | `_unused_procs.py` | Proc defined but never called (iRules only) |
| O125 | Code sinking (LCP) | `_code_sinking.py` | Assignment used only in one branch |
| O126 | Dead store after tail position | `_elimination.py` | Variable only used by eliminated tail expr |
| O127 | Inline single-use assignment | `_propagation.py` | Non-constant variable set once and used once |
| O128 | End-offset index rewrite | `_pattern_recognition.py` | `[expr {[llength/[string length] - N}]` → `end-(N-1)` as index arg |
| O129 | Fold pure builtin command substitution | `_propagation.py` | Pure builtin command substitution with constant arguments |
| O130 | Lappend build chain | `_pattern_recognition.py` | `set` + `lappend` sequence folded into single assignment |

### Priority ordering

`_OPT_PRIORITY` in `_types.py` assigns each code a priority (higher = more
impactful).  When multiple optimisations overlap the same range, the highest
priority wins.

### Grouped optimisations

Related edits share a `group` ID (allocated via `PassContext.alloc_group()`).
For example, O100 (propagate constant) + O109 (remove dead store) share a
group, producing one primary diagnostic with the dead store as related info.

### Worked examples

**ICIP (O103)**: `proc double {n} { expr {$n * 2} }` + `[double 21]`
→ evaluate body with `n=21` → result `42` → replace call with constant.

**Code sinking (O125)**: `set msg "error"` before an `if` where `msg` is only
used in the else branch → move `set msg` into the else branch.

**GVN/CSE (O105)**: a repeated scalar expression with the same SSA operands
and no intervening state change can reuse its dominating result. A repeated Tcl
command call is only a static candidate when the resolved registry form is
pure, CSE-enabled, referentially transparent, transition-closed, and free of
direct effects. It is not eligible until world analysis also proves every
registry-declared live-dispatch dependency, including the absence of relevant
command and execution traces. The current world SSA does not yet provide that
content proof, so production call CSE abstains.

**Load forwarding (O127)**: `set x [HTTP::uri]; log local0. $x`
→ `log local0. [set x [HTTP::uri]]` — eliminates the redundant variable load
by inlining the assignment at the single use site.

**DCE (O107)**: Code after `return` is unreachable → remove.

**Tail-call (O121/O122)**: `proc fact {n acc} { if {$n <= 1} { return $acc }; fact [expr {$n-1}] [expr {$acc*$n}] }` → rewrite as `while` loop.

## Decision rule

- To add a new optimisation: create the detection logic in the appropriate
  pass file, assign a new O-code, add its priority to `_OPT_PRIORITY`, and
  emit `Optimisation` objects through `PassContext`.
- Purity is necessary but not sufficient for CSE or code motion. Result
  stability, completion, transitions, mutable-world reads, callbacks, command
  identity, interpreter policy, and live traces must also be covered by the
  relevant proof. Missing facts cause the pass to abstain.
- Always group related edits with `PassContext.alloc_group()` to ensure
  atomic code action application.

## Related docs

- [Examples 13–19 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-13-icip--interprocedural-constant-propagation-o103)
- [Optimisation table in walkthroughs](../../../docs/design/example-script-walkthroughs.md#optimisation-opportunities-across-examples)
- [GLOSSARY.md](../../GLOSSARY.md)
- [kcs-o125-code-sinking.md](../../../docs/design/compiler/o125-code-sinking.md)
- [kcs-tail-call-recursion-optimisation.md](../../../docs/design/compiler/tail-call-recursion-optimisation.md)
- [kcs-pass-authoring-checklist.md](../../../docs/kcs/kcs-howto-add-compiler-pass.md)
