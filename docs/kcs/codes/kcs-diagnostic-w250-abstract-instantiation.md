# KCS: W250 — Why does the analyser warn about instantiating this class?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, tcloo

## Profiles

default

## Question

Why does the analyser flag `SomeClass new` / `SomeClass create obj` with
**`W250`**?

## Why

The class is defined with `oo::abstract`. `TclOO`'s `oo::abstract`
metaclass *removes* the `new` and `create` constructors from the class, so
instantiating it directly is a runtime error:

```
cannot create object: ... is abstract
```

An abstract class exists only to be subclassed — a concrete subclass keeps
`new` / `create` and is what you instantiate. The analyser reports `W250`
when it can see, in the current document, that the class named in a
`Class new …` or `Class create …` call (including the assignment shape,
`set o [Class new …]`) was created with `oo::abstract`.

It is **sound**: it only fires when the class's recorded metaclass is
`oo::abstract`, never on the `oo::abstract create Foo { … }` definition
itself, and never on a concrete subclass.

## Example that triggers it

```tcl
oo::abstract create Shape {
    method area {} { error "subclass must override" }
}

# W250 — Shape has no `new`/`create`.
set s [Shape new]
Shape create shape1
```

## Fix

Instantiate a concrete subclass instead of the abstract base:

```tcl
oo::abstract create Shape {
    method area {} { error "subclass must override" }
}

oo::class create Circle {
    superclass Shape
    variable r
    constructor {radius} { set r $radius }
    method area {} { return [expr {3.14159 * $r * $r}] }
}

# Fine — Circle is concrete.
set s [Circle new 2]
```

## How to suppress

Add `# noqa: W250` on the line **above** the offending command, or disable the code
via `tclLsp.diagnostics.disabled`.

## Related

- [KCS codes index](README.md)
- [Type navigation (TclOO)](../features/kcs-feature-type-navigation.md)
