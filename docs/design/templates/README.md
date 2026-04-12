# Design-doc templates

Templates for writing design documents — contracts, references, and
ownership matrices. These are for technical documentation that describes
how the system is built. Technical jargon is allowed.

If you are writing a user-facing answer, a how-to, or a Q&A, use the
templates at [`docs/kcs/templates/`](../../kcs/templates/README.md)
instead.

## Available templates

- [template-contract.md](template-contract.md) — for ownership, contracts,
  and integration boundaries.
- [template-reference.md](template-reference.md) — for compact reference
  or decision pages.
- [template-matrix.md](template-matrix.md) — for producer/consumer
  ownership matrices.

## Quality bar

Design docs should include, at minimum:

- **Symptom or purpose** — why this contract exists, or what confusion it
  resolves.
- **Operational context** — where in the pipeline it sits and who consumes
  it.
- **Decision rules / contracts** — the numbered rules downstream code
  must follow.
- **File-path anchors** — the files where the contract is implemented.
- **Failure modes** — what goes wrong when the contract is broken.
- **Test anchors** — the tests that guard the contract.

Link back to the [design docs index](../README.md) from every file.
