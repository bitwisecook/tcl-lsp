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

## Limits

The check never claims a name the document, the registry, or a package
already accounts for. It stays quiet when:

- **The registry knows the command.** `tcl::chan::memchan`, `tcl::chan::cat`,
  and the rest of tcllib's virtual-channel packages live inside `::tcl::chan`
  but are public, documented commands, so they are never flagged. For
  `::tcl::chan` — the one namespace Tcl shares with third-party packages —
  only a tail that is a real subcommand of the public `chan` command counts
  as private at all.
- **The file defines the command itself.** A `proc ::tcl::dict::mine` (or the
  same proc written inside `namespace eval ::tcl::dict { … }`) makes
  `::tcl::dict::mine` the document's own command, not Tcl's.
- **A `package require` covers the name.** `package require
  tcl::chan::memchan`, or a requirement on any namespace prefix of the
  called command, means the name belongs to that package.

Two further limits are deliberate:

- **No per-version modelling.** Which subcommands a private namespace carries
  on a given Tcl release is not part of Tcl's documented contract and churns
  release to release, so the check fires the same way under every dialect.
- **A quick fix only when the rewrite is legal.** The fix is offered only
  when the tail after the namespace is a genuine subcommand of the public
  ensemble in the active dialect. `::tcl::clock::GetSystemTimeZone` is real
  private machinery with no public spelling, so it warns with no fix rather
  than suggesting the broken `clock GetSystemTimeZone`. A command head
  written as a braced or quoted word (`{::tcl::dict::create}`) likewise
  warns without a fix, because the head token's range does not cover its
  delimiters.

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
performs exactly this rewrite whenever the rewrite is legal — see
**Limits** above.

## How to suppress

Add `# noqa: W143` at the end of the offending line.

## Notes

Tcl's own public, documented commands that also live directly under
`tcl::` — `tcl::mathop::+`, `tcl::mathfunc::sin`, `tcl::prefix` — are never
flagged, in either the `::`-rooted or the bare spelling. Only the 11
private, undocumented implementation namespaces listed above trigger this
check; a user's own namespace nested under `tcl::` (for example
`::tcl::mycustom::foo`) is unaffected, since it is not one of them.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `W113`, `W116`, `W128` — other codes about calling a
  command in a way that is technically legal but not the intended,
  documented usage.
