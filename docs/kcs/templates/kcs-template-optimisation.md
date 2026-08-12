# KCS: <CODE> — <plain-English question>

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, <compiler-pass-tag>

<!--
  - `all-editors` expands to the full LSP editor set.
  - `optimisation` is the content tag (the filename-prefix type tag
    is added automatically).
  - `<compiler-pass-tag>` is the pass that produces this rewrite:
    `gvn`, `dce`, `licm`, `const-fold`, `code-sinking`, `tail-call`,
    `unused-procs`, `instcombine`, `strength-reduce`, and so on. See
    docs/kcs/STYLE.md rule 11 for the full vocabulary.
-->

## Profiles

<Which optimiser profiles enable this rewrite. The vocabulary is
`readability`, `standard`, and `full`. Most O-codes are always on
within their enabled profile. If the rewrite is safety-gated
(disabled when taint or shimmer conditions are not met), say so
here, not in the body.>

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

<When this optimisation is skipped. Common reasons include:
- a variable is [shimmered](../../GLOSSARY.md#shimmer);
- a basic block contains a [barrier](../../GLOSSARY.md#barrier)
  (`upvar`, `eval`, a call to a proc with unknown side-effects);
- [taint colours](../../GLOSSARY.md#taint-colour) require sanitisation;
- the rewrite would change observable script results (for example,
  a top-level `set` whose value is printed).

Link each technical term to the glossary on first use.>

## How to disable

<If the user can disable the rewrite per-file or globally, name the
exact setting and value. If the rewrite is always on within its
profile, say so and link to the
[optimiser feature](../features/kcs-feature-optimiser.md) page for
how to pick a different profile.>

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [<compiler-pass>](../../GLOSSARY.md#<pass-anchor>) — the pass that
  produces this rewrite
- Related codes: `<CODE1>`, `<CODE2>`, `<CODE3>` — pick 2-3 O-codes
  in the same family.
