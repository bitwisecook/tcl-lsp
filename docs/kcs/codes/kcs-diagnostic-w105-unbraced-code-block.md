# KCS: W105 — Why must code blocks be braced?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why does the analyser warn about an unbraced code block or a missing variable in `namespace eval`?

## Why

An unbraced code block is substituted before the command sees it, which can cause unexpected variable resolution, break `namespace eval` scoping, and prevent byte-compilation.

## Why is it sometimes an error?

The warning escalates to **Error** severity when the unbraced code block provably contains a substitution (`$var` or `[cmd]`). An unbraced code block with a substitution is evaluated twice at runtime — once by the Tcl parser and once by the consuming command — which changes behaviour and can execute attacker-controlled text. Without a substitution the finding is style-only and stays a Warning.

## Symptoms

- A yellow squiggle appears under the code body, with the message "unbraced code block".

## Example that triggers it

```tcl
namespace eval ::foo "proc bar {} { puts hello }"
```

The analyser reports **`W105`** on the quoted body.

## Fix

```tcl
namespace eval ::foo {
    proc bar {} { puts hello }
}
```

Brace the code block so it is compiled in the correct namespace scope.

## How to suppress

Add `# noqa: W105` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W100`, `W106`
