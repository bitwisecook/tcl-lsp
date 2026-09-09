# KCS: IRULE3001 — Why does the analyser warn about tainted data in an HTTP response body?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default, dialect:irule

## Question

Why does the analyser flag user-controlled data embedded in an `HTTP::respond` body?

## Why

User-controlled input in the response body can inject HTML or JavaScript, enabling cross-site scripting (XSS).

## Symptoms

- A yellow squiggle appears under the tainted argument, with the message
  "Tainted variable $host in HTTP response body (HTTP::respond); risk of XSS or
  content injection".

## Example that triggers it

```tcl
when HTTP_REQUEST {
  set host [HTTP::host]
  HTTP::respond 200 content "<h1>$host</h1>"
}
```

The analyser reports **`IRULE3001`** because `host` carries tainted data into the response body.

## Fix

```tcl
when HTTP_REQUEST {
  set host [HTTP::host]
  set safe [htmlencode $host]
  HTTP::respond 200 content "<h1>$safe</h1>"
}
```

`htmlencode` (equally `HTML::encode`) marks its result HTML-escaped, which
clears the finding. Hand-rolled escaping with `string map` does not: the
analyser has no way to tell a complete escape from a partial one.

## How to suppress

Add `# noqa: IRULE3001` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `IRULE3002`, `T100`, `T101`
