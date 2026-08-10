# KCS: IRULE1004 — Why is an omitted event priority accepted?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default, dialect:irule

## Question

Why does a `when` block without an explicit priority not show a diagnostic?

## Why

BIG-IP gives a `when` handler priority **500** when the clause is omitted. The
handler is valid, so the analyser does not report `IRULE1004` for ordinary
iRules. Add an explicit priority when your deployment needs a deliberate order
between multiple rules.

## Symptoms

- No squiggle appears for a valid handler that relies on BIG-IP's default.

## Valid default-priority example

```tcl
when HTTP_REQUEST { pool main }
```

The analyser accepts this and BIG-IP runs it at priority 500.

## When to make the priority explicit

Use an explicit priority when the order matters:

```tcl
when HTTP_REQUEST priority 500 { pool main }
```

`IRULE1004` remains available for a dialect whose command-registry policy
requires explicit priorities. The standard BIG-IP policy does not enable that
rule, because an omitted value has defined runtime behaviour.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE1001`, `IRULE1002`
