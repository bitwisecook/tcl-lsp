# KCS: My iRule's `contains` condition is never folded or flagged

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, tcl-lsp CLI, mcp

## Question

Why does an iRules word operator — `contains`, `starts_with`, `ends_with`,
`matches_glob`, or `matches_regex` — behave as if the tools had never heard
of it, so a condition that is obviously always true is neither simplified by
`tcl opt` nor reported by the analyser?

## Symptoms

- `tcl opt myrule.irule` leaves `if {$x contains "cd"} { … }` untouched, but
  the same file with `tcl opt --dialect f5-irules myrule.irule` folds it to
  `if {1}` and reports **O101 Fold constant expression**.
- Even with `--dialect f5-irules`, `tcl diag` reports nothing for that
  condition, while the plain-Tcl equivalent (`if {$x == 1}`) draws
  **I230 Condition … is always true**.
- The editor shows the same gap: no I230 on an iRule word-operator condition
  the analyser can prove constant.

## Answer

Update to a build that includes the fix for issue #1048. Two separate faults
produced these symptoms, and both are fixed:

1. **The command line dropped the detected dialect.** Only the diagnostics
   verbs (`diag`, `lint`, `validate`) resolved a file's dialect from its
   contents and name. Every other verb — `opt`, `format`, `minify`,
   `explore`, `diagram`, `callgraph`, `dataflow`, `symbols`, `highlight`,
   `dis`, `diff` — silently analysed the file as `tcl8.6`. They now use the
   same detection: a `# tcl-dialect:` directive, a shebang, a
   `package require Tcl` guard, content signals such as a `when EVENT {`
   handler, then the file extension, falling back to `tcl8.6`.
2. **The dialect never reached the compiler's expression parser.** Every
   `if`, `while`, `for`, and `expr` condition was parsed with the plain-Tcl
   operator set, so a word operator was not read as an operator at all — it
   became an opaque expression the constant folder could not evaluate. The
   condition is now parsed with the document's own operator set, which is
   what lets the fold, and the I230 that reports it, happen.

Check the fix with a two-line file, `probe.irule`:

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

Both must now behave identically with and without `--dialect f5-irules`.

Plain Tcl is deliberately unchanged: `contains` is not an operator there, so
the same condition in a `.tcl` file still draws
[W003](codes/kcs-diagnostic-w003-dialect-invalid-expr-operator.md) — "operator
is not available in dialect 'tcl8.6'" — and no I230.

If detection picks the wrong dialect for a file (an iRule with no `when`
handler and a `.txt` name, say), name it explicitly with `--dialect`, or pin
it in the file itself with a `# tcl-dialect: f5-irules` comment on one of
the first few lines.

## Related

- [KCS index](README.md)
- [Which commands does tcl-lsp consider available in a dialect?](kcs-qa-which-commands-are-available-in-a-dialect.md)
- [Expression parsing — Pratt parser and braced vs unbraced expressions](../design/compiler/expression-parsing.md)
- [Glossary](../GLOSSARY.md)
