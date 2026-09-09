# KCS: IRULE5006 — Why does the analyser warn about a top-level command in a nested body?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default, dialect:irule

## Question

Why does the analyser flag a `proc`, `when`, or `timing` command used inside a nested body?

## Why

Commands like `proc`, `when`, and `timing` must be at the iRule top level; nesting them produces undefined behaviour.

## Symptoms

- A yellow squiggle appears under the command, with the message "'proc' is only
  valid at the top level of an iRule."

## Example that triggers it

```tcl
when HTTP_REQUEST {
  if {[HTTP::path -normalized] eq "/a"} {
    proc inner {} {}
  }
}
```

The analyser reports **`IRULE5006`** because `proc` is defined inside a control
structure.

## Fix

Declare the proc at the top level and `call` it from the event:

```tcl
proc inner {} {}

when HTTP_REQUEST {
  if {[HTTP::path -normalized] eq "/a"} {
    call inner
  }
}
```

## How to suppress

Add `# noqa: IRULE5006` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE5005`, `IRULE5007`
