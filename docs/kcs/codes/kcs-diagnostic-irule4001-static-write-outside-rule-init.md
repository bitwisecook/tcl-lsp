# KCS: IRULE4001 — Why does the analyser warn about writing a static variable outside RULE_INIT?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default, dialect:irule

## Question

Why does the analyser flag a write to a `static::` variable outside `RULE_INIT`?

## Why

`static::` variables persist across connections on the executing TMM. A runtime update changes that worker's value and does not propagate to other TMMs.

## Symptoms

- A yellow squiggle appears under the `set`, with the message "Writing to
  'static::counter' outside RULE_INIT changes persistent state for connections
  on this TMM. The update is not propagated to other TMMs."

## Example that triggers it

```tcl
when HTTP_REQUEST {
  set static::counter [expr {$static::counter + 1}]
}
```

The analyser reports **`IRULE4001`** because `static::counter` is written in `HTTP_REQUEST`.

## Fix

Initialise in `RULE_INIT` and use local variables for per-request data:

```tcl
when RULE_INIT {
  set static::counter 0
}
```

## How to suppress

Add `# noqa: IRULE4001` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE4002`, `IRULE4005`
