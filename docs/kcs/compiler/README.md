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
- [kcs-qa-what-does-a-member-effect-say.md](kcs-qa-what-does-a-member-effect-say.md)
  — the closed, family-neutral vocabulary a definition-body member word
  declares (callable, dispatch redirect, state declaration, relation,
  visibility, retraction, init script); the derived `member_rows` query
  the class hierarchy, method arity, and `my` dispatch fold over; and what
  a pack's own definer family adds to declare a new one.
- [kcs-qa-what-does-a-value-transfer-declaration-say.md](kcs-qa-what-does-a-value-transfer-declaration-say.md)
  — the registry's three-state `semantics` declaration on a command,
  subcommand, or form; what the constant-propagation driver reads from it;
  which descriptors derive one; and the two separate columns — descriptor
  present, route enabled — the generated inventory keeps.
- [kcs-qa-where-does-a-clause-shape-come-from.md](kcs-qa-where-does-a-clause-shape-come-from.md)
  — the clause-grammar descriptor behind `if`, `try`, `for`, `while`, the
  `foreach` family, and `catch`; the one derived `clause_plan` query every
  consumer reads instead of matching keywords; and `clause_shape_check`,
  the escape hatch for a chain the grammar cannot spell.
- [kcs-qa-why-does-a-constant-fold-depend-on-the-dialect.md](kcs-qa-why-does-a-constant-fold-depend-on-the-dialect.md)
  — the direct route's release rules: leading-zero numerals, index
  numerals, the integer tower, and non-ASCII operands answer per release,
  and a profile naming none gets the unanimous answer or a recorded
  decline the Explorer's `sccp` view shows.
- [kcs-qa-why-does-a-regexp-sometimes-not-fold.md](kcs-qa-why-does-a-regexp-sometimes-not-fold.md)
  — the dissector's typed three-way match result, why a `Stopped` search
  declines instead of folding to no-match, and the compile/cache budgets
  that bound the work before the engine starts.
- [kcs-qa-why-does-a-renamed-command-stop-folding.md](kcs-qa-why-does-a-renamed-command-stop-folding.md)
  — a command the module shadows, renames, or aliases is no longer
  evaluated with its builtin semantics: the shared lattice declines with
  `rebinding-suspected`, the editor's memoised lattice and a standalone
  build agree, and a rewrite re-proves under the whole module's trust.

## See also

- [KCS index](../README.md)
- [Compiler design docs](../../design/compiler/README.md)
