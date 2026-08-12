# KCS: W314 — Why can a definition have no fully-qualified name?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser

## Profiles

default

## Question

Why does the analyser warn that a proc, class, or namespace "has no absolute (fully-qualified) name"?

## Why

In Tcl, a written run of two or more colons is a namespace separator, and the
whole run counts as one separator. A lone `:` is an ordinary name character,
so `proc : args {…}` is legal — but no absolute spelling can ever reach it:
`:::` parses as "the empty-named command in the global namespace", not as
`:`. The same applies to a namespace named `:` — everything inside it is
reachable only by relative lookup (for example, `namespace inscope : :`).

This bites real tooling and real Tcl:

- `namespace which :` prints `:::`, a string that does not resolve back.
- A namespace ensemble cannot dispatch an exported command named `:` (the
  ensemble builds `::ns:::`, which re-parses as the empty name).
- Callback qualification (`namespace code`, `interp alias` targets) breaks
  the same way.

The behaviour is identical in Tcl 8.4 through 9.1 (`TclGetNamespaceForQualName`),
and was verified against tclsh 8.6 and 9.0.

## Symptoms

- A yellow squiggle on the definition name, with the message "has no absolute
  (fully-qualified) name".

## Example that triggers it

```tcl
proc : args {
    return "hello"
}

namespace eval : {
    proc helper {} { }
}
```

The analyser reports **`W314`** on the `:` name in both definitions.

## Fix

Use a name with at least one non-colon character, so an absolute form exists:

```tcl
proc fwd args {
    return "hello"
}
```

If the colon name is deliberate (for example, a `:` proc that forwards to a
child interpreter in an interactive shell), the definition still works when
called bare — the warning only flags that qualified access, ensembles, and
`namespace which` output cannot reach it.

## How to suppress

Add `# noqa: W314` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [Design: namespace resolution](../../design/compiler/namespace-resolution.md)
- Related codes: `W113`, `W123`
