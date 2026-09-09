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

Locals live for one connection, and events fire in a fixed order. A variable read in an event that runs before the one that sets it is simply not there yet — and a per-request value is not guaranteed to exist by the time a per-connection event runs.

## Symptoms

- A hint appears under the `set`, with the message "Variable 'user': variable
  set in HTTP_RESPONSE is not yet available in HTTP_REQUEST (fires earlier)" —
  or, for the per-request/per-connection case, "… set in HTTP_REQUEST
  (per-request) may not be set yet in CLIENT_CLOSED (per-connection)".

## Example that triggers it

```tcl
when HTTP_RESPONSE { set user [HTTP::header value Server] }
when HTTP_REQUEST { log local0. $user }
```

The analyser reports **`IRULE4003`** because `user` is read in `HTTP_REQUEST`,
which fires before the `HTTP_RESPONSE` that sets it.

## Fix

Set the variable in an event that fires first:

```tcl
when HTTP_REQUEST {
  set user [HTTP::header value User]
  log local0. $user
}
```

Where the value genuinely has to survive past the connection, put it in a
`table` entry rather than a local.

## How to suppress

Add `# noqa: IRULE4003` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE4001`, `IRULE4004`
