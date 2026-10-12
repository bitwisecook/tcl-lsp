# Name-handling implementation ledger

This is temporary coordination material. Keep each obligation's current status,
owner and acceptance evidence here; retain experiments separately. Remove this
ledger from the completed implementation diff. Git records implementation history.

The integration baseline is `c49ef6fd5`, including remote `work` `bcaf88c12`.
Local checkpoint `e803f53f1` contains the implementation and scoped validation;
`df919cbed` contains the maintained wild-pattern catalogue and new Jim/F5
bundles. Source advice, W210, runtime variable/array handling, native bootstrap,
gate repair and consumer/documentation audits currently have assigned agents.
Check git and assignment status before taking shared-file ownership. Measurements identify their own source and binary;
a source audit is not an executed test.

## Active assignments

| Obligation | Owner | Status | Acceptance evidence / next check |
| --- | --- | --- | --- |
| Central architecture entry point | Coordinator | Entry point reconciled; full documentation audit pending | Actual Original, Logical, Jim and hosted owners and canonical proofs are linked; existing contracts still need per-consumer verification. |
| Complete consumer census | Coordinator | Audit required | Use [naming-consumer-matrix](../../design/contracts/naming-consumer-matrix.md), [implementer guide](../../design/compiler/name-resolution-implementer-guide.md) and [consumer integrations](../../design/analysis/name-resolution-proofs/consumer-integrations.md); account for each row below. |
| Namespace deletion and retained namespace objects | Runtime agent | Bounded fix verified | Seven native lifecycle/ensemble controls pass. Real 8.5 `namespace-56.2` passes. Preserve the separate cached-child mismatch after 8.5 real initialization. |
| Real matching `init.tcl` / `tcltest.tcl` bootstrap | Bootstrap / VM agents | Scoped fixes verified; full startup incomplete | Physical byte length and 9.1 auto-path use shared owners. Four native runtime controls and three VM driver controls pass. Both engines' helper-presence tests match all five C releases and Jim; native drivers retain startup refusals. |
| Static aliases, constructor hints and auto-path keys | Static agent | Three focused regressions pass | Current source binding handles paths/shadows; written variable tails and constructed colon namespaces are preserved; unknown effects withdraw imported constants. Unused analyser alias helper removed. Broader typing/class/dispatch controls remain red. |
| Corpus naming patterns | Corpus agent | Source/C/Jim observations retained | All 35 public pins verified; 19 sites / 12 repositories, 15 reductions, 75 C observations retained. Catalogue links 45 original-question Jim captures and four distinct native controls. 11 reached original Jim questions are registered canonically; four setup failures remain unanswered. Static held-out coverage, adapted-question review and F5 observations remain required. |
| Wiki naming patterns | Wiki agent | C questions indexed; Jim observations retained | 17 read pages and 30 probes. C passes 21/22/29/29/29. All 44 Wiki/corpus questions are indexed with original runner snapshots. Jim's 30 adapted inner questions have separate captures; unchanged C harness and our native Wiki attempts stop before tests. Canonical Jim registration and consumer coverage remain required. |
| Related issue state and reproduction | Issue agent / coordinator | Updated; two defects remain open | #2450 closed from published removal. #2286 positives pass but direct/alias undefined-read controls fail. #2287 namespaced and top-level token controls fail; generic conditional import/path advice needs repair. |
| Latest Jim oracle | Coordinator | Upstream reference verified and executed | Upstream HEAD matches shared pin `5bac7c99`; revision-bearing 0.84 build passes 339 tests in 12 real Jim naming suites. VM/runtime Jim conformance remains required. |
| F5 appliance questions/answers | Appliance chat / coordinator | Exact pattern bundle pending | [Pattern bundle](../../../experiments/name-handling/patterns/f5/README.md) has 50 questions across eight contexts, all 400 rows pending. Existing BIG-IP controls are linked separately. Returned documents must bind exact input, release, execution channel and reached event; ingest only matching answers. |
| Required gates and publication | Coordinator | Red; PR readiness blocked | Rust gate still needs formatting/full checks; latest complete proof audit has 158 changed implementation evidence digests. Command-backing and owner-resolution also report existing drift. An xtask build stopped at ENOSPC before any owner/backing gate executed; its entire cache was cleaned. Review evidence and repair all required gates at the PR stage. User-authorised development pushes may proceed with failures recorded. |

## Consumer acceptance census

Experiment structure: the native-ensemble capture now has 11 discrete
question directories with exact original source/input, scoped results and
limitations. Build provenance links the immutable historical snapshot; original
capture bytes are unchanged. New experiments follow this question-led format.

Every row requires an actual call-site/owner audit and relevant executed checks.
Existing contracts and proof records are evidence to inspect, not automatic
completion marks. Unknowns must remain explicit, with useful bounded assistance
when its separate preconditions hold.

| Consumer | Shared integration to verify | Status |
| --- | --- | --- |
| Lexer/parser/source ingestion | Complete source/channel/configuration, exact counted bytes and editable geometry | Audit required |
| Registry and dialect overlays | Generic roles, binding transitions and independently selected Native/hosted applicability | Audit required |
| Analyser diagnostics and Tcltest phases | Positioned binding/frame facts; setup/body/cleanup continuity; #2286/#2287 exact reproductions | Positive fixtures pass; genuine-undefined-read controls fail |
| Lowering, SSA, optimiser and type inference | Original invocation/entry/effect proofs; candidate hints remain conditional | Focused fixes pass; 27 typing and 3 class-relation controls remain red |
| Variable places/cells/arrays/links | Selected release, operation, frame, exact tail/index, alias and lifetime | Audit required |
| Classes, methods, forwards and metaclasses | Original class/family receiver; contextual dispatch; construction is not implied by `new` spelling | Audit required |
| LSP navigation, hover and completion | Structured current subjects; bounded candidates separated from definite identity | Audit required |
| LSP references, rename and refactoring | Complete occurrence coverage, exact geometry and independent rewrite permission | Audit required |
| Database/workspace/import/package settlement | URI/source currency, loader reachability, ordered mutation and independent identities | Audit required |
| VM | Actual native object/cache/purpose/lifetime, shared command cores | Native driver selection tested; real-suite startup needs compiler recipes |
| Runtime/WASM and middleware | Actual native objects, byte names, frame/callback/namespace lifecycle; shared command cores | Frozen real 8.5 var.test matches C: 69 pass, 17 skip, zero fail. Compiled alias/namespace controls pass; seven namespace failures and startup refusals remain |
| Bytecode, WASM, native/AOT and other codegen | Explicit backend admission; preserve dynamic dispatch and selected operation semantics | Audit required |
| CLI/tooling/diagrams/minifiers | Shared source owner, independent display and edit permissions | Audit required |
| MCP/AI/editor integrations | Same structured subjects and refusal semantics as shared providers; regenerate authored templates | Audit required |
| F5/EDA/framework surfaces | Selected ContextRegistry and profile/channel; no borrowed C capability or inferred runtime entry | Audit required |

## Documentation and comment completion

The scope is the whole repository, including pre-existing material unrelated to
this issue and files unchanged by name-handling work. Remove or correct stale
claims, obsolete guidance, abandoned plans and development chaff against current
code; account for this broad audit before marking documentation complete.

Audit durable contracts, KCS pages, glossary, user docs, generators, prompts and
comments against current code. Remove obsolete migration narration, phase/ledger
references and completed TODOs. Keep minimal how/why explanations, legal headers,
current limitations and reproducible oracle provenance. Regenerate affected
outputs and run the complete documentation/owner/proof gates. Remove this whole
temporary work directory and its links only when the consumer census is complete.

## Completed evidence work

- Real upstream naming suites were captured against both engines and matching C
  references; [results](../../../experiments/name-handling/results/RESULTS.md)
  retain their original subject, failures, skips, aborts and capture limitations.
- Corpus and Wiki reductions preserve actual source provenance, positive cases,
  adversarial cases and capability boundaries. Their observations do not establish
  unexecuted recogniser precision, current engine conformance or another provider.
- Integrated remote ownership/proof changes are retained on `work`; checkpointed
  agent changes use those owners rather than introduce parallel resolvers.
- Twelve shared syntax conformance tests pass with all five C references and
  the selected latest Jim required. Three existing Engine typed host-error/header
  controls pass after repairing the test compilation graph.
- The native runtime's six-file 8.5 capture has four completed files with 165
  passes, 34 skips and three assertion failures. Namespace/upvar stop on host
  refusals; their remaining cases are unmeasured. C controls have 500 passes,
  42 skips and zero failures. See the [current report](../../../experiments/name-handling/results/RESULTS.md).
- All 140 missing annotated test/catalog bindings were reviewed and reconciled.
  The complete proof audit has 1,179 questions, zero pending candidates and zero
  unlinked tests; 139 implementation evidence digests still require review.

## Concrete next work

Keep this list and the acceptance rows current at every coherent commit and
before handing off. Link exact raw results and distinguish passing controls,
remaining failures, unreached checks and source-only inspection. Research
scripts/capture validation passed for `df919cbed`; this does not repair the
publication gates or establish current engine/static conformance.

1. Repair conditional readonly import/export/path advice in the existing
   `source_catalogue_advice` / `source_transition_advice` owner. Preserve package
   availability and positioned namespace context, with known shadows, forget,
   dynamic mutation and unknown effects withdrawing definite advice. This is
   needed by semantic tokens and useful metaprogramming assistance; it grants no
   native dispatch, entered frame, successful load or rewrite permission.
2. Restore genuine-undefined-read detection in selected Tcltest caller-shared
   phases, for direct and captured-prefix aliases. Keep expected-result data opaque.
3. Provide actual native compiler recipes for the selected `namespace which`
   worker. Repair 8.4 indexed receiver, 8.6 command-entry epoch and 9.x local-alias
   handler prerequisites; bootstrap refusals cannot become Tcltest assertions.
4. Finish the measured Jim maximum variable arity and execute the VM declaration
   matrix. The original C 8.5 var.test failures are fixed in the frozen capture;
   original regex enumeration and broader VM/runtime conformance remain open.
5. Repair original cell/bootstrap prerequisites and the measured 8.5 namespace
   failures. The cached/fresh public question now matches C after real startup;
   escaped NUL production still needs its actual string/source owner. The byte-count helper must
   continue to read physical representation rather than reconstruct encoding.
6. Review the broader typing/class-relation/dispatch failures against actual
   source-entry prerequisites. Recover conditional candidates through shared
   facts; do not weaken negative controls or manufacture Native/Normal evidence.
7. Review changed implementation evidence bindings, formatting and owner/backing
   drift; finish required gates and publish. Never rewrite original native
   inputs or provider answers merely to match a new implementation digest.
8. Review remaining Jim adaptations and unreached setups separately; 11 reached
   original corpus answers are registered in the existing canonical proof index. Add held-out
   static consumer coverage and ingest matching F5 answers when the appliance
   chat returns them. Maintain the [wild-pattern catalogue](../../../experiments/name-handling/patterns/README.md)
   alongside each new experiment, answer or recognised idiom.

[Retained gate/scoped validation](../../../experiments/name-handling/results/current-validation/README.md)
separates passed controls, failed controls and gates which were not reached.

## Current bounded assignments

- W210: shared body-source controls pass; missing-read diagnostics remain red.
  Compiler debug investigation is active; temporary tracing must be removed.
- Source advice: five new focused controls pass; six broader owner failures
  need baseline attribution before packaged LSP validation.
- Runtime cells: the C 8.4 declared-cell repair awaits focused execution;
  original 8.6 command epochs and 9.x simple-cell startup remain unresolved.
- Jim variable arity: source and oracle-backed VM/runtime tests are ready;
  fresh execution is pending.
- Gate owners/backing and generated AI prompts: source fixes ready, gates and
  generation unexecuted after the interrupted build.
- Proof source review: syntax review is in `/tmp/name-syntax-proof-review.json`;
  final source freeze and explicit invariant/scope review are required before
  renewing implementation evidence. No blanket digest refresh.

Clean validation builds source `/tmp/name-env.sh`, which disables incremental
caches and dev/test debug info. The interrupted cache was removed; do not reuse
its artifacts or claim it ran a check. Coordinate source freezes and build
ownership; use small jobs and update this snapshot before handoff.
