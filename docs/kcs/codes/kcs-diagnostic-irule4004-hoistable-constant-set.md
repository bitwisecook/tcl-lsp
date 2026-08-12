# KCS: IRULE4004 — Why does the analyser warn about a constant set in a per-request event?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default, dialect:irule

## Question

Why does the analyser flag a constant assignment inside a per-request event?

## Why

Assigning a fixed value on every request wastes CPU; moving it to `RULE_INIT` or `CLIENT_ACCEPTED` runs it once.

## Symptoms

- A hint squiggle appears under the `set` call, with the message "constant set could be hoisted".

## Example that triggers it

```tcl
when HTTP_REQUEST {
  set pool_name "main_pool"
}
```

The analyser reports **`IRULE4004`** because `pool_name` is assigned the same constant on every request.

## Fix

Hoist the assignment to `RULE_INIT`:

```tcl
when RULE_INIT {
  set static::pool_name "main_pool"
}
```

## How to suppress

Add `# noqa: IRULE4004` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE4001`, `IRULE4003`
