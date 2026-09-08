# KCS: IRULE2002 — Why does the analyser flag a deprecated iRules command?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default, dialect:irule

## Question

Why does the analyser report that an iRules command is deprecated?

## Why

The command registry records a replacement for it. The old spelling still works, but it is frozen, so a BIG-IP upgrade can take it away.

## Symptoms

- The command is struck through and carries a yellow squiggle, with the message
  "'remote_addr' is deprecated in iRules. Use 'IP::remote_addr' instead."

## Example that triggers it

```tcl
when CLIENT_ACCEPTED {
  log local0. [remote_addr]
}
```

The analyser reports **`IRULE2002`** on `remote_addr`.

## Fix

Use the replacement the message names:

```tcl
when CLIENT_ACCEPTED {
  log local0. [IP::remote_addr]
}
```

Where the replacement takes the same arguments, the editor offers a **Replace
with '…'** code action that swaps the name for you. Where it restructures the
arguments — `matchclass`, reported as
[`IRULE2001`](kcs-diagnostic-irule2001-deprecated-matchclass.md) — no automatic
fix is offered.

## How to suppress

Add `# noqa: IRULE2002` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE2001`, `IRULE2003`
