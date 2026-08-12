# KCS: IRULE3101 — Why does the analyser warn about a URI path without a leading slash?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default, dialect:irule

## Question

Why does the analyser flag an `HTTP::uri` or `HTTP::path` set to a value not provably starting with `/`?

## Why

A relative path breaks HTTP routing and can redirect traffic to unintended destinations.

## Symptoms

- A squiggle appears under the `HTTP::uri` or `HTTP::path` setter, with the message "URI path does not start with /".

## Example that triggers it

```tcl
HTTP::uri "newpath"
```

The analyser reports **`IRULE3101`** because the value `"newpath"` does not begin with `/`.

## Fix

Prefix the path with a forward slash:

```tcl
HTTP::uri "/newpath"
```

## How to suppress

Add `# noqa: IRULE3101` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `IRULE3102`
