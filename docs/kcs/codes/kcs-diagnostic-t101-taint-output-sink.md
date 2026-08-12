# KCS: T101 — Why does the analyser warn about tainted data in an output sink?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default, irule

## Question

Why does the analyser flag user-controlled data flowing into `puts`, `log`, or similar output commands?

## Why

Unsanitised user input in log or output commands can inject misleading log entries or enable log-based attacks.

## Symptoms

- A yellow squiggle appears under the output command, with the message "tainted data flows into output sink".

## Example that triggers it

```tcl
set host [HTTP::host]
log local0. $host
```

The analyser reports **`T101`** because `host` carries tainted data into `log`.

## Fix

```tcl
set host [HTTP::host]
set safe_host [string map {"\n" "" "\r" ""} $host]
log local0. $safe_host
```

Sanitise the value before logging, or use a structured log format.

## How to suppress

Add `# noqa: T101` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `T100`, `T102`
