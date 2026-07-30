# KCS: W123 — Is this command unresolved?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default (on in the default profile)

## Question

Why does the analyser flag a command it cannot resolve?

## Why

A command the analyser cannot find in the registry, user procs, or unknown handler will likely fail at runtime.

## Symptoms

- A blue squiggle (hint) appears under the command name, with the message "unresolved command".

## Example that triggers it

```tcl
unknownCmd $arg
```

The analyser reports **`W123`** on `unknownCmd`.

A proc, class, `rename` target, or `interp alias` that was renamed or
deleted away, with no later re-establishment under the same name, is also
unresolved — calling it fails `invalid command name` at runtime just like
a name that was never defined:

```tcl
proc helper {} { return 1 }
rename helper {}
proc caller {} { helper }
```

The analyser reports **`W123`** on `helper` inside `caller`. Defining a
fresh `helper` (or `rename`-ing a different command to that name) after
the deletion re-establishes it and clears the warning.

## What does not trigger it

A call the file demonstrably makes *before* the deletion is not a
problem, even when the call is written inside a proc body. The analyser
follows the chain of enclosing definitions to decide this: if some
top-level call reaches the proc before the deletion runs, the nested call
resolved at the time it ran, and no warning is reported.

```tcl
proc helper {} { return hi }
proc inner {} { helper }
proc outer {} { inner }
outer
rename helper {}
```

Nothing is flagged here. `outer` runs on the fourth line, which runs
`inner`, which runs `helper` — all before the rename.

The chain only follows call sites that are guaranteed to run. A call
written inside an `if`, a loop, or a `switch` arm proves nothing about
whether the enclosing proc really reaches it, so it does not silence a
warning on a later call:

```tcl
proc helper {} { return hi }
proc b {} { helper }
proc a {} { if {0} { b } }
a
rename helper {}
b
```

The analyser reports **`W123`** on `helper` inside `b`, and it is right
to: the last line really does fail with `invalid command name "helper"`.

A `rename` or deletion written inside an `if`, a loop, or a `switch` arm
is treated the same way, in the other direction. Because it might never
run, it is not taken as proof that the command is gone:

```tcl
oo::class create Dog {
    method bark {} { return woof }
}
if {0} { rename Dog {} }
Dog new
```

Nothing is flagged on `Dog`. This is a deliberately simple rule about
where the `rename` is written, not about whether the branch is taken —
`if {1} { rename Dog {} }` is treated the same way, so a deletion the
analyser could in principle prove does happen is still not reported.

A command whose name is vacated by a `rename` is still flagged, even
though the command itself survives under its new name. `rename Dog Cat`
moves the class to `Cat`; `Dog new` afterwards fails with `invalid
command name "Dog"`, so the old name is reported.

## Commands defined in another file

The check reads one file at a time, so a `proc` that lives in a sibling
file is not something it can see on its own. Two things make it visible.

Always on, needing no setting: a command an installed library auto-loads
(`tclIndex`), and a command defined by a package the file `package
require`s. The package's `pkgIndex.tcl` is found on the search path,
including the one the file builds for itself with `lappend auto_path [file
dirname [file dirname [info script]]]`.

Off by default, opt in with `tclLsp.features.crossFileResolution`: every
`proc` and class the **workspace** defines, whether or not anything links
the two files. Turn it on and this stops firing:

```tcl
# helper.tcl
namespace eval tcl::mathfunc {
    proc Pi {} { return 3.141592653589793 }
}

# vector.tcl — no source, no package require
proc angle {dp} { return [expr {Pi()}] }
```

With the setting off, `Pi` is reported as unresolved even though Go to
Definition and Find All References both resolve it to `helper.tcl` — the
two disagree, and the offered quick fix ("did you mean `ni`?") would break
working code if accepted. With it on, they agree.

A name nothing in the workspace defines is still reported, so turning the
setting on removes false alarms without hiding real ones.

## What a `proc unknown` changes

Defining your own `unknown` handler at **global** scope switches this check
off for the whole file: Tcl sends every unresolved command word to that
handler, so the analyser cannot prove any name is really unresolved.

```tcl
proc unknown {cmd args} { exec $cmd {*}$args }
totallyBogusCommand
```

Nothing is flagged here.

A `proc unknown` written **inside a namespace** is an ordinary procedure
that happens to share the name, and changes nothing:

```tcl
namespace eval ::mylib {
    proc unknown {cmd args} { exec $cmd {*}$args }
}
totallyBogusCommand
```

The analyser still reports **`W123`** on `totallyBogusCommand`, and running
this really does fail with `invalid command name "totallyBogusCommand"` —
Tcl consults `::unknown` for a bare unresolved word regardless of the calling
namespace. To register a per-namespace handler, use `namespace unknown NAME`,
which the analyser models separately.

## Commands that only resolve inside a TclOO method

`link`, `my`, `next`, `nextto`, `self`, and `classvariable` are not global
commands. They are reachable only from inside a method body — a `method`,
`constructor`, `destructor`, class-side method, or `oo::objdefine method`.
Written anywhere else they really are unresolved:

```tcl
# tcl-dialect: tcl9.0
link foo
```

The analyser reports **`W123`** on `link`, and running this fails with
`invalid command name "link"`. The same call inside a method body is
fine, and so is the fully qualified spelling `::oo::Helpers::link`, which
is a real command everywhere (calling it outside a method fails for a
different reason — it reports that it may only be called from inside a
method).

An `apply` lambda written inside a method body does **not** count: `apply`
runs its body in the global namespace, so the object context is gone and
these words are unresolved there too.

A Tcl 9 class `initialise` / `initialize` body is **not** flagged, even
though only `my` actually works there. That body runs in the class
object's own namespace, so the words really are found — calling one fails
with `self may only be called from inside a method`, which is a different
error from an unknown command, and `W123` is only about the latter.
Completion and hover still decline to offer them there.

For the full rule, see
[Where can I call `my`, `next`, `self`, and `link`?](../kcs-qa-where-can-i-call-my-next-self-and-link.md).

A built-in `expr` math function (`sin`, `max`, `abs`, …) called with
function-call syntax inside `expr` resolves to the command it dispatches to
(`::tcl::mathfunc::<name>`) and never draws `W123`, whether or not it has
also been overridden by a `proc ::tcl::mathfunc::<name>` in the file:

```tcl
set x [expr {sin(1.0) + max(1, 2, 3)}]
```

## Fix

```tcl
package require mypackage
mypackage::knownCmd $arg
```

Define the command, or use `package require` to load the package that provides it.

## How to suppress

Add `# noqa: W123` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W120`, `W307`
