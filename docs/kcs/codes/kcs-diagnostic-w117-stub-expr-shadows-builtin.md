# KCS: W117 — Why does a stub expr function shadow a built-in?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn that a stub expression function or operator shadows
a built-in?

## Why

`expr` already provides `abs`, `sin`, `cos`, and the rest, plus its operator
set. Declaring a stub under one of those names replaces the built-in's known
signature with yours, so every expression using it is checked against the
wrong shape.

## Symptoms

- A yellow squiggle under the stub declaration, with the message "Stub
  expression function 'abs' shadows built-in function." — or "Stub expression
  operator '…' shadows built-in operator." for an `expr-op` stub.

## Example that triggers it

```tcl
# tcl-lsp: stubs-begin
# tcl-lsp: stub expr-func abs 1
# tcl-lsp: stubs-end
```

The analyser reports **`W117`** on the stub line.

## Fix

```tcl
# tcl-lsp: stubs-begin
# tcl-lsp: stub expr-func clamp_abs 1
# tcl-lsp: stubs-end
```

Choose a name no built-in function or operator uses. A stub that comes from a
`.tcl.stubs` sidecar rather than an inline block is never flagged.

## How to suppress

Add `# noqa: W117` on the line **above** the stub declaration.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W113`, `W116`
