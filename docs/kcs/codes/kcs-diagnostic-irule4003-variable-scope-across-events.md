# KCS: IRULE4003 — Why does the analyser warn about variable scoping across events?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default, dialect:irule

## Question

Why does the analyser flag a variable set in one event and read in another?

## Why

A local variable set in one event may be undefined or stale when referenced in another event.

## Symptoms

- A hint squiggle appears under the variable reference, with the message "variable scope concern across events".

## Example that triggers it

```tcl
when HTTP_REQUEST { set user [HTTP::header User] }
when HTTP_RESPONSE { log local0. $user }
```

The analyser reports **`IRULE4003`** because `user` is set in `HTTP_REQUEST` but read in `HTTP_RESPONSE`.

## Fix

Use a connection-scoped table or set the variable in `CLIENT_ACCEPTED`:

```tcl
when HTTP_REQUEST { table set [IP::client_addr] [HTTP::header User] 30 }
when HTTP_RESPONSE { log local0. [table lookup [IP::client_addr]] }
```

## How to suppress

Add `# noqa: IRULE4003` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE4001`, `IRULE4004`
