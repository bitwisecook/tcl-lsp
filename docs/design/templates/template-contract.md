# Contract: <the thing that must hold, in a noun phrase>

<Lead paragraph, with no heading above it. Say what this contract
guarantees, where in the pipeline it sits, who consumes it, and when a
contributor needs to read it. Two or three sentences. If the contract
exists to prevent one specific class of breakage, name it here — that is
what makes the page findable. Do not open with `## Symptom`,
`## Overview`, or `## Operational context`.>

## <A heading named after the subject>

<The substance of the contract goes in sections named after what they
describe — `## The algorithm`, `## The cell`, `## Layers`, `## What the
index holds`. Add as many as the subject needs; this is normally most of
the page. Every section below this point is optional: keep the ones you
have something true to write in and delete the rest.>

## Decision rules / contracts

<Numbered obligations downstream code must follow. Each rule is
something a reviewer could fail a diff against — not a restatement of
the prose above. Use `## Decision rule` (singular) when there is one.
Delete this section if the contract is better expressed as prose.>

1. <Rule 1>
2. <Rule 2>
3. <Rule 3>

## File-path anchors

<Paths relative to the repository root, each with what it owns. Verify
every path exists before you commit — a stale anchor is the defect this
section exists to prevent.>

- `rust/<crate>/src/<module>.rs` — <the crate that owns the contract>
- `rust/<crate>/src/<module>.rs` — <the consuming crate>

## Failure modes

<What actually goes wrong when the contract is broken, in terms a
reviewer can recognise in a diff.>

- <Failure mode 1>
- <Failure mode 2>

## Test anchors

<The tests that fail when the contract breaks. Name real tests; delete
this section rather than guess.>

- `rust/<crate>/src/<module>.rs` — <unit tests guarding the contract>
- `rust/<crate>/tests/<file>.rs` — <integration tests>

## Discoverability

<`## Discoverability` is the closing-pointer heading used under
`docs/design/contracts/`. Use `## Related docs` instead if the page is
filed under `docs/design/compiler/`.>

- [design docs index](../README.md)
- [compiler design index](../compiler/README.md)
- <neighbouring doc, with a clause saying what it answers that this one
  does not>
