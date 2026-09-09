# KCS: W149 — Why does the analyser say my call matches a different release?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser say my argument count belongs to another release of the
command, rather than reporting too many or too few arguments?

## Why

Some commands changed how many arguments they take between releases of the
package that owns them. The analyser picks the shape that applies at the
document's [resolved version floor](../../GLOSSARY.md#version-floor) and checks
the call against it.

W149 fires when the count does **not** fit the shape the floor selects but
**does** fit one of the command's other shapes. The call is not malformed; it
is written for another release, and that has a different fix. A count fitting
no shape at all stays an ordinary
[E002](kcs-diagnostic-e002-too-few-arguments.md) /
[E003](kcs-diagnostic-e003-too-many-arguments.md).

## Symptoms

A yellow squiggle under the call, with a message naming both releases:

- "3 arguments to 'probe::grew' matches Probe 5.0, but the resolved floor
  3.0 selects the Probe 3.0 shape — raise the floor with `package require
  Probe 5.0`, or write the call for the Probe 3.0 shape"
- "2 arguments to 'probe::grew' matches Probe 3.0, but the resolved floor
  5.0 selects the Probe 5.0 shape — that shape was valid until 5.0; write
  the call for the Probe 5.0 shape"

The first says the call is written for a **later** release than the file
targets; the second says it is left over from an **earlier** one.

## Example that triggers it

```tcl
package require Probe 3.0

probe::grew a b c
```

With `probe::grew` recorded as taking two arguments from 3.0 and three from
5.0, the three-argument call is the 5.0 shape while the floor is 3.0. The
analyser reports **`W149`** on the call.

## Fix

**Written for a later release** — raise the floor so the file targets that
release (`package require Probe 5.0`, or whatever pins the version in your
project), or rewrite the call in the older shape. Raise the floor when you
deploy on the newer release; rewrite when you must keep supporting the older
one.

**Written for an earlier release** — rewrite it in the current shape. The
floor cannot go backwards without abandoning whatever else the file needs from
the newer release.

With no resolvable floor the analyser falls back to the command's default
shape and never raises W149. A command the file defines itself silences the
check, exactly as it silences E002 and E003.

## How to suppress

Turn the code off for a project with `disabled = W149` under `[diagnostics]`
in `.tcl-lsp.ini`, for one file with a `# tcl-lsp: disable=W149` directive at
the top of the file, or in your editor with `tclLsp.diagnostics.W149` set to
`false`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W135`, `W136` (not introduced yet), `W139` (retired),
  `W144` (deprecated), `E002` / `E003` (a count fitting no shape).
