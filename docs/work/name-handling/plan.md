# Completion plan

Audit resumed at the user’s request. Six disjoint lanes continue actual source/operational/generated/guidance review. The 2026-10-16 00:42 UTC checkpoint retains **7,779 provisional purpose records / 1,360 hash-verified members**, not a repository migration total. Root has mapped its 330 records to a separate proposed work-unit catalogue; independent and cross-owner reconciliation remain unfinished. See [the current audit status](audit-status.md). Source/build freeze and original validation limits remain.

The goal is one coherent set of name-handling tools: parsing at ingress,
structured identity throughout semantic work, shared lookup policy over
appropriate state adapters, common variable/frame and binding facts, and
presentation only at output. Both static and live consumers must follow it.

## Scope and completion boundary

One reusable name-lookup owner and adoption by every consumer is the central
objective. Fix #2286 and issues entirely about naming. Treat unrelated runtime,
clock, numeric and general OO behaviour only as a bounded dependency when it
blocks relevant naming validation. A broad upstream suite result does not replace
call-site migration acceptance. Complete the naming inventory, then the
repository-wide documentation/comments cleanup and all required PR gates.

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

## Current execution priorities

| Owner | Next actual naming-consumer work | State and boundary |
| --- | --- | --- |
| Compiler + Core |Finish authentic Logical variable source-scope/occurrence/declaration transport, then typed alias/reference/rename consumers |Portable Compiler draft is incomplete and unreviewed. TEMP270 shows source word/key/reference geometry passes before namespace/local point adapters refuse; alias control has no actual VarDefs before the public join. Do not weaken current joins or invent Native frames. |
| DB + Compiler |Replace Absent/W123-ledger call inventory with the positive retained Logical source catalogue and a genuine source-selected occupancy facet |Portable DB facade is unadopted; consumer draft is not a patch. Builtin/local/alias source occupancy stays terminal; None means unrepresented, never absence. Callback earlier-unknown effects remain a barrier. |
| Compiler |Migrate live `apply_variable_alias`→`alias_cell` source-definition inventory constructors and `param_traits::scan_segment` recursive command-body geometry |These remain genuine naming consumers. Presence-only protective `link_target` uses and report transport are distinct; Core/Server positive String alias joins are retired. |
| Core/LSP |Migrate aggressive minifier `alias_string_literals` reference-boundary construction |Controlled original-six and excluded initial evidence are retained; no Rust implementation yet. Use actual full input/config, shared scalar construction and complete following-fragment boundaries. Inline229 and formatter230 are adopted; all four added Core272 controls pass, while formatter88 and integration55/15 annexes remain unexecuted. |
| Runtime |Finish remaining actual namespace helper/import consumers through the existing physical/publication/argv/lifetime owners |Parent237/Text238 are adopted; their Runtime/VM controls remain unexecuted despite passing pure Syntax branches. Import manual cache/alias/cycle handling is not safely replaced by a naive helper call. Wider roster/traversal/reentrant/final-free obligations remain separate. |
| VM/Codegen |Finish selected C deletion-incarnation operation receipt and no-argument Parent current-holder read |Portable drafts have API/lifetime peers pending. Preserve strict generic namespace currentness; a report label cannot issue a token. |
| Every surface |Continue naming-specific audits across all registered targets, hosts, editors, CLI/debugger, F5/EDA, MCP/AI/templates and generated guidance |Finite330 is a bounded reviewed universe, not a migration count or complete-codebase census. Generic command counting/separator parsing alone is adjacent syntax, outside this task without a real naming/operand-role caller. Dormant TS package insertion remains dormant. |

Source238 is adopted on `work`; checkpoint `4e20920a5` is push-confirmed. Source232 owner clauses and the219 RHS catalogue are applied. Parent/Text sources237/238 are integrated; the fresh selected ordinary272 results below now cover source238. Native Runtime/VM acceptance remains separate. Ordinary271 failed its fresh build with exit101, Registry/Compiler rustc SIGKILL9, seven partial ELFs and zero tests; source drift{}. That failed271 attempt executed no tests. Fresh ordinary272 on actual source238 seals956 controls: 643 PASS, 312 FAIL, 1 TIMEOUT. Its single-job fresh build passes in348.483727 seconds with14 test ELFs, errors[] and unchanged authored source, executing zero tests during the build. All12 selected scopes retain unchanged captured source/binaries, exact single runs and no missing/empty filters. Every265929 selector remains with1 gain and0 prior-PASS regressions/drops; eight additions within retained scopes are7 PASS/1 FAIL; the independent IRules19 scope is16 PASS/3 FAIL. Original#2286 runs first and passes72.400995379s under the unchanged180-second budget. Runtime174/VM133 and formatter88/integration55/15 annexes remain unexecuted plans; normal owner CLI250 still197 diagnostics. The cgroup OOM counters are cumulative without a before/after cause baseline; do not claim measured causation. Runtime174 and VM133 are fully peered future plans, unexecuted.

TEMP270 built successfully and ran twelve exact diagnostic controls once: eight Compiler failures and four DB failures, no timeouts or source/binary drift. All three temporary files and97,819 authored inputs restored exactly at08:45:43.133493 UTC, changed inputs{}. Diagnostic results establish first guards only; they provide no ordinary acceptance.

Read [current-state.md](current-state.md) for scoped failures and sealed272; preserve the unchanged180-second original#2286 budget and every earlier selector. Do not prioritise validation/documentation chores over finishing actual naming-consumer migrations.

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

## Sub-agent ownership and inventory deliverables

Root owns shared documents, Git, issue actions and final integration. Assign
disjoint owners for static/registry producers, runtime/VM, middle-end/codegen,
LSP/workspace/tools/AI, oracles/F5 and final docs/comments. Each bounded job
states exact paths, input dependency freeze, private build target, expected
positive/adversarial evidence and unresolved dependencies. Serialize writes
or captures involving the same owner; one target cache has one active builder.
Use [the full prompt](continue-prompt.md) for the coordination protocol.

Every call-site row records file/function, operation, input/output
representation, owner API, dialect/provider/source/frame/namespace context,
lifetime/invalidation, editable spans, intended consumer purpose, oracle
question/observation, regression selectors and disposition. Build the inventory
from actual tracked members, backend/provider rosters and generator sources.
A group-level matrix or finite drift gate does not prove call-site completion.
Record reconciled search hits, including legitimate presentation adapters.
Before removing the temporary inventory, move useful current owner/consumer
contracts into durable documentation.

## Harness integrity and oracle handoff

When using C Tcltest, execute the original init.tcl and tcltest.tcl from the
exact interpreter version with original test/support/package inputs. Verify
upstream revision and byte hashes, binary/library pairing and loaded harness.
Current run_suites.py independently accepts source, executable and version;
shared source checks do not certify original Tcltest bytes. Add meaningful
integrity checks and fail a mismatch. Preserve unchanged-suite, backend-overlay
and reduced-probe channels separately. The current overlay needs 8.5+;
8.4 uses --no-overlay. Debug-frame settings affect launch expectations.
Jim's original testing.tcl/tcltest.tcl wrapper and actual extension capabilities
have their own provenance; rediscover latest upstream at run time.

Consume [existing F5 answers](../../../experiments/name-handling/f5/answers/README.md)
and the [outstanding operation audit](../../../experiments/name-handling/f5/answers/outstanding-operation-audit.md)
before requesting more runs. Hosted proc-definition fixtures need explicit
owned absolute names and catalogue/body/invocation/frame controls; unqualified
proc inside namespace eval has measured appliance-specific publication.
Prioritise unreached supported contexts and variants: actual forwarding/link
frames, alias/import/trace mutation, warmed descendants, other TMM traffic,
suspended activations, reload/restart, protocol availability and exact bytes.
Each remains a named question; do not infer one context from another.

Prepare bounded scripts/questions on work using REQUEST.md and ANSWER-TEMPLATE.md.
The appliance-access chat returns answers, safe raw output, provider identity,
input hashes and cleanup evidence under f5/answers/<run-id>/, committed to work.
Both chats inspect incoming commits before pushing. Root verifies and promotes
only actually reached answers into deterministic consumer tests. Continue
independent C/Jim work while waiting; unanswered required F5 semantics stay
blocked. Scope original numbered matrices, pattern matrices and supplements
separately.

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
   and Jim provenance checking. Require validated all-five C discovery using required_tclshs(TclVersion::ALL),
   or TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 for available-interpreter callers;
   invalid configured references fail and absent required providers are not
   green. Verify original init.tcl/tcltest.tcl and library/source pairing
   independently of patchlevel or file-existence checks. Extend drift checks
   to semantic duplicates and
   representation misuse without banning legitimate presentation operations.
8. **Clean all documentation and comments, then audit again.** Use the checklist
   below. Remove temporary planning material before the final PR, after obligations
   are resolved or represented as measured unsupported boundaries with tested
   refusal in current contracts. Unavailable required evidence is not a
   supported limitation and remains a blocker.

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
require all five releases for available-interpreter runners with
`TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1`, or use `required_tclshs(TclVersion::ALL)`.
Jim discovery validates its revision-bearing patchlevel through the shared
reference manifest and `require_jimsh`; actual build capabilities still need
operation probes.

Use existing suites first: syntax command/variable/namespace conformance;
matching upstream C Tcl `tcltest` suites and Jim's equivalent oracle suite;
compiler command conformance, namespace, dispatch-proof and binding suites;
VM resolution/colon-run/variable/lifecycle suites; WASM namespace/import tests;
LSP core/server references, rename safety, cross-file/import/OO tests; editor
name-resolution fixtures; MCP/CLI integration. Add meaningful regressions for
new measured gaps. Manual fuzz/generator campaigns remain outside normal CI.

Follow `AGENTS.md`: isolated build directories for concurrent worktrees,
`make rust-check` for Rust changes, and `make prep-pr` at the PR stage.
The user authorises regular development pushes before green gates while
failures and unexecuted checks remain explicit in the tracking documents.
CI runs the deep tiers; reproduce failures locally when needed. Do not claim a
passed gate when tools or oracle binaries are absent.

Completion requires every row and call site accounted for, no competing lookup
policy, no semantic display-name keys, correct context/invalidation, no unsafe
rewrite on uncertain identity, complete supported oracle evidence and green
required gates. Review open and closed related issues with their regressions;
GitHub status alone is not a completion gate or proof of a branch fix.

## Conditional source advice for imports and paths

#2287 is closed with both complete packaged-server import controls passing.
Retain those controls and their exact scoped receipts rather than treating the
original highlighting failure as current. The remaining positive Logical catalogue/occupancy transport is unresolved; inspect the current shared producer and build a
public-path positive/negative before broadening assistance.

The consumer already delegates to Compiler
`registry_invocation::source_structure::source_registry_words` and retains
conditional `OriginalRegistryWords`. Review the producer path through
`command_binding::source_catalogue_advice`, the original command-world
candidate paths and `source_transition_advice::AdviceGraph`. Review current import/export/path support at the particular source site;
the accepted #2287 source schema does not establish callback catalogue
completeness or native import execution. Actual
execution-backed import facts and conditional source applicability are separate
authorities; missing one cannot silently become the other.

Extend the existing owner generically where the source can support assistance:

- Retain exact original import/path/export operands, their source scope,
  order, Registry generation, selected release/policy and applicability
  obligations. Preserve per-site export snapshots and actual imported-token
  semantics separately from a possible source-schema candidate.
- Use registry-authored export/role/provider facts with explicit availability
  conditions. A package requirement alone proves neither installation nor
  body entry. Unknown provider/version/effects retain uncertainty; known
  absence, disabled release, shadowing or deletion must refuse that schema.
- Model current/path/root candidate order for the selected policy. C Tcl 8.4
  lacks namespace path; Jim's independently selected namespace helper and
  flat-name recipe require their own evidence. Do not inherit C rules merely
  because source words look alike.
- Invalidate or widen affected advice for forget, force import, rename,
  replacement, export changes, namespace deletion/recreation, unknown package
  effects and source/configuration changes. Preserve unaffected scopes.
- Keep original geometry/candidate obligations through every consumer. A
  conditional option/body role supplies no Native identity, entered frame,
  completed world, references/rename completeness or optimiser permission.

Acceptance includes the two server import controls, genuine W210 negatives,
known user-command shadows, forgotten/replaced imports, conditional exports,
namespace path alternatives, same-tail siblings, package/provider absence and
release-specific unsupported operations. Include directives appearing as
braced data, copied/stale source and dynamic inputs that must remain unknown.
Where current evidence cannot support a safe source schema, retain the reason
and bounded candidates; do not achieve a green test by weakening assertions
or switching the entire source to Logical compatibility.

## Documentation/comment cleanup checklist

- Audit the entire repository, including pre-existing material unrelated to
  #2286 or name handling. Do not limit cleanup to changed files or this issue's
  documents. Identify obsolete API/architecture claims, unsupported guidance,
  abandoned plans, completed TODOs and other development chaff; remove or correct
  them against current code. Keep live limitations and useful current guidance.
- Apply the whole cleanup to documentation and code comments, including API
  doc comments, implementation/test/script comments, templates and generated
  sources. Fix generator owners and regenerate rather than editing outputs only.
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
- Search the entire tracked tree again, including unchanged tests and generated
  sources; inspect hits rather than erasing issue numbers mechanically.
- Remove `docs/work/name-handling/` and temporary index links from the final
  diff; durable experiments/answers stay under `experiments/name-handling/`.
- Treat this cleanup as merge acceptance, not optional follow-up. Find temporary
  ledgers, continuation prompts, state documents and coordination narratives
  throughout the tracked tree, not just this directory. Remove their links and
  generated copies. Review references to phases, lanes, steps, slices and agents
  in context; keep actual compiler/runtime semantics and user procedures.
  Review retained experiment descriptions too: preserve inputs, results and
  provenance, while removing coordination/process narration around them.
- Verify the final tracked tree and branch diff have no dangling coordination
  links or temporary process documents. Keep the cleanup obligation here while
  work is active, then remove this checklist with the rest of this directory.
- Rewrite any final PR title/body for the final behaviour and validation only.
  Git retains development history. Do not publish a phase/slice/agent chronology.

Final source238 validation: [ordinary272 current snapshot](current-state.md#current272-selected-scopes); exact portable receipts and unfinished owner work are indexed in [handoff/README.md](handoff/README.md). Historical scope in this document remains bounded to its stated capture.
