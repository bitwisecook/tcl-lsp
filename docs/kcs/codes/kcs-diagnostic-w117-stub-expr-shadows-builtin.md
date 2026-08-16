# KCS: W117 — Why does a stub expr function shadow a built-in?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn that a stub expression function shadows a built-in?

## Why

Tcl's `expr` has built-in math functions like `sin`, `cos`, and `abs`. Defining a `tcl::mathfunc` with the same name replaces the built-in implementation, which can subtly change numerical results throughout the application.

## Symptoms

- A yellow squiggle appears under the function name, with the message "stub expr function 'abs' shadows a built-in function".

## Example that triggers it

```tcl
proc tcl::mathfunc::abs {x} {
    return [expr {$x < 0 ? -$x : $x}]
}
```

The analyser reports **`W117`** on the `abs` function definition.

## Fix

```tcl
proc tcl::mathfunc::my_abs {x} {
    return [expr {$x < 0 ? -$x : $x}]
}
```

Choose a function name that does not collide with a built-in math function.

## How to suppress

Add `# noqa: W117` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W113`, `W116`
