# KCS: IRULE5001 — Why does the analyser warn about an ungated log in a high-frequency event?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default, dialect:irule

## Question

Why does the analyser flag a `log` call inside a per-request event with no guard?

## Why

A `log` in a per-request event runs once per request. At production traffic that floods syslog and costs TMM time on every connection.

## Symptoms

- A hint appears under the `log` call, with the message "'log' in HTTP_REQUEST
  fires on every request. Set a debug flag in CLIENT_ACCEPTED (e.g. set debug 0)
  and gate with if {$debug} {...}."

## Example that triggers it

```tcl
when HTTP_REQUEST {
  log local0. "req: [HTTP::uri -normalized]"
}
```

The analyser reports **`IRULE5001`** on the `log`. The check is positional: it
fires for any `log` in an event the registry marks high-frequency.

## Fix

Gate the log behind a debug flag so it costs nothing in production:

```tcl
when CLIENT_ACCEPTED { set debug 0 }
when HTTP_REQUEST {
  # noqa: IRULE5001
  if {$debug} { log local0. "req: [HTTP::uri -normalized]" }
}
```

The gate is the real fix, but it does not clear the diagnostic — the analyser
does not track which condition a `log` sits under. Suppress the line once you
have gated it, or remove the `log`.

## How to suppress

Add `# noqa: IRULE5001` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE5002`, `IRULE4004`
