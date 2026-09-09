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

An unbraced expression is substituted twice — once by the Tcl parser, once by
the command that evaluates it — and cannot be byte-compiled. Bracing it makes
it safe, predictable, and faster.

The finding is an Error when the expression provably contains a substitution
(`$var` or `[cmd]`), because the second evaluation can run attacker-controlled
text. Without a substitution it is style-only and stays a Warning.

## Symptoms

- A squiggle appears under the expression argument — red when the expression
  substitutes, yellow when it does not.
- For `expr` the message is "Expression is not braced: may cause double
  substitution and prevents byte-compilation. Use expr {...} instead."; for
  another command taking an expression argument it names the command and the
  text, as in "Expression argument to 'if' is not braced: may cause double
  substitution. Use braces: {1<2}".
- A **Wrap expression in braces** quick fix on the diagnostic.

## Example that triggers it

```tcl
set a 1
set b 2
set x [expr $a + $b]
puts $x
```

The analyser reports **`W100`** on the expression `$a + $b`, at Error
severity because the expression substitutes.

## Fix

```tcl
set a 1
set b 2
set x [expr {$a + $b}]
puts $x
```

Wrap the expression in braces to prevent double substitution and enable byte-compilation.

## How to suppress

Add `# noqa: W100` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W105`, `W106`, `W114`
