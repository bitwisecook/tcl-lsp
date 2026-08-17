# KCS: E004 — Why does the analyser say my `if` is malformed?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why do I see a red squiggle saying my `if` statement is malformed, or has extra words after `else`?

## Why

Tcl's `if` command has a specific shape: a condition, an optional `then`
keyword, and a body — optionally repeated after `elseif`, and optionally
finished with an `else` (or an unlabelled trailing) body:

```
if expr1 ?then? body1 ?elseif expr2 ?then? body2 ...? ?else? ?bodyN?
```

Any call that doesn't fit this shape — a condition with nothing after it, a
dangling `elseif`/`else` with no body, or words trailing the final body — will
always raise a runtime error, so the analyser catches it statically. `E004`
reports the *exact* piece that's missing or extra, and anchors the squiggle
tightly on it rather than the whole `if` statement.

**A leading `else` or `elseif` is not itself an error.** `if else {a}` is
structurally well-formed — `else` there is just the condition text (an
ill-typed one: Tcl will fail evaluating it as a boolean expression, a
different problem `E004` doesn't cover), not a keyword. `then`, `elseif`, and
`else` are only ever meaningful in the specific positions the shape above
describes.

## Symptoms

- A squiggle on the condition, `then`, `elseif`, or `else` word, with a
  message like `No script following "1" argument` or `No expression after
  "elseif" argument`.
- A squiggle on the word(s) trailing the last recognised body, with the
  message `Extra words after "else" clause in "if" command`.

## Example that triggers it

```tcl
if {$x}
```

The analyser reports **`E004`** — `No script following "$x" argument` — on
the condition: there's no body to run.

```tcl
if {$x} {puts yes} else extra
```

The analyser reports **`E004`** — `Extra words after "else" clause in "if"
command` — on the unexpected word `extra`.

## Fix

For a missing body, add one:

```tcl
if {$x} {puts yes} else {puts no}
```

For extra trailing words, either wrap them into the final body or remove
them, depending on what you meant:

```tcl
if {$x} {puts yes} else {puts no; puts also-no}
```

Most editors offer a quick fix for both directions: **merge trailing words
into the body** (for extra words) or **remove incomplete trailing clause**
(for a dangling `elseif`/`else` with nothing after it). Neither fix is
offered when the very first clause (the initial condition + body) never
completed — there's no well-formed prefix to fall back to, so the editor
won't guess a body for you.

## How to suppress

Add `# noqa: E004` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `E001`, `E002`, `E003`
