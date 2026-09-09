# KCS: IRULE3002 — Why does the analyser warn about tainted data in an HTTP header?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default, dialect:irule

## Question

Why does the analyser flag user-controlled data used in an HTTP header or cookie value?

## Why

Injecting CRLF or other control characters into headers enables header injection and response splitting attacks.

## Symptoms

- A yellow squiggle appears under the tainted argument, with the message
  "Tainted variable $val in HTTP header/cookie value (HTTP::header replace); risk
  of header injection".

## Example that triggers it

```tcl
when HTTP_RESPONSE {
  set val [HTTP::header value X-Custom]
  HTTP::header replace X-Reply $val
}
```

The analyser reports **`IRULE3002`** because `val` carries tainted data into a response header.

## Fix

```tcl
when HTTP_RESPONSE {
  set val [HTTP::header value X-Custom]
  set safe [URI::encode $val]
  HTTP::header replace X-Reply $safe
}
```

`URI::encode` marks its result free of carriage returns and newlines, which
clears the finding. Hand-rolled stripping with `string map` does not: the
analyser cannot tell which characters it removed.

## How to suppress

Add `# noqa: IRULE3002` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `IRULE3001`, `T100`, `T102`
