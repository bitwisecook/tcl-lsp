# KCS: When does a `namespace import` in another file count?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp-cli, analyser

## Question

`namespace import`, `namespace export`, and `namespace forget` all take
effect **when they run**, not where they are written. Two files give no
clue about which runs first. So when does the server treat a
`namespace export` or a `namespace import` written in another file as
having happened?

## Answer

Only where the workspace proves the order. There are two proofs, and
everywhere else the server abstains toward answering — the foreign
export counts, and a foreign revocation revokes nothing.

**`source FILE` proves an exact order.** Sourcing a file runs its whole
body at that statement, so everything written above the `source` had run
when the file loaded, and nothing written below it had. That gives a
complete order for every pair of statements in one `source` tree.

**`package require NAME` proves half an order.** A `package require`
that returns has left the package loaded, so the file that
`package provide`s it has run. It does **not** prove the file ran *at*
the require: if some other file required the package first, this one
finds it already loaded and evaluates nothing. So the server will say
"the provider has already run" from the require onwards, and will never
say "the provider has not run yet".

That asymmetry is not caution for its own sake — the same file answers
differently depending on what ran before it:

```tcl
# probe.tcl, unchanged in both runs
namespace eval ::mymod {}
namespace eval ::app { namespace import ::mymod::* }
package require mymod
```

Run on its own, `::app::helper` does not exist: the import saw an empty
`::mymod`. Run after any other file did `package require mymod`, the
same three lines give a working `::app::helper`.

### Where the server stays silent

No order at all — so the pre-existing lenient answer stands — when:

- the two files sit in different `source` trees and neither requires a
  package the other provides;
- the `source` path is computed (`source [file join $dir x.tcl]`) and
  does not fold to a file the workspace holds;
- a file is `source`d from two different places, or sits on a `source`
  cycle: Tcl tolerates re-sourcing, so it has no unique position;
- **two** files in the workspace `package provide` the same package —
  which one a `package require` runs is decided by the runtime
  `auto_path` order, and the files need not be equivalent;
- the workspace holds a `package ifneeded` registration for the package
  — that script is what actually runs, and it may load something else;
- the `package require` is conditional (`if {[catch {package require
  Tk}]}`) or names a variable (`package require $pkg`).

For the relation itself, its proofs, and its measured effect on real
code, see the
[import-order design note](../design/import-order-source-graph.md).
