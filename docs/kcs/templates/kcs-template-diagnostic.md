# KCS: <CODE> — <plain-English question>

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, <compiler-pass-tag>

<!--
  - `all-editors` expands to the full LSP editor set.
  - `diagnostic` is the content tag (the filename-prefix type tag is
    added automatically; add the content tag here so readers can
    filter feature pages + code pages together).
  - `<compiler-pass-tag>` is the pass that produces this code.
    Pick the canonical tag from docs/kcs/STYLE.md rule 11 (for
    example `ssa`, `sccp`, `taint`, `cfg`, `ipa`, `lexing`,
    `lowering`, `type-infer`). If more than one pass contributes,
    list them all.
  - If the code is opt-in, also add `opt-in`. (If you add this tag,
    remember to register it in `TAG_DISPLAY` in the same change.)
-->

## Profiles

<Which diagnostic profiles enable this code. The vocabulary is
`default` (on in the default profile), `strict` (on in the strict
profile), `opt-in` (off unless the user enables the code), and
dialect restrictions like `dialect:irule` or `dialect:tcl` when the
code only fires for one dialect. Multiple profiles may apply.>

<If the code is opt-in, name the exact setting here, for example
`tclLsp.diagnostics.W242` with default `false`, and give one
sentence on why it is off by default.>

## Question

<The question a user asks when they see this code in their editor,
written exactly as they would ask it. One sentence, no jargon.>

## Why

<Simple, plain-English explanation of why this check exists. Write it
as if explaining the real-world consequence to a colleague: "A missing
variable will cause a runtime error, stopping the Tcl script with an
error." One or two sentences. No jargon without a glossary link.>

## Symptoms

- <What the user sees first: the squiggle colour, the Problems panel
  message, the exact code text the analyser prints.>
- <Any follow-on symptoms that confirm the diagnosis.>

## Example that triggers it

```tcl
# A minimal self-contained snippet the analyser flags with this code.
# Keep it to one screen and use realistic names, not `foo`/`bar`.
```

The analyser reports **`<CODE>`** on <the specific token, range, or
line — say exactly what the reader sees>.

## Fix

```tcl
# The same snippet rewritten so the analyser no longer flags it.
```

<One or two sentences on why the fix works. If there are multiple
valid fixes, show the one that is most idiomatic first and mention
the alternatives in prose.>

## How to suppress

<Inline suppression with the exact comment form the analyser
recognises, for example `# noqa: <CODE>` on the offending line. If
the code can be disabled per-file or via a setting, show the setting
key and value. If the code cannot be suppressed, say so and explain
why — "this is always-on because X".>

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [<compiler-pass>](../../GLOSSARY.md#<pass-anchor>) — the pass that
  produces this code
- Related codes: `<CODE1>`, `<CODE2>`, `<CODE3>` — pick 2-3 codes in
  the same family or that a reader searching for `<CODE>` might
  also want.
