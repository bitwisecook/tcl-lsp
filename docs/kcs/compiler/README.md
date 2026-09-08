# KCS compiler-fact-contract notes

This folder holds Contributor-facing notes about a **compiler fact** — an
answer an analyser pass, diagnostic, or downstream tool reads from the
compiler (a dialect's command registry, an SSA fact, a resolved namespace
path, and so on) that a consumer must not treat as fixed for the life of the
process.

Write a note here only for something a contributor writing or maintaining a
compiler-fact consumer has to know. See rule 13 in
[`../STYLE.md`](../STYLE.md) and the "is this worth a note" guidance in
[`../README.md`](../README.md).

Every note here is a Q&A note
([`../templates/kcs-template-qa.md`](../templates/kcs-template-qa.md)) — one
question, one plain-English answer, for the **Contributor** audience — and
links out to the owning design doc under
[`../../design/compiler/`](../../design/compiler/README.md) for the full
contract. This index does not duplicate that contract.

## Notes

- [kcs-qa-is-the-command-registry-fixed-at-compile-time.md](kcs-qa-is-the-command-registry-fixed-at-compile-time.md)
  — SpecTcl packs layer commands onto a dialect's command registry at load
  time; the (profile, overlay-key) identity, the lookup-only rule and its
  fallback, workspace scope versus the per-document overlay path, and what
  that means for W002/W123 and other command-existence facts.

## See also

- [KCS index](../README.md)
- [Compiler design docs](../../design/compiler/README.md)
