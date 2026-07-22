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
