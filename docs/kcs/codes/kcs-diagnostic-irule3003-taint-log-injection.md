# KCS: IRULE3003 — Why does the analyser warn about tainted data in a log command?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default, dialect:irule

## Question

Why does the analyser flag user-controlled data passed directly to a `log` command?

## Why

An attacker who controls log content can forge log entries, confusing monitoring and incident response.

## Symptoms

- A yellow squiggle appears under the `log` call, with the message "tainted data in log command".

## Example that triggers it

```tcl
set host [HTTP::host]
log local0. "Host: $host"
```

The analyser reports **`IRULE3003`** because `host` carries tainted data into the log message.

## Fix

Strip newlines before logging:

```tcl
set host [HTTP::host]
regsub -all {\r|\n} $host {} clean
log local0. "Host: $clean"
```

## How to suppress

Add `# noqa: IRULE3003` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `IRULE3001`, `IRULE3002`, `T101`
