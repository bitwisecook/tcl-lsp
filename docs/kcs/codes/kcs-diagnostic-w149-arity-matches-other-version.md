# KCS: W149 — this call matches a different release of the command

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## What does W149 mean?

Some commands changed how many arguments they take between releases of the
package that owns them. When a spec records those shapes, the analyser
picks the one that applies at the document's [resolved version
floor](../../GLOSSARY.md#version-floor) and checks the call against it.

W149 fires when the call's argument count does **not** fit the shape the
floor selects, but **does** fit one of the command's other shapes. That is
a different fault from a miscounted call, and it has a different fix: the
call is not malformed, it is written for another release.

Reporting it as "too many arguments" would send you to correct a call that
is already right somewhere — which is why it is its own code.

## Symptoms

A yellow squiggle under the call, with a message naming both releases:

- "3 arguments to 'probe::grew' matches Probe 5.0, but the resolved floor
  3.0 selects the Probe 3.0 shape — raise the floor with `package require
  Probe 5.0`, or write the call for the resolved floor 3.0 selects the
  Probe 3.0 shape"
- "2 arguments to 'probe::grew' matches Probe 3.0, but the resolved floor
  5.0 selects the Probe 5.0 shape — that shape was valid until 5.0; write
  the call for the Probe 5.0 shape"

The two directions matter. The first says the call is written for a
**later** release than the file targets; the second says it is left over
from an **earlier** one.

## Example that triggers it

```tcl
package require Probe 3.0

probe::grew a b c
```

If the registry records `probe::grew` as taking two arguments from 3.0 and
three from 5.0, the three-argument call is the 5.0 shape while the floor is
3.0.

## How do I fix it?

**Call written for a later release** — either raise the floor so the file
actually targets that release (`package require Probe 5.0`, or whatever
pins the version in your project), or rewrite the call in the older shape.
Raising the floor is right when you deploy on the newer release; rewriting
is right when you must keep supporting the older one.

**Call written for an earlier release** — rewrite it in the current shape.
The floor cannot go backwards without abandoning whatever else the file
needs from the newer release.

## What it does *not* do

- A count that fits **no** declared shape is an ordinary
  [E002](kcs-diagnostic-e002-too-few-arguments.md) or
  [E003](kcs-diagnostic-e003-too-many-arguments.md). There is no version
  story to tell and inventing one would be noise.
- With **no resolvable floor** the analyser falls back to the command's
  default shape and never raises W149: with no release known, a fitting
  alternative shape proves nothing about which release the file targets.
  This is the standing "no version known ⇒ do not gate" rule.
- A command the file defines itself (a `proc` shadowing the name) silences
  the check, exactly as it silences E002/E003.

## How do I turn it off?

Turn the code off for a project with `disabled = W149` under
`[diagnostics]` in `tcl-lsp.toml`, for one file with a
`# tcl-lsp: disable=W149` directive at the top of the file, or in your
editor with `tclLsp.diagnostics.W149` set to `false`.

## Related diagnostics

- [W135](kcs-diagnostic-w135-command-needs-newer-package.md) and
  [W136](kcs-diagnostic-w136-option-needs-newer-package.md) report a
  command or option that does not exist yet at the resolved floor. W149 is
  the same idea one level down: the command exists, but this *shape* of it
  belongs to a different release.
- [W139](kcs-diagnostic-w139-retired-at-resolved-version.md) and
  [W144](kcs-diagnostic-w144-deprecated-at-resolved-version.md) cover the
  removal and deprecation ends of the same lifecycle.
- [E002](kcs-diagnostic-e002-too-few-arguments.md) /
  [E003](kcs-diagnostic-e003-too-many-arguments.md) are what a count fitting
  no shape at all still produces.
