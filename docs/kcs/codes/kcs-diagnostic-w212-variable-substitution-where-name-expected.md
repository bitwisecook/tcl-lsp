# KCS: W212 — Why does the analyser warn about a variable substitution where a name is expected?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser flag `set $x` as suspicious?

## Why

`set $x` sets a variable whose name is the VALUE of `x`, not `x` itself — almost always a mistake.

The check applies to every argument a command reads as a plain variable *name*,
not just `set`'s first argument. The name positions come from the command
registry (the `VarWrite` / `VarRead` argument roles), so `incr $x`,
`info exists $x`, `vwait $x`, `catch {…} $res`, `lassign $l $x`, `scan $s %d $x`,
`dict with $d …`, and `array set $a …` are all flagged the same way. `upvar` is
the one nuance: only its *local*-name slots are strict name positions — a
computed `$remote` in the other-var slot is a legitimate indirect link and is
not flagged.

## Symptoms

- A yellow squiggle appears under the substituted variable, with the message "variable substitution where name expected".

## Example that triggers it

```tcl
set $varname "hello"
```

The analyser reports **`W212`** on `$varname` in the name position.

## Fix

```tcl
set varname "hello"
```

If you genuinely need to set a variable by computed name, use `upvar 0 $varname local`.

## How to suppress

Add `# noqa: W212` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W210`, `W306`
