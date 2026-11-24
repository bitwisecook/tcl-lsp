# KCS: IRULE1009 — Which priorities can an iRules event handler use?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default, dialect:irule

## Question

Why is the priority on my `when` handler underlined?

## Why

BIG-IP rejects a rule with an invalid handler priority. Use an integer
from **0 to 1000**, inclusive.

## Symptoms

- IRULE1009 underlines the invalid priority value.

## Example that triggers it

```tcl
when HTTP_REQUEST priority 1001 {
    pool main
}
```

## Fix

```tcl
when HTTP_REQUEST priority 1000 {
    pool main
}
```

If the default priority of **500** works for your rule, omit the clause.

## How to suppress

Put `# noqa: IRULE1009` directly above the `when` command.
[Diagnostic suppression](../kcs-howto-suppress-diagnostics.md)

## Related

- [KCS codes index](README.md)
- [IRULE1004 and the default priority](kcs-diagnostic-irule1004-missing-priority.md)
