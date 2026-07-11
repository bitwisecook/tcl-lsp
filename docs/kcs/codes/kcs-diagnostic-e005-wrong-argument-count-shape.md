# KCS: E005 — Why does the analyser say a command has the wrong argument shape?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why do I see a red squiggle saying a command's argument count is wrong, even
though it looks like it's "in range" — neither obviously too few nor too many?

## Why

Some commands don't just require a *minimum* and *maximum* argument count —
they require the arguments to come in **pairs**, or in some other fixed
shape, and a count that satisfies the range but breaks the pairing still
fails at runtime with "wrong # args". `E002`/`E003` catch a count that's
outright too low or too high; `E005` catches a count that's individually
"enough" but doesn't fit the command's key/value-pair or paired-argument
shape — an odd `dict create` tail (an unpaired key with no value), an
unpaired `foreach` var-list (a var-list with no source list), or a `switch`
pattern with no body.

## Symptoms

- A red squiggle appears under the whole command, with a message like "wrong
  argument-count shape for 'dict create': expected 0, 2, 4, …, got 3".
- The count itself doesn't look obviously wrong — it's not caught by
  `E002`/`E003`, which only check the overall minimum/maximum.

## Example that triggers it

```tcl
dict create a
```

The analyser reports **`E005`**: `dict create`'s `?key value ...?` tail must
be an even count (0, 2, 4, …) — one key with no value is a genuine Tcl
runtime error ("wrong # args").

```tcl
foreach x $list y {puts $x}
```

The analyser reports **`E005`**: `foreach`'s `varList list ?varList list
...?` pairs must be complete before the trailing `body` — `y` here has no
source list to iterate.

```tcl
switch $s a b c
```

The analyser reports **`E005`**: a flat (non-braced) `switch` needs complete
`pattern body` pairs — `c` is a pattern with no body.

## Commands this check covers

- `dict create` / `dict replace` — an even number of `key value` words.
- `dict update` — an even total (the dictionary variable, one or more `key
  varName` pairs, and the trailing body).
- `foreach` — an odd total (one or more `varList list` pairs, and the
  trailing body).
- `switch` — an odd total for the flat `pattern body ...` form, **or**
  exactly 2 total (the subject plus a single braced blob) for the
  shorthand form (`switch $s {pattern body ...}`). Both are valid; only a
  count matching neither is flagged.

A `switch` whose only remaining word is a single *bareword* (not actually
braced) is genuinely invalid Tcl too, but reads the same as the valid
shorthand by argument count alone — the analyser can't tell them apart
without evaluating the braces, so it stays silent on that one narrow shape
rather than risk a false positive.

`{*}`-expanded arguments make the true final count unknowable at analysis
time, so `E005` — like `E002` — never fires when the tail is expanded.

## Fix

```tcl
dict create a b
```

Complete the pair (or pairs) so the argument count fits the command's shape.

## How to suppress

Add `# noqa: E005` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `E002` (too few arguments overall), `E003` (too many
  arguments overall)
