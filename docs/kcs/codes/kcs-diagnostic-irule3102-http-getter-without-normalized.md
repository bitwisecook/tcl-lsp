# KCS: IRULE3102 — Why does the analyser warn about an HTTP getter without -normalized?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default, dialect:irule

## Question

Why does the analyser flag `HTTP::path`, `HTTP::uri`, or `HTTP::query` used without `-normalized`?

## Why

Non-normalised values allow URL evasion attacks using double-encoding, dot-dot sequences, or null bytes.

## Symptoms

- A squiggle appears under the getter call, with the message "HTTP getter used without -normalized".

## Example that triggers it

```tcl
if {[string match "*admin*" [HTTP::path]]} {
  reject
}
```

The analyser reports **`IRULE3102`** because `HTTP::path` is called without `-normalized`.

## Fix

Add the `-normalized` flag:

```tcl
if {[string match "*admin*" [HTTP::path -normalized]]} {
  reject
}
```

## How to suppress

Add `# noqa: IRULE3102` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `IRULE3101`
