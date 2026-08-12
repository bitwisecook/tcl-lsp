# KCS: <CODE> — <plain-English question>

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, <compiler-pass-tag>

<!--
  - `all-editors` expands to the full LSP editor set.
  - `optimisation` is the content tag. Write it out: the help database
    is built from `docs/kcs/features/` only, so nothing indexes a page
    in this directory and no tag is added for you.
  - `<compiler-pass-tag>` is the pass that produces this rewrite:
    `gvn`, `dce`, `licm`, `const-fold`, `code-sinking`, `tail-call`,
    `unused-procs`, `instcombine`, `strength-reduce`, and so on. See
    docs/kcs/STYLE.md rule 11 for the full vocabulary.
  - Nothing validates these tags, so an unrecognised one is indexed
    rather than rejected. Check each against the rule 11 tables.
-->

## Profiles

<Which optimiser profiles enable this rewrite, as a comma-separated
list. The five tiers, in order, are `off`, `readability`, `standard`,
`full`, and `aggressive`; `readability` is the default in the editor.
List from the lowest tier that enables the code up to `full` — for
example `readability, standard, full`, `standard, full`, or `full`.
`aggressive` is left off the line because it enables everything `full`
does and differs only by running to a multi-pass fixpoint.

Membership follows the code's *category*, not the code itself: the
category is declared on the code's row in
`rust/tcl-core-types/src/diag_code.rs`, and
`rust/tcl-compiler/src/optimiser/profiles.rs` maps categories to tiers
(`standard` is `readability` plus constant folding and pattern
recognition; `full` and `aggressive` are everything). Read the category
off the row rather than guessing from the code number.

If the rewrite is safety-gated — skipped when taint or shimmer
conditions are not met — say so under "Safety conditions" below, not
here.>

## Question

What does `<CODE>` rewrite, and when does it fire?

## Why

<Simple, plain-English explanation of why this rewrite exists. Write
it as if explaining the benefit to a colleague: "Braced expressions
compile to bytecode; unbraced ones are re-parsed on every call,
which is slower and risks double substitution." One or two sentences.
No jargon without a glossary link.>

## Before

```tcl
# Input the optimiser sees. Keep it to one screen and use realistic
# names, not `foo`/`bar`.
```

## After

```tcl
# The rewritten output the optimiser produces.
```

## Safety conditions

<When this optimisation is skipped. Link each technical term to the
glossary on first use. Common reasons, for reference — write only the
ones that actually apply to this code:>

- <a variable is [shimmered](../../GLOSSARY.md#shimmer);>
- <a basic block contains a [barrier](../../GLOSSARY.md#barrier)
  (`upvar`, `eval`, a call to a proc with unknown side-effects);>
- <[taint colours](../../GLOSSARY.md#taint-colour) require sanitisation;>
- <the rewrite would change observable script results (for example, a
  top-level `set` whose value is printed).>

## How to disable

<Say only what you have checked. The scopes, smallest first:

  - **One command** — `# noqa: <CODE>` on its own comment line
    **immediately above** the command. Comments attach forward, so a
    trailing `;# noqa` at the end of the flagged line does not silence
    that line, and a blank line between the comment and the command
    detaches the directive.
  - **One code** — `tclLsp.optimiser.<CODE>` set to `false`. Every
    O-code has a generated key; confirm this one by searching
    `editors/vscode/package.json` for `tclLsp.optimiser.<CODE>`.
  - **A whole tier** — `tclLsp.optimiser.profile`, which turns off every
    category the chosen tier does not enable.
  - **Everything** — `tclLsp.optimiser.enabled` set to `false`, the
    master switch over all O-codes.

Then link the [optimiser feature](../features/kcs-feature-optimiser.md)
page for how to pick a profile.>

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [<compiler-pass>](../../GLOSSARY.md#<pass-anchor>) — the pass that
  produces this rewrite. `<pass-anchor>` is the glossary heading's own
  anchor, which is usually **not** the tag name: the `const-fold` tag's
  entry is `#constant-folding`, `tail-call` is
  `#tail-call-optimisation`, `strength-reduce` is `#strength-reduction`,
  and `unused-procs` is `#unused-procs-elimination`. Open
  [the glossary](../../GLOSSARY.md), find the heading, and copy its
  anchor. Drop this line if the pass has no glossary entry.
- Related codes: `<CODE1>`, `<CODE2>`, `<CODE3>` — pick 2-3 O-codes
  in the same family.
