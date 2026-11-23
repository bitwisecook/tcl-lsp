# Expression parsing, evaluation and diagnostics

How `expr` and expression arguments are tokenised, parsed, represented, evaluated, folded, rendered and checked across execution (VM, runtime, codegen) and analysis (compiler, analyser, LSP). Part of [tidying the architecture](architecture-tidy-plan.md).

The shared core includes the tokeniser, main parser, expression tree (`tcl_expr::ExprNode`), evaluation walk (`ExprOps`), numeric and math-function code, and F5 predicates. Consumers still maintain several precedence tables, duplicate floor division and math-function lists, and re-parse expressions. [Parser plan](parser-crate-plan.md)

## List membership precedence

C Tcl places `in` and `ni` at `PREC_EQUAL` with `==`, `!=`, `eq` and `ne` in `generic/tclCompExpr.c`. The shared parser and Tcl dialect table now use the same left-associative level. Tcl 8.4 rejects list membership operators. Jim binds them below equality. See the [reference interpreter results](../../../evidence/tcl/expression-membership-precedence.md).

| Expression (`a` = 2, `l` = `{1}`) | C Tcl 8.5.19, 8.6.18, 9.0.4, 9.1.0 | Jim 0.84-15-g2f8e155 | VM and runtime before item 26 (8.6, 9.0) |
| --- | --- | --- | --- |
| `$a == $a in $l` | 1 | 1 | 0 |
| `"b" eq "b" in {1}` | 1 | 1 | 0 |
| `"x" in {x} == 1` | 1 | 0 | 1 |
| `1 != 1 ni {0}` | 0 | 0 | 0 |
| `2 < 3 in {1}` | 1 | 1 | 1 |

The conformance cases include `2 != 2 ni {0}` to distinguish the two groupings for `ni`. For `if {$a == $a in $l}` with those variable values, I230 reports the alternate branch as unreachable and W154 shows `($a == $a) in $l`. The renderer and parser now agree on this grouping. [Shared conformance cases](../../../rust/tcl-test-support/src/expressions.rs)

Other measured faults, from 61 expressions in [`oo-dispatch-probe/expressions/`](handover/oo-dispatch-probe/expressions/) run under tclsh 8.6.18 and 9.0.4, the VM and the runtime (integer and bignum edges, `**`, floor division and modulus, NaN and Inf, octal by release, `_` separators, boolean words, numeric versus string comparison, math functions, lazy `&&`/`||`/`?:`, shifts, number formatting):

- **Both engines in 8.6 mode lack `max()` and `min()`**: `invalid command name "tcl::mathfunc::max"`; they exist since 8.5. Everything else, including every release-dependent case (`int(1e20)`, `010`, `1_000`), matches tclsh.
- **W233 on `-7.0/0`** says it "raises divide by zero at runtime"; Tcl returns `-Inf` when either operand is a float. Only integer division and modulus by zero raise.
- **The optimiser folds 28 of the 61 under 8.6 and 34 under 9.0** (`set r [expr {…}]; puts $r`), and every rewrite gives tclsh's output. An earlier version of this page said it folds none: the probe passed no dialect, which makes every release-dependent fold decline. Folds, declines and missed folds are in [constants and folding](constant-folding-design.md).
- Correct: the W155 hint on `-7/2`, `-7%2` and `-1 >> 1` (grouping changes the value), W148 on `1_000` under 8.6.

## Structure

- **Parsers:** the lexer's `tokenise_expr*`; the Pratt parser (all consumers except Jim execution); a Jim term-stack parser (`jim_function_tree.rs`) used only at run time, which always uses the Jim 0.84 binding powers whatever the release, and has no `BinOp` for Jim's `<<<`/`>>>`; the 8.4 validator (`syntax_84.rs`), which builds no tree. `tcl-bigip-query` has its own parser for its DSL, not Tcl `expr`.
- **Precedence tables.** The Tcl parser's `binary_bp`, `BinOp::precedence` and tcl-dialect's `TCL_PRECEDENCE_ROWS` agree on list membership. Other tables include F5 and Jim's dialect rows, the Jim parser's unary, ternary and call levels, the 8.4 descent and `UNARY_BP`.
- **Evaluators:** the shared `ExprOps` walk is implemented by the VM, the runtime and the compiler's folder, over the shared `number_tower`, math functions and F5 predicates. Separately: the interval evaluator (`intervals.rs`, `i64` `+ - *` and comparisons only), native lowering's `floor_div`/`floor_mod`, the wasm floor adjustment, and BPF's deliberately truncating arithmetic. Floor division and modulus are written five times.
- **Math-function lists, seven:** tcl-expr's `mathfunc.rs` (itself several lists), tcl-dialect's `TCL_MATHFUNCS` and Jim tables (a hand copy whose tie to tcl-expr is claimed in a comment, with no test), the lexer's list (drift-guarded), the registry's 8.6 roster and value-transfer whitelist (hand copies), and two derived ones (LSP and VM). No membership disagreement found.
- **Type inference** over `ExprNode` is test-only; production rebuilds expression types from command tokens through the registry.
- **Re-parsing is the norm:** SCCP branch evaluation, folds O100/O101, operator validity, W100, W110, W154/W155, W156, the LSP precedence hover, parenthesis refactors, `brace_expr`, the minifier and the source graph re-parse source text (47 compiler files, 7 LSP, 7 registry, 3 codegen and more). Some check the result against the IR with `matches_tree`; hover and W154 do not.
- **Conformance:** syntax-failure and syntax-state vectors drive the parser, VM and runtime; `native_arithmetic_conformance.rs` pins integer edges against tclsh and Jim; the registry has differential fold tests. The list membership vectors exercise both unparenthesised operator orders, with relational precedence and Tcl 8.4 rejection as controls.

## Plan

1. **List membership precedence.** Item 26 sets `in` and `ni` at the equality level in the shared parser and Tcl dialect table. Conformance covers the reference interpreters, VM, runtime, Tcl constant folder and analyser.
2. **One precedence source:** tcl-dialect's `PrecedenceTable`, per release and per Jim version, generated from or checked against the C and Jim sources, drives the parser, `BinOp::precedence`, the renderer, the hover and W154/W155. Delete the other tables. Select the Jim table by the selected Jim release; add `BinOp` variants for `<<<`/`>>>`.
3. **Render is the inverse of parse:** a property test over generated trees (`parse(render(t)) == t` under every dialect) and over the conformance corpus; codegen's fallback and the optimiser's rewrites rely on it.
4. **An expression-semantics vector table:** the 61 cases here plus a precedence matrix generated from the C table (every operator pair, both associativities), run through tclsh 8.4–9.1, Jim, both engines, the compiler folder and the interval evaluator. Fix the 8.6 `max`/`min` gap in both engines.
5. **One floor division and modulus** in `tcl-numeric`, used by the VM fast tier, the folder, native lowering and wasm emission; BPF keeps its documented truncating semantics behind its own grammar.
6. **One math-function list** in tcl-expr, from which tcl-dialect, the lexer, the registry roster and whitelist, the LSP and the VM are generated or tested.
7. **Diagnostics and editor features read the lowered expression** with its source map (architecture plan, structured words and semantic model) instead of re-parsing; until then every re-parse is checked with `matches_tree`. Make expression type inference a production query over the tree, or delete the test-only walker.
8. **Fold where it is safe:** let the optimiser rewrite proven constant expressions (with the conformance table as its oracle), and make W233 follow Tcl's rule (float operands do not raise).
