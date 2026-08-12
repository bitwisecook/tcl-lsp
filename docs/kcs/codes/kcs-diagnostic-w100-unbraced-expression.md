# KCS: W100 — Why must expressions be braced?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why does the analyser warn about an unbraced expression in `expr`, `if`, or `while`?

## Why

Unbraced expressions undergo double substitution, which can execute arbitrary code and prevents byte-compilation. Bracing the expression makes it safe, predictable, and faster.

## Why is it sometimes an error?

The warning escalates to **Error** severity when the unbraced expression provably contains a substitution (`$var` or `[cmd]`). An unbraced expression with a substitution is evaluated twice at runtime — once by the Tcl parser and once by the consuming command — which changes behaviour and can execute attacker-controlled text. Without a substitution the finding is style-only and stays a Warning.

## Symptoms

- A yellow squiggle appears under the expression argument, with the message "unbraced expr body".

## Example that triggers it

```tcl
set x [expr $a + $b]
```

The analyser reports **`W100`** on the expression `$a + $b`.

## Fix

```tcl
set x [expr {$a + $b}]
```

Wrap the expression in braces to prevent double substitution and enable byte-compilation.

## How to suppress

Add `# noqa: W100` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W105`, `W106`, `W114`
