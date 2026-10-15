# Structured procedure and body identity: proposed #1778 migration

Fresh ordinary272 on actual source238 seals956 controls: 643 PASS, 312 FAIL, 1 TIMEOUT. Its single-job fresh build passes in348.483727 seconds with14 test ELFs, errors[] and unchanged authored source, executing zero tests during the build. All12 selected scopes retain unchanged captured source/binaries, exact single runs and no missing/empty filters. Every265929 selector remains with1 gain and0 prior-PASS regressions/drops; eight additions within retained scopes are7 PASS/1 FAIL; the independent IRules19 scope is16 PASS/3 FAIL. Original#2286 runs first and passes72.400995379s under the unchanged180-second budget. Runtime174/VM133 and formatter88/integration55/15 annexes remain unexecuted plans; normal owner CLI250 still197 diagnostics.

## Current238 handoff scope

This is a dated13 October design proposal, not the present Module representation. Current issued procedure IDs, original Authored/Logical/Native body cards and report-label projections are adopted. Current226 readonly coverage preserves all Authored+Installed rows with unique injective actual allocation/site/origin/formals/namespace/current Module/FU/CFG pairing; it grants only Authored analytical admissibility. Remaining live source-scope/catalogue consumers are in the current ledger; no old rendered-name architecture should be reinstated.

Priority remains finishing actual naming-consumer migrations in parallel; [implementation-ledger.md](implementation-ledger.md) assigns the next owners. [current-state.md](current-state.md) records the current executed scopes; [handoff/README.md](handoff/README.md) contains explicitly unadopted continuation work.

## Retained bounded audit

The text below records its stated source/date and earlier acceptance only. It is not a fresh whole-surface current-source certification; later implementation and current failures are governed by the handoff above.

Read-only proposal for the current `work` sources, 2026-10-13. Do not apply
until the evaluated-original-argv implementation has passed the queued genuine
WASM checks. This plan changes semantic map keys; it does not introduce another
name parser/resolver or broaden native compilation admission.

## Problem and outcome

`Module.procedures` retains only the first `Procedure` for a rendered qualified
name. Distinct byte-valued namespace/simple-name tuples can share that string;
repeated declarations of the same actual slot also have separate bodies and
implementation allocations. A printable name cannot distinguish either case.
Current CFG/unit/interproc/codegen maps propagate that loss. Preserving argv at
publication fixes reinterpretation of a rendered name but cannot recover a body
already discarded by a map collision.

The outcome is an inventory retaining every actual readable declaration/body,
with typed source identity for cross-pass joins and independently retained shared
publication coordinates. A source lookup selects a current implementation from
the existing positioned owner; it never selects the first display-label match.
Unsupported compilation preserves the actual executable declaration/invocation
and takes the existing generic runtime path.

## Reuse the existing owners

* `tcl_core_types::ByteCommandSlot` / `ByteNamespacePath` / `NameBytes` encode
  structural bytes. These are naming coordinates, not runtime command tokens or
  live namespace incarnation proofs. No split, qualify, trim, unroot or private
  key decoder may manufacture them from reporting text.
* `signature_scan::scope::SignatureSourceCommand` retains publication purpose,
  `NamePolicyProtocol` and the byte slot. Its checked `procedure_from_key` /
  `procedure_in_context` use shared creation/publication recipes and the selected
  grammar. Retain an original producer's issued value; consumers must not
  reissue a value from `Procedure.qualified_name`.
* `command_binding::{CommandAllocationSite, SourceOriginId, CommandAllocation,
  AllocationIncarnation, SourceNamespaceKey}` retain source-instance/offset,
  implementation incarnation and exact namespace allocation context. A source
  span alone is insufficient. `RepeatedFresh` explicitly asserts no distinct
  runtime token and must continue to retract direct/native claims.
* `command_binding::SourceCommandDefinition` is issued only by positioned lookup
  and supplies a selected implementation allocation for navigation. It grants
  no successful call, native compiler or body-entry licence.
* `SourceProcedureImplementationBody` / `SourceInstalledProcedureBody` already
  retain allocation, original evaluated source, formal topology and namespace
  context, including retired implementations. Extend/reuse this inventory for
  lowering retention, rather than inventing a second source history walker.
* `OriginalSourceProcedurePublications` and its publication/candidate carriers
  distinguish original declaration slot from later moved target slots. Keep
  conditional callback advice separate from actual runtime selection.

A minimal compiler body-unit ID belongs in the existing IR/source provenance
owner, issued at lowering from the authentic source/declaration producer. It is
an internal cross-pass identity, not another interpreter-name semantic axis.
Expose the retained source declaration/allocation projection as needed through
`command_binding`; do not export `DeferredImplementationId` unchanged: its
`command: String` is presently part of equality and serves wider deferred state.

The exact identity representation needs one small owner design decision before
editing: original authored declaration identity is its full source-origin/site;
retained installed bodies additionally have allocation incarnation and original
body/source/namespace context. Two generated source instances with identical
bytes/offsets must remain distinct where the original provenance says they are.
A compiler-local issued ordinal may index bodies, but it must carry typed
provenance for lookup joins and cannot become a runtime token. Do not wrap a
rendered FQN in a newtype and call the issue fixed.

## Target representation

`Procedure` keeps `name` and `qualified_name` only as reporting/source spellings.
Add an owned identity/provenance carrier and explicit body kind. Actual procedure
publications retain an optional issued `SignatureSourceCommand` (or its existing
owner-owned publication carrier) plus exact `SourceNamespaceKey` for the body.
Unknown/dynamic/unsupported publications need no guessed slot. A method, event,
lambda, namespace-eval body and installed-only analysis body retain explicit
kind/context and never acquire callable authority through their label.

The primary inventories use a typed body/declaration ID. An optional secondary
publication index maps `(NamePolicyProtocol, ByteCommandSlot, actual retained
namespace context as required by the source owner)` to ALL associated IDs;
chronology remains with the positioned source-command world. This is not a
`slot -> last Procedure` replacement resolver. Equal display labels retain all
rows, and same-slot redefinitions retain every body. Presentation consumers may
use an explicit multimap or list; a label lookup cannot choose arbitrarily.

Use deterministic ordering derived from source origin/declaration ordering and
body kind for reports/table layout, not a printable name as equality. Keep
original body offsets, parameter topology, source/lexer/registry metadata and
source-origin identities in memo validation. A body-content cache can reuse
analysis only under its existing full context checks; cached results must be
joined back to the current typed unit ID independently.

## Concrete migration slices and callsites

| Slice | Existing paths / callsites | Required change and acceptance |
| --- | --- | --- |
| IR and producers | `ir.rs::Procedure`, `Module.procedures`, `body_units`, `irules_event_bodies`, `lambda_body_units`, `redefined_procedures`; `lowering/mod.rs::try_lower_logical_procedure_declaration` (~3268), `lower_proc` (~3805), `lower_when` (~4238), `register_body_unit` (~4558); `lowering/installed_bodies.rs` | Issue typed identities from actual producers, retain every body, retain explicit namespace/body-kind, replace first-wins insertion. Preserve malformed-formal/unknown-body/dynamic-name and `suppress_proc_register` runtime-time constraints. A separate declaration inventory can retain an uncompiled body without publishing a callable. |
| Source joins | `registry_invocation/original_procedure_call.rs::original_procedure_calls_for_targets`; `command_binding/source_transition_advice/source_callback.rs::original_ir_procedure_name`; `command_binding/retained_source.rs::matches_original_procedure`; `command_binding.rs::discarded_procedure_history` (~15690) | Join via actual source/implementation provenance. Current callsites scan spans/body bytes then return a string; return typed IDs/borrowed rows instead. Delete the rendered-name recovery walker only after complete inventory preserves its readable-body coverage and unknown-history opacity. No ambiguous span, byte-identical foreign source or absent receipt can select a body. |
| CFG and SSA | `cfg.rs::Module.procedures`; `cfg_builder/mod.rs`, `execution_regions.rs`, `summary_candidates.rs`, `upvar_info.rs`, `global_write_info.rs`; `ssa.rs` | Propagate IDs one-for-one; derive frame namespace from retained typed context, not label `rsplit`. A synthetic analysis unit must stay distinct from an executable procedure. |
| Compilation units/cache | `compilation_unit.rs::CompilationUnit.procedures`, `build_procedure_units` (~2755), `BuiltProcedureUnits`, parameter constants, extra call-site contexts; `compile_service.rs`, `lattice_rebase.rs`, `unit_scope.rs` | ID-keyed IR/CFG/SSA/function-unit joins, seeds, tainted writes and stored parameter constants. Keep cache original source/image, grammar/registry/entry/frame checks and valid rebase semantics; missing ID rejects reuse. |
| Interproc/call graph | `interprocedural.rs` summaries/`ProcScan`/`ScanCtx`/method source-procedure view; `interprocedural/{transfer.rs,transfer/source_context.rs,eager.rs,completion.rs}` | Summaries, SCC nodes, callers, definition points and transfer facts keyed by typed unit ID. Resolve selected declaration/allocation with the existing invocation owner before joining the callee unit. Union real may-targets; a missing/unknown alternative must continue to widen. Name reporting remains a field. |
| Other analyses/rewrites | `common_aot_plan{.rs,/authored_source.rs,/declared_arguments.rs}`, `registry_invocation.rs`, `var_escape/{original_slots.rs,walker.rs,slot_resolution.rs,known_names.rs,api.rs}`, `taint_interproc.rs`, `taint.rs`, `value_transfer.rs`, `def_use.rs`, `inlining/*`, `optimiser/{unused_procs.rs,tail_call.rs,...}`, `native_integer_proof.rs`, `math_function_binding.rs` | Migrate semantic joins and trusted body selection to IDs while preserving proof/admission checks. Rename-only presentation traversals need no invented name authority. Fold/inlining/direct call must reject equal-label wrong implementation, replacement/alias ambiguity and `RepeatedFresh`. |
| Native lowering / WASM | `native_lowering/{ir.rs,lower.rs}` `DefineProc.entry_key`; `codegen/wasm/backend.rs::ProcedurePlan`, `procedure_plan`, native bound/defined/report maps, direct-call plan; `native_emit.rs::EntryTable::slot`, `push_entry_index`; `pipeline.rs` | Replace entry key with typed compiled-body ID issued from actual admitted declaration. Table indices/arity/direct sets and FunctionUnit joins use this ID. Definition argv is unchanged and evaluated once. Preserve native header/currency and all existing original-operation, formal, namespace, source-site, definer and replacement guards. If a definition cannot attach an entry, emit its original generic definition; do not omit it. |
| Other emitters/tooling views | `codegen/emitter/{mod.rs,generate.rs,proc_defs.rs}`, `codegen/{mod.rs,backend.rs}`, compiler explorers/API and downstream consumers of units | All semantic maps use IDs; emission still uses the original command/name operands under the target's actual contract. Output labels and diagnostic APIs expose explicit display plus identity or multiple rows where necessary; no display->first-body adaptation. Audit CLI/MCP/AI/LSP consumers of changed public maps with build/type checks. |

`/tmp/codegen-1778-consumer-paths.txt` retains the broad bounded `rg -l` inventory
of compiler source files mentioning procedure maps/qualified names/redefinition
sets/body-name helpers. It is an audit seed, not a claim every match is semantic
or every consumer is already migrated. For each affected file, record whether it
indexes a semantic unit, uses source spellings, or formats a report. Stop only
when every map join has one of those dispositions.

## Implementation order and ownership

1. Agree the minimal issued body/declaration carrier and full-provenance joins.
   Add fixed deterministic structural collision controls first. Coordinate
   `ir.rs` and command-binding contracts with compiler/current-source owners.
2. Add complete ID-indexed inventory/producers, preserve executable declaration
   Statements and installed/original-declaration analysis roots. During a brief
   compatibility period a reporting index must return all matches; ambiguous
   old label queries refuse. Do not retain an old semantic first-wins map.
3. Thread IDs through CFG/SSA/units and all interproc facts in one coherent
   dependency slice. Build once sources are stable. Capture failures without
   concurrent source mutations and fix the measured joins.
4. Migrate consumers/optimisations and eliminate qname-derived namespace/label
   kind parsing. Retain unknown/unreadable and complexity limits explicitly.
5. Migrate compiled entry identity/maps and direct-plan joins; execute genuine
   runtime-linked WASM controls for two colliding displayed names and source
   redefinition. Only now remove `redefined_procedures: HashSet<String>` in
   favour of source-owner may-implementation/redefinition evidence.
6. Remove redundant discarded-body recovery and development compatibility
   adapters. Update current contracts and minimal how/why comments; preserve
   experiment inputs/results/seals, keep chronological development text in Git.

Do not overlap the initial identity/provenance slice with pending W210/body
coverage/Registry changes. Root assigns IR/lowering, binding joins, analysis maps
and emitters as separate owners only once interfaces are stable. Build target
leases and source freezes remain serial; disjoint read-only audits can proceed.

## Oracle and regression acceptance

First retain exact inputs/raw outputs from the existing five C binaries and
pinned latest-verified Jim identity. For F5, send exact programs and questions to
the BIG-IP-access chat; its answer documents in this branch are separate provider
facts. Runtime availability/metadata catalogues cannot substitute for execution.
Use plain deterministic scripts unless Tcltest adds useful structure; Tcltest
must be the original version-appropriate `tcltest.tcl`/`init.tcl` from that built
interpreter. Preserve each failing orchestration and supplement it separately.

Required controls, with expected answers established by actual oracle records:

* C structural collision: namespace `a:` / leaf `p` versus namespace `a` /
  leaf `:p`, with equal reporting FQN but separate return constants. Create and
  invoke in their actual namespace contexts so the shared display is never
  reparsed to choose one; also query/report display solely as observation.
  Jim uses its own flat-name publication policy; don't assert the C geometry.
* Root colon publication/empty simple leaves, colon-only names, whitespace,
  case, Unicode and byte-valued names supported by each real provider. Distinct
  source spelling cannot stand in for equal/different actual slot identity.
* Same-slot sequential redefinition: both bodies retained for analysis; actual
  invocation before/after yields first/second; no summary, cache, table or fold
  borrows the wrong implementation. Prove untouched separate-slot control too.
* Alias/import/rename chains select the current implementation allocation while
  retaining called slot/origin token; replacement and unresolved edges reject
  direct/native shortcuts. Token survival does not imply body survival.
* Namespace delete/recreate and retained/dead namespaces: equal paths are not
  equal incarnations. Calls/body frame context must use the genuine retained
  namespace owner, preserve stale-command/deletion results and generic fallback.
* Repeated factory allocation at one source site: distinguish first/second
  actual identities and retain bounded coverage; `RepeatedFresh` stays uncertain.
  Separate generated source origins with equal body bytes/spans cannot share
  current-entry or edit identity. Literal equal source in two channels is a
  negative provenance control.
* Events/lambdas/namespace-eval/installed-only bodies: coverage and correct
  frame context persist, but no synthetic report label installs a named Tcl
  procedure. Ordinary user names resembling internal `::when::`/`#` labels
  cannot collide with their internal body identities.
* Genuine WASM: distinct emitted entries for supported collision/redefinition
  cases, independently observed values and native counters, stable original
  argv evaluation/lifetime, real replaced-definer/unknown-name negatives.
  Unsupported cases execute preserved original source via generic path; source
  catalogue hints/software callable stubs cannot certify native entry authority.
* Unit/interproc tests: rows and CFG/SSA/function-unit IDs stay injective;
  summaries/formals/taint/side-effect facts attach to actual declaration; changing
  only display names does not redirect semantics; missing/foreign origin or
  ambiguous allocation returns conservative unknown rather than first match.

Validation capture must record actual executed binary/tool identities, inputs,
exact argv/env/status and frozen compiled source closure, including extensionless
`include_str!`, external F5 text and generated OUT_DIR inputs discovered from
actual dep-info. The earlier Runtime receipt has an explicit 79-input post-only
coverage supplement; do not repeat that suffix-filter coverage gap. Required
API/compiler/CLI/type checks and real runtime-linked WASM are separate results.
Full gates and cross-codebase consumer audit are necessary before PR readiness.

## Acceptance boundaries

This slice can close compiler map-loss only after every semantic join uses typed
identity and oracle-derived controls pass. #1778 covers runtime/VM/LSP indexes,
sidecars/traces/teardown too; a compiler migration alone is not full issue closure.
Original-argv acceptance, F5 answers and current naming-base/variable-policy
producer work remain independent prerequisites or explicit outstanding items.

Final source238 validation: [ordinary272 current snapshot](current-state.md#current272-selected-scopes); exact portable receipts and unfinished owner work are indexed in [handoff/README.md](handoff/README.md). Historical scope in this document remains bounded to its stated capture.
