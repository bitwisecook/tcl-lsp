# KCS: W145 — Ambiguous keyword abbreviation

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser, command-walk

## Profiles

default

## Question

Why does the analyser warn that an abbreviated subcommand or option is
ambiguous?

## Why

Tcl lets you abbreviate an ensemble subcommand or an `-option` to **any
unique prefix** of its full spelling, so `string le` is `string length` and
`lsearch -noc` is `lsearch -nocase`. A prefix that matches *more than one*
entry is not an abbreviation at all — it is a runtime error on every path
that executes the line:

```
% string l abc
unknown or ambiguous subcommand "l": must be bytelength, cat, compare,
equal, first, index, is, last, length, map, match, range, repeat, replace,
reverse, tolower, totitle, toupper, trim, trimleft, trimright, wordend,
or wordstart
```

`l` prefixes both `last` and `length`, so tclsh cannot pick one. The
analyser reports the same thing statically, quoting only the **matching**
candidates — that is what you need to disambiguate — rather than the whole
table.

The check is version-range aware, because a prefix's uniqueness changes with
the Tcl release. `string c` meant `compare` until Tcl 8.6.2 added `string
cat`; from then on it is ambiguous. A word that is unique in every version
of the document's target range is legal code and draws nothing.

## Symptoms

- A yellow squiggle under an abbreviated subcommand or `-option` word, with
  a message like:
  "Ambiguous abbreviation 'l' for 'string': matches 'last', 'length'."

## Example that triggers it

```tcl
puts [string l $s]
```

The analyser reports **`W145`** on `l`.

## Fix

Write enough of the keyword to be unique — or all of it:

```tcl
puts [string length $s]
```

One quick fix is offered per matching candidate ("Expand to `string last`",
"Expand to `string length`"). They are **manual-pick** fixes: the tool
cannot know which one you meant, so Fix All never applies them.

## Where it does not fire

The check abstains rather than risk a false positive on legal code:

- **Unique prefixes.** `string le`, `lsearch -noc`, and every other
  unambiguous abbreviation are legal and draw nothing (the formatter can
  expand them to canonical spellings — see `tclLsp.format.expandAbbreviations`).
- **Strict tables.** A command whose dispatch opts out of prefix matching,
  and any ensemble the file configured with `namespace ensemble … -prefixes
  0`, treat an abbreviation as a plain unknown keyword — `W001` owns that.
- **Dynamic words.** `string $sub`, `string [pick]`, and `{*}`-expanded
  words are not known until run time.
- **Command names.** Tcl never prefix-matches a command name, so `str
  length` is an unknown command (`W123`), not an ambiguous abbreviation.

## How to suppress

Add `# noqa: W145` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `W001` (unknown subcommand), `W004` (option not available in
  the active dialect), `W123` (unknown command).
