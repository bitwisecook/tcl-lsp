# KCS: IRULE3002 — Why does the analyser warn about tainted data in an HTTP header?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

irule

## Question

Why does the analyser flag user-controlled data used in an HTTP header or cookie value?

## Why

Injecting CRLF or other control characters into headers enables header injection and response splitting attacks.

## Symptoms

- A yellow squiggle appears under the header command, with the message "tainted data in HTTP header value".

## Example that triggers it

```tcl
set val [HTTP::header value X-Custom]
HTTP::header replace X-Reply $val
```

The analyser reports **`IRULE3002`** because `val` carries tainted data into a response header.

## Fix

```tcl
set val [HTTP::header value X-Custom]
set safe_val [string map {"\r" "" "\n" ""} $val]
HTTP::header replace X-Reply $safe_val
```

Validate the value or strip control characters before setting the header.

## How to suppress

Add `# noqa: IRULE3002` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `IRULE3001`, `T100`, `T102`
