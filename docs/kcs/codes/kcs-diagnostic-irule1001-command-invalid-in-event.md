# KCS: IRULE1001 — Why does the analyser flag a command as invalid in this event?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default, dialect:irule

## Question

Why does the analyser report that a command is not valid in this event context?

## Why

A command only works in events that give it the data and the connection state it needs. Called elsewhere it returns nothing useful, or raises an error.

## Symptoms

- A yellow squiggle appears under the command. The message names the event and
  the reason: "'HTTP::respond' may not work in RULE_INIT: transport is none,
  needs tcp; requires profile FASTHTTP or HTTP." A command with a closed set of
  legal events reads "'…' cannot be used in RULE_INIT. Available in: …"
  instead.

## Example that triggers it

```tcl
when RULE_INIT {
  HTTP::respond 200
}
```

The analyser reports **`IRULE1001`** on `HTTP::respond`: `RULE_INIT` runs once
at rule load, with no connection and no HTTP profile.

## Fix

Move the command to an event that carries the connection it needs:

```tcl
when HTTP_REQUEST {
  HTTP::respond 200
}
```

## How to suppress

Add `# noqa: IRULE1001` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE1002`, `IRULE1003`
