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

Logging on every request generates millions of log lines, overwhelming syslog and slowing the BIG-IP.

## Symptoms

- A hint squiggle appears under the `log` call, with the message "ungated log in high-frequency event".

## Example that triggers it

```tcl
when HTTP_REQUEST {
  log local0. "req: [HTTP::uri]"
}
```

The analyser reports **`IRULE5001`** because the log runs unconditionally on every request.

## Fix

Gate the log with a debug flag:

```tcl
when HTTP_REQUEST {
  if {$static::debug} { log local0. "req: [HTTP::uri]" }
}
```

The gate clears the diagnostic. What counts as one:

- Any enclosing `if`, `switch`, or loop whose condition reads a `static::`
  variable — `$static::debug`, `${static::debug}`, or
  `[info exists static::debug]`.
- Any enclosing condition that reads a variable set by an event that runs
  less often than once per request: `set debug 0` in `RULE_INIT` or in
  `CLIENT_ACCEPTED`, then `if {$debug}`. A flag the request path sets itself
  does not count — gating on per-request state is not gating.

Nested bodies inherit the gate, so a `log` deeper inside a gated branch stays
quiet. Every arm of a gating command counts, `else` included.

## How to suppress

Add `# noqa: IRULE5001` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE5002`, `IRULE4004`
