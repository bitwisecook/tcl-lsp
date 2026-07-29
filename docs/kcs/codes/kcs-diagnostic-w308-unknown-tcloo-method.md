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

## What a deleted class changes

The check asks whether the class is still live **at the dispatch**, not
whether it survives to the end of the file. A class deleted later in the
file is fully alive at any dispatch written before the deletion, so those
dispatches are still checked:

```tcl
oo::class create Dog { method bark {} { return woof } }
proc walk {} {
    set d [Dog new]
    $d fly
}
walk
rename Dog {}
```

The analyser reports **`W308`** on `$d fly`. Running this really does
fail with `unknown method "fly"` — the trailing `rename` happens after
`walk` has already run.

A class deleted *before* the dispatch is a different matter. There the
constructor itself fails, so the object is never created and the method
name is not the real problem. The analyser stays quiet about the method
and reports the dead class instead, as
[`W123`](kcs-diagnostic-w123-unresolved-command.md).

`rename Dog Cat` does not delete the class. It moves the class to a new
name, and every object already built from it keeps answering its
methods, so those dispatches are still checked:

```tcl
oo::class create Dog { method bark {} { return woof } }
set d [Dog new]
rename Dog Cat
$d fly
```

The analyser reports **`W308`** on `$d fly`. The vacated name `Dog` is a
separate question, and is reported separately as `W123` if anything
still calls it.

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
- [W123 — unresolved command](kcs-diagnostic-w123-unresolved-command.md)
- Related codes: `W001`, `W307`
