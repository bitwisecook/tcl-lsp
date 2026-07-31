# KCS: feature — Hover

> **Audience:** User
> **Type:** Functionality

## Summary

Command documentation, proc signatures, variable info, and taint status on hover.

## Applies to

all-editors, MCP, analyser

## How to use

- **Editor**: Hover over any symbol to see documentation.
- **MCP**: `hover` tool — pass source, line, and character position.
- **Settings**: Toggle with `tclLsp.features.hover`.

## Operational context

The hover provider resolves the symbol under the cursor and returns documentation from the command registry, proc signatures from analysis, variable types, and taint tracking status for iRules.

### Commands defined in another file

When nothing in the current document explains the word under the cursor and
that word is a command being called, hover looks further afield — the same two
steps go-to-definition takes:

1. **Across the workspace.** A `proc` or class declared in another open
   document, including one reached through `source`.
2. **Through the library index.** A command an installed library auto-loads
   (a `tclIndex` or `pkgIndex.tcl` on the configured library paths), even
   though no file in the workspace declares it.

The popup is rendered from the *defining* file, so it reads exactly as it does
when you hover the declaration itself.

Hover only looks further afield for a **command being called**. An ordinary
argument word that happens to share a name with a proc in another file shows
nothing, so a `puts widget` never pops up an unrelated `widget` procedure.

### `expr` math functions

Inside an `expr` expression, hovering a function call shows that function's
documentation under its bare name:

```tcl
set a [expr {sin(1.0)}]
#            ^ hover here
```

Both spellings now read the same, because both come from the same command
registry entry: the bare `sin(…)` inside an expression, and the
`::tcl::mathfunc::sin` command spelling.

Your own override wins, exactly as it does when the code runs. A `proc` in a
`tcl::mathfunc` namespace is a real command, so hovering the call shows the
procedure:

```tcl
namespace eval tcl::mathfunc {
    proc li {list index args} { lindex $list $index }
}
if {li({0 1 0}, 1)} { … }
#    ^ hover shows `proc ::tcl::mathfunc::li`
```

A function your chosen Tcl version does not have shows nothing — `isnan(…)` is
Tcl 9.0 and later, `gamma(…)` is Tcl 9.1 and later.

### Positions that are not references

Hover stays silent where the text under the cursor is not a reference at all,
even when it looks like one:

- A `$name` inside a comment, or inside a brace-quoted value Tcl passes
  through unchanged (`set t {plain $level here}` really does print
  `plain $level here`).
- A word in a procedure's own **literal** parameter list — a parameter name
  shows the parameter, and a default value shows nothing, rather than a
  command that happens to share the name.

Both of those are decided narrowly, so a real reference is never hidden:

- Only a `#` that genuinely starts a command is a comment. A brace in the
  middle of a word is an ordinary character, so `puts a{# $v` still shows
  `$v` — that line really does substitute it.
- Only a *literal* parameter list is data. A computed one is live code and
  stays clickable, so `proc p [makeargs] {…}` still shows and jumps to
  `makeargs`, which really is called when the procedure is defined.

- A **namespace-qualified variable** declared in another file is described
  rather than passed over: hovering `$::tomato::version` names the cell
  (`::tomato::version`) and how many times the declaring file itself uses
  it. A bare `$v` is still a within-file question and shows nothing when the
  file has no such variable.
- A **namespace name** is described as one. Hovering the `::tomato` of
  `namespace children ::tomato` — or of the `namespace eval ::tomato { … }`
  block itself — names the namespace and counts every `namespace eval` block
  that declares it and every other place that refers to it, **across the
  whole workspace**, saying how many documents it looked at. Nothing is shown
  for a namespace no file in view declares, so a guess is never presented as
  an answer. In particular, a namespace whose name also happens to be a
  command's — `namespace exists string` — never shows that *command's*
  documentation instead: the two are different kinds of symbol, and the
  position says which one is meant.

### Caller-frame variables

Some variables are created by the procedure you call, not by the code you are
reading. `upvar` links a name in the caller's frame to a local in the callee,
so this is legal and common:

```tcl
proc setdef {varName} {
    upvar 1 $varName target
    set target "default"
}
proc build {} {
    setdef options          ;# creates `options` in build's own frame
    return $options         ;# …and this reads it
}
```

`build` never assigns `options`, so there is no `set` to hover. Hover on
`$options` shows a **Caller-frame variable** card naming `setdef` and the
parameter (`varName`) that carried the name, which is the whole explanation of
where the value comes from.

The call may be written inside an `if`, `while`, `foreach`, `catch` or
`switch` body — those run in the very frame they are written in, so the
variable still belongs to the enclosing procedure and hover still finds it. A
call inside a nested `proc`, an `apply` lambda, a `namespace eval` or an
`uplevel` body is a *different* frame, so it contributes nothing here.

Only `upvar 1` (or an omitted level, which means the same) reaches the
caller's frame. `upvar 0` aliases the callee's own local, `upvar #0` a global,
and `upvar 2` the caller's caller — none of them creates anything where you
are reading, so hover stays silent for those.

Two further limits are deliberate. A callee that binds a *literal* caller-side
name (`upvar 1 options options`) spells nothing at the call site, so there is
no word to attribute the variable to and hover stays silent. And a `$`-led
read that nothing binds shows **nothing at all** rather than falling back to a
command or method of the same name: Tcl keeps variable names and command names
in separate tables, so `$dataset` can never mean a method called `dataset`.

## File-path anchors

- `rust/tcl-lsp-core/src/hover.rs` — the provider and its renderers, including
  `qualified_symbol_hover` for a symbol defined in another document and
  `qualified_variable_hover` for a namespace variable declared in one
- `rust/tcl-lsp-core/src/caller_frame.rs` — caller-frame variable resolution
  and the `$`-led abstention shared with Go to Definition and Find References
- `rust/tcl-lsp-core/src/expr_context.rs` — the shared "is the cursor on an
  `expr` math-function call, and what does it resolve to" helper
- `rust/tcl-lsp-core/src/inert_text.rs` — the comment / data-brace tests that
  keep hover silent on text Tcl never substitutes
- `rust/tcl-registry/src/mathfunc.rs` — the registry's math-function query
  (bare name to command name, plus the two version axes)
- `rust/tcl-lsp-server/src/lib.rs` — `cross_document_hover` /
  `cross_document_variable_hover`, the workspace and library-index fallbacks

## Failure modes

- Missing hover after command registry updates.
- Incorrect position mapping in multi-line constructs.
- A command reached only at run time (built by `eval`, or dispatched through a
  variable) has no declaration to point at, so hover shows nothing.

## Test anchors

- `rust/tcl-lsp-server/tests/e2e/hover.rs`
- `rust/tcl-lsp-core/src/hover.rs` — renderer unit tests
- `rust/tcl-lsp-core/tests/mathfunc_and_word_recognition.rs` — the `expr`
  math-function and not-a-reference cases
- `rust/tcl-lsp-core/src/namespace_symbol.rs` (`mod tests`) — the namespace
  resolver and its hover text
- `rust/tcl-lsp-server/tests/e2e/issue1088_namespace_symbols.rs` — namespace
  hover in one file and across files

## Screenshots

- `02-hover-proc` — hover showing proc signature and documentation

![hover showing proc signature and documentation](../screenshots/02-hover-proc.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
