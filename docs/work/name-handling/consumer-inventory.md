# Bounded name lookup consumer inventory

Current bounded callsite audit on `work`; captures bind exact source versions. This is a
callsite audit, not a claim of complete workspace coverage. Validation statuses distinguish
source inspection from execution.

[The surface roster](surface-roster.json) enumerates52 workspace and11 excluded
packages plus editor/script/AI surfaces. It is a reconciliation checklist;
package membership and lexical search absence do not certify semantic coverage.

The [Runtime audit](runtime-consumers.md) records actual policy/geometry and
physical-adapter boundaries. The [LSP/DB/server/editor audit](lsp-consumers.md)
adds23 bounded semantic priorities and an accompanying sealed roster of330
lexical candidates across381 tracked files. Candidate attribution is lexical;
unreconciled rows remain unfinished. These reports supersede stale individual
statuses below where explicitly stated; they are not whole-surface acceptance.

The [codegen/middle-end audit](codegen-consumers.md) reconciles actual bytecode,
WASM and BPF emitters, contract-only target families, and remaining #1778 map
consumers. String labels, DSL map symbols and genuine command slots have
separate contracts; a map audit is not execution acceptance.

[Registry consumers](registry-consumers.md) adds62 audited paths plus byte-prefix
and F5-domain preconditions. Its recorded source predates the latest exact
descriptor-roster and namespace.which edits, which are applied with validation
pending; those gaps are not completed simply because their proposals exist.

| Callsite | Operation/input/context | Shared owner/API and disposition | Tests/evidence or next action |
| --- | --- | --- | --- |
| `rust/tcl-cli/src/commands/lookup.rs::run_command_info` | User-provided exact registry command name + dialect + actual pack-layered store; JSON/human output | Implementation ready; consumer tests pending. Uses one `ResolvedContext::resolve_spec` call, removing independent trim/case-insensitive candidate scan and supplied-empty-name rejection. Retains literal output spelling; metadata, not live binding. | New `tests/cli.rs::command_info_preserves_exact_command_names` (2 positive/9 negative); existing project pack discovery control. No Rust test run yet. Original C5/Jim custom probe retained under `experiments/name-handling/results/cli-command-name-literal`. |
| `rust/tcl-mcp/src/tools.rs::command_info` | Original JSON literal string, fixed iRules context + actual store + optional release metadata | Audited source adapter: `context.resolve_spec` already preserves names. BIG-IP release whitespace handling is a distinct version input. | Existing `command_info_preserves_exact_literal_names`; retained `mcp-exact-command-name` results. No fresh execution here. |
| `rust/tcl-mcp/src/tools.rs::generate_docstring` | Original source/dialect + literal requested procedure declaration; span-preserving output | Audited adapter through `Core::procedure_symbol::for_literal_name`; distinguishes zero/one/multiple declarations. Catalogue, not call-time target. | Existing qualified namespace/disambiguation controls in tools.rs; no fresh execution. |
| `rust/tcl-mcp/src/tools.rs::read_proc_docs` | Retained analysis over actual source and original declarations; separate source bytes/name spans and display metadata | Audited adapter via `Core::procedure_symbol::declarations`; original record matched by span, not name. | Existing documentation controls; no fresh execution. |
| `rust/tcl-mcp/src/tools.rs::update_docstrings` | Original declaration inventory + editable source spans | Audited adapter via same Core inventory; reverse original-span edit order. | Inspect whole editing tests/currentness before declaring acceptance. |
| `rust/tcl-mcp/src/irule_gen.rs::SourceInventory::capture` | Source plus actual registry; context-owned event/command/configuration-reference metadata | Audited adapter `OriginalIrulesSourceContext::capture` + `presentation_commands`; reporting vector cannot independently recover execution. | Retained mcp-static-name-inventory results; own callback/static/configuration tests. No fresh execution. |
| `rust/tcl-mcp/src/irule_gen.rs::extract_variables` | Original source context/typed registry variable roles; presentation names | Uses source owner, not a cell/frame resolver. Do not replace with global/bare FQN scans. | Existing live static setup/inert exclusion controls; no fresh execution. |
| `rust/xtask/src/gen_ai.rs` | Diagnostic/dialect metadata rendered into AI templates/generated copies | Inspected bounded name search: no independent Tcl namespace target resolver found. Metadata generation adapter; templates still need separate semantic/documentation audit. | Generator drift gate belongs root; no generated files touched. |
| `rust/tcl-lsp-server/src/lib.rs::settle_call_against_workspace` | Explicit Logical retained source/context + ordered invocation candidates + namespace/import transitions + workspace index | Audited policy adapter consumes retained candidates, `ContextRegistry::resolve_command`, Core forced-import/pending-transition checks and index existence. Native/hosted source uses separate original evidence. Candidate loop is table existence selection, not local candidate generation. | Existing original_workspace_diagnostics_tests cover unsupported cross-document substitutions; consumer_audit owns wider callback surfaces. |
| `rust/tcl-lsp-server/src/lib.rs::refine_original_w123_diagnostics` | Original diagnostic subject + actual provider source access + package requirements/release | Audited adapter delegates `PackageResolver::{auto_loads_original_command,package_defines_original_command}`; scanned declarations are original slots rather than display FQN keys. | Inspect package source/load order invalidation controls before full acceptance; no fresh execution. |
| `rust/tcl-lsp-db/src/lib.rs::original_lookup_command_signatures_using` | OriginalCommandLookup with selected policy/namespace/path + original item header publication slots; Salsa project inputs | Audited structural adapter indexes each retained candidate and delegates `lookup.matching_slot_publications`; no local name normaliser. | Existing exact-original-header/replacement/collision tests; no fresh execution. |
| `rust/tcl-lsp-db/src/lib.rs::original_lookup_command_arities` | Same original lookup/policy and header arity projection | Audited structural adapter through same shared matching policy, not bare tails. | Original arity/signature database controls; no fresh execution. |
| `rust/tcl-lsp-db/src/lib.rs::merge_command_arities` | Logical declaration display keys and bare-tail assistance; item_sigs firewall | Existing `compiler::naming::key_tail` owner is reused. Tail-assisted policy remains in DB and must stay explicitly Logical; do not promote to identity. | Existing callback_command_keys Original guards; full consumer acceptance requires negative same-tail controls. |
| `rust/tcl-lsp-db/src/lib.rs::callback_command_keys` | Legacy Logical callback reference/candidates; raw absolute spelling only when no retained lookup | Existing owner for absolute `canonical_written_command`. Original input/prefix/lookup guards decline before fallback. Local namespace heuristics still need migration review: reuse retained shared candidates rather than newly reconstructing keys where context exists. | Existing original callback signature guards incl. lib.rs around 5364/9806/10886; no fresh execution. |
| `rust/tcl-registry/src/model/context.rs::ResolvedContext::resolve_spec_in_realm` | Exact metadata label + actual registry + authoring surface/realm, provider/package/version | Central metadata lookup authority through registry `get_for_surface`, full context availability and roster. This is not written Tcl runtime lookup. | Rooted fallback/package/context model tests; CLI and MCP now converge here. |
| `rust/tcl-registry/src/model/assembly.rs::ContextRegistry::resolve_command` | Metadata lookup over context-owned store and rootability gate | Registry-owned adapter; strips exactly one marker under its own roster rules. Source/slot identity cannot be inferred from return. | `resolution_applies_the_rooted_name_fallback`; fresh test absent. |
| `rust/tcl-registry/src/registry.rs::{get,get_for_surface,get_exact}` | Authored descriptor spelling and context-specific rooted fallback | Registry semantic owner; exact label table vs permitted rooted fallback are distinct operations. General command-resolution policy must not replace this catalogue metadata authority. | Registry method-only `my` vs namespaced Helpers/operator/profile tests. |
| `rust/tcl-registry/src/registry.rs::{declares_command_at,declares_command_at_rooted_key}` | Constructed reporting keys + exact descriptor labels | Audited adapter uses `syntax::naming::unroot_rooted_key`, `key_holder_and_tail`, typed CanonicalCommandDescriptorKey. Explicit no written canonicalisation. | Exact declaration/operator/method controls; fresh test absent. |
| `rust/tcl-registry/src/mathfunc.rs::global_command_bare_name` | Constructed global command reporting key | Integrated shared `Syntax::key_holder_and_tail`; exact global holder policy remains Registry-owned. No written runtime identity is inferred. | Current source inspected; execute affected consumer controls. |
| `rust/tcl-registry/src/mathfunc.rs::is_in_mathfunc_namespace` | Constructed command/holder reporting key | Integrated shared `key_holder_and_tail` for holder/member components; no private splitter. | Original dependency/consumer controls retain separate admission authority. |
| `rust/tcl-registry/src/private_tcl_namespaces.rs::classify_private_tcl_namespace_call` | Written literal head for private namespace diagnostic and selected public catalogue exception | Integrated shared qualifier component extents and canonical_written_command; actual surface availability and Jim refusal preserved. | Current source inspected; execute current diagnostic controls; no Native selection granted. |
| `rust/tcl-registry/src/dialects.rs::is_script_body_command` | Bootstrap authored version-scan recursion before dialect construction | Integrated written-name geometry and checked root projection; static recursion roster owns this bootstrap purpose. | Execute version-scan inert-data/body/qualification controls; no runtime command binding. |
| `rust/tcl-registry/src/native_compilation.rs::{namespace_code_grammar,namespace_origin_grammar,namespace_current_grammar,info_level_grammar,array_compilation_grammar,named_ensemble_grammar}` | Original native registration and compilation query; descriptor labels compared with one-strip or trim variants | Native owner boundary needs audit before change. Typed original input/selected registration descriptors must decide equality; display string canonicalisation cannot promote Native identity. | Selected native compilation proof records must be reviewed/resealed only after actual semantic validation. Do not blanket replace strips. |
| `rust/tcl-registry/src/native_string_compilation.rs` selected registry facade | Checked native identity label translated to metadata availability lookup via one strip | Potential sanctioned descriptor adapter; input producer/selected registration provenance must be audited. | Native string operation/version controls; no fresh execution. |
| `rust/tcl-registry/src/native_tcloo_registration.rs::compilation` | Selected original registration identity -> compilation descriptor via one root strip | Potential sanctioned descriptor adapter; same original-registration evidence requirement. | TCL OO runtime/VM accessor gaps stay separate unless directly caused by naming. |
| `rust/tcl-registry/src/value_transfer/reference_body.rs` defined/command equality | Independently repeated root trimming equates descriptor/body labels | Migration review needed: exact typed descriptor labels/checked source binding policy. Literal-colon and root marker identity may differ. ID string generation near848 is presentation only; equality near276 affects eligibility. | Value-transfer proof tests/source binding originals needed; do not conflate generated label with semantic comparison. |

Non-Tcl names excluded after inspection: CLI `pkg.rs` splits version `@`,
`gui.rs` filesystem extension/path formatting, CLI-support spec importer
case-folds source file extensions, registry F5 rule identity uses appliance
object path geometry. F5 rule/procedure `::` ownership remains in its domain
and is not plain Tcl namespace parsing; actual F5 questions belong provider
handoffs. Unrelated clock, allocator, glob platform and OO return/evaluation
compatibility failures do not become naming fixes merely because a suite
contains naming tests.

## LSP, database and workspace adapters

The following source inspection adds provider boundaries; it does not claim fresh
execution. Audit all callers and existing controls before marking verified.

| Callsite | Shared selection and retained input | Remaining completion |
| --- | --- | --- |
| Core `definition::resolve_called_proc` | Original providers delegate `original_declaration::select_at_offset` and retained procedure metadata. Explicit Logical providers consume retained references before lexical namespace/import advice. | Logical simple-tail fallback remains assistance only; import/currentness/collision tests and final comment cleanup. |
| Core `definition::lookup_var_read_at` | Explicit lexical-advice guard, original source applicability and one shared Core scope lookup. Original providers refuse this compatibility adapter. | Shared Logical source scope policy is integrated; original variable/provider and frame-specific refusal controls retain independent current execution obligations. |
| Core `definition::lookup_var_in_scope_chain` | Logical namespace-frame visibility delegates shared compiler logical_variable_scope_advice with actual input and original scope/uplevel chain. | Shared policy is integrated; execute current namespace/upframe and original refusal controls. Command namespace remains distinct from variable/frame identity. |
| Core `completion::array_element_completions` | Partial source operand plus scope adapter above; suggests recorded array indices. | Verify explicit Logical caller guard and partial-source grammar; completion metadata cannot establish original cell identity. |
| Core `rename::rename_var` | Matches actual document/config, source spans and Core scope adapter; collision gate checks a different declaration span. | Audit all ingress advice guards and original-variable rename path; same display or tail cannot authorise edits. |
| Core `graphs::resolve_invocation_target` | Original declaration/source allocation joins and issued bodies select the actual current declaration; lexical label-plus-offset selection is retired. | Core32 graph/body controls remain bounded and include failures; replay current original body-unit coverage repair. |
| Core `graphs::build_nodes` | Issued ProcedureId report rows retain body identity; original source joins have current owner guards. | DB Core13 lexical graph declaration and inlay type joins are integrated; current consumer execution pending. |
| DB `callback_command_keys` | Refuses original callback/input records; retained resolved slot/candidates precede explicit legacy absolute-name helper. | Existing legacy policy is bounded, not original lookup authority; test same-tail, ambiguous/colliding and missing-receipt controls. |
| DB `merge_command_arities` | Uses Syntax `key_tail`; complete key and bare-tail assistance are separately aggregated. Original qualified source spelling is checked before exact-key contribution. | Item signature firewall and original publication keys require full audit; bare-tail arity cannot select a live target. |
| Server workspace settlement | Retained candidates plus shared context registry and original package/transition facts, indexed document existence. | Current Document literal-positive/computed-negative autoload-definition test and load-order/cache invalidation acceptance. |

The broad naming searches currently produce223 name-related source paths and113
paths matching the focused lookup heuristics. These include tests, geometry,
presentation and providers; neither count is an audited consumer count. Reconcile
the complete workspace/backend/editor roster and every semantic match before
completion. The source manifests and exact acceptance receipts remain authoritative.

## Current residual audit dispositions

The [41-callsite audit](../../../experiments/name-handling/results/final-consumer-audit-20261014.json) is a bounded source audit, not acceptance. Debugger reference ingress, Logical method ancestry and iRules event/priority extraction now delegate their actual shared owners; fresh execution remains pending. F5 configuration-object resolution is integrated through explicit independent path policies; bounded software comparisons pass and appliance questions remain unanswered. Zero-production-caller GVN name APIs are retired; production GVN retains original-source purity controls. Explorer report joins retain full scoped body keys and every equal-label row.

## Current implementation and audit priorities

Current bounded snapshot: 2026-10-14T09:27:09Z. The [outstanding audit](outstanding-consumer-audit.md)
and machine inventory retain original issue and source observations.

The current shared consumers retain complete original source, analysis input,
configuration, Registry and issued body identity. Logical and Hosted readonly
cards, DB held FunctionUnit/card joins, Core linked navigation, OO creator/body
owners, SSA name/scalar owners and private source registrations are integrated.
Logical effect/frame consumers use genuine retained source advice when no
reached/compiler/declaration layout owner exists. A present stronger owner
keeps its old route or refusal. Branch joins retain this advice only when both
complete original receipts agree; nested children retain their recorded source
owner. Strict invocation metadata, Normal transfers and Native entry remain
independent.

Compiler58 builds and seals 542 unique selected controls: 388 PASS/154 FAIL,
zero source drift and no missing/empty selectors. Its exact plan of 349 controls
is 227 PASS/122 FAIL; the full var_escape filter is 161 PASS/35 FAIL across 196,
with three shared selectors. All original 337 Compiler51 controls are retained
and improve nine without regressions. The array-root liveness regression is
repaired; actual Hosted declaration/card identity, Logical source inventory,
event held-body ownership and scalar literal/mutation consumers improve. New
Native source-card positives, Native MAY dependency positives and protected
scope/sibling-body positives still fail; no whole consumer closure follows.

Compiler55/56 retain build-only failures with zero source drift; their actual
shared token API/test-profile corrections are integrated. Compiler57 builds but
runs no semantic controls because three new exact selectors missed the nested
test module; source and assertions are unchanged. The corrected exact paths
supply Compiler58. The integration-only fifth fixture accessor remains queued.
MCP52 stops in compilation before nine actual binary selectors, source drift
zero; its missing scope initializer is corrected. F5 production53 builds and
seals 32 controls: 9 PASS/23 FAIL (lib 3/6, model 6/25, original golden
differential FAIL). Its inventory records one new obsolete-agent-cache receipt;
zero total drift is not claimed. Diagnostic54 is build PASS/2 FAIL/source drift
zero; temporary diagnostics are removed, and actual producer repairs remain
required.

DB61 builds and executes all ten original controls: 0 PASS/10 FAIL, source drift
zero and no missing/empty selectors. Whole-file lattice shift regresses from
DB48; cold lower/optimisation counts, taint cascade counts, edit reuse and actual
memo/direct diagnostics remain failures. Quoted/braced callee edit still fails. Unchanged cold,
edit, supplied-input and uncached comparison assertions remain required. Core34
retains 39 PASS/25 FAIL across 64 controls with zero source drift; linked hover,
rename, graph producer and strict Native inlay remain open. Runtime09 retains
78 PASS/1 FAIL across 79 controls, all original 77 PASS and zero source,
compiled-input and artifact drift; the C8.5 error-code correction awaits replay.
The real native server build and eleven editor controls pass in their bounded
capture with zero source/output/binary drift. MCP41 dependency linking fails
before semantic controls; this supplies no MCP acceptance.

The F5 XC call/expression migration completes independent source review.
Canonical descriptors come from the actual supplied Registry and original
call, condition offsets or switch operand span. Missing/stale/foreign ownership,
literal bracket text and unavailable translation descriptors decline advice.
The v4 implementation and current contract are integrated; the original
differential/model capture is red. Selected descriptor pointer provenance
refuses authored overrides with absent, true or false translation flags. Fresh
custom getter controls keep the original pool independently available; their
authored override now retains the actual selected surface/lifecycle/traits and
asserts exact selected pointer provenance before refusal. The missing structured
condition producer remains open.
The Hosted structured source wrapper is integrated with complete retained analysis
input and exact original policy/context checks. It supplies guarded If/Switch
source grammar independently of runtime invocation and Native entry. Current
contracts, guarded arity, original retained fixture and body geometry corrections
are integrated; Diagnostic59 stops at a test-only context-helper type mismatch, zero semantic
controls and source drift zero. The actual helper correction and unused wrapper
API retirement are integrated. Diagnostic60 seals five controls: 4 PASS/1 FAIL, source drift zero and no
missing/empty selectors. All three original Native card controls and the original
source-advice join pass; protected source-frame scope still fails. Its actual
logs include temporary source-facade instrumentation despite an inaccurate
no-diagnostics source_channel label; the original receipt is preserved. Temporary
code is retired exactly before production work resumes. The subsequent Native
issuer guard requires complete original input, including genuinely same-context
changed analyser/unit refusal controls; those new controls await replay.

Appliance questions remain unanswered; software and C/Jim observations do not
establish Native F5 dispatch or execution.

Source registration gate45 builds and passes 44 exact controls, including the
15 new source-registration controls, with zero source drift and no missing or
empty selectors. The full owner CLI46 retains 202 failures from before the
ancestor visibility repair; the repaired full current manifest is unverified.
Thirteen initial and sixteen additional private source routes are integrated.
Fifty-four lexical candidates are actual public trait members checked by the
existing declaration gate; 149 candidates retain route/shape review obligations.
Private helpers cannot become public solely to satisfy documentation checks.
The Logical advice retainer records its actual Option.as_mut()/if-let issuer
route; this receiver shape remains unsupported by the current source gate.

The last strict proof audit reports 1,959 structural errors across 2,606 questions;
narrow owner-description corrections are integrated, and a fresh audit remains
required. Original provider, evidence, source-anchor, replay and census digests
are preserved. Changed-source evidence is not renewed by documentation edits.
The link/index mirror checks 3,259 Markdown files and 126,064 local links with
zero broken links or unindexed design/KCS entries; actual Rust KCS, diagnostic
tag and audience checks remain required. Original qualified Hosted-name
refusals and O124 source-edit applicability remain explicit. The separate Native-name source-only declaration body producer is integrated
with independent read-flow and frame-free source recipes. The frame-free recipe
retains the actual original transition tape, body/schema, ParamList topology, byte
slot and dialect namespace geometry; physical frame gates remain unchanged.
All original Native card controls await current consumer replay. Native MAY
dependencies and typed var-escape original-substitution consumers are integrated;
their exact actual static/type API routes and current agent guide are registered.
Dynamic heads retain all known dependency IDs; open alternatives supply no
return value, argument binder or selected call. Original-source scanner refusal
remains pessimistic and Normal frameless ownership stays independent. Its original
header/read-flow owner cannot borrow Logical or Hosted cards or supply an
installed allocation, publication, callable body or entered frame.

Native FunctionLattice-only correspondence is independently reviewed with full
current source entry, actual dependencies and injective relocation. Integration,
negative-control replay and whole BodyAnalysisCapsule transport remain pending.
The taint replay-report v3 and display-consumer source review resolves full current Module ownership
and synthetic-span provenance: complete source entry and actual retained Module
owner must match; residual spans require complete original/synthetic bounds or a
genuine registration anchor, with no borrowed original snapshot on unmapped
synthetic bytes. Actual whole-CU callback and per-function command-head scopes remain distinct
in Compiler/DB and Explorer/Core reports. The report/display and namespace
frame/context heap transport are integrated after independent review; new
controls and fresh fixed-budget graph replay remain required. Logical immutable
body inventory now delegates its positively issued source tokens to the existing
shared Realm stamping. Native child source grammar is integrated with full
input/origin/vector/card checks and explicit expansion refusal; its MAY
reader now uses a separate genuine deferred-body/vector purpose receipt and
checks actual requested original name bytes, poison/barriers and reached target
compatibility. Present child receipt selects this terminal source reader before
legacy lookup. Logical MAY summaries retain actual BodyWalk scope and current
issued body IDs; only an independently named descriptor without aliases or
uncertainty can exclude a procedure edge. Global/CFG readers consume IDs and
opacity only. Authored/Standalone source roles validate the complete original
vector/configuration/snapshot and actual retained Registry; variable-escape
consumers preserve literal empty/sigil names and unknown coverage. All three
consumer migrations are integrated after independent source review. Compiler62
seals build-only missing documentation failures with source drift zero; concise
API contracts are corrected. Compiler65 builds but executes no controls because the taint filter is empty;
its actual selector listing is corrected. Compiler66 stops at a Cow/String type
conversion, source drift zero and no semantic controls; the representation-only
conversion is corrected. Compiler67 is running the expanded current plan. No
semantic result is claimed for that run.
Shared exact Registry registration descriptor and Logical callback candidate
projections are integrated after independent source review. MCP path source categories
retain actual Event/Procedure kind separately from rendered labels; the typed
consumer migration is integrated and awaits original actual binary controls.
These source reviews establish no cache or whole-report acceptance.

The 52 workspace members, 11 excluded packages and inherited 330 lexical LSP
rows still require complete surface reconciliation. Original experiments,
failures, oracle sources, legal attribution and unanswered F5 questions remain
retained. No whole-codebase, Native, F5 or PR-ready acceptance is established.
This temporary coordination directory and its references are removed before
the merge-ready PR; durable documentation describes current contracts.
