# KCS: E006 — Why is my formal-parameter list invalid?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser say that a procedure or method's parameter list is
invalid?

## Why

Tcl reads each formal parameter as either a name or a two-field `name default`
list. A parameter may not be namespace-qualified or an array element, and a
specifier with more than two fields makes Tcl reject the whole callable.

## Symptoms

- A red squiggle appears under the literal parameter-list word with `E006`.
- The message gives Tcl's reason, such as `too many fields in argument
  specifier` or `not a simple name`.

## Example that triggers it

```tcl
proc greet {{name default extra}} {
    puts "Hello, $name"
}
```

The analyser reports **`E006`** because the one parameter specifier has three
fields.

## Fix

```tcl
proc greet {{name default}} {
    puts "Hello, $name"
}
```

Use a single name or a `name default` pair. Duplicate names remain valid Tcl,
although the later parameter shadows the earlier one.

The analyser does not report `E006` for a computed parameter-list value such
as `proc greet $parameters { ... }`, because that value is only known at run
time.

## How to suppress

Add `# noqa: E006` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `E002`, `E003`, `E005`
