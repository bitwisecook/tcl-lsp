# Constants, constant folding and procedure folding

How the compiler represents constant values, which engines fold them, how the optimiser rewrites source with them, and how far procedure calls are folded. Part of [tidying the architecture](architecture-tidy-plan.md); faults 3, 4, 6 and 10 of the [cross-cutting review](cross-cutting-review.md).

**Verdict:** the optimiser's constant rewrites are **safe**: across 118 fixtures, each executed under tclsh 8.6.18 and 9.0.4 before and after the rewrites, no output changed (236 pairs, [`constants/`](handover/oo-dispatch-probe/constants/)). They decline when the release, a math-function override, a trace, a redefinition or a caller link could change the answer, and they fold through `rename`, redefinition and release-specific octal correctly. The weaknesses are elsewhere: **at least nine folders and ten constant value types**, with three literal parsers, two `format` implementations and several string commands written again beside `tcl-cmd-core`; float spelling and missing-release defaults decided in several places; a no-dialect API that folds nothing; and procedure folding that stops at zero-argument, single-file, non-recursive cases.

## Correction to earlier studies

[Expressions](expression-design.md) and [lists, dicts and arrays](lists-dicts-arrays-design.md) reported that the optimiser folds none of 61 expressions and none of 12 list and dict operations. That was an artefact of the probe: it called `optimise_with_dialect(source, registry, None)`. With `None` the unit is built for Tcl 9.0 but the passes are told no release is selected, so every fold that consults the release declines. With an 8.6 or 9.0 profile the same fixtures give:

| Fixtures | 8.6 folded | 9.0 folded | Wrong rewrites |
| --- | --- | --- | --- |
| 61 expressions (`set r [expr {…}]; puts $r`) | 28 | 34 | 0 |
| 12 list and dict operations | 11 | 11 | 0 |

Those two pages are corrected. The `None` behaviour is itself a trap: `optimise()` is documented as `optimise_with_dialect(…, None)` and silently folds nothing, even where every release agrees (`1 + 2`).

## Measured behaviour

Fixtures `c01`–`c20`, `d01`–`d20` and `e01`–`e08` in [`constants/folds/`](handover/oo-dispatch-probe/constants/folds/), rewrites in `optimiser.out`, executions in `execution.tsv`.

**Folds, all correct:**

- Constant propagation through `set` chains in procedures and at top level, constant `expr`, constant branches with the dead arm removed (`c01`, `c05`, `d01`).
- Literal string, list and dict commands (`string toupper`, `string repeat`, `string map`, `lindex`, `llength`, `lsort`, `dict get`), and `[list …]` turned into a braced literal (`d03`, `d04`, `c16`).
- Bignum and wide-overflow results (`2**64`, `wide(9223372036854775807) + 1`).
- Release-specific octal: `expr {010 + 0}` becomes `8` under the 8.6 profile and `10` under 9.0 (`c18`).
- Zero-argument procedures with a constant result (O103): `puts [p]` becomes `puts 11`; booleans keep their spelling (`true`, `false`); floats keep theirs (`1.0`, `0.5`).
- Through a mid-procedure `rename ::string` and a user `proc ::string`, the folds give `3` before and `99` after (`c12`), which is what tclsh prints.

**Declines that are correct:** division by zero inside `catch`, `clock seconds`, a read trace, a user `tcl::mathfunc::double`, a global changed after the definition, a procedure redefined mid-procedure, a procedure with side effects, callee `upvar`, `uplevel 0`, `info exists` in a branch (folded only where proven), and float results under 8.4–8.6, because `tcl_precision` is a mutable variable there (9.0 removed it; tclsh 8.6 prints `0.333` after `set tcl_precision 3`, 9.0 prints the full value).

**Missed folds** (every release agrees, nothing can intervene):

- Procedures with arguments (`double 21`, `add 2 3`), recursive procedures (`fact 5`), and a constant procedure inside an expression (`expr {[k] * 2}`).
- Loops (`for` summing 0..4), even though `static_loops` can run them; `append` and `lappend` chains in procedures, `incr` of a copy, `format`, string concatenation of two constants (left as an O102 hint).
- Bare literal spellings in `expr` (`0x1F`, `0b101`, `1_000`, `1e21`, `-0.0`), short-circuit operators with a constant left side (`0 && [error boom]`), and math functions. Math functions are declined because a program can define `tcl::mathfunc::*`; without proof that it does not, that is correct, and the fix is whole-program proof, not a weaker check.
- Float results under 8.5 and 8.6 when the program never writes `tcl_precision` (the same whole-program proof).
- Anything across files: interprocedural summaries are per compilation unit.

**Where folding runs:** the LSP's default optimiser profile is Readability, which turns constant folding off in editors; `tcl opt` and MCP `optimize` default to Full; `tcl compile --optimise` applies every pass.

## Constant values: ten types

| Type | Owner | Used by |
| --- | --- | --- |
| `LatticeValue`, `ConstValue` (Int, Float, Bool, String) | `analyses.rs` | SCCP, optimiser, minifier |
| `TclValue` (Int, Float, Big), `EnvValue`, `FoldValue` | `tcl_expr_eval` | expression folder |
| `FoldedExpressionContents` | `sccp/original_value_facts` | diagnostic SCCP |
| `StaticValue` | `static_loops` | loop summaries |
| `ConstantReturn` | `interprocedural` | procedure summaries |
| `ExactValue` (bytes, numeric reading, representation) | `tcl-registry` value transfer | value-transfer model |
| a second `ConstValue` | `tcl-registry/value_transfer/const_ops.rs` | `ConstOps` |
| `String` | `const_subst`, registry `const_fold` callbacks | command-substitution folds, codegen |

`ExactValue` is the right model: it keeps the original bytes and a numeric reading only where the bytes round-trip, which is what Tcl's dual representation needs (`01`, `+5`, `" 5"` and `1.0` keep their spelling). The others are projections of it, converted by hand at about eight bridges. Three of those bridges spell floats with the Tcl 9 shortest form whatever the release (`const_to_exact`, `exact_of_static`, `ConstantReturn::text`); the optimiser's renderers use the selected release instead. No wrong output was observed, because 8.x float folds decline earlier, but the decision is made in four places.

## Folding engines

1. **Expression folder** (`tcl_expr_eval`): the shared `tcl_syntax::expr` tree walk with the compiler's own arithmetic (division, modulus, power, shifts, bignums), not `tcl-cmd-core`'s `mathop`. Release policy through `FoldPolicy` (octal, numeral syntax, character model, float format).
2. **Command-substitution engine** (`const_subst`): a retained path for the optimiser (O116, O118, O129) and a text path for codegen and the analyser; nesting capped at 16.
3. **Value-transfer model** (`ConstOps` in `tcl-registry`): delegates to `tcl-cmd-core` for `string range`, `list`, `llength`, `string length`, `split`, `string first`, `string match`, `format` and `binary format`; declines when no release is named and releases differ.
4. **Registry `const_fold` callbacks:** list and dict commands, through `tcl_cmd_core::index` and the shared list splitter; string commands, of which `toupper`, `tolower`, `reverse` and `map` are written again (ASCII only, so they decline rather than err) beside `tcl_cmd_core::string`; `format` and `scan` with their own renderers beside `tcl_cmd_core::format`.
5. **SCCP**, twice: the main solver and the "original value facts" solver used for diagnostics, with `parse_literal_value`, `extract_foreach_elements` and `fold_assign_value` defined in both.
6. **Static loops**: run loops through registry routes, capped at 4096 iterations; its own literal parser (`original_simulation.rs`) trims, reads `010` as ten and `TRUE` as a boolean. Only tests call it today.
7. **Intervals**: canonical decimals only; with no dialect they assume the 9.0 grammar.
8. **Codegen**: `try_emit_constant_fold` through `const_subst`; `fold_integer_expression` through `tcl_expr_eval`; and two re-implementations, `fold_const_branch` (Rust `i64`/`f64` parsing with no release: `08` is true, `NaN` and `inf` are accepted) and `concat_cmd` (`str::trim`, which strips Unicode whitespace that Tcl's `concat` keeps). `if {08}` under 8.6 still raises in the VM, so `fold_const_branch` is not reached for that case; it is a latent copy, not a measured wrong result.
9. **Minifier**: substitutes SCCP integer and string constants, refuses booleans, floats and tainted values.

Also: the `auto_path` mini-evaluator, with known wrong answers recorded in the test-scrub handover.

**`format` has two implementations** chosen by entry point: O129 runs the registry's `fold_format`; codegen and the analyser's text path run `tcl_cmd_core::format` through `FORMAT_TEMPLATE`.

**Open wrong results** recorded in the test-scrub handover and not re-measured here: `string compare --` and `string equal --` folded where every interpreter raises; `-length 010`/`08` on 8.x; `string is double 08` on 8.5 and 8.6; Jim's `string is boolean t`.

## Procedure folding (O103)

**What exists:** a command substitution calling a user procedure folds when the call's execution target is proved, matches its authored declaration, is not redefined, has no caller-linked parameters, and the procedure is pure (a fixpoint over callees). The value comes from, in order: the binding's proved normal result, the summary's constant return, or re-running SCCP on the callee with constant arguments bound, carrying whole-module command trust so a shadowed builtin keeps its user meaning. A bare call statement is a hint only, since replacing `answer` with `42` makes an invalid command. Tests cover rename-over, namespace-relative rename, `interp alias`, and withdrawal when an unknown external entry could rebind the procedure. The Jim external-entry rebinding fix awaits re-verification after integration (coordinator handover).

**What it misses** (measured above): calls with arguments, recursion, calls inside `expr`, and anything across files. Re-running SCCP per call site does not scale to arguments; the general mechanism is a **specialised summary**: evaluate the callee's body unit once per distinct tuple of constant argument values (memoised as a query keyed by procedure identity and argument values), with fuel for recursion (`fact 5` takes five levels; a budget of iterations and depth stops `fib 40`), and widen to unknown when the budget is spent.

**Latent inconsistency:** `ConstantReturn::Bool` is rendered `1`/`0` by O103 but `true`/`false` by `ConstantReturn::text`. The binding's normal result is used first, so the measured folds spell `true`; the second path would not.

## What constants need from the rest of the plan

- **Identity** ([command lookup](command-lookup-centralisation.md), [rename and alias](rename-alias-design.md)): command folds are only valid for the proved target, never for a spelling. O103 already does this; the builtin folds do it through whole-module trust sets keyed by name, which become `CommandRef` checks on annotated commands.
- **Contracts** ([user and library commands](user-and-library-commands.md)): purity, constant results and "result as a function of arguments" are contract facts. A library procedure's specialised summary is the same query as a user procedure's.
- **Whole-program facts:** "no `tcl::mathfunc` override", "no `tcl_precision` write", "no `unknown` handler that could intervene" are absence proofs over every reachable body unit, in the naming sense: Logical advice cannot grant them, so they need the potential-execution coverage of the [cross-cutting review](cross-cutting-review.md) (fault 8).
- **Release policy** ([expressions](expression-design.md)): octal, numeral syntax, float spelling and character model come from one per-release policy owner; no folder defaults to 9.0 when the release is absent, and "absent" folds what every release agrees on instead of folding nothing.
- **Typed values** ([analysis data structures](analysis-data-structures.md)): `ExactValue` as the single constant value, with lattices holding it and SCCP constant sets keyed by it, replacing the stringified `(tag, String)` keys.

## Plan

1. **One constant value:** `ExactValue` (bytes, numeric reading, representation) as the value in SCCP, the expression folder, static loops, return summaries and `ConstOps`; the other nine types become views or go; one literal parser (retire the three `parse_literal_value` copies); float spelling decided once from the release policy.
2. **One evaluator per command:** registry folds call `tcl-cmd-core` (`string` case conversion, `reverse`, `map`, `format`, `scan`, `concat`, `mathop`) the way value transfers already do; codegen's `fold_const_branch` and `concat_cmd` use the shared expression folder and `tcl_syntax::list::concat_values`; one `format` path for every entry point.
3. **One SCCP:** the diagnostic "original value facts" solver and the main solver share their fold functions, then one solver with two outputs.
4. **Specialised procedure summaries:** fold user and library procedures with constant arguments, recursion under a fuel budget, calls inside expressions, and cross-file calls through contract queries; static loops wired into SCCP for bounded constant loops.
5. **Whole-program absence facts** for `tcl::mathfunc`, `tcl_precision` and `unknown`, unlocking math functions and 8.5–8.6 float folds where proven.
6. **No silent no-op API:** remove `optimise()` and the `None` dialect from public entry points, or make "release unknown" fold what every release agrees on.
7. **Execution-checked conformance:** every fold fixture run before and after rewriting under every tclsh and jimsh, per release, as a committed test (today the registry fold differential is mostly 9.0-only and the optimiser coverage suite was checked by hand).
