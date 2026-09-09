# KCS: W300 — Does source with a variable path allow code execution?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn when `source` is called with a variable argument?

## Why

Sourced files execute as Tcl code; an attacker-controlled path leads to arbitrary code execution.

## Symptoms

- A yellow squiggle appears under the path argument, with the message *"source with a dynamic path (variable or command substitution) executes arbitrary Tcl code. Ensure the path is not influenced by untrusted input."*

## Example that triggers it

```tcl
source $filepath
```

The analyser reports **`W300`** on the `source` call.

## Fix

```tcl
set helpers "/opt/app/lib/helpers.tcl"
source $helpers
```

Source a fixed path, or a variable the analyser can prove holds a literal
one. A computed path — `source [file join $dir $name]` included — stays
flagged, because the file it loads is decided at run time.

## How to suppress

Add `# noqa: W300` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W101`, `W313`
