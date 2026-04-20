# KCS: W307 — Can a non-literal command name execute anything?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn when a command name is built from a variable or substitution?

## Why

A command name built from a variable or command substitution cannot be statically verified and may execute anything, including attacker-controlled code.

## Symptoms

- A yellow squiggle appears under the command invocation, with the message "non-literal command name".

## Example that triggers it

```tcl
$computed_name $arg
```

The analyser reports **`W307`** on the command invocation.

## Fix

```tcl
switch $action {
    run  { run_cmd $arg }
    stop { stop_cmd $arg }
}
```

Use a literal command name or a validated dispatch table instead.

## How to suppress

Add `# noqa: W307` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W101`, `W123`
