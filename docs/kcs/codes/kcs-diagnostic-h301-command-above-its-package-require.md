# KCS: H301 — Command used above its `package require`

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser, command-walk

## Profiles

default

## Question

The analyser hints that a command is used above the `package require` that
provides it. The script runs fine — is this wrong?

## Why

It is not wrong, and the hint does not say it is. Tcl resolves a command name
at the moment the call runs, not when the file is read, so where the
`package require` sits in the file has no effect on whether the call
succeeds. A file that requires at the bottom and only calls the package's
commands afterwards — or from inside a `proc` that nobody has called yet — is
correct Tcl, and the analyser treats it as correct: the command stays
available, and no availability warning fires.

What the hint reports is a *reading* problem. Someone scanning the file
top-down meets the call before the line that explains where it came from, and
every widely-used Tcl style guide puts requirements at the top for exactly
that reason. Moving the line costs nothing and removes the question.

`H301` is the ordering half. The other half — a command whose package is
never required **at all** — is
[`W120`](kcs-diagnostic-w120-missing-package-require.md), and the two are
disjoint by construction: `W120` fires when the requirement is absent, `H301`
when it is merely late. You will never see both for the same package.

## Symptoms

- A hint (the lowest severity — not a warning) over the earliest call that
  sits above the requirement.
- One hint per package, not per command: three `csv::` calls above one
  `package require csv` produce one hint, because there is one line to move.

## Example that triggers it

```tcl
csv::join {a b}
package require csv
```

And the deferred form, which is genuinely correct at run time and still
reads backwards:

```tcl
proc later {} { csv::join {a b} }
package require csv
```

## Fix

Move the requirement above the first use:

```tcl
package require csv
csv::join {a b}
```

No automatic fix is offered. The edit is a move — delete one line, insert it
elsewhere — and a code action carries a single replacement, so offering
"insert at the top" alone would leave the file requiring the package twice.

## Where it does not fire

- **The package is ambient.** An F5 surface, an EDA shell's own tool
  commands, or a package a loaded spec pack declared with `ambient_package`
  is part of the runtime, and there is no `package require` for it to be
  above.
- **The file provides the package.** A file carrying `package provide csv`
  is the implementation, and it does not have to require itself first.
- **Every requirement is conditional.** A `package require` inside an `if`
  or a `catch` has no unconditional position, so there is no "before" for
  the call to be after.
- **The document defines the command itself** — a `proc`, a class, an
  `interp alias`, a static `rename` target, an ensemble, or a declared stub.
  Then the name resolves to that definition, whatever the registry happens to
  call the same word.
- **The dialect has no `package` command** (iRules), or the file loads
  packages dynamically, so the available set is not knowable from the text.

## How to suppress

Add `# noqa: H301` on the line **above** the offending command, or turn the
code off for the workspace in the editor's diagnostic settings.

## Related

- [KCS codes index](README.md)
- [W120 — command used without a corresponding `package require`](kcs-diagnostic-w120-missing-package-require.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
