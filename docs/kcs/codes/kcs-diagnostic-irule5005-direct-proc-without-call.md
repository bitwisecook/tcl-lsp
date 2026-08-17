# KCS: IRULE5005 — Why does the analyser warn about a direct proc invocation without call?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default, dialect:irule

## Question

Why does the analyser flag a proc invoked directly instead of via `call`?

## Why

iRules procs must be invoked via `call` to maintain the execution model; direct invocation bypasses event context.

## Symptoms

- A squiggle appears under the proc name, with the message "direct proc invocation without call".

## Example that triggers it

```tcl
proc helper {} {
  return 1
}
when HTTP_REQUEST {
  helper
}
```

The analyser reports **`IRULE5005`** because `helper` is invoked directly instead of via `call`. The same rule applies when one iRules proc invokes another proc.

## Fix

Use `call` to invoke the proc:

```tcl
when HTTP_REQUEST { call helper }
```

Inside another proc, use the same form:

```tcl
proc caller {} { call helper }
```

## How to suppress

Add `# noqa: IRULE5005` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE5006`, `IRULE5007`
