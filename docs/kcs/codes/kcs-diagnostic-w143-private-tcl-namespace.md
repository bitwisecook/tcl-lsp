# KCS: W143 — Direct call into a private `::tcl::` implementation namespace

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, analyser

## Profiles

default

## Question

Why does the analyser warn on a call like `::tcl::dict::create`?

## Why

Real Tcl backs several of its built-in ensemble commands with a private
sub-namespace under `::tcl::` — `dict create` is implemented by
`::tcl::dict::create`, `string totitle` by `::tcl::string::totitle`, and so
on for `array`, `file`, `info`, `clock`, `binary`, `namespace`, `encoding`,
`zlib`, and `chan`. Calling directly into one of these namespaces works —
it is not a runtime error — but it is entirely undocumented, unsupported,
and not a stable public contract: which subcommands exist under a given
`::tcl::` namespace, and even whether the namespace exists at all, changes
between Tcl releases. `::tcl::zlib`, for example, is missing on some Tcl
8.6 builds. The public ensemble command (`dict`, `string`, …) is the
documented, version-stable way to reach the same functionality, so the
analyser suggests it instead.

This check works at the namespace level only: it does not track which
subcommands each private namespace happens to carry on any particular Tcl
version, because that detail is not part of Tcl's documented contract and
churns release to release. It fires the same way regardless of dialect or
Tcl version.

## Symptoms

- A yellow squiggle appears under the command word, with a message like:
  "'::tcl::dict::create' is a private Tcl implementation namespace; use the
  public ensemble command instead — e.g. 'dict create'."

## Example that triggers it

```tcl
set d [::tcl::dict::create a 1 b 2]
```

The analyser reports **`W143`** on `::tcl::dict::create`.

## Fix

```tcl
set d [dict create a 1 b 2]
```

Use the public ensemble command and subcommand instead of calling directly
into the private namespace that backs it. A quick fix is offered that
performs exactly this rewrite.

## How to suppress

Add `# noqa: W143` at the end of the offending line.

## Notes

Tcl's own public, documented commands that also live directly under
`tcl::` — `tcl::mathop::+`, `tcl::mathfunc::sin`, `tcl::prefix` — are never
flagged. Only the 11 private, undocumented implementation namespaces listed
above trigger this check; a user's own namespace nested under `tcl::` (for
example `::tcl::mycustom::foo`) is unaffected, since it is not one of them.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `W113`, `W116`, `W128` — other codes about calling a
  command in a way that is technically legal but not the intended,
  documented usage.
