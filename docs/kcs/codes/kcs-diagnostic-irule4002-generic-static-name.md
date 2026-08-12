# KCS: IRULE4002 — Why does the analyser warn about a generic static variable name?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default, dialect:irule

## Question

Why does the analyser flag a `static::` variable with a generic name?

## Why

Names like `static::debug` or `static::timeout` collide with identically named variables in other iRules on the same virtual server.

## Symptoms

- A hint squiggle appears under the variable name, with the message "generic static:: variable name".

## Example that triggers it

```tcl
set static::debug 0
```

The analyser reports **`IRULE4002`** because `debug` is too generic and risks a collision.

## Fix

Prefix the variable with the iRule name or a unique namespace:

```tcl
set static::myirule_debug 0
```

## How to suppress

Add `# noqa: IRULE4002` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE4001`, `IRULE4005`
