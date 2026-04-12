# KCS: W125 — Why does the analyser flag an orphaned control-flow keyword?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn about a control-flow keyword used outside its expected context?

## Why

Keywords like `break`, `continue`, and `return` are only meaningful inside a loop or procedure. Using them at the top level or outside the right context raises a runtime error.

## Symptoms

- A yellow squiggle appears under the keyword, with the message "orphaned control-flow keyword outside loop or proc".

## Example that triggers it

```tcl
break
puts "done"
```

The analyser reports **`W125`** on the top-level `break`.

## Fix

```tcl
foreach item $list {
    if {$item eq "stop"} { break }
}
puts "done"
```

Move the control-flow keyword inside the appropriate loop or procedure body.

## How to suppress

Add `# noqa: W125` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W001`, `E001`
