# KCS: W147 — Mutually exclusive options

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser

## Profiles

default

## Question

Why does the analyser say that two options cannot be used together?

## Why

Some Tcl commands accept alternative ways to select one mode. Supplying both
options is a runtime error because Tcl cannot use both modes at once. `W147`
is emitted when the analyser can prove that both literal options occur in one
leading option list.

## Symptoms

- A warning appears over the two conflicting option words.
- The message says that the options cannot be used together.

## Example that triggers it

```tcl
source -encoding utf-8 -nopkg library.tcl
```

In Tcl 9, `source -encoding ... fileName` and `source -nopkg fileName` are
separate forms. The analyser reports **`W147`** because both forms were
requested. `glob -directory root -path prefix *.tcl` is another example.

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

The check abstains when an option name is dynamic, expanded with `{*}`, or
appears after the first positional argument. It also honours the active Tcl
dialect and the command's registry-declared option set.

## How to suppress

Add `# noqa: W147` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `E002` (too few arguments), `E003` (too many arguments),
  `W004` (option unavailable in the active dialect).
