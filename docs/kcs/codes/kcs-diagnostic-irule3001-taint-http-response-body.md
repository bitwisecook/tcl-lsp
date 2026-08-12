# KCS: IRULE3001 — Why does the analyser warn about tainted data in an HTTP response body?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

irule

## Question

Why does the analyser flag user-controlled data embedded in an `HTTP::respond` body?

## Why

User-controlled input in the response body can inject HTML or JavaScript, enabling cross-site scripting (XSS).

## Symptoms

- A yellow squiggle appears under the `HTTP::respond` call, with the message "tainted data in HTTP response body".

## Example that triggers it

```tcl
set host [HTTP::host]
HTTP::respond 200 content "<h1>$host</h1>"
```

The analyser reports **`IRULE3001`** because `host` carries tainted data into the response body.

## Fix

```tcl
set host [HTTP::host]
set safe_host [string map {& &amp; < &lt; > &gt; \" &quot;} $host]
HTTP::respond 200 content "<h1>$safe_host</h1>"
```

HTML-escape the value before embedding it in the response.

## How to suppress

Add `# noqa: IRULE3001` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `IRULE3002`, `T100`, `T101`
