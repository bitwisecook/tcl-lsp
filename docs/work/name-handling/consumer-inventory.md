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
| `rust/tcl-lsp-db/src/lib.rs::merge_command_arities` | Logical declaration display keys and bare-tail assistance; item_sigs firewall | Existing `compiler::naming::key_tail` owner is reused. Tail-assisted policy remains in DB and must stay explicitly Logical; do not promote to identity. | Existing original callback candidate guards; full consumer acceptance requires negative same-tail controls. |
| `SignatureCommandInvocation::logical_callback_candidate_keys` and three DB readers | Explicit Logical callback candidates under actual retained analysis purpose | Shared Signature owner retains the existing source/prefix/lookup refusals and candidate projection; three production DB readers delegate it. Original callback selection remains separate. | Original callback assertions remain required in current DB execution; source migration supplies no Native identity or complete callback closure. |
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
| DB Logical callback candidate readers | Delegate `SignatureCommandInvocation::logical_callback_candidate_keys` under the actual retained Logical purpose; original input/prefix/lookup refusals remain. | Retain original same-tail, ambiguity, collision and missing-receipt controls; candidate metadata is independent of original callable lookup. |
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

Current bounded snapshot: 2026-10-14T14:28:00Z on `work`.

Shared command, variable, body and source owners are integrated across compiler/CFG,
summaries, DB, LSP, server libraries, MCP/AI metadata, codegen and tooling. The
current migrations include exact source-read/definition Symbol/Version selection,
ValueKey hover/inlay joins, Registry-owned source name roles and substitutions,
strict Normal physical slots, typed word extents, procedure editing references,
minifier erasure/collision readers, shared code-lens member keys, Runtime namespace
byte addressing and literal catch/try and variable-name data. Empty and sigil names
remain data. Source MAY uncertainty does not establish a physical frame, callable
publication, represented read value or editing permission.

Authored declaration cards, child scopes and control grammar retain their actual
header/body/vector/input/entry/configuration/store. Hosted case grammar uses the
actual retained source configuration. Native children retain their genuine birth
tape, parent and distinct analysis attachment; independent cache-hit analyses cannot
donate that attachment. Missing or changed owners remain unavailable. Conditional
global-body candidate exclusion is withdrawn: descriptor metadata cannot close
alias/observer alternatives. Original #0 vocabulary, uncertainty and assertions
remain required.

MCP89 seals at12:17:04 with build101, zero semantic controls, zero source drift and
zero provider-closure drift. All five original C Tcl providers plus Jim pass matching
interpreter/library metadata checks using their original init/Tcltest libraries.
The build stops at two private compiler API accesses in minifier. The integrated public
SSA lexical facade keeps private rebasing inside compiler and preserves the separate
physical read/erasure guards; fresh MCP execution remains required.
No MCP semantic acceptance follows from provider metadata success.

Compiler90 seals12:44:33 with build101, zero semantic controls and zero source
drift. Minifier borrows a Registry through a temporary context Arc (E0716); the
repair must retain that actual context or use its existing borrowed accessor.
No current consumer assertion has run in this scope.

Compiler91 seals12:56:51 with build101 after Rust compilation reaches linking;
ld terminates with SIGBUS. Zero semantic controls execute and source drift is
zero. The genuine held-context lifetime repair is integrated; fresh linking and
original/new assertion execution remain required.

Current source integrates SCCP and instance-variable typed destination consumers,
positioned test-fixture minifier symbols, actual inline codegen admission, Jim cold
original-key/error-frame selection, and the declaration source syntax memo with
explicit original expansion refusal. Neutral syntax restores current receipts;
all original DB cold/edit/equivalence and Runtime controls remain required.
Compiler92 seals at13:43:45 with147 controls:123 PASS/24 nonpassing (19 assertion
failures, three depth SIGABRTs and two600-second timeouts), zero source drift and
no missing or empty selectors. Four source-cache controls pass (cold3/edit1/shift0/
current cached versus fresh equality and expansion refusal); SCCP/instance typed
writes, Native child ownership and the original forty-case alias setup also pass.
Namespace import, SSA/word/receiver/scalar/dictionary, Native certificates, selected
codegen hooks, two substitution helpers, global-body escape and Logical substitution
remain red. The unchanged original #2286 and analyser-cap controls time out at600
seconds. Original source/assertions and code budgets remain unchanged. These
results establish no complete consumer/cache or #2286 acceptance.

Current source additionally integrates Logical class/member receipts, shared current
Document/outline guards, dynamic alias/scanner and Shimmer name-data consumers,
exact token attachment joins, SCCP phased Call/destroy inventories and shared
completion reference extents. Genuine root source-image issuance and the new
CFG/dictionary fixture producers preserve existing assertions. Core93 stops at a
private test API access before semantic execution; the public purpose premise is
corrected. Core95 builds successfully, then the initial selector mismatch executes
zero controls. The corrected same-binary, expected-source replay seals at13:56:53:
92 controls,66 PASS/26 FAIL, zero source drift and no missing or empty selectors.
The original workspace member and deep graph controls and new current-source
outline control pass. All71 original Core84 controls remain, with two gains and
zero regressions on that common set: workspace member and qualified Logical
variable scope. Code-lens, completion, hover/inlay and remaining Core failures
remain visible. The distinct inputless Authored definition owner and actual header
attachment transport are now integrated; their new controls remain unexecuted.

Compiler88 builds and seals113 controls:96 PASS/17 nonpassing (13 assertion failures,
three depth SIGABRTs and one analyser-cap timeout), zero source drift and no missing
or empty selectors. Current source also integrates scalar-only CFG name-data projection and nonrecursive
Native cold preparation, preserving array-root invalidation and original budgets.
Scalar assignment escape consumers now retain actual Script/SSA tokens and selected
FIRST_ARG+VarWrite name data with exact IR-field correspondence and unchanged unknown
residual. Dictionary destinations use the selected hook/role/direct written slot and
the same schema for safe-on-uninit; unavailable ownership remains generic dispatch.
The current partial replay retains failures in these consumers; fixes must pass
the unchanged original and new controls. Other
retained scopes remain distinct: Compiler82=194 PASS/46 nonpassing across240,
focused83=6 PASS/6 FAIL across12, Core84=48 PASS/23 FAIL across71, and broader
Compiler67=442 PASS/157 FAIL across599. Both original eval/up-frame regressions pass83;
new minifier/linked controls pass84 with all original68 Core controls retained.
These results establish no complete consumer acceptance.

Original #2286 C8.4 still records1 versus2, W210 at bytes160–167; unchanged82 times
out after180 seconds. Namespace import remains0 versus1. Original matching
Tcltest programs, all forty alias setup cases, assertions and raw results are
preserved. DB61 retains ten original failures, including cold0 versus3. The genuine source-body
syntax request/callback is now integrated; actual cold/edit/uncached equivalence
replay is required independently of lattice/full-capsule equality. Runtime09 retains
78 PASS/1 FAIL with all original77 PASS. F5 production53 retains9 PASS/23 FAIL across32
and one actual experiment receipt delta; appliance answers remain outstanding.

Runtime97 and98 retain zero semantic controls and zero source/provider closure
drift. All six matching original interpreter/library metadata checks pass. Runtime97
stops at a test-only Code path; Runtime98 builds runtime successfully and stops at
a Registry test source-slice coercion. Both exact representation repairs preserve
original bytes/assertions. Runtime99 is sealed:95 of99 controls pass; four retained Jim bootstrap/key/error-stack/import controls fail. Full captured source/provider, compiled-input and artifact drift are zero. Matching
C8.4–9.1 libraries and the verified latest Jim remain independent oracle owners.

Runtime audit rows26 and29 already use shared rename-destination and import metadata
owners, retaining actual generations/tokens rather than reporting labels. OO
namespace operands now use the checked original object/context cache and shared
byte-address owner. Current Runtime/codegen consumer replay remains required.

Remaining work is current Native/Hosted/Authored producer replay and syntax memo
transport; exact SSA read/definition/hover availability; namespace path/import MAY
candidate completeness; original depth/graph budgets; all DB cold/edit/direct memo
controls; all target/runtime consumers; actual F5 answers; and complete consumer
registration. The52 workspace members,11 excluded packages and330 lexical rows
remain reconciliation obligations. Source gate45 passes44 exact controls including15
registration controls, but full current private-owner registration is unverified.
The last strict proof audit retains1959 errors across2606 questions. Unsupported
routes remain explicit; private APIs are not exposed to satisfy the gate.

Final cleanup removes development chronology from durable documents/comments while
preserving original experiments, source programs, failures, legal provenance and
protected evidence/provider digests. Comments explain only current how/why. This
temporary tracking directory and its references are removed before the merge-ready
PR. All required formatting, generation, proof, consumer, cache and provider gates
must pass; no complete cache, Native, F5 or PR-ready acceptance is claimed.

Current15:58 BST receipt: Runtime99 remains95/99 with zero captured source/provider/compiled-input/artifact drift. VM original key/publication migration is independently reviewed; runtime Jim registration producer fixes and bounded retirement await integration. Compiler101 linker failed without semantic execution. Compiler102 links successfully; two original/shared TclOO controls pass, then a new isolated global read equality fails and its large existing Debug assertion output interrupts capture. Raw output is retained compressed with hash; no whole-plan acceptance. Compiler103 reuses that exact diagnostic binary under an expected-source comparison and seals10 original SSA/certificate/Authored controls:5 pass/5 fail, source drift0. Foreign-owner refusal, direct source reads and three Authored controls pass; definition/readonly-version and three Native certificates remain red. Historical92 #2286 phase diagnosis completes two analysis cycles then times out during subsequent source preparation, with zero source/executable drift and unchanged original probe. Temporary traces are removed before ordinary replay. VM original key/publication consumers are integrated with verification pending. Next tracking16:13 BST; hourly16:13 BST.

Current integrated consumers also include original VM table-key/publication/local restoration and selected error frames; Runtime original byte registration and bounded retirement; Logical procedure insertion with genuine header cards/current workspace source; complete selected source-expression write names with additional effect and trace residuals; and Native source child inventories through their own word/site owner. Original controls remain unchanged and ordinary affected replay is required. VM105 is the active full35 replay; actual import-key producer and Named source point/definition correspondence remain parallel implementation obligations.
