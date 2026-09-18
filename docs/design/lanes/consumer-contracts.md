# Lane: consumer contracts — step 1, the document repairs

## Goal

Step 1 of [registry-consumer-contracts.md](../compiler/registry-consumer-contracts.md)
§ *Build order*, documents only: take the four rulings (and the fifth,
narrower one on the pack-authored `state_transitions` resolver) as decided,
and repair every document whose stated rule the rulings replace. No `rust/`,
`runtime/`, `editors/`, or `art/` file is touched by this lane.

## Decisions taken

- § *Rulings still open* becomes § *Rulings*. Each subsection keeps its
  statement, its rationale, and its consequences, and gains one **Decided.**
  sentence naming the build-order steps that land it. The four
  "If the owner decides the other way" paragraphs and the fifth ruling's
  "*The other way*" clause are dropped: a decided ruling has no other way,
  and leaving them would make the page argue against its own status box.
- The fifth ruling's lead-in read "emitting alias facts only" while its own
  contract sentence allows `VariableCellAliasTransition` **and**
  `NamespaceTransition` facts. The two families are the reading carried
  through, in the page and in `spec-packs.md`.
- Every repaired rule is stated as the design has it, in the page's own
  voice, with one present-tense "today" sentence wherever the tree still
  does the thing the ruling replaces. Each such sentence was checked against
  the tree with grep before it was written; no sentence claims behaviour
  that does not exist.

## Document inventory

| Document | Section | Status |
|---|---|---|
| `compiler/registry-consumer-contracts.md` | § Rulings, § The two hook bodies that remain, § Codegen…, § C Tcl extensions, § Build order, § Related docs | done |
| `compiler/command-registry.md` | § Authoring a spec without Rust | done |
| `contracts/dialect-stubs.md` | § Flags, § Stubs are declarations | done |
| `registry/spec-packs.md` | § Workspace trust…, § What a pack still cannot say | done |
| `runtime/c-extension-shim.md` | § The implemented subset, § Out of scope | done |
| `runtime/c-extension-abi.md` | § 7 Header scope | done |
| `runtime/tclvm-opcode-status.md` | note 5, `startCommand` | done |
| `runtime/rename-alias.md` | § 3.5 Invalidation | done |
| `compiler/aot-command-priority.md` | § 5 | done |
| `docs/GLOSSARY.md` | C extension shim | done |
| `design/README.md`, `design/compiler/README.md` | the index entries | done |
| `docs/kcs/` | — | checked, no note states a replaced rule |

## Open uncertainties

- `docs/kcs/kcs-qa-what-is-the-c-extension-shim.md` and the glossary entry
  describe compiling against `tclshim.h`. That is what the tree does today,
  so both keep it; the glossary entry names the authored header as the
  contract beside it. The KCS note is left alone rather than made to promise
  a header the tree does not contain.
- `aot-command-priority.md` § 5 groups `load`/`unload` in one cell. The
  WASM runtime registers `load` on `unsupported_cmd`
  (`runtime/rust/src/cmd_misc.rs`) and does not register `unload` at all;
  the repair changes only the "fixable?" answer and leaves the grouping.
