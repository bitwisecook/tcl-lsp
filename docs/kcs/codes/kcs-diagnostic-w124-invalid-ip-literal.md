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

An IPv4 octet above 255 is not an address at all — any networking API rejects
it, and it is almost always a typo. A leading zero is worse than useless: some
resolvers read `010` as octal 8.

## Symptoms

- A red squiggle under the address literal, with the message "IPv4 octet 4
  (256) exceeds 255 — this is not a valid IP address."
- A yellow squiggle and "IPv4 octet 4 (01) has a leading zero — may be
  interpreted as octal in some contexts." for the leading-zero case.

## Example that triggers it

```tcl
set addr "10.0.0.256"
puts $addr
```

The analyser reports **`W124`** on the address literal, at Error severity.

## Fix

```tcl
set addr "10.0.0.255"
puts $addr
```

Correct the octet. The check traces the literal through assignments, so it
also finds an address built up in a variable and used later.

## How to suppress

Add `# noqa: W124` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W121`
