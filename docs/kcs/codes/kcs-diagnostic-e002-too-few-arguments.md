# KCS: E002 — Why does the analyser say a command has too few arguments?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why do I see a red squiggle saying a command was called with too few arguments?

## Why

Calling a command with fewer arguments than it requires will always raise a runtime error. Catching this statically prevents unexpected failures in production.

## Symptoms

- A red squiggle appears under the command, with the message "too few arguments for 'puts'".

## Example that triggers it

```tcl
puts
```

The analyser reports **`E002`** on the bare `puts` token.

## Fix

```tcl
puts "hello"
```

Supply the required arguments so the command can execute successfully.

## How to suppress

Add `# noqa: E002` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `E001`, `E003`
