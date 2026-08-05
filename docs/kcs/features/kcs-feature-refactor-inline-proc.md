# KCS: feature — Inline proc

> **Audience:** User
> **Type:** Functionality

## Summary

Replaces a call with the called proc's body, binding the proc's parameters to
the call's argument **values** — defaults and all — and refusing, with a
reason, wherever that cannot be done without changing what the program does.

## Applies to

all-editors, refactoring

## How to use

Put the cursor on a call to a proc defined in the same file and trigger code
actions (Ctrl+. in VS Code, `<leader>ca` in Neovim). Choose **Inline proc
'name'**.

When the call cannot be inlined safely, the entry still appears — greyed out,
with the reason. That is deliberate: a missing menu entry tells you nothing,
whereas "the body calls 'return', which acts on the call frame" tells you
exactly what to change first.

## Example

```tcl
proc double {x} {
    expr {$x * 2}
}
double 5
```

becomes

```tcl
proc double {x} {
    expr {$x * 2}
}
expr {5 * 2}
```

A parameter with a declared default binds that default when the call omits the
argument, which is what Tcl itself does:

```tcl
proc greet {{name world}} { puts $name }
greet                       ;# inlines to  puts world
```

## What it will not do, and why

Inlining is a *binding* problem, not a text-replacement problem. `f {a b}`
does not pass the four characters `{a b}`; it passes the three-character value
`a b`, because the braces are the caller's quoting and the parse consumes
them. Splicing the written word into the body changes the value the body sees.

| Refused | Reason |
|---|---|
| The body is more than one command | Several commands cannot be spliced into one caller word without changing how the caller parses. |
| The body uses `return`, `break`, `continue`, `upvar`, `uplevel`, `global`, `variable`, `info level`, … | These act on the *call frame*. `return` in an inlined body returns from the caller. |
| The body assigns a variable | A proc's locals vanish when it returns; a caller's do not. The assignment would leak — and could overwrite a caller variable of the same name. |
| The call passes the wrong number of arguments | Tcl would raise `wrong # args`; inlining must not silently make the call work. |
| The body reads `args` | `args` is a *list*. No single spelling of a list means the same thing in every word context. |
| An argument's value is not a plain word | Only a value with no whitespace, quotes, braces, brackets, `$`, `\`, `;`, or `#` can be written into the body verbatim and still mean itself. |
| A parameter used in an `expr` operand is bound to a non-number | `expr {abc * 2}` reads `abc` as a function name, not as the string. |
| A parameter used more than once is bound to a run-time value | `f [next]` with the body reading the parameter twice would call `next` twice. |

The frame-sensitive command list is the command registry's own, so a command
gains this protection by being described in the registry — not by being added
to a list inside the refactoring.

## Operational context

Implemented in `rust/tcl-lsp-core/src/refactor/inline_proc.rs`. The call head
is resolved with the same resolver go-to-definition and find-references use, so
the three cannot disagree about which proc a call reaches. Expression operand
positions come from the registry's `ArgRole::Expr`, and variable-writing
argument positions from `ArgRole::VarWrite`.

## Failure modes

- A proc reached through `interp alias` or a dynamic `rename` is not resolved,
  so no action is offered.
- Cross-file procs are not inlined; the definition must be in the same
  document.

## Test anchors

- `rust/tcl-lsp-core/src/refactor/inline_proc.rs` (unit suite)
- `rust/tcl-lsp-server/tests/e2e/code_actions.rs`
- `editors/vscode/src/test/refactorActions.test.ts`

## Discoverability

- [KCS feature index](README.md)
- [Refactoring tools](kcs-feature-refactorings.md)
- [Extract into proc](kcs-feature-refactor-extract-proc.md)
