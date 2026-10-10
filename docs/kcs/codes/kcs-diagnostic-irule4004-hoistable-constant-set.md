# KCS: IRULE4004 — Why does the analyser warn about a constant set in a per-request event?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, cfg

## Profiles

default, dialect:irule

## Question

Why does the analyser flag a constant assignment inside a per-request event?

## Why

Assigning a fixed value on every request wastes CPU; moving it to `RULE_INIT` or `CLIENT_ACCEPTED` runs it once.

## Symptoms

- An informational underline appears on the `set`, with the message
  "`set pool_name ...` runs on every request — consider hoisting to a
  once-per-connection event."

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

## Limits

The warning needs the assignment to be the variable's only write in the rule.
A second write in a branch the analyser proves never runs does not count, so
`set svc "foo"; if {0} { set svc "bar" }` still draws it; a second write that
can run, in this event or another, does not.

## How to suppress

Add `# noqa: IRULE4004` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE4001`, `IRULE4003`
