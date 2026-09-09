# KCS: IRULE2001 — Why does the analyser flag `matchclass` as deprecated?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default, dialect:irule

## Question

Why does the analyser report that `matchclass` is deprecated?

## Why

`matchclass` was removed after BIG-IP v10. It does not exist on current platforms and will raise a runtime error.

## Symptoms

- The `matchclass` token is struck through and carries a yellow squiggle, with
  the message "'matchclass' is deprecated since BIG-IP v10. Use
  'class match <item> <operator> <class>' instead."

## Example that triggers it

```tcl
when HTTP_REQUEST {
  set data [HTTP::host]
  matchclass $data ::hosts
}
```

The analyser reports **`IRULE2001`** on the `matchclass` token.

## Fix

Use `class match`:

```tcl
when HTTP_REQUEST {
  set data [HTTP::host]
  class match -- $data equals ::hosts
}
```

The editor offers **Replace with 'class match'** as a review-required code
action: the rewrite supplies the `equals` operator that the two-word
`matchclass` form left implicit, so check the result before accepting it.

## How to suppress

Add `# noqa: IRULE2001` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE2002`, `IRULE2003`
