# KCS: My iRule's `contains` condition is never folded or flagged

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, tcl-lsp CLI, mcp, diagnostic, lowering, sccp, const-fold

## Question

Why does an iRules word operator — `contains`, `starts_with`, `ends_with`,
`matches_glob`, or `matches_regex` — behave as if the tools had never heard
of it, so a condition that is obviously always true is neither simplified by
`tcl opt` nor reported by the analyser?

## Symptoms

- `tcl opt myrule.irule` leaves `if {$x contains "cd"} { … }` untouched.
- `tcl diag` reports nothing for that condition, while the plain-Tcl
  equivalent (`if {$x == 1}`) draws **I230 Condition … is always true**.
- The editor shows the same gap: no I230 on an iRule word-operator condition
  the analyser can prove constant.

## Answer

A word operator is only an operator in a dialect that has it. If the file is
being analysed as plain `tcl8.6`, `contains` is not an operator at all — it
becomes an opaque expression the constant folder cannot evaluate, so nothing
folds and nothing is reported. So the question is which dialect the file
resolved to.

Every verb — `opt`, `format`, `minify`, `explore`, `diagram`, `callgraph`,
`dataflow`, `symbols`, `highlight`, `dis`, `diff`, as well as `diag`, `lint`,
and `validate` — resolves the dialect the same way, in this order: a
`# tcl-dialect:` directive, a shebang, a `package require Tcl` guard, content
signals such as a `when EVENT {` handler, then the file extension, falling
back to `tcl8.6`.

Detection only fails when the file offers none of those signals — an iRule
with no `when` handler and a `.txt` name, say. Two ways to fix it:

- Name the dialect on the command line with `--dialect f5-irules`.
- Pin it in the file itself with a `# tcl-dialect: f5-irules` comment on one
  of the first few lines. This is the better fix, because the editor reads it
  too.

Confirm with a two-line file, `probe.irule`:

```tcl
when HTTP_REQUEST {
    set x "abcdef"
    if {$x contains "cd"} { HTTP::respond 200 }
}
```

```console
$ tcl opt probe.irule
when HTTP_REQUEST {
    set x "abcdef"
    if {1} { HTTP::respond 200 }
}
# optimised: 1 rewrite(s)
# O101  Fold constant expression

$ tcl diag probe.irule
probe.irule:3:8: info    I230     Condition '$x contains "cd"' is always true; …
```

Both behave identically with and without an explicit `--dialect f5-irules`,
because the `when` handler is itself a detection signal.

Plain Tcl is deliberately different: `contains` is not an operator there, so
the same condition in a `.tcl` file draws
[W003](codes/kcs-diagnostic-w003-dialect-invalid-expr-operator.md) — "operator
is not available in dialect 'tcl8.6'" — and no I230. A W003 on your condition
is therefore the clearest signal that the file resolved to plain Tcl.

## Related

- [KCS index](README.md)
- [Which commands does tcl-lsp consider available in a dialect?](kcs-qa-which-commands-are-available-in-a-dialect.md)
- [Expression parsing — Pratt parser and braced vs unbraced expressions](../design/compiler/expression-parsing.md)
- [Glossary](../GLOSSARY.md)
