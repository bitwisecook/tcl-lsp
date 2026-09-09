# KCS: IRULE2003 — Why does the analyser flag an unsafe iRules command?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default, dialect:irule

## Question

Why does the analyser report that a command is unsafe in iRules?

## Why

The command reaches outside the iRule's own stack frame, so it can read and rewrite state the rule does not own. TMM runs every rule in one interpreter, so that reach crosses rules.

## Symptoms

- A red squiggle appears on the command, with the message "'uplevel' is unsafe
  in iRules and may allow context escalation".

## Example that triggers it

```tcl
when HTTP_REQUEST { uplevel 1 {set x 1} }
```

The analyser reports **`IRULE2003`** on `uplevel`, which runs its script in a
caller's frame.

## Fix

Run the script in the event's own frame, and keep shared state in `static::`:

```tcl
when RULE_INIT { set static::ns "value" }
when HTTP_REQUEST { set x 1 }
```

`global` and `::`-qualified names are a different problem — they pin the
virtual server to one TMM and are reported as
[`IRULE6001`](kcs-diagnostic-irule6001-global-variable-cmp-pinning.md).

## How to suppress

Add `# noqa: IRULE2003` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE2001`, `IRULE2002`
