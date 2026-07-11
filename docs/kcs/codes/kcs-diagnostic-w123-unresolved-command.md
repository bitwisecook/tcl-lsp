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
