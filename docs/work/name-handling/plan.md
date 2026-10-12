# Completion plan

The goal is one coherent set of name-handling tools: parsing at ingress,
structured identity throughout semantic work, shared lookup policy over
appropriate state adapters, common variable/frame and binding facts, and
presentation only at output. Both static and live consumers must follow it.

## Inventory required for every consumer

Build a per-call-site ledger from every workspace member in `Cargo.toml`,
`runtime/rust`, editor integrations, scripts, generated-source templates and
AI/tooling surfaces. Record file/function, operation, input representation,
current owner, replacement API, namespace/frame/interpreter/release context,
output representation, invalidation/lifetime rules, source spans, oracle
case IDs, consumer tests and disposition. Dispositions are: audited adapter,
migration required, migrated and verified, unsupported with tested rejection,
or blocked on a named oracle question. An unexamined row is unfinished.

Search broadly for `split`, `rsplit`, `rfind`, `trim_start_matches`,
`strip_prefix`, `starts_with`, `contains` around `::`; `qualified_name`,
`all_procs`, `all_classes`, alias walks, reconstructed var refs, direct registry
lookup and command caches. Inspect callers and produced representations.
String operations in presentation or source lexing are not automatically bugs;
a sanctioned semantic adapter must explain its precondition and have tests.
The final check reconciles all matches and all workspace members with the ledger.

## Consumer completion matrix

| Group | Files / surfaces to cover | Completion evidence |
| --- | --- | --- |
| Grammar/identity ingress | core scanner, lexer, syntax naming/word parts, dialect grammar, runtime types/API, written command/namespace/variable constructors | Colon runs, empty tails, lone colons, Unicode, binary values where supported, arrays and spans; parse once with selected dialect. |
| Registry/declarations | scoped `CommandRegistry` lookup, resolved invocations, overlay/stub surfaces, signature scan, SpecTcl declarations and studio schema | Written names and qualified implementation names resolve under scope/profile; aliases retain roles; no independent normaliser or copied command knowledge. |
| Signature/analyser | signature-scan handlers/factory/context/command-prefix/types; analyser scope/handlers/commands/indirection/OO/class hierarchy/lattice/item-tree | Common target facts and abstentions at call sites, ordered mutation/import effects, correct callback execution scope, `tcltest` setup/body/cleanup coverage. |
| Compiler binding and lookup | `command_binding`, alias, dynamic names, auto_path evaluation, dispatch proofs, site claims, unit/connection scope | Structured static identity, current/path/global policy, mutation-aware trust; no source-name or display-tail fallback proving a target. |
| Variables/frames | `var_resolve`, `var_scoping`, place bridge, escape/observability, frame-effect summaries, variable aliases | Plain/rooted top-level equivalence, relative-qualified names, literal/empty/array names, links and exact eval/uplevel/upvar distance; one storage identity per cell. |
| Middle end | CFG/SSA/memory SSA, SCCP/GVN, dataflow, type/object inference, effects, value transfers, interprocedural/taint, all optimisers and inlining | Each read/write/summary uses shared places and binding facts; no stale folds after writes/mutation or forced builtin identity; negative cases protect dynamic uncertainty. |
| Codegen, every target | backend inventory, generic bytecode/emission, expression/substitution/proc emission, WASM semantic/native plans, common AOT/native lowering, all target registrations | Equivalent observed lookup/storage results under interpreted, bytecode and compiled paths; direct calls guarded; generic fallback token-faithful. BPF unsupported features rejected explicitly. |
| VM | command arena/table/cache, namespaces, frames/cells, aliases/interpreters/imports/rename, traces, ensemble/OO/unknown/mathfunc, source/eval/uplevel/upvar | Shared policy adapters preserve tokens, generations, byte/UTF-8 limits, retained/dead namespace state and structured errors; run shared command/variable/namespace vectors. |
| WASM runtime | same live surfaces over byte namespace tree, namespace/vars engine and C exports | Same matrix with byte-exact names, frame/lifetime fidelity; imports and trace references follow real command identity. |
| LSP providers | definition/references/rename safety, namespace symbol/rename/import, completion/hover/signature help/tokens, symbols/selection, call/type hierarchy/definition, implementation, highlights/linked editing, inlays/lenses, refactors/minifier/graphs | Common selection/enumeration and abstention; collisions never cross-link or rewrite; exact spans survive long colon runs and list elements. |
| Workspace/server middleware | index/symbols, source graph, package/autoload, reachability/re-homing, settlement, cache invalidation, overlays and server request/diagnostic paths | Same identities across files and load-order evidence; all providers and diagnostics agree; stale context rejected. |
| Hosts/editors | native server, WASI/browser, VS Code/Zed/JetBrains/Neovim/Emacs/Helix/Sublime protocol and generated grammars | Clients preserve server identities/spans; source regexes use grammar owner/generator; no duplicated lookup in adapters. Run host-specific tests when touched. |
| F5/dialects | iRules `call`, rule/protocol namespace distinction, executable-region walkers, iApp/tmsh, BIG-IP query/refactor clients, EDA/Tk/Jim integrations | Context/profile-specific policy from registry and measured oracles; do not normalise BIG-IP object paths as Tcl names or fake unsupported command namespaces. |
| Tooling/AI | `tcl` CLI and support/query/refactor/compile/diagnostic paths, MCP including docstrings, debugger/explorer/evaluation, F5 CLI/query, AI prompts/skills and generators | CLI/LSP/MCP agree on identities and abstentions; generated AI guidance refers to current central APIs; equivalent tool-visible results. |
| Enforcement/docs | owner manifest, drift gates, vector runners, contracts/design/KCS/README/glossary, generated docs, source comments | Coverage beyond proc/class equality scans; accurate current contracts; reproducible oracle evidence; zero final process/history artefacts. |

## Measured engine priorities

Use [the current suite evidence](../../../experiments/name-handling/results/RESULTS.md)
to order the live-engine lane: restore 8.4/9.1 real-library bootstrap and a
release-selectable VM runner; reduce suite-stopping namespace deletion,
rename/trace and TclOO output-restore witnesses; resolve dead-cell/link/frame
semantics; then complete path/unknown/introspection, arrays and OO library
consumers. Promote C-passing cases as exact regressions and retain sequence tests.
A passing focused suite is insufficient while original upstream files abort,
time out or remain unexecuted. Static, backend, Jim and F5 obligations in the
consumer matrix remain required.

## Dynamic-pattern coverage and abstention

All static consumer rows must handle ordinary Tcl metaprogramming idioms through
shared analysis facts. Literal-only lookup is insufficient. Implement and audit
the [pattern acceptance matrix](../../../experiments/name-handling/case-catalogue.md#dynamic-pattern-acceptance)
alongside the [evidence/use contract](../../design/contracts/name-handling.md#dynamic-code-and-familiar-naming-patterns).

For each pattern record semantic owner, preconditions, supporting spans/value
provenance, namespace/frame/interpreter context, conditional assumptions,
invalidation and budget, result category, allowed consumers and abstention reason.
Reuse or extend the registry's declarations and the existing value/binding/frame
summaries. Consumers receive common results; they do not add private recognisers.

Acceptance requires positive idiom coverage and adversarial near misses. Test
shadowed name builders, intervening rename/write/trace, conditional or empty
factories, incomplete indexes, source order, aliases and changing OO receivers.
Compare wrong targets, missed provable targets and abstention reasons separately.
Heuristic candidate ranking never satisfies a transformation proof. Improvements
must reduce unnecessary abstention without increasing confident wrong results.

Source discovery uses the maintained pinned corpus and the Tcl Wiki under
[the investigation protocol](../../../experiments/name-handling/pattern-sources.md).
Start from `docs/CORPUS.md`, the authoritative performance manifest/fetcher and
the existing import-example references. Record concrete project/Wiki provenance,
reduce positive and adversarial examples, verify semantics on real oracles and
evaluate recognisers on held-out project families. Preserve benchmark pins and
separate patterns observed in code from identities proved at a particular site.

## Ordered work

1. **Reproduce and establish evidence.** Refresh issue relationships and the
   inventory. Reduce #2286 faithfully: a recognised `tcltest::test` invocation
   whose setup/body/cleanup share the correct execution frame, including
   parameters and writes performed by the body. Test both true missing reads
   and reported false positives. Inspect package/stub/import resolution before
   adding command-specific treatment. Run the existing vectors and retain the
   full oracle matrix; record absent references as absent, not green.
   Reuse C Tcl upstream `tcltest` cases with each matching real interpreter,
   and Jim's equivalent suite with its identified build. Preserve exact test
   definitions, suite revision, dependencies, constraints and harness results;
   skipped cases provide no measured answer. Add bespoke probes where the
   upstream suites do not cover the identity/context question.
2. **Settle API/identity boundaries.** Use existing owner crates and dependency
   direction. Specify source-name parsing, structured namespace/command slots,
   live command identity, variable places, span-bearing components and
   unresolved/ambiguous results. Avoid a catch-all utility crate or a parallel
   name system. Extend owners only where needed. Determine the shared
   token/byte policy adapter so runtime state remains in its own store.
3. **Migrate static producers first.** Signature scan, analyser, module/proc/class
   records, binding facts and workspace indexing must retain authoritative
   identity. String-keyed bridges use injective encoding, with a documented
   boundary; rendered FQNs never become keys. Propagate necessary context and
   spans. Preserve compatibility without keeping two semantic authorities.
4. **Migrate every downstream static consumer.** Apply the matrix, including
   variable places, summaries, inferred object types, middle-end facts and
   optimisers. Fix relative-qualified and top-level plain/rooted equivalence.
   Alias paths, lifecycle ordering and unknown facts use common decisions.
5. **Migrate live adapters and every backend.** Consolidate duplicated policy
   while preserving retained namespace/command generations, command objects,
   alias/import distinctions, variable links and traces. Audit backend
   registrations and direct/AOT calls, guard invalidation and runtime fallback.
   Do not use old blanket claims that all codegen performs runtime dispatch.
6. **Migrate LSP, middleware, tooling, AI and dialect surfaces.** Provider and
   workspace lookup agree; identity is carried through native/browser hosts,
   CLI/MCP/refactoring and generated prompts. Preserve refusal gates and exact
   source edits; a matching tail never overrides unresolved identity.
7. **Turn evidence into conformance and enforce adoption.** Every observed case
   has release/context expectations and consumer tests. Add matrix completeness
   and Jim provenance checking. Shared syntax runners currently use
   `available_tclshs()` and skip absent/invalid references: migrate to validated
   all-five discovery/assertions (or `witness_tclsh` for every `TclVersion::ALL`)
   rather than relying on the environment variable alone. Extend drift checks
   to semantic duplicates and
   representation misuse without banning legitimate presentation operations.
8. **Clean all documentation and comments, then audit again.** Use the checklist
   below. Remove temporary planning material after obligations are resolved or
   accurately represented as supported limitations in current contracts.

Rows dependent on unknown F5 semantics stay blocked until the other chat
returns evidence. Continue independent C/Jim work while waiting. Do not invent
F5 answers, copy C expectations, or waive a blocker to call the goal complete.

## Validation and completion

For each changed semantic operation, run deterministic oracle-derived cases
against the owner, affected consumers and appropriate public surfaces. Include
colliding display names, negative/ambiguous cases, mutations and namespace
recreation; successful ordinary names alone are insufficient. Optimised and
unoptimised/compiled execution must preserve observable results and effects.

Final evidence includes all five pinned C releases, an explicitly identified
latest upstream Jim revision/build, and returned F5 results for each supported
context. Unsupported features have measured capability results and tested
refusal; unavailable experiments remain pending. Set
`TCL_LSP_REQUIRE_TCLSH=1` for witness-based C callers and explicitly
validate all five releases for available-interpreter runners; the variable
alone does not enforce their matrix today. Jim needs its own validated
discovery contract.

Use existing suites first: syntax command/variable/namespace conformance;
matching upstream C Tcl `tcltest` suites and Jim's equivalent oracle suite;
compiler command conformance, namespace, dispatch-proof and binding suites;
VM resolution/colon-run/variable/lifecycle suites; WASM namespace/import tests;
LSP core/server references, rename safety, cross-file/import/OO tests; editor
name-resolution fixtures; MCP/CLI integration. Add meaningful regressions for
new measured gaps. Manual fuzz/generator campaigns remain outside normal CI.

Follow `AGENTS.md`: isolated build directories for concurrent worktrees,
`make rust-check` for Rust changes, and `make prep-pr` before every push.
CI runs the deep tiers; reproduce failures locally when needed. Do not claim a
passed gate when tools or oracle binaries are absent.

Completion requires every row and call site accounted for, no competing lookup
policy, no semantic display-name keys, correct context/invalidation, no unsafe
rewrite on uncertain identity, complete supported oracle evidence and green
required gates. Review open and closed related issues with their regressions;
GitHub status alone is not a completion gate or proof of a branch fix.

## Documentation/comment cleanup checklist

- Reconcile all existing name/frame/binding contracts against implementation
  and source/measurement evidence; update API ownership and consumer tables.
- Correct the specific contradictions in [current-state.md](current-state.md).
  Preserve older experimental measurements with their original provenance.
- Update design indexes, affected KCS notes, README-F5/README, glossary and
  generator source templates. Regenerate generated docs/prompts/catalogues.
- Remove milestone numbers, migration/phase narration, before/after bug stories,
  TODOs for completed work, agent notes, abandoned proposals and progress lists
  from shipped code comments and architecture/user docs.
- Keep necessary how/why comments and precise public preconditions. Prefer
  clear names, types and small functions to prose paraphrasing the code.
  Do not delete legal headers, oracle provenance, required machine-readable
  markers, or a justified live limitation merely because they contain dates.
- Search the entire tracked tree again, including touched tests and generated
  sources; inspect hits rather than erasing issue numbers mechanically.
- Remove `docs/work/name-handling/` and temporary index links from the final
  diff; durable experiments/answers stay under `experiments/name-handling/`.
- Rewrite any final PR title/body for the final behaviour and validation only.
  Git retains development history. Do not publish a phase/slice/agent chronology.
