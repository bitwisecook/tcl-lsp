# KCS: Expression parsing — Pratt parser and braced vs unbraced expressions

## Symptom

A contributor needs to understand how `expr` bodies are parsed into AST trees,
why braced expressions enable static analysis while unbraced ones do not, or
needs to add support for new operators (e.g. iRules extensions).

## Context

The Pratt parser in `expr_parser.py` uses binding powers to handle operator
precedence without recursive-descent ambiguity.  Braced expressions (`expr {…}`)
are parsed into `ExprNode` AST trees; unbraced expressions fall back to
`ExprRaw` and cannot be statically analysed (diagnostic W100).

Source: [`core/parsing/expr_parser.py`](../../../core/parsing/expr_parser.py),
[`core/compiler/expr_ast.py`](../../../core/compiler/expr_ast.py)

## Content

### Braced vs unbraced expressions

**Braced** — `expr {$a + $b * 2}`:
- Braces protect content from Tcl substitution.
- The Pratt parser receives the verbatim string and produces an `ExprNode` tree.
- Enables constant folding, type inference, and algebraic simplification.

**Unbraced** — `expr $a + $b * 2`:
- Tcl substitutes variables *before* the expression is compiled.
- The parser receives an already-substituted string it cannot statically analyse.
- Falls back to `ExprRaw(text="${a} + ${b} * 2")`.
- Triggers diagnostic **W100** ("Unbraced expr body").

### Pratt parser binding powers

The parser assigns each binary operator a `(left_bp, right_bp)` pair.  Higher
binding powers bind tighter:

| Operator | Left BP | Right BP | Notes |
|----------|---------|----------|-------|
| `\|\|`, `or` | 4 | 5 | Logical OR |
| `&&`, `and` | 6 | 7 | Logical AND |
| `==`, `!=`, `eq`, `ne` | 14 | 15 | Equality |
| `<`, `>`, `<=`, `>=` | 16 | 17 | Comparison |
| `+`, `-` | 20 | 21 | Additive |
| `*`, `/`, `%` | 22 | 23 | Multiplicative |
| `**` | 25 | 24 | Exponentiation (right-associative) |

### Worked example — `expr {$a + $b * 2}`

Tokenisation produces: `$a`, `+`, `$b`, `*`, `2`.

Pratt parsing:
1. Parse `$a` → `ExprVar("a")`
2. See `+` (bp 20,21) — enter infix with left=`ExprVar("a")`
3. Parse `$b` → `ExprVar("b")`
4. See `*` (bp 22,23 > 21) — tighter, recurse
5. Parse `2` → `ExprLiteral("2")`
6. Build `ExprBinary(MUL, ExprVar("b"), ExprLiteral("2"))`
7. Return to `+`: build `ExprBinary(ADD, ExprVar("a"), MUL-node)`

### iRules extensions

| iRules operator | Equivalent | Binding power |
|----------------|------------|--------------|
| `starts_with` | prefix eq | (14, 15) |
| `ends_with` | suffix eq | (14, 15) |
| `contains` | substring eq | (14, 15) |
| `matches_glob` | glob match | (14, 15) |
| `matches_regex` | regexp | (14, 15) |

These are registered in `_BINARY_BP` alongside the standard operators.

## Decision rule

- To add a new operator, add its binding power entry to `_BINARY_BP` in
  `expr_parser.py` and its `BinOp`/`UnOp` variant in `expr_ast.py`.
- If the expression is unbraced, no AST is produced — downstream passes must
  handle `ExprRaw` gracefully (skip constant folding, skip type inference).
- Right-associative operators use `left_bp > right_bp` (e.g. `**` uses 25, 24).

## Related docs

- [Example 21 in walkthroughs](../../example-script-walkthroughs.md#example-21-expression-parsing--braced-vs-unbraced)
- [GLOSSARY.md — AST](../../GLOSSARY.md#ast)
- [kcs-compiler-pipeline-overview.md](kcs-compiler-pipeline-overview.md)
