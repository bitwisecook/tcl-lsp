# KCS: IRULE4005 — Why does the analyser warn about a race on a static variable?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default, dialect:irule

## Question

Why does the analyser flag a `static::` variable that is written and read across events?

## Why

One connection writes while another reads, producing unpredictable values under concurrent traffic.

## Symptoms

- A squiggle appears under the `static::` variable, with the message "potential race on static:: variable".

## Example that triggers it

```tcl
when HTTP_REQUEST {
  incr static::count
}
when HTTP_RESPONSE {
  if {$static::count > 100} { log local0. "threshold" }
}
```

The analyser reports **`IRULE4005`** because `static::count` is written in one event and read in another.

## Fix

Initialise in `RULE_INIT` and keep other events read-only:

```tcl
when RULE_INIT { set static::count 0 }
when HTTP_REQUEST { log local0. "count: $static::count" }
```

## How to suppress

Add `# noqa: IRULE4005` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- Related codes: `IRULE4001`, `IRULE4002`
