# KCS: T100 — Why does the analyser warn about tainted data in a code-execution sink?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, taint

## Profiles

default, irule

## Question

Why does the analyser flag user-controlled data flowing into `eval`, `expr`, `exec`, `uplevel`, or `subst`?

## Why

User-controlled input that reaches eval, exec, or similar sinks can execute arbitrary code, compromising the system.

## Symptoms

- A yellow squiggle appears under the sink command, with the message "tainted data flows into code-execution sink".

## Example that triggers it

```tcl
set uri [HTTP::uri]
eval $uri
```

The analyser reports **`T100`** because `uri` carries tainted data into `eval`.

## Fix

```tcl
set uri [HTTP::uri]
# Validate or sanitise the input; avoid passing it to eval.
if {$uri in $allowed_commands} {
    eval $uri
}
```

## How to suppress

Add `# noqa: T100` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `T101`, `T102`, `W300`
