# KCS: IRULE1202 — Why does the analyser flag multiple respond or redirect calls?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, dataflow

## Profiles

default, dialect:irule

## Question

Why does the analyser report multiple `HTTP::respond` or `HTTP::redirect` calls on different branches?

## Why

Only one response wins. The losing branch's response is silently discarded, which masks logic errors and produces unexpected behaviour for clients.

## Symptoms

- A squiggle appears on the second respond or redirect call, with the message "multiple respond/redirect on branches".

## Example that triggers it

```tcl
if {$cond} { HTTP::respond 403 } else { HTTP::redirect "https://example.com" }
```

The analyser reports **`IRULE1202`** because both branches issue a response.

## Fix

Ensure each execution path issues at most one response:

```tcl
if {$cond} { HTTP::respond 403 } else { pool fallback_pool }
```

## How to suppress

Add `# noqa: IRULE1202` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE1201`
