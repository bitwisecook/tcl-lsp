# KCS: W122 — Why does the analyser flag an IPv4 address with invalid octets?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why does the analyser warn about an IPv4 octet greater than 255 or with a leading zero?

## Why

An octet above 255 is invalid and will be rejected by networking APIs. A leading zero (e.g. `010`) can be silently interpreted as octal by some parsers, producing a different address than intended.

## Symptoms

- A yellow squiggle appears under the IP literal, with the message "IPv4 octet out of range or has leading zero".

## Example that triggers it

```tcl
set addr "192.168.01.300"
```

The analyser reports **`W122`** on the address literal.

## Fix

```tcl
set addr "192.168.1.255"
```

Ensure every octet is between 0 and 255, with no leading zeroes.

## How to suppress

Add `# noqa: W122` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W121`, `W124`
