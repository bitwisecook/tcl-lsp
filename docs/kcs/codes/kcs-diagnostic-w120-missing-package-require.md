# KCS: W120 — Why does the analyser want a package require?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn that a command is used without a corresponding `package require`?

## Why

Without an explicit `package require`, the command may not be available at runtime. Adding the require makes the dependency visible, ensures the package is loaded, and helps other tools track dependencies.

## Symptoms

- A yellow squiggle appears under the command, with the message "command 'http::geturl' used without 'package require http'".

## Example that triggers it

```tcl
set tok [http::geturl "https://example.com"]
```

The analyser reports **`W120`** on the `http::geturl` call.

## Fix

```tcl
package require http
set tok [http::geturl "https://example.com"]
```

Add the appropriate `package require` before first use of the command.

## How to suppress

Add `# noqa: W120` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W001`, `W002`
