# KCS: feature — Completions

> **Audience:** User
> **Type:** Functionality

## Summary

Context-aware completions for commands, subcommands, variables, switches, and procs.

## Applies to

all-editors, MCP, analyser

## How to use

- **Editor**: Triggered automatically as you type or with Ctrl+Space.
- **MCP**: `complete` tool — pass source and cursor position.
- **Settings**: Toggle with `tclLsp.features.completion`.

## Operational context

The completion provider offers context-sensitive suggestions based on the cursor position: command names, subcommands after a known command, variable names after `$`, proc arguments, switch flags, and package names after `package require`.

When what you have typed matches nothing by prefix and is at least two
characters long, the provider falls back to fuzzy matching, so a small typo
still finds the intended name: `lsaerch` offers `lsearch`, `string lenght`
offers `length`, `lsort -ncoase` offers `-nocase`, and `$bnaana` offers
`$banana`. Choosing a fuzzy suggestion replaces the typo. Prefix matches are
never mixed with fuzzy ones — a fragment that matches something today keeps
exactly today's list.

### Inside an `expr` expression

Where the cursor sits in an expression, the `expr` math functions are offered
under their bare names, ranked first because they are what you can actually
call there:

```tcl
set a [expr {si     ;# offers sin, sinh
if {ma              ;# offers max, min-style names too
```

This covers every expression argument, not just `expr` — an `if` or `while`
condition and a `for` loop test all count, because the command registry is what
says which argument of which command is an expression.

The list follows your chosen Tcl version: `max` needs Tcl 8.5 or later,
`isnan` needs 9.0, and `gamma` needs 9.1. Typing the same prefix outside an
expression offers no math functions at all, because Tcl cannot call them
there.

## File-path anchors

- `rust/tcl-lsp-core/src/completion.rs`
- `rust/tcl-lsp-core/src/expr_context.rs` — the expression-argument test
- `rust/tcl-registry/src/mathfunc.rs` — the registry's math-function query

## Failure modes

- Missing completions after registry or parser changes.
- Wrong context detection (e.g. offering commands where variables are expected).

## Test anchors

- `rust/tcl-lsp-core/src/completion.rs` (unit tests)
- `rust/tcl-lsp-core/src/expr_context.rs` (expression-context unit tests)
- `rust/tcl-lsp-server/tests/e2e/completion.rs`
- `rust/tcl-lsp-core/tests/mathfunc_and_word_recognition.rs` — the `expr`
  math-function cases

## Screenshots

- `03-completions` — completion list triggered on partial command

![completion list triggered on partial command](../screenshots/03-completions.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
