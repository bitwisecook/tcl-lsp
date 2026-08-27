# KCS: W147 — Mutually exclusive options

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser, command-walk

## Profiles

default

## Question

Why does the analyser say that two options cannot be used together?

## Why

Some Tcl commands accept alternative ways to select one mode. Supplying both
options is a runtime error because Tcl cannot use both modes at once. `W147`
is emitted when the analyser can prove that both are present in one call.

The exclusion may also be *directional* and may reach an option's **value** or
a positional argument, because that is how real libraries phrase it:
`struct::tree walk` rejects `-order in` together with `-type bfs` (an
in-order breadth-first walk is not a thing), and `bibtex::parse` rejects
`-channel` together with an inline text argument. All of these are one
registry relation type, checked generically.

## Symptoms

- A warning appears over the two conflicting option words.
- The message says that the options cannot be used together.

## Example that triggers it

```tcl
source -encoding utf-8 -nopkg library.tcl
```

In Tcl 9, `source -encoding ... fileName` and `source -nopkg fileName` are
separate forms. The analyser reports **`W147`** because both forms were
requested. `glob -directory root -path prefix *.tcl` is another example, and
so is:

```tcl
package require struct::tree
set t [::struct::tree]
$t walk root -order in -type bfs v script
```

which reports the library's own message, *"unable to do a in-order breadth
first walk"*.

## Fix

Choose the one mode intended for the call:

```tcl
source -encoding utf-8 library.tcl
# or:
source -nopkg library.tcl
```

No automatic fix is offered: removing either option changes the operation,
and the analyser cannot infer which intent is correct.

## Where it does not fire

The check abstains when an option name is dynamic or expanded with `{*}`, and
— for a command whose options are a leading run, which is nearly all of core
Tcl — when the word appears after the first positional argument. A command
whose parser really does read options anywhere (`http::geturl`, which takes
its URL first) declares so, and is checked accordingly. The check honours the
active Tcl dialect, the resolved package version, and the command's
registry-declared option set.

## How to suppress

Add `# noqa: W147` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [W152 — option relation unmet](kcs-diagnostic-w152-option-relation-unmet.md)
  — the "this one needs that one" half of the same relation model.
- Related codes: `E002` (too few arguments), `E003` (too many arguments),
  `W004` (option unavailable in the active dialect).
