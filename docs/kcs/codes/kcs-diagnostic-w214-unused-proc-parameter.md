# KCS: W214 — Why does the analyser warn about an unused proc parameter?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, liveness

## Profiles

default

## Question

Why does the analyser flag a procedure parameter that is declared but never read?

## Why

An unused parameter may indicate a signature mismatch or missing logic that was meant to use it.

## Symptoms

- The parameter name is faded out in the parameter list rather than squiggled —
  W214 is a hint tagged as unnecessary code, so editors grey it instead of
  underlining it.
- The Problems panel carries a message like `Parameter 'greeting' of proc
  '::greet' is unused`.

## Example that triggers it

```tcl
proc greet {name greeting} {
    puts "Hello, $name"
}
```

The analyser reports **`W214`** on the `greeting` parameter.

## Fix

Use the parameter or remove it from the signature:

```tcl
proc greet {name greeting} { puts "$greeting, $name" }
```

## Where the mark lands, and which proc it names

The faded range covers just the offending parameter's *name* inside the parameter
list, so each unused parameter gets its own tight range rather than stacking on
the whole `proc` definition.

The fully-qualified name in the message is the proc's **defining** namespace,
which for a proc created inside another proc's body is the namespace the
*enclosing* proc is defined in — not the namespace the `proc` command was
written in. Tcl resolves the name in the namespace current when the body
executes, and a proc body executes in the namespace of the proc itself:

```tcl
namespace eval ::a {}
proc a::outer {} { proc helper {unused} { return 1 } }
a::outer
info commands ::a::helper    ;# -> ::a::helper
info commands ::helper       ;# -> {}   (nothing was created here)
```

so W214 reports `::a::helper`. The name word's own qualifier wins over any
enclosing `namespace eval`, and an absolutely-written inner name (`proc ::abs
…`) is unaffected.

## How to suppress

Add `# noqa: W214` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [liveness](../../GLOSSARY.md#liveness)
- Related codes: `W211`, `W220`
