# KCS: feature — Rename

> **Audience:** User
> **Type:** Functionality

## Summary

Rename a proc, variable, or class consistently across the file and the
workspace.

## Applies to

all-editors, MCP, refactoring

## Question

What does the rename feature do, and how do I use it?

## How to use

- **In the editor**: put your cursor on the proc or variable, press
  `F2`, type the new name, and press **Enter**. The editor updates the
  definition and every reference in the current file in one step.
- **From a script or MCP tool**: call the `rename` tool with the source
  file, a cursor position, and the new name. The tool returns the full
  set of text edits for the editor or script to apply.

## Options

- `tclLsp.features.rename` — turn the rename feature on or off. Default:
  on.

## How it finds references

Rename uses the same shared proc-reference matching as **Find
References**, so the definition and every call site are always updated
together. For the full contract, see
[LSP feature providers](../../design/contracts/lsp-feature-providers.md).

Renaming a class rewrites every use of its name, including the
`superclass`, `mixin`, and `[incr Tcl]` `inherit` arguments that name it
in other class bodies — across files. Because rename and **Find
References** read the same recorded references, they can never disagree,
so a class rename keeps the inheritance graph intact rather than leaving
a base-class name dangling.

A command name stored in a variable and dispatched as `$cmd` is kept
alive too: the rename rewrites the **defining constant's literal**
(`set cmd target` becomes `set cmd renamed`), never the `$cmd` head
itself, so the renamed script still runs. When a contributing constant
has no exact source spelling to rewrite, the whole rename is refused
rather than left half-applied. A file sourced under several namespaces
is one physical declaration with several runtime names; renaming it
updates every namespace's call sites together. For the full contract,
see
[resolution soundness](../../design/resolution-soundness-945.md).

Renaming a `TclOO` method rewrites the declaration plus every `my method`
dispatch site (however deeply nested in `[…]` substitutions or same-frame
control-flow / `eval` bodies — see [Find References](kcs-feature-references.md)),
every external `$obj method` site, and every override-family member (a
superclass or subclass that (re)defines the same method) — all as one
rename. A **pure-consumer** file — one that only calls the method (`set f
[Factory new]; $f make`, or a bare `Factory make` classmethod dispatch) and
declares no part of the class — is rewritten too: rename and **Find
References** resolve those call sites through one shared resolver, so a
consumer file can never be left calling a name the rename has already taken
away. A bare classmethod dispatch is rewritten wherever it is written,
including inside a `namespace eval` body or an `apply` lambda body — a class
command is an ordinary command and resolves from any frame, unlike a `$obj`
receiver. A subclass's own `Subclass method` dispatch renames with the
parent's `classmethod`, but **not** with a stock-TclOO `self method`, which
is not inherited and so was never calling the renamed member. Renaming a
property rewrites the declaration plus every `my
<property>` read; a property has no `$obj` dispatch or inheritance model,
so those are out of scope by design, not a gap. Because a method is never a
bare-callable command (only `my method` dispatches it), a method / property
rename only ever rewrites `my`-dispatched sites — never an unrelated bare
command invocation that happens to share the renamed name.

Renaming a method also rewrites the class's own `export` / `unexport` /
`filter` lists, which name methods rather than merely mentioning them.
Leaving one behind is not cosmetic: a `TclOO` method whose name starts with
an upper-case letter is *unexported* by default, so a class that writes
`method Foo {} {...}` plus `export Foo` stops working the moment the two
disagree — tclsh 9.0 and 8.6 both answer `unknown method "Bar": must be
destroy` for the renamed method.

Renaming a **namespace variable** written with a `::` qualifier
(`$::ns::v`, `set app::ns::v 1`) renames the cell across the whole
workspace: the `variable v` declaration wherever it lives, every
`variable` / `global` / `namespace upvar` alias of it inside a proc or
method body *and* that alias's own unqualified `$v` reads, and every
qualified occurrence in any file. An unqualified `$v` that is not such an
alias is deliberately left alone — a bare name means whatever the local
scope chain supplies, which is a per-file question the cell's identity
cannot answer.

A method, a classmethod, and a property can share one name within the same
class (rare, but each lives in its own independent table, so it's legal);
renaming resolves to whichever declaration the cursor actually sits on
rather than a fixed priority, so the rename never silently retargets a
different member than the one you clicked.

### When rename refuses

Rename answers with an **error and a reason**, not a silent no-op, whenever
it can see that no edit set would keep the program running. Precision is the
point: a refused rename costs you a keystroke, a wrong one silently breaks
code. The gate refuses when:

- a member of the class you are renaming is dispatched on a **receiver whose
  class is not tracked** — `$other X` where `$other` came from `lindex
  $args 0`, a `dict get`, or any other value the source does not tie to a
  constructor. The call may well reach the member you are renaming (real
  `TclOO` code does exactly this in copy constructors), and nothing in the
  source proves it does not;
- a member name is **computed at run time** on a receiver of the class —
  `$obj $m`, `my $m`. No edit can keep such a site consistent with a renamed
  declaration;
- two different classes bind the **same object command** (`Factory create
  rex` and `Widget create rex` in one namespace), so which class a later
  `rex make` reaches is a runtime fact;
- an `export` / `unexport` / `filter` naming the member was recorded but its
  word cannot be located to rewrite;
- for a namespace variable: the new name would **collide** with a cell that
  already exists, or a file in that namespace **computes a variable name**
  (`set $n 1`, `variable $n`) that might be this very cell.

In each case, running **Find References** first shows you what rename can and
cannot see.

## Failure modes

- The rename updates some but not all of the references. This almost
  always means the symbol is visible under more than one scope; run
  **Find References** first to confirm what would be touched.
- The rename is applied to a different symbol than the one you clicked.
  This can happen if the cursor is on a namespace-qualified name where
  the qualifier is ambiguous.
- The rename does nothing at all. When a `$cmd` dispatch of this command
  gets its value from a constant with no exact source spelling (for
  example a `foreach` list element), no edit set can keep that dispatch
  working, so the rename is refused outright.
- The editor shows an error naming an untracked receiver, a computed member
  name, an ambiguous object command, or a computed variable name. That is
  the safety gate above: the rename is refused deliberately, because the
  edit set it could produce would change what the program does. Give the
  receiver a spelling the analyser can follow (`set other [Vector3d new
  ...]`) — or rename by hand, checking each site — rather than expecting a
  partial edit set.

## Screenshots

![rename dialog inline](../screenshots/18-rename.png)

## Related

- [KCS feature index](README.md)
- [Glossary](../../GLOSSARY.md)
- [LSP feature providers](../../design/contracts/lsp-feature-providers.md)
