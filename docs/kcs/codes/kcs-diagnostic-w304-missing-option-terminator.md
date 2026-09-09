# KCS: W304 — Can a missing option terminator cause option injection?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn about a missing `--` option terminator?

## Why

User-controlled values starting with `-` are interpreted as options, enabling option injection that can alter the command's behaviour.

Only commands that document a `--` terminator are checked: `file delete` /
`copy` / `rename`, `exec`, `glob`, `regexp`, `regsub`, `unset`, and their
peers. `file exists` takes no options, so it is never flagged.

## Symptoms

- The **first positional argument** is marked, not the command name.
- A substituted value the analyser cannot pin down is reported as
  information: *"'file delete' parses leading '-' as options. Insert '--'
  before substituted input to reduce option-injection risk."*
- When the variable's most recent literal `set` starts with `-`, the same
  finding is raised to a warning and names the value.
- A quick fix, **Insert '--' option terminator**, is offered on the
  diagnostic.

## Example that triggers it

```tcl
file delete $path
```

The analyser reports **`W304`** on `$path`, the first positional argument
of `file delete`.

## Fix

```tcl
file delete -- $path
```

Add `--` before user-supplied arguments to prevent option injection.

## How to suppress

Add `# noqa: W304` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W300`, `W313`
