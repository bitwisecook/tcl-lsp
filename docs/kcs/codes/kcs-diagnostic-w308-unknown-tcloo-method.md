# KCS: W308 — Why does the analyser flag an unknown method on my object?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, analyser

## Profiles

default

## Question

Why does the analyser warn that a method does not exist on my object?

## Why

When the analyser can see the class an object holds, it checks each
`$obj method …` dispatch against that class and its superclasses. If the
method is not defined anywhere in the class hierarchy, calling it raises
`unknown method "…"` at run time, so the analyser reports **`W308`** on the
dispatch.

The check only fires when the receiver's class is statically known and the
class defines no `method unknown` catch-all — a snit-typed receiver, a
forwarded method, or an object of an unknown class is left alone to avoid
false positives.

## Symptoms

- A yellow squiggle (warning) appears under the `$obj method` call, with the
  message `Unknown method '<name>' on class '<class>'`.

## Example that triggers it

```tcl
oo::class create Point {
    method x {} { return 0 }
}
set p [Point new]
$p distance
```

The analyser reports **`W308`** on `$p distance` — `Point` has no `distance`
method.

## Fix

Call a method the class actually defines, or add the method to the class:

```tcl
oo::class create Point {
    method x {} { return 0 }
    method distance {} { return 0 }
}
```

## How to suppress

Add `# noqa: W308` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `W001`, `W307`
