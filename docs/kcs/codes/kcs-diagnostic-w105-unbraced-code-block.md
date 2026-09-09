# KCS: W105 — Why must code blocks be braced?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why does the analyser warn about an unbraced script argument to `eval`,
`uplevel`, `if`, or `while`?

## Why

An unbraced script argument is substituted before the command sees it, so the
text that runs is not the text you wrote. It also cannot be byte-compiled.

The finding is an Error when the block provably contains a substitution
(`$var` or `[cmd]`), because the block is then evaluated twice and the second
pass can run attacker-controlled text. Without a substitution it is style-only
and stays a Warning.

## Symptoms

- A squiggle under the script argument — red when the block substitutes,
  yellow when it does not.
- The message names the command: "Code block argument to 'uplevel' should be
  braced for clarity and to prevent accidental substitution. Use braces:
  { … }", or, for the Error case, "Code block argument to 'eval' is not braced
  and contains substitutions — risk of double substitution. Use braces:
  { … }".
- A **Wrap code block in braces** quick fix on the diagnostic.

## Example that triggers it

```tcl
proc reset_counter {} {
    uplevel 1 "set count 0"
}
```

The analyser reports **`W105`** on the quoted body.

## Fix

```tcl
proc reset_counter {} {
    uplevel 1 {set count 0}
}
```

A bare variable holding a script (`eval $cmd`) and a whole-word command
substitution (`eval [list set y $x]`) are not flagged: bracing either would
turn the reference into literal text.

## How to suppress

Add `# noqa: W105` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W100`, `W101`, `W106`
