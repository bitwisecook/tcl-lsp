# Design-doc templates

Templates for writing design documents — contracts, references, and
ownership matrices. These are for technical documentation that describes
how the system is built. Technical jargon is allowed.

If you are writing a user-facing answer, a how-to, or a Q&A, use the
templates at [`docs/kcs/templates/`](../../kcs/templates/README.md)
instead.

## Available templates

- [template-contract.md](template-contract.md) — for ownership, contracts,
  and integration boundaries. Files land in
  [`../contracts/`](../contracts/command-resolution.md).
- [template-reference.md](template-reference.md) — for compact reference
  or decision pages.
- [template-matrix.md](template-matrix.md) — for producer/consumer
  ownership matrices.

A template is a starting shape, not a form to fill in. Delete every
section your document has nothing true to say in, and add the sections
your subject actually needs.

## The shape of a design doc

Read two or three of the best current documents of the kind you are
writing before you start. [`../contracts/command-resolution.md`](../contracts/command-resolution.md),
[`../compiler/error-recovery.md`](../compiler/error-recovery.md), and
[`../compiler/pass-fact-ownership-matrix.md`](../compiler/pass-fact-ownership-matrix.md)
are the reference points. What they have in common:

1. **A title that names the subject.** A noun phrase, optionally with a
   short qualifier after an em dash. Contract pages may open with
   `Contract: `. No `KCS:` prefix — that belongs to KCS notes.
2. **A lead paragraph, with no heading above it.** One or two paragraphs
   directly under the title saying what the document covers, who needs
   it, and when to read it. Do not fence this off behind `## Summary`,
   `## Overview`, `## Purpose`, `## Symptom`, or `## Operational
   context`. A design doc is not a KCS note and has no symptom.
3. **Body sections named after the subject**, not after this template.
   `## Ghosts`, `## The recovery loop`, `## Written names: colon runs and
   addressability` — headings a reader can navigate by. The substance of
   the document lives here, and it is usually most of the document.
4. **The pointers a contributor needs**: the files that implement it, the
   tests that guard it, and the neighbouring docs.

## What each optional section is for

Take a section only when your document has something true to put in it.
Over half the pages under [`../contracts/`](../contracts/lexing.md)
carry none of them, and are better for it.

- **`## Decision rules / contracts`** — numbered rules downstream code
  must follow. Real obligations, not a restatement of the prose above.
  Use the singular `## Decision rule` when there is exactly one.
- **`## File-path anchors`** — where the contract is implemented. This
  section is legitimate here and nowhere else: rule 8 of
  [`../../kcs/STYLE.md`](../../kcs/STYLE.md) bans it from KCS notes
  precisely because the design docs are its canonical home. Paths are
  relative to the repository root (`rust/tcl-lexer/src/lexer.rs`). Some
  compiler docs write the same list as a short `Source:` list in the
  lead instead; either is fine, but do not keep both.
- **`## Failure modes`** — what actually goes wrong when the contract is
  broken, in terms a reviewer can recognise in a diff.
- **`## Test anchors`** — the tests that would fail. Omit it rather than
  guess: a named test that does not exist is worse than no section.
- **A closing pointer section** — `## Discoverability` under
  [`../contracts/`](../contracts/parsing.md), `## Related docs` under
  [`../compiler/`](../compiler/README.md). Match the directory you are
  filing into.

## Before you merge

- [ ] The document opens with a lead paragraph, not a heading.
- [ ] Every path, symbol, command, and test name in it exists — check
      each one against the tree rather than against memory.
- [ ] It describes the current state. No changelog entries, no
      "Status:", no "recently changed", no milestone or phase numbers.
- [ ] Every relative link resolves and every code fence is closed.
- [ ] It is linked from [the design index](../README.md), or from a
      subdirectory `README.md` that is itself linked from there.
      `cargo xtask kcs-index-links` fails the build otherwise.
- [ ] British English, and the Oxford comma in lists of three or more.
