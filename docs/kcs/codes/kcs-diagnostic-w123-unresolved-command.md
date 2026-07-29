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
