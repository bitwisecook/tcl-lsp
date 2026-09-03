# KCS: How do I add a new analysis pass to the compiler?

> **Audience:** Contributor
> **Type:** How-To

## Applies to

all-editors

## Question

How do I add a new analysis pass to the compiler, or materially change an
existing one?

## Before you start

- You have a local checkout with the development environment set up
  (`cargo build --workspace` passes).
- You have read the
  [compiler pipeline overview](../design/compiler/compiler-pipeline-overview.md)
  and know where in the pipeline your pass should sit.
- You know which facts your pass reads (what it takes as input) and
  which facts it produces (what downstream code consumes).
- If your pass produces diagnostics, you have picked a diagnostic code
  family and a default severity.

## Answer

1. **Accept existing facts as input.** Consume a
   [compilation unit](../GLOSSARY.md#compilation-unit) or function unit
   fact. Do not re-parse, re-lex, or re-lower inside the pass — if the
   data you need is not already on the unit, extend an earlier pass to
   produce it rather than duplicating work in yours.
2. **Emit typed findings with stable ranges.** Every finding has a code,
   a message, and a primary range. For a finding that spans more than
   one site, attach the other sites as related ranges so the LSP can
   underline all of them.
3. **Keep output ordering stable.** Sort findings by range, then by
   code, before you return them. Unstable ordering makes tests flaky
   and makes diff-based debugging painful.
4. **Wire suppression and severity in the diagnostics layer.** Add the
   new diagnostic code family to the severity map, and make sure
   `# noqa` suppression works for your code the same way it works for
   every other code.
5. **Add tests.**
   - At least one direct pass test that constructs the input fact and
     asserts on the output.
   - At least one diagnostics integration test that runs the full
     pipeline and checks that your finding appears at the right line
     and column.
   - Fixture scripts under `tests/fixtures/` for any complex
     scenario you cannot express inline.
6. **Update documentation.**
   - Write a design doc for the pass under
     [`docs/design/compiler/`](../design/compiler/README.md) if one
     does not already exist.
   - Link it from
     [`docs/design/compiler/README.md`](../design/compiler/README.md)
     and from the top
     [`docs/design/README.md`](../design/README.md).
   - Link the tests that anchor the expected behaviour from the design
     doc.
7. **Run the pre-PR gate.** `make prep-pr` runs formatting, lint,
   type-check, and fast tests. Every step must pass before you open a
   PR.

## How to tell it worked

- `make prep-pr` is green.
- Your new finding shows up in the LSP for the fixture script you wrote.
- A reviewer can follow the design doc link from
  [`docs/design/compiler/README.md`](../design/compiler/README.md) and
  read about your pass without opening source files.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [Compiler pipeline overview](../design/compiler/compiler-pipeline-overview.md)
- [Downstream pass contracts](../design/compiler/downstream-pass-contracts.md)
- [Pass-fact ownership matrix](../design/compiler/pass-fact-ownership-matrix.md)
