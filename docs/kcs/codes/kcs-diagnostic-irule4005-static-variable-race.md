# KCS: IRULE4005 — Why does the analyser warn about a race on a static variable?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, ssa

## Profiles

default, dialect:irule

## Question

Why does the analyser flag a `static::` variable that is written and read across events?

## Why

One connection writes while another reads, producing unpredictable values under concurrent traffic.

## Symptoms

- A yellow squiggle appears on the write, with the message "Potential race:
  'static::myapp_hits' is written outside RULE_INIT and read in another event.
  static:: variables persist across all connections on the same virtual server;
  concurrent writes can produce unpredictable results."

## Example that triggers it

```tcl
when HTTP_REQUEST { incr static::myapp_hits }
when HTTP_RESPONSE { log local0. "$static::myapp_hits" }
```

The analyser reports **`IRULE4005`** on the write: `static::myapp_hits` is
written outside `RULE_INIT` and read from another event.

## Fix

Write `static::` only in `RULE_INIT` and keep every other event read-only:

```tcl
when RULE_INIT { set static::myapp_hits 0 }
when HTTP_REQUEST { log local0. "hits: $static::myapp_hits" }
```

For a counter that really has to move at request time, use `table incr`, which
is atomic across TMMs.

## How to suppress

Add `# noqa: IRULE4005` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- Related codes: `IRULE4001`, `IRULE4002`
