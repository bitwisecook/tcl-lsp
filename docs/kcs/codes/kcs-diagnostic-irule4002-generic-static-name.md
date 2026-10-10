# KCS: IRULE4002 — Why does the analyser warn about a generic static variable name?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default, dialect:irule

## Question

Why does the analyser flag a `static::` variable with a generic name?

## Why

Names like `static::debug` or `static::timeout` collide with identically named variables in other iRules on the same executing TMM. Each TMM has its own static cells; identical names do not create one shared cell across TMMs.

## Symptoms

- A yellow squiggle appears under the variable name, with the message
  "'static::debug' is a generic name that can collide with other iRules.
  static:: names share storage across rule owners on each executing TMM — prefix
  with the application or rule name (e.g. 'static::<app>_debug')."

## Example that triggers it

```tcl
when RULE_INIT { set static::debug 0 }
```

The analyser reports **`IRULE4002`** because `debug` is too generic and risks a collision.

## Fix

Prefix the variable with the iRule name or a unique namespace:

```tcl
when RULE_INIT { set static::myirule_debug 0 }
```

## Limits

A generic `static::` name written only in a branch the analyser proves never
runs is not reported; the warning lands on the first write that can run.

## How to suppress

Add `# noqa: IRULE4002` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE4001`, `IRULE4005`
