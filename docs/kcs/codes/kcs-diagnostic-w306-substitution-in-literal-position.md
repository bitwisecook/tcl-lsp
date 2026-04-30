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

A bare single substitution as the entire pattern is the canonical Tcl idiom
for a parameterised regex and is **not** flagged:

```tcl
regexp $pattern $string         ;# OK — single bare $var
regexp ${ns::pattern} $string   ;# OK — single bare ${var}
regexp [build_re] $string       ;# OK — single bare [cmd]
```

There is no equivalent ``{...}``-braced form for these (bracing would
suppress the substitution), so flagging them would be a false positive.

## Fix

When the pattern is meant to be a literal, brace it:

```tcl
regexp {^hello$} $string
```

When the pattern is genuinely a parameter, use a single bare ``$var``
without surrounding double quotes.

## How to suppress

Add `# noqa: W306` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W100`, `W303`
