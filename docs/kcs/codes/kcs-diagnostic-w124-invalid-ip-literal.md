# KCS: W124 — Why does the analyser flag a malformed IP address?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why does the analyser warn about a malformed IP address literal?

## Why

A malformed IP address (wrong number of octets, illegal characters, or invalid group structure) will be rejected at runtime by any networking API and almost always indicates a typo or formatting error.

## Symptoms

- A yellow squiggle appears under the address literal, with the message "malformed IP address literal".

## Example that triggers it

```tcl
set addr "10.0.0"
```

The analyser reports **`W124`** on the incomplete address.

## Fix

```tcl
set addr "10.0.0.1"
```

Provide a well-formed IPv4 or IPv6 address.

## How to suppress

Add `# noqa: W124` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W121`
