# KCS: W106 — Why is an unbraced switch body dangerous?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why does the analyser warn about an unbraced `switch` arm body?

## Why

An unbraced arm body is substituted before `switch` runs it, so the code that
executes is not the code you wrote, and the arm cannot be byte-compiled.

The finding is an Error when the body substitutes, or when the `switch` uses
`-regexp` — there the patterns are substituted too.

## Symptoms

- A yellow squiggle under the arm body, with the message "switch body should
  be braced to prevent accidental substitution. Use braces: { … }".
- A red squiggle and "switch body is not braced — contains substitutions that
  risk code injection" when the body substitutes, or "switch -regexp body is
  not braced — patterns and actions undergo extra substitution, risking code
  injection" under `-regexp`.

## Example that triggers it

```tcl
set x a
switch $x a "puts A" b "puts B"
```

The analyser reports **`W106`** on each quoted arm body.

## Fix

```tcl
set x a
switch $x {
    a {puts A}
    b {puts B}
}
```

Brace every arm body. Bracing the whole pattern–body list as well keeps the
patterns literal.

## How to suppress

Add `# noqa: W106` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W100`, `W105`
