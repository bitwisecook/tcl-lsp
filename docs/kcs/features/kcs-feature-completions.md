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

### After a `TclOO` receiver

A `$obj ` completion offers the receiver class's *instance* members and a
`ClassName ` completion offers the class object's own members — two separate
tables, exactly as the interpreter keeps them. Which of them a member word
touches follows the wrapper it was written under: an unwrapped (or
`private`-scoped) `method` / `export` / `unexport` / `filter` / `deletemethod`
acts on the instance side, the same word under `self` acts on the class-object
side, and neither reaches across.

That siding matters to the offered list in both directions. A `self unexport m`
must not stop `$obj m` being offered, and — since the class-side flip now
travels between files on a channel of its own — a `self unexport m` written in
one file *does* stop `ClassName m` being offered in another (#1119). Before
that channel existed the class-side flip was simply lost, so completion went on
offering a member the interpreter answers with `unknown method "m"`.

The receiver word itself resolves the way Tcl resolves any command word — the
namespace in effect where it is written first, then the global one, then through
`namespace import` — so `namespace eval ::a { C cm }` reaches `::a::C` even when
the class is declared in another file, an inner `::a::C` shadows a global `::C`,
and an import that has not run at that point binds nothing (#1178 review).

A member a later word in the same body deletes is not offered; one a
`renamemethod` moves is offered under its **new** name, carrying the source's
body and visibility (#1121). A body real Tcl would reject outright still
completes normally — the partial class is kept for exactly that reason — but
the offending word carries a
[`W315`](../codes/kcs-diagnostic-w315-class-definition-cannot-run.md).

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
- `rust/tcl-lsp-core/src/oo_dispatch.rs` — same-document member visibility (`method_dispatch_provider`, per `MethodBucket`)
- `rust/tcl-lsp-core/src/workspace_index.rs` — `method_dispatch_chain` / `class_method_dispatch_chain` (the cross-file per-side export union)
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
