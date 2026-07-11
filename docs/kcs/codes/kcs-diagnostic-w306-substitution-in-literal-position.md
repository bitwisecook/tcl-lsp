# KCS: W306 — Can a substitution in a literal-expected position cause issues?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn about a substitution in a position that expects a literal?

## Why

A regexp pattern or class name undergoes unintended variable or command substitution, which can alter the match semantics or execute code.

## Symptoms

- A yellow squiggle appears under the argument, with the message "substitution in literal-expected argument position".

## Example that triggers it

```tcl
regexp "test$x" $string
```

The pattern mixes a literal with a substituted variable inside double quotes.
If the user meant `$` as the regex end-of-line anchor (or `$x` as a literal),
the substitution will silently produce the wrong pattern.  The analyser
reports **`W306`** on the pattern argument.

## Not flagged

A pattern that is *exactly one* variable reference — whether bare
(``$var`` / ``${var}``) or wrapped in double quotes (``"$var"`` /
``"${var}"``) — is the canonical Tcl idiom for a parameterised regex and
is **not** flagged:

```tcl
regexp $pattern $string          ;# OK — single bare $var
regexp ${ns::pattern} $string    ;# OK — single bare ${var}
regexp "$pattern" $string        ;# OK — quotes group nothing here
```

The quoted form ``"$pattern"`` is byte-for-byte identical at runtime to
the bare ``$pattern`` (the quotes surround a single substitution and group
no literal text), so it is the same idiom, not a foot-gun.  There is no
equivalent ``{...}``-braced form for these (bracing would suppress the
substitution), so flagging them would be a false positive.

The moment literal text is concatenated with the substitution
(``"prefix$pat"``) a literal *was* expected in that word, and the warning
fires again.

## Still flagged: bare ``[cmd]`` patterns

Bare command substitutions like ``[a-z]`` are still flagged.  This is the
classic Tcl foot-gun: the user intends a regex character class, but Tcl
parses ``[a-z]`` as command substitution (calling a command named
``a-z``).  Catching that confusion is exactly the purpose of W306, so the
exemption above does **not** apply to ``[cmd]`` patterns.

## Fix

When the pattern is meant to be a literal, brace it:

```tcl
regexp {^hello$} $string
```

When the pattern is genuinely a parameter, use a single ``$var``
reference (bare or as the sole content of a quoted word) rather than
concatenating it with literal pattern text.

## How to suppress

Add `# noqa: W306` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W100`, `W303`
