# KCS: T103 — Why does the analyser warn about tainted data in a regexp pattern?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default

## Question

Why does the analyser flag user-controlled data used as the pattern argument of `regexp` or `regsub`?

## Why

A regular expression is a small program, and the pattern argument is where
it is written. A tainted value there lets whoever supplied it change what
the match means: `.*` where you expected a literal name matches everything,
and an alternation or a backreference can make the match succeed on input
you meant to reject.

The second hazard is cost. A pattern with nested quantifiers — `(a+)+$` is
the classic — can take exponential time on a short non-matching subject, so
an attacker who controls the pattern controls how long your process spends
in the regex engine. That is a denial of service from a single request.

Both are avoided the same way: the pattern must be something you wrote, or
something whose metacharacters have been turned into ordinary text.

## Symptoms

- A yellow squiggle under the `regexp` or `regsub` call, with the message
  "Tainted variable `$var` in regexp pattern position (`regexp`); risk of
  regex injection or ReDoS".
- Only the pattern argument is flagged. The same variable used as the
  *subject* is a different question, and this code says nothing about it.

## Example that triggers it

```tcl
set pattern [gets stdin]
if {[regexp -- $pattern $line]} {
    puts "matched"
}
```

The analyser reports **`T103`** on the `regexp` call, naming `$pattern`.

## Fix

```tcl
set pattern [gets stdin]
if {[regexp -- [regex::quote $pattern] $line]} {
    puts "matched"
}
```

`regex::quote` backslash-escapes every regex metacharacter, so the value
matches as the literal text the user typed and can neither redirect the
match nor build a pathological pattern. A quick fix ("Wrap `$var` with
`[regex::quote]`") writes that wrap and, when the helper is not already
defined in the file, inserts it.

Assigning the quoted value first works identically:

```tcl
set safe [regex::quote $pattern]
regexp -- $safe $line
```

If the user is only choosing between patterns you wrote, match the choice
against a whitelist instead and use your own pattern — that removes the
hazard rather than escaping around it.

## How to suppress

Add `# noqa: T103` on the line **above** the offending command.

To turn it off more widely than one line, use the file-level directive
`# tcl-lsp: disable=T103` in the leading comment block, or `disabled = T103`
under `[diagnostics]` in `.tcl-lsp.ini` at the workspace root.

T103 is marked internal in the diagnostic registry, which means it gets no
generated entry in your editor's settings list — there is no tick-box for it.
Setting the key by hand still works: `"tclLsp.diagnostics.T103": false`.

See [how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `T100`, `T102`, `T106`
