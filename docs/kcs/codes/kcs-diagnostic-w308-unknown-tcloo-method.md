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

"Statically known" includes every flow the compiler's object-type lattice
proves, not just a direct `set p [Point new]` assignment: a handle returned
by a factory procedure (`set p [mk]`), returned by a method and captured
(`set b [$a make]`), aliased (`set q $p`), or passed as a procedure or
constructor parameter. The same facts drive hover, go-to-definition, and
Find All References, so this warning and those features always agree on the
receiver's class.

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

## `self`'s own dispatch spelling

`my method` is `TclOO`'s bareword self-dispatch keyword — a call inside a
method body that reaches the *enclosing* class. `[self]` and `[self
object]` are the same receiver spelt as a command substitution (`self`
alone, with no argument, is documented as equivalent to `self object`), so
`W308` checks them identically:

```tcl
oo::class create Widget {
    method render {} { return {} }
    method refresh {} {
        [self] render
        [self] paint
    }
}
```

The analyser reports **`W308`** on `[self] paint` — `Widget` has no
`paint` method — exactly as it would for `my paint`.

## Methods the class system generates for you

A method does not have to be written as a `method` to exist. A class
created by Tcl 9.0's `oo::configurable` metaclass answers `configure` for
the `property` members it declares, even though no `method` body defines it,
so it draws no `W308`:

```tcl
oo::configurable create Point {
    property x y
}
set p [Point new]
$p configure -x 27
```

The same holds for a class that merely *inherits* from a configurable one —
`configure` is a real method on the configurable ancestor — and inside a
method body via `my configure`.

Two receivers are deliberately *not* covered, because both really do fail at
run time and `W308` is right to report them:

- A plain `oo::class` receiver has no `configure` at all.
- `cget` is **not** generated. Tk widgets, snit and \[incr Tcl] all pair
  `configure` with `cget`, but `oo::configurable` does not — `configure`
  with a single `-property` argument is how you read one. `$p cget -y`
  above would fail with `unknown method "cget"`.

## Template methods a subclass supplies

An abstract base class may `my`-dispatch a method it never defines, leaving
each concrete subclass to write it — the template-method pattern:

```tcl
oo::class create Formatter {
    method run {} { my Render }     ;# no Render on Formatter's MRO
}
oo::class create HtmlFormatter {
    superclass Formatter
    method Render {} { ... }
}
```

This runs fine: `my` late-binds on the actual receiver, which is always a
subclass instance when the base is never instantiated, and it bypasses
export filtering, so even a capitalised (unexported) subclass method is
reachable. `W308` abstains when any known class whose linearisation
contains the receiver class resolves the method — whether that subclass is
in the same file or, in an editor workspace, in a sibling document. A
subclass that gets the method from its own `mixin` counts too.

The boundary is evidence, not charity:

- A `my` dispatch **no** subclass anywhere defines is still a warning —
  that is the typo case the check exists for.
- The abstention is `my`-only. `[self] Render` in the base keeps its
  `W308`: it dispatches through the object's own command, where an
  unexported subclass method really is unreachable (the same reach split
  as `my varname` versus `[self] varname`).
- Single-file tools (`tcl diag` on one file, the fp-sweep harness) see no
  sibling documents, so a base analysed alone still warns — the workspace
  view is what supplies the refuting subclass.

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

Objects built through the *new* name are checked too, because the class
itself is unchanged — only the command that reaches it moved:

```tcl
oo::class create Dog { method bark {} { return woof } }
rename Dog Cat
set d [Cat new]
$d fly
```

The analyser reports **`W308`** on `$d fly`, naming the class `::Dog`, and
`$d bark` stays quiet. An `interp alias {} Cat {} Dog` and a chain of
renames (`rename Dog Cat; rename Cat Kitten`) work the same way.

Order matters, because the rename has to have run:

```tcl
oo::class create Dog { method bark {} { return woof } }
set d [Cat new]
rename Dog Cat
$d fly
```

Here `Cat` does not exist yet when `[Cat new]` runs, so nothing is built and
the analyser makes no claim about the method. The out-of-order call itself is
reported as `W128` instead. Inside a procedure or method body the order does
not matter — the whole file loads before any body runs.

An alias that binds extra words (`interp alias {} Cat {} Dog create`) is left
alone. Those words shift the constructor's own arguments, so `Cat new` is not
the call `Dog new` would be.

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
