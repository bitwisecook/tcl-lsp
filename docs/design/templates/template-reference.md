# <The subject, in a noun phrase>

<Lead paragraph, with no heading above it. Say what this page is a
reference for, where in the pipeline it applies, and when someone
reaches for it — "Read this when debugging variable versioning or a
missing phi node" is the shape.
[`../compiler/ssa-construction.md`](../compiler/ssa-construction.md) and
[`../compiler/data-structure-reference.md`](../compiler/data-structure-reference.md)
are the worked examples. Do not open with `## Symptom`, `## Overview`,
or `## Operational context`.>

<Optionally follow the lead with a short source list, which is how
several compiler references carry their anchors instead of a
`## File-path anchors` section further down. Use one form or the other,
never both.>

Source:

- `rust/<crate>/src/<module>.rs` — <what lives here>

## <A heading named after the subject>

<A reference page is mostly body sections named after what they
describe: the data structures, the algorithm, the version matrix, the
worked examples. Sub-headings (`###`) carry the individual entries.
Everything below is optional — keep only what you have something true to
write in.>

### <Entry>

<Definition, invariants, and a worked example where one clarifies. Keep
each example to one screen.>

## Decision rule

<Use this when the reference has one load-bearing rule a reader must not
miss — the thing that is easy to get wrong. Numbered
`## Decision rules / contracts` when there are several. Delete it when
the page is pure reference and imposes no obligation.>

## Failure modes

<What goes wrong when the reference is misread or the invariant is
broken. Delete if the page names no failure.>

- <Failure mode 1>
- <Failure mode 2>

## Test anchors

<The tests that pin the behaviour described here. Name real tests;
delete this section rather than guess.>

- `rust/<crate>/tests/<file>.rs` — <what it pins>

## Related docs

<`## Related docs` is the closing-pointer heading used under
`docs/design/compiler/`. Use `## Discoverability` instead if the page is
filed under `docs/design/contracts/`.>

- [design docs index](../README.md)
- [compiler design index](../compiler/README.md)
- [glossary](../../GLOSSARY.md)
- <neighbouring doc, with a clause saying what it answers that this one
  does not>
