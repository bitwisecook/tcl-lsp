# KCS: W103 — Does open with a pipeline prefix execute shell commands?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn when `open` is called with a `|` pipeline prefix?

## Why

A pipe prefix in the path executes shell commands under attacker control, enabling arbitrary command execution on the host.

## Symptoms

- A yellow squiggle appears under the `open` call, with the message "open with pipeline prefix".

## Example that triggers it

```tcl
open "|$cmd"
```

The analyser reports **`W103`** on the `open` call.

## Fix

```tcl
open $validated_path
```

Validate that the path does not begin with `|` before passing it to `open`.

## How to suppress

Add `# noqa: W103` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W300`, `W313`
