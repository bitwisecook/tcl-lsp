# Expression parsing — Pratt parser and braced vs unbraced expressions

How `expr` bodies are parsed into AST trees, why braced expressions can be
analysed statically while unbraced ones cannot, and where to add a new
operator such as an iRules extension.

The Pratt parser in `rust/tcl-syntax/src/expr/parser.rs` uses binding powers
to handle operator precedence without recursive-descent ambiguity.  Braced
expressions (`expr {…}`) are parsed into `ExprNode` AST trees; unbraced
expressions fall back to `ExprNode::Raw` and cannot be statically analysed
(diagnostic W100).

Source: `rust/tcl-syntax/src/expr/parser.rs` (Pratt parser),
`rust/tcl-syntax/src/expr/ast.rs` (AST), `rust/tcl-syntax/src/expr/operators.rs`
(the operator catalogue — each `BinOp`/`UnaryOp` spec carries its `surface`
and grammar version), `rust/tcl-lexer/src/expr_lexer.rs` (the expression
tokeniser; a word operator is an operator token only when the profile's
`LexerGrammar::has_word_operator` admits it)

### Braced vs unbraced expressions

**Braced** — `expr {$a + $b * 2}`:
- Braces protect content from Tcl substitution.
- The Pratt parser receives the verbatim string and produces an `ExprNode` tree.
- Enables constant folding, type inference, and algebraic simplification.

**Unbraced** — `expr $a + $b * 2`:
- Tcl substitutes variables *before* the expression is compiled.
- The parser receives an already-substituted string it cannot statically analyse.
- Falls back to `ExprNode::Raw { text: "${a} + ${b} * 2" }`.
- Triggers diagnostic **W100** ("Unbraced expr body").

### Pratt parser binding powers

The parser assigns each binary operator a `(left_bp, right_bp)` pair.  Higher
binding powers bind tighter:

| Operator | Left BP | Right BP | Notes |
|----------|---------|----------|-------|
| `\|\|`, `or` | 4 | 5 | Logical OR |
| `&&`, `and` | 6 | 7 | Logical AND |
| `\|` / `^` / `&` | 8 / 10 / 12 | 9 / 11 / 13 | Bitwise |
| `==`, `!=`, `eq`, `ne` (and the iRules string operators) | 14 | 15 | Equality |
| `<`, `>`, `<=`, `>=`, `in`, `ni`, `lt`, `le`, `gt`, `ge` | 16 | 17 | Relational, list membership |
| `<<`, `>>` | 18 | 19 | Shift |
| `+`, `-` | 20 | 21 | Additive |
| `*`, `/`, `%` | 22 | 23 | Multiplicative |
| `**` | 23 | 23 | Exponentiation (right-associative: equal binding powers) |

### Worked example — `expr {$a + $b * 2}`

Tokenisation produces: `$a`, `+`, `$b`, `*`, `2`.

Pratt parsing:
1. Parse `$a` → `ExprNode::Var { name: "a", .. }`
2. See `+` (bp 20,21) — enter infix with left=`ExprNode::Var { name: "a", .. }`
3. Parse `$b` → `ExprNode::Var { name: "b", .. }`
4. See `*` (bp 22,23 > 21) — tighter, recurse
5. Parse `2` → `ExprNode::Literal { text: "2", .. }`
6. Build `ExprNode::Binary { op: BinOp::Mul, left: Var("b"), right: Literal("2") }`
7. Return to `+`: build `ExprNode::Binary { op: BinOp::Add, left: Var("a"), right: MUL-node }`

### iRules extensions

| iRules operator | Equivalent | Binding power |
|----------------|------------|--------------|
| `starts_with` | prefix eq | (14, 15) |
| `ends_with` | suffix eq | (14, 15) |
| `contains` | substring eq | (14, 15) |
| `equals` | string eq | (14, 15) |
| `matches` | string eq (semantics unpinned) | (14, 15) |
| `matches_glob` | glob match | (14, 15) |
| `matches_regex` | regexp | (14, 15) |

The bare `matches` is measured *present* only
(`docs/design/bigip-irule-parser-measurements.md` §4a `e_matches`:
`expr {"abc" matches "abc"}` answers `1` in all three F5 contexts and
fails on both host builds). That probe is a single-operator,
exact-equality expression, so it pins neither the operator's binding
power nor which string-match reading it takes. It therefore inherits its
siblings' equality class, the VM answers it as a string equality — the
one reading the measured cell exercises — and the compiler deliberately
declines to constant-fold it. §12 of the measurements carries the
discriminating re-probe.

Each is a `BinOp` whose spec in `rust/tcl-syntax/src/expr/operators.rs`
carries `surface: Some(SpecSurface::IRULES)`, has a binding power in
`binary_bp()`, and is lexed as an operator only when the profile's grammar
admits the word (`LexerGrammar::has_word_operator`).

### Who supplies the dialect

`parse_expr(source, dialect)` takes the dialect because the word operators
are gated on it: with `None` (or a non-iRules dialect) `contains` is an ordinary
bareword, the parse fails, and the result is `ExprNode::Raw`. That fallback is
silent — the expression still compiles, it simply stops being analysable — so a
caller that forgets the dialect loses constant folding, type inference, and
every diagnostic derived from them (I230, O101, O112) with no error anywhere.

The dialect therefore has to be threaded from the document all the way down:

| Layer | Carrier |
|-------|---------|
| Consumer (CLI verb, LSP document, analyser) | the dialect name (`f5-irules`) |
| Compilation unit | `UnitBuildOptions::dialect`, set by `CompilationUnit::build_for_dialect` |
| Lowering | `Lowerer::dialect`, set by `lower_to_ir_with_dialect` / `Lowerer::with_dialect` |
| `if` / `while` / `for` conditions | `parse_expr(cond, self.dialect)` in `lowering/structured.rs` |
| `expr` / `return [expr …]` / `set x [expr …]` | `LoweringCommand::dialect` in the lowering hooks |
| Constant folding | `FoldPolicy::from_registry`, which reads the registry's stamped dialect profile |

Two failure modes: building a unit with `CompilationUnit::build_for` (or
`build_for_with_config`, which fixes only the *lexer* config) while holding a
real dialect, and handing the pipeline a registry with no profile stamped on
it — `registry_for_dialect` stamps it, a hand-assembled `build_default` +
`load_surface` does not, and `FoldPolicy::from_registry` reads such a registry
as plain Tcl.

## Decision rule

- To add a new operator, add its binding power entry to `binary_bp()`, map its
  text to the new `BinOp`/`UnaryOp` variant in `binop_from_text()` /
  `unaryop_from_text()` (all in `rust/tcl-syntax/src/expr/parser.rs`), add the
  variant in `rust/tcl-syntax/src/expr/ast.rs`, and give it a spec (surface,
  grammar version) in `rust/tcl-syntax/src/expr/operators.rs`. A word-like
  spelling is tokenised as an operator only where the profile's grammar admits
  it. Skipping any of these steps fails silently — the parser falls back to
  `ExprNode::Raw`, compiling fine but disabling structured analysis for
  expressions using the new operator.
- If the expression is unbraced, no AST is produced — downstream passes must
  handle `ExprNode::Raw` gracefully (skip constant folding, skip type inference).
- A left-associative operator has `right_bp = left_bp + 1`; a
  right-associative one (`**`) uses equal binding powers (23, 23).

## Related docs

- [Example 21 in walkthroughs](../example-script-walkthroughs.md#example-21-expression-parsing--braced-vs-unbraced)
- [GLOSSARY.md — AST](../../GLOSSARY.md#ast)
- [compiler-pipeline-overview.md](compiler-pipeline-overview.md)
