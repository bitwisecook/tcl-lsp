# KCS: feature — Extract into proc

> **Audience:** User
> **Type:** Functionality

## Summary

Moves a selected run of commands into a new proc and replaces the selection
with a call — carrying any variable the moved code assigns back to the
caller's frame with `upvar`, so the extraction does not silently change what
the program does.

## Applies to

all-editors, refactoring

## How to use

Select one or more whole commands and trigger code actions (Ctrl+. in VS Code,
`<leader>ca` in Neovim). Choose **Extract selection into proc**. The editor
then opens a rename on the generated name, which is a placeholder.

## The problem it solves

A `proc` has its own variable frame, so anything the moved code assigns stops
being the caller's variable. Extracting the middle two lines of

```tcl
set x 0
set x 1
puts $x
puts "after=$x"
```

into a proc that takes `x` as an ordinary parameter prints `after=0` instead
of `after=1`: a parameter is a *copy*, and the caller never sees it change.

The extraction classifies each variable the selection touches:

| Variable | Becomes |
|---|---|
| Read, never written | An ordinary value parameter — a copy is correct, nothing writes it back. |
| Written, and read again after the selection | Passed **by name** and re-bound with `upvar 1`, so the assignment lands in the caller's frame. |
| Written, and never read again | A proc local. It stops leaking into the caller entirely. |
| Bound by a loop the selection contains (`foreach n …`, `dict for {k v} …`) | Written, and classified by the two rows above — the caller keeps a loop variable after the loop exactly as it keeps an assignment. |

A name the selection reads *before* it writes is a parameter either way: the
list word of `foreach x $x …` is evaluated before the loop rebinds `x`, and
`set y [expr {$y + 1}]` reads `y` before its own assignment lands. Reading a
name only after the write — the `$n` in a `foreach n … {…$n…}` body — makes it
the loop's own local, not something the caller supplies.

The classification covers the selection's whole statement tree, so a write
inside an `if`, `foreach`, `switch` arm, `try`, or `catch` body counts exactly
as a top-level one does:

```tcl
set total 0
foreach n {1 2 3} {
    set total [expr {$total + $n}]
}
puts $total
```

extracts to a proc that takes `total` by name, not one that takes it by value
and drops the sum. A body that opens its own variable frame — a nested `proc`,
`namespace eval`, `uplevel`, or an `apply` lambda — is deliberately not
counted, in either direction: what it writes and what it reads are that
frame's variables, not the selection's, so a nested proc's parameter never
becomes a parameter of the extracted one.

Nor does a name that only *looks* like a reference. A braced word substitutes
nothing, so the `$notavar` of `set msg {$notavar}` and the `$a` of an `apply`
lambda handed to `lsort -command` are literal text rather than variables the
caller has to supply. A braced word that carries script or an expression is
still read as such, and so is every word of a command that performs Tcl
substitution itself: `subst {hello $name}` does read `name`.

## Example

Selecting the middle two lines of the first example extracts to:

```tcl
set x 0
proc extracted_proc {xName} {
    upvar 1 $xName x
    set x 1
    puts $x
}

extracted_proc x
puts "after=$x"
```

which prints `1` then `after=1`, exactly as the original did.

The definition is placed immediately **above the enclosing top-level
command**, not at line 0, so it lands after any `package require` or
`namespace` prologue. The generated name is checked against the workspace's
symbols and the command registry, so it never accidentally shadows a builtin.

## What it will not do, and why

A refused extraction still appears in the menu, greyed out, with its reason.

| Refused | Reason |
|---|---|
| The selection contains `return`, `break`, `continue`, `upvar`, `uplevel`, `global`, `variable`, `info level`, … | These act on the *call frame*. `return` would return from the new proc; `break` would escape a loop that no longer encloses it; `upvar 1` would alias one frame too far. |
| A written variable's name is computed (`set $n 1`) | Which variable leaves the selection is a run-time fact. |
| A written variable is an array element (`set a(x) 1`) | An array element is not a place the scalar `upvar` protocol can carry. |
| A command head is computed (`$cmd …`, `{*}$words`) | Its argument roles — and so which variables it reads and writes — are unknown. |
| The selection is inside a `namespace eval` or a class definition body | The extracted proc would be created in a different namespace, changing what its unqualified calls and variables resolve to. |

The frame-sensitive command list and the read/write argument positions are
the command registry's own, so a command gains this treatment by being
described in the registry rather than by being named inside the refactoring.

## Operational context

Implemented in `rust/tcl-lsp-core/src/refactor/extract_proc.rs`. The selection
is snapped to whole segmented commands; the "is this variable read again?"
question is asked over the frame the selection runs in, which ends at the
nearest body that opens a variable frame of its own (a proc body, a
`namespace eval`, an `apply` lambda) or at the file. An `if` or `foreach` body
is not such a boundary, so a selection made inside a loop is still classified
against the code that owns the variable. Each enclosing same-frame body counts
in full rather than only the part after the selection, because a loop runs
again and reads on its next pass what the previous one assigned.

Both the selection's own classification and that read-again question walk the
statement tree through `nested_dispatch_regions`, the shared same-frame walker
Find All References and the caller-frame scan use, and find the frame
boundaries through its complement, `frame_shifted_dispatch_regions`. It is
registry-driven throughout: `Plain` body arguments, `switch`-style clause arms
via the registry's own `CaseListSpec`, and `[…]` command substitutions are
descended, the ones inside a braced expression argument through the expression
parser's own script bridge, while `Structural` bodies and `apply` lambdas end
the walk.

## Failure modes

- A selection that covers no complete command offers nothing at all.
- Variables reached only through `upvar`, a trace, or a computed name are not
  modelled; those selections are refused rather than guessed at.
- The extracted proc is always created at the top level of the current file;
  cross-file placement is not supported.

## Test anchors

- `editors/vscode/src/test/refactorActions.test.ts`
- `rust/tcl-lsp-core/src/refactor/extract_proc.rs` (module tests)

## Discoverability

- [KCS feature index](README.md)
- [Refactoring tools](kcs-feature-refactorings.md)
- [Inline proc](kcs-feature-refactor-inline-proc.md)
