# KCS: W218 — Why is `args` flagged when it is not the last parameter?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, analyser

## Profiles

default

## Question

Why does the analyser warn about `args` in my procedure's parameter list?

## Why

In Tcl, `args` collects any remaining arguments **only when it is the final parameter**. The C implementation marks the special variadic behaviour on the last formal parameter alone, so an `args` declared anywhere else is an ordinary parameter that happens to be called `args`. A call then binds exactly one value to it, and the extra-arguments behaviour the author expected never happens — silently.

## Symptoms

- A yellow squiggle appears under the `args` parameter name, with the message "`args` here is an ordinary parameter".
- Calls with a variable number of arguments fail with "wrong # args" even though the procedure declares `args`.

## Example that triggers it

```tcl
proc greet {args name} {
    puts "Hello, $name: $args"
}
```

The analyser reports **`W218`** on `args` — it is not the final parameter, so it receives exactly one argument.

## Fix

```tcl
proc greet {name args} {
    puts "Hello, $name: $args"
}
```

Move `args` to the final position (or rename it if an ordinary parameter was intended).

## How to suppress

Add `# noqa: W218` at the end of the line declaring the procedure.

## Related

- [KCS codes index](README.md)
- [W214 — unused proc parameter](kcs-diagnostic-w214-unused-proc-parameter.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
