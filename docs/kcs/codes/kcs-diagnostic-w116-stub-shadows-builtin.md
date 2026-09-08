# KCS: W116 — Why does the analyser warn about a stub shadowing a built-in?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn that a stub command shadows a built-in command?

## Why

A stub declares a command so the analyser knows its shape. Declaring one under
the name of a command the dialect already provides hides the real signature,
so arity and argument checks on every call now follow your stub instead.

## Symptoms

- A yellow squiggle under the stub declaration, with the message "Stub command
  'puts' shadows built-in command."

## Example that triggers it

```tcl
# tcl-lsp: stubs-begin
# tcl-lsp: stub puts {channel:channel string}
# tcl-lsp: stubs-end
```

The analyser reports **`W116`** on the stub line.

## Fix

```tcl
# tcl-lsp: stubs-begin
# tcl-lsp: stub log_puts {channel:channel string}
# tcl-lsp: stubs-end
```

Give the stub a name no built-in already uses. A stub that comes from a
`.tcl.stubs` sidecar rather than an inline block is never flagged.

## How to suppress

Add `# noqa: W116` on the line **above** the stub declaration.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W113`, `W117`
