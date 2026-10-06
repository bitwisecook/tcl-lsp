# Contract: naming consumer matrix

This matrix describes the naming interfaces used by the compiler, interpreters
and editor providers. It complements the [shared owner inventory](shared-utility-contracts-rust.md),
the [resolved query contract](resolved-semantic-queries.md), and the
[implementer guide](../compiler/name-resolution-implementer-guide.md).

Name bytes, written names, constructed locations and live allocations are
different coordinates. `NameBytes` retains bytes; `ByteNamespacePath` retains
already constructed components; `ByteCommandSlot` combines those components
with a simple name. None identifies an interpreter allocation. A native name
projection selects one operation's input extent and qualification. It supplies
no command existence, frame, namespace token, cell lifetime, provider lifetime,
observer closure, current implementation, compiler selection or editable span.

## Shared interfaces and their consumers

| Surface | Current owner and interface | Consumers and purpose boundary |
| --- | --- | --- |
| Exact byte storage and constructed geometry | `tcl-core-types::name_bytes::{NameBytes,ByteNamespacePath,ByteCommandSlot}` | Syntax naming, compiler source scopes, runtime API snapshots, VM/Runtime name tables. Checked UTF-8 is presentation evidence; display text cannot recover component boundaries. |
| Engine and naming policy ingress | `tcl-syntax::naming::{ExecutionNamePolicy,NativeNameProtocol,NamePolicyProtocol,NamePolicyAuthority}`; `InvocationDialect::{native_name_protocol,authored_name_policy,authored_logical_name_simulation}` | Compiler runtime entry, signature scanner, autoload evaluator, VM and Runtime. The independent execution policy selects native C/Jim, explicitly authored simulation or bounded observed context. It does not derive from the producer-string protocol. |
| Command lookup input and publication | `NativeNameProtocol::{command_lookup_input,command_lookup_slot,command_publication_slot,command_c_api_publication_slot}` | Original-source command binding, signature declaration scope, VM/Runtime command registration and dispatch. Publication and lookup use separate purposes; a publication key is not another written lookup. |
| Current command lookup and navigation | `SourceCommandBindings::invocation_at_source`; `SourceInvocationBinding::{proved_target,command_reference,linked_definition,lookup_command_word,selected_slot_presence}` | Compiler semantics and positioned LSP definition, references, rename, hover, signatures and call hierarchy. Called slot, alias terminal and current declaration allocation are distinct projections. |
| Early compiler selection | `NativeCompilationSpec`; `SourceInvocationBinding::native_compilation_admission_selection`; `CompiledNamespaceContext` | Lowering, CFG construction, codegen, bytecode and VM/Runtime replay. Selection before word evaluation cannot be replaced by later live dispatch. |
| Namespace addressing and reporting | `NativeNameContext`; `NativeNameProtocol::{namespace_address_path,jim_namespace_canonical_input,jim_namespace_construction}`; native namespace-object owner | Original-source entry, signature scopes, VM/Runtime namespace command and object caches. Actual namespace geometry or Jim's original namespace object survives transport; rendered names are separate. |
| Namespace imports, exports and forget | `NativeNameProtocol::{namespace_pattern_input,namespace_pattern_parts}`; `command_binding::namespace_slots`; `SignatureNamespaceImportSource` | Source command lifecycle, analyser and signature records, workspace import index. Pattern purpose extent and complete colon-run geometry are selected by the shared owner; scanner records retain C components or Jim flat source. Export snapshot, force replacement and later forgetting remain separate facts. |
| Scalar/root and element names | `ExecutionNamePolicy::variable_input`; native `NativeVariableProjection`; bounded `ExecutionVariableNameProjection` | VM/Runtime variable ingress and compiler point contexts. Combined `root(index)` and explicit root/index have independent parsing and storage purposes. |
| Physical variable identity and contents | `ResolveContext`; `PointResolveContexts`; `var_resolve::resolve_place`; `SsaSourceView::{read_reference,read_word,reaching_binding}` | SSA, effects, native opcode admission, diagnostics and variable reference providers. Actual frame, namespace token, allocation generation, alias target, root/index and represented store remain separate from the displayed name. |
| Global, namespace variable and upvar local names | `naming::{global_local_name_bytes,variable_local_name_bytes}`; `state_transition::local_alias_name`; native namespace-upvar owner | Registry transition facts, compiler declaration assistance, VM/Runtime link creation. Local alias spelling does not establish a completed physical link; Jim rooted `global` creates no local alias. |
| Frame selection and uplevel | Registry `frame_effect`; native frame-reference owner; source entered-frame selection | Source analysis, lowering, VM and Runtime. Cached level syntax is evaluated against the current caller chain; namespace equality does not establish activation identity. |
| Alias creation, target lookup and rename | Native command naming projections; registry `CommandBindingTransition`; `AliasTargetLookup`; command token lifecycle | Source kernel, analyser indirection, signature records, workspace links, VM/Runtime command mutation. Written global targets, caller-relative targets and constructed interpreter-domain keys differ. Rename source lookup differs from destination publication. |
| Formal storage and compiled local names | `NativeNameProtocol::{formal_storage_name_input,formal_enumeration_name_input}`; `NativeCompiledVariableProtocol`; `LocalVarTable` | Procedure analysis, compiler slot allocation, bytecode, VM and Runtime formal binding. Counted declaration names, compiler comparison and dynamic hash equality differ; ordered compiled slots can contain repeated names. |
| Variable trace receiver and callback report | `NativeNameProtocol::{trace_registration_input,trace_query_input}`; `report_native_variable_access_trace_names`; native variable-observer owner | Registry/command helpers and VM/Runtime trace registration, introspection and execution. Receiver lookup, linked cell, callback name1/name2, active-chain suppression and trace lifetime are independent. |
| Command trace and mutation epochs | Native command-object/cache owner and native command trace owner | Source semantic withdrawal, VM/Runtime command lookup, rename and trace callback dispatch. A matching name or surviving token cannot revive a replaced implementation or retired cache. |
| Package names and loaders | `NativeNameProtocol::package_key`; shared package/version owner; `TrustedPackageLoader` | Registry package facts, source binding, autoload evaluation, LSP package resolver, VM/Runtime package operations. Package advertisement and literal `require` do not establish the loaded provider, exact version or installed surface. |
| Command inventories and patterns | Native glob owner; native command-name/object owner | VM/Runtime `info commands`, `info procs`, namespace exports and command caches. Exact-key lookup and pattern scans have different native extents; map sorting supplies no native inventory order. |
| Variable inventories and lifetime | Native hash ABI/table owner; `NativeEntryLedger`; `Frames::var_names_bytes_checked`; `Namespaces::vars_in_bytes_checked` | VM/Runtime `info vars`, `info globals`, `info locals`, arrays and namespace introspection. Physical entry births, undefined trace shells and compiled slot order are retained independently of current values. |
| Name diagnostics | `report_native_name_bytes`; native variable/namespace diagnostic projections; native instruction-name owner | Shared command handlers, VM and Runtime failure presentation. Diagnostic clipping and formatting do not replace the counted lookup key or actual failure stage. |
| Source variable lexer and nested source | `LexerConfig::with_grammar`; `CommandTokens::native_lexer_config`; native parse-context owner | Lexer, parser, source kernel, nested body parsing and LSP source projections. Unbraced lexical acceptance differs from braced full-name runtime lookup; producer bytes are not inferred from Unicode spelling. |
| Workspace command resolution | Signature `resolved_command_reference`, `resolved_definition`, `resolution_candidates`, `source_name`; workspace settled-sites and import/link index | Cross-file definition, references, rename, completion, hover, signature help and diagnostics. Positioned source receipts precede workspace assistance; exact authored geometry precedes presentation-key fallback. |
| Incremental project callback diagnostics | `tcl-lsp-db::{project_command_arities,command_arity,project_callback_diagnostics}` | Server opt-in callback arity and database corpus checks. Complete constructed declaration keys and bare-tail advice are separate. Positioned reference slots and ordered candidates precede written qualified assistance. Relative-qualified callbacks without retained lookup withdraw. Bare tails remain explicit assistance without execution/provider authority. |

## BIG-IP context evidence

`f5::BigIpExecutionContext` distinguishes TMM iRules, tmsh CLI scripts, iApp
implementation, iCall scripts, host-shell Tcl, iApp presentation/APL and
presentation Tcl callbacks. `f5::storage` applies a host storage policy after a
namespace cell has resolved. Its root `::static` worker classification supplies
no naming policy, worker selection, initialization publication or value equality.

The [appliance follow-up report](../../../scripts/dev/bigip-probes/resolution-2286/FOLLOWUP_APPLIANCE_RESULTS.md)
records naming controls on BIG-IP `21.1.0.1`, build `0.0.26`, in one TMM group
with four units. Dynamic naming controls ran in TMM `HTTP_REQUEST`. They do
not establish name behavior in CLI scripts, iApp implementation, iCall, APL or
presentation callbacks. Only the independently bounded expression payload was
repeated in CLI, iApp implementation and one triggered iCall handler.

| Measured purpose | Retained observation | Evidence boundary |
| --- | --- | --- |
| Dynamic scalar/root and array index identity | Embedded NUL remains counted and distinguishes variable/root/index storage | Runtime names in reached TMM HTTP events; no literal-source NUL or other-context grant |
| Byte-producing `format %c` | `233` produces byte `e9`; `769` produces byte `01` | Producer behavior does not identify a Unicode normalization or native C string recipe |
| Unbraced and braced variable syntax | Literal unbraced acceptance follows the measured ASCII lexical surface; braced full names resolve | Lexer acceptance and resolved cell identity are separate purposes |
| Activated procedure replacement | Deleting configuration leaves the reached active procedure; recreation replaces fresh-call body/formals; rejected replacement preserves prior callable | Fresh invocations on the reached TMM units; no already-entered suspended-frame lifetime claim |
| Bare expression `matches` | The measured discriminators select whole-string glob behavior | Exact expression payload/context coverage; equality alone does not distinguish the operator |

`NativeNameProtocol` remains the infallible C/Jim recipe owner. The independent
`ExecutionNamePolicy::ObservedBigIp` carries an exact `MeasuredBigIpNameScope`;
`f5::naming::observed_name_policy` admits only the measured build, TMM iRule
context and HTTP_REQUEST event. Its operation-bounded counted recipe accepts
already-produced nonempty dynamic unqualified scalar/array roots without colon
or parenthesis grammar, and combined `root(index)` with a nonempty index and
no nested parentheses. Root and index retain NUL and raw producer bytes. The
measured discriminators support this bounded naming algorithm, without
establishing every byte/length invariant. Separate forms, empty operands,
qualification, static namespaces, command names and formals remain unavailable. An observed projection grants no physical storage receiver, native
cache, procedure-header compiler or compiler-local provider.

`NativeCompilationEntry`, `SourceAnalysisEntry`, `SourceAnalysisOptions`,
`BindingBaseline` and `ResolveContext` retain this issuer independently. Native
command consumers request `command_name_policy`, which retains a separately
supplied native or authored command provider and declines observed-only command
authority. An authored command simulation grants no measured CPP or cache facts. `ProcDef` and `ItemSig` retain `SignatureSourceCommand`; source
scopes retain original namespace geometry. Completion inserts a source spelling
only after both publication and written-lookup round trips select the retained
slot, then renders that word through the shared Tcl list owner.

## Remaining consumer boundaries

These are current limits of the interfaces above, rather than permission to
infer a resolved name from presentation text.

| Current path | Limit |
| --- | --- |
| Namespace pattern analyser assistance | Scanner and analyser import/forget consumers require retained original namespace geometry and independent command-name policy. Missing scope/purpose evidence withdraws; presentation adapters supply no physical token. |
| Namespace navigation and refactoring | Namespace references require retained original source/context geometry for component edits. Rendered namespace keys cannot recover colliding partial-colon component paths. |
| Workspace import source transport | `SignatureNamespaceImportSource.native_source` travels through import-edge lifting and target settlement as exact C components or Jim flat source. Export/forget lifecycle grouping uses presentation keys and cannot supply an exact execution or writable-reference receipt. |
| Empty relative namespace addressing | The shared C address helper declines an empty relative selected operand in a nonroot context. Global empty addresses and rooted trailing-delimiter addresses remain separate. Jim's flat namespace owner retains its independent behavior. |
| Name operand producer selection | Backend operand materialisation, formal splitting and namespace variable containers retain independent producer-string issuers. The naming issuer cannot grant physical producer, header or CPP facts. Measured BIG-IP naming supplies no `format %c` producer replay. |
| Class hierarchy and class source consumers | `ClassDef` retains object/provider-advice publication slots and original relation/metaclass lookups. Shared source queries and workspace relation edges compare exact retained slots and withdraw rendered collisions or missing contexts. Constructed-graph compatibility, workspace class report queries and local/index receipt reconciliation have narrower advisory boundaries; provider advice proves no executable constructor or allocation. |
| Logical F5 variable storage | The measured issuer and native recipe are separate. Physical counted storage requires its own context-specific receiver capability; projected keys alone cannot establish a cell. Compiler place resolution withdraws observed physical cells because runtime event arenas do not issue a source/native frame token. |
| Shared owner gate | The inventory validates source owners and named gates and checks several physical name consumers. The native name input/reporting row currently has no specific gate; it does not prove absence of every local string-based naming rule. |

Consumer authority remains bounded even where a compatibility projection exists.
An advisory tail or namespace guess cannot establish executable dispatch,
physical storage, provider survival or writable-reference consensus. A retained
negative lookup remains distinct from missing evidence, and a supported narrower
purpose does not close an unsupported execution context.

## Native evidence coverage

The basic command, variable and namespace vector tables cover the five pinned
C Tcl releases. `jim_name_resolution_conformance` has independent original
Jim controls for publication, ordered lookup, namespace geometry, global alias
spelling and counted scalar/combined receivers; its inputs and expectations
are independent of those C tables. Jim's Tcl array commands use combined
variable input, so those controls grant no separate root/index API semantics.
Execution/body controls retain separate Jim cases and capabilities; header and cache manifests remain release-specific. A selected
unit-test filter count is not evidence that those tests executed. Native Tcl
and Jim source/result-byte captures are controls until the corresponding owner
and consumer comparisons run. Pure-model, positioned consumer, LSP and broad
cache coverage remain separate proof obligations.
