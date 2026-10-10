# Using resolved Tcl semantics

Use this guide with the [semantic query contract](../contracts/resolved-semantic-queries.md), the [owner inventory](../contracts/shared-utility-contracts-rust.md), and the [evaluated Tcl semantics](evaluated-tcl-semantics.md) reference.

## Start at the owner and follow the consumer path

The following routes are entry points into this guide. Start with the affected
semantic axis, then use the purpose table to select a query. Each query's
contract states its required evidence, permitted uses and limitations.

| Semantic axis | Read first | Follow through |
| --- | --- | --- |
| Commands, aliases, imports, renames or missing-command fallback | [Original lookup](#resolve-dynamic-command-names-at-their-actual-invocation) | [Navigation](#navigate-through-the-called-slot), [compiler admission](#keep-compiler-admission-separate-from-execution), original argv and dispatch |
| Variables, globals, namespace cells, upvar or uplevel | [Storage and contents](#resolve-storage-and-values-separately) | Captured read address, reaching store, cell lifetime, observers, SSA and runtime address |
| Script operands, callbacks or handler completion | [Evaluation boundaries](#preserve-source-evaluation-and-lookup-boundaries) | [Completion](#route-completion-from-the-same-frozen-invocation), original role positions, body frame and diagnostic assistance |
| Packages, availability or dialect-dependent syntax | [Actual entry](#carry-the-actual-entry-through-every-layer) | [Host realm](#retain-host-availability-and-invocation-realms), exact provider/version, lexer and nested-source entry |
| Expressions, native object conversion or rewriting | [Expression preparation](#prepare-expression-functions-using-their-native-owner) | [Operand and result receipts](#native-scalar-math-results-and-original-argv-addresses), object methods, conversion effects, erasure and both backends |
| iRules static/global ownership | [Execution domains](#preserve-irules-execution-domains) | Initialization publication, current TMM, event/connection activation and interprocedural caller domain |
| Shared facts | [Discriminating experiments](#native-regression-requirements) | [Consumer review](#consumer-boundaries-of-shared-facts), owner contract, DSL/rendering parity and gates |

Each shared function's contract identifies its inputs, the exact fact it returns,
the observations that withdraw that fact, and each consumer's permitted use.
Keep caller-purpose projections over one retained owner receipt; separate
projections must not reconstruct lookup or parsing independently. Keep actual
engine, source instance, invocation site, frame, original operand layout and
relevant allocation generations in that receipt. Add only the dependencies the
fact actually needs, so navigation is not needlessly blocked by an unrelated
execution obligation.

A query must distinguish missing knowledge from a proved negative. Closed empty
coverage differs from unobserved coverage; a normal result differs from guaranteed
completion; compiler-selected execution differs from handler lookup. If a new
case is unsupported, retain that residual at the shared owner and propagate it
through every consumer. Preserve an already established narrower fact. Use a
heuristic only in an explicitly advisory projection after the exact query has
been attempted; it cannot fill an executable, writable-reference or erasure
receipt.

Each owner contract identifies its query, consumer purposes, native positive
and discriminating negative cases, and unsupported inputs. Compiler, codegen,
VM/runtime adapters and editor providers consume the same owner rather than
reimplementing its rules through local name checks, parsers or final-environment
maps. Registry metadata has corresponding DSL, renderer, Studio and runtime
backing, with precise unsupported surfaces recorded in `GAPS`.

## Select the proof the consumer needs

### Shared query inputs and results

A shared function should answer one semantic question over an original owner
receipt. Its signature must retain the coordinates that identify that question:
actual interpreter and dialect, source instance and site, evaluation phase,
frame or allocation owner, and original operand origins. A command name or
displayed variable name cannot stand in for those coordinates. For example,
`invocation_at_source` retrieves an original dispatch, whereas
`lookup_command_word` resolves another value in that dispatch's retained world.
The latter does not create another invocation.

The return contract distinguishes an unknown result from a proved absence,
rejected invocation, empty inventory or
normal continuation. If `Option` is sufficient, document exactly what `None`
and `Some(empty)` mean. A timing query's closed empty positional inventory is
not permission to erase a callback; a successful representation query describes
the normal continuation, not whether that continuation is reached. Use a typed
variant when these meanings cannot be represented unambiguously.

Keep original, effective and selected-member operand spaces explicit. Alias
prefix insertion, ensemble selection and receiver-method selection change the
effective layout. Project values, object receipts, roles, physical reads and
source origins together through the same mapping. A captured prefix operand
has no written edit position. A consumer must not repair an index with a local
`+ 1`, remove a selector from just one vector or assign a captured object the
source span of a written word. Expansion can make cardinality unknown while
retaining useful bounded source metadata; that metadata cannot supply a frozen
argv layout.

Build candidate facts once at the retained boundary, then derive purpose
projections from that collection. Possible navigation, unanimous grammar,
normal effects and executable selection require different evidence. Reusing
the collection must preserve each projection's unknown alternatives and actual
dialect policy. Do not cache a projection across source sites or frames merely
because the spellings match. Private receipt construction and narrow accessors
help prevent an advisory consumer from supplying executable authority.

`RegistryInvocationAssistance::unanimous_command_words` projects only the
command identity, original effective words, source origins and frozen alias
prefix shared by every closed candidate. It rejects an empty candidate set,
an unknown residual, possible absence, or any disagreement in those fields.
Candidate-specific roles, operations and effects do not travel through this
view. URI advice uses it because distinct retained lookup alternatives can
agree on diagnostic words. Executable selection, writable references and body
execution must use their own purpose-specific queries; they cannot promote
this advisory view. Keep controls for disagreeing values, origins and prefixes
alongside the positive convergence case when extending this projection.

Invalidate facts at the owner that observes the change. Dependencies should
name the relevant interpreter, allocation, command or namespace token, cell
lifetime, contents version, observer registration and dispatch generation.
An unrelated command definition should not permanently erase a retained stock
registration; a relevant replacement or trace must withdraw its guard. A
normal conversion can preserve contents while changing the same object's
physical cache. Keep those two updates separate from value inference and from
world-effect closure.

Conversion APIs also describe failure transitions. Publish original string
materialisation and cache mutation in the native order before presenting a
guest failure. An error-code update has an explicit `Unchanged` case distinct
from setting `NONE`. Primitive getter diagnostics and propagated script
completion are different stages; use the selected engine's unit and clipping
rules at the stage that owns them. Neither a registry error template nor a host
admission refusal can replace the original guest completion.

Consumer tests cover a real positive and a discriminating withdrawal through
each affected layer. Conversion-sensitive tests assert the retained conversion
order as well as the resulting value; direct-operation cases require their own
proof. Recursion and time budgets are part of the analyser's contract. Receipt
storage must preserve those bounds, including the size copied into recursive
frames.

A command spelling, an available registry description, an implementation identity, a selected native operation, and a successful normal transfer are separate facts. Each query must retain its unknown alternatives. Select the narrowest contract that answers the consumer's question.

| Consumer purpose | Shared query | Permitted use |
| --- | --- | --- |
| Actual lexical ingress | `LexerConfig::with_grammar`, `Module::native_lexer_config`, `CommandTokens::native_lexer_config` | Select actual lexical axes before segmentation or nested parsing, preserving coordinates and caller modes; unknown execution axes keep explicit configuration. Syntax grants no execution identity. |
| Host availability phase | `InvocationRealm`, `SurfaceQuery::with_realm`, `SourceInvocationBinding::invocation_realm` | Select rule-loader policy or the audited interpreter table; neither establishes compiler identity or body entry. |
| Name candidates and lookup order | `tcl_syntax::naming` | Search actual namespace/path entries using interpreter command existence. |
| Compiled binding and transparent replay namespace | `CompiledNamespaceContext`; `SourceNamespaceKey::to_compiled_context`; `Instruction::source_namespace_context`; `NativeOperationSelectionSite::replay_namespace_context` | Preserve actual native identity or exact constructed components through dependency and instruction transport; validate token/path/lifetime at use, and never parse presentation to recover a context. |
| Procedure publication | `native_procedure::{procedure_name_publication,published_procedure_key}` | Preserve the selected namespace owner or the actual rendered-name creation protocol; a constructed declaration key is not another written lookup. |
| Aggregate original-source command effects | `ModuleCommandBindings::analyse` | Reuse the lowering-owned projection only through its complete source, grammar, registry, entry, frame and compilation-scope guard; changed inputs re-enter the shared original-source owner. A namespace spelling cannot identify an actual activation. |
| Implementation at a source invocation | `SourceCommandBindings::invocation_at_source` | Retrieve the exact source instance and dispatch boundary; a missing site remains unknown. |
| Original statement invocation carrier | `Script::retained_source_tokens_for_statement` | Recover a direct or consumed typed-lowering carrier only for a statement owned by that script; conflicting carriers decline. CFG clones use `Function::source_tokens_at` at their actual block and index. |
| Live dispatch after words evaluate | `SourceInvocationBinding::proved_target` | Establish the post-argv target and alias prefix. |
| Navigation to the called slot | `SourceInvocationBinding::command_reference`, `SourceCommandReference::{slot,binding}` | Retain the actual slot and direct/imported binding; alias target execution, implementation guards and object classes require independent proofs. |
| Terminal declaration through a reached alias/import | `SourceCommandReference::linked_definition`, `SourceInvocationBinding::linked_definition` | Retain the original unobserved lookup chain and exact source declaration; cannot replace a missing edge with a later link or borrow a direct-definition/rename licence. |
| Selected slot presence | `SourceInvocationBinding::selected_slot_presence` | Distinguish present, absent, may-present and unknown independently of alias target validity and missing-command fallback. Candidate lookup and navigation share `ModuleCommandBindings::source_lookup_paths`; consumers must not rebuild path alternatives. |
| Another computed command value at the same invocation | `SourceInvocationBinding::lookup_command_word` | Resolve in the retained namespace and table; cannot add another execution, substitute a default registry or search unrelated namespace tails. |
| Exact pre-fallback command absence | `SourceCommandBindings::command_is_absent_at` | Prove a missing table entry at that site; does not prove that its missing-command handler fails or has no effects. |
| Logical structured analysis | `logical_structured_invocation`, `LogicalStructuredInvocation` | Consume the exact selected handler or a declaration's owned conditional frame, selected facts and original argument indices. Conditional structure grants no actual activation, outward store or native opcode. Source-shape hooks decline inserted prefix operands, rewritten selectors and expanded elements they cannot map exactly. |
| Declared formal input and direct argument components | `SourceCommandBindings::symbolic_declaration_formal_value_at`, `symbolic_declaration_formal_components_in_word` | Select the immutable original declaration frame for parameter-role and caller-frame advice. Ordinary incoming formal slots and source-attested copies retain that conditional input; changed declarations, writes, observers, caller links and conflicting alternatives withdraw it. Direct outer argument components exclude nested command-result reads. No entered-call values or actual caller stores follow. |
| Original formal input with entered-frame evidence | `SourceCommandBindings::symbolic_formal_value_at` | Preserve the original source/read receipt and independently selected entered-frame evidence for actual or joined purposes; declaration-only advice cannot substitute for those observations. |
| Caller-name navigation templates | `SourceCallerFrameInvocationTemplate` | Match the original selected call, declaration allocation, exact caller frame and shared Tcl/Jim formal binding plan. Map only editable original literal operands; captured prefixes, default/rest values and caller links supply no editable literal argument. This is scoped symbolic navigation, never a physical caller-cell or completed-write receipt. |
| Captured private name with late handler lookup | `SourceInvocationBinding::admitted_named_invocation`, `native_named_command_words` | Emit the selected name and original remaining words; resolve its handler after argv, retaining compiler prerequisites separately. |
| Private-handler source operand layout | `named_arguments::ProjectedNamedArguments`, `SourceScriptOperands` | Consume compiler-selected written selectors once, projecting original words and all frozen argument receipts together. Retain original source sites and physical read addresses; captured late-handler prefixes stay separate and have no written-word receipt. |
| Actual native empty-procedure compiler | `native_procedure_noop_plan`, `NativeCompilerSelectionSite` | Validate the selected header and callable incarnation once before argv; evaluate all original operands and return the selected empty result. |
| Whole-expression native preparation | `ExpressionMathBindings::preparation_for_context`, `PreparedExpressionWitness` | Validate exact bytes, parser axes and actual fixed-table owner; execute the returned native tree before consulting reached calls. |
| Implicit expression function identity for analysis | `ExpressionMathBindings::{resolved_call_for_value_analysis,proved_invocation,proves_intrinsic}` | Identify the reached native implementation or unchanged direct intrinsic at its original expression site. These queries grant neither operand effect closure nor call erasure. |
| Implicit expression function erasure | `ExpressionMathBindings::{resolved_call,proves_intrinsic_for_erasure}`, `native_fold_dependency` | Require independently retained operand effect closure and actual implementation identity; preserve consumed command/table prerequisites. Unknown object effects, alias prefix objects and conflicting observations decline. |
| Compiler choice at admission | `native_compilation_admission_selection`, `admitted_inline_invocation`, `admitted_named_invocation` | Preserve the original compiler recipe and its guards; cannot establish a reached handler or normal effects. |
| Guarded native opcode emission | `AdmittedNativeCompilerInvocation`, `proved_native_admitted_inline_operation`, `native_operation_selection_plan` | Emit an admitted operation with all original lookup prerequisites and exact replay source. |
| Native operation or generic implementation actually selected | `SourceInvocationBinding::proved_execution_target` | Feed strict lowering, body analysis, result specialization, and executable rewrites. |
| Handler unchanged across possible compiler protocols | `SourceInvocationBinding::proved_handler_target` | Establish identity convergence only; query the separate authored successful-handler contract for normal transfer. |
| Original manufacturer of a live class | `SourceInvocationBinding::proved_class_definition_factory` | Read the retained factory and exact class incarnation for diagnostics; does not invoke a constructor, prove its normal result, or admit an opcode. |
| Possible constructor parameter flow from an argument substitution | `SourceInvocationBinding::constructor_entry`; `object_types::scan_substitution_constructor_edges` | Use the original entered child's retained constructor declaration, formals/body source and receiver frame. Child evaluation precedes its parent handler, so a possible parameter edge cannot depend on a normal outer assignment. Explicitly unentered children and missing original entry receipts decline; this grants no normal constructor result, callable installation or effect closure. |
| Current instance allocation and class | `SourceVariableAccess::proved_object_instance`, `SourceInvocationBinding::retains_object_instance_at_dispatch` | Combine the exact physical contents receipt with the post-argv dispatch generation; unchanged SSA or a class label cannot license it. |
| Instance allocation passed into an entered Value formal | `object_instance::retain_value_formals` | Match the frozen original argument receipt to the actual formal binding plan, then retain it only in that activation's unobserved Incoming scalar cell. Defaults, rest/reference bindings, alias prefixes and unknown expansion do not borrow it. Receiver calls align every prepared receipt vector when removing their method selector; normal reads still revalidate the physical cell, original allocation and current class dependencies. |
| Original receiver method declaration | `SourceInvocationBinding::class_definition_method_entries`, `object_receiver_method_entry`, `named_object_receiver_method_entry`, `receiver_self_method_entry` | Match the exact class incarnation, receiver table, original name/body source and receiver frame. Variable objects require the original head read through argv; named objects require their actual installed command allocation at dispatch. Declaration inventory grants no visibility or execution proof. |
| Original definition-phase method name | `SourceDefinitionMethodReference`, `SourceCommandBindings::definition_method_reference_inventories` | Retain the accepted original worker, created class allocation, written operand and declaration available at that phase. A known name without an entry cannot borrow a later method. Missing or conflicting coverage remains a whole-rename hazard. |
| Receiver-local builtin hazard inventory | `SourceInvocationBinding::receiver_self_builtin_candidates` | Query the authored native operation in the exact original method frame. Generic deferred activation retains an unknown receiving-allocation residual. A declared override excludes the builtin; this grants no physical alias, normal transfer, compiler or navigation authority. |
| Local receiver navigation and edits | `receiver_identity::{original_class,method_at_command,method_at_cursor,class_at_read}` in `tcl-lsp-core` | Match the temporal receipt to the actual document's original declaration using the retained realm and lexer. Definition, hover, references, rename and type-definition share this adapter. A nominal completion candidate or document-final class name cannot supply identity. |
| Instance method-name absence at a dispatch | `SourceObjectInstanceProof::instance_method_names` | Consult the immutable creation-time inventory only while the instance receipt remains current; it includes private names, so presence cannot prove visibility, execution or arity. Unknown handlers and unsupported inheritance keep the inventory unknown. |
| Effects on the normal continuation | `NormalTransferInvocation` | Track the supported variable reads/stores and value operands; cannot license a body phase, an opcode, or compiler-error ordering. |
| Original command object and compiler literal actions | `InvocationDialect::native_command_name_protocol`, `native_compiled_command_literal`, `native_compiled_command_name_literal`, `NativeLiteralAction`, `Vm::native_namespace_command_name` | Resolve the original object through its live token and independent native command/reference epochs. Compile-time priming follows ordered registration, even when data first occupied the deduplicated slot. C8.5 hiding copies the current object and withdraws its registration. Jim and explicit logical simulation do not acquire C cmdName caches. Names, namespace paths and serialized descriptors cannot establish a live cache hit. |
| Native variable inventories | `NativeHashAbi`, `supported_backend_hash_abi`, `InvocationDialect::native_variable_table_protocol`, `NativeEntryLedger`, `Frames::var_names_bytes_checked`, `Namespaces::vars_in_bytes_checked` | Supply ABI facts independently of the selected release and retain actual entry birth/retirement, including undefined trace shells. C compiled declaration order and multiplicity precede dynamically created hash entries. Variable glob filtering preserves this order. Jim ordinary variable tables use their retained seed and lazy capacity; Jim arrays use the dictionary backing. Missing native selection refuses; content bytes, defined-name snapshots, host map sorting and compiler epochs cannot establish table order. |
| Selected primitive getter | `InvocationDialect::native_scalar_getter_protocol`, `NativeScalarGetterProtocol::{materialize,cached_conversion,fresh_conversion,fresh_conversion_with_range_error}` | Select the actual engine independently of numeral overlays, getter kind and original RawString/ByteArray storage. Missing or conflicting authority/native dependencies abstain through the host channel. This supplies no expression, completion, effects or opcode proof. |
| Authored logical numeric simulation | `InvocationDialect::authored_logical_numeric_simulation`, `AuthoredLogicalNumericSimulation::{parse_number,parse_boolean,current_number_boolean}` | Require an explicitly installed host capability and consistent F5 logical axes. Full-byte Number/Integer and BooleanValue/NumericTruth stages retain logical origin; they do not supply native primitive getter, cache, error-state, effects or compiler evidence. |
| Original getter cache and outcome | `NativeScalarGetterConversion::into_parts` | Retain required original-string preparation, full cache value and returned scalar/error independently. Materialize before cache mutation, commit cache even on failure, then publish the outcome. Wrapped Wide return is not the cached magnitude; WordBoolean is not an integer cache. |
| Full native numeric probes | `NativeNumberGetterKind`, `NativeNumberGetterConversion`, `native_number_probe` | Keep direct Number, copying Bignum and increment's inline Number purposes distinct. Number admits floating caches; fresh Bignum admits integers only. Increment checks actual resident-empty storage after Int/Double fast paths. Retain full Big magnitudes and apply cache changes before interpreting the command's arithmetic constraints. C8.4 and Jim do not supply these C primitives. |
| Physical integer member update | `tcl_cmd_core::native_increment::{increment,missing_dictionary_member}` | Observe original receiver sharing before its working handle. Probe current and amount in native order, then reject floating categories through the actual Int presenter. Missing generic dictionary members validate Bignum and retain the original amount; compiled immediates manufacture their own integer. Preserve reading-increment errorInfo and typed backend refusals. |
| Authenticated resident-string mutation | `ResidentStringMutation`, `Value::adopt_native_object_representation_with_string_mutation` | Preserve retains the original allocation while applying the reached cache; Replace adopts the donor allocation even when bytes are equal; Discard requires absent donor storage. Validate the receipt and cache origin before changing the original shared object. Content equality supplies no allocation authority. |
| Primitive getter error state | `NativeScalarGetterProtocol::failure_presentation`, `NativeScalarGetterErrorCode::{Unchanged,Set}` | Apply the update to actual prior interpreter state. Omitted error metadata or setting `NONE` cannot stand in for Unchanged. Primitive message bytes and separately measured Eval propagation stay distinct from expression-stage errors. |
| Native C character units | `NativeTclUtf::{decode_unit,encode_unit,previous_character_boundary}` | Consume the selected canonical C build's original bytes, including modified NUL, surrogate units and invalid-byte handling. Checked Rust Unicode projection remains a separate host operation; this does not attest object storage or close custom callbacks. |
| Reached numeric operand cache | `PreparedExpressionWitness::integer_relational_operand_conversion`, `InvocationFacts::integer_index_operand_conversion`, `NativeOperandNumericCacheProduction::with_original_cache_class` | Require the selected runtime stage, exact original successful read, closed conversion effects and actual normal numeric branch. Refine from independently proved current cache class; unknown class stays Numeric, Double stays Double, and grouped children/immediate bytecode indices do not publish on their parent. No canonical string proof follows. |
| Normal result/operand representation | `normal_representation_invocation`, `NormalRepresentationInvocation` | Project accepted frozen operand layout, type and representation metadata for analysis; cannot establish purity, completion or opcode selection. |
| Selected argument type assistance | `InvocationFacts::argument_type_hint`, `NormalRepresentationInvocation::argument_type_hint` | Read the frozen selected member, option grammar and positional offset without another registry lookup; unsupported layout, unknown arity and rejected arguments decline. |
| Bounded ordinary range result bytes | `NativeResultSelection::ordinary_range_literal_result` | Use the selected actual index/list grammar, current ordinary input and exact length; unsupported serialization keeps bytes unknown independently of range shape. No object freshness, world preservation or erasure authority follows. |
| Selected constant range bytes | `NativeResultSelection::constant_range_literal_result`, `CommandSpec::run_const_fold_in`, `SubCommand::run_const_fold_in` | Parse and serialize under retained actual invocation axes. Without those axes, portable folds require equal bytes across native policies. Successful rendering supplies only a value, independently of input conversion and executable erasure. |
| Exact procedure normal-result type | `SourceCommandBindings::procedure_implementation_bodies`, `ObjectHandleFacts::normal_procedure_result` | Match full implementation allocation, original body and formals to a separately proved current handler. Retired implementations remain distinct; absent or recursive unresolved results stay overdefined. No physical object, completion, effects or rewrite authority follows. |
| Possible regex operand hazards | `NormalRepresentationInvocation::possible_pattern_source_argument_indices` | Project possible written Pattern positions under the actual retained handler and option grammar. This supplies May hazards, never a guaranteed role, writable literal, completion or effect proof. Unknown expansion and unsupported layouts decline. |
| Nominal naming-factory candidates | `NamedObjectFactory`, `NormalRepresentationInvocation::naming_factory_candidate` | The selected registry descriptor and retained normal handler supply the frozen name and a class candidate. Advisory callback reachability may consume it; physical object identity, method execution and editable references cannot. Unprovided or replaced factories supply no candidate. |
| Normal handler diagnostic completion | `normal_handler_completion_route` | Follow the converged handler in a diagnostic body-flow projection; retain independent compiler uncertainty in execution, CFG pruning and opcode queries. |
| Normal result security | `normal_taint_invocation`, `NormalTaintInvocation::{source_colour,is_sanitiser,transform_colour}` | Read selected successful-handler properties with provider/private-worker prerequisites. Conditional transforms use frozen evaluated operands and actual native list rules; unknown mappings never acquire colours. Compiler uncertainty remains independent. |
| Editor assistance | `resolved_declared_assistance`, `registry_invocation_assistance` | Present declarations or possible candidates with their uncertainty. |
| Physical variable access | `ResolveContext`, `var_resolve`, `PointResolveContexts` | Resolve ownership, lifetime, aliases, selected array elements, presence, and observers. |
| Contents at an exact substitution | `SsaSourceView::read_reference`, `read_word`, `read_expression_variable_contents` | Preserve the original reference or word, represented contents version and explicit unknown residual. |
| Non-reading point binding | `SsaSourceView::reaching_binding` | Query the actual cell and represented reaching store at a boundary; cannot invent a missing lexical read. |
| Read-local conversion-cost advice | `CommitWalker::{cost_for_word,cost_for_expression,cost_for_native_read}` | Reconcile semantic contents and replayed commitment with the captured current physical representation once. Unknown representations do not restore stale container or numeric commitments; closed alternatives grant May cost advice only. |
| Exact read completion/presence | `SsaSourceView::read_completion_at`, `read_contents_presence_at`, `read_contents_presence_alternatives_at` | Assess closed physical alternatives without fabricating a value version. Potential missing-read diagnostics consume the separate alternatives witness; definite error queries remain unchanged. |
| Ordinary incoming logical slot | `SsaSourceView::read_expression_incoming_slot` | Recover contents across distinct validated caller activations; retain every physical cell and decline aliases, statics, observers and unknown writes. |
| Conditional analysis value | `FunctionUnit::semantic_values`, `SemanticValueFacts::expression`, `DiagnosticValueFacts` | Consume immutable contents/numeric values and producer obligations for diagnostics; cannot donate those values to execution erasers. |
| Native compiler traversal and context | `NativeCompilationSpec` | Select original syntax and actual engine/frame context before consulting runtime flow. |
| Future callback callers | `SourceCommandBindings::future_body_inventories`, `SourceFutureBodyInventory::invocations` | Retract caller seeds using separately retained future source/frame and lookup alternatives; cannot establish reached document execution. |
| Runtime admission | `NativeCompilationEntry`, `FunctionAsm::validate_native_compilation_entry` | Validate the actual entry and compiler obligations before source effects or formal binding. |

Declaration layouts retain separate `OriginalDeclaration` and
`EnteredActivation` issuers. Declaration formal, read, flow and lifecycle advice
uses `original_declaration_layouts`, requiring unanimous original allocation,
source, word vector, grammar and typed frame/context. Entered activations remain
available to actual invocation, read, native and effect queries; they neither
supply nor contradict declaration semantics. Missing, foreign or conflicting
original declaration receipts withdraw the advice. The associated internal
`SourceCommandBindings::scoped_lifecycle_advice(tokens, registry)` query retains
this purpose. Its body source distinguishes known absence from retained source;
unavailable evidence remains separate. No declaration preview proves actual
entry or successful effects.

The normal-transfer adapter is purpose typed. It does not expose a general `InvocationFacts` accessor or implement `Deref` to the strict invocation type. Consumers needing runtime body execution must call the strict adapter. Guarded native compilation uses the separate admitted compiler adapter; it never imports successful-handler or normal-effect facts. Extend the authored successful-handler contract only after demonstrating that every admitted compiler protocol has equivalent normal effects for the actual argv and physical frame.

A namespace owner can differ from the namespace obtained by parsing its rendered
name. With `namespace eval : {proc p {} {...}}`, pinned C Tcl 8.4/8.5 and Jim 0.84
publish global `::p`; C Tcl 8.6 and later publish `p` in the literal-colon namespace
owner. That owner's constructed key is `:::::p`, but evaluating that spelling as
an absolute command name resolves `::p`. The source kernel therefore registers
deferred declarations through the exact newly published slot, not a second
lookup of its display name. Unknown engines decline this ambiguous publication
axis. Ordinary namespace names, where both protocols agree, keep their common
key. Generic namespace invocations may retain their exact entered declaration
body inventory while the invocation itself remains generic.

Keep written operands, constructed keys and native identities separate.
`naming::qualify_namespace` parses a written namespace operand under an authored
constructed context; empty relative names and trailing separators select the
namespace itself. `qualifier_segments` parses written separator runs.
`key_holder_and_tail` and `key_segments` choose an analytical split of a
constructed text key. They do not recover a native path from its display.
Native component paths `[a:, b]` and `[a, :b]` both display as `::a:::b`, yet
identify different namespace tables. A display string cannot be an inverse for
both paths, even when it is valid Unicode and contains no NUL.

Retain `ByteNamespacePath` components before rendering them. An actual
namespace additionally requires its interpreter and incarnation token; equal
component paths do not identify a deleted-and-recreated namespace. Use
`NativeCompilationEntry::retained_namespace_context` to keep that complete
context. Its display is presentation only. A globally callable source spelling
requires the independent native round-trip spelling receipt, which can decline
for a retained context that still supports exact relative lookup.

`ModuleCommandBindings::install_runtime_entry` installs the genuine native rows
before `OriginalSourceCommandWorld::for_runtime_entry` selects the matching
command policy and exact retained namespace geometry. Branches share this
initial baseline. Missing native rows remain missing; an incomplete, foreign
or withdrawn world cannot be reseeded from Registry metadata. This
[bootstrap contract](../analysis/name-resolution-proofs/command-original-native-entry-world-bootstrap.md)
supplies neither source publications nor successful root completion.

A written relative command is parsed only at lookup ingress.
`NativeCompilationEntry::command_lookup_cursor` uses the actual retained
namespace token and the original command bytes; advance `next_candidate` only
when the preceding candidate is absent. Each reached candidate retains the
namespace token and exact slot. Do not eagerly demand provenance for an
unreached fallback or recover a native table from a candidate's display.
Authored text contexts use
`command_resolution_candidates_from_namespace_keys`; that analytical helper
cannot replace the token-bearing native lookup cursor.

The source identity owner separates `SourceNamespaceKey::Authored`, `Native`
and `Allocated`. `Native` retains `NativeNamespaceContext`; `Allocated` retains
the original allocation site, execution incarnation and component path, which
supply no actual native token. `SourceCommandKey::Slot` combines that namespace
domain with the selected counted simple `NameBytes`. `Authored` remains a
symbolic text domain. Use `exact_native_path` only when component geometry is
retained, `native_context` only for an actual context, and `display` only for
optional presentation. A text query cannot impersonate a slot merely by
matching its display. Test these distinctions
with `retained_namespace_candidates_preserve_unaddressable_contexts` and
`native_colon_context_lookup_does_not_publish_a_global_source_alias`.

Carry executable lookup context with `CompiledNamespaceContext`, rather than
serializing that display. `Native` preserves the actual interpreter/token/path
receipt; `ConstructedPath` preserves exact component geometry without claiming
an actual incarnation. The public `SourceNamespaceKey::to_compiled_context`
returns the native receipt for `Native`, constructed geometry for `Allocated`,
and no executable context for `Authored`. Projecting an allocation's geometry
does not publish its source allocation site or grant runtime allocation effects.
Keep the source key when those independent analysis identities matter.

Set `CommandBindingIdentity::namespace_context` and
`ProcedureBindingIdentity::namespace_context` when the producer has that
context. A supplied context is authoritative over the text fields. Preserve it
through emission in `Instruction::source_command_namespace_context` and
`NativeOperationSelectionSite::namespace_context`; use
`source_namespace_context()` and `replay_namespace_context()` to obtain the
retained context. Their absent-field fallback retains the existing exact
component path, rather than parsing a reporting string.

VM `resolve_compiled_namespace_context` checks a native context's interpreter,
token, component path and actual live or retained namespace owner. Deletion and
same-path recreation cannot substitute another token; a foreign interpreter
cannot donate one. Constructed geometry is resolved by the selected namespace
component owner and supplies no token authority of its own. Keep this validation
at binding checks and before stale-command replay. Replay changes the validated
resolution context without adding a call frame; a different namespace at global
level is unavailable because the global frame must keep `uplevel #0` semantics.
Do not recover a namespace from a display or weaken a failed native check into a
constructed-path lookup.

Preserve `replay_context_rejects_inconsistent_component_geometry`,
`compiled_native_namespace_context_separates_equal_colon_displays`,
`compiled_native_namespace_context_rejects_deleted_and_recreated_token`,
`compiled_native_namespace_context_rejects_foreign_interpreter` and
`replay_namespace_context_preserves_colon_identity_and_retired_owner` when
changing this transport. These tests separate geometry, actual identity and
retained-owner lifetime; a same-spelled namespace test alone cannot do so.

Installed and implementation body transport retains authoritative
`namespace_key` alongside the original source and declaration allocation.
`SourceProcedureImplementationBody` borrows that key;
`SourceInstalledProcedureBody` owns it. Their `namespace` and `command` strings
are presentation only. Use `SourceCommandBindings::selected_source_in_context`
to select an original body in that exact key; it filters retained point, phase,
compiler and read observations. The internal
`executed_script_entry_namespace_context_at` accepts only unanimous original
source/frame observations and preserves that frame's actual key. Missing or
conflicting observations cannot license body entry, a compiler hook or a
callable installation.

`Script::namespace_context` carries the retained body key into
`cfg::Function::namespace_context`. The central `function_source_entry` combines
the retained point entry and permitted entry overlays, then applies that exact
namespace identity. Consumers use this entry for cells and selected source
facts; a function's printed name or namespace is not an alternative owner.
Original-name projections remain diagnostic text.

Jim lookup also requires the actual namespace-object carrier. Runtime frames
and declarations retain their original Jim namespace objects; compilation
snapshots retain the selected `NativeCompilationNamespace::jim_namespace_object`
spelling independently of C component geometry. Source lookup consumes that
selected Jim context and actual namespace paths. A constructed C path, a checked
Unicode display or equal bytes from another object do not supply a Jim frame,
command cache, native object role or namespace token.

Pure source naming selects `InvocationDialect::authored_name_policy`. The
returned `NamePolicyProtocol` carries `AuthoredSimulation` authority even when
its recipe matches a known C release or pinned Jim. Unversioned Tcl uses the
explicit C8.6 analysis abstraction; a vendor compatibility version does not
issue a native naming recipe. Actual runtime consumers still require their
separate native issuer and original lookup context.

Procedure completion follows the independently selected declaration policy.
The resolved Xilinx EDA profile retains its declared Tcl 8.5 layer, so its
original declaration metadata uses the Tcl 8.5 recipe with `AuthoredSimulation`
authority. This is source simulation, not evidence of a vendor native engine.
F5 iRules without a selected command-name recipe uses bounded lexical ASCII
declaration advice under its full retained grammar. A failed original byte
producer never enables that lexical fallback.

Path navigation carries that issuer through `PathConstantAssignments`,
including inventories with no rows. Keep the original namespace body intervals,
source offsets and selected local/global homes when combining batches, taking
prefixes or refreshing a document snapshot. `FoldedPathConstants::at(offset)`
selects only the accepted original lexical scope at that site. Jim
namespace-eval locals remain in their activation; only known global-home values
export to other documents. Import agreement requires equal naming issuers and
equal values across every route. Dynamic or unsupported scope and alias paths
withdraw advice; unsupported body scopes cannot borrow enclosing globals.

`OriginalSourcePathExpression` retains one genuine whole source word, full
input/configuration, exact source image and Realm generation. Selected
`SourcePathOperation` metadata owns Join, Dirname, Normalize or ScriptPath and
its effective argument ordinals, including bound values. A command with a
`file` spelling but a selected list implementation supplies no path operation.
Completed values remain data; they are never reparsed as Tcl source.

Use `capture_source_target_path_expression(analysis, target)` for source edges
and `capture_source_path_expression_at_analysis(analysis, span)` for an
independently required original whole-word span. Missing actual input, foreign
source/configuration, uncertain operations or unavailable whole operands
withdraw the plan. `matches_analysis` checks the same actual generation;
`matches_assignments` joins a plan to its retained path inventory before a host
folds an indexed edge. `evaluate(document_filename, resolve_var)` takes explicit
host filename advice and the position-gated variable resolver. It supplies a
candidate path, without observed Tcl values, current cwd or `info script`.

`WorkspaceSource::original_path_expression` carries that plan into
`SourceResolver`, alongside the original site offset and typed assignment and
imported-constant inventories. A reporting path or literal flag cannot replace
an absent plan. `WorkspaceIndex::resolved_source_edges` supplies the same edge
inventory to source seeds, imports, run order and source-ancestor package
queries. Preserve the plan and inventory generation through every refresh.
Explicit literal-only graph and standalone evaluator APIs keep their own
caller contracts; they cannot reconstruct unavailable actual analysis.

`capture_source_auto_path_expression(analysis, entry)` independently checks the
selected assignment or list-append schema, genuine receiver and value operand.
`OriginalSourceAutoPathExpression::evaluate` completes the whole argument
before interpreting its value: assignment splits a list under the same selected
grammar, while list append retains one element. Neither list element is parsed
again as a substitution. Unproved local/global aliases and known shadows
contribute no package directories. This is package path advice, not an observed
global mutation or package installation. See the
[retained path contract](../analysis/name-resolution-proofs/retained-path-source-inventory.md).

Background `signature_scan::extract_signatures` consumes this authored policy
from the retained document registry. Procedure declarations use the shared
command-publication slot, nested namespace bodies use the selected namespace
input, and rename destinations and aliases use their distinct publication
purposes. Jim retains its flat command/namespace keys and selected procedure
body namespace. C8.4/8.5 reject colon-prefixed procedure tails outside root
before the scanner records that declaration. These records support symbols,
cross-file arity and source navigation; they do not establish an installed
command, compiler hook, original cache, reached frame or body execution.
Retain `SignatureProc::source_name` and `body_namespace` independently of the
reported command label. `SignatureNamespaceScope` preserves C components or
Jim counted flat bytes; `display()` is presentation and `source_spelling()` is
a checked optional written input. `SignatureSourceCommand::slot()` and
`policy()` identify an authored declaration, while `matches_written()` checks
its original local lookup. `procedure_declarations` and `class_declarations`
preserve every original declaration; the presentation maps abstain on label
collisions. Use `procedures_for_written_name` with the actual authored scope and
policy for source assistance. These records supply no runtime token or
compiler admission. `SignatureCommandAliasTarget::WrittenCaller` retains a
caller-relative operand; every context-independent projection declines it.
For a `WrittenGlobal` operand, `selected_global_name` retains the exact authored
slot, while `reported_global_key` supplies metadata only. `checked_global_key`
alone supplies a checked global String lookup. Preserve
`Indirection::target_source_name` and the workspace link's original target/source
receipts; an unaddressable selected slot does not enter a global String map.
Global alias chains,
workspace indexes and indirection advice must use that checked query rather
than interpreting a reported target as a global address.

Validate naming changes with
`signature_publication_keys_match_six_native_namespace_controls` and
`signature_names_keep_jim_flat_keys_and_old_c_creation_rejections`, retaining
all six selected engines and the negative creation cases.

Jim global variable keys use `naming::jim_global_variable_key_bytes`. Its namespace input is a rooted constructed key: remove exactly its root marker and preserve the retained segments. An absolute written name beginning with `::` instead discards all leading colons, so `:::x` addresses `x` while `:x` stays distinct. Interior runs such as `foo:::bar`, raw non-Unicode bytes and embedded NUL remain exact. The Unicode wrapper delegates to this byte owner. Physical command tables retain `ByteNamespacePath` and `NameBytes`; variable bindings retain `NameBytes`. Raw bytes therefore remain exact in storage. Their use still requires the selected original getter, name purpose and actual namespace/frame incarnation. A pure key projection supplies neither lookup side effects nor script-parser authority.

For class diagnostics, a recorded syntactic `ClassDef` is insufficient to prove
that a later command still denotes that class. Query the live definition factory
and incarnation, then apply the factory's registry grammar under the retained
dialect and availability phase. Read alias prefixes and frozen arguments from
that same invocation. Traverse nested invocations through `word_subst`, including
generic assignments; checking only `AssignValue` loses their original command
proofs. The abstract-class tests pair native Tcl 9.0/9.1 definitions with older
release absence, alias-prefixed construction and a replacement procedure.

Original factory provenance also does not establish the current method table.
An abstract class can export `new`, replace it, or install an `unknown` method;
all three can make the invocation succeed. A constructor can normally complete
after changing its new object's class through `oo::objdefine`. Retain object
dispatch generations and actual constructor effects before assigning a concrete
result class. The shared execution vectors pair these mutations with a rename
that preserves manufacturer absence, and record actual feature errors on older
C Tcl and Jim.

Strict construction consumers select
`CommandRegistry::native_default_construction_grammar` with the retained
actual dialect and realm. The profile-based wrapper is for authored catalogue
inventory. Neither query supplies a live factory or allocation proof.
`PreservingCommandPublication` is a separate normal-transfer receipt shared by
fresh native class and procedure definitions. Common validation pins the exact
original factory, genuinely absent destination, known namespace and addressable
published key, bounded next allocation, unobserved execution and current stock
literal arguments. Class publication additionally requires original metaclass
dependencies, fresh private namespace and an accepted closed definition body.
Procedure publication requires the selected original C native definition
parser, valid formals, no persistent-static operands and exactly one matching
procedure definition transition; its body is retained rather than executed.
The shared procedure-name publication policy supplies the actual key, rather
than a consumer reconstructing it with ordinary command lookup rules.

A literal root `:source` publication stays distinct from `source` across the
five measured C releases. A nonroot `:ns` holder publishes into `ns` under
C8.4/C8.5 and retains `:ns` under C8.6/C9.0/C9.1. Lambda namespace construction
is an independent operation: a C lambda explicitly naming `:ns` selects `ns`,
while its omitted namespace selects root. Tcl8.4 Apply and the tested Jim child
surface remain unavailable. The [finite source observations](../analysis/name-resolution-proofs/procedure-original-root-and-colon-holder-publication.md)
and exact release source windows supply these bounded distinctions; they do
not grant a physical namespace or procedure allocation.

Only the matching definition kind, name and source instruction may preserve
unrelated existing method/delegate tables. For example, a genuinely fresh
`proc pass {value} {return $value}` does not retire the allocation already
stored in `original` before `pass $original`. Replacing a command, reusing an
unknown allocation family, executing arbitrary definition code, converting
an unknown object, observing publication or modifying the metaclass withdraws
the receipt. Unknown/private namespace ownership remains declined. Ordinary
command mutation and object reconfiguration retain their existing invalidation;
a `Create(Class)` or `Define(Procedure)` fact alone cannot suppress it.

A variable's unchanged SSA version proves its contents provenance, not its
referenced object's class or method table. `oo::objdefine $object class Other`
can change dispatch while leaving that variable untouched. A strong object
class query therefore also needs the source-owned object allocation and current
dispatch generation. Keep advisory `ObjectTypeCandidates` open when adding
possible callback edges; a positioned physical read alone cannot close them.

An inherited navigation entry retains its original `declaring_class` separately
from the object's current receiver class. The closed single native superclass
assignment composes instance entries and the native Tcl 9 `classmethod`
delegate entries; local declarations take precedence. A native classmethod
retains the original class factory allocation, declaration worker and delegate
dispatch epoch. Its instance forward through `myclass` is distinct from the
method body on the generated delegate, and neither declaration receipt licenses
execution through that forward.
The plain delegated body interpreter separately validates the actual receiver
class, declaring allocation, unchanged native factory and declaration worker,
delegate epoch, superclass dependencies and observer absence. It enters the
original formals and body without manufacturing an instance allocation. A body
that needs `self` or `my` still requires an independently retained actual
receiver protocol; declaration navigation cannot supply it. This body entry
preserves later method observations only through its real normal continuation.
The source receipt retains every inherited base allocation and rechecks those
dependencies before fresh manufacture and navigation. Base replacement,
dispatch mutation, unknown constructors, multiple bases and custom providers
withdraw that closure. Ordinary `self method` declarations belong to the class
object's own table and do not acquire the delegate inheritance policy. This
inventory grants no method execution, visibility or result proof.

For example, a retained stable `set x value` can establish the normal write to `x` even when an unselected engine's compiler protocol is unknown. That does not prove when the native compiler validates `set`, whether earlier commands execute on a syntax error, or that codegen may emit a store opcode. An unknown `return`, `try`, or expression protocol cannot acquire those broader guarantees through handler identity.

## Resolve dynamic command names at their actual invocation

Prefer the executed lookup carried by the source owner before considering
advisory values or diagnostic usage heuristics. For another candidate word,
use `lookup_command_word` on that invocation's immutable snapshot. The result
retains imports, aliases, namespace paths, dialect availability, provider
transitions and command allocation lifetime. An unrelated namespace containing
the same tail cannot establish that an unqualified candidate resolves.

```rust,ignore
let invocation = realm.invocation_at_source("", command_offset);
let candidate = invocation.lookup_command_word(candidate_contents);
let proved_target = candidate.proved_target();
```

A missing snapshot or non-unique result remains unknown. In particular,
`proved_target().is_none()` does not mean the command is absent. A candidate
absence consumer queries `candidate.selected_slot_presence()` in the same
immutable lookup world. Presence concerns the called slot, even if an alias's
terminal command is missing. Physical absence is independent of what a
configured `unknown` handler subsequently does. Diagnostic consumers use
`selected_slot_diagnostic_presence()` to retain uncertainty about a custom
fallback and an already selected native compiler recipe. Its `Absent` result
is unresolved-command advice, not a guarantee that initial autoload will fail.

Configured namespace unknown-prefix uncertainty applies after execution lookup
exhausts the actual command table. Known selected slots and native compiler
lookups retain their separate table and registration certainty; a configured
fallback does not make an already known compiler lookup uncertain. When lookup
reaches that fallback, unknown prefix or value bytes cannot establish that no
handler effects or child body execution occur. Preserve the selected lookup
timing and unresolved coverage.

For an interpolated head, retain its original source lookup instead of folding
a union of constant strings collected from every procedure. A same-named
variable in another activation is not the read that produced this command
word. W123 source diagnostics consume the actual positioned lookup;
the W307 adapter queries selected-slot presence before its advisory object-usage policy and resolves
both presence and absence candidates through the same snapshot. A custom
fallback cannot acquire a guaranteed-failure diagnosis from a missing slot.
Neither adapter grants an opcode or pure-handler contract. Production
interpolation queries consume the positioned lookup and original frame receipt.

The four shared `lookup_*` execution vectors and corresponding diagnostic
regressions cover an unrelated namespace tail, an actual selected namespace,
`dict` availability before and after Tcl8.5, and interpolation isolated from
another function's variable. Their native observations run on all five C
releases and Jim; the diagnostic tests exercise each supported C dialect.

Executable switch lowering retains the unchanged original invocation when its
independent compiler receipt is available. The native recipe owns original arm
body visits and duplicate masking; analysis CFG branch facts do not grant
compiler entry. Unit ChunkEntry guards validate the exact immutable entry world,
while each original Named invocation retains its separate BeforeArguments
compiler-selection guard. Later child-world dependencies remain instruction-scoped.
An authenticated Generic selection without preparatory work uses generic
dispatch; Inline and preparatory selections still need the supported original
emitter and cannot borrow that Generic purpose.
Select the actual descriptor before applying a generic-emitter whitelist. A
catalogue entry, command spelling or unsupported specialized recipe cannot
stand in for that selection. Unit cache prerequisites clone the exact admitted
entry selection; instruction guards retain their own original validation timing.

## Navigate through the called slot

Runtime alias-loop diagnostics consume actual resolved simple binding keys.
`NativeNameProtocol::rename_alias_loop_name` selects the source key for C8.4 and
the attempted destination key for C8.5+, with the native CString report extent.
Jim has no C alias-loop presentation purpose. Keep this selection independent of
written qualifiers, display labels and logical compatibility; missing actual
binding or purpose authority remains a typed refusal. The strict alias and legacy
trace integrations construct the matching physical core before registrations and
use the shared validated oracle runner. Jim child-handle aliases have their own
native controls; absent C-style alias and variable-trace surfaces are explicit
errors, not skipped C vectors.

Use `command_reference(value)` on the actual consuming invocation. An
interpreter alias has its own named slot even when its terminal target is
missing. A namespace import instead retains the origin token; renaming the
origin preserves that token, and deleting it retires the import family. The
navigation carrier distinguishes these cases explicitly and does not invent an
import allocation or borrow the alias's terminal command name.

`definition()` retains the current implementation allocation, including its full
source instance and incarnation. A command token can survive procedure
redefinition, so its original `declaration` is not the current definition.
Class/instance command kinds also share a category: only the class-definition
receipt grants a class declaration. Imports retain their origin implementation;
alias wrappers retain their called slot without donating a direct definition.

`linked_definition()` is the separate terminal navigation receipt from the
original closed alias/import traversal. Definition providers prefer this receipt
and then `definition()`, while direct rename/reference views retain the called
slot and `is_direct_definition()`. An alias to a procedure can navigate to its
retained source declaration without becoming that procedure's direct slot.
An alias to a builtin has neither declaration receipt and cannot borrow a
displaced class with the same name. Missing, observed or opaque traversal edges
withhold linked navigation; final name maps cannot close them.

`SignatureCommandLookup` separates an invocation head from a name consumed by
another command and a deferred reference. A consumed name keeps its actual
consuming offset independently of the editable word span. LSP navigation uses
`invocation_reference_at` for that receipt; execution-head predicates use
`invocation_head_at` and require `lookup.is_execution_site()`. Thus an `info args`
or `rename` argument can navigate to its captured procedure without acquiring
call semantics or being counted as an executed head. Consumed-name hover uses
the same retained declaration receipt and refuses name assistance when it is absent.

The definition carrier is a privately minted `SourceCommandDefinition`.
`kind()` distinguishes Procedure from Class; `allocation()` supplies the complete
implementation allocation. An instance cannot borrow a class definition, and a
procedure cannot borrow a same-named class assistance record. A validated provider
class dispatcher can support class declaration navigation without acquiring a
TclOO class, object-class or construction licence.

Keep this receipt in `SignatureCommandInvocation::resolved_definition` and
`WorkspaceInvocation::resolved_definition`; do not flatten it into a qualified
name or a boolean. Retain the complete `resolved_command_reference` too: a
selected alias, instance or builtin with no direct definition differs from
missing navigation evidence. A known non-definition cannot borrow a displaced
class or procedure from a same-named assistance record.
`AnalysisResult::proc_for_definition` matches its original
Authored source and exact command token spans against retained declarations.
Its token lookup uses `AnalysisResult::body_lexer_config`, the actual grammar
retained at analysis ingress, rather than reconstructing a profile from the
display dialect string. Without that context the query declines.
A loaded or materialised source cannot donate editable document coordinates.
`proc_def_in_effect_at` gives retained unanimous allocations priority over its
legacy declaration assistance; conflicting allocations decline rather than
choose the first reference. Reached bodies can execute before a later top-level
redefinition, so textual load-before-body shortcuts cannot replace this fact.
Constant contributor deduplication includes the allocation; equal spans and
slots are insufficient to merge references from different implementation worlds.
Workspace settlement gives these positioned receipts priority over final name,
link and wildcard-import assistance. The direct view retains the editable called
definition slot; the linked view retains the current implementation's original
declaration. An imported spelling does not become a direct rename target.
Publish class assistance through `AnalysisResult::retain_class_declaration`.
It preserves displaced declarations in `superseded_classes`, while member
updates retain the same declaration identity. Fragment grafting must preserve
and relocate that history along with the final records. This history supplies
the original declaration only after a positioned receipt selects it; it does
not establish that the class exists or executes.

Create written invocation records with `SignatureCommandInvocation::written`.
Then use `retain_reference(&reference)` to transfer the slot, declaration category,
current allocation and candidate projection together. Override indirect or
specialised reference flags only from their original source/purpose owner. This
keeps an instance, import or alias from accidentally inheriting a direct-definition
boolean when a new field is added.
Use `clear_positioned_reference` to withdraw the complete reference and its
definition projections together. Remaining written names and candidates are
assistance, and do not replace that withdrawn receipt.

For declaration navigation, select `proc_for_definition` or
`class_for_definition` from the retained `kind()`, passing the actual document
bytes. If an original record is missing, foreign, overwritten or conflicting,
retain that refusal; do not retry a final QName map. These are navigation APIs and
their returned declaration records cannot provide current receiver methods,
argument layouts or normal-completion facts.

`is_direct_definition` distinguishes direct procedures/classes from wrappers;
`is_user_command` filters catalogue builtins for source references. Neither is
an implementation or execution query. Unknown namespace paths, unbounded table
mutations and conflicting slot alternatives cannot be resolved by searching
all recorded declarations for a matching tail. The carrier is privately
constructed from the immutable lookup world so consumers cannot manufacture it
from a catalogue name.

Executed computed heads use `SourceInvocationBinding::evaluated_command_reference()`
for the command word frozen at their actual post-argv lookup. A retained Loaded
implementation can support external call reporting even when SSA contributor
provenance is unavailable. Mark the original computed word indirect; the frozen
name does not become editable source. Exact contributor proof supplements the
same full reference and source execution point, rather than appending a stale
second invocation or replacing temporal identity with a value string.

Constant-head edit references combine this query with the exact reaching value
contributors. Evaluated bytes are command names: `$target`, a name containing
spaces, and the empty name must not be rejected as substitution syntax.
Expanded prefixes use `WordValueRules::split_list` for the actual selected
dialect, while the list owner supplies a byte-exact first-element span when
available. `find_element_with_syntax` retains the selected grammar's original
element extent, including Jim's unterminated literal elements; decoded elements
can still support navigation without inventing a byte-preserving source span. Preserve source spans independently: a resolved runtime value
with no exact authored spelling can support navigation, but cannot support a
rename edit. Array/dictionary dispatch references additionally require the
physical consumed cell and entry's reaching producer; a matching variable base
in another procedure or a retained overwritten entry is insufficient. Resolve
the command at consumption, after argument evaluation, rather than at the
source literal's original write.

The navigation controls pair an alias with an absent terminal, import
rename/delete lifetimes, unrelated namespace tails and an unknown relative
path with an independently qualified lookup. The consumer controls must also
cover source-exact rename edits and differing branch targets.

A reached `self` result carries its actual object allocation independently of
name bytes. `frozen_object_receiver_method_entry` requires the unchanged
original head expression and validates the receiving allocation after argv,
including private dispatcher and class/base generations. It supplies original
method navigation only. The normal native variable-link protocol separately
uses `CellOwner::AllocatedInstance` and preserves sequential partial effects;
generic receiver frames remain advisory candidates without physical aliases.

## Preserve directive metadata without turning it into execution proof

Use `namespace_directive_footprint` for namespace import/export metadata. It
selects the converged handler and its frozen arguments independently of native
opcode eligibility. A moved handler can retain its directives; a replacement
with the same written name cannot borrow them. The footprint exposes only
import/export transitions, with unknown operands preserved.

`NamespaceTransition::export_pattern_operands` owns the exact `-clear` control
word. Native C Tcl 8.4–9.1 treats `-c`, `-foo` and `--` as ordinary export
patterns. Export directive rows describe observed declarations; their collection
must not become an interpreter export table or certify a later import. The
source command-world owner applies clearing and resolves imports against actual
namespace and command incarnations. Lowering records metadata before its opcode
gate and never uses the metadata to bypass that gate.

## Keep compiler admission separate from execution

A callback in an earlier instruction can change later command lookup, but cannot retroactively change the compiler recipe of an already admitted chunk. `native_compilation_admission_selection` queries the immutable compilation table; `native_compilation_selection` queries the stricter executable proof after possible runtime effects. An admitted generic invocation remains generic when live lookup becomes unknown. An admitted inline operation retains its original prerequisites and validates them at its authored boundary. A selected private name remains a late lookup of that exact name.

Use `proved_native_admitted_inline_operation` only in guarded bytecode emission, its source replay plan, compiler operand contexts, and associated dependency collection. Obtain hooks and original operand layouts through `admitted_native_compiler_invocation`; `AdmittedNativeCompilerInvocation` exposes compiler metadata, exact source words and body contexts, while keeping its general invocation facts private. Direct private ensemble registrations retain their actual slot identity and written operand origins even when the registry projects logical member metadata. Keep `proved_native_inline_operation` for semantic specialisation and executable erasure that require a proved execution. Do not replace every strict query with the admission query. General invocation facts from a compiler recipe would incorrectly donate normal effects from a handler that may never execute. Tests must pair callback-induced live uncertainty with a genuinely unknown compiler table, and verify the release-specific native behavior after a callback replaces the original command.

An emitted operand must retain its original `WordExpr`, separately from its compatibility text. `emit_word_from_source` evaluates that word once and preserves braces; native hooks map their logical arguments back through `original_hook_argument`. Increment's `emit_increment_amount` uses the same source-word emitter: `{[set ::seen 4]}` remains literal bytes and fails numeric conversion, while an unbraced `[set ...]` executes once before the native read protocol. The amount's numeric validation phase is a later registry-owned decision, not permission to parse its evaluated bytes as another Tcl word.

Expression preparation with native script compiler visits also needs its full
ordered closure. `SourceExpressionPreparation::has_closed_script_compilation`
checks the private receipt against the original expression source and witness
spans. `script_compilation_dependencies` exposes the actual entry dependencies;
it does not prove that any bracketed command executes. The producer allocates
that closure only if the native expression protocol actually asks to compile a
script, keeping ordinary modern Tcl and Jim preparation on their existing path.

`CodegenCtx::retain_expression_preparations` validates every witness and closure
before recording any guards. It retains compiler-entry command requirements
alongside fixed-function table requirements, including when no math call was
reached. The present registry-binding ABI cannot validate a source procedure
header or a prefixed alias as a registry implementation, so those dependencies
need a corresponding header receipt before this emission path can accept them.
A boolean “script compiled” flag cannot replace that receipt. The C8.4 tests
pair actual ordered script visits with a removed closure and verify that a
refusal leaves previously retained guards intact.

## Retain compiler coverage independently of runtime reachability

`BodyExecutionSpec::ArrayIteration` describes the C Tcl 9 caller-frame iterator separately from its `Basic3Arg` named worker compiler. `array_iteration::select` retains the two destination names, array name and body operand from frozen argv. Argument acceptance does not prove body entry: the physical owner must establish an array subject and a possible next element, then store the key and value sequentially before entering the body. Missing/scalar subjects, an empty array, invalid binding lists and a failing value destination have different entry and partial-store outcomes. Normal handler proofs also require the actual stock `::tcl::array::for` worker; a replaced private worker cannot donate this body contract. The shared native corpus covers those outcomes and private-worker replacement across all six engines. SpecTcl packs still reject native `body_execution` contracts explicitly rather than pretending to supply the live worker proof.

The compiler visits source commands in bodies and branches that execution may never enter. `SourceCommandBindings::compiler_invocations` retains their recipes under the complete source identity. `SourceInvocationBinding` keeps that compiler snapshot separate from its runtime `lookup_state`. A compiler-only attachment can answer admission, source origin and `native_compiler_dialect()`; it cannot manufacture a reached target, a frozen runtime argv, normal transfer or a physical frame. Missing runtime evidence therefore does not erase an actual compilation decision.

Literal children visited by an enclosing native compiler retain a separate `SourceCompiledChild` receipt: the original parent admission and guards, immutable compiler table, exact child source mapping and compiler context. A later possible generic handler branch can request compilation from an opaque runtime world; that request cannot replace the original parent compiler visit at the same source offset. Artifact admission uses the retained original child receipt, while runtime handler, callback and error uncertainty remain. A receipt from another source instance or a different literal body does not apply. The child also shares the original ordered compiler-visit inventory. Source queries restore those compiler admission and layout fields only while every retained enclosing-parent visit still has the same admission and namespace. A new missing, rebound or generic parent visit withdraws the lease; runtime target and variable evidence never come from this compiler inventory.

On the F5 rule-load surface, `deferred_rule_declaration_candidates(offset)` exposes only registry-validated declaration alternatives, their exact retained body sources and an explicit unknown residual. Declaration metadata consumers can retain a second possible event after an opaque first declaration without asserting that either handler executed. A validated procedure declaration with a stable actual handler publishes its name on the normal path; unresolved compiler errors still stop before subsequent declarations. Replaced handlers and unbraced bodies cannot borrow the stock declaration grammar.

`SourceRuntimeReachability` distinguishes `Reached`, `NotEntered`, `Conditional` and `Unknown`. `Conditional` retains a declaration or template preview and its potential source effects; it proves neither actual entry nor non-entry. Only completed coverage can prove `NotEntered`. Opaque evaluation, suspension and unsupported host behavior retain `Unknown`; callers must not treat every absent point as an unentered call. Call-site summaries may omit a proved unentered invocation, but must retain conditional and unknown callers as possible input. Test both a command after `return` and a runtime-unentered branch whose original condition requires compilation: each retains its actual compiler recipe without acquiring runtime effects. A statically false branch can be omitted by the native compiler itself and must not receive an invented recipe. Pair those with dynamic evaluation whose entry remains unknown.

## Preserve actual script locations and automatic native error receipts

`tcl_runtime_api::script_source_location::ScriptSourceLocation` retains a real
source-entry filename and the one-based start of that script value. The source
handler supplies its filepath; plain host evaluation can legitimately have no
filename. `Value` and `CompiledUnit` retain that metadata through procedure
creation and script entry. `Instruction::source_value_line` records the original
literal word's creation line, independently of the instruction's execution
line. The codegen source-word owner stamps it only when original literal source
coordinates prove that line. Escaped, concatenated and otherwise unproved
materialized values do not acquire a filename or an affine mapping by equality
of bytes. Recompiling an existing procedure preserves its original body value's
location. Selected foreign variable frames do not substitute their source
location for the executing script's location.

Automatic Jim errors use `JimEvaluationFrame` receipts, not C error annotations
or a lexical reconstruction of argv. Dispatch records the actual selected
command name and whether it is a procedure. Its invocation view borrows the
actual execution-owned argv; the error boundary materializes the list while
those objects remain live. Procedure depth remains separate from physical `uplevel` selection. The
shared `capture_jim_error_frames` owner selects Jim's evaluation frames before
they unwind and emits four components per record: procedure, file, line and
invocation. Missing evidence abstains. `JimErrorStack` captures once per episode,
preserves the last cached trace across successful evaluations, and accepts an
explicit trace as an arbitrary raw value. A synthesized procedure return error
captures at its actual boundary after removal of the issuer. Native tailcalls
retain their lookup namespace and resolve their target only after issuer frame
teardown and leave callbacks.

Error bookkeeping must not introduce object owners or string conversions before
execution. Jim can choose an in-place string operation from actual sharing and
retain a native cached character count. Cloning every argv element into a
diagnostic list changes that decision; stringifying arguments for a byte-only
frame can also change bytearray purity. Keep the execution's argv owner and its
diagnostic view distinct. A suspended continuation retains its real execution
owner, while a synchronous adapter can borrow caller-owned handles with a
scope that ends before their release. Restore that scope on unwind and swap it
with the matching coroutine context. Capture bytes only on an actual guest
error, before the borrowed argv is released; a host-only refusal must not
materialize or publish guest error frames. `info level`, argument bindings,
alias forwarding and pending tailcall ownership follow their separate native
contracts. Do not subtract diagnostic references from an object's sharing
count or compensate inside a string operation. Test unique versus shared
cached strings, normal dispatch without string generation, error capture
before teardown, and suspension with the unchanged automatic-error corpus.

The permanent `JIM_AUTOMATIC_ERROR_CASES` corpus is consumed by both runtimes
against an actual configured Jim interpreter. Its 20 cases cover root, nested
procedure, eval, selected uplevel, rename, tailcall, explicit trace, return error,
new catch, substitution, lookup and arity failures, namespace entry, cached
trace, multiline body-source location and malformed scripts rejected before
prefix effects. Standalone Runtime retains byte filenames in a private object
sidecar without changing the public C object header; object retirement and
value mutation revoke that metadata, while duplication preserves it. These
checks complement the shared
presenter's tests; the presenter alone does not certify a runtime's receipt
capture or source-location handoffs.

## Emit original variable operands and expansion segments

Use `registry_invocation::compiled_local_name_value` for direct local-variable eligibility and its evaluated name. Source spelling alone cannot decide this: a braced `{a($x)}` names a literal array element, a substituted `a($x)` evaluates its key, and a bare backslash-decoded name can require the stack protocol even when its resulting value is known. Emit the original word through the hook's operand map for the stack form; do not replace it with a cached value or evaluate an inert private selector.

`CodegenCtx::emit_native_argument_list` owns native list construction from original words, bracing and expansion boundaries. Its `arguments_from` indexes logical hook operands; `prefix_count` includes already evaluated stack values. `yieldto` supplies the captured current namespace as one prefix value. Expanded `lappend` supplies only its value operands, starting after the variable name. Its grammar permits expanded values in C Tcl 9.1 while an expanded variable-name word remains generic. A single expanded list also requires the release's representation protocol before the variable update. Keep this compiler rule in `NativeCompilationGrammar::VariableAppend`, rather than selecting it by command spelling in downstream consumers.

`emit_word_from_source` retains the lexical components of an unchanged compound word. Text fragments are decoded once and pushed as data, variable fragments perform their actual reads, and command fragments retain their own nested invocation source proof. The substitution plan must still agree with the emitted value; a rewritten value cannot borrow the old components. Braced words remain literal and opaque components keep the established fallback. Protected catch/try statements use the ordinary `emit_stmt_with_start_cmd` boundary owner, removing only the final result's `POP` when the capture needs that value. A suspension inside one argument cannot erase the surrounding command boundary or the next command's quoted variable reads.

Select actual lexical axes **before segmentation** with
`LexerConfig::with_grammar(actual.lexer_grammar)`. This central overlay retains
source offset, line, column, strict quoting and the caller's BOM mode. Both
`Lowerer::set_source_analysis_options` and the explicit `SourceCommandBindings`
entry apply it; materialised scripts use their retained before-call dialect.
An absent execution dialect preserves the supplied explicit configuration.
Changing grammar only at emission cannot repair an earlier word split.
`CommandTokens::native_lexer_config(fallback)` gives nested-source consumers the
same retained choice; `word_subst` applies it before lifting nested calls.
These helpers select syntax only, and grant no command, cell or representation
identity. Regression coverage pairs Jim `$café` with C Tcl's shorter variable
name and checks that source coordinates and parser modes survive the overlay.

Pass `Module::native_lexer_config()` when consuming original syntax. It selects
actual invocation grammar over assistance metadata and preserves parser mode
and source coordinates. `whole_var_ref` recognises one entire original
reference. Bytecode's `emit_variable_reference`, native expression lowering
and word planning consume this shared syntax. Template emission retains a
separate `LiteralElement` operand for a braced name's finished key. Do not send
that key through substitution, even if it contains dollars, backslashes or
brackets. Direct WASM scalar procedure admission must reject an element
reference rather than use the AST's base name as a local slot.

C Tcl accepts `$(k)` as an empty-name array read; Jim selects expression sugar
there. Jim also accepts high bytes in a bare name and counts nested index
parentheses. C Tcl ends the name before high bytes; C8 indices stop at the
first token-level close parenthesis, while C9 rejects a raw nested opening
parenthesis as an invalid character in an array index. The main lexer and component scanner share
these boundaries. An unsupported expression substitution declines variable
emission and retains its original execution path. Verify the scanner and
source emitter units alongside
`original_variable_operands_preserve_native_index_evaluation_and_lexical_axes`:
its actual C8.4–9.1/Jim comparisons cover word, template, nested-command and
expression consumers. The tests must reject partial emission for compound
references and compare a literal key with a reached dynamic-key sibling.

The `variable_word_` rows in `execution_conformance` own those original scripts.
The VM and standalone Runtime consume the same twelve specimens, including
the C9 nested-index rejection, Jim empty-array expression syntax, and separate
successful and failing empty-name cell accesses. Run the owner corpus and both
execution consumers when changing variable syntax or emission. Keep actual
native failures as expected outcomes; an unsupported host path is not native
agreement. Compare original result, options, cells and callbacks at each
consumer's actual execution entry.

Moving a computed store into a later operand requires `SsaSourceView::read_word_produces_value` as well as the exact represented read version. Earlier variable operands must pass that same physical read-success query; a version alone does not establish that a scalar read of an array root, a missing variable, or an invalid Jim dictionary element succeeds. Keep opaque earlier-word evaluation as a motion obligation.

No-value operations need their own normal contracts. `append x` reads an existing cell and errors when it is absent. `lappend x` reads an existing cell and creates an empty value only when absent; C Tcl validates the existing list, while Jim preserves its bytes. An existing value must not receive a synthetic write trace. Compiled container operations can retain a physical cell across a read callback where a generic command re-resolves its name. Put that distinction in the shared variable-update owner and test deletion, recreation, alias retargeting and parent-array retirement before changing an opcode adapter.

## Count the selected signature through its shared grammar

`ArityCount::Arguments` counts every frozen argv entry after the command and
selected subcommand. It remains the default: do not silently subtract options
from existing signatures. `ArityCount::Positionals` is an explicit authored
contract (`Arity::with_positionals`, SpecTcl `arity -positionals ...`). It counts
only operands after that descriptor's leading option grammar. For example,
`interp create -safe jail` has one positional operand, while
`interp create -- -safe` also has one: the final dash word is a name.

Use `ResolvedInvocation::argument_count_for_arity` or
`InvocationFacts::arity_accepts_frozen_arguments` after structured resolution.
Legacy `ResolvedCall::argument_count_for_arity` delegates to the same owner.
Do not call `facts.arity.accepts(raw_source_word_count)` in a consumer.
`Some(false)` proves signature rejection; `None` means the selected grammar
has not established a count. Unknown cannot prune a continuation as an arity
error or license a successful transfer. Accepted count alone does not prove
option-value validity, successful execution, compiler selection or effects.

`spec::leading_option_word_count_for_arguments` is the source-word boundary
owner. It uses the same exact-name, alias and minimum unique-abbreviation
matcher as the literal adapter; callers supply an availability-filtered option
table and the declared prefix policy. A declared `--` or proved non-option
prefix establishes the boundary even when trailing values are dynamic. A
dynamic option position or unknown expansion leaves it unresolved. Value spans
come from `OptionSpec`; fixed-arity values consume their declared slots even
when dynamic or equal to `--`. A value-dependent arity hook needs its actual
inputs before it can establish the boundary.
Never fill unknown words with empty strings to obtain an execution proof.

When adding a signature count mode or option shape, test raw-count signatures,
subcommand offsets, aliases, abbreviations, repeated flags, terminators, dynamic
positions and expansion. Keep SpecTcl, Studio renderers, release-window editing
and presentation in parity. The positional-arity tests cover these contracts;
C Tcl 8.4–9.1 probes establish `interp create` behaviour independently. Current
Jim rejects those child-interpreter forms: catalogue visibility does not make
them native Jim operations.

Argument-role resolvers declare their input purpose. `ArgRoleResolverInput`
separates frozen literal values from exact cardinality;
`InvocationSemantics::arg_role_resolver_input` exposes the selected contract.
`arg_role_count_resolver` receives only the count, so `foreach x $values body`
retains its body position without claiming any contents for `$values`.
An unknown expansion count keeps roles incomplete. Never call a value resolver
with invented empty operands to recover count-dependent positions. SpecTcl
accepts a known native count descriptor and excludes unsupported Tcl count
hooks; Studio retains that independent field in both renderers.

Deferred timing does not identify a future activation. Query the selected
`BodyExecutionSpec::deferred_entry_frame(actual_dialect)` before retaining a
future script entry. Native `after ms script` and `after idle script` author
`DeferredGlobalScript`: their exact selected Body operand later enters the
global frame, independently of the registering procedure's locals. Six-engine
probes establish that frame; host or missing native policies remain unknown.
Future body inventory can retract caller precision, but cannot mutate the
registering call's normal continuation or invent actual runtime reachability.
`SourceCommandBindings::future_body_inventories` retains each independently
analysed entry. Lowering projects its exact registration, source, offset and
binding into `Module::future_call_sites`; UnitScope uses those possible callers
only to retract seeds. A bounded unrelated target leaves other seeds intact,
while an unknown future head retracts its explicitly permitted reach set.

`NativeVwaitProtocol` selects the original argv grammar separately from a live
variable cell or notifier capability. The basic wait owner accepts one original
name; selected legacy option-like names remain distinct from extended forms.
Extended C options, channels and Jim signal/script forms require separate
capabilities. Both ports decline those forms before registering global observers
or processing pending events, without treating typed unavailability as a native
guest arity result. Registry source roles use the same selected operand layout.
See the [vwait operand contract](../analysis/name-resolution-proofs/event-original-vwait-operand-boundaries.md).

Callback taint replay also uses independent retained procedure entries. Its
unique synthetic formal receives the explicit callback input taint, and one
interprocedural solve processes the batch. Do not append sequential synthetic
calls: arbitrary effects of the first callback would erase another callback's
incoming command proof. Retain the actual source entry and loaded provider
contract when rebuilding that batch.

Runtime expansion and compiler syntax have separate carriers. The source walker
freezes each written word immediately after its substitutions, using the actual
native list grammar for an expansion. `EffectiveInvocationWord::KnownExpansion`
retains the resulting elements; runtime invocation adapters flatten them before
selecting roles or checking handler arity. A later substitution cannot change an
already frozen expansion. A malformed list stops before later substitutions;
an unknown list retains unknown cardinality.

`InvocationWordOrigin::ExpandedElement` identifies both the original written
word and the native list element. It supplies neither an editable source range
nor a fresh object or retained representation proof. An expanded script value
therefore enters a materialised source instance. Native admission and Named
emission retain the original expanded word shape and evaluate that original word
once. They do not execute the frozen element bytes. The frozen-argument tests
cover ordering, malformed lists, compiler shape and materialised script origins.

## Carry the actual entry through every layer

Resolve the editing environment once and retain its complete profile, lexer grammar, registry snapshot, provider contracts, and source identity. `Module::dialect_profile`, `Module::registry_snapshot` and `Module::lexer_config` carry those exact handles into consumers and code generation; `resolved_profile()` and `resolved_registry()` expose them, with a documented name fallback only for compatibility modules; `SourceAnalysisEntry::invocation_dialect` carries the actual runtime policies independently. A profile name is a presentation/compatibility input, not a way to recreate a custom profile or native engine. Use `DocumentEnvironment::default_execution_point` only when the complete canonical environment identity proves its authored default. A custom environment, an altered profile with the same name, or a family without a proved release retains an unknown runtime point. Explicit runtime/profile selection overrides the editing default. A runtime driver supplies its measured engine point independently; it must not obtain one by guessing from a command catalogue.

`ResolvedAnalysisInput` supplies the corresponding analyser ingress. Its four
inputs are the actual analyser profile, compilation-unit profile, availability
generation and body lexer configuration at offset zero. Keep the two profiles
separate: an environment such as Tk can use permissive analyser assistance
while its compilation units retain a specific core and package surface.

```rust,ignore
let input = ResolvedAnalysisInput::new(
    analyser_profile,
    unit_profile,
    generation,
    body_config,
);
let analysis = Analyser::new()
    .with_resolved_input(input)
    .with_source_analysis_entry(actual_execution_entry)
    .analyse(source, display_label);
```

The input's read-only `analyser_profile`, `unit_profile`, `context_registry`
and `lexer_config` accessors expose the retained axes to adapters. The context
accessor shares the same immutable generation; it does not assemble a new one.

Full, chunked, incremental and per-item walks use that same ingress. Deferred
bodies retain it, and `ItemBodyKey` includes it independently of byte offsets.
Clones share the immutable generation; a different generation invalidates the
body key even when its display name is unchanged. Snapshot restoration and a
full-rebuild recovery retain both the editing input and the separate execution
entry. A snapshot taken during a default walk captures the environment actually
resolved for that walk. Restoring an empty snapshot withdraws an earlier
temporal world; it cannot leave the receiving analyser's old authority attached. `ContextRegistry::with_command_store` reassembles the supplied store
under the retained availability context; it does not infer a context from the
store's label or attest native implementations. `RegistrySnapshot::shared_registry`
shares that exact frozen store when an analyser context must retain a module's
authored overrides and package surface; callers must also retain the original
availability context.

`AnalysisResult::resolved_profile` and `resolved_registry` expose the retained
editing metadata. LSP consumers use `profile_for_analysis` and
`registry_for_analysis`; their name-based compatibility path applies only to
records without retained inputs. `AnalysisResult::retained_command_realm`
exposes the original temporal world, including its native-entry and provider
obligations. A missing realm cannot be replaced by analysing the source again
under a default profile. The retained realm participates in result equality
through every semantic inventory; only derived lookup caches are excluded.

Tests must distinguish retained inputs from a display-name reconstruction,
exercise isolated bodies and incremental recovery, and check cache isolation
between generations. The native `${::a{b}c}` fixture returns `7c}` on C Tcl
8.4–8.6 and Jim, and `8` on C Tcl 9.x. A retained 8.4 profile labelled `tcl9.0`
must preserve that retained variable-name rule throughout the analysis.

An embedder with an established `SourceAnalysisEntry` supplies it through
`CompilationUnit::build_with_source_entry(source, options, &entry)` or
`Analyser::with_source_analysis_entry(Arc::new(entry))`. The analyser forwards
that same entry to document-realm construction, CFG compilation and whole-file
trust analysis; `document_realm_bindings_with_source_entry` is the shared realm
adapter. Default constructors delegate to the same build implementation. Do not
add a second entry constructor that silently discards a package loader contract
in a later analysis phase.

With a driver-established `entry` and matching `options`, the shared ingress is:

```rust,ignore
let unit = CompilationUnit::build_with_source_entry(source, options, &entry);
let analyser = Analyser::new().with_source_analysis_entry(Arc::new(entry));
```

The driver supplies the entry; the example does not reconstruct it from a
dialect name. Use the same retained entry when creating a cross-file scan or
an incremental cache key.

`SourceCommandBindings` and `CommandBindingRealm` compare retained semantic state, including source origins, deferred/future bodies, compiler visits, variable continuations and runtime coverage. Only the derived unpositioned projection cache and per-point `OnceLock` lookup cache are excluded. Consumers may retain the exact realm across analysis results without reconstructing a dialect or treating a pointer as semantic equality. Immutable `SourceLookupSnapshot` carriers retain the complete original state and realm without computing a fingerprint during ordinary lookup. Hashing or equality initializes its derived memo on demand; a fingerprint match still requires full semantic equality. Joining a retained point invalidates the detached snapshot's memo and preserves the predecessor snapshot. The memo grants no command, compiler, frame or object authority.

A package catalogue and a loaded implementation are different evidence. A
Tcllib factory's authored successful object return describes a selected known
handler; the command spelling alone does not establish that handler. Positive
provider tests need an actual trusted loader/implementation entry, with paired
unloaded and replaced-handler controls. A Tcllib package may select a pure Tcl
or accelerated implementation, so object-return metadata cannot establish a
native compiler hook or its absence. Retain the selected implementation's
independent compiler registration.

A selected definer recipe also needs explicit attestation. Populate
`TrustedPackageLoader::definition_dispatchers` for the exact installed factory,
and retain its complete private/core lookup and namespace-state dependencies.
The default empty set proves no recipe. The bounded Snit 2.3.4 path accepts only
literal deferred declarations under the attested `SnitType` recipe; immediate
initializers, type constructors, altered private workers, changed templates and
replaced dispatchers decline. Native inspection identifies its standard type
command as an ordinary namespace ensemble, not a procedure or alias. The
alternative simple-dispatch template is a nonempty procedure, but selecting
that variant requires separate declaration/configuration proof. An audited live
installation receipt supplies compiler registration independently of nominal
name-return metadata; neither command kind nor the returned name licenses
constructor execution or an opcode.

For `package require snit; snit::type ::Counter {method bump {} {return 1}};
Counter create mine`, the reached creation site's retained binding can answer
`binding.nominal_definition_name_result(registry)`. Its closed deferred
`typemethod` inventory excludes named methods from bare-word construction;
`conditional_construction_name_at` owns that layout alongside explicit
manufacturers. On normal completion this is only a callable-name possibility
for W307. Constructors may mutate or remove
an instance before returning; the query establishes no current object, class,
method set, W308 fact, purity, or executable specialization. The paired fixture
`snit_dispatcher_recipe_is_conditional_nominal_and_dependency_guarded` checks
missing attestation, replacement and private-state mutations.

The separate `ItclClass` recipe is attested for Itcl 4.3.2 on C Tcl 8.6.18,
9.0.4 and 9.1.0. The independently built native providers agree on the bounded
dispatcher protocol and inspected compiler hooks; C 9 adds the exact installed
`::itcl::build-info` dependency to its otherwise identical 143-command roster.
Its bounded declaration accepts literal deferred methods, class-scoped procs,
constructors and destructors after validating the provider's installed private
roster, stock core lookups, mutable namespace state and execution observers.
Immediate directives, duplicate declarations, unsupported formals and uncertain
name/namespace allocation decline. Itcl class-proc slots such as `C::ping` are
distinct from `C ping`, which constructs an instance named `ping`. Its normal
name-result protocol preserves a relative requested spelling or substitutes a
generated `#auto` suffix; it does not imply a qualified result. A constructor
can remove that command before returning. Actual compiler inspection supplies
an independent absent-hook receipt for the installed dispatcher. No TclOO
class, instance, constructor closure, W308 or specialization proof is inferred.
The [native Itcl probes](../../../rust/tcl-compiler/tests/data/native_itcl/README.md)
reproduce these distinctions and private-parser observer failures.

Native expression source follows the same origin discipline. `ExecutedExpressionSource::from_arguments` retains the single native operand unchanged, or combines multiple C Tcl operands through the shared `concat_values` policy, trimming each selected operand before joining. Jim's selected expression grammar rejects multiple operands. The expression has its own derived origin; unchanged operand slices additionally retain their original origin and offset. `variable_source` and `SsaSourceView::read_executed_expression_variable` resolve a variable only through that exact piecewise map and the retained physical read context. Decoded literals, alias-prefix values, expanded words and references spanning pieces supply no authored read site. These read projections grant no whole-expression preparation, opcode, constant-folding or source-edit permission.

Expression preparation and reached operand coercion have separate sharing footprints. Preparation can convert original literals and compiler pool objects before runtime reads; it cannot borrow the current bytes of an unexecuted variable operand. The source walker snapshots each runtime operator's possible operand bytes before evaluating its operands, records their original physical reads, then invalidates representations on the continuing operator path. Boolean conditions convert before a lazy branch is selected. Unknown callbacks or sharing retain an unknown footprint. Numeric-only conversion may preserve an independently proved current numeric shape while retiring concrete numeric and frozen object receipts. This timing does not publish a container type from an expression operator or change contents merely because its representation changed.

Expression read queries validate both the original extent and the exact original
variable spelling. A synthetic `$z` node placed over the original `$x` extent
cannot borrow its storage, contents or representation receipt, even when the
two names have the same length. Preserve the parser's original node and source
base when calling `SsaSourceView`; an unpositioned node supplies no read receipt.
The paired regression
`expression_read_requires_original_spelling_as_well_as_extent` checks the real
read alongside same-extent substitutions and missing source bases. Apply this
same identity discipline when adding projections for decoded or combined
expression sources.

For an actual file read, put a `TrustedSourceModuleLoader` in
`SourceAnalysisEntry::trusted_source_modules`. Its constructor takes the
audited native family, evaluated path, selected encoding, driver-owned file
implementation identity and decoded source bytes. The driver applies the
native encoding and file EOF rules before supplying those bytes. This is a
contract for a file the native loader can actually read; a workspace filename
or matching procedure text supplies no such contract.

The selected native handler uses `source_file::select` to locate the filename
and encoding operands and `source_file::completion_route` for the file return
boundary. Source analysis enters the retained `ExecutedScriptSource` in the
caller frame and restores the previous origin on every completion. A
`SourceOriginKind::Loaded` retains the file implementation identity separately
from authored and materialised source. Unknown paths, a replaced source
handler, conflicting loader contracts or invalidated filesystem dependencies
cannot select that loader.

Cross-file drivers use `scan_source_call_sites_with_source_entry` and forward
the same entry into `CompilationUnit::build_with_source_entry`. Caller evidence
retains the reached `CommandAllocation`; correlating it with an analysed
declaration requires the retained source implementation, declaration offset
and body bytes. A matching qualified name or equal bytes alone cannot identify
the live procedure after replacement. `matches_source_declaration` is a
contents-analysis projection, not an edit, opcode, frame or object-erasure
licence. Test loaded and unloaded entries, unknown filenames, partial source
errors and later redefinitions when changing this path.

Actual variable-observer knowledge is a separate entry axis. `NativeVariableObserverPresence` distinguishes a closed absent table, bounded engine hooks, possible callbacks, and unknown registrations. `permits_no_callbacks` removes only the arbitrary-callback residual for the first two cases. It supplies no variable value, namespace-cell existence, alias lifetime, object representation or frame proof. Runtime providers must derive it from their actual complete observer state; foreign or partial providers retain `Unknown`. Source installation preserves unknown contents while applying this independent fact. Later source trace changes and callback reentry must still update observer state and command-selection timing. Test a global increment with no observer and with an observer replacing a command, across the release-specific compiler behavior.

`NativeCompilationEntry::namespace_variable_tables` is an independent allocation
inventory. The VM captures complete original root keys, including undefined and
linked entries, from the actual C namespace VarTables with their interpreter,
namespace token and component path. Command-table closure cannot supply this
inventory. The source importer accepts only matching native C issuers and exact
visible namespace incarnations; a missing, foreign, conflicting or opaque table
remains open. These rows prove neither values nor links, representations,
observer absence or compiler-hook identity. `NamespaceCellPresence` can prove
incoming root absence only within an accepted table. A closed newly created
array additionally requires that exact absent root and an original successful
literal element store. Unknown callbacks or world mutation withdraw both table
absence and `closed_array_roots` evidence.

`NativeCompilationEntry::same_compilation_world` compares those inventories when
the VM reuses cached source compilation. Original ensemble target shimmering
does not invalidate this comparison, while its retained registrations,
configuration and context still have to match. This method is used by the three
VM source-cache entry checks; it is not a running-frame freshness test.
`NativeCompilerCacheStamp` and actual native Bytecode compiler epochs remain
separate physical cache owners. A legitimate variable store does not acquire a
new compiler epoch from the inventory comparison.

Source callback interpretation uses the physical receiver selected before the access. Registrations run newest first within a cell, with array-root callbacks preceding element callbacks. The active read/write guard follows that receiver and element; it is not a guard on the callback prefix or the root registration. A different element can reenter the same root callback, and unset callbacks still run during an active write. The shared execution vectors retain these five C-release controls. Known source prefixes execute in their actual callback frame. Unknown registrations, unenumerated member-order alternatives and unsupported callback-driven registration changes keep an explicit residual. A captured normal store publishes to its original receiver before delivering write callbacks; it cannot re-resolve a name after a callback. Read-observed read-modify-write operations retain a `CapturedCellArena` receipt through callbacks and selected-frame restoration. Contents reset preserves the protected scalar or element shell; whole-array and namespace retirement are permanent for the original receiver. A recreated name cannot revive that receipt. The active trace guard consults its live receipt, so a recreated array root can run its own trace during the old root's callback.

`ResolveContext::capture_protected_cell` requires a distinguishable active owner. A receipt identifies a reached capture, not a unique source allocation family. Repeated recursive activations sharing one abstract owner cannot establish a Must-same receiver; unknown mutations and divergent lifetime joins withdraw the lease evidence. Do not compare `After(offset)` generations alone to identify a recreated instance. Destruction preserves the original root kind until retirement; it does not publish a scalar value definition.

Whole-array unset follows `InvocationDialect::variable_destruction_protocol` and the neutral `VariableDestructionPhase` contract. Root lookup removal and root trace retirement do not retire the original elements. The captured root callbacks run while old element aliases and their read guards still select that physical generation. Each original member captures its current physical callback registrations at its own retirement phase, then becomes undefined immediately before those callbacks. Root callbacks and earlier member callbacks may mutate a later member’s registrations through an old element alias; changes through a recreated root name select that new generation. Not-yet-retired members remain live. A root callback can update an old element alias, while that member's own unset callback cannot store through the now-retired alias. Runtime `RetainedArrayCell` holds the detached generation separately from a refillable root RMW shell. Variable trace registrations retain the owning root generation, so a recreated name's trace changes cannot replace the old callbacks. Native execution uses its actual table iteration; abstract analysis must retain unknown order or join supported member permutations, never infer alphabetical native order.

The registry's `native_rmw` policy owns fetch errors, amount grammar, validation timing and native failure presentation. Tcl 8.4 validates an increment amount before its read callbacks; later C releases read first. Root read callbacks can replace array contents with a scalar, whereas a non-array element lookup fails before callbacks. Known prefixes registered on an unenumerated target remain possible observers. Only a closed, normal, world-neutral callback can preserve precision across its optional execution; a mutating, failing or unknown prefix retains the residual. The durable RMW corpus records the selected native differences and recreated-root trace control.

A reused interpreter supplies `NativeCompilationEntry` to `CompileService::compile_script_with_entry` or `compile_procedure_with_entry`. Custom compile services delegate these overloads explicitly. Forwarding only `compile` silently loses the command generations, paths, aliases, execution traces, and actual variable activation that make specialization valid. `BytecodeCompileService::native_entry_options` is the shared adapter into source analysis; `native_entry_config` selects the measured `NativeCompilationEntry::lexer_grammar` for runtime compilation. The entry retains three independent contracts: `profile` identifies the source policy, `invocation_policy` identifies the logical native handler policy (including an active host override), and `execution_point` identifies the physical compiler engine. The measured `lexer_grammar` is independent of all three; an engine point alone cannot reset configured grammar flags. Use the adapter and lexical configuration before segmentation or validation. The assistance profile supplies catalogue identity, never missing native evidence. Without measured lexical grammar, preserve the supplied explicit configuration.

`SourceAnalysisOptions::native_compiler_dialect()` and the positioned `SourceInvocationBinding::native_compiler_dialect()` project only the physical compiler contract. A supplied entry with no engine point returns `None`, even if its logical policy or the assistance profile names Tcl. Without a live entry, an explicitly authored dialect continues to describe authoring compilation. Use this projection for compiler-hook selection, original implementation prerequisites, compiler frames and operand-layout recipes. Original argv materialisation uses the logical dialect's retained lexer and word-value rules; normal handlers, formals, frame selectors, numeral and character policies, object results and completion settlement use the logical invocation policy. Passing the physical projection to those consumers would let a C9 simulation engine donate C9 semantics to F5 source.

The adapter and direct source entry obtain logical policies through `SourceAnalysisOptions::logical_invocation_dialect()`, exclusively from `invocation_policy` when a live entry is supplied. A handwritten options object cannot override a missing or different live policy with its authoring dialect. `SourceAnalysisOptions::native_lexer_config(config)` separately chooses measured lexical axes while preserving the driver's coordinates and modes; missing measured axes keep that configuration even when the logical policy is unknown. Source interpretation and lowering overlay only these retained lexical axes on logical word rules, preserving logical numeral/frame policies. Missing logical policy cannot be repaired from `profile`, the engine point or editor assistance. Unknown physical compilation and unknown logical handler semantics remain separate: either axis can be retained while the other is missing. `physical_c9_compiler_cannot_replace_f5_logical_value_or_frame_policy` exercises F5 source on a C9 compiler, a separate Tcl8.4 host policy, and both missing-axis cases. `native_entry_keeps_logical_engine_and_lexical_evidence_independent` covers measured Jim grammar with a different assistance profile and a separately configured grammar flag.

Public embedder registration of a command named `set` does not establish the engine's stock implementation. Only the engine's bootstrap registration may stamp native identity. A wrapper or test double must preserve that distinction.

Compiler-hook presence is another independent axis. A host command can have opaque semantics and a proved absent compiler hook; that permits generic native compilation without granting builtin effects. Use `NativeCompilerHookPresence` from the actual registration. A missing registry descriptor cannot prove hook absence. Procedure header compilation is separate from body admission, and native optimizations of particular procedure signatures need their own audited policy.


Native compilation has its own frame and context: file-root direct evaluation, a compiled script object, and a procedure compiler differ. Runtime caller frames used by `upvar` or `uplevel` are another axis. Retain loop and exception-range context through the registry's body-operand transitions. A separately compiled body starts a new compiler context even when it runs in the same variable activation.

### Retain conditional source declarations

Authored procedure layout comes from `ResolvedInvocation::authored_source_procedure_arguments`. The selected descriptor, original argv cardinality and authored name/formal/body roles define possible source positions. This query supplies no native parameter acceptance, successful publication or entered body. Native `InvocationFacts` retain their own completeness and acceptance outcomes.

`SourceCommandBindings::original_logical_procedure_declaration(tokens, registry, input)` issues a conditional declaration from the complete positive Logical input and unanimous original declaration observations. It checks source geometry, lexer configuration, command store, retained namespace and selected source descriptor. Its `LogicalProcedureDeclaration` supplies the authored name/key, strict shared formals, original parameter text and original deferred body. Captured alias operands retain their effective positions; an original body must retain its actual written word. Lowerer consumes this source recipe for procedure/body inventory while preserving generic runtime dispatch.

`OriginalCompilationLookupAdvice::closed_logical_source_lookup` checks the independently retained Logical command world. Missing targets, unknown or absent alternatives, opaque mutations and missing namespace/input remain terminal. An unknown physical variable frame does not supply or retract this separate source command layout. The query grants no physical frame, native lookup, normal completion or execution effects.

Quiet Logical rename and alias continuations use their exact original source vector, selected Registry transition, retained source table, occupancy checks and observer guards. Registry abrupt-edge commit classifications remain unchanged. The conditional authored continuation is not a native successful-completion certificate.

`ResolvedInvocation::authored_source_possible_option_arguments` follows selected availability, prefix grammar, reserved words, option widths and terminators. A dynamic source operand preserves every admitted next position; unresolved option widths preserve later uncertainty. Compiler taint advice consumes the same whole original vector under retained metadata and lexer configuration. Missing, foreign or mismatched supplied metadata refuses assistance; explicit standalone source queries retain their separate contracts.

## Retain original source bytes and parser channels

`SourceImage` owns an immutable byte buffer and its `SourceChannel`. A document
and an evaluated native value can contain equal bytes while using different
continuation and command-boundary rules. Retain the channel supplied by the
entry owner; content inspection and source-origin labels cannot reconstruct it.
`ExecutedScriptSource::text`, `SourceOriginId::source_image`, `Module::source`
and bytecode command-source fields retain that image through execution,
compiler admission and error reporting.

Use `Lexer::with_source_image`, `SourceMap::from_image`,
`build_document_image` and `segment_commands_image_with_offset_and_config`.
The image segmenter returns `Some([])` for a proved empty script and `None`
when its analytical tree is unavailable. An unavailable tree cannot license
an empty executable body. A checked `try_text` view is suitable for Unicode
analysis only while the original image and channel remain retained.

`NativeWord::from_group` captures original byte words from the shared lexer and
command grouper. Its complete written span includes expansion markers; its
operand and substitution parts preserve their original extents. Closing
delimiters and variable extents come from the shared lexical owners. Do not
reconstruct those spans from token lengths or decode opaque names for storage.
Native lexical provenance establishes no command implementation or native
compiler permission.

Executable word consumers use `NativeWord::executable_parts()`. Its arena owns
one immutable source image and flat ordered lists; a variable index selects a
child `PartListId` rather than recursively nested vectors. Traverse these lists
with an explicit work stack to preserve variable-name/index/read and
concatenation order. Name and script spans select original bytes, while literal
text distinguishes unchanged source bytes from decoded escape output.
`ExecutablePartArena::source_span` supplies the shared token-closer convention
without changing the full raw span. `decompose_template` requires independently
selected native template variable grammar; ordinary written-word decomposition
cannot grant Jim's different `subst` acceptance. Geometry refusal and authentic
syntax-error components remain separate. The bounded `NativeWord::parts()`
compatibility view can be unavailable while the authoritative arena is complete;
never use its advisory fallback to prove constness, effects or admission.

Compile through `ScriptCompileTargetBytes` or `ProcedureCompileTargetBytes`,
with a constructed `ByteNamespacePath` and original `NameBytes` formal keys.
The concrete byte service retains actual entry facts. Custom services implement
the byte doors explicitly, including their refusal and plain-dispatch behavior.
An opaque `Statement::NativeCall` retains unknown variable and callback effects;
empty named read/write sets do not establish purity or writable source positions.
C compiler obligations remain attached to the complete original source unless
actual compiler evidence discharges them. Emitting individual generic calls
cannot bypass an unresolved chunk-entry obligation. `Emit::emit_command_image`
and the WASM byte interner preserve a host-admitted script's complete bytes.

`NativeCompilationEntry::lookup_command_bytes(namespace_token, original_bytes)`
resolves only the immutable actual compilation snapshot. It returns a unique
registration, closed absence, or a typed missing/conflicting capability. C
lookup uses the selected CString extent and ordered namespace-path/global
candidates. A retained deleted namespace remains distinct from a visible
same-named recreation; unresolved descendants of the retained generation cannot
borrow the new tree. Jim requires its independently retained namespace-object
bytes. The result establishes neither handler semantics nor future dispatch.

The compiler's `closed_generic_byte_compilation` consumer discharges only
original generic compilation. It requires an authenticated actual C policy,
the exact current namespace, and original `NativeWord` geometry. Constant heads
must resolve to an absent compiler hook or closed absence. C8.4 and C8.5 require
a single literal-text component for command-specific selection; C8.6 and later
also accept text/backslash-only constant heads through their native value rules.
A variable or command-substitution head cannot select a compileproc from its
future runtime value. Every bracket child, including an array-index child, is
checked under the same snapshot before the complete source can lose its
provider obligation. Expanded heads, present/unknown compiler hooks, missing
native policy and unresolved lookup remain provider-required. This certificate
does not close runtime effects or authorize a command-specific opcode.

Native error-source formatting has its own authenticated recipe:
`InvocationDialect::native_error_log_protocol` and
`NativeErrorLogProtocol::excerpt`. Its byte clipping and incomplete-character
rules differ by C Tcl release. Neither host UTF-8 repair nor an authored grammar
supplies this native formatting authority; Jim uses its separate retained stack
objects and evaluation state.

## Preserve source evaluation and lookup boundaries

Original `CommandTokens` retain word shape, lexical origins, nested dispatch sites, and reached variable access contexts. Evaluated argument values are a second carrier. Never replace the original words with those values: the compiler selects from original syntax, while runtime operations consume the already evaluated argv.

Lifecycle callbacks can replace the lookup slot selected by a command mutation. Retain a receipt for the captured command instance and the exact mutation site before entering callbacks; deleting the captured old instance must preserve a new same-name command. A repeated factory allocation is an abstract family, so `RepeatedFresh` equality alone cannot prove that a replacement is the captured instance. Rename and namespace teardown require their own phase contracts: rename exposes a transient destination during callbacks, while a same-name namespace lookup during teardown can still select the dying namespace. The shared command-binding vectors pin replacement observers, moved replacements, repeated factory allocations and namespace teardown against all five C releases; Jim command-trace absence is measured independently.

Use `resolved_tokens_invocation` and its effective-word projection together for strict command facts and argument indices. Alias-prepended arguments have values but no editable source ranges. Preserve `EffectiveCommandWords::origins` when slicing or forwarding arguments; a decoded dollar sign in a braced or inserted value is not a variable substitution.

Selected roles are relative to their registry member. Add the retained
`InvocationFacts::argument_offset` once to obtain an effective post-head argv
index, then ask `EffectiveCommandWords::written_argument` for its original
written operand. Both accepted roles and `RegistryInvocationShape` candidates
use this mapping; a candidate retains its offset without acquiring execution
authority. For `info body p`, the role points to written argument 1; an alias
capturing `info body` moves it to written argument 0, and an alias capturing
`info body p` supplies no editable operand. `CaseInvocation` already indexes
the complete post-head argv and must not receive that member offset again.

Compiler-named descent has a different address space:
`SourceNamedInvocationProof::arguments_from` counts consumed original written
selectors. `ProjectedNamedArguments::capture` takes the original words, frozen
context and selected count, removes those selectors from the words and every
frozen vector together, then flattens known expansion for runtime argv. The
projected words retain their original source sites; values, representations,
object and method-prefix receipts, and selected physical reads retain their
original owners and epochs. `SourceScriptOperands` indexes those aligned
vectors. Captured late-handler prefix values remain in the target's separate
prefix carrier and cannot acquire a written word or physical read receipt.
An empty remaining argument list is valid. `ProjectionError` distinguishes a
missing head, unavailable consumed selectors, a misaligned sidecar and an
unaligned effective-word fallback. Any such error produces an unknown residual;
`Ok(None)` preserves an absent optional sidecar; a present sidecar retains its
head even when no arguments remain. This projection neither re-evaluates operands nor refreshes
receipts after mutation, and the full invocation inventory keeps its original
written positions for navigation and registration. Use the internal
namespace-code prefix test and the selected info-body alias role matrix when
changing this mapping; local offset arithmetic cannot replace the shared owner.

Command absence is not a proved error-only completion. A failed ordinary command lookup can enter `unknown`, an autoloader, or another retained fallback, which can read or mutate variables before returning or raising an error. Catalogue unavailability cannot establish that those effects are absent. Preserve the fallback target and prepended missing-command word in the same invocation projection. Recover a closed result/effect proof from an actual known fallback implementation, or retain its unknown effects. Paired native controls should include a fallback that reads old contents and mutates storage, and a known error-only fallback that permits the original contents proof.

Captured alias prefixes use `EffectiveInvocationWord` for every retained argv slot. An unknown retained value is one `Dynamic` word, not an empty literal and not a missing argument. Keep `EffectiveCommandWords::binding_prefix` and origins separate from presentation spellings. Never materialize a lazy runtime object merely to export compiler-entry metadata: use its existing string representation if present and retain unknown bytes otherwise. Export must leave shared object representation unchanged.

A diagnostic range is not automatically editable. Use `Function::statement_source_edit_span(block, index)` for CFG command edits and `Function::terminator_source_edit_span(block)` for branch conditions. These combine exact per-node script ownership with `Statement::source_edit_span` syntax eligibility; a missing semantic lookup at unreachable code is not missing lexical ownership. `Script::is_authored_source` gates recursive source walkers, and `is_fully_authored_source` is needed when a rewrite moves an entire structured body by offsets. Tests constructing CFGs manually must supply their actual authored carrier and include a missing-carrier negative. For a command deletion, typed synthetic boundaries can refer to a whole operand while owning no source command. Derived script bytes have their own `SourceOriginId` and coordinate system. Never apply their ranges to the authored document. Use the retained `ExecutedScriptSource` mapping to prove an affine authored range, or decline the source rewrite.

Structured script-operand spans cover the complete written word, including its closing delimiter. Lexer-token spans use a different convention. Normalize the range once in `Lowerer::script_word_span`; consumers must not widen a complete span again. `Module::lexer_config` retains the ingress grammar, including custom axes, and `PassContext::lexer_config` uses it for source extraction.

A body rewrite needs the original word's statically known evaluated value, not a later frozen value from a variable read or command substitution. `extract_body_text` delegates to the word/escape owners and returns `None` for unsupported input. Preserve the evaluated bytes verbatim: trimming or re-indenting changes nested braced data. End inserted scripts with a newline so a final comment cannot consume the following outer command. Deleting a final empty body or false loop can expose a preceding result; retain the wrapper until a result-use proof establishes equivalence. Validate both command completion and output in native rewrite tests.

A `NamedInvocation` fixes a private command **name**, not a command token. Modern Tcl's basic ensemble compilers can emit a name such as `::tcl::info::level` and remove the original member operand. Preserve the written operand indices when composing that projection. `native_named_command_words` supplies the runtime projection even when its late handler is unknown or absent; that uncertainty does not erase the selected name. Semantic consumers separately need `proved_execution_target`. Emitting the public ensemble, freezing the original private handler, or granting that handler's inline hook are different behaviors. Test both a public replacement and a private replacement during argv.

The name alone does not define the invocation layout. `NativeNamedInvocationProtocol::Direct` emits the private name and ordinary operands selected by a Basic compiler. `EnsembleRewrite` retains the original command word, canonical selected member words, and already-evaluated operands, then invokes the captured private name with `INVOKE_REPLACE`. Its usage rewrite preserves the public invocation in wrong-argument errors. Actual custom-map selection retains the canonical selected member separately from its mapped worker; two members mapping to the same worker must not collapse that evidence. `ArgumentUsageRewrite` owns chained ensemble and alias usage projection. Alias handlers dispatch their evaluated object vector directly through the native dispatcher with the selected lookup namespace and caller variable frame; they never reconstruct and compile that vector as a script.

Usage rendering retains the runtime's word type through
`ArgumentUsageRewrite<W>` and `rewrite_argument_usage`. The byte-valued Runtime
retains the actual argument objects in its `EnsembleRewrite` receipt and only
materialises the needed header words when `wrong_args_for_prefix` renders an
error. Capturing a rewrite must not stringify numeric operands or pure byte
arrays before their handler reaches them. It applies the actual rewrite before
the selected `NativeArgumentUsageHeader` policy.
`ListWords` uses the shared byte-list encoder, while `RawWords` retains native
space joining. Neither path converts a non-Unicode command name into replacement
characters. Binary handlers select their suffix through `NativeBinaryArgumentUsage`
and retain their actual invoked prefix independently. Test renamed names containing
spaces across C 8 and C 9, and byte-valued rewrites whose implementation prefix is
too short to admit a rewrite. This presentation contract does not establish private
worker installation or native compiler admission.

`NativeBinaryRootDispatch` selects the actual root protocol: C8.4/8.5 index
tables, modern C ensembles or the Jim distribution script. Index-table errors
consume the shared `OptionTable` renderer; `binary_root_index_error_code` retains
C8.4's `NONE` separately from C8.5's lookup code. This presentation policy grants
no worker registration or native compiler hook.

Binary handlers also retain their registered invocation layout: the public monolithic handler, a codec ensemble root and a private codec worker supply different header words. `NativeBinaryArgumentUsage` selects only the measured usage tail through `InvocationDialect::binary_argument_usage`; it never infers the handler layout from a private command's spelling. Apply `rewrite_argument_usage` to the actual invoked header before appending that tail. `argument_usage_header_style` selects raw words on older C/Jim or list words on C9; byte-valued runtimes use their existing list encoder without converting the header to Unicode. This preserves aliases and custom ensemble maps, including nested codec maps, independently of native compiler selection and normal-handler effects.

Character length has its own representation protocol: `string_length_representation` selects C8.4/8.5 conversion to string, C8.6's preservation of any byte-array representation, C9's pure proper byte-array shortcut, and Jim's cached string count. Modern C also returns an existing zero/one-byte string length without conversion. Preserve these representation effects independently of the returned integer. Jim indexed copying follows native leading-byte seeks, while length/reverse use validated decoding. Range results retain their prescribed native character count until another intrep conversion; a seek beyond owned bytes is a typed reached native-access refusal, never an invented byte.

Jim Binary reads retain the string representation. The selected UTF8 byte-read door returns the object's exact string bytes without installing a C Tcl byte-array representation. In particular, scanning a range result must preserve its native cached count even when that count differs from decoding its current bytes. C Tcl byte-array conversion remains a separate operation selected by its own protocol.

Jim's string equality and ordering are distinct native operations. Plain case-sensitive `string equal` compares bytes, while `string compare`, limited equality and nocase equality compare decoded numeric units with the retained native character counts. Its simple case tables cover BMP units only and do not expand characters using host Unicode casing. Case conversion stops at the native NUL and creates fresh encoded bytes. These contracts do not license changing expression equality, glob matching, or dictionary-key identity to the same algorithm.

Jim substring searches also retain their distinct native protocols. `first` compares exact needle bytes at decoded character starts, while `last` scans every byte inside its exclusive selected prefix, including continuation bytes. The resulting position counts decoded units preceding the matched byte. Cached character counts control native leading-byte seeks; access beyond owned bytes remains a host refusal. Repeat count conversion is separate from allocation: retain `prepare_repeat_count`, charge the actual byte extent multiplied by its nonnegative value, then call `repeat_with_count` without repeating coercion.

The selected Jim glob owner matches decoded numeric units and uses the same pinned simple uppercase table for nocase comparisons. It retains the native bracket grammar and the order of star continuations. The owned terminating NUL can participate in a native decode, so `?*` matches an empty subject even though `*?` does not. A subsequent decode outside that storage remains a reached host refusal. This operation does not use the existing byte-glob compatibility fallback or plain byte equality.

Jim trimming separates byte selection from object ownership. `RawString::jim084_trim_plan` uses the actual default set (space, TAB, LF, CR and NUL), decoded numeric-unit membership and the pinned backward start-byte scan. That scan can drop trailing continuation bytes even with an empty trim set. `ValueOps::jim_string_trim_result` applies the plan to the original physical object. Left/full trim checks the original subject getter before the character getter; right trim checks characters before the subject. A left cut creates a fresh NULL-primary working header. Full trim passes that SAME header to the right operation, which performs native string-intrep conversion before cutting. An unshared suffix cut preserves an existing cached count; a shared cut creates a fresh NULL-primary result with no cached count. An all-trim right operation creates a fresh empty NULL-primary object, including an originally empty subject. Concrete adapters selecting Jim must implement this hook; the ordinary byte construction default is for non-native fixture models.

Diagnostic argv must not change that sharing decision or force a string representation before the handler runs. The VM moves actual command words into one shared argument-vector owner and retains that same container in its evaluation frame. Actual error capture acquires the native argument-list references. Release completed frame argv after capture/settlement, while retaining it across an unfinished child activation. The byte-valued Runtime borrows the actual caller-owned argument handles through a scoped activation and materialises them only at capture. Never compensate for diagnostic copies by subtracting guessed reference counts.

Array enumeration uses exact key bytes through `array_key_bytes_checked_at`, and matched element removal uses `unset_elem_bytes_at` on the captured receiver. A byte-capable dictionary store must not convert those keys to Unicode or rebind a retired receiver by its spelling. Presence, size and patterned unset treat native dictionary decoding failures as non-array absence; this does not swallow a retained host refusal. Jim's exact `*` pattern destroys its dictionary root even when its contents cannot be decoded, while C removes members and retains the empty array. Traced hard read failures retain result, error code and error-info bytes independently through `ArrayReadFailure::new_bytes`.

Temporary values created by integer normalization need a transient hold in pointer-based adapters. Pin the normalized result before coercion, retain the coercion result, then unpin before propagating either success or error. Owning value models keep their ordinary handle lifetime. This applies to Jim's safe-expression repeat count as well as captured read-modify-write validation.

Binary operands use a separate object conversion protocol. Preserve real byte-array payloads independently of their lazily generated strings. A string conversion cache retains the selected `NativeBinaryByteConversion`; repinning cannot reuse a different conversion's cached bytes. Tcl9 encode/scan use checked Latin1 while format can narrow a copy of a wider string. `NativeBinaryDecodeSource` distinguishes a pure C8.6 byte array from its string form and the C9 proper-byte conversion attempt. Decoder errors consume `NativeBinaryDecodeInput`, retaining original Unicode versus byte payload and the actual byte offset. Select exact decoder switches before converting the data, and retain `UuDecodeError::Short` separately from invalid-input presentation. VM values retain arbitrary raw Jim string bytes. Byte-capable list, dictionary, binary and selected string paths use that byte owner; a Unicode-only consumer must check its projection and preserve a reached host refusal if the actual bytes cannot be represented. This remains distinct from pre-execution compiler admission.

The pinned Jim Binary root is scripted. Bootstrap `binary_scripted_ingress()` as a real procedure with its original formals and tailcall body; its lazy body has `UncompiledSource` provenance until a genuine compiler admits it. The canonical compound target remains `binary <operand>` after renaming the root. Registered compound callbacks have their own actual callable identity and no C worker semantic stamp. Abbreviations and replacement dispatch must follow this script's actual lookup, rather than the C ensemble resolver.

The standalone Runtime installs actual ensemble tokens and immutable worker
callbacks through `Interp::register_stock_ensemble` and
`register_stock_nested_ensemble`. Shared `EnsembleImplementationFamily` selects
the release floor; each handler supplies its own callback roster. Public and
private tokens, intermediate codec maps and selected worker identities remain
independent. Compiler prerequisites validate every live map edge and sealed
worker allocation; a changed worker is resolved through generic dispatch after
argument evaluation. Profile changes replace only unchanged engine-owned
generations and configurations, preserving custom workers and maps. Jim's
scripted root is a real Runtime procedure with separately owned compound helper
allocations. Those receipts admit the actual distribution helpers without
assigning a C primitive compiler identity. Preserve preexisting host helpers and
publish later helper allocation receipts through the real registration owner.
Those receipts grant callable admission only; they do not assert stock semantics.

The Runtime expands a live list through its actual element objects, retaining
them before releasing the source list. Do not stringify and reparse typed lists:
that changes numeric and binary representations and rejects valid lists holding
raw Jim bytes. Generic expansion uses the original native List getter selected
by the actual VM engine or retained Runtime artifact protocol. Its typed
`NativeListParse` failure carries that producer to the shared command-error
publisher, preserving native error-code and result-header behavior without
claiming a successful List conversion. Located literal expansions still use
their original element line receipts; dynamic expansion does not invent source
coordinates.

An ensemble's runtime implementation and its compiler registration/configuration are independent owners. A modified native map can make the runtime handler opaque while retaining the same native ensemble compiler. `NativeCommandCompiler` exports its actual configuration; `NativeEnsembleCompilerPrerequisite` retains the public token, namespace incarnation, map and selected worker's compiler capability. `SourceNamedInvocationProof::captured_name()` returns the compiled name without donating stock semantics to its late handler. An unmodelled delegated worker compiler remains a provider obligation.

Map construction has its own counted-input purpose:
`NativeNameProtocol::ensemble_map_prefix_input(namespace_report,
namespace_has_parent, original)`. The Runtime and VM adapters retain the actual
entered namespace token and obtain its report and parent presence from that
owner. Parent presence cannot be inferred from the report's spelling. The C8.5+
recipe preserves the complete original rooted head; it qualifies a relative
head while retaining its raw NUL suffix. C8.4 and Jim refuse this C construction
purpose. Do not apply publication or later command-lookup extent before storing
the prefix. Map readback, later call selection, live implementation and compiler
admission remain separate questions. The
[counted map observations](../analysis/name-resolution-proofs/ensemble-original-counted-map-prefix-construction.md)
retain all original vector fields and their distinct provider outcomes.

`compile_original_selected_worker` consumes the actual retained worker binding
and an `OriginalSelectedWorkerInvocation` containing the complete original
compiler words, selected operand offset, canonical member replacements, actual
compiler dialect and frame context. An execution trace veto preserves public
Generic dispatch. An absent compiler hook follows the release-selected fallback;
decline by a present compiler reaches that fallback independently. Unknown hook
or original geometry remains unavailable. A selected named compiler retains its
exact Direct or EnsembleRewrite layout; an operation compiler retains its
`NativeInstructionPlan` and ordered preparation.
`NativeCompilationSpec::select_registered_worker_native_words` applies the
installed worker grammar independently of the public head spelling. Neither
query proves public lookup/configuration or a late runtime handler. Retain those
independent prerequisites instead of resolving a worker from reporting bytes.
`configured_byte_worker_uses_its_installed_named_compiler` and
`original_selected_worker_trace_veto_and_dictionary_operation_are_distinct`
exercise these separate selection branches.

`compile_original_selected_worker_path` descends nested ensemble compilers
through the original selected map-prefix objects. Use
`original_ensemble_selector_at` for the exact flattened selector position; its
parser projection does not manufacture a handler or registration. Retain every
intermediate registration and configuration in `nested_compilers`, and validate
those prerequisites before operand evaluation with
`NativeCommandCompilerPrerequisite::matches_registration_with`. The lookup
uses each retained original namespace and invocation word, preserving unavailable
provider errors and rejecting a foreign interpreter or changed registration. The final leaf compiler binding
is separate from late runtime handler lookup. A dynamic nested member retains
public Generic dispatch; an unavailable original registration remains a provider
obligation. Compiler/preflight, VM and Runtime consume the same path recipe and
validate their own actual registration owners.

Dictionary lookup compilation uses `compile_native_dictionary_lookup` and
`NativeDictionaryCommand::select_original_lookup`. Check the actual selected
command's hook release before preparing operands. The retained instruction
orders the original dictionary, every key, and then the default when present;
its key count comes from the original parser projection. Literal expansion
members retain their original word and value spans. Dynamic expansion or a
native arity decline stays Generic; unavailable geometry/operations keep their
separate refusal. Selection does not parse a dictionary, invoke a getter or
evaluate a key/default. Compiler/preflight and the Runtime artifact consume the
same ordered operand recipe, while reached execution supplies genuine original
objects and binding authority. Preserve
`dictionary_lookup_preserves_original_key_order_and_parser_expansion` and
`dictionary_default_operand_is_evaluated_after_every_original_key` when changing
either producer.

Runtime dictionary scopes read the original variable-name objects through
`dictionary_scope_variable_read`. `DictionaryScopeRead` separates the initial
read from quiet update writeback and the creating `with` lookup. Each selected
C receiver stays live across its read callbacks; writeback reads that receiver's
current value and performs a fresh original-name lookup for the final store.
The epilogue saves and restores genuine private error headers at the native
update/with boundaries, so a quiet failed read preserves the body's completion.
Mapped variables and frozen `with` keys retain their original objects through
the corresponding getter and setter calls. Jim retains its separate variable
protocol and does not acquire C trace authority from this scope operation.

Procedure-header and ensemble choices use one `NativeCompilerSelectionSite` boundary with a purpose-typed `NativeCompilerSelectionPrerequisite`. Validate at its retained `guard()` boundary, hold the selection across argument evaluation, and replay the exact original command once when the premise fails. A function-wide handler guard cannot capture this operation. Check worker/compiler configuration before argument effects, while resolving the selected name's runtime handler afterward. Preserve constructed namespace keys by removing exactly one root marker; repeated trimming changes literal-colon namespace identities.

The `info commands` compiler is a useful mixed case: an absolute, compile-time-known trivial pattern selects command resolution followed by singleton-list construction; the general supported shape emits a private named invocation. The same subcommand description cannot authorize both as one opcode. `InfoCommandsResolve` emission requires the exact selected operation, independently of its editor description.

An empty variadic procedure is another compiler protocol, independent of procedure-body analysis. `NativeProcedureHeaderPrerequisite` retains actual interpreter, callable and namespace incarnations, header selection, original lookup word, and validation phase. A `NativeCompilerSelectionSite` validates once before the first operand and retains that selection through all operand substitutions. A changed premise replays the exact original command through plain dispatch. A function-wide opaque registry identity cannot express this protocol. Unknown body-object compilation state must not be guessed from whitespace text.

A lazy procedure first admits its body against the actual activation snapshot; provisional assembly retained while defining it is not a native cache admission. Subsequent entries keep native compiler-cache invalidation separate from command-reference lookup and runtime guards. C Tcl advances `compileEpoch` when a command with a non-null compiler hook is retired, renamed, hidden or exposed, and at the authored ensemble, execution-trace and fixed-function invalidation points. Namespaces also retain their own `resolverEpoch`: changing a command path advances it, and publishing a command that shadows a global counterpart with a compiler hook advances the affected namespace and qualified parent contexts. Namespace creation alone invalidates lookup references. Attaching `TclCompileNoOp` after ordinary procedure creation does not advance the native compiler epoch, so a warmed generic call remains generic until another genuine compiler invalidation. Recompiling on every lookup mutation would change that behaviour. Command-delete traces run before the native compiler epoch advances. An already active chunk keeps its admitted selection. Test generic-to-NoOp and NoOp-to-generic transitions, first activation, warmed bodies, reusable handles and callback reentry.

Raw command compiler attachments belong to each command token. An imported
command copies the origin attachment at import creation; it does not follow
later attachment changes on the origin. Installing a real procedure header
updates that exact origin token without advancing the compiler epoch. Capture
the raw attachment before retirement and use the shared
`native_compiler_cache_invalidated` policy at the actual mutation door. A
namespace resolver epoch belongs to its namespace incarnation, including a
retained activation; a recreated public spelling receives a new incarnation.

A successful native ensemble configuration update executes a setter transaction,
even when its new values equal the existing values. The shared
`native_ensemble::configuration_compiler_mutations` recipe accounts for two
compiler-affecting setters in Tcl 8.5 and three in Tcl 8.6–9.1. Apply each only
when the original configuration token has an actual compiler attachment.
Read-only queries and rejected option parsing execute no transaction. The first
execution trace and removal of the last execution trace use the same raw token
attachment; additional traces and rename/delete-only traces do not advance the
compiler counter.

Portable native operand projection uses `native_instruction_plan` with the
original complete word vector, retained selection, physical compiler dialect,
and compiler context. Its operand indices include the original command head.
The concrete producer supplies actual indexed-local allocation, literal
registration, opcode execution, and captured command prerequisites. A projected
recipe cannot create a compiler capability or a local-variable table. Compiler
admission and emission consume the same recipe; a backend declines a recipe it
does not execute.

An ensemble's selected worker compiler snapshot is an admission proof. Do not revalidate that worker's current compiler hook before each execution: attaching NoOp to a previously ordinary worker does not invalidate a warmed C Tcl chunk. Retain the admitted name/operation while its native cache stamp is valid. A public ensemble map or compiler-configuration change follows the engine's cache invalidation rules. Tests must distinguish changes before cached entry from changes during argument substitutions, and warm the body before testing hook attachment.

Use `registry_invocation::static_command_word` for a statically known original head's lookup value. It delegates to the shared word evaluator with the retained escape and word rules. For `se\x74`, that value is `set`; the source carrier still retains `se\x74` for exact replay. The returned value proves neither implementation identity nor compiler eligibility. `NativeCompilationWordShape::compiler_head` independently handles original shape and engine release: C Tcl 8.4/8.5 do not inline that escaped head, while 8.6+ can simplify it. Dynamic and expanded heads have no static value from this API. A compatibility emitter may retain its supplied name only when there is no applicable static source value; that fallback supplies no new proof.

Before semantic specialization or executable erasure, use `registry_invocation::proved_native_inline_operation`; it accepts only the retained singleton Must Inline execution selection and returns its operation and validation phase. Guarded bytecode emission instead uses `proved_native_admitted_inline_operation` and `native_operation_selection_plan` to preserve the admitted recipe through later runtime uncertainty. `proved_execution_target` identifies the handler; a Generic call can have a known handler while still requiring lookup after arguments and execution observers. A successful-handler or value query therefore cannot licence before-arguments opcodes. Bytecode lowering retains these as calls, and codegen also protects analysis-produced IR by recovering its original source carrier and emitting the selected generic or captured-name path. Deferred procedure inventory remains separate from executing its declaration. An unresolved compiler alternative produces a typed artifact refusal rather than public generic dispatch that loses a possible compiled operation.

Use `native_site_binding_requirement` when collecting or stamping structured-command opcode guards. A generic call must not acquire a live opcode guard from handler convergence: adding a NoOp compiler hook to its worker does not necessarily invalidate native cached generic code. Captured-name ensemble plans retain public compiler/configuration validation separately from the worker capabilities measured at admission. Test a warmed cache, then change the worker hook while keeping the public configuration stable.

Use `native_operation_selection_plan` to retain a proved inline operation across its argument effects. Handle all three outcomes: `Ok(Some(plan))` supplies the original source instance, replay bytes, namespace, selection phase and exact public/private lookup prerequisites; `Ok(None)` supplies no inline plan (including generic and independent procedure-header paths); `Err(reason)` means a proved operation lacks a safe carrier and requires a typed native-provider refusal. Do not turn that error into generic lookup or drop its dependencies.

`NativeOperationSelectionSite` marks the first instruction before argument evaluation and the end of that selected operation. Nested selections may begin at the same instruction. Coalesce reentrant emitter bridges only when the source allocation, exact native operation, selection phase and complete prerequisite set agree. The VM validates the selection once at its native phase, retains that selection through argument effects, and continues to validate unrelated live dependencies. Chunk-entry prerequisites and before-argument prerequisites remain distinct. A selected opcode has no retained public command token; do not synthesize an `EnteredCommandSite` to represent it. A private worker's compiler-admission identity is also distinct from its logical parent operation and any late captured invocation name.

Keep these ranges through peephole passes and validate them with `FunctionAsm::validate_native_compilation_entry`. Invalid, backward, missing-source or unguardable metadata produces a host admission refusal before source effects. Test replacement before entry, replacement during argv, overlapping nested ranges, cache reuse and malformed foreign metadata. Comparing relocated procedure artifacts may normalize diagnostic coordinates in the test, but production must preserve exact source coordinates.

For an inline script operand, use `proved_native_inline_body_context` together with the exact literal source carrier. Both source preflight and code generation delegate to `NativeCompilationSpec::body_context_for_invocation_operand`; this centralizes protected exception ranges and clause-dependent contexts. Scope and restore the returned context around that operand. A bare `try` must not acquire the protected context of a `try` with handlers, and a child must not inherit its parent's dispatch proof.

Each nested command gets its own proof. A parent `catch`, `try`, expression, or callback cannot donate its handler identity to a command inside its body. Inline emission scopes both the child token carrier and the exact body coordinate. Missing authored coordinates and derived scripts keep an explicit residual rather than a written-name fallback.

When rebuilding a token snapshot, use `restore_source_proofs` to transfer proof metadata while preserving the new snapshot's actual argument layout and synthetic marker. In particular, `foreach`'s preloop evaluated-argument boundary owns the input values. Its iteration bindings must not evaluate those original input substitutions again.

C Tcl 9.1's `uplevel` compiler illustrates why runtime argument values and original compiler words need separate queries. Only a procedure compiler with a known original first word can select `UPLEVEL`; earlier releases, script frames and a substituted first word use generic dispatch. Use `NativeCompilationSpec::uplevel_operands` for the exact optional-level/script layout. It delegates frame-word detection to the shared dialect-aware frame grammar, checks the native compiler's integer limits, and leaves frame existence for execution. Codegen evaluates every original script operand once, uses the shared concatenation operation for multiple fragments, and the VM enters the selected frame handler without another public lookup. Tests replace `uplevel` during argv and cover explicit/default levels, invalid levels, dynamic words and concatenation.

Selecting a foreign variable frame must remain suspendable. `Vm::select_execution_frame` moves the actual hidden `CallFrame`s and namespace tokens into `SelectedFrameRestore`; `EvalReq` and its execution activation own that restoration through coroutine parking. Restore those owners before caller continuation or ordinary frame retirement on error, return, refusal and coroutine deletion. Synchronous evaluation uses the same selection owner. Do not copy variable values or retire a hidden caller while a selected script can still access its aliases. Prepare the selected script with `prepare_script_commands`, retaining a malformed tail on the request so an earlier prefix, including a yield, executes before that error. `selected_frame_coroutines.rs` pins sixteen cases against C Tcl 8.6, 9.0 and 9.1: caller and alias writes, nested selections, multiple coroutines, namespace identity after deletion, abrupt completions, deletion cleanup, shifted error stacks and malformed tails after suspension.

## Route completion from the same frozen invocation

`exact_invocation_completion_words`, `invocation_completion_knowledge` and
`invocation_completion_route` resolve the structured words through the common
invocation owner. A known subcommand survives an unrelated dynamic operand;
unknown words are never replaced by an empty argument list to select a
completion descriptor. Exact rejection uses the selected signature count and
preserves unknown option layouts.

Use `invocation_completion_words` when a structural consumer needs the simpler
fall-through/current-procedure-result classification. Its literal compatibility
adapter is `invocation_completion`; both delegate return options to the same
native return parser as the full route. Dynamic option values remain unknown.
A pending successful return with level one can produce this procedure's normal
result; level two cannot. Consumers retaining catch, namespace, `uplevel` or
procedure boundaries use the full `ReturnCompletionRoute` and settle it at the
actual boundary rather than flattening every return into a normal result.

Keep each `InvocationFacts` projection coupled to its frozen argument carrier.
`frozen_argument_count` is the raw cardinality; `arity_argument_count` is the
signature's assessed count. Operand-role helpers check both accepted signature
and matching raw cardinality before projecting an index. A fact projection
computed for three arguments cannot be reused to describe five arguments.
Representation and successful-transfer consumers retain the selected handler
and its original frozen operands together through their typed proof wrapper.

For segmented analyser commands, build `CommandTokens::from_segmented` with the
actual source map and lexer grammar, then let `SourceCommandBindings::stamp_original_tokens`
attach its exact source proof. Resolve through `resolved_tokens_invocation` and
query `invocation_completion_words` through `with_argument_words`. Return-result
indices address effective argv, so map them through `EffectiveCommandWords::origins`
before selecting a written segment operand. A captured alias prefix has no editable
segment index. A compatibility argument spelling must never become a literal
option value or replace a missing implementation proof.

## Treat compiler failure as an entry event

`SourceCommandBindings::native_compilation_failure_at` is a must query: every reached entry must agree on the same rejection of that actual source chunk. The broader failure inventory includes possible failures and does not license an unconditional rejection.

The source IR and CFG retain the failure at the chunk boundary. Analysis terminates the chunk before its first body effect. Execution retains a neutral admission error and the native binding dependencies that selected it. Those dependencies exist even though the rejected command never reaches runtime dispatch.

Retained compiler rejection terminates traversal before executable source
emission. The admission error keeps reached original prefix registrations,
enclosing private worker compiler selections and the actual fixed math table as
chunk-entry prerequisites. Later source or transform dependencies outside that
rejected traversal cannot join its admission proof. An earlier unresolved
compiler visit withdraws presentation of a later error and retains a native
provider admission obligation; it cannot be replaced with a fabricated Tcl
error.

`codegen::native_failure` centralizes this transfer into `NativeCompilationError` and `CommandBindingIdentity`. Guard validation precedes error presentation. Tcl 8.4's chunk-entry dependencies must not be revalidated as though each native instruction performed late lookup. The actual procedure name belongs to runtime presentation, not the source cache.

`NativeCompilationError` contains proved result bytes, error code, and source contexts. The presenter owns quoting, source excerpts, compiler frames, and native limits. Native errors also write intrinsic global error storage, whose observers can mutate the command world before a surrounding catch resumes. Query the shared intrinsic storage subjects and resolve their actual write access; do not add error-global string matching to consumers.

A definite failure with unresolved presentation uses `NativeCompilationPreflight::UnpresentedDefiniteFailure`. It is a host admission obligation. Ordinary late command dispatch does not resolve it, and a guessed Tcl error would introduce catch-visible behavior. Use `validate_native_compilation_entry` and a genuine compiler provider before entering such an artifact. Low-level foreign artifact APIs must preserve this admission distinction.

Do not inline, relocate, or cache a rejected chunk as a normal body. A template owner either relocates its complete source/dependency/context proof or declines template reuse. Rewriting its statements without the boundary changes native behavior: Tcl 8.4 can reject unreachable syntax and can reject a procedure body before argument-count validation.

## Keep compiler traversal and successful transfer independent

`SuccessfulHandlerSpec` describes only the reached handler's normal effects. It lives separately from `NativeCompilationSpec`: changing compiler eligibility must not silently change variable roles or authorize a body traversal. Unknown frozen alias-prefix values retain their argv positions as `None`; a consumer may use the known positions while declining conclusions that require the unknown bytes. Collecting `Vec<Option<String>>` into `Option<Vec<String>>` is appropriate only when that consumer really requires every value.

Use `NormalRepresentationInvocation` for successful result/operand representation facts. Its evidence cannot establish that a call is effect-free, select an opcode, enumerate an evaluated body, or erase a possible error. Likewise, `PossibleVariableNameOperands` supplies possible variable-name positions for assistance and conservative inventory; it cannot create physical definitions or a successful variable transfer. Extend the registry descriptor for the missing purpose rather than broadening an unrelated proof query.

Name-access hazards consume retained handler candidates even when no single
normal handler is proved. Use `phased_operands` for exact captured positions and
`phased_unresolved_roles` for unresolved positions; each keeps its candidate's
destruction flag. Joining that flag before joining operands can mistake a
possible store for a destroy when candidates differ. `DynamicNameBarrier` uses
these projections without donating normal writes or closed command presence.
A computed head with an actual retained `array` handler supplies its name roles;
an unknown variable merely named `array` supplies none. Keep the unknown residual
in the source execution owner, independently of this enumerated hazard union.
Collect this May footprint even when strict execution facts are also available.
A compiler-selected private array worker cannot be re-queried by its slot
spelling to reconstruct the original public selector's variable roles; consume
the retained phased operands from the selected handler instead.

Unknown expansion cardinality retains possible name roles without proving
signature validity. A definite wrong argument count supplies no such role.
The parent `set` descriptor keeps read/write alternatives at its common first
name operand; an accepted frozen form replaces that union with its exact role.
`set fixed {*}$tail` retains the literal name, while `set {*}$names VALUE`
has an unknown name at the expansion boundary. Every role at or after an
unresolved expansion has an unknown argv value. This May projection cannot
authorize an opcode, a body entry, a definite variable definition or a normal
store.

For a strict retained invocation, consume its selected `facts.frame_effect`,
roles and traits through `with_argument_words`. That argument view retains the
actual dialect and captured prefix positions. A captured literal `$n` names a
variable called `$n`; it cannot be reparsed as a source substitution. An
unknown original variable-name operand still contributes its name hazard.
Lexical script scans without a retained call can supply a conservative hazard
inventory, but their catalogue lookup supplies no dispatch, normal effect,
body-entry or representation proof. Missing-package assistance separately
uses `ResolvedContext::resolve_spec` for its selected surface; a catalogue's
unfiltered `package` row does not establish that the document can require it.

An encoder's character data comes from `ByteArrayEffect::encoded_value_argument` through `NormalRepresentationInvocation::encoded_value_word`. Exact evaluated cardinality locates the final data object; an encoding name or option value cannot supply its binary provenance. Standalone calls still consume that data even when their encoded result is discarded. Check both assigned and discarded results, a binary encoding-name operand with ordinary data, and a replaced handler.

Nested native ensembles use `SuccessfulHandlerSpec::EnsemblePathLeaf` and `NativeHandlerLookupPaths::select`. The exact frozen selector chooses an ordered path; unknown selector values or expansion cardinality decline. For C8.6+ `binary encode hex`, the path first selects the original `::tcl::binary::encode` ensemble, then its original `::tcl::binary::encode::hex` worker. Resolve each intermediate slot as its own target and check its actual mapping and implementation identity before proceeding. A public token or terminal worker alone cannot prove the full path. `SuccessfulHandlerSpec::stock_native_workers` enumerates all eligible path edges for the explicitly trusted stock entry; `stock_native_implementation_slots` and private-registration prefix projection use that same inventory. Enumeration does not prove a live mapping or select a runtime path, and a terminal slot whose semantic argument layout cannot be rebased retains unknown facts. This normal-handler contract supplies no native compiler hook or opcode selection. SpecTcl rejects native path assertions explicitly, and Studio declares that field excluded from round-trip authoring.

Nested worker compilation is a separate `SubSubCommand::native_compilation` descriptor. `SubCommand::nested_native_compilation` selects only an exact frozen selector under the actual dialect; it does not inherit the outer compiler descriptor or rebase result, roles or normal effects. The pinned C8.6–9.1 Binary tables give encode-hex `Basic1Arg`, wrapped encoders no compiler hook, and decoder workers `Basic1Or2Arg`. Preserve those differences when registering private workers and selecting original public invocations. A live nested ensemble map and each intermediate implementation still require their own proof. Studio and SpecTcl explicitly exclude authoring these native worker assertions.

`NativeCompilationGrammar::WithImplementationPath` independently authors those
ordered compiler prerequisites. `implementation_prerequisites(dialect)` supplies
the eligible outer-to-inner lookup rows for bootstrap and live admission; missing
release evidence, a broken path, or an unsupported wrapped grammar declines.
Every live original map and worker identity must be proved separately. Wrapped
encoders retain both prerequisites even though their terminal hook is `NoHook`.
The wrapper supports named invocation and absent-hook grammars only; it cannot
grant body or opcode semantics. Native `Basic` hooks deliberately emit the
terminal private invocation for accepted argument counts. Compiler fallback
retains the outer ensemble invocation and its actual usage rewrite instead.

`native_compilation_for_registration` checks every original compiler-path edge.
An intermediate ensemble reverse-selects its separately authored registration
using the retained public selector prefix. A terminal worker selects its own
compiler descriptor directly. Neither query needs to rebase normal-handler
argument facts: a cumulative prefix that cannot be represented by that semantic
offset still returns no semantic facts. Missing, conflicting or unavailable
compiler descriptors keep registration unknown, even when the worker exists.
A uniquely authored normal-worker registration can supply its original logical
command and frozen member selectors independently of a compiler path. The
registration query selects that exact member's own compiler descriptor under
the actual dialect; it does not resolve a semantic invocation or manufacture
arguments to meet its arity. For example, `::tcl::namespace::origin` selects its
own `NamespaceOrigin` descriptor from the exact private worker mapping. A
replaced descriptor, unknown engine or conflicting mapping supplies no proof.

`NamespaceOrigin` owns C8.6+ `TclCompileNamespaceOriginCmd`: one original
unexpanded name operand emits `ORIGIN_CMD`, with the original ensemble/private
worker dependencies retained before argv. The operand evaluates once; the
opcode then resolves its command in the current activation namespace and
follows the live import chain through the existing runtime owner. Replacing
the private worker during the operand cannot replace that captured opcode;
replacing it before compilation withdraws the original recipe. Wrong argument
counts preserve the ensemble's normal invocation fallback, while Jim retains
its generic helper dispatch. The shared authenticated value bridge supplies
statement, substitution, return, catch and try-handler emission without
reclassifying the command name. This is neither a Basic late-private-name
invocation nor normal-handler purity or argument-conversion evidence. The
native origin mutation matrix and every-result-context emission test exercise
these distinctions. `NamespaceOrigin` hook/intrinsic names are exposed through
the SpecTcl loader/catalogue; native compiler descriptors retain the existing
runtime-only schema gap rather than becoming editable catalogue claims.

Original compiler policy is retained separately from the full lookup snapshot.
Every original compiler observation must agree on the logical word policy,
physical compiler and invocation realm; a missing axis stays missing. Different
variable or command worlds can withdraw a common lookup snapshot without
discarding these unanimous policies. Admitted emitters use this policy carrier
and the independent original implementation dependencies, while normal-handler
consumers still require the actual post-argument world. A policy conflict or
missing observation withdraws the compiler policy; body-template relocation
cannot recreate it from an authoring profile or later handler.

`NamespaceCode` owns the C8.6+ literal prefix compiler. Exactly one original
simple word, excluding an existing `::namespace inscope ` prefix, emits the
four-element list `::namespace inscope [namespace current] script`. The current
namespace is read at execution, preserving receiver-method namespaces. Computed
scripts and already-scoped scripts retain the actual ensemble rewrite to the
private worker; they do not acquire the literal builder. Original word shapes,
physical compiler availability and before-argv implementation dependencies are
required independently of normal `NamespaceCommandPrefix` result semantics.
The shared value bridge covers statement, substitution, return, catch and
try-handler contexts. Literal-builder and dynamic-fallback controls exercise
the same descriptor used by SpecTcl's hook and intrinsic catalogue.

The normal `namespace code` handler uses
`InvocationDialect::namespace_code_handler_policy`, independently of that
literal compiler grammar. Its native byte predicate follows the selected actual
engine: C Tcl 8.4 skips leading colons, requires more than 17 remaining bytes,
and recognizes `namespace` followed by zero or more ASCII spaces and
`inscope`, without requiring a trailing delimiter. C Tcl 8.5–9.1 recognizes
the exact `::namespace inscope ` prefix only when the argument exceeds
20 bytes. Jim recognizes that exact prefix including a 20-byte argument.
Do not replace these rules with list parsing, Unicode whitespace, resolved
command names, or a shared literal-compiler predicate.

Adapters return the original argument object when this predicate succeeds.
Otherwise they construct `::namespace inscope currentNamespace argument` as a
four-element list retaining the original argument as its last member. Keep its
exact bytes and object identity on both paths. Unknown or unaudited native
engine facts provide no native handler policy; consumers retain a host refusal
rather than selecting another release. VM and Runtime explicitly request
`LogicalNamespaceCodeProvider::Tcl84CoreSimulation` for the authored F5
iRules/iApps logical handler contract. The returned policy retains its logical
simulation origin, reuses the C Tcl 8.4 byte rule, and grants no actual-engine or
compiler proof. Without that provider, vendor compatibility projections select
no handler behavior.

A foreign-interpreter alias retains only its installed local wrapper token as an
opaque command. Its actual absent compiler-hook row can close original generic
compilation independently of its unknown forwarding target. Do not retain the
foreign target or prefix, infer parent-cell effects, or borrow registry handler
facts. A present or unknown hook remains an unknown compiler decision when no
other exact descriptor exists. The source mutation inventory must still retire
the original wrapper token when its local slot changes.


Trait walkers retain `TraitScanEnv::executed_source` when inspecting nested bodies. A literal child is checked against that source carrier; a materialized child uses the decoded carrier text and mapping. A missing mapping remains unknown. For `uplevel`, validate the selected engine's frame-level grammar before treating a dynamic word as a script or substituting a call-site argument.

Compilation walks syntax independently of runtime branch pruning. Use the checked expression owner with the complete `ExprParseContext`, and the registry's ordered compiler steps for nested expressions and array indices. Tcl 8.4 can reject a nested command in an expression branch that runtime never evaluates. Lexical/syntactic rejection, math-function lookup, argument traversal, and arity rejection have observable order; do not replace that order with a set of possible errors.

A compiler math-function lookup uses the actual `NativeMathFunctionTable`, including table closure, generation, implementation token, and trusted arity. `lookup` distinguishes absent, present, and unknown/conflicting entries. A registry function description is assistance, not evidence that a native function is installed. Cache keys and executable admission must retain the premises of a lookup failure as well as a successful lookup.

For `catch`, use `CatchInvocation::output_order` with the exact dialect and the selection proved for the consumer: immutable `native_compilation_admission_selection` for guarded opcode emission, and strict `native_compilation_selection` for normal-transfer analysis. Generic dispatch and Jim write the result before options; C Tcl 8.5's inline compiler does too, while C Tcl 8.6 and later inline compilation writes options first. Resolve each output name at its own write, after the preceding write's traces. Retain captured result/options objects independently of those names. Unknown ordering cannot authorize specialization or an ordered transfer. The typed `CapturedCatchOutputs` CFG boundary performs only these residual output stores; it must not reevaluate the source words or invoke the body again.

For a source edit, keep the owner projection and source slicing together:

```rust,ignore
let local = unit.cfg.statement_source_edit_span(block, index)?;
let authored = unit.abs_span(local); // handles a normalized cached unit
let written = source.get(authored.as_range())?;
// The transformation's identity, effect, completion and operand proofs still apply.
```

`SourceCommandBindings::selected_source` shares immutable retained inventory
collections. Analysis writes detach that backing before mutation; selecting a
procedure body must not clone the entire document's points, child scripts,
compiler recipes and deferred outcomes. The sharing/detachment regression keeps
exact source queries intact. Future global callbacks use the registry's
`BodyExecutionSpec::DeferredGlobalScript` contract and a separate inventory:
their unknown incoming contents cannot erase the registering command's normal
continuation or mark the callback as reached there.

Do not reuse a cached script's authored label as current-document evidence. `BodySourceProofs` restores the actual root, child-script, statement, and terminator source carriers with command/read proofs. Missing or conflicting mappings decline the artifact. The same bytes at the same numerical offset can belong to a different source instance.

Typed lowering can consume the original command token carrier. For example,
`AssignExpr` retains that carrier in its script's `command_binding_sites`, so
`statement.tokens().is_none()` does not establish that no actual variable read
was retained. Original IR consumers use
`Script::retained_source_tokens_for_statement`; it requires the statement to
belong to that exact script and checks the retained alternatives for conflict.
CFG consumers use `Function::source_tokens_at(block, index)` and the separate
captured-iteration input query. They must not borrow an original IR carrier by
matching a cloned statement's span. A synthetic iteration input cannot grant a
new invocation or another variable read. A conflicting retained carrier declines
even when the statement has a direct token vector. The shared ownership query
must arbitrate all retained alternatives before operation selection.

`CompilationUnit::build_with_analysis_input(source, options, entry, input)`
retains the complete caller input through lowering, the module and its function
units. Keep its ContextRegistry, lexer configuration and independently selected
source entry together. A supplied entry remains authoritative. Without an
explicit entry, `SourceAnalysisEntry::for_supplied_source` selects source advice
only when the input positively supplies the matching Logical or hosted policy.
Native, missing and contradictory inputs remain unknown; metadata availability
provides no Native execution entry.

A caller using `Lowerer` directly retains that input with
`with_resolved_analysis_input(input.clone())`. That builder shares the supplied
source-entry policy while preserving an explicitly selected entry. Retain the
complete input instead of copying its ContextRegistry alone. Use
`InvocationMetadataContext::for_source_input` or `for_module` to validate the
actual unit profile, complete lexer configuration and command generation at
metadata joins. Keep an actual Native entry separate. Constructed literal
bodies in global and caller-frame summaries retain the same source-advice
policy; their possible write footprint grants no entered frame, physical link
or store.

`FunctionUnit::build_full` supplies that checked metadata to
`build_ssa_with_context_for_entry_and_metadata`, retaining it across SSA rename
passes. Validate the complete source configuration before normalising nested
fragment settings such as BOM handling. Standalone SSA entrypoints keep their
explicit caller contract; they cannot recover missing actual input from a
Registry profile. Source metadata availability does not provide a Native
execution entry.

Synthetic optimisation modules retain all three ingress facts: the actual
lexer configuration, resolved dialect profile and registry snapshot. The
database's function memo takes them from its memo key; a top-level projection
copies them from the original module. Reconstructing a registry or grammar
from the display dialect can lose independent or custom axes and gives future
consumers different semantics from the module that produced their facts.

## Prepare expression functions using their native owner

`SourceMathInvocation` retains an original function occurrence with an explicit
purpose. Its `origin`, `site` and `function` are source metadata; consumers must
call `reached()` before using actual function identity, fold dependencies or
erasure proofs. That query validates the retained original metadata and rejects
conditional previews. `conditional()` returns checked original expression
topology only: its `frame()`, `namespace_context()`, `source()` and `arity()`
preserve the exact declared/evaluation context without a function registration,
native header, compiler entry, normal result or totality guarantee.

`SourceCommandBindings::math_invocations_for_script(registry, source)` includes
checked original conditional occurrences when the actual source walk did not
run. `implicit_math_invocations_for_script(source)` retains the recorded
inventory, including preview occurrences with Conditional purpose; its name
does not make those records reached. Preserve the purpose through CFG, SCCP,
loop summaries and function-unit queries. A conditional or invalid occurrence
keeps `requires_native_math_binding_validation()` true; native-integer and
common-AOT consumers retain their existing `MathBindingPrerequisiteRequired`
decline. Source topology cannot discharge that prerequisite from a function's
spelling, arity or an intrinsic evaluator. Conflicting actual observations
withdraw the conditional descriptor rather than replacing them with advice.

Readonly function-name presence uses the actual post-operand snapshots of
reached implicit-function observations. Every snapshot passes
`original_function_snapshot_is_closed`: matching selected name policy, closed
opaque domain, quiet source/command observers, callback-free lookup and current
ordinary literal-pool effects. `original_function_diagnostic_presence` joins
the genuine reached lookup-word bytes to the checked
`NativeExpressionFunctionCommandName`; an implicit function has no written
command head. Unknown from that exact function-purpose query is terminal.
Independently closed declaration previews are queried only when no reached
actual observations exist. The checked expression and constant operand topology remain mandatory;
this source presence supplies no written command head or Native Normal.
See the [function navigation contract](../analysis/name-resolution-proofs/original-function-navigation.md).

Modern Tcl resolves an implicit math command after its reached argument expressions. Query `ExpressionMathBindings::resolved_call(function, ast_start)` with the same source origin and expression base used to construct the AST. Namespace lookup, aliasing, rename, observer callbacks, and operand-induced changes apply to that implicit call too. The result identifies the actual implementation independently of an alias spelling. `proved_invocation` is the narrower same-function, zero-prefix projection. A stable `expr` command does not prove the math function is stock. Standalone mathematical evaluation uses an explicitly separate intrinsic policy; an execution fold with no binding query declines a reached function call.

Implementation-only analysis uses `resolved_call_for_value_analysis`. A known
native implementation may still convert an operand whose custom object callbacks
are unknown. The [semantic query contract](../contracts/resolved-semantic-queries.md)
and [owner inventory](../contracts/shared-utility-contracts-rust.md) define this
separation. Executable callers use `resolved_call`, which additionally requires
the source owner's `object_callback_effects_closed()` observation. Boolean
intrinsic adapters that remove a call use `proves_intrinsic_for_erasure`, rather
than `proves_intrinsic`. Neither gate discharges the expression's independent
preparation, operand representation, result-object or observer obligations.
Source rewrites, SCCP and static-loop evaluation use the resolved gate; O112
structure elimination uses the boolean erasure gate. Bytecode folding validates
effect closure when retaining consumed math dependencies. Missing closure remains
Unknown even if command/table identity, function arity and mathematical values
are known.

The fixed-table identity control
`fixed_math_name_and_arity_do_not_establish_native_implementation_or_owner`
keeps identity and erasure distinct. The actual-source control
`executable_math_requires_actual_closed_operand_effects` pairs literal and reached
local numeric inputs with an unknown input and a withdrawn effect observation.
Run the complete `optimiser::structure_elimination::tests` group alongside it to
cover O112 eligibility and refusal under the same operand-effect premises.

Fixed-function engines retain actual installed function rows. Function presence and arity are sufficient for compiler validation; stock implementation identity is a separate prerequisite for arithmetic folding. Opaque rows remain present with unknown arity or implementation. Successful bytecode folds retain reached-call dependencies as mutable command lookup guards or an actual interpreter-owned `NativeMathFunctionPrerequisite`. Whole-expression preparation is a separate obligation: fixed-table validation can inspect a function in a skipped lazy branch. Do not donate a reached-call proof from that branch, and do not discard the preparation prerequisite because no call executed. A fold whose result was rejected consumes neither ledger.

Diagnostic fixed-function presence is a separate source inventory. A closed fresh native C Tcl 8.4 or measured Jim table can provide stock-roster advice; an actual interpreter entry overrides that assumption with its own registration table. Unknown entry, opaque mutation and conflicting observations withdraw the advice. `diagnostic_math_function_presence_at` cannot manufacture a `NativeMathFunctionPrerequisite`, compiler registration or ordinary command QName.

Use `native_fold_dependency` to project an actual consumed call into an artifact guard. `SccpResult::required_math_invocations`, `StaticLoopSummary::required_math_invocations`, and `cfg::Function::required_math_invocations` carry consumed obligations separately from the passive source-call inventory. A transform that removes an expression must preserve its consumed ledger; emitters retain those guards atomically and refuse an artifact whose obligations they cannot represent. The original expression base and statement, terminator, or loop source owner must travel together. Reparsing rendered text cannot recreate the mapping.

Use `ExpressionMathBindings::with_preparations` and `preparation_for_context` to select the exact expression entry. `PreparedExpressionWitness::tree()` is the native term tree for the entire expression, not a replacement for any recursive subtree. Preserve `required_expression_preparations` alongside `required_math_invocations` in SCCP, static-loop summaries and CFG transforms; code generation validates the preparation context and retains its actual table owner atomically. Missing or conflicting preparation evidence declines execution folding. Reached preparation does not discharge C Tcl 8.4's separate compiler traversal of the original chunk.

Original conditional expression advice retains unchanged contiguous source and
its declaration, frame and namespace receipt. Handler-time evaluated or
materialised expression sources have an independent `EvaluatedSource` issuer:
they neither replace nor invalidate the `OriginalSource` analytical descriptor.
Only receipts with the same original-source issuer meet for this query; conflicts
withdraw the descriptor. Evaluated preparation, completion, actual compiler
admission and physical result/header evidence retain independent requirements.

Conditional expression advice uses a separate owner:
`ConditionalExpressionEvaluation::prepare` checks the exact original expression
under its retained `ExprParseContext`. `tree` supplies original operand geometry,
and `lexer_grammar` retains the selected grammar for conditional SSA May reads. `normal_result_representation` combines that tree with the original
operand advice and an independent `ConditionalExpressionPoolState`. Source
consumers select the stamped original invocation or declaration receipt before
calling this query. `TypeInfer` uses those receipts for result, `AssignExpr` and
`Return` advice; a conditional source template retains
`SourceRuntimeReachability::Conditional`, rather than claiming actual entry.
A fresh authored numeric-pool receipt is not stock object-class provenance:
getters, numeric command-name conversion and unknown callbacks withdraw it,
and joins meet it independently. Unknown functions, script results and
unsupported syntax retain unknown representation. None of these queries grant
normal completion, `PreparedExpressionWitness`, registered math/CPP authority,
original native headers, a rewrite or executable source admission. Keep those
independent proofs for physical consumers. Validate changes with the original
`native_conditional_expression_results` fixtures and preserve their unknown
pool, callback and opcode counterexamples.

`ConditionalExpressionEvaluation::context` preserves the original preparation
axes, and `numeric_reentry_is_idempotent` answers the selected inner normal-path
arithmetic law. `SourceInvocationBinding::nested_expression_normalisation`
additionally validates the original outer/inner source, checked trees and same
frame, namespace key and reachability before propagation or standalone
`expr_simplify` uses that law. It does not supply invocation, compiler entry,
normal totality, object headers or execution erasure.

Declaration lifecycle advice uses `SourceCommandBindings::scoped_lifecycle_advice`.
It requires immutable declared source/frame/key, unanimous original selected
provider layout, a live audited package version/surface/options/hooks, and
issuer-specific phase coverage. `ConditionalDispatch`,
`ConditionalPhaseDispatch` and `ConditionalNotEnteredPhase` are analytical
region dependencies; actual phase getters exclude these previews. A conditional
lifecycle template cannot stand in for an entered callback, physical body owner,
actual compiler prerequisite or proof that execution did not occur.

Evaluation has two purposes. `FoldEvaluation` can supply a numeric analysis value while retaining reached object-coercion and result-object obligations. Execution erasure additionally requires those native effects to be proved. `NativeOperandProofs` describes the actual object, incarnation and existing representation; a constant in an SSA contents map does not supply that evidence. Jim's selected-object result can preserve bytes such as `003` and sharing even when numeric analysis yields `3`. Preserve its result dependency instead of returning a newly formatted integer.

Source substitutions must call `substitute_expr_constants_for_execution` on the original AST with actual operand proofs. An empty proof set deliberately preserves a retained variable when evaluation would coerce its object. Substituting a literal first and then folding manufactures a fresh object and loses the original obligations. Keep mathematical helpers separate from execution transformations, and test both a foldable literal and a retained/shared object whose representation or returned bytes are observable afterward.

Diagnostic value queries use `FunctionUnit::semantic_values()` and its immutable `SemanticValueFacts` view. This view resolves chained producer contents through the same SSA solver as execution SCCP, while retaining each expression's `FoldEvaluation`, native preparation, reached function dependencies and coercion/result obligations. `contents((symbol, version))` answers a particular represented store, rather than a display-name search. `expression(ExpressionEvaluationPoint::Statement { block, index })` or `Branch { block }` exposes the corresponding producer obligations. A guarded unit can decline this view. The projection is lazy and its cache detaches when source or physical coordinates are relocated. Do not copy its known values into `SccpResult::values`: that map independently requires execution-erasure proof.

Construct the value policy with `FoldPolicy::for_retained_entry(registry, actual_invocation, lexer_config)`. The registry catalogue can differ from the execution engine; numeric/result and character-unit policies come from the retained invocation, while the exact lexical overlay remains independent. `InvocationDialect::characters` also preserves an embedding contract that differs from its underlying C release.

The `native_numeric` owner distinguishes source-produced numeric objects from
numeric contents and from live runtime object tokens. A closed native expression
preparation must additionally prove the executed numeric result recipe before
it can mint `SourceNativeNumericObject`. A mathematical constant arithmetic
result is insufficient: Tcl 8.5 and later can compile it to a pooled result
literal whose representation was changed by an earlier use. A numeric result
normalisation instruction or a reached numeric operation needs its own selected
native recipe; a pooled result needs actual current pool representation evidence.
Chained producers must retain those original reads and prove no new coercion or
selected-object result obligation; mathematical values alone cannot close them.
The source evaluator freezes that receipt before later argv effects; an actual
normal `set` retains it in the existing physical representation map only when
its store, contents origin and coercion epoch agree. An ordinary Value formal
binding can copy the same frozen numeric receipt into its actual incoming
activation slot; defaults, rest/reference slots, expansion and alias prefixes
do not acquire this proof. Plain `set x 4`, list
expansion values and pooled literal strings do not acquire numeric object proof.

`ResolveContext::contents_native_numeric_at` checks the live scalar cell,
generation, defined contents, store kind, observer freedom and actual native
axes. `SsaSourceView::read_expression_native_numeric` then supplies the exact
original read. `NativeOperandObjectIdentity::Source` retains the producer and
all distinct physical cells; it does not claim a unique runtime allocation or
absence of aliases. The prepared-expression owner checks the original operand
tree before withdrawing receipts; an earlier blanket expression coercion cannot
erase the evidence needed by that check. Non-numeric operations still withdraw
shared representations. For example, `expr {$alias in $alias}` changes a shared
integer object into a list in actual Tcl 8.6, 9.0 and 9.1, so a later read of the
other cell cannot retain its numeric receipt. SCCP and the expression rewrite
gate consume these receipts separately
from `SemanticValueFacts`; analysis values cannot manufacture them. Numeric body
proofs require exact source restoration and cannot pass a normalized template
cache merely because their rendered values agree. Numeric integration controls
cover pooled-result refusal, repeated normalisation and disjoint sharing
footprints independently of the stored-result protocol.

The selected native compiler policy lives in
`runtime_expr_validation::native_expression_constant_pooling(NativeExprSyntax)`.
C Tcl 8.4 emits runtime operator instructions; C Tcl 8.5 and later precompute
constant operator trees; Jim executes its original operator tree. Unknown
native syntax returns no policy. `PreparedExpressionWitness::numeric_result_recipe`
distinguishes numeric normalisation, executed numeric operators, reused pooled
values and unresolved protocols. Codegen may reproduce the original selected
instruction recipe; an analyser value cannot substitute a different recipe.
`native_expression_pooled_subtrees` inventories maximal constant compiler
subtrees in linear time so sharing footprints include a folded `3+4` result
`7`, alongside original literal spellings. This inventory supplies no runtime
object identity or representation proof. Verify the selected pooling policy,
original stored-result recipe and rewrite eligibility independently.

Stored and returned expression constants need their output protocol too.
`ConstValue` preserves contents, so strict SCCP cannot replace a retained
normalisation, numeric operation or pooled-result instruction with a raw literal.
Semantic values and consumed branch truth have separate purposes; faithful
codegen can retain the original selected native instruction recipe.
`expression_rewrite_equivalence` additionally compares the original and proposed
result recipes before licensing a positioned partial rewrite. An equal numeric
value alone cannot change pool reuse into operand normalisation.

An input receipt does not close output-object sharing. Replacing a numeric
operation by a selected operand or pooled literal can connect its result to an
existing object. In actual Tcl 8.6, `llength` of the replacement result can then
change the original operand's representation, while `llength` of the arithmetic
result leaves it numeric. `expression_rewrite_equivalence` therefore declines
that change with `NativeResultAllocation`; retained-input arithmetic rewrites
must preserve a numeric result operation and its exact numeric representation.
Source producer families still do not establish unique runtime allocations.

Conditional expression advice has a separate API:
`PassContext::report_prepared_expression_candidate` requires the original
preparation and records an empty replacement with `hint_only`. O110 identities
and O120 string comparisons may use it when conversion or output sharing is
unproved. It cannot licence edits or constant propagation. In the native C
8.6/9.0/9.1 counterexample, comparing a trimmed `007` with `"hello"` using `==`
changes the shared operand from pure String to Int; `eq` leaves it String, even
though both comparisons return false. A nonnumeric right operand proves value
equality, not conversion equivalence. Actual numeric-producer applied edits and
unknown/string-input refusal controls must remain paired.

The value-free O120 gate reads the original preparation through
`ExpressionMathBindings` and requires its exact original variable access and
successful current numeric representation. `equality_fallback_protocol` is a
selected native protocol: C preserves the already-numeric representation in
this closed fallback; Jim converts it to String and therefore declines the
edit. Only unchanged Eq/Ne operands with a fixed nonnumeric ASCII mate and the
same Boolean result protocol qualify. Source preparation and execution traces
must remain unobserved. Numeric-like literals, String producers, missing reads
and Jim cache effects retain the original operation. Mathematical equality or a
type label cannot construct this witness.

`SourceExpressionPreparation::original_variable_source(node)` retains the
actual `ExecutedExpressionSource` mapping used by the reached expression walker.
It projects an original variable occurrence to its full source identity and
site; a rendered offset or base-name match cannot replace it. The separate
`original_expression_mapping()` admits one unchanged contiguous written operand
only. `ExpressionMathBindings` selects derived parser coordinates only when the
statement's retained command binding identifies the exact parent invocation.
Numeric operands, already-numeric checks and increment proofs consume these
original read sites while compiler/math calls keep their actual parser source.
Multi-operand concatenation can retain individual reads without acquiring one
editable whole-expression extent. Missing sidecars for materialised sources and
conflicting observations decline. `executed_expression_source::tests` and
`expression_preparation::tests` cover original mappings and cache protocols.

Successful callable types likewise do not require a TclOO class allocation.
`ObjectHandleFacts::normal_procedure_result` queries an inferred normal type by
the actual retained implementation's full source allocation site. W307 consumes
that type independently from `returns_object` class candidates, so a loaded
provider's callable Object result need not invent a class name or physical
receiver. A same-name replacement cannot borrow another declaration's result.
`NormalRepresentationInvocation::callable_result_class_candidate` projects only
nominal factory assistance from the selected handler; it grants no created
command token, result bytes, lifetime, or method implementation.

List-value navigation provenance likewise uses the selected normal handler's
`NativeResultSelection::ListArguments` contract through
`NormalRepresentationInvocation::plain_literal_list_result`. It retains the
first original written operand only for plain literals and verifies the exact
result bytes frozen by the outer store. A command named `list`, an alias to a
procedure, or coincidentally equal result bytes cannot manufacture writable
provenance. A renamed native list handler keeps its original result contract.

Grouped variable stores require their own proof. `Script::statement_result_use` establishes whether normal completion is consumed, and `assess_grouped_store_rewrite` checks the actual replacement command slot. Neither closes output-address order, native partial-error text, observers or shared-object effects. O119 packing and its paired deletions remain hints while those obligations are unresolved; an empty result from `lassign` or `foreach` cannot replace a final `set` result.


End-offset index candidates require a separate schedule-equivalence owner. The shared `RepeatedReadContentsWitness` establishes exact closed singleton read continuity. O128 consumes it only for its bounded single-index known-list case, alongside the independent native handler/compiler and preparation proofs. Other candidates remain advisory: matching `$L` in `lindex $L [expr {[llength $L]-1}]` does not prove the two reads agree. A native read trace can change the second value, and removing that read changes both the result and callback count.

Resolve native index syntax through `tcl_cmd_core::index::resolve_opt_in`.
Its pure Jim integer-expression path uses the syntax-owned evaluator and native
wide arithmetic; it declines operands needing variables, scripts, strings or an
installed function table. Keep Jim's symbolic end encoding until the container
operation applies its bounds. Portable folders use `index_result_consensus` to
compare the actual selected element or clamped range across native targets.
For example, `end--1` has different C and Jim raw indices, but every measured
engine selects an empty `lindex` result and the same clamped `lrange` result.
An invalid index in any target still declines the portable fold.

List range output then uses the separate syntax-owned
`NativeListResultSerialization` selected by
`InvocationDialect::list_result_serialization`. C Tcl 8.4 renders a first
`#value` element as `#value`; C Tcl 8.5+ and Jim render `{#value}`. Actual
registry folds call `run_const_fold_in`, while a folder without actual axes
must decline that disagreement. `const_subst` and analyser static-command values
share this entry rather than reconstructing a release from display metadata.
The native renderer corpus contains 2,304 ASCII element comparisons. Registry
`range_folding_uses_actual_native_serialization_and_portable_agreement` and
compiler `range_folding_retains_the_actual_native_result_bytes` check selected
and portable rendering. Range bytes alone establish no object callback,
freshness, completion or erasure proof. Other list-producing operations require
their own selected rendering contract.

A closed index-rewrite proof must retain typed evidence for all of these requirements: the selected outer indexing operation and nested length/expression handlers; the exact two original `SourceSite` reads and their physical cell, contents generation and reaching value; observer-free ordered access; the selected list or character-index model; whole-expression preparation and command dependencies; equivalent native conversion, error and result behavior; and the exact editable source mapping. Missing evidence returns a typed decline, rather than a replacement licensed by a command spelling. Both reads must refer to the same live contents and shared value object without an intervening effect. Do not infer this from equal display names or equal numeric length facts.

`read_schedule::RepeatedReadContentsWitness::prove` is the shared contents
continuity query. It consumes exact retained read sites, requires closed
singleton physical alternatives with unchanged cell lifetime, reaching contents
origin and stored bytes, and retains one live
cell's known stored bytes and native dialect. Defined presence, lifetime,
and observer freedom are independent requirements. Initial internal representation
is not a contents-continuity premise. The witness does not prove omitted command purity, compiler
acceptance or representation equivalence; `end_offset` supplies those separate
premises and checks its replacement with the shared native index parser.

The initial executable case uses known valid list bytes stored in one unobserved live cell under an explicit native entry, two exact reads of the same represented store, proven stock handlers, and a bounded nonnegative literal offset. The retained native `llength` and `lindex` handlers both interpret that object as a list; the rewrite retains `lindex`'s conversion, including when the original object is numeric and another variable holds it. This schedule equivalence does not license replacing a general conversion with a literal. Pair each positive with a traced-read counterexample, changed/aliased contents, invalid list bytes, replaced handlers, unavailable native axes, and an unrepresented read. A string variant additionally needs the actual character model. General unknown-container advice must not acquire an executable proof from these bounded positives.

Before proposing a source expression replacement, call `expression_rewrite::expression_rewrite_equivalence` through the positioned pass adapter. The owner validates the original retained native preparation and AST, evaluates its real operand obligations, and prepares the proposed expression under the selected engine. Old math-call coordinates do not license functions introduced by a replacement. A known arithmetic result is insufficient when the original also coerces a shared object or returns retained bytes.

Formal-parameter consumers use `Module::parameter_grammar()` and the shared native formal parser and binder. Do not split the parameter list with a C-only helper. The CommonAOT direct tier accepts only a selected fixed positional layout it can implement; it explicitly declines defaults, rest slots, caller references and duplicate names sharing one native cell. Test `args x` and `&x` across C and Jim, rather than inferring those constructs from a catalogue name.

The selected registry `ArgTypeHint.shimmers` table and representation effect project into `InvocationFacts::operand_representation_coercions`. This is a reached native handler contract, independent of variable writes and result purity. Source invalidates shared numeric representation receipts after these operand conversions; `llength $alias` can change the representation of an object also stored in another cell. Expression conversions use their prepared-tree owner so lazy reachability and already numeric operands remain distinct. Unknown option or expansion layouts retain a coercion obligation. When every actual selected operand has known bytes, the physical owner may preserve a known container representation whose current bytes differ from every coerced operand: the objects cannot be identical at that moment. Unknown contents or observers retain the full sharing obligation. Exact current byte disjointness can preserve numeric receipts as well as container receipts; equal or unknown operand bytes withdraw them. Every possible coercion still withdraws pending frozen values through the representation stamp. Prepared-expression sharing footprints include both lazy branches and reject command/function/unknown operands; this footprint is not evidence that an unvisited read executes. Already-numeric variable operands can be excluded only under a numeric operand topology; list membership still invalidates matching numeric objects.

Frozen alias-prefix bytes establish argument contents, but not private object identity or effect-free numeric coercion. Native experiments show a shared list prefix can shimmer during invocation; Jim exposes the changed representation through `concat`. A prefixed math fold needs a reached object/representation/effect proof and a matching guard. Until that evidence exists, retain ordinary alias resolution and decline that fold. Zero-prefix aliases can still resolve their actual terminal math implementation.

Jim prepares the entire fixed-function expression before any substitutions. Use `prepare_fixed_function_expression` and execute its returned native tree. A conventional AST plus an eager validation pass is insufficient: Jim's term-stack parser can reassociate nested function arguments. For example, its accepted `abs(pow([mark]),2)` associates the second argument with `pow`. Native function existence and arity must be checked again on a warm cache when actual registration evidence changes. Retain parser, numeral, operator, escape, and native result-protocol axes independently.

The BPF frontend passes `Module::native_lexer_config` to its private CFG
lowering entry. Expression variable operands consume
`ExprNode::variable_reference` under that retained config. A scalar slot uses
the complete decoded name, including literal dollar, namespace or empty-name
bytes. An indexed reference declines the current scalar-only BPF subset;
`Var.name` is only a dependency label and cannot lend the root scalar slot to
an element read. The public CFG-only `lower_function` entry has no module
carrier and retains its supplied registry-profile compatibility grammar.

### Project argument roles at their retained source site

Build original `CommandTokens` with the retained lexer configuration and attach
its source binding before querying roles. `invocation_argument_role_assistance`
combines possible registry candidates, applicable catalogue assistance and
applicable document declarations. It maps roles through the effective argv
owner to original written arguments. Captured alias-prefix values and expanded
materialized elements have no editable original argument. Never recover their
positions by subtracting a prefix length, or query the document's final import
list after a source-site lookup has become unknown or changed.

This projection serves formal-list diagnostics and command-name navigation.
Its union grants no physical contents read, store, execution, completion or
optimisation permission. `ResolvedStatementInvocation::written_argument_roles`
uses the same mapping with the selected invocation's existing proof strength;
consumers that collect executed expression reads must still establish their
own actual read/body purpose. Syntax assistance is not that proof.

## Keep recursive source-driver construction out of retained frames

The command/body driver retains the original written arguments, source values,
representation receipts, variable read owner and compilation snapshot through
recursive descent. Keep owned preparations and dialect-bearing invocation views
in boxes constructed by nonrecursive leaf helpers. Moving a completed binding
branch also belongs in a leaf: unboxing a full state directly in the driver can
reserve a state-sized temporary in every retained recursive frame.

Pass the complete `SourceExecutionContext` by reference through script,
command, native-handler, selected-body, loop and body-operand descent. Keep the
owner alive until that descent returns. A changed chunk snapshot, selected
compiler, namespace/frame entry, evaluated-body realm or loop depth gets a new
complete boxed context from a nonrecursive helper. Its struct update retains
all other fields. Borrowing a context must not drop a read owner, evaluated
expression source or original argument receipt to shorten the frame. Cold
preparation adapters accept that reference too and copy any required value
inside their own frame, so the recursive caller does not reserve the outgoing
context value. The box describes analysis storage only; it establishes no
native object alias, frame lifetime or successful body entry.

Cold handler, definition and lifecycle construction must finish before ordinary
body descent. Capture the physical coercion receipt before the handler, then
unpack it in a leaf after the handler returns, preserving the existing result
publication order. Receiver dispatch, live lookup, empty-procedure result
construction and trailing named or residual compiler branches also keep their
construction in separate helpers. The inline body path must not retain stack
slots for those alternatives. Preserve the existing receiver priority and the
live, inline, named, unknown and compilation-error join order.

Boxed analysis storage preserves native admission, source identity, observer
order and execution depth. Verify the default-stack and
original constrained-stack tests; raising a depth budget or thread stack does
not establish this contract.

Retain every execution axis when preparing a boxed view: actual realm and
native dialect, compilation context and snapshot, namespace and physical frame,
original written argv and values, representation/object receipts, and the
variable read owner. A box is storage for that evidence, not evidence that an
unknown axis has become known. For example, unknown handler facts retain the
original completion route and conservative state mutation; an independently
proved malformed argv still returns its original Tcl Error without entering a
body. If argument substitution or an enter observer has no normal successor,
return its existing abrupt outcomes and omit handler dispatch. A boxed early
completion must carry those outcomes unchanged, rather than manufacture a
successful continuation or lose a retained source identity.

## Preserve host obligations across executable backends

`NativeCompilationAdmission` retains the original evaluated chunk, definite failure if proved, and any unresolved provider obligation. Its `plan` distinguishes a whole host script, a host procedure admitted before formal binding, and missing-source refusal. Native/direct execution must decline before effects when it would bypass that admission. A WASM host fallback evaluates the whole original chunk before procedure-table installation; per-command fallback cannot discharge a chunk compiler obligation.

`NativeCompilationPreflight::ProviderRequired` is distinct from `UnpresentedDefiniteFailure`. Neither is a guest Tcl error. Resolve the obligation through a genuine native provider, or preserve the typed host refusal before execution starts.

`CompileError::Unsupported` carries an unavailable compile-service capability through `NativeExecutionError::CompileServiceRefusal`; it is not a source validation error. `CompileError::Message` retains the source-rejection channel even when its diagnostic bytes happen to equal a host refusal's display text. The VM and engine procedure-definition adapters preserve this distinction too.

`NativeExecutionError` likewise separates compiler admission from a reached expression refusal. Unsupported expression syntax or native error presentation must not become a made-up catchable completion. The retained refusal payload identifies the actual source, profiles, interpreter, namespace, and frame. It is diagnostic and non-resumable: cleanup retires the suspended guest activation without running guessed traces or finally bodies. An implementer adding resumable provider delegation needs an explicit continuation protocol and new tests, rather than retrying those bytes in another interpreter.

Generated code carries this distinction through the shared code-generation ABI.
`CodegenAbiImportId::requires_host_refusal_check` owns the policy: operation
imports require a check, and an explicitly classified transport or cleanup
import can omit it. A new import defaults to requiring a check. Query
`HostRefusalPending` immediately after a reached operation, before adopting its
completion out slots, reading typed scratch outputs, dispatching a Tcl completion
or choosing a fallback. A pending refusal branches to ownership cleanup and
unwinds the guest activation. It must not execute guest `finally` bodies, publish
result/options variables or retry the source after earlier effects.

Native procedure entries return three distinct statuses:
`NATIVE_PROC_STATUS_RAN` (0) supplies an ordinary Tcl completion;
`NATIVE_PROC_STATUS_DECLINED` (1) declines before effects and can select the
original source path; `NATIVE_PROC_STATUS_HOST_REFUSED` (2) means an operation
was reached and retained a host-only refusal. Status 2 leaves the caller's
completion out record untouched and cannot select source replay. The refusal
query neither clears the retained payload nor manufactures a Tcl result. A
script entry has no status result; its caller must inspect the same retained
host channel before treating the entry as an ordinary completed evaluation.

Byte carriers obey the same contract. Use `ValueOps::as_bytes` and exact-byte
constructors for transport, and the checked Unicode projection only at an
operation that actually requires Unicode. Preserve `CmdError` message bytes
and its typed host tag separately. A failed projection is not a catchable
synthetic error named after the Rust conversion failure. Never apply a lossy
decoder to an executable value, command name, script or error result. Presentation
can escape bytes, but that escaped text cannot re-enter execution as their value.

`NativeValueAccessRefusal` keeps Unicode projection and native string access
failures distinct. A Jim seek or copy may use the actual owned terminating NUL;
it cannot read unrelated storage beyond that allocation. Retain the typed
`NativeStringAccessError` instead of fabricating a guest error or byte. Jim range
results also carry the native prescribed character count through
`ValueOps::new_jim_string`. That count belongs to the actual string internal
representation: list, dictionary and numeric conversion withdraw it, and a
newly reconstructed string counts its own bytes independently.

Array enumeration retains exact key bytes through
`VarStore::array_key_bytes_checked_at` and selects each physical read through
`array_read_elem_bytes_at`. The Unicode-only default checks its key projection;
byte-capable adapters override the door using their existing captured receiver.
Never render a key to Unicode and then treat the rendered spelling as its cell.

The generated refusal regression in `tests/wasm_codegen.rs` executes a native
procedure under Wasmtime with both ordinary and refusing host controls. It
checks a prior effect occurs once, the transient frame is released once, the
completion record remains untouched on refusal. Separate real-runtime
regressions require `catch`/`try` to bypass the retained host refusal,
without replaying earlier effects or executing later writes. This complements actual C/Jim differential
tests; an injected host refusal is evidence about the adapter protocol, not a
claim about a native interpreter's guest behaviour.

General lowering owns result/options handles directly in its statement cleanup slots; it releases those objects rather than a completion wrapper. Semantic lowering adopts and releases the completion wrapper only after the host-refusal query succeeds. The General and Semantic Wasmtime controls check these distinct ownership paths with ordinary and refusing host results.

## Resolve storage and values separately

`ResolveContext::literal_contents_alternatives_at` supplies bounded exact texts
from one current physical receiver for operand-layout consensus. Its closed
case retains the typed cell, lifetime and index plus at most eight distinct
strings and 8192 total bytes. Both incoming branches must supply known values
from that same receiver; missing contents, opaque writes, read observers,
retirement and overflow retain an explicit unknown residual. Joins, selected
frame restoration, raw-slot capture and alpha relocation preserve this proof.
The singleton literal query and SSA constant queries keep their original
contracts. Frozen words retain their original argument cardinality; a registry
layout can use these alternatives only when every combination agrees on the
same purpose-specific positions. Do not select a value or command target from
one alternative.

Closed presence joins use the selected typed slot recorded by the presence
owner to query an untouched predecessor. A defined/undefined union may justify
a potential missing-read diagnostic, while definite presence, completion and
value-version queries still decline. Physical identity labels are storage
encodings, not evidence from which to reconstruct an implicit local slot.

Use physical cell identities with interpreter/activation/worker ownership, allocation generation, lifetime, and presence. A variable name is a lookup input; it is not a storage identity. Content writes, destruction, recreation, and alias retargeting are different transitions.

`NamespaceCellPresence::join` intersects definite allocations and retains every
possible allocation. A definite cell lost at an alternative becomes possible;
a cell definite on both alternatives remains definite. Joining identical facts
preserves their representation. The returned boolean reports a real change to
the allocation facts, including whether the namespace surface remains closed.

`upvar` and reference formals need the selected frame-level grammar and actual
reached caller frame. C Tcl links and Jim name-based wrappers have separate
lifetimes. Jim static capture keeps its actual mutable cell or original link
wrapper before formal writes. Recursive wrapper lookup retains frame-storage
correspondence separately from activation and variable-cache epochs.
`JimCallFrameStoragePool` permits LIFO reuse only after the selected backend
releases the actual frame owners; current activation identity remains fresh.
Live slots, foreign pools and expired storage cannot match a recycled slot.
A copied retired frame supplies no original storage correspondence. The
[six native source controls](../analysis/name-resolution-proofs/procedure-static-jim-native-link-frame-reuse-boundaries.md)
and [storage-owner contract](../analysis/name-resolution-proofs/procedure-static-jim-original-link-frame-storage.md)
retain their separate scopes.

Construct `SsaSourceView::at_statement` or `at_terminator` at the actual consumer
point. Preserve `SourceSite` for `read_reference`, the original `WordExpr` for
`read_word`, or the original AST variable and parser base for an expression read.
`WordExpr` and `WordPart` preserve each reference's verbatim source spelling:
`$x` stays `$x`, while `${x}` stays `${x}`. The separately retained compatibility
`argv_texts` may canonicalise the first spelling; it must not be copied back
into a structured reference. A captured access matches both its original
`SourceSite` and original spelling. Equal names or equivalent Tcl values cannot
repair a mismatched receipt, and two entries at one source site remain
unresolved. This also preserves whether an index is substituted, as in
`$a($i)`, or taken literally, as in `${a($i)}`. Braces do not prove scalar
storage: all tested C Tcl releases and Jim read an array/dictionary element
through `${a(k)}`. Jim additionally permits a formal named `a(k)` and installs
its argument into local `a` as dictionary entry `k`; both reference spellings
read that entry. C Tcl rejects that formal at definition. A successful read
alone does not prove a whole-name scalar slot or lookup precedence. Physical
lookup must retain the actual native family, frame and name-object protocol
independently of source syntax. Quoted sole references use the individual reference
site; compound and braced data words have no sole read identity.

An evaluated index has its own ordered value phase. `SourceVariableAccess`
retains the selected typed address in each actual read context after index
parts finish and before the outer read's observers. For
`$a($k[set k new])`, the first part's value is frozen before the second part
changes `k`; a later lookup of `k` cannot reconstruct the selected key.
Variable, command and decoded text parts use their actual completed values;
an unknown or failing part retains uncertainty or its abrupt continuation.
Joins withdraw conflicting addresses within one context, while different
physical activations keep their own receipts. Relocation maps both the context
and the selected cell. SSA address, contents, representation, numeric and
presence projections share this carrier instead of reparsing the index.

When changing the word producer, test the source-spelling contract in
`ir::tests::structured_variable_references_preserve_original_syntax` and the
physical consumer contract in
`ssa::cell_resolution_tests::word_read_requires_original_spelling_as_well_as_site`.
The latter pairs a successful current read with a different spelling at the
same site and requires every read, contents and representation query to decline
the replacement. Then exercise nested command recovery, expression reads,
representation costs, copy-on-write and byte-array consumers: they share this
producer, so a local parser test does not verify those consumers. Native
lowering and WASM planning both use the common `variable_word_place` projection
and retain separate execution-admission obligations.
Pass the actual retained `LexerConfig` to that projection; it delegates whole
reference recognition and span validation to `scan_var_ref` and
`RawVarRef::source_span`. It accepts the native empty-name reference and uses
the C8/C9 brace-closing rule selected at ingress. A malformed, compound or
source-mismatched reference declines. Both `$a(k)` and `${a(k)}` select one
symbolic `CellPlace::Element`; their common shadow must be replaced by either
store spelling. `${a($i)}` has the literal key `$i`, while `$a($i)` needs
reached index evaluation and cannot borrow that static key. Physical native
lookup and trace handling still decide whether the storage is a C array or a
Jim dictionary. Verify the native-shadow, literal/dynamic key and release-close
controls together with the Runtime named-read integration tests.

`read_spelling` is a compatibility consensus projection when an older evaluator
has already lost those sites; conflicting references abstain. Do not replace
independent earlier/later reads with that consensus, or use `reaching_binding`
to invent a lexical reference. Frozen evaluated arguments can establish their
own values without re-reading a name after a later argument mutates it.

Use `SsaSourceView::externally_mutable_by` to test explicit named observer or
alias dependencies at the store's actual point. It matches the symbol's
canonical cell key or a source spelling that selects that symbol there;
resolve each dependency through the retained point environment because an
alias absent from this statement's sparse operand inventory can still select
the cell.
`SsaFunction::var_name` is a diagnostic label and cannot identify mutable
storage. A later retargeted alias must not contaminate an earlier unrelated
store merely because both symbols acquired that display label. Nonempty
dependencies with missing point bindings remain unknown, while a dynamic
trace independently covers every cell. SCCP and type inference consume this
same projection.

Synthetic whole-array refreshes use
`SsaFunction::is_array_root_refresh_version(symbol, version)`. This follows the
typed physical array may-def producer and aggregate-only phi propagation;
parentheses in a source spelling do not establish an array root. Ordinary
scalar stores, including Jim dictionary-backed arrays and a scalar recreated
after an unset, remain separate from those synthetic refresh versions.

For arrays, retain a bounded root when the index is dynamic. An unknown index can overlap every element of that root without clobbering every variable. `SsaReadContents` retains possible represented stores, incoming contents, and an explicit unknown residual. Join values from actual overlapping stores; do not identify a value by the written array name alone.

Incoming formal contents do not need a fabricated scalar SSA definition. `SsaSourceView::read_expression_incoming_slot` returns a `SsaIncomingSlotRead` for the exact source read and all actual cell identities represented by that logical formal. It rejects nonlocal or static ownership, observers, writes and unresolved residuals. `NativeOperandIdentity::IncomingSlot` lets bounded analysis join distinct caller activations while retaining those physical alternatives. Typed WASM emission declines that identity until a separate frame-materialisation and object-coercion contract proves how to consume it. Never substitute a display-name match or version zero for that missing proof.

`SsaStatement::destruction_defs` records contents invalidation, not successful
value publication. Its new versions have Unknown contents origin rather than
`WrittenAt`, and grant no object representation or store provenance. Conditional
destruction also belongs to `may_defs` and retains its predecessor use. Preserve
an existing `UseClass::Substituted` read; only a newly inserted predecessor
needed solely for existence or destruction receives `UseClass::Name`, without
a value operand to rewrite.
Taint tombstones retain predecessor influence rather than treating destruction
as a clean successful assignment. Keep def/use, type, representation and
dead-store consumers consistent with those separate purposes.

`read_completion_at` and `read_contents_presence_at` project the exact retained read across all closed physical alternatives without requiring a contents version. A definite missing-read proof requires `ContentsPresence::Undefined`: an existing array root may have `VariableReadCompletion::Error` while its presence remains `Defined`. An exact SSA value version also requires `Defined` contents and no proved scalar/array-kind rejection; a default `Incoming` origin for an uninitialised slot cannot grant version zero. Unknown observers, residuals or conflicting alternatives preserve uncertainty.

`read_contents_presence_alternatives_at` returns a purpose-only
`SsaReadPresenceAlternatives` when every original physical continuation has known
presence and no read observer. Its `may_be_undefined` query can identify a
potential missing-read diagnostic across mixed Defined/Undefined continuations.
The contents owner retains that closed union in its existing presence lattice;
`ResolveContext::closed_contents_presence` exposes it only for this purpose,
while `contents_presence` projects the union to `Unknown` for Must consumers.
It does not strengthen `read_completion_at`, supply an SSA version, or establish
that an invocation must fail. Unknown/residual continuations decline the witness;
a scalar read of a defined array is a kind error, not missing contents.
An actual unobserved substitution whose `read_produces_value` witness closes all
read failures has only a normal completion, even when its bytes remain unknown.
Do not add arbitrary Return, Continue or Break alternatives solely because the
value is not a literal. Observed and unresolved reads retain their completion
uncertainty independently.
A closed zero-iteration or no-output continuation cannot be suppressed by a
legacy loop-entry SSA assumption. For `foreach x $items {set y $x}; puts $y`,
unknown iterable bytes retain the possible empty path; initializing `y` first
keeps it defined on that path. A conditional scan target likewise supplies
navigation and a possible store independently of definite contents presence.
A logical incoming argument contents seed may populate only the semantic expression view after `read_expression_incoming_slot` proves each occurrence; it does not certify native object representation or permit producer erasure.

Caller seeding uses the shared formal parser and `bind_formal_arguments` plan under `Module::parameter_grammar()`. A proved zero-argument call can supply C defaults; an opaque caller cannot. Jim references, renamed rest parameters and retained static fallback need their actual activation binding provenance before a logical seed can describe a physical slot. Test known defaults beside opaque callers, and two different activations beside an alias retarget to a global cell.

Loop conditions retain their original command owner in `Function::condition_binding_sites`. This is source evidence for repeated condition reads, not a synthetic runtime command boundary. Keep it distinct from `command_boundary_sites`, which owns replay and guard timing. Freeze a structured command's owner before lowering its initializer or child bodies; those children can replace the current block's owner.

Presence and contents are separate. A formal can be present with unknown contents. `info exists` depends on native lookup/read behavior and observers; it cannot infer absence from a missing literal value. Modern Tcl ensemble specialisations additionally retain their selected private implementation identity and the native validation boundary; a captured invocation name remains distinct from a captured token. An unchanged public `info` command alone does not establish the worker implementation.

`NativeResultContract::IncrementStore` separately authors the numeric shape produced at a successful captured Increment store. `normal_numeric_store_production` requires exact retained cardinality and the actual arithmetic protocol. Publish that shape into the captured cell before write callbacks; callbacks and representation epochs can withdraw it. This excludes simultaneous ordinary container representation at an exact preparation point, but proves no concrete number, frozen object, foldable result or post-callback representation. The general result-selection query remains Unknown.

A conditional range-result recipe can use a still-current frozen original
operand, or an exact current successful variable read when every later original
argv word is a literal without substitutions. The latter proves the current
physical input at dispatch; it does not revive an expired frozen epoch receipt.
The selected native list grammar, actual bytes and nonempty range recipe remain
required. Unknown later argv effects, callbacks or an unproved input kind decline
this current-read alternative.

`NativeResultSelection::ordinary_range_literal_result` can additionally retain
exact result bytes through the selected native list-object renderer. Capture
those bytes beside the independent
range-shape proof before coercion, then publish them only on the reached normal
result. Full and clamped ranges still canonicalize spacing; they cannot borrow
the original input string. Quoting, escapes, leading hash and non-ASCII output
follow the actual engine's renderer; C 8.4's leading-hash policy remains distinct.
Unknown or empty selections and byte output without a Unicode projection decline
this byte recipe without removing an independently proved List shape. The heap-backed source receipt supplies
neither object identity nor permission to erase the producer.

`InvocationFacts::stock_list_length_protocol` is a separate normal cache and
completion recipe. The caller proves the original stock object, closed hooks,
current successful read and actual native Length handler before consuming it.
`normal_cache_disposition` accepts independently current String/List/Dict/
ByteArray/Numeric/Boolean cache evidence; it never derives that class from bytes.
C9 numeric and word-Boolean Length methods preserve their cache. Independently
proved Numeric inputs on C8.5/8.6 become List even with unknown string bytes:
stock numeric objects cannot use the canonical empty-string shortcut. Other
C85+ empty preservation remains Unknown unless the original class itself closes
the branch.
StockUnknown on C9 remains Unknown even with nonempty bytes. A Preserved result
can restore only a current class summary under the same physical-origin/read
proof; concrete values and frozen epochs stay retired. The conditional OK/Error
completion bound is independent of this cache disposition and of observers.

`normal_empty_list_root_provider` selects only the reached Generic zero-argument
native List result. Its `EmptyListRoot` receipt closes root list-method effects,
including conversion of the native empty String, without providing StringAccess,
strict List representation, allocation freshness or representation preservation.
Compiled pooled empty objects require their separate history proof. A source
join with ordinary List must retain this root-method-only purpose and cannot
promote custom members to general stock-string closure. The 204 same-object
Length observations and ten empty-result reuse contrasts are maintained in
`rust/tcl-syntax/tests/data/native_list_methods/stock_length/`.

`InvocationFacts::normal_numeric_result_production` separately selects the actual ListLength handler's integer result. Both the compiler's LIST_LENGTH instruction and the generic handler create a normalized numeric result, including literal list operands. The source normal-result receipt retains that shape independently of unknown result bytes, then publishes it at an exact captured Set store before callbacks. An ordinary Value formal can copy the actual frozen receipt into its own Incoming activation slot; default, rest, reference and foreign/static bindings cannot borrow it. A literal count such as `set count 2` does not supply this producer. Producer effects, C9 object-method residuals and later coercion remain independent obligations.

`PreparedExpressionWitness::normal_numeric_result_production` privately mints
an actual runtime arithmetic/bitwise descriptor. Publish Expression numeric shape
only on its reached normal successor; concrete numeric object publication also
requires separate operand/value evidence. Boolean, literal and constant-pooled
results cannot borrow this descriptor. Representation is independent of result
bytes, operand effects, object identity and erasure permission; arithmetic,
Boolean and callback-withdrawal controls exercise those distinctions.

`NativeExpressionNumericResultProduction::normalises_result_string` is a
separate recipe property for audited native setters/new-object paths. Use it with
`number::canonical_numeric_bytes_may_equal` only to exclude sharing with literal
pool objects whose bytes lie outside the native numeric output language. The
predicate is conservative and does not parse or provide a value. Unary operand
forwarding and exponentiation shortcuts do not supply this property. Source
numeric shapes retain the private production, so disjointness may preserve the
current shape while still advancing the sharing epoch and retiring frozen operand
proofs. Unknown or possibly overlapping bytes withdraw the shape. The native
420-row result-string corpus and formatter/recipe/source-pool controls cover
normalisation and sharing independently.

`native_numeric_conversion` owns conditional original-operand cache recipes,
separate from primitive Wide extraction and arithmetic result construction.
`PreparedExpressionWitness::integer_relational_operand_conversion` selects only
the actual C85+ prepared `<` numeric branch. Source ingress proves both original
inputs select integer numeric conversion, retaining the actual physical read,
original source mapping, closed string-update/free effects and pre-getter cache
class. A normal string comparison cannot supply this proof. C84 and Jim abstain
from this initial recipe. Unknown stock cache publishes only Numeric; a sealed
original Integer, String or ordinary List cache can refine it to Int, while a
cached Double remains Double. `with_original_cache_class` accepts native cache
class evidence, never a semantic type or integer-looking text alone.

`InvocationFacts::integer_index_operand_conversion` instead selects a reached
scalar Integer index branch under the actual native compiler selection and
original word shape. Runtime lrange indices can publish Int on normal success
with independently proved integer-only contents and closed effects. The initial
lindex recipe additionally requires an actual Integer cache on the original
index object: a grouped List or cached Double can instead convert an extracted
child. Immediate Inline literal indices, unknown compilation and unresolved
operand mapping decline; they do not donate a getter on a variable.

The private `numeric_operand_cache` adapter captures these inputs before native
coercion and publishes only on the corresponding normal continuation, with the
same physical cell, contents origin and full source attestation still live.
Read observers, unknown callbacks, retargeting and missing paths withdraw the
receipt. It installs only a current numeric shape. The original string remains
unchanged, and concrete numeric objects, values, frozen epochs and erasure
permissions remain absent. SSA/shimmer consume the current category through the
common read projection; O114 separately requires integer-conversion acceptance
and its read/store schedule. Codegen and Runtime must execute their own selected
getters and preserve error/cache behavior, rather than treating this source
receipt as executable admission. Add further operators or index layouts at the
registry recipe owner with native cache contrasts and exact original-object
source controls before extending publication. The durable native matrix is in
`rust/tcl-syntax/tests/data/native_numeric_operand_conversions/`.

`SourceIntegerConvertibleContents` is a separate closed contents domain in the
existing representation owner. A join can retain integer contents from an
authenticated current stock String/List cache and actual Integer-producing
native stores under the same selected C bignum protocol. Stock class with an
unknown current cache cannot enter this domain: integer-looking original bytes
may coexist with a cached Double, whose rounded value and Increment acceptance
are different. Actual Double and unspecified Numeric caches also decline. `contents_integer_increment_conversion_at` requires a successful live
physical read and closes only expression-number/Increment-integer conversion.
Its strict representation remains unknown: it supplies no literal value, current
numeric intrep, frozen operand, freshness or general object-effect closure.
Double, unknown/custom, missing, incompatible-engine and generic shared-coercion
alternatives withdraw the domain. Fixed-width C and Jim still require their
separate exact range/getter evidence. O114 combines this purpose query with the
original read/store identity, observer and selected-handler schedule; a semantic
Int label cannot author an edit. Direct `incr` requires a current Integer producer.
When only conversion acceptance is closed, the selected C bignum recipe retains
`expr {+ ORIGINAL_VAR}` before the increment, using exact original variable text,
a freshly validated native preparation and the same unobserved native expression
handler. This preserves conversion of a shared original input instead of moving
it to Increment's private copy. The 300 native cache observations cover original
string/List/Integer/bignum inputs across C85/C86/C90/C91: compiled direct Increment
agrees in these probes, generic direct Increment differs in 40 cases, and all 60
original/residual pairs agree. The residual does not extend to fixed-width C or
Jim without their separately audited recipe. These measured inputs do not close
an unknown or cached Double input. Loop edits additionally require the original
read/store observation, admissible current cache and sharing proof. A missing
stock-to-List conversion receipt cannot be replaced by an integer type label.

`SourceInvocationBinding::sole_rhs_read_store_observations` retains the original
expression allocation, mapped variable occurrence and scalar physical address in
each actual evaluation. `SourceReadStoreObservation::read_context` is before the
original RHS read and runtime coercion. `setter_context` is after argv evaluation
and selected handler preparation, before the captured original write. A category
established by the RHS cannot prove the earlier input was already Integer. The
receipt supplies address/order evidence only, independently from conversion
acceptance, object effects, literal values and handler equivalence.

Consumers must match every original read alternative to its observation by full
expression source identity and mapped operand, then validate the proposed setter
in every retained setter context. A single joined context or equal unknown
addresses cannot replace this coverage. Missing/conflicting observations, unknown
cell generation, relocation, callbacks, a templated RHS or another destination
withdraw the receipt. `increment_rewrite::assess_increment_rewrite` consumes the receipt; assignment
rewrites reuse this owner rather than construct
a local read/store pairing. `command_binding::read_store_schedule::tests` cover
separate pre-read/setter contexts, unknown physical generation refusal, and
callback/template/destination withdrawal. These observations establish no
arithmetic or source replacement permission on their own.

Use `NativeResultContract` for the selected operation's result dependency. A native `set` write returns its captured cell after write observers without triggering a new read trace. A callback can change or delete that cell. Selecting a fresh name lookup or a read operation afterward is a different observable action.

Keep bytes, cell identity, and shared value-object representation distinct. Jim expression coercion can shimmer a list object shared by two variables without writing either cell. The representation owner withdraws incompatible shared evidence across every alias and caller restoration. A permanent per-variable “list” flag cannot express this.

## Preserve iRules execution domains

Use the shared placement/event descriptors and `WorkerExecution` supplied by the
execution owner. Connection, event activation, worker and interpreter storage
remain distinct. Storage lifetime alone does not specify publication to other
workers. Helpers inherit their actual caller domain through reached call edges,
rather than lexical proximity to an event definition.

The explicit authored policy `AuthoredTmmStaticPolicy::RuleInitPublication`
selects logical publication independently of physical engine, source and naming
providers. `Vm::install_irules_static_simulation` installs that model;
`::tmm::_static_enroll` enrolls actual namespace incarnations. The selected
`AuthoredTmmWorkerTopology` distinguishes a root interpreter from enrolled
children. Mere child-interpreter existence does not enroll a worker.

Resolve aliases and attached array members to their actual root cells before
classifying publication membership. Reached RULE_INIT static mutations publish
ordered original Values to enrolled recipient cells before normal recipient
traces. Only the replicated primitive suppresses re-publication; recipient
callbacks resume the ordinary initialisation context and can publish further
reached mutations. Partial writes and recipient errors remain visible.
Completion-bearing checked storage doors report those failures; the legacy
void/bool storage facade retains its interface while still publishing storage.
Ordinary globals, later event stores and root-frame timer stores remain per
worker, with timers selecting `ExecutingWorker` context.

The journal retains real authored Value owners and replays through normal
storage primitives for late enrollment. It is not serialization or a cell
snapshot. Deleted namespace/interpreter receipts retire membership.
`NativeCompilationEntry::authored_tmm_static` retains the independently installed
policy, optional event context, actual enrolled namespace, recipient namespaces
and observer epochs, and outward observer class. Source analysis applies that
receipt only to resolved static cells after links. A name prefix or an ordinary
global cannot borrow it. Missing event context retains possible outward
callbacks; the current interpreter's local observer table cannot close recipient
effects. Incompatible receipts withdraw reuse.
Direct `NativeVariableObserver` registrations count as opaque recipient callbacks
alongside script traces, even though guest trace introspection does not expose
those rows. Their original registration token and actual attached cell determine
lifetime. Unsetting and recreating a same-named recipient cell retires the old
registration; replacement/removal changes the retained observer epoch, and an
absent event context remains `Unknown`. A cached script label cannot close this
observer inventory.


This simulator contract supplies no concrete Runtime TMM backend or appliance
capability. Without a corresponding provider or vendor observation, native F5
behavior and cross-worker values remain unavailable.

## Keep typed storage keys through cell analysis

`VariableCellKey` is the primary address for facts about a selected cell. It
retains actual namespace identity, activation or instance ownership, retained
raw slots, member keys and replaced lifetimes as distinct domains.
`CellOwner::NamespaceIdentity` carries the original `SourceNamespaceKey`;
equal component paths do not identify a recreated namespace. Use
`canonical_binding_value_key`, `canonical_place_key` and
`canonical_literal_variable_key` for fact lookup and publication. Their
compatibility `*_name` counterparts and `compatibility_name()` return diagnostic
text, which cannot reconstruct those domains.
`namespace_membership_for_advice` returns either a retained exact namespace key
or an explicitly authored namespace spelling. It supplies structural membership
advice only: no existence, contents, native identity or TMM storage proof.
Compatibility parsing applies only to the authored alternative.

`VariableCellTable` and `VariableCellSet` accept exact typed queries. A string
query addresses only `Authored` entries; it cannot access a native entry by
copying its displayed label. Namespace membership uses `namespace_contains`
and `VariableCellKey::is_in_namespace`, preserving component boundaries and
interpreter identity. Captured-cell retirement consumes the selected namespace
identity and does not retire a same-spelled recreated owner. Unknown bindings,
generations, observers or unsupported alias routes withdraw facts rather than
selecting a text substitute.

SSA consumers use `SsaFunction::cell_key(symbol)`, `cell_keys()` and the typed
`cell_symbol` query. `cell_name`, `cell_names` and `var_name` are presentation
or source-spelling projections, not an interning input for native storage.
Def-use chains and phi inputs retain typed keys; their source-name lookup is
allowed only through the separately retained proved source-symbol mapping.
Array member fanning uses typed root/member relationships, not string prefixes.
`MemoryLocation::storage_key` carries canonical proof independently of its
`name` field. Unordered exact-key inventories may expose ordered diagnostic
views; presentation order cannot establish storage equality.

Preserve `namespace_retirement_preserves_colliding_paths_and_recreated_owners`,
`array_entry_facts_keep_original_namespace_incarnations`,
`typed_chains_cannot_be_queried_by_native_presentation_text` and
`canonical_memory_keys_preserve_native_incarnation_and_member_lifetime` when
changing these consumers. Test a foreign or recreated owner and a copied
presentation label alongside ordinary authored source names.

`VariableCellKey::activation_identity` identifies the activation from retained
cell roots and lifetime/element wrappers. SSA `invocation_read_bindings` compares
that typed owner with the authentic before-statement or before-terminator
`ResolveContext`. Foreign callee-local reads remain in the physical invocation
inventory and do not enter caller SSA; caller, namespace and genuine root-alias
reads retain their independent keys. Display labels cannot identify an
activation. See the [caller SSA contract](../analysis/name-resolution-proofs/variable-invocation-caller-ssa-frame.md).

SSA source classification retains `VarReferenceScanner::lexer_config()` for
braced ownership and nested substitution roles. Registry profile grammar cannot
replace vertical-tab or adjacent-list boundaries selected by that scanner.
Queried setter reads likewise consume the shared original entered/conditional
child expression owners. `set_value_reads` preserves ordinary substitutions and
returns before its unqueried direct-Registry expression adapter; a same-spelled
procedure or unknown child cannot acquire expression reads from a source label.
See the [retained grammar](../analysis/name-resolution-proofs/variable-ssa-retained-lexical-config.md)
and [selected setter reads](../analysis/name-resolution-proofs/variable-selected-setter-expression-reads.md)
contracts. The [finite CLI child observations](../analysis/name-resolution-proofs/variable-setter-original-child-expression-identity.md)
retain shared sequential process state and Jim alias refusals independently of
compiler admission or physical cell evidence.

## Retain opaque native effects beside named SSA data

Use `Statement::has_opaque_native_accesses` when projecting one statement and
`Function::has_opaque_native_accesses` or
`SsaFunction::has_opaque_native_accesses` when a consumer needs the function's
residual. These queries inspect retained IR, including retained child scripts;
they do not parse opaque operands or reconstruct variable names.

| Consumer purpose | Required projection |
| --- | --- |
| Named SSA uses and definitions | Keep the genuine named entries. An empty set does not discharge the opaque residual. |
| Physical reads and mutation | `place_bridge::read_places` and `statement_mutation_places` retain unknown accesses; `def_places` supplies no invented definite definition. State SSA reads unknown storage before a clobber. |
| Value reuse, store removal and movement | Check the opaque residual before using empty named sets, clean bindings or sparse source tokens as evidence. Unknown accesses withdraw those proofs. |
| Taint and receiver identity | Widen existing SSA values and return summaries; withdraw receiver identity. Do not create a guessed variable to represent the residual. |
| Source edits | `Statement::source_edit_span` declines the opaque invocation. A diagnostic range cannot author edit eligibility. |

The `opaque_native` compiler controls cover named-set emptiness, physical clobber,
value numbering, store liveness, taint, receiver withdrawal and exact bytecode
literals. The WASM image control checks the complete byte buffer, including NUL
and non-Unicode bytes. Emission tests establish byte transport; they do not
supply a stock handler, compiler admission or closed effect contract.

## Native regression requirements

Verify result bytes, completion options, storage state and mutation count at the
actual entry. File-root, procedure, object and C API entries can select different
compilation or propagation behavior. Run the same native fixture against the
pinned C Tcl 8.4, 8.5, 8.6, 9.0 and 9.1 releases and pinned Jim; native feature
absence is an outcome. Owner-level and consumer-level checks cover different
obligations: a source receipt does not establish emitted operations or executable
admission. Advisory projections need controls against executable or writable
reference promotion.

Use the following pairs to verify these axes. Assert the native
result and completion/options as well as the owner receipt; for a compiler
change, also assert the emitted operation and original argument evaluation.
An editor-only May projection needs its own negative against executable or
writable-reference promotion.

| Axis | Affirmative experiment | Dependency-changing control | Consumers to verify |
| --- | --- | --- | --- |
| Command incarnation | Call the original procedure through its retained slot or import | Redefine, rename, delete/recreate, mutate during argv, or retire an import origin | Call graph, navigation, rename, effects, lowering and codegen |
| Captured command prefix | Register an actually returned prefix with its original receiver/namespace | Replace private dispatcher or wrapper; skip registration; construct an unscoped `my` prefix | Callback inventory, receiver navigation, argument advice and deferred dispatch |
| Physical variable read | Read the selected cell before a later word changes an alias/index | Retarget `upvar`, recreate an array root, attach a read/write trace, or enter another activation | SSA, liveness, missing-read advice, taint, conversion cost and runtime |
| Frame and namespace | Execute the body in its actual caller/namespace and record original source | Dynamic level, unknown caller, unavailable namespace, or conflicting entered frames | Body units, procedure publication, diagnostics, locals and completion routing |
| Package provider | Retain an actually selected loader, accepted version and installed surface | Advertise without loading; replace loader; reject version; stop with partial effects | Availability, call identity, body discovery, caches and native entry |
| Compiler timing | Compile the original chunk before an argv mutation | Dynamic/expanded head, separately compiled child, malformed body, or unknown compiler entry | Admission, body traversal, exact replay, bytecode and WASM |
| Object conversion | Reach the selected getter on the same original object | Custom updater/free callback; raw String versus ByteArray; stale cache; unavailable native state | Source effects, selected result, VM and Runtime cache/error adapters |
| Representation merge | Join closed current alternatives with independent producer receipts | Add an Unknown/custom/missing arm, failing conversion, pooled constant or forwarded result | Shimmer advice, numeric/category proofs, constant folds and executable erasure |
| Actual dialect | Retain the exact native family/release/build and independent lexical axes | Conflicting axes, vendor compatibility version, unknown engine or missing provider | Lexer, registry grammar, nested parser, native selection and both runtimes |
| iRules domains | Execute a helper on its actual caller's selected TMM | Change TMM, event/connection activation, initialization epoch or worker certainty | Cell overlap, alias ownership, cross-event analysis and simulator state |

A shared fixture may cover several consumers, but each consumer must demonstrate
that it consumes the right projection. An unchanged warning count cannot prove
that a physical read survived; a matching result string cannot prove unchanged
completion options, object cache or callback ordering. Include error outcomes
and unavailable native features alongside normal outcomes.

Use `scripts/dev/run-resolution-oracles.sh` for the strict reference setup. `TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1` and `TCL_LSP_REQUIRE_JIM_ORACLE=1` prevent missing engines from silently reducing coverage. The shared vector owner is `tcl_syntax::execution_conformance`; native execution, source rewrites, and runtime integration must exercise the same fixtures where their entry contracts agree. Registry's dialect differential uses the shared exact-release locator rather than a PATH release prefix. Its package tests execute every selected C reference and retain Jim's unsupported comparator/deferred-loader surfaces explicitly. Runtime's array-trace suite installs the actual 9.0 profile before registering core commands and uses that release's validated source-built interpreter. A loaded distribution command's presence does not prove a core constructor, handler or compiler hook.

### Regression checks

Run the owner's exact test, its dependency-changing control and the relevant
consumer tests. The strict native corpus and Rust repository checks are:

```sh
export TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1
export TCL_LSP_REQUIRE_JIM_ORACLE=1
cargo test -p OWNER_CRATE --lib FULL_TEST_NAME -- --exact
cargo test -p CONSUMER_CRATE --lib FULL_TEST_NAME -- --exact
scripts/dev/run-resolution-oracles.sh
make rust-check
```

Replace the owner, consumer and test placeholders with the actual crate and
fully qualified test names. Check that an exact filter runs one test; zero
matches is missing verification. Tests describe the source used to build their
executable. Keep the documented stack and time limits; a timeout supplies no
semantic result.

The following boundaries require independent semantic checks. An owner unit
test does not establish the consumer's source, execution or presentation
obligations.

| Boundary | What the consumer must demonstrate |
| --- | --- |
| Registry and dialect | Exact selected engine and independent axes; unavailable and unknown remain distinct; DSL, renderer, studio and backing parity are checked or precisely recorded in `GAPS` |
| Source and body analysis | Original source site, frame, role indices, entered-body purpose and all candidate residuals survive |
| IR, CFG and SSA | Actual operand evaluation, captured physical reads, reaching stores and unknown joins survive the projection |
| Diagnostics and LSP | Advisory facts cannot grant execution, writes or rename positions; declaration and use inventories retain their intended distinction |
| Rewrites and bytecode/WASM | The emitted operation uses the selected compiler recipe, keeps original argument order and rejects a changed dependency |
| VM and Runtime | Same-object cache mutations, failure state, byte-valued completion and host refusal follow the actual reached stage |
| Host and iRules harness | Physical engine and logical dialect remain separate; worker, connection and activation identities are inherited from actual execution |

Stored native observations are useful regression vectors, but rerunning such a
vector only verifies the current implementation against the stored result.
Claim fresh interpreter agreement only when the test actually launches or
links the recorded native interpreter. Record its release or commit, build
configuration, ingress (file, procedure, object or C API), fixture hash and
observable result/options/cache transitions. A simulator test does not replace
a vendor observation, and a nominal compatibility version does not select a
measured native contract.

## Consumer boundaries of shared facts

Constructor argument advice uses `SourceInvocationBinding::constructor_entry`,
which retains the original class incarnation, declaration source, parsed
formals, body and receiver frame. It does not require a callback-free
constructor and does not prove the returned object's class or successful
completion. The native manufacturer descriptor selects the payload offset;
the shared formal binder rejects impossible arities. A consumer may attach
advice only to the unit whose exact declaration and evaluated body match that
receipt. A document-final class definition or equal constructor name cannot
replace it. Class reconfiguration, argument callbacks, inheritance and
unmodelled constructor chains must withdraw this entry proof independently of
nominal handle candidates.

Method navigation uses the sibling `SourceReceiverMethodEntry` inventory. Its
key includes the receiver table: an instance declaration cannot authorize a
class-object reference with the same name. The entry retains the original
name-word site, declaration allocation, evaluated body, parsed formals and
receiver-method frame. Validate these against the actual unit's source before
editing or navigating. The class query reads the current retained class
receipt, while the object query requires the exact original head-word read and
its post-argv object-dispatch generation. Obtain the invocation's actual reads
through `CommandBindingRealm::variable_accesses_for_invocation_args`; nested
substitutions and body phases have their own temporal owners. Do not scan
variable spellings and attach the resulting name to a receipt. Reconfiguration or an opaque callback
withdraws the receipt independently of an unchanged variable version. A
document-final method map or a matching QName cannot replace it. The entry's
`is_exported` status records native visibility at that retained generation;
using it for public dispatch still requires the current receiving allocation
and class dependencies. A method declaration resets the registry-authored
default, while a later original `export` or `unexport` changes an already
installed method. A visibility directive before a future declaration cannot
change its default. Name inventory still includes private methods and supplies
no public dispatch, execution, arity or return-type licence by itself.

Definition metadata has its own `SourceDefinitionMethodReference` receipt.
The accepted native class registration retains each original private worker,
unchanged name operand, receiver table, and method declaration available at
that definition phase. An `export` before a declaration records no method
entry and cannot borrow a later same-name method. Coverage is keyed by the
full created class allocation, including its incarnation, and survives class
retirement. `definition_method_reference_inventories` distinguishes complete
empty coverage from missing or conflicting coverage; whole-method rename must
honour that distinction. These original source inventories belong to the
retained analysis realm and are not donated to native body templates or
callable method lookup.

Procedure result inference uses a separate body-purpose inventory. Lowering
retains compact `Module::procedure_implementation_bodies` records from the
source owner's exact deferred implementations, including retired incarnations.
Match their full allocation, original body source and formal names to the
actual analysis unit; installed analysis bodies remain separate from callable
IR declarations. `TypeInfer` may propagate a conditional normal result only
when the positioned `proved_handler_target` selects that same full allocation.
`ObjectHandleFacts::normal_procedure_result` uses this key, so a replacement
procedure cannot borrow the old result through its name. These result types
provide neither call completion nor object allocation, class identity, method
dispatch, purity or rewrite erasure authority. Test original/replaced calls
and a renamed original together; recursive bodies without an independent
normal result must remain overdefined.

A `my variable` invocation still selects a receiver method. The spelling is
not reserved against a class-defined `variable` method: real C8.6, C9.0 and
C9.1 accept an override which returns normally without installing any alias.
A stock namespace-alias hazard therefore requires the built-in method to
remain an actual candidate; normal alias transfer additionally requires its
selected implementation and the receiver namespace. A catalogue keyword or
deferred method declaration cannot supply either proof.

Cached source positions are not cached execution proofs. In incremental
analysis, `CmdCommandSite::rebase` relocates original command words into the
document; it cannot establish the document's classes, aliases or mutation
history. Before result classification, the common diagnostic path restores
those words through `SourceCommandBindings::stamp_original_tokens` using the
document's retained source owner. A missing original site stays unknown.
Whole-file and per-item analysis must agree on both the diagnostic and its
exact source span. Pair a known constructor with a renamed or replaced
constructor when modifying this handoff.

Retained source inventories use shared immutable backing. Selecting one body
must not copy the complete document's dispatch points, compiler coverage and
deferred outcomes. A mutation detaches its backing through the owner's
copy-on-write path; it must not modify another selected body's proofs. Test
isolation after a write as well as the unchanged large multi-procedure budget.
Sharing storage does not merge source instances or confer a proof on a missing
site.

| Owner change | Consumers to inspect |
| --- | --- |
| Command target/selection or alias argv | Lowering, inline emitters, CFG completion, SSA, variable transfer, SCCP/GVN/DSE, taint mitigations, LSP definition/reference assistance, source rewrites, runtime caches. |
| Body/frame/completion grammar | Source flow, evaluated-body regions, procedure activation, native compiler traversal, exception ranges, callback summaries, runtime control state. |
| Physical binding or lifetime | Source variables, point contexts, scalar/memory SSA, existence, type inference, scope diagnostics, upvar/uplevel/reference formals, Jim statics, iRules/TMM state. |
| Implicit math binding or native function protocol | Source expression traversal, syntax preparation, installed-table export, runtime/VM caches, SCCP, source rewrites, codegen folding, artifact dependency guards, result representation. |
| Result or representation | Source result flow, constant propagation, type inference, binary/list/expression operations, native runtime values, VM values and expression cache. |
| Provider/dialect/entry identity | Registry projection, package loaders, SpecTcl/Studio model parity, incremental keys, compile-service wrappers, command guards, interpreter/profile rebootstrap. |
| Compiler failure/admission | Analysis reachability, template/inlining eligibility, bytecode/native/direct/WASM entry, neutral metadata/presenter, command and math-table dependency validation, procedure-formal admission, script first-tick admission, error-storage observers. |
| Source instance/mapping | Lowered bodies, CFG synthetic boundaries, diagnostics, codegen source metadata, every source rewrite, edit conflict resolution, incremental source inventories. |
| Pattern or option operand layout | Registry role/layout queries, analyser pattern checks, security diagnostics, taint pattern positions, regex source spans and semantic tokens; preserve the retained dialect and original operand origins. |
| Output lookup sequence | Normal transfer, physical mutation/presence, SSA clobbers, catch merge boundaries, runtime output writes, both inline catch emitters. |

Registry semantic fields have matching model support or an explicit unsupported
`GAPS` declaration and rejecting loader. SpecTcl loading/export, Studio forms,
registry cache keys, source attachment, IR relocation and runtime admission
consume that same declared contract.

A shared query specifies its input identity, output purpose and invalidation
boundary. `None` or an unknown alternative never supplies a missing prerequisite.
Native engine/release, physical frame, provider and original operands remain
independent inputs where behavior depends on them. Compatibility defaults cannot
supply those facts. Strict consumers can decline while diagnostic consumers use
a narrower fact; the API exposes that purpose distinction.

Adapters, caches and presentation paths must retain the same prerequisites as
their owner. Cache-isolation and incremental-restoration tests cover those
boundaries separately from direct descriptor construction.

For variable writes, distinguish a *name position*, a *possible write*, and a
*committed normal write*. `ArgRole::VarWrite` identifies the operand and its
source declaration; it does not establish defined contents. `binary scan`
with empty input sets none of its targets on all six reference engines. A
dynamic `foreach` can run zero iterations. Both need a retained undefined
alternative at a later read, while initialising the variable beforehand is a
positive control. A known nonempty literal loop, a matching literal regexp,
and a successful literal scan require the shared normal transfer to prove the
actual assignments. Do not restore an assumption that a possible loop ran to
make diagnostics silent.

An unknown caught command is a separate uncertainty: it can write the
caller's variables through `uplevel` before throwing. A test for an unrelated
missing read should use a known handler such as `error stop`, and pair it with
the unknown-command case that must decline. The same closed-presence API can
support a possible-missing diagnostic without proving a definite execution
error, constant, or removable command.

Fallback spelling scanners must use structured invocation words and the
retained `InvocationDialect` for both argument-role and frame selection. A
computed switch has an unknown value; its written `$option` is not a literal
switch. When the shared layout query declines, retain the scanner's opaque
obligation. Ordinary computed values whose layout is already known must not
blind unrelated variable roles. Brace quoting remains part of the source
carrier in this compatibility path.

For a regex pattern, `NormalRepresentationInvocation::pattern_source_argument_index`
combines the selected handler's registry role with its effective operand origins.
For a reached compiled regexp it requires the registry's
`NativeCompilationSpec::operand_layout` plus the original compiler registration
and independent guard prerequisites. `NativeCompilationOperandLayout::Pattern`
records the last two operands as pattern and subject, even for a dynamic pattern
whose value begins with `-`. Layout evidence does not change an unresolved native
operation to Inline; its source-carrier integration is a separate requirement. This differs from the generic handler:
`regexp -about aa` succeeds with pattern slot 1, so successful argc 2 alone
cannot license slot 0. Generic or unknown compiler paths retain the selected
option grammar's uncertainty. The facade returns only a diagnostic written
argument position; it exposes no opcode or effect licence. A pattern captured
in an alias prefix has no such position. Authoring-only checks use `source_pattern_index` with the original
segmented words and retained `InvocationDialect`. That query supplies syntax
layout, rather than execution evidence. Neither caller should scan leading dash
words independently or replace unknown option values with invented literals.

For generic handlers, the facade can additionally consume
`ResolveContext::substitution_literal_alternatives` at each original retained
read boundary. `CommandRegistry::arg_role_assignments_consensus` compares the
selected structured grammar across those closed, bounded value possibilities;
it returns roles only. Thus `seed` versus `left` preserves the Pattern slot,
whereas `seed` versus `-command` can change regsub's slot and must decline.
Unknown contents, read observers, differing physical read contexts with an
opaque alternative, unknown expansion, or a product above the query's bound
cannot provide this layout proof. These alternatives do not become literal argv,
SSA constants, completion evidence, or compiler acceptance.
When positional arity is still unknown, the facade retains a restricted
PatternLayout proof. Only its conditional normal-edge Pattern getter can use
that proof; representation hints, operand roles, effects, value assignment and
security properties remain unavailable. Known rejected arity supplies no proof.

Hazard consumers use `possible_pattern_source_argument_indices` separately
when no unanimous Pattern slot exists. Its registry owner,
`possible_pattern_argument_indices_words`, selects authored Regexp/Regsub
identity and the actual option table; it does not classify a command by its
written name. An unknown prefix can retain several possible Pattern operands,
but supplies no guaranteed role. Missing values, unsupported layouts and unknown
expansion decline rather than invent a pattern. The facade maps only original
written operands; captured alias-prefix values have no editable source slot.
Taint regex hazards consume this May projection; pattern source tracking and
literal edits continue to require the guaranteed role and original read.
Run `registry_regex_hazards::tests::unknown_prefix_preserves_possible_patterns_without_guaranteed_roles`
and `invalid_counts_and_missing_option_values_have_no_possible_pattern` with
selected C and Jim availability controls when changing either adapter.

`unfilled_trailing_roles_words` uses the same retained grammar for optional
argument fixes. Its layout callback sees prospective words as ordinary Dynamic
slots, preserving the original option/value shapes and selected dialect.
`with_source_argument_words` shares the exact contiguous source-word projection
between authoring Pattern checks and trailing argument fixes; recovery or changed
cardinality declines. It does not establish a reached handler.

Regex source tracking then passes the original `WordExpr` to
`SsaSourceView::read_word`. The shared sole-substitution query includes a variable
inside quotes, and the physical read owner handles qualified names and aliases.
Track `(Symbol, Version)` from that read, rather than a display name or a union of
statement-wide versions. The paired regressions cover qualified/quoted reads,
`upvar` aliases, retargeting between reads and an unknown receiver. Literal source
proof still requires every reaching definition to be lexical source; a known
result from substitution does not become an editable pattern literal.

A positioned pattern consumer with its owning `FunctionUnit` follows the same
sequence as production:

```rust,ignore
let tokens = function.cfg.source_input_tokens_at(block, index)?;
let normal = normal_representation_invocation(registry, None, tokens)?;
let argument = normal.pattern_source_argument_index(registry)?;
let word = tokens.words().get(argument + 1)?;
let read = SsaSourceView::at_statement(&function.ssa, block, index).read_word(word)?;
```

Here `None` in the invocation call does not substitute a dialect: the tokens
retain their source binding and native axes. Each failed query declines the
consumer's proof. Preserve `read`'s physical identity and optional version;
do not replace a missing version with a name-based search or version zero.
If the consumer queries broader contents provenance, retain its explicit
`unknown_residual` as well.

A scheduled tailcall belongs to its issuing variable frame. Catching the returned completion does not remove that request: later commands can run, a successful procedure return dispatches the replacement, an error suppresses it, and another empty tailcall clears it. The VM stores this request on `CallFrame`, so selected-frame suspension and coroutine parking preserve its owner. Replacement command lookup retains the issuing namespace while the replacement executes in the caller variable frame. The authentic original compiler recipe comes from `compile_native_coroutine`; generic handlers and opcodes share scheduling and settlement.

Native list mutation and deferred control transfer also retain their evaluated
word protocol. `lset` captures its scalar/array/stack address before index/value
argv, loads the original cell afterward, and stores through the same compiler
address shape. A zero-index replacement still performs that read; its result
bytes do not require a list representation. Both statement and nested-value
emission consume the same typed hook and original operand map.

For admitted original coroutine compilers, consume
`compile_native_coroutine` and retain its ordered `NativeCoroutineStep`s.
C8.6/C9.0 evaluate the original tailcall head placeholder and replace it with the
executing procedure namespace at execution; no early namespace producer or
stack reversal belongs to that original compiler. C9.1 produces the namespace
first and registers a simple first target with its command-literal role.
Legacy `TAILCALL` has a bounded one-byte count, `TAILCALL4` retains C9.1's wide
count, and `TAILCALL_LIST` consumes the same expanded/empty namespace-prefixed
List. The original List owns its members through replacement lookup and target
completion after the issuer leaves. An empty C request terminates its procedure;
Jim's actual empty tailcall completes normally.

YieldTo keeps its original namespace-prefixed List through suspension, then
resolves the original namespace object and target in the restored caller flow.
A namespace-only List reports the original wrong-argument error before parking.
Creation must retain original argv and its selected namespace; printed fully
qualified names and generated scripts cannot replace that transport. The VM's
admitted direct invocation driver carries no compiler-hook receipt. The
Runtime's private serialised original-object handoff preserves yielded, resumed
and final result headers; its wire-byte probe/inject APIs grant no header
identity. Native wasm suspension reports a typed unavailable capability.

Compiled `error` is a C8.6+ `returnImm error 0` operation. Its emitter retains
original message/errorInfo/errorCode values in native evaluation order. The
`ReturnImmediate` diagnostic carrier asks the VM to build TIP348 `INNER` from
the actual result and original options stack values; source text remains the
separate `errorInfo` presentation. Catch and try use this same selected emitter.

## Retain host availability and invocation realms

`InvocationRealm` is execution-entry data, not a command descriptor flag. `SourceAnalysisEntry` retains the driver phase, deferred bodies retain their definition phase, and immutable source lookup snapshots include it in equality and hashing. An audited evaluated-script handler selects a runtime phase from the actual script-materialisation protocol. Authored braced bodies preserve the parent rule-loader policy; a derived source origin alone cannot establish a runtime entry. Missing or conflicting phase evidence cannot borrow a runtime command table.

CFG command replay projects the unanimous immutable entry baseline from its retained execution lookup snapshots. It rebuilds the initial world with that actual dialect, entry phase, native table, compiler mode, loader contracts and unknown-entry flag; it never uses a post-argv command table as the entry world. Conflicting or missing lookup snapshots on retained invocation carriers make replay opaque. Registry snapshots must agree before a retained baseline can be reused.

`Function` retains its `CfgMetadataContext` from CFG construction. `analyse_command_binding` uses the same retained availability and grammar owner for replay, `binding_at`, rebound names and wildcard detection. Each statement obtains its command lookup namespace from unanimous original invocation tokens. That holder is separate from the reporting `cfg.name`, variable activation namespace and procedure publication namespace. Native source queries retain their original naming input and exact interpreter namespace identity; a positive Logical source producer uses its own retained source model. Missing source consensus, a foreign store or namespace owner, changed grammar and unavailable supplied metadata leave the lookup Unknown. They cannot borrow the body holder or reconstruct a namespace from a displayed function name.

An explicitly standalone CFG without retained source ownership has a separate unpositioned contract. Supplied-source paths never enter that contract after a failed original query. These command-state projections do not issue handler selection, body admission, compiler instructions, runtime frames or Normal completion.

Procedure declarations also have an entry grammar. `native_procedure::procedure_definition_body_policy` selects `OriginalBracedLiteral` for an F5 rule-loader declaration and `EvaluatedValue` for a native interpreter entry. Check the original body word through the shared `NativeCompilationWordShape`; parser recovery and expansion remain unknown. This policy is separate from procedure header compilation, formal binding, runtime body entry and package availability. A quoted Tcl script can be a runtime procedure body without becoming a file-level iRule declaration.

Physical procedure creation uses the independently installed engine. The VM selects declaration capture order, original body sharing/copying and procedure-header compilation through `actual_native_invocation_dialect`; logical naming, formal grammar and expression policies remain separate. Inspect sharing on the original body before acquiring a temporary owner, then compile the chosen original or genuine copied body. An authored Tcl 8.4 naming policy on a C9 host does not select a C8.4 header or activation. Missing physical engine evidence retains a host refusal; a vendor compatibility label cannot supply it. Test the same original formal/body objects under different logical policies, including shared body copies and absent-engine negatives.

Diagnostic flow follows retained `EvaluatedBodyRegion` phases and the registry's `PossibleBodyTopology`. A possible body preserves the unchanged incoming path. A captured catch phase routes its abrupt Tcl completion to the continuation through its typed `RegionTarget::Exit`; it must not be recognised by a written command name. Nested payload getters retain their exact command-substitution tokens and parent dispatch proof even when a normal assignment remains a generic call.

Body reconstruction requires agreement on the original parent, effective
operand, exact source and every recorded entered frame. Use
`executed_script_entry_namespace_context_at` for the unanimous frame's typed
namespace key. Its text projection cannot recover an actual namespace incarnation
or component boundaries. Deferred observations have no entered-frame authority.
A body retained as both a unit and a generic region contributes each namespace
directive once per original invocation site and actual lookup namespace.

`SourceCapturedMethodPrefix` is separate navigation metadata for an actual native list builder with a frozen receiving allocation and unchanged method selector. It travels through the normal result and original argv as immutable `Arc` storage. A selected normal store retains it only at its actual physical destination and original contents origin; a successful unobserved read recovers it from that selected address. Typed writes, opaque effects, object retirement and class dispatch changes withdraw the corresponding store or currentness proof. `captured_method_prefix_arguments` exposes only the original written argument positions at registration. Consumers still need the selected deferred-prefix role and must not infer future callback entry, visibility, completion or opcode selection. `NormalRepresentationInvocation::deferred_script_source_argument_indices` projects the selected structured timing through original written positions and returns `None` for unknown coverage. `Some([])` does not prove callback absence or edit completeness: alias-captured operands have no written source position here. The LSP shared receiver adapter accepts positive original positions only, validates the captured selector's exact editable spelling and declaration receipt, and retains these references separately from actual method-call edges. Test an actually reached receiver registering an external or namespace-scoped internal prefix, then change the registration role, private dispatcher, wrapper, selected member or physical stored value. Unreached method frames and a same-spelled list procedure supply no prefix receipt. Receiver-frame relocation remains unsupported and withdraws this metadata rather than borrowing an old allocation.

An internal self-dispatch prefix additionally captures the actual reached receiver frame and private dispatcher generation. An unscoped list containing that dispatcher is not a registration receipt. `NativeResultContract::NamespaceCommandPrefix::normal_scoped_prefix_operand` identifies the original script operand of the selected native namespace wrapper; its normal result selection remains Unknown. Only a normal wrapper reached in the same actual receiver frame can retain the scoped prefix. A private dispatcher mutation withdraws internal prefix currentness independently of public object-method navigation. These receipts describe the original selector at registration; delayed execution still needs its own current dispatch proof.

For selected instance setters, `ResolvedInvocation::new_instance` supplies the owning class option table when the authored `CONFIGURES_INSTANCE_OPTIONS` trait is selected and the method has no explicit option table. Query forms do not inherit setter callbacks. Instance script timing and callback taint consume this same availability-filtered option view and prefix policy, preserving original argument indices rather than independently looking up constructor metadata.


For iRules, `irules_policy::runtime_surface_admits` adds only the measured fifteen interpreter-present commands to the positive loader surface. The sixteen interpreter-absent commands and later stock Tcl additions remain excluded. The shared fork point selects native member/option availability, independently of the unknown vendor compiler protocol. `rule_loader_refuses` is a separate authored-source policy query. Physical interpreter baselines use `effective_semantics_for_dialect_in_realm(..., InterpreterRuntime)` while invocation facts and diagnostics use the retained point phase. This prevents command presence, loader restrictions and compiler hooks from drifting into interchangeable facts.

`VendorSourceNameInput` retains the actual whole word, complete source channel
and lexer configuration, lexical command site and independently selected hosted
context. `VendorSourceNamePolicy::authored` supplies source advice only for its
selected hosted Tcl context. Its narrow literal projection accepts plain ASCII
text and leaves opaque, escaped, expanded or computed producers owned but
unavailable. An unavailable vendor input cannot reopen a reporting-string
fallback or select a C/Jim materialisation recipe. The independently selected
`ObservedBigIpNamePolicy` supplies only the recorded appliance build/event and
variable-address purpose; authoring context cannot issue that observation.
Neither source advice nor its correspondence check grants publication, table
membership, a compiler primary, runtime cell, Normal completion or editing.
The [vendor source-context contract](../analysis/name-resolution-proofs/vendor-original-source-context-input.md)
binds these separate issuers.

SpecTcl and Studio author the same command descriptors. They do not expose a command-local realm override: a definition pack cannot certify an execution-entry phase or override host load policy. Custom registries still pass through the same contextual phase and package filters.

Alias-origin traversal queries `Namespaces::root_command_context_checked` for
its actual root lookup purpose. C adapters keep their independently checked
namespace-tree root. The Jim Runtime and VM adapters require the live original
object context and actual retained top holder before issuing their root command
context. The VM also requires its live command world. Equal bytes on a foreign
holder or a retired context refuse. The shared
`origin_bytes` and `origin_bytes_checked` consumers use that purpose instead of
converting a namespace label into a C tree token. The exact Jim CLI namespace
composition is separate evidence for its guest result and supplies no pointer,
header or epoch claim. See the [root-context contract](../analysis/name-resolution-proofs/original-root-command-context.md)
and [finite Jim source controls](../analysis/name-resolution-proofs/jim-flat-command-context-source-controls.md).

## Resolve missing commands through their native entry

For command-presence diagnostics, query `SourceInvocationBinding::selected_slot_presence` at the retained dispatch point. It reports the raw selected slot, so an alias or import remains present even when its terminal implementation is missing. Namespace lookup paths are assessed independently before their results are joined; a name tail elsewhere in the document supplies no presence proof.

W123 consumes the narrower `SourceCommandBindings::diagnostic_slot_presence_at` advisory. It accepts exact document-origin dispatch points or the same-origin points of independently retained future entries. Derived and loaded source offsets cannot stand in for document coordinates. An absent slot produces advice only when its fallback remains the explicit initial handler or is itself absent. Custom, replaced or uncertain handlers suppress this advice. The initial Tcl `unknown` handler may autoload a command, so the hint says “Unresolved command … at this source point”; it never certifies execution failure.

Catalogue names and lexical declarations can supply reviewed spelling suggestions. Explicit stubs, extra commands and scoped authoring declarations can suppress the hint, but none of these grants implementation identity, argument roles, native compilation, purity or an executable target. A package requirement changes only the points reached through its actual loader receipt; it does not suppress command-presence advice throughout the document.

The command lookup namespace and missing-handler namespace are separate retained inputs. `command_lookup::native_lookup_fallback_policy` selects the protocol from the actual interpreter contract. A C8.5 alias selects a namespace handler in the global lookup namespace; C8.6 and later select it from the caller frame. The handler prefix head still resolves in the independent lookup namespace. Jim selects its default `unknown` head relative to that namespace: a local `n::unknown` can handle `missing` in `n`, while the C default `::unknown` remains global. Command-slot advice therefore queries the selected fallback policy rather than a canonical handler-name roster. A missing configured handler reports the original missing command rather than recursively selecting another fallback. Same-interpreter aliases dispatch already evaluated object vectors and retain ordinary procedure suspension; missing-handler execution keeps its native callback boundary.

F5's authored Tcl8.4 interpreter contract selects the default `::unknown` prefix used by the simulator's explicit mocks. This is separate from rule-loader availability and the vendor compiler's unknown hook protocol. It is an interpreter/simulator contract, not a live BIG-IP measurement. An unknown interpreter protocol or base release supplies no fallback proof.

The VM's primary `TclError` is `Guest(Completion<Value>)` or `Host(TclHostFailure)`.
A propagated guest completion owns its original control code, result object and
complete options object; adapters do not reconstruct return, break, continue,
custom codes or errors from a message. Ordinary newly authored errors explicitly
supply `NONE`. Value-access and execution-provider failures use the Host branch,
which the dispatch boundary retains before guest catch/try settlement. A
context-free primitive getter cannot resolve an `Unchanged` error-code receipt;
it retains the original typed record as a capability refusal until a VM-aware
consumer can apply the actual interpreter state. `CmdErrorDetails` distinguishes
Default, Unchanged and Set; Unchanged performs no write or guest-global lookup.

Error-info accumulation, command logging and catch publication consume exact
result/code/info bytes. Checked Unicode views serve explicit text consumers.
Dictionary wording and codes come from typed parser failures. These carriers do
not admit unrepresented byte command names or scripts. Native completion tests
must preserve those byte and admission limits.

Expression quotes use `expression_quote_control`, separately from `subst` command
settlement. C preserves abrupt bracket completions and all their options. The
measured Jim084 quote protocol consumes Return into a value while retaining its
level/options; break and continue become quote-activation errors, including
inside an enclosing loop. The explicit `LogicalExpressionQuoteProvider` is an
embedding-host simulation contract for F5's authored Tcl84 core, not a native F5
measurement or compiler grant. Availability alone cannot install that provider.

The simulator explicitly installs a C9 physical engine. Only framework sourcing
and initialization use `try_eval_native_host_source` to activate its logical C9
host policy and source grammar. The configured user dialect remains F5; its
source grammar and logical policy are restored on return.
`NativeCompilerPolicy` retains the physical engine, logical handler profile and
explicit quote, numeric and name providers in cache receipts. Both fast and plain source
caches validate original entries; reusable procedure bodies recompile when their
policy differs, and frozen activations reject a changed policy before executing.
`eval_retained_activation_value` executes an original source object in its
captured frame. The scheduler retains a single script operand directly and uses
the selected shared concat owner for multiple operands. Source bytes and source
location survive through parked coroutine frames; Unicode conversion supplies
no activation or execution authority. The retained policy includes every logical
provider, so installing a different name provider invalidates the callback even
when its physical engine and source grammar remain the same.

Child interpreters retain the engine and logical provider independently. Neither
this host capability nor command-surface availability certifies F5 hardware.

Expression conversion errors select
`InvocationDialect::expression_operand_error_presentation` separately from
numeric acceptance. Supply `NativeExpressionOperandStage` from the actual
failed conversion protocol: Jim binary numeric fallback is floating-point,
bitwise conversion is integer, and condition conversion is boolean. Jim's
numeric unary protocol can fail at its final boolean conversion too; an
operator's broad numeric category is insufficient. The selected Jim 0.84
presenter retains original high bytes, clips at the first NUL like its native
formatter and supplies guest error code `NONE`. C or unknown native points
supply no Jim presenter. Preserve an operational host-refusal tag before
using any guest presenter; never infer the stage from English error text.

A read/modify/write operation captures its physical variable receiver before firing the read observer. `native_rmw` owns missing-content policy and typed failure presentation. The VM and standalone Runtime retain that receiver through alias retargeting, scalar unset/recreation and write callbacks; they do not resolve the source name again. An original array or namespace retirement cannot write into a same-named recreation. C8.4 propagates a failed read, whereas later C increments from zero after a failed fetch. The shared executable vector corpus supplies each release's actual behaviour. Amount conversion has its own retained protocol: C8.4 and Jim validate before the read; modern C converts the fetched current value before the amount. Jim's amount uses the original-object `GetWideExpr` adapter, while the existing value uses ordinary Wide conversion. `prepare_legacy_amount` returns a selected amount receipt without adding an operand reference; `increment_legacy` applies C8.4 copy-before-conversion or Jim conversion-before-copy. Cache inspection is checked so unavailable physical storage remains a host refusal.

Jim expression preparation distinguishes preserved storage, rejected `Expression(NULL)` and a prepared tree. `JimExpressionObjects` owns every original term, including lazy branches, indexed by the scanner's original source and body spans. Concrete adapters construct numeric terms with the scanner-selected constructor and retain raw token bodies for other terms. Command terms own their original filename object and signed native line. Safe integer-expression evaluation returns variable token bodies without lookup and rejects reached command or escaped substitutions; plain string terms and unvisited branches retain their native behaviour. Evaluation retains the parent and tree and restores the same backing after a reached operation shimmers the parent. Jim duplicates retain the resident spelling and retire the expression primary; C cache duplication remains independent.

Use the checked original-object expression preparation door before evaluation. Keep retained native parsing actions separate from guest diagnostics, and inspect a rejected primary before reparsing: its next native evaluation returns an error without replacing the current result. Preserve exact native span identity rather than matching term text. `info script` retains and returns Jim's original current-filename object; copying its text loses the object and sharing effects.

Distinct-array destruction has three shared physical phases in `tcl-runtime-api::variable_destruction`: detach the original root lookup and root registrations; retire each original member immediately before that member's frozen unset callbacks; then complete the old owner's retirement. Root callbacks can still read and write not-yet-retired members through preexisting element aliases. Recreated roots and registrations cannot replace the captured old receivers or callback prefixes. Native member order is not a portable guarantee. The source owner retains typed old-root storage and active member read receipts, and joins every order of a complete inventory of at most four members. Opaque inventories, unknown traces and repeated allocation families retain uncertainty. A unique operation receipt alone never proves that two instances of a source allocation family are the same cell. The shared vectors sort visibility results or callback labels instead of assuming a hash-table order.

The member receiver is captured before the root callback, but its callback list is frozen immediately before that individual member retires. Root or earlier member callbacks can change a later old member's registrations through a retained element alias. Mutating a recreated root by name changes only the new root's registrations. These two routes must remain distinct; freezing every member's callback list before the root callback loses the measured old-alias behaviour.

C Tcl 8.4 expression preparation can compile nested script substitutions before evaluating the expression. `SourceExpressionPreparation` retains a private ordered compiler closure, including the immutable compilation table, exact original script spans and selected invocation recipes. `has_closed_script_compilation` checks those receipts against the witness; `script_compilation_dependencies` supplies the original registration guards to artifact consumers. A table-only expression witness cannot authorize erasing these compiler visits.

A named instance command has its own manufacturer allocation. The source owner
publishes `named_object_instance_at_dispatch` only after the closed native
manufacturer installs that exact command token. The method navigation query
checks its incarnation and the post-argv object-dispatch generation; renaming,
reclassification, replacement or an opaque callback cannot reuse the old class.
A normal interpreter factory is different: its typed `InterpreterTransition`
projects a single child-path component into a parent command in the global
namespace. A nested child path installs no callable command in this parent.
That slot receipt grants neither a procedure/class definition nor child-body or
compiler-hook authority.

A class definition receipt is independent of constructor execution. `proved_class_definition_factory` selects the actual registry factory and the live class allocation, including factories whose manufacturer methods are initially unexported. Closed lexical constructor bodies can be retained without invoking them. Object method configuration revokes the receipt; a rename preserves its physical identity. This receipt does not establish the class of an instance returned by a constructor that may reclassify or replace it.

Dispatch-table command navigation follows the actual consuming invocation. `table_value_provenance` resolves the original variable read through every closed physical context, then follows represented reaching stores of that same cell and lifetime. Array entries, literal dictionaries, native dictionary constructors and dictionary updates retain the original value word or list-element span. The selected list grammar owns those element extents; decoded escapes cannot become editable source merely because their value is known. Dictionary update paths use frozen evaluated keys, separately from operand presentation used for value typing. Unknown stores, observations, unrepresented incoming values, provenance cycles and bounded-inventory overflow decline navigation.

A table value's producer namespace does not choose its command target. The consumer's post-argument `command_reference` snapshot resolves the retained literal. A later declaration can therefore supply the target, and deletion before consumption withdraws it. Different consuming worlds must agree on the same navigation identity before one shared literal is marked rename-safe. This receipt grants navigation only, independently of execution, arity, compiler capability and normal completion.

Represented store offsets are paired with `ContentsSourceProofs`, a physical contents attestation of the complete `SourceOriginId`. Native dispatch selects the active origin; changing that selection does not clear untouched cells. Strong stores stamp their actual source, conditional/broad stores retain a source only on consensus, and opaque observations withdraw it. Joins intersect full-origin attestations. Frame restoration and retained raw-slot transfers carry the same physical receipt; alpha relocation maps source instances as well as offsets and activation owners. An equal offset in materialised or loaded source cannot donate an authored literal's navigation span, even when the stored text is identical.

Receiver-method lookup may retain a stock observer-registration candidate alongside an unknown receiver namespace. `possible_variable_trace_transitions` preserves only this May exposure metadata from original candidate identities and frozen operands, with an explicit unknown residual. The module observer summary can protect that variable without claiming a completed registration or normal transfer. A replaced absolute handler cannot borrow the stock trace descriptor.

A represented variable version is not a successful-read receipt. Code motion that depends on an earlier substitution producing a value also asks `SsaSourceView::read_word_produces_value`. The physical owner checks every closed original context, selected native variable/container policy, defined contents, current lifetime, read observers and scalar/array compatibility. Jim element reads validate the current dictionary and key or a complete typed member inventory. Missing contents, malformed dictionaries, unknown root kinds and observers decline this proof; it neither grants a command completion nor replaces the separate SSA/value dependency.

The normal successor of a represented store has its own contents receipt. For interpreter-owned captured stores, `SourceInvocationBinding::normal_variable_continuation` retains the actual joined physical world after variable callbacks; command observer uncertainty withdraws this sidecar. A generic observed-store replay cannot replace that source closure. Immediate presence is read from the captured post-callback physical world; registered future observers do not erase that snapshot, and their later access gates remain in force. `SsaSourceView::normal_store_contents_preserved` checks the exact physical definition, its current generation, defined contents, original full source identity and the surviving store origin. A closed write callback that leaves the represented value intact can retain that constructor's immediate type. A callback that changes the value, an unenumerated callback, retirement or a foreign source store withdraws the receipt. This projection does not remove observer hazards from later reads or promise an immutable lifetime.

`SourceInvocationBinding::original_normal_result` reads a separate immutable
inventory recorded after the original handler and its leave observers settle.
Its key retains the full source allocation; the query validates unchanged words,
pre-handler targets, frame, frozen argv and compiler/observer basis. Every
represented Normal alternative must agree on result bytes. Representation joins
independently, and an unknown Normal result withdraws the bytes projection.
`original_invocation_completes_normally` requires an independent completion
certificate and no abrupt alternatives. A known result on successful routes does
not prove totality or permit erasing an operation. The selected value-only
`return` retains its pending OK/level-one route through closed original argument
and source-prefix evaluation; only the actual procedure boundary converts it to
Normal. Configured returns and unknown observer or operand paths cannot reuse
that certificate.

`SourceCommandBindings::original_arguments_rejected_before_handler` owns a
separate actual pre-handler failure query. It matches unchanged original words,
source occurrence and typed namespace/frame, and requires every represented
actual operand outcome to be Error with no normal continuation. Actual handler
dispatch or an unrepresented possible body entry withdraws rejection. These
unrepresented entries retain original body origin and optional actual
namespace/frame: unknown namespace covers every context, while a known exact
key filters selected-source views. Opaque or expanded arguments, unknown frame
binding, recursion/depth boundaries and source/preflight skips retain uncertainty.
Declaration previews grant neither actual rejection nor that uncertainty.
Lowering uses the receipt only to withdraw normal setter/handler transfer; it
retains conditional declaration grammar and original AST inventory and acquires
no physical entry, CPP, effects or read value. Preserve actual versus preview
purpose and exact context when changing the query or its consumers.

`SourceCommandBindings::conditional_procedure_normal_result` meets original
declaration-body results with unknown formal inputs. It matches the same
implementation allocation, source, formals and namespace; different return and
fall-through values withdraw the result. An actual call's frozen arguments can
produce a caller-specific result without becoming a declaration constant.
Called declarations use a separate collector with the accepted declaration and
lookup environment. `seed_deferred_inputs` binds unknown formals and widens
namespace contents; the shared body walker records conditional results. Only
those result observations are merged, preserving separate actual dispatch, read,
compiler and caller-result inventories.
Consumers still prove the actual callable, formal binding, argument getters and
effects independently. Neither result query constructs a physical value, cache,
SSA definition or compiler admission.

`native_numeric::source_read_integer_contents` projects a separate accepted
number-conversion receipt from an unchanged original read. Every represented
physical alternative must provide closed integer contents, a successful read
and the selected stock-object consistency recipe. It supplies a semantic integer
without claiming a current numeric primary or unique object. The expression
evaluator consumes it only for Number conversion; Boolean, String, List,
comparison and result-normalisation obligations remain independent. Exact Value
formal binding can retain this original contents lineage for a caller-specific
arithmetic result; unknown incoming values and rest-member selection do not
supply it. The selected recipe covers C Tcl 8.5–9.1; C Tcl 8.4 and Jim require
their separate cache/conversion evidence.

Floating result bytes also require the selected double-string policy. Tcl
8.4–8.6 have mutable thread-wide precision, so a mathematical floating value
does not establish its spelling without an independent precision receipt.
Tcl 9 and Jim have their own immutable format policies. Integer arithmetic
results do not borrow floating formatting authority.

Closed dictionary stores preserve bytes through the selected `DictSet` protocol,
the same captured physical receiver and the exact surviving source store. The
dictionary owner uses the selected native list grammar and shared duplicate-key
rules for each key in the frozen path. Read or write observers, unknown contents
and a retargeted receiver withdraw that bytes projection. Separately,
`NativeResultContract::DictionaryValue` retains a successful dictionary getter's
selected value bytes for a nonempty frozen key path. Missing or malformed values
remain unknown, and the getter grants no fresh object or representation: its
result can share an existing dictionary value object.

## Retain factory bodies without replacing native calls

`SourceCommandTarget::registry_identity()` returns the original semantic registry
descriptor identifier when that issuer is retained. It may be unrooted and is
independent of the current renamed/imported command slot. Use it for descriptor
queries, retain the actual slot for lookup, and do not normalize this identifier
into a written command name. The query supplies no new lookup, implementation
binding, compiler selection or entered-call authority.

`SourceCommandBindings::installed_procedure_body_units` projects exact singleton
procedure implementations still installed at the analysis boundary. Each unit
retains its allocation instruction and incarnation, evaluated body origin,
authoritative `namespace_key` and decoded native formals. Namespace and command
text remain presentation. Opaque, retired, replaced,
repeated-fresh and static-bound implementations decline. This bounded final-state
projection supplies analysis coverage for the retained final installation state.

`SourceCommandBindings::original_declaration_body_units` independently retains
every original declaration's exact allocation/site, source and namespace key.
Later deletion, replacement or an opaque callable continuation does not erase
this original analytical inventory. The lowerer tags those units in
`Module::original_declaration_body_units`, separately from the current installed
inventory. Both tags are excluded from `executable_script_roots`; original body
coverage proves neither runtime entry nor an installed callable or native CPP.
Consumers that need current installation use `installed_procedure_body_units`,
while declaration diagnostics and advice retain the original inventory.

The lowerer selects that origin's retained source inventory in the body's exact
namespace key through `lower_executed_script_in_context` and
`selected_source_in_context`; it does not analyse the rendered body in a new
default world. It records the result in
`Module::body_units` and tags it in `installed_procedure_body_units` with the full
allocation. These tags are excluded from `executable_script_roots` and never
enter `Module::procedures`. Missing child entry or compilation evidence stays
unknown through the normal selected-source lowering path.

`specialise_factories_with_cap` limits this metadata per actual allocation site.
It preserves the original factory call, whose argument substitutions, declaration
timing, callbacks, errors and result remain native. In particular, C Tcl's
procedure-definition result is empty while Jim returns the created name; an
empty replacement block would erase that distinction. Namespace selection comes
from the installed implementation, not from the factory caller's spelling.

For representation diagnostics, `SsaSourceView::read_word_representation` queries the actual captured read and requires every closed context to agree on a known current representation. It also retains the successful-read and observer gates. A surviving constructor's semantic type does not substitute for this receipt: a callback may preserve bytes while coercing the shared object. Unknown or conflicting representation evidence remains unavailable.

Expression consumers use `read_expression_representation` and
`read_expression_representation_alternatives` with the parser's original base,
or the retained `ExecutedExpressionSource` for an evaluated expression. The
operand map must belong to the same full invocation origin. A transformed AST,
missing extent, unknown residual, failed read or observer prevents proof; never
recover it from a matching variable spelling or a semantic return type. Both
expression and word APIs delegate to the same physical-read owners.

Shimmer's private `representation_cost_type` projects these receipts for both
ordinary arguments and expression operands. It keeps semantic numeric-intent
hints separate from claims about List/Dict conversion costs. For example,
`set x [lrange $unknown 0 1]` does not establish a List representation: an empty
result or a Tcl 9 abstract-list result defeats that inference. A selected
nonempty range over a proved ordinary List retains its positive conversion
control. This cost projection grants no execution, constant or erasure licence.


`read_word_representation_alternatives` has a separate cost-advice purpose. It
unions a closed set of ordinary List/Dict possibilities from every original
physical read context, after proving successful unobserved reads and current
cell lifetimes. A missing, opaque or unknown incoming representation prevents
that closed set. Physical joins preserve both known container possibilities,
while `read_word_representation` still declines disagreement and the strict
stored representation is Unknown. Shimmer advice may report a possible
conversion; this carrier never proves a constant, one current intrep, a native
handler, or permission to erase or move an operation.

Actual object results carry their bounded manufacturer allocation separately from result bytes. A selected native return can preserve its original frozen object operand through the procedure completion boundary; fall-through preserves the actual final command result. Every normal or return alternative must agree on that allocation and retain its current class and dispatch dependencies. Errors, text-only results, conflicting branches and changed dispatch withdraw the object proof. An unchanged class label or generated object name cannot create it.

The source result owner carries a selected native List or Dict representation independently of whether the result bytes are known. Each written argument freezes that receipt separately from its text; normal joins require the same receipt on every represented route. Publication into a stored cell requires its exact normal write, full source attestation, and a known unchanged shared representation epoch. A possible coercion advances that epoch even when no cell currently has a representation entry, because a frozen argument can still retain a shared object. Unknown epochs do not prove unchanged representation. The variable world retains the greatest issued stamp across selected-frame restoration. A join of different issuance histories permanently withdraws the next stamp, so repeated allocation families at loop backedges cannot produce an unbounded sequence of abstract receipts. When the issuance history remains known, a selected successful Dict constructor or Fresh List recipe may establish a later stamp after an uncertain join; it cannot revive an older frozen receipt. Counter exhaustion remains unknown. Without a stamp, a selected constructor's immediate result shape can pass only through the final plain command-substitution operand into an unobserved store; it cannot serve as a frozen receipt across later argv effects or command observers. This records temporal validity, without proving a fresh native allocation or unique sharing. Repeated coercions while the stamp is already unknown do not issue receipts or increase the high-water mark, so an unknown loop state can converge; recovery requires a known issuance history. A command leave callback that applies `llength` to a shared dictionary can therefore withdraw the outer store's earlier Dict receipt without changing its bytes.

Completed result bytes join independently from representation metadata. Every
represented route must supply identical bytes; a missing or differing value
withdraws that evidence. Differing numeric receipts or representations withdraw
only their own metadata. A procedure's fall-through result may preserve a
frozen representation only while its epoch is current before and after the
settled completion boundary. An abrupt route that becomes normal has no such
receipt and prevents this projection. Immediate constructor results cannot
cross this boundary as frozen values.

The source store owner publishes a known List or Dict representation from the original frozen value before write callbacks. Those callbacks can withdraw the representation while preserving the contents store receipt. Compiled C constant lists need a separate literal-object prerequisite: the native compiler can reuse the same object across procedure calls, and a caller's dictionary conversion changes the representation seen on a later call. A mathematical list value or constructor type alone cannot establish that the reused object still has a List representation.


## Retain results of reached native substitution templates

A factory can obtain a body through a reached `subst -nocommands` call. The
source owner asks `ResolvedInvocation::native_substitution_template` for the
authored template protocol, frozen operand index and enabled substitutions.
The selected invocation retains its available option table, prefix grammar
and actual native family. A command name or the presence of a `subst` row in
the catalogue does not supply this protocol.

The source walker obtains the exact `ExecutedScriptSource` and uses the shared
word-component scanner under the retained lexer grammar. It performs scalar
reads in order through the existing physical read and observer owner. Earlier
read effects survive a later parse error. The current rejection proof covers
C Tcl. Jim 0.84 selects a separate template-variable policy: a braced scalar
name may run to the end of the template, and escaped variable markers remain
inert even under `-nobackslashes`. This policy never changes written-word
parsing. Jim expression sugar, enabled command substitutions and array indices
retain their effect residual until their completion protocol is represented.

A successful projection retains resulting bytes for subsequent source
installation. It does not manufacture native object identity, a numeric
representation, constructor closure or an opcode proof. Keep the original
substitution and factory calls; consume installed child bodies only through
the allocation-keyed analysis metadata described above.


## Retain separate contributors to constructed command prefixes

Navigation through an actual array element or dictionary value follows the
current physical read, full-origin store receipt and selected native constructor
operands. `NormalRepresentationInvocation::list_constructor_words` preserves
the original ordered List operands. The table owner retains these as separate
contributors rather than inventing one contiguous source span. An expanded
constructed prefix uses its first operand directly: `{my handler}` denotes one
command name and must not be split again into `my` and `handler`. Unknown first
operands cannot borrow a later known argument. Constructor results also retain
their ordered key/value contributors when used as dictionaries; duplicate keys
keep the final contributor. None of this establishes executable dispatch or a
current native object representation. Command lookup still occurs at the
consuming invocation, after its arguments have run.

## Preserve the lookup purpose of command-name references

`SignatureCommandInvocation::lookup` retains `SignatureCommandLookup` in
workspace rows and body-fragment relocation. An invocation head represents
execution. `ConsumedName` retains the exact offset of the handler consuming a
name after evaluating its arguments; its written argument span remains the
edit target. `DeferredReference` supplies navigation assistance without
asserting current existence or an executed call edge. Command-prefix
registrations and constant-dispatch contributor literals use that last purpose.
Existence probes keep their independent suppression policy. Required contents
reads and existence-name dependencies are separate in
`DeclarationInvocationFlow::{reads,name_queries}`. The graph retains name
queries as uses without claiming current contents. Original diagnostic queries
use `in_expr_for_diagnostics_at` or `in_tokens_for_diagnostics` and require the
closed original handler plus unanimous effective operation and argv. They may
suppress missing-read warnings under that selected handler's semantics.
`in_expr_at` remains actual-only; physical existence, executable admission and
SCCP branch erasure require independently reached receipts.

For an immediate consumed name, query the consuming point and call
`lookup_command_word(name)` before asking for its command reference. W123 uses
`diagnostic_command_slot_presence_at(name, consuming_offset)`, which shares the
actual root and independent future-world lookup owner. This catches a target
deleted by an earlier argument substitution and preserves a valid rename's
source reference before the command moves it. Argument spans, final command
tables and rendered qualified names cannot substitute for that point.

Navigation and rename keep these references. Call hierarchy and execution
graphs exclude consumed names and deferred declarations from executed calls.
The diagnostic still describes unresolved selected-slot advice, independently
of default autoload behavior or a custom missing-command implementation.

The LSP matching adapters retain this distinction through presentation. The
reference adapters match the called definition slot; the call adapters match
the terminal procedure reached by an execution head. Both receive the actual
document source and use `AnalysisResult::proc_for_definition` or
`class_for_definition` to match the retained allocation. A receipt with no
matching definition is a settled refusal, rather than missing evidence that
permits a final-name fallback. Never obtain the current document's bytes from
the candidate receipt: that makes a foreign-source check tautological.

| Presentation consumer | Matching purpose |
| --- | --- |
| Rename, document references, highlights, linked editing and code-lens counts | Called-slot reference identity and original editable source. |
| Symbol graph reference positions and counts | The same reference-span adapter, deduplicated by actual span. |
| Call hierarchy and graph call-site positions | Execution-head purpose plus the retained terminal procedure identity. |
| Definition and declaration hover | Retained called or linked declaration; known non-definitions settle to no declaration. |
| Completion usage ranking | Advisory name usage; its count grants no execution or editable-reference proof. |

Graph scope membership compares byte spans, rather than line numbers. Two
procedures and a top-level call can occupy one physical line. A line-based
membership test would move the top-level call into a procedure even when the
command target itself was resolved correctly.

LSP reference consumers pass the actual document bytes into the common
`references::{invocation_references_proc,invocation_references_class}` matcher.
Editable references use the selected direct called slot and exact original
allocation; aliases and imported wrappers do not become target rename edits.
`invocation_calls_proc` instead requires an execution lookup and follows the
separately retained terminal declaration. Find References, highlights and local
code-lens counts may report terminal references while rename and linked editing
keep their direct-slot policy. Known non-definitions, conflicting incarnations
and foreign editable declarations block all wildcard, final-name and indirection
fallbacks. A matching qualified name alone cannot override a retained receipt.
`invocation_calls_named` is the reporting-only external-target adapter: an actual
foreign Procedure allocation may identify its terminal qualified name, but cannot
supply an editable declaration or local physical identity. Consumed-name and
deferred-reference rows never become call edges or linked self-call ranges.

## Distinguish compiled list construction from literal reuse

`NativeResultSelection::list_construction` consumes the actual native compiler
selection and original post-head word shapes. C8.4/8.5 emit a runtime LIST for
nonempty compiled lists. C8.6–9.1 may push an interned object when every operand
is compile-time known; a previous use may have changed that object's internal
representation. Ordinary dynamic operands produce a fresh List, as does a
proved generic native list handler. Expanded or missing compiler shape evidence
remains unknown. Frozen argument bytes and a semantic List type cannot replace
this query. A fresh list result does not prove private element objects or erase
operand coercions.

Normal ordinary-container coercion is a separate registry contract from a result type or operand hint. The source owner first proves an existing ordinary List/Dict domain; C 9 AbstractList values and unknown representations remain excluded. Possible shared input objects retain the union of their old ordinary domain and the selected conversion target. Exact disjoint bytes preserve the old shape. An original final variable operand can commit the target only at normal completion with the same physical receiver and contents origin; the C 8.5+ empty-list shortcut retains both alternatives when actual bytes cannot exclude preservation. These closed alternatives are conversion-cost advice, not a Must representation or an erasure prerequisite.

### Receiver-local dispatch preservation

Actual `my variable` storage uses a separate `CellOwner::AllocatedInstance`
receipt containing the complete source allocation, its caller frame and its
bounded First or Second incarnation. A receiver class label, method declaration
or `Instance(String)` label supplies no physical storage. The source dispatcher
must independently retain the active receiver and its current builtin operation
before calling `link_allocated_instance_variables`. The helper validates local
names and destinations in order: an empty name is valid, while an invalid later
name or occupied direct local leaves earlier links installed. Linking requires
no value in the object variable. Existing links may be retargeted; direct formal
values and traced undefined locals cannot become links. Captured local wrappers
follow the exact new cell. Frame entry, restoration and proof relocation retain
object cells through typed slot metadata. Reached destruction calls
`retire_allocated_instance_variables` with that exact allocation, withdrawing
contents and source attestations without retiring another object's cells.

A successful direct local scalar store can establish its current value and
representation even when a loop join lost the old cell incarnation. This narrow
lookup proof requires the current activation, a strong represented write,
defined scalar contents and closed observers. It does not recover the unique
physical incarnation or a canonical physical SSA name. Namespace, element and
captured-cell accesses retain their independent lifetime requirements.

An actual native instance allocation owns its receiver-local dispatcher. The
source owner enters a retained plain method only from that current allocation,
the current class/base dependencies and the original method body/formals. An
external object call additionally requires the native default exported-name
policy; receiver-local calls can select the original private method. Generic
deferred method analysis has no receiving allocation and supplies only May
builtin hazard candidates.

`receiver_self_method_entry` requires a separate dispatcher generation as well
as the class/instance generation. Command-table mutations withdraw the private
dispatcher witness because its actual namespace is not a displayed class name.
A fresh, closed manufacturer establishes a new original dispatcher; internal
restamping of its already installed named command preserves that same witness.
The actual C8.6, C9.0 and C9.1 controls replace the object-local `my` procedure
and observe the replacement while leaving the class and object allocation
unchanged. They also distinguish an unrelated global `my` from the object-local
dispatcher and a declared `variable` override from native object-variable links.
Bodies retaining an actual receiver allocation stay in their complete memo
identity until that proof has a source/frame relocation contract.

`SourceObjectState` retains independent object, class-delegate and receiver
dispatcher generations. Precise mutations issue a new dispatcher generation;
opaque dispatch withdraws all three generation receipts to absorbing `None`.
Repeated opaque invalidation preserves that unknown state, and a later precise
mutation or join with an older snapshot cannot revive a frozen receipt.

The actual manufacturer also freezes the selected grammar's method namespace
path. A current reached allocation and private-dispatcher epoch prove that the
fresh receiver namespace has no extra local commands. Lookup then follows that
helper path before global fallback; it does not use the displayed class holder.
Variable `namespace_known` remains false. A generic receiver frame or a namespace
path/unknown-handler mutation cannot use this lookup envelope.

Successful paired-list iterator entry comes from
`InvocationFacts::iteration_entry` under the actual native list grammar. Empty
inputs skip the body; invalid cardinality or a known malformed list retains an
error without body entry. A required first iteration still performs every
sequential binding store and its possible errors. Only its completed normal or
continue successor seeds the repetition join, so the initial missing-value
world cannot reappear as a zero-iteration exit. Unknown inputs keep the ordinary
may-skip flow. Array iteration uses its independent physical member inventory
rather than borrowing the paired-list selector.


### Expression result protocols and contents-only replacements

`expression_rewrite::expression_result_protocol_equivalence` compares the
retained prepared result instruction family separately from mathematical
values and operand effects. A contents-only replacement carries no native
normalisation, fresh result allocation or pooled-object reuse, so the query
refuses it. SCCP stored/returned execution constants and the optimiser's
whole-expression fold entry use this query. Partial expression rewrites keep
the native expression wrapper and still prove operand, error and value
obligations independently. Native codegen remains a consumer of the selected
compiler recipe rather than of this source-edit licence.

A pooled result can retain exact normal-completion bytes without acquiring
`SourceNativeNumericObject`. The source expression result adapter records its
native string/selected operand bytes separately; only the existing normaliser
or numeric operation, discharged coercions and current representation epoch
can mint a numeric receipt. For example, modern C `expr {2+2}` yields known
contents `4` while its reused pool representation remains unknown. Known
contents cannot substitute for the original result operation.


Conditional native output commitments remain separate from command completion.
The selected scan or regexp handler can prove `Written`, `Unchanged` or
`MayWrite` for each retained output argument without proving its physical store.
The source walker resolves those destinations in native order. It publishes
unknown bytes at the exact write site only when the live address, scalar/array
admission and observer closure prove that store succeeds. A known later output
failure preserves earlier stores on the error route; an unknown or observed
address keeps the conditional protocol. A match does not supply the captured
text, an SSA constant or an unconditional completion proof.
A conditional write into a definitely missing bounded target retains the
closed `DefinedOrUndefined` presence union. Its Must presence, read-success and
value proofs remain unknown. Unknown target addresses or observers retain an
opaque presence residual rather than gaining this union.

A selected native producer can separately retain `SourceNativeNumericShape`
when its bytes are unknown. Its private provenance distinguishes an Increment
store, a ListLength result, and an audited executed arithmetic or bitwise
expression. The authored result contracts and private prepared-expression
descriptor select their actual arithmetic or normalized-result protocols.
Exact stores publish the
shape before write callbacks, and ordinary Value formals retain only the
original frozen receipt. `contents_already_native_numeric_at` requires a live,
successful unobserved physical read. The closed per-cell shape survives a
join only when every normal branch retains an actual numeric producer from the
same engine. Matching categories retain Int or Double; differing categories
retain Numeric. Differing writer origins do not prevent this point-local
category proof, while concrete Numeric producers still require their retained
origin. Missing, nonnumeric, different-engine or opaque branches withdraw it.
The joined category carries neither a concrete value nor a frozen epoch.
`contents_native_string_access_closed_at` is a separate effect query. A
successful actual scalar read with original stock class, current native numeric
class, or the sealed closed integer-contents domain can rule out a custom
string updater/free callback. The conversion-only domain does not become a
numeric cache category through this query; index acceptance, expression
number conversion, value, completion and erasure require their own contracts.
The query accepts this shape or an independently proved concrete Numeric producer. It serves only the
sharing footprint: one currently numeric object cannot simultaneously have an
ordinary List/Dict representation. Shared coercions and opaque callbacks
withdraw the shape through the existing representation map. This does not mint
`SourceNativeNumericObject`, a constant, an object token or a frozen operand.
A separately checked numeric-only prepared tree may retain an existing numeric
shape while retiring its overlapping concrete receipts and advancing the shared
epoch. The tree rejects script, function, string and index/list operations;
this exception never applies to general representation invalidation.

An immediate native constructor result uses a distinct private validity state
from a frozen epoch receipt. When a joined loop permanently loses its epoch
high-water mark, the current constructor can still supply its selected result
shape through that native result boundary and into the final direct store.
It cannot become a frozen receipt, survive an intervening callback, or restore
an older operand's stamp.

### Retained entered-script call inventory

`SourceCommandBindings::possible_entered_body_invocations` and the realm
delegation project possible calls from the existing entered-script inventory.
Each observation retains the exact executed source and entered activation.
Deferred declarations retain source mappings without that entry licence;
dispatches from nested deferred bodies are excluded. The scanner follows
recorded child entries iteratively and queries the original origin and offset,
preserving lambda namespace selection without parsing its body again.

Interprocedural reachability consumes these bindings for possible internal call
edges. They do not establish unconditional execution, completion, purity,
effects, SSA values or executable rewrites. A missing executable FunctionUnit
also cannot discard a genuine retained normal factory candidate: the nominal
object collector reads original IR script carriers for that advisory purpose.
It still requires the selected normal factory handler and grants no physical
object or method receipt.

`possible_normal_handler_effect_regions` adds coarse read/write regions from
the selected normal handler's existing resolved effect footprint. It does not
requery a command name or reduce the strict scanner's unknown, callback,
completion or purity obligations. Interprocedural scanning uses this separate
May projection to preserve reached host-region reads when compiler entry remains
unproved.

### Original method reference sets

`receiver_identity::method_at_command` selects a retained temporal method entry
using the actual command realm, written head, evaluated selector and post-argv
receiver proof. It retains the original declaring class separately from the
actual receiver class. `method_call_spans` matches original class and method
name-word spans plus the receiver table; `method_calls_on_receiver` instead
matches the actual receiver allocation for inherited-call consumers. Neither
query uses the final class map or a lexical MRO as dispatch authority.
`method_references_for_declaration` lets an actual call retain its original
reference set after a same-name class declaration replaces the document's
current class. A computed selector may report identity but supplies no edit.

Native `classmethod` requires the actual C9.0+ worker. C8.6 needs a separately
provided package/source implementation; comments or keyword assistance are not
provider evidence. The C9 source owner retains the original delegate's declaring
class across inheritance. Native instance-side forwarding is a separate protocol
and cannot use a class-table entry as an instance dispatch receipt. Its consumer
admission declines without the original forwarding/helper owner.
Use explicit C9 positive fixtures and a C8.6-unprovided refusal, while retaining
genuine C8.6 package cases with their actual provider context. Internal receiver
and stored-prefix fixtures must enter the relevant methods through an actual
created receiver; an unreached method body supplies no live allocation by its
enclosing class spelling.

A stored command-prefix method needs an independent capture receipt. A
class-definition operand uses the separate `SourceDefinitionMethodReference`
inventory described above. `receiver_identity::definition_reference_at_cursor`
selects that original phase; a known name without a method entry definitively
declines navigation and rename instead of borrowing a later declaration.
`definition_method_reference_spans` adds proved metadata uses to the
declaration's reference/edit set. An original declaration operand remains
available to cursor navigation but is excluded from this use-only set by the
full `entry.declaration() == reference.invocation()` identity. Reference,
highlight and rename consumers add the declaration according to their own
include-declaration policy; they must not count it twice. `method_call_spans`
remains limited to actual dispatch sites, so metadata never becomes a
call-hierarchy edge. Missing or
conflicting metadata coverage and unproved stored prefixes remain whole-rename
hazards. Test both `method Ping ...; export Ping` and `export Ping; method Ping
...`, with a same-named procedure decoy, and verify definition, hover,
references, prepare-rename and the complete rename transaction together.

Standalone reached export and unexport operands are captured before the
definition transition retires the current class table. Direct invocations and
closed visibility-only scripts retain the original private worker and method
entry; each full invocation site must agree across observations. Creation
coverage remains separate. Unknown standalone coverage blocks whole rename
without erasing independently known creation operands. Retraction, arbitrary
definition bodies and class-object metadata still require their own effects
and phase contracts. A stored command prefix separately needs capture-time
receiver allocation and original selector provenance; stored bytes or a final
class map cannot supply either proof.

### Reconcile representation costs at the actual read

Shimmer consumers must use the `CommitWalker` cost queries, rather than call
`state_of` and independently select a semantic type. The latter is raw replay
state for transfer and first-use reporting. An `upvar` callback can convert a
Dict to a List without changing bytes or the represented SSA contents version;
raw replay can still report the earlier Dict. The current physical List takes
precedence and the stale first-conversion location is withdrawn.

`SsaReadRepresentationAdvice` is the internal captured-read projection shared
by word, expression and native named reads. Its three axes are independent:
one unanimous current representation, closed possible container representations,
and a current native numeric shape. `captured_read_representations` validates
successful unobserved reads, live contents and their actual physical contexts
once. Missing inventory, unknown read effects or disagreement cannot donate a
current representation. Numeric shape remains separate from the container
projection; Unknown alone never proves a numeric intrep.

Use `cost_for_word` for an original sole variable substitution. Use
`cost_for_expression` with the original parser base or retained evaluated
operand mappings. Exact spelling and source extent both matter: substituting
another variable into the same AST extent must decline. Use
`cost_for_native_read` for a selected native variable-name operand such as the
target of `incr`. This query matches the physical SSA cell against retained
post-argv native reads. The context before the statement cannot replace that
inventory: an increment amount can change the target during evaluation.

Positional representation hints do not describe every native cache route.
`NormalRepresentationInvocation::stock_length_operand_cache` selects the actual
Length protocol, while `operand_preserves_captured_cache` supplies only an
authenticated current class from the original read. Warning, replay and loop cost consumers
use this same projection: C8.6 numeric Length converts to List; C9 numeric Length
preserves its cache. Semantic Int, replay alone, missing class evidence and the
wrong operand cannot suppress a conversion. Use the paired selected-Length
cost and replay tests when changing this projection. This metadata grants no
success or erasure proof.

The same spelling check applies to word reads. `word_variable_access` routes
SSA identity, successful-read, contents, place and representation queries
through one unique original site and exact variable wrapper. A synthetic `$z`
with the source range of `$x` cannot borrow any of those projections. A token
range by itself is only location information, not an operand attestation.

The returned cost combines current type advice with reconciled commitment.
An actual List or Dict supersedes replayed state; an actual pure String clears
it. Closed container alternatives retain possible costs without setting
every-path commitment. A numeric replay commitment additionally needs current
native numeric shape. These are diagnostic facts: none can erase a conversion,
replace an operand, prove a constant or establish object identity.

When changing these functions, run the callback representation pair, numeric
shape and invalidation controls, membership/ordinary-range expression cases,
and direct/nested loop conversion spellings. Test both increment operands and
argument-side mutations of the target. Review all use-site and expression
consumers together; a source search for detector calls to raw `state_of` must
find none. Record native observations and artifact hashes separately from the
source-level regression tests, and record declines and failures as well as
passes.


### Native object-method effect receipts

C Tcl 9 list length, indexing, slicing and iteration can dispatch native object-type methods that mutate cells and command bindings. Before applying a selected handler's effect contract, use `InvocationFacts::native_list_method_requirements` with its frozen arguments, actual `NativeCompilationSelection` and independently current native numeric index positions. The ordered conservative footprint retains each original operand and its Duplicate, Length, Index, Slice or Elements obligation. Alternative branches need not execute every listed method. Independently current ordinary List/Dict evidence or the arithmetic-sequence provider's explicit `world_is_closed_for(method)` capability must close every obligation. Unknown grouped indices, nested elements and preparation routes retain a residual. Numeric index-object proof removes only its own grouped path and supplies no value or successful conversion. Unknown objects and replaced creators retain host callback effects and withdraw the reached mutable world.

`command_binding/object_callbacks.rs` records this distinction privately at the full original source invocation site. Binding joins intersect closure; missing or conflicting observations remain unknown. Both strict and normal registry projections consume the same effect receipt. It grants neither opcode admission nor normal completion, and the frozen original handler and arguments remain selected even when object effects are unknown.

Original unchanged literal operands have a separate effect-only pool proof in `command_binding/literal_object_pool.rs`. The source driver must supply a fresh authored Tcl interpreter, and the exact original word must agree with the frozen argv and its owning source. The pool identity intersects on joins and is permanently withdrawn by unknown host/object effects. Creating a later ordinary object or a new representation epoch cannot restore it. A runtime command/namespace snapshot contains no literal-pool evidence and cannot mint this proof. The proof closes native object-method effects without claiming a List representation, valid list, result, completion or opcode. Compiled literals share objects globally: a host command given an earlier compiled same-byte literal can install an abstract intrep that subsequent compilations reuse, so known bytes alone remain insufficient.

Generic foreach parses and duplicates variable-list objects at runtime; selected Inline foreach consumes variable-list syntax during compilation, so its runtime footprint includes value-list objects only. Compiler preparation is a separate phase. Unknown selection cannot fabricate runtime variable-list objects or discharge its preparation residual.

The provider shares the existing physical contents representation lifetime, including write, shared-coercion, callback and frame invalidation. It carries no bytes, object identity, allocation freshness or ordinary container representation. Its strict representation is Unknown and its ordinary List/Dict alternative set is absent. The audited stock methods close interpreter-world effects separately from receiver/cache mutation: Slice can modify the receiver and Elements can materialize a cache. Source representation preservation remains limited to the existing Length path. Native C9.0/C9.1 controls distinguish stock providers from custom Length, Index, Slice, Duplicate and Elements callbacks; Slice completion remains independent because generic C9.1 can propagate a custom Return while its compiled instruction reports Error.

### Selected stock Length cache and empty constructor effects

`InvocationFacts::stock_list_length_protocol` requires the actual ListLength
operation, authored native result contract, accepted frozen argv and selected
Leaf handler. `command_binding/container_coercion.rs` independently captures the
final original physical argument after argv evaluation, its successful current
read, contents origin and full source owner. A sealed original stock object,
current native numeric cache, or selected empty-root capability closes its root
conversion effects. Its OK/Error envelope does not prove normal completion.

Only the reached normal successor can publish `normal_cache_disposition` at the
same unchanged physical contents. Tcl 8.4 and Jim convert supported stock inputs
to List; Tcl 8.5/8.6 also convert authenticated Numeric inputs to List without
requiring known string bytes. Other original classes need independently known
nonempty materialized bytes. Tcl 9 preserves current Numeric/Boolean caches and does not promote
StockUnknown to List even with nonempty bytes. Existing actual List/Dict proofs
use their separate ordinary-container contract. A preserved numeric input keeps
only its current cache summary; concrete numeric values, frozen epochs and
executable operand receipts remain retired. Changing these rules requires
same-original-object probes across the affected handler/opcode and release, not
a semantic type hint or a primitive GetDouble policy. The 204 native cases are
retained in `tcl-syntax/tests/data/native_list_methods/stock_length/`.

`normal_empty_list_root_provider` is a separate selected normal manufacturer
contract for an actual Generic zero-argument List invocation. Inline/Unknown
compiler outcomes decline because pooled empty objects can retain earlier cache
history. The result supplies root-method world closure, no List cache, bytes,
freshness or StringAccess proof. Joining it with an actual ordinary List
intersects only those root capabilities: a joined value can be nonempty and
contain custom members. Closed selected list preparation preserves this effect
class while retiring representation epochs/caches; unknown object callbacks,
physical writes, observers and general coercions withdraw it. ArithmeticSequence
alone retains the audited readonly Length cache permission. Source SSA and
shimmer must continue to see Unknown representation for the effect-only join.
The source zero/one iteration test uses a Generic constructor and retains the
original pooled-empty invocation as an explicit residual control.

### Original expression variable identity

`ExprNode::Var.name` remains a base/dependency label. A contents consumer must
use the original `Var.text`, with the actual lexer configuration, or the exact
positioned read. `variable_reference(config)` delegates to the shared lexer and
keeps the complete braced name separate from an unevaluated array index.
`variable_nodes()` only enumerates direct original occurrences; it supplies no
execution proof. `${$b}` names `$b`, `${a(k)}` names `a(k)`, and `$a($k)` owns a
separate index-evaluation protocol. A second name decoder or base-name lookup
cannot substitute for that protocol.

The following queries separate contents identity from dependency projection.
Missing original reads or grammar cannot supply scalar contents or formal
identity from a base label.

| Consumer | Purpose and current owner rule | Limits and semantic checks |
| --- | --- | --- |
| `type_infer::{infer_expr_type,WordTypingCtx::expression_variable_types}` | map original AST occurrence extents to exact SSA read/contents receipts; `expr_base` belongs to the original statement/terminator. | `expression_contents_types_use_original_element_and_literal_name_reads`; `${$b}`, braced element and evaluated index discriminate scalar-label drift. |
| `tcl_syntax::expr::eval` | Engine walk passes original reference text and parser offset; no `Var.name` contents lookup. | Positioned-expression/source controls retain actual grammar. |
| `tcl_expr_eval::FoldOps::variable_reference_at` | actual invocation grammar supplies complete static-reference lookup independently from object/coercion proof; dynamic index evaluation declines. Explicit mathematical compatibility grants no native proof. | Complete-name/element tests cover the shared cell syntax owner. |
| `native_numeric::expression_operands` | concrete object receipts come from exact retained reads; environment keys use the proof’s actual lexer and complete static cell spelling. | Static element/sigil controls distinguish identity; index substitution requires its own evaluation receipt. |
| `sccp::incoming_expression_environment` | Incoming formal evidence additionally requires exact unobserved current Activation scalar read; name alone cannot admit an array. | Alias/static/reference-formal refusals cover physical identity; lexical keys retain the complete name. |
| `intervals::{eval_expr_with_reads,guard_constraint,refine_interval_for_value}` and `interval_bounds::collect_divzero` | transfer and divisor ranges use original node/base reads with represented SSA versions; guard narrowing additionally requires the same actual symbol/version at the branch. The base-name guard index only selects possible candidates. | `interval_reads_distinguish_elements_with_the_same_base_label` and `divisor_ranges_keep_literal_sigils_and_array_elements_separate`; unknown/missing reads yield TOP. |
| `uri_split::{expr_traces_to_uri,TraceCtx::variable_origin,trace_to_uri_family}` | URI expression provenance follows the exact original node/base read; backward copies and phi traversal retain the captured Symbol without a display-name round trip. | Root/element and literal-sigil controls distinguish identity; absent represented reads decline. |
| `common_aot_plan::closed_integer_add_expression` | actual module lexer and shared cell projection must identify an original Named formal; an indexed formal base is insufficient. | Scalar-sigil and array-out-of-subset contrasts check the admitted subset. |
| `native_integer_proof::prove_operand` | Exact source/version/Place proof; incoming parameter additionally validates current Activation scalar slot. Observer dependencies resolve at the original physical point through `externally_mutable_by`, including `escaping_var_names_at`; the base label is not observer identity. | Incoming/alias/element refusals and the literal-sigil observer pair cover physical identity; missing address/effects decline. |
| `dispatch_proof::expr_world_hazard` and native variable-name/alias hazards | Original `Var.text` retains substitution syntax. Evaluated native names and alias targets separately retain literal sigils/keys through the shared literal element-name owner. | Literal-sigil/name and alias-key controls distinguish the two input roles; ambiguous substitution syntax remains conservative. |
| `interprocedural::classify_return_expr` | original scalar cell projection uses the retained invocation’s actual grammar; missing grammar supplies no scalar passthrough. | Indexed formal and literal-sigil contrasts distinguish exact formals. |
| `interprocedural::{note_params_in_expr,walk_collect_param_refs}` | May parameter-dependency collection; may retain array-root dependency but grants no passthrough or returned contents. | Dependency collection does not establish returned contents. |
| `optimiser::helpers::expr_simplify` | conditional type candidate eligibility uses unanimous exact occurrence reads keyed by original text. Missing/conflicting reads withdraw it; self-comparison requires identical original reference text. Actual edit still requires central execution equivalence. | Differing-element and sigil controls check original occurrences; these facts supply no conversion/observer/representation proof. |
| `increment_rewrite::assess_increment_rewrite` (O114) | Exact retained Set/Expr and prospective Incr handlers; all original read/store observations retain known physical addresses and pre-RHS versus pre-write contexts. Current Integer and conversion acceptance remain separate. `increment_expression_schedule` retains original non-unit literal conversion; the bounded C bignum residual additionally keeps original shared-input normalisation. Missing/observed/unknown-cache evidence declines. | Use original-observation, unknown-stock, integer-domain join, disjoint-restoration and applied increment-pair controls. Loop precision additionally requires unanimous original read/store schedules and admissible current cache evidence; the conversion domain alone cannot grant an edit. The native cache corpus covers only its measured string/List/Integer/bignum contrasts. |
| `optimiser::code_sinking::expr_references_var_at` | Conservative dependency suppression, not contents donation; array-root dependence remains relevant. | Preserve current conservative result; actual physical sink proof remains separate. |
| `dispatch_proof::expr_world_hazard` | May observer hazard, not a contents proof. | Complete reference/element observer identity is required; missing grammar cannot prove an unobserved scalar. |
| `codegen::wasm::direct_expr_supported` | Actual `variable_reference_place(text,config)` must produce Named and equal the admitted formal; indexed references decline. | Original-word native/WASM controls check emission against the admitted scalar plan. |
| `native_lowering` and bytecode expression emission | Original `Var.text` uses the selected `whole_var_ref`/physical cell plan. | Actual braced element, literal-sigil and selected grammar corpus. |
| `bpf_tcl_ir::lower` | Original references use the actual module configuration; elements are outside the supported subset. | Scalar-sigil and array-out-of-subset source tests check the subset boundary. |
| `ExprNode::{vars_with_config,vars_parsed_only}` | Base/dependency collection, intentionally distinct from contents identity. | Parser/name-field tests check dependency labels without contents donation. |

Jim expression substitution belongs to word decomposition. `WordPart::Expression`
borrows the original parenthesized expression; `scan_expression_sugar` selects
only the actual Jim variable grammar and preserves the original token extent.
Evaluate it through the expression engine, independently of command lookup for
`expr`. A component is neither an array reference nor a fabricated command
script. Keep its lexical source for diagnostics, and preserve host refusals
before publishing any guest completion. Source assistance that cannot prepare
this expression must retain an Unknown residual, with no body or read proof.

A Runtime build without its expression engine stops a reached sugar component
with `NativeValueAccessRefusal::ExpressionEngineUnavailable`. This is a backend
capability refusal, not a Jim syntax error or a compiler preflight result. The
pending host channel bypasses guest capture and preserves earlier effects. Do
not treat this branch as a native guest outcome.

The shared word-component corpus contains twelve original variable specimens
for five C releases and pinned Jim. Additional Jim controls cover direct
expression entry, replacement of the command named `expr`, ordered callbacks
and original parenthesized diagnostics. An unavailable expression backend
remains a separately tagged refusal rather than a native guest outcome.

### Keep authored logical numeric simulation explicit

An F5 logical simulator installs
`AuthoredLogicalNumericSimulation::Tcl84Core` deliberately. Resolve it with
`InvocationDialect::authored_logical_numeric_simulation` on the retained logical
invocation dialect; conflicting family, core-point, compatibility-version or
numeral axes decline. A physical C9 host engine, broad command surface, compatible
C8.4 version or absence of native getter evidence cannot install this capability.
Authentic `native_scalar_getter_protocol` and `scalar_numeric_input_policy` stay
None for vendor profiles. The installation belongs in interpreter and compiled
cache policy stamps and child-interpreter capability propagation.

The pure Syntax protocol reads complete original bytes. Number and Integer
stages retain `Number`, including exact large integer magnitude; the authored
wide-arithmetic owner separately applies its width and overflow rules. It does
not use Jim's first-NUL extent or a primitive C getter cache recipe. Boolean
conversion returns `LogicalBooleanOrigin::Word` or `Numeric(Number)` with its
logical value. `BooleanValue` serves command options/value conversions and
rejects fresh NaN; `NumericTruth` serves reached conditions, logical operands and
unary negation and retains the explicitly authored C8.4 NaN truth behavior.
`current_number_boolean` separately consumes an independently current numeric
cache, retaining cached NaN truth in both stages without string preparation or
native intrep conversion. These are
logical simulation declarations, not observations of vendor hardware.

`LogicalNumericSimulationFailure` retains invalid-number, integer, boolean and
NaN-value distinctions. The simulation completion owner preserves the original
object and full diagnostic bytes, settles its logical error and cache policy,
and keeps these failures separate from `NativeScalarGetterError`, materialization
obligations and native Unchanged/Set receipts. A parsed integer magnitude is not
permission to install a wrapped native Int cache. Known cache fast paths require
the same explicit installed logical capability before use.

Focused controls are `cargo test -p tcl-syntax --lib
logical_numeric_simulation::tests` and `cargo test -p tcl-registry --lib
logical_numeric_simulation_requires_explicit_consistent_f5_contract`. They check
full-byte/NUL refusal, legacy radix grammar, exact magnitude, Word/Numeric and
NaN-stage distinctions, and conflicting logical/native selection axes. These
pure tests do not attest F5 native execution or physical getter storage.

The VM installs this contract with `Vm::set_logical_numeric_provider` only after
an explicit physical engine is installed. Child interpreters and compiled-cache
and frozen-activation policy stamps retain the capability. `NumericContext`
passes the selected logical dialect and capability through inline arithmetic,
expression evaluation, conditions, portable getter/increment adapters and fixed
math operand preparation. Parsed strings do not acquire native Int/Double cache
receipts. Current numeric objects use the separately authored current-number
Boolean stage; current NaN truth differs from fresh BooleanValue spelling.

An explicit framework host activation selects the host expression grammar and
logical host numeric policy and source-script lexer for that activation. It
restores F5 source and expression grammar and simulation on return.
`host_activation_preserves_separate_authored_numeric_and_expression_policies`
contrasts legacy octal/wide/UTF16 behavior with host C9 decimal/bignum/scalar
behavior; `authored_numeric_provider_is_explicit_and_scoped_to_logical_activations`
checks absent providers, fresh/current NaN, child/cache propagation and foreign
axes. These controls verify authored simulation behavior, independently of
vendor-hardware agreement.

### Primitive scalar conversion APIs and consumer limits

The pure selected getter protocol describes parsing, cache transitions and error
presentation. Concrete object adapters independently supply native object identity,
cache mutation, error state and backend admission. Parser agreement does not
establish expression equivalence. An engine-interface `NativeScalar` or
`Resident` payload transports cache and string data into a reconstructed object;
it does not supply a live reference or write mutations back to the sending
engine's original aliases. The C shim's local getter state therefore cannot
close original VM object-effect obligations. The consumer boundaries are:

| Consumer boundary | Owner route and retained obligations |
| --- | --- |
| VM `Value::native_scalar_getter` and Runtime `typed_value::native_scalar_getter` | Select the primitive getter on the actual engine; retain original storage, cache kind and full cache magnitude. Generic arithmetic number parsing and the common Boolean word table cannot replace release-specific primitive behavior. |
| VM/Runtime `ValueOps::{as_int,as_double,as_bool}` | Consume the physical adapter; retain failure origin through the command API. `ValueError::NotIntegerBytes`, `NotDoubleBytes` or `NotBooleanBytes` alone cannot reconstruct cached-versus-fresh failure, cache-on-error or Unchanged error state. |
| Runtime `tcl_value_get_*` and generated increment-result extraction | Use the actual current interpreter and apply the selected update before publishing the ABI completion. A fixed-code `TypedError` cannot represent Unchanged error state or retained getter origin. |
| Runtime `tcl_codegen_value_try_{wide_int,double}` | Keep the speculative purpose explicit: measured cache changes still apply on guest conversion failure, without publishing that guest error; missing native dependency or materialization remains a host refusal. |
| Direct VM getters in `command`, `cmd_try`, `cmd_chan`, `cmd_binary`, `cmd_namespace`, `cmd_dict`, `cmd_math` and interpreter debug | Engine-specific primitive calls require a selected adapter; unselected `Value::as_*` supplies no such contract. Return codes, Int-width fields, expression functions and conditions retain their own selected protocols rather than borrowing Wide extraction. |
| Direct Runtime Boolean consumers in dict filters, namespace prefixes, interpreter debug and ensemble options | Pass the actual interpreter/object to the selected adapter. Do not discard metadata into message-only errors or default a failed conversion silently. |
| Command-core `format`, `clock`, `info`, list/string counts and concrete binary format adapters | `ValueOps` supplies the shared object-access boundary; each caller still owns its actual Wide-versus-Int width and command-specific continuation. Formatting arbitrary integer magnitude is not Wide extraction. |
| Text-only `sort::{parse_wide,parse_real}`, lsort/lsearch, `switch::wide_int`, timerate/count helpers and byte-only binary format | Text-only helpers have distinct native conversion purposes. Object-aware paths preserve original key objects; explicitly bounded byte-only helpers cannot claim physical cache/error behavior. Default numeral grammar, Rust float parsing and hardcoded diagnostic codes do not supply an actual getter contract. |
| VM/Runtime ByteArray string generation and C character operations | Materialize via the selected storage recipe and share `NativeTclUtf` at native character doors. Standard UTF-8 `char::from` generation and Rust Unicode-only counting do not implement modified-NUL or native surrogate/invalid-byte units. `try_as_str` remains a checked host projection. |
| VM/Runtime error and completion adapters | Keep typed failure origin and Unchanged/Set until the actual interpreter state applies it. Absent metadata or assigning `NONE` is not Unchanged. Apply primitive-versus-Eval clipping at the correct propagation stage. |

The adapter sequence is:

1. Select the actual native engine, getter kind and independently retained object
   storage; inspect the actual cached representation first.
2. If no cached recipe applies, obtain the original native materialized string and
   any required retained Jim range state. Missing state is host abstention, not a
   guest Invalid parse. Rust process errno cannot supply retained native Jim range state.
3. Fulfil the returned original-string preparation **before** changing the cache.
   Apply the complete cache value even when the getter outcome is an error.
4. On failure, render from its retained origin and original materialized bytes,
   then apply Unchanged or Set to actual interpreter state. Publish primitive
   bytes or the separately measured Eval propagation at the reached stage.

`native_scalar_probe` uses the same original-cache preparation and adoption as
`native_scalar_getter`, returning an outer host refusal and an inner native
outcome. It does not render guest failure. Use it for native NULL-interpreter
probes: a failed cached Double or NaN conversion can leave its string absent.
Discarding an already rendered getter error cannot recover that storage state.
The regular getter renders the failure only when the consuming native stage
requires a diagnostic.

`ValueOps::native_char_len` is the checked native count door. It preserves a
capability refusal as `ValueError`, retains original String/Jim counts, and
selects ByteArray shortcuts before materialisation. `try_char_len` and
`try_as_str` are checked host Unicode projections. `native_unicode_units`
retains the selected C native units independently of resident bytes. The
physical `native_object_snapshot` exposes primary cache, storage identity and
resident string without invoking an updater; its String class comes from a
real retained descriptor, never from byte validity alone.

Completion keywords retain `CompletionCodeCache::TclKeyword`; Jim installs
`CompletionCodeCache::Jim` after a reached successful conversion. These caches
are distinct from numeric caches. Jim's return-code descriptor has no string
updater and can keep an original integer's string absent. Checked string access
returns a typed unavailable-updater refusal; it must not fabricate numeric or
empty spelling.

The shared transport retains this sequence: `ValueError::NativeScalarGetter`
heap-owns the selected protocol, getter kind and cached/fresh failure origin.
`CmdErrorDetails` consumes exact result/info/line bytes, boxed primitive record,
and `CmdErrorCodeUpdate::{Default,WrongArguments,Unchanged,Set}`. Unchanged performs no store,
including no write traces; saving and assigning the previous bytes is not the
same operation. Default supplies the neutral NONE update. WrongArguments carries authenticated
arity failure and is resolved through the selected command policy; guest message
bytes cannot author that receipt. Result-only conveniences and positive
`error_code_bytes` projections cannot author a complete native completion.

Expression invalid-type failures retain a separate
`NativeExpressionInvalidTypeErrorCode` from the actual dialect and reached
conversion stage. C84 direct `Tcl_ExprObj`/`Tcl_ExprBooleanObj` leaves seeded state
unchanged, while subsequent Eval propagation sets NONE. Later C retains its
selected structured stage code. NaN/domain/range/parser failures cannot borrow
this record; Jim's independent direct API is not authored by it. The seeded
90-row same-command fixture is in `native_scalar_getters/errors/expression_stage`.
Dynamic raw variable input and literal expression-source NUL are distinct native
stages; this fixture cannot donate parser equivalence or primitive acceptance.

Keep expression `GetNumber`, integer-index conversion, implicit math, unary and
condition conversion, strict `string is boolean`, scanning and lseq separate.
Primitive NUL acceptance, Boolean grammar, numeric width and error codes do not
license those stages. Cache subtype also stays separate from canonical string
production and alias/object effects.

The durable native fixtures live in
`rust/tcl-syntax/tests/data/native_scalar_getters` and
`native_numeric_operand_conversions`; their READMEs give native build commands
and manifests retain original sources, libraries and result hashes. Registry
`native_numeric_conversion::tests` and
`primitive_getter_selection_rejects_conflicting_actual_axes`, plus Syntax
`scalar_getter::tests` and `native_tcl_utf::tests`, cover the selected contracts.
To run focused Rust controls, use
`cargo test -p tcl-registry native_numeric_conversion::tests` and
`cargo test -p tcl-syntax scalar_getter::tests`; run the native fixtures against all
six pinned engines before claiming concrete adapter agreement. Physical adapter
controls must retain the same original alias, raw String/ByteArray distinction,
cache-on-failure, prior error state, native unit behavior and propagation stage.
A host refusal never counts as a matching native guest outcome.

### Native scalar math results and original argv addresses

Selected source argv reads carry a private heap-backed receipt after each original
word finishes. It retains the original source, spelling, word coordinate, selected
physical address and read world. Expansion elements and captured prefixes do not
borrow another word's receipt. Native container coercion, object-method closure,
range results and numeric index sharing consume that receipt and independently
check current contents origin, full source attestation, lifetime and observers.
The address is never looked up again through a later alias or index value.

A scalar math result category comes from the authored actual native implementation
protocol, after operand execution and before command leave callbacks. The
Sqrt recipe supports C Tcl and Jim; Double supports C Tcl. Each requires one
original operand and proves a Double only on its native normal path. Jim
numeric-unary double requires its separate conversion protocol. A fixed-table engine
requires its actual registration identity, arity and interpreter/table receipt;
a mutable wrapper requires the reached builtin binding. Names and semantic result
types do not supply either proof. C numeric operands retain their category during
GetDouble; Jim conversion weakens prior integer subtype evidence to Numeric. Concrete
numeric operands and frozen receipts still retire. The new result is created after
that conversion and retains its independently proved category. Current read advice
uses `contents_native_numeric_category_at`, with closed alternatives agreeing on
Int/Double or joining to Numeric. Unknown input or callback worlds cannot borrow a
subtype from an earlier SSA definition. Native GetDouble may run a custom
operand object's string updater even when sqrt returns normally. The source
world therefore requires current native numeric input or exact original fresh
literal-pool provenance; an unknown formal or an outer List/Dict representation
cannot close this effect. The selected normal Double result is independent of
that input-effect obligation. Shared invocation facts retain the residual and
withdraw purity and frameless effect claims when it remains unproved.


A private frozen RHS may also retain `SourceStockLiteralObject`, minted only
from the still-closed original source literal pool. The normal exact store
places this effect-only class receipt in the existing physical representation
owner. Current read success, cell lifetime and source attestation remain
independent obligations. Its initial strict representation is Unknown and it
supplies no numeric value, contents, object identity or allocation freshness.
Selected ordinary list/dictionary and numeric-only conversions may keep the
stock class while retiring epochs and concrete receipts; ordinary representation
alternatives remain cost advice. Unknown effects, overwrites and missing join
receipts withdraw the class. Native scalar `llength` conversion can invoke
custom string/free-intrep hooks on every C release, so its operand class gate
is separate from the Tcl 9 abstract-list method protocol.

Implicit math inventories retain operand effect closure independently of command
or fixed-table implementation identity. Executable math erasure must consume
the [shared erasure gate](#prepare-expression-functions-using-their-native-owner);
successful value analysis cannot silently promote it. Command observer
boundaries withdraw the captured input class proof.

### Closed BPF control syntax

A BPF frontend selects `BpfOpKind::Conditional` and asks
`CommandRegistry::bpf_conditional_operands` for the shared validated clause
positions. This purpose is static-language syntax. It does not establish a live
Tcl handler, an interpreter frame, normal completion or a native compiler hook.
The BPF subset rejects command replacement and factories, computed heads,
expanded arguments and dynamic condition/body words. It checks each original
literal against the actual source slice before assigning an affine condition or
body base. Nested expansion uses the unchanged `source_nesting_limit` and an
iterative worklist. Independently lowered body execution receipts are discarded;
only syntax and original spans enter the BPF CFG. A conditional left unexpanded
has an explicit subset diagnostic, including malformed and out-of-bound source.

The checked expression comparison contract separately owns byte equality and
native character ordering. Jim `eq`/`ne` and membership compare complete bytes;
numeric equality fallback and ordering use the actual object's cached native
character count. A conversion can withdraw that count without changing bytes.
An unavailable character model, failed Unicode projection or access outside the
retained storage is a typed host refusal. Runtime expression adapters inspect
that tag before publishing guest message/code bytes, including generated calls
and boolean contexts; guest `catch` and finally cannot consume it.

### Definite role grammar and possible name navigation

Use `invocation_argument_role_assistance` only to discover possible original
operand roles for navigation. It preserves opaque alternatives and unloaded
catalogue or declared assistance. Use `invocation_argument_role_consensus` for
a definite malformed-formal diagnostic: every actual retained implementation
must provide a complete accepted argument layout, and the original role must
agree across all candidates. Alias prefix values have no editable source index.

A possible `CommandName` role records `PossibleConsumedName` with its original
consuming point and `rename_safe = false`. It can support navigation, but cannot
establish a required command lookup, W123 absence or a rename. A unanimous
actual role records `ConsumedName`; a callback declaration remains a separate
`DeferredReference`. Preserve those purposes when rebasing a body fragment.

Jim's explicit double getter is distinct from arithmetic's integer-first
preparation. On an existing integer object it installs a coerced-double cache
that retains the exact wide integer and the integer string updater. The native
small-integer branch avoids generating bytes; larger integers first preserve
their exact spelling. A later wide getter restores Int on that same object,
including through aliases. Do not replace this cache with a rounded floating
object. Fresh string-to-double grammar is a separate selected protocol: Jim's
decimal-integer attempt followed by native floating parsing must not be replaced
with the general base-zero arithmetic parser.

### Original numeric conversion hooks and reached Increment inputs

`InvocationFacts::numeric_object_conversion_arguments` consumes the authored
`RepresentationEffect::CoerceNumericValues` layout, exact accepted original
argv and actual native engine. Its output inventories original scalar object
hook obligations; it provides no numeric acceptance, result category, Leaf
handler, compiler choice or purity. Generated math rows retain their body
world descriptor separately. Both direct handler preparation and implicit math
calls project the same original-object closure into the private callback-effect
inventory; `object_callbacks::refine_facts` removes effect/purity authority when
that proof is missing. Unknown updater/free effects withdraw source worlds even
when the already selected implementation and its normal result remain known.

`InvocationFacts::increment_object_protocol` requires the selected Increment
operation/result contract, exact accepted one/two operands and actual engine.
It retains amount conversion BeforeRead on C8.4/Jim and AfterRead on C8.5+, plus
the separate missing-read policy. The amount uses its frozen original argv
object. Old contents use the protected physical receiver after each completed
read continuation, never the variable-name alias again. The private
`captured_numeric_input_hooks_closed` checks that live lease, contents generation
and sealed current object class without executing or querying a second read
observer. Actual Numeric, pure native String, issued stock-object or closed
integer-contents class can close its numeric object hooks; ordinary outer
List/Dict, semantic Int and byte spelling alone cannot.

The normal numeric store remains independently authored and is published before
write callbacks. Open old/amount hooks can change cells, commands, namespaces or
observers, so they retire source worlds before the remaining store phases;
normal arithmetic does not restore those worlds. A completed read callback that
installs a current native Integer may close the subsequent input conversion,
while future read/write observers retain their own obligations. Missing and
retired receivers continue through their existing typed native failure routes.

The durable `native_scalar_math/operand_hooks` fixture records 24 Increment,
96 implicit math and 20 implementation-selection timing observations across the
six engines. Custom updater/free hooks mutate current locals and command tables
while producing normal numeric results. Replacing sqrt during input conversion
changes subsequent dispatch, while the current call retains its previously
selected implementation. Keep these effect and identity purposes separate when
changing source, SSA/shimmer, folding, codegen or runtime consumers; do not reuse
primitive Wide/GetDouble cache policy for the reached Increment/Expr stages.

## Physical string recipes, list length and usage headers

String materialization, object list length and wrong-argument presentation have
separate selected owners. A numeral grammar or a compatible Tcl release does
not select any of these native protocols. `InvocationDialect::execution_point`
retains the actual engine and build; `native_string_protocol` authenticates its
supported string recipe. Logical F5 simulation uses an explicitly requested
provider and retains that logical origin independently of native authority.

| Purpose | Shared owner | Required adapter evidence |
| --- | --- | --- |
| Resident string access | `NativeStringInput::ResidentString` | Original exact bytes; no Unicode projection or regeneration |
| Pure ByteArray update | `byte_array_string_recipe` and `NativeStringProtocol` | Supported recipe selected before backing allocation and retained with that backing |
| Physical object list length | `native_object_length_protocol` / `object_length_protocol` | Exact original stock class and canonical resident-empty storage identity |
| Conditional selected Length cache summary | `InvocationFacts::stock_list_length_protocol` | Selected handler, accepted frozen argv, original stock class and independently reached normal continuation |
| Wrong-argument header | `native_usage_protocol` / `usage_protocol` | Retained original header words after the actual dispatch rewrite |
| Cached index header expansion | `NativeUsageWord::{CanonicalIndex,RewrittenIndex}` | Authenticated original index cache and whether the word belongs to the restored ensemble prefix |

`InvocationDialect::native_string_materialization` issues the general object-string
recipe for actual C Tcl or Jim, or the explicitly authored F5 Tcl8.4 core
simulation. `ByteArrayStringRecipe` is the narrower C-only updater capability.
List parsing, dictionary parsing and character counting use the general owner;
Jim's lack of a C ByteArray type does not exclude its native string/list parser.
Source lexer overrides do not select these original-object conversion rules.

Jim List parsing pins original Source information before string materialization.
`Vm::native_object_list_elements_in` and Runtime's checked List conversion
associate the original object with its actual interpreter through a weak context
receipt. Handler argv, return-option lists, formal lists and variable-store values
use that ingress; a low-level protocol-only call cannot invent the interpreter.
An expired or foreign receipt causes a typed host refusal. Association adds no
object or filename ownership.

The Source primary owns the original filename object and signed line. Each parsed
List member owns a Source primary using that same filename identity. The List
scanner supplies native line deltas from its existing escape and element walk;
backends add them to the retained signed baseline. Filename access does not force
its string or numeric cache. Selected Jim header duplication handles resident zero-length storage before any
primary-specific hook, producing canonical empty NULL storage and no filename
owner. Other Source duplicates retain the real filename object. Retiring Source
releases that owner outside the object's primary borrow.

Jim expression preparation carries a separate cache action from its displayed
failure. Missing wrappers and empty expressions preserve the original primary;
lexical or tree rejection installs rejected Expression storage. The fixed-function
preparation receipt comes from the same token/tree pass and actual function table.
Consumers must not select cache replacement from diagnostic text or successful
parsing alone.

The Runtime ByteArray backing stores its sealed `ByteArrayStringRecipe`.
Duplication copies the recipe, raw payload, conversion class and proper-byte
marker. The native updater therefore needs no current interpreter or thread
local selector. C materialization emits native modified UTF bytes: raw byte NUL
becomes `c0 80`, and bytes `80` and `ff` become `c2 80` and `c3 bf`. Resident
strings remain authoritative, including literal NUL, modified NUL and opaque
bytes. Jim has no C ByteArray updater; its binary format/scan byte values use raw
string storage. Missing recipes cause a typed host refusal before allocating a
ByteArray or replacing the guest result.

For the supported `encoding convertto utf-8` path, call
`tcl_cmd_core::encoding::convert_to_utf8(ops, original)`. It requires actual
Native C8.4–9.1 authority, reads the original through its selected string getter
and delegates result construction to `ValueOps::native_external_utf8_result`.
Each backend independently authenticates the matching C ByteArray recipe.
`external_utf8_octets` is a pure byte codec; equal output bytes cannot grant
result storage or worker availability. Preserve C9 strict-error character
index and byte offset independently, with their typed errorCode and String
result producer. The retained
[object-storage observations](../analysis/name-resolution-proofs/encoding-original-utf8-convertto-object-storage.md)
and [error-coordinate observation](../analysis/name-resolution-proofs/encoding-original-utf8-convertto-error-offset.md)
record the fixed inputs and release differences.

Only the UTF-8 output target is modelled by this conversion path; other output
targets return a typed refusal. `encoding convertfrom` remains a bootstrap
passthrough without external-decoding semantics. Jim's separate encoding
extension capability is not supplied by the C recipe or a registration shim;
the tested initialized Jim image reports `encoding` unavailable. Unsupported,
foreign and authored-only result issuers refuse before physical construction.

Canonical empty-string storage is a physical identity, not `bytes.is_empty()`.
VM `NativeStringStorageIdentity` distinguishes CanonicalEmpty, Allocated and
Unknown. A selected native constructor supplies its actual receipt through
`Value::new_native_string_bytes`; `from_native_string_bytes` delegates to that
owner. Imported bytes use `from_string_bytes` and retain Unknown. Transported
resident storage and caches are reconstructed with
`from_native_scalar_cache_with_storage`; modern word-Boolean descriptors require
the original resident string and exact original release. Unknown empty storage
refuses any operation whose selected native action depends on singleton identity.

`Value::native_list_constructor` applies the selected constructor rather than
inferring a primary cache from an empty list of Rust members. C's zero-element
constructor produces canonical empty NULL-type string storage; Jim produces a
pure List. `list_with_native_canonical` reconstructs explicitly transported List
cache state. Cached compound inspectors retain member identities without invoking
list or dictionary conversion.

The original-object bridge uses `native_object_identity` only for equality and
lifetime tracking. It captures `native_object_is_shared` before retaining its own
capability handle. `adopt_native_object_representation` writes checked getter and
string effects onto the same original object, including effects preceding guest
failure. It does not perform COW. Unknown engines, foreign cache origins and
unsupported opaque primary caches retain typed refusal; a numeric carrier does
not represent a completion-code or frame-reference cache.

`obj::has_canonical_empty_string` inspects the central resident-buffer owner.
The owner preserves that identity across duplication and handles append,
shimmer and destruction without freeing the shared empty buffer. C8.4–8.6
ByteArray updaters allocate their empty string buffer. C9 ByteArray updaters use
canonical empty storage. Both representations have zero bytes, but their next
list-length conversion differs.

`NativeObjectLengthProtocol::action` selects `CachedList`, `Constant`, or
`ConvertToList` before reading the string. C9 tests canonical empty storage
before its native length methods; numeric and word-Boolean caches otherwise
return one without materializing or replacing their backing. C8.5/8.6 retain an
existing List, then honor canonical empty storage; other supported stock caches
convert. C8.4 and Jim convert numeric inputs to ordinary List storage. Empty
bytes alone cannot justify the preservation shortcut. Unknown stock classes,
unsupported storage and unknown engines retain a host refusal.

### Original-object append and publication

`InvocationDialect::native_object_append_protocol` authenticates the actual
engine or an explicitly authored logical provider. The pure shape recipe chooses
adoption, duplication, binary backing append, native String append or preservation
before materialization. C9 empty checks use `native_c9_string_emptiness`, shared
with physical comparison. Canonical empty identity, proper ByteArray backing,
pure Dictionary and canonical List state remain independent inputs.

`tcl_cmd_core::native_append::append_object` owns source/receiver conversion
order, COW preparation, native character-count combination and Unicode versus
byte append. Concrete VM and Runtime adapters supply original physical snapshots,
updaters and cache setters. Missing C receivers adopt the original source object;
existing empty C9 receivers duplicate its representation into a distinct receiver.
Binary shortcuts use backing bytes without generating a source string. C String
Unicode backing uses `NativeTclUtf::encode_units`: modified NUL and individual
surrogate units retain the selected release's encoding and unit width.

`PreparedAppendValue` retains the selected operation's ownership receipt.
`append_continuation` applies only within the same Jim batch, with no intervening
publication, callback or additional owner. It does not repeat Jim's original COW
decision or guess a refcount discount. C publication consumes the receipt after
one operand. `append_operands` publishes each C operand and runs its write
observers; Jim publishes the complete batch once. With operands, append does not
run read observers. The no-operand form remains an ordinary checked read.

Both command adapters retain the physical variable receiver across C write
callbacks. A write observer can replace the contents used by the next operand.
Unset/recreate behavior follows the retained cell and binding owner. The Runtime
releases protective working-object references before observers and sets the
command result after the final operand, preserving native COW observations.
Fixed native fixtures assert source/receiver primary caches, resident strings,
original identities and the separately reached result projection.

The VM and Runtime `ValueOps::list_len` adapters consume this protocol. Full bignum
magnitude, NaN payload bits and word-Boolean cache identity remain intact when a
constant-length method applies. This protocol does not change the distinct
`list_elements` purpose: an operation that needs member objects must obtain
those members. Dictionary-to-list conversion reads the actual dictionary
members directly, retains their original objects before releasing dictionary
backing, and leaves an absent string representation absent. Converting a
Dictionary through serialization would lose those physical guarantees.

The usage owner renders original byte operands. C8.4 appends CString extents;
C8.5/8.6 keep the first word raw and scan later words independently as list
elements; C9 scans every original word independently. Hash protection therefore
applies to every scanned word, not only the first position in an ordinary
serialized list. Jim's generic presenter retains length-delimited raw words.
An authenticated normal C index-cache operand appends its expanded CString
without quoting. An index expansion restored by an ensemble rewrite goes
through the selected element scan. These classes cannot be inferred from the
word's spelling.

`CmdError::wrong_args_bytes` and `with_error_code_bytes` preserve byte-exact
messages and structured codes. Runtime NamespaceCurrent/NamespaceCode and
Binary/Dictionary headers, and the corresponding VM Namespace/Binary adapters,
consume the selected presenter. Unsupported presentation returns a host refusal;
there is no raw-word or list-rendering fallback. The shared dictionary dispatcher
accepts its selected usage prefix as bytes. Static helper syntax remains a
separate contract from retained invocation-prefix presentation.

Jim compound-command ingress has a separate CString command-head construction.
The generic usage presenter must receive the operands produced by that ingress;
it cannot infer the transformation from a written command name. The monolithic
Jim Namespace adapter does not model that compound-head construction. Generic
Jim procedure usage and native compound usage therefore require distinct ingress
evidence when original command words contain literal NUL.

The physical Runtime list-length test consumes all 204 rows in
`native_list_methods/stock_length`, checking native completion, cache class and
string presence. Additional tests retain dictionary member identity, full
bignum storage and NaN payload bits. ByteArray tests cover selected updater
recipes, raw versus resident storage, allocated versus canonical empty buffers,
duplication, append and refusal without guest-result replacement. Native scalar
getter tests cover original value/cache transitions and Boolean-to-Wide conversions.
Usage tests cover raw NUL, modified NUL, opaque bytes, per-word hash quoting,
normal versus rewritten index expansion, and explicit logical-provider origin.


`list::concat_selected` is the common original-object concat entry for builtins,
compiled stack operations and concatenated script consumers. The actual physical
engine selects `ConcatPolicy::Tcl(version)` or Jim's representation-sensitive
recipe; a logical source profile does not authenticate a List cache. Adapters
supply `NativeConcatListShape` without conversion, then perform only the selected
copy, first-member getter and borrowed-member append. C8.4 admits pure Lists;
C8.5+ admits canonical Lists or empty string operands. C8.6+ checks each later
first member for `#` before appending, and drops the partial result before string
fallback. C9 abstract length/index hooks retain their genuine original headers;
a first abstract input duplicates that header without GetElements, while a later
Index result remains a fresh rc0 temporary until its native bounce point.
The Runtime supplies its actual arithmetic-series primary for this path; the VM
has no corresponding actual primary, and an object snapshot cannot supply
`Indexed` authority.
Jim's all-List path owns new backing and does not borrow C canonical rules.

The string path gets originals in native allocation and assembly order. C8.5+
results have an unknown-count String primary with allocated resident storage,
including whitespace-only empty results; C8.4 and Jim retain NULL primaries.
Successful concat can return malformed List text, so its generic result contract
is String and supplies no unconditional canonical-List fact. C9.1 folded nonzero
concat retains the exact private constructor result through `PrivateConcatString`
when the native producer creates that String cache. C9.1 compiled zero-argument
concat uses the ordinary registered empty literal. Representation tests compare originals before
observer getters, including native child references and backing identity; byte
result equality alone cannot establish those contracts.

`InvocationDialect::native_object_cat_protocol` selects C9 `TclStringCat`
independently of append. `native_cat::concatenate` inspects original binary,
Unicode and resident-string state before choosing its representation. Its byte
path retains pending updater evaluation, its first and last effective operands
retain their original identities, and its in-place permission observes original
sharing before acquiring a result handle. C9 dictionary append combines several
source operands through this door before appending once. A sequence of ordinary
append calls cannot establish those representation or identity effects.

`PreparedNativeDictionary` seals a receiver's native COW decision and conversion
recipe. Shared `native_dictionary` path operations borrow members after parent
preparation, preserving original keys and the reference escalation caused by a
parent copy. Working-handle references cannot be subtracted from arbitrary
getter clones to justify mutation. `native_list_append_elements` retains original
members and observes original sharing before List conversion; a dictionary
lappend with no new operands leaves its existing member representation untouched.

Native Index cache carriers retain an owned `NativeIndexTable` reader, byte
stride, selected entry and exact C release. The table's equality key grants no
pointer access authority. `from_native_index_cache` and same-object mirror
adoption authenticate the actual C origin; absent resident spelling uses that
retained table's canonical-word updater. `from_native_string_cache` preserves
actual String units/counts or Jim's retained optional count with independent
resident storage. Neither cache is reconstructed from equal string bytes.

Native trim consumers retain the selected `NativeStringTrimInstruction`, not a
command-name test. Its compiler visits the original subject before the explicit
character operand or registered default literal. At execution,
`string::compiled_trim` converts the character header before the subject header;
a no-cut result preserves the same subject, while a cut creates a fresh
NULL-primary string object. Interpreted C `string::trim` always creates a fresh
result. `native_string_trim::trim_range` supplies counted native UTF byte
boundaries and `default_trim_set` selects the release-specific literal, without
issuing cache, header or compiler authority. C8.4/8.5 and Jim retain their genuine
generic compiler paths. Both executable consumers use the same Core result
owner; preserve the original operand holds through result publication. The
six-engine paired controls inspect primary, resident spelling, reference count
and subject identity before any observer getter.

The whole-body compiler replay owner captures actual root/limit state and an
actual command-owned procedure namespace independently of source labels.
Runtime first materializes the original local literal array and applies its
ordered actions, then validates the complete compiler stamp after callbacks.
A selected replay drops that array and first-pass executable, literal and
auxiliary preparation while retaining the same local table. Only the final
array belongs to the installed body. Root and enabled-limit facts select only a
reached compilation pass; changing them does not invalidate warm native Bytecode
or reallocate its retained local table. Consumers use `native_compiler_replays`
and the common pass owner; they cannot substitute extra per-command anonymous
slots or install discarded-pass source Bytecode.

`info::command_list` obtains the original pattern through the selected string
getter. C namespace qualification consumes its CString extent before the
independent `InfoCommandsSearch` matcher chooses exact table-key comparison or
a native UTF scan. A raw FF key and a UTF-8 U+00FF key are distinct exact keys;
a wildcard or bracket scan can match both. Compatibility Unicode matching
cannot supply this native lookup or scan. Jim uses its flat-table namespace
helper and counted original pattern, with its own lookup and matching recipe.
The 110 paired object-vector controls keep original raw-String or genuine C
ByteArray patterns through dispatch and inspect exact result name bytes.

C switch option consumers call `tcl_cmd_core::switch::parse_options` with the
actual C release and use `switch::usage` for inline and single-list arity
failures. C8.4 scans every leading option and accepts repeated modes with the
last mode selected; C8.5+ leaves subject/body operands outside that scan and
rejects repeated modes. Original option headers still pass through the shared
Index lookup door. Preserve release-specific usage and error metadata when
changing either adapter; the all-six native switch controls exercise both
successful mode selection and failed option/arity paths.

The C handler materializes original pattern and subject bytes before selecting
its CString comparison extent. Final `default`, Exact and Glob comparisons,
and the body `-` discriminator stop at resident NUL on every supported C
release. Use `native_glob::equal_c_strings` and `match_c_string_glob` for those
entry points, and `switch::body_is_fallthrough` for the checked original body
getter. They preserve opaque resident bytes and selected native units without
requiring a Rust Unicode projection. Regexp consumes the original counted
objects; Integer retains its own original conversion purpose. These handler
recipes do not replace compiler STR_EQ, STR_MATCH or jump-table recipes, which
have independently selected object and operand extents.

Jim option and matching consumers select their actual owners before conversion.
`native_jim_switch_protocol` selects the core two-byte option scan; it does not
install an Enum primary. `native_jim_switch::select` borrows the original command,
subject and current case-list members. Default and dash checks reach the genuine
ComparedString door on those same headers. A regexp comparison creates its own
fresh command head, and the regexp `--` flag remains set when a later option
selects `-command`. After every match callback, including failures, refetch the
same case List before reading its current body. Reset the actual Jim empty-result
header before evaluating that original body or returning no match.

`native_jim_lsearch::lsearch` instead uses Jim's exact Enum option table and default
exact mode. Convert original Index operands before descending into Lists. Its
adapter retains the native search List and selected command head, borrows child
headers without extra native references, and publishes the result before releasing
those holds. `invoke_jim_match_command` returns the full integer and actual
completion: switch treats a nonzero match as true and follows the native negated
return-code convention; lsearch ordinary equality requires one, and its Boolean
inversion preserves the integer. Lsearch converts callback failure to Error while
switch propagates the selected callback completion after List refetch.

Jim index conversion selects the original safe-expression operand before
evaluation. `evaluate_index` retains the same ordinary object or requests the
native fresh counted suffix for an `end` expression. Actual safe GetWideExpr owns
expression, context and result effects. Existing Int and Index cache paths bypass
expression evaluation. Guest expression failure becomes the selected bad-index
result; unavailable physical storage or expression authority remains a typed
refusal. `JimIndexEvaluationError::Index` retains index-geometry failure, while
`Expression` retains the actual safe-expression completion, including host
refusal. C Index table identity never licenses a Jim Enum, ComparedString or Index.
`number::native_int32_low_bits` projects the low signed 32 bits only after the
selected getter, operand range and physical width have been established. Its
pure bit projection cannot turn an unavailable conversion into a successful
Index, completion code or integer getter.


### Argument-count error identity

`CmdError::wrong_args_bytes` accepts an already selected byte header and retains
`CmdErrorCodeUpdate::WrongArguments`. Use `wrong_arguments_message_bytes` only
at an authenticated argument-count check that constructs a custom native usage
sentence. An arbitrary guest error with identical text remains `Default` and
resolves to the exact `NONE` error-code list. Result text never selects semantic
error identity.

`CmdErrorCodeUpdate::resolve` produces `ResolvedCmdErrorCodeUpdate` before the
adapter changes guest result, options or interpreter state. Only the
`WrongArguments` receipt requests the selected Registry metadata provider.
Actual C Tcl 8.4/8.5 and Jim publish `NONE`; actual C Tcl 8.6–9.1 publish
`TCL WRONGARGS`. F5 simulation requests its explicit logical Tcl 8.4 provider
and retains logical origin. Unsupported native engines produce an operational
host refusal before a guest state change. Primitive getter `Unchanged` and
exact `Set` actions retain their separate obligations.

`NativeWrongArgumentsProtocol::string_result` separately selects the C
append-based String producer. Runtime `report_cmd_error` and VM
`completion_from_cmd_error` consume that recipe only for the structured
`WrongArguments` cause. Jim and logical provider rows supply no native C
String producer. Message bytes, the error-code list and object primary cache
remain separate assertions; one matching field cannot establish the others.

Regex helpers retain a complete `CmdError` inside `RegexError`. Consumers call
`into_cmd_error` and use their normal error adapter; extracting only message
bytes loses the arity, primitive-state and host-refusal receipts. Incomplete
native `try` clauses use the Registry clause argument metadata, including
`TCL OPERATION TRY ON ARGUMENT`, `TRAP ARGUMENT` and `FINALLY ARGUMENT`.

### Native diagnostic and binary string units

Physical byte-array storage retains its mandatory string materialization
recipe. Its updater produces native modified UTF-8, including the zero unit
encoded as `c0 80`. Resident string bytes remain authoritative. Binary conversion
uses `NativeBinaryByteConversion::convert_native` with an independently selected
`NativeTclUtf` recipe; it decodes the original native units and applies narrowing
or checked Latin-1 conversion without Rust Unicode repair.

Expression operand diagnostics use the shared byte presenter. C Tcl 8.4–8.6
messages do not consume an unused operand's Unicode view; C Tcl 9 diagnostics
use the native CString value extent. Boolean diagnostic operands preserve
native string bytes and release-specific clipping boundaries. Pure dictionary
to-list conversion reads member objects directly only while no resident string
exists; a resident dictionary spelling can contain duplicate keys and remains
the authoritative list input.


### Retained array search storage

The standalone Runtime captures `RetainedArrayCell` with an array operation's
physical binding. Its search key inventory contains attached element identity
rows, including undefined cells. Candidate existence is read from the same
captured incarnation without guest callbacks. Detached destruction retains
that incarnation's key inventory and removes retired members independently of
a new array under the same name. These queries never resolve a replacement
through the source spelling. A checked role-trait consumer without the retained
operation target receives a host refusal rather than a guessed current binding.

`VarStore` byte methods retain the root and element operands separately.
`Traces::fire_bytes` reports operational access failure outside its inner guest
callback outcome. Unicode convenience methods remain checked adapter surfaces;
a native byte consumer uses the corresponding byte method and preserves all
rows through listing and storage.

### Jim Source length and strict nested schemas

`NativeStockListInputClass::JimSource` denotes the genuine pinned Jim Source
primary. Runtime obtains it from the exact `JIM_SOURCE_TYPE`; VM uses its
actual `IntRep::JimSource`. Source spelling, a Source-like label or generic
Unknown value cannot select that class. Choose the physical operation through
`InvocationDialect::native_object_length_protocol` or the independently
explicit logical `object_length_protocol`, then
`NativeObjectLengthProtocol::action(class, canonical_empty)`.

The Jim Source class is admitted only by the selected Jim084 engine. C refuses
that class even when the supplied canonical-empty flag is true. The authentic
Source-to-List conversion keeps resident string bytes and gives the first child
its original source filename and line. Script, Expression, Regexp and arbitrary
unknown primaries receive no equivalent admission. The conditional
`NativeStockListLengthProtocol::normal_cache_disposition` query is analysis
advice for a separately reached normal conversion, not a physical conversion
or current cache observation. See the [original Source conversion proof](../analysis/name-resolution-proofs/list-original-jim-source-length-conversion.md).

Nested selector and option matching follow the selected
`SubCommand::prefix_matching` policy at the shared resolver. Jim's namespace
ensemble schema uses `PrefixMatching::Strict` for `create` and `-automap`;
its prefix value remains an independent actual operand. The original
`_namespace_ensemble` scripted helper has an ordinary mutable binding, so a
renamed, deleted or overridden helper cannot be replaced by catalogue source.
The Jim schema supplies no C `NamespaceLegacy` compiler identity, configure or
exists selector, `-map` option or C publication/effect facts. Keep acceptance,
conditional source layout, current helper resolution and execution as separate
queries. The [rooted-result native controls](../analysis/name-resolution-proofs/namespace-jim-original-scripted-ensemble-publication-with-rooted-result.md)
retain the actual created name and target calls independently of inputs whose
namespace-qualified result variable is unavailable.

## Original command objects and literal cache actions

Select `InvocationDialect::native_command_name_protocol()` from actual native
engine authority. `NativeCommandNameProtocol::cache_is_current` consumes live
node and referencing-namespace observations from the same interpreter. A rename
can move the node before its epoch changes; its captured reporting namespace is
not a cache-hit equality key. Getter misses refresh the original object,
installing an unresolved descriptor on C8 while preserving the prior primary
representation on C9. Jim and explicit logical name simulations resolve the
original bytes through their selected lookup and install no C command cache.

The native compiler query supplies a retained namespace token, original lookup
binding and ordered actions. `LiteralTable::prime_native_command_name` records
priming after local deduplication, including data-first registration.
`hide_native_literal` withdraws the local hash key and records C8.5's reached
object copy. Interpreter-global registrations have per-bytecode leases; a cache
record contains no callable, procedure or bytecode ownership.

`LiteralTable::retain_syntax_error_info` records the C9.1
`NativeLiteralAction::RetainSyntaxErrorInfo` after the original options and
message object-array slots exist. Both concrete pools check that actual release
and those original slots, then retain the same message header as the private
merged Dictionary's `-errorinfo` member. Reconstructed bytes or a new equal
String cannot replace that relationship. Preserve chronological actions when
deduplicating local entries; the action grants neither a compiler selection nor
permission to publish a guest error.

Concrete dispatch and namespace command queries consume the original object,
then report the live selected token's full name. Buffer-only lookup APIs remain
separate. At callbacks, `OriginalObjectResult::CommandName` retains an
engine-private scope receipt plus resident string storage. Recovery validates
the original interpreter and descriptor origin; data snapshots explicitly
refuse this cache rather than replacing it with a String representation.

`ordered_literal_actions_match_sixty_original_native_observations` consumes the
five native fixtures in `rust/tcl-vm/tests/data/native_literal_pools/command-actions`.
Getter replacement/miss, original-allocation mirror and opaque callback scope
controls test the independent runtime and transport contracts.

## Ensemble usage rewrite entry and handler adaptation

Select `native_ensemble_rewrite_protocol` from the actual interpreter dialect.
Its `resets_at` query receives a reached `EnsembleRewriteResetEvent`: Tcl 8.5
clears after successful ordinary command lookup; Tcl 8.6 and 9.x clear before
ordinary lookup and when an admitted bytecode body begins. Failed lookup emits
no after-success event. Internal ensemble, alias and OO forwarding invoke the
target without emitting an ordinary lookup event. Tcl 8.4 and Jim have no C
rewrite state. An authored F5 provider is selected explicitly and does not grant
a native ensemble receipt.

A stock handler's parser prefix is a separate adapter, retaining the exact
synthetic prefix objects and original private command head. Usage rendering
applies that adapter only to its original argument vector, then applies the
currently active real ensemble rewrite. A truncated parser prefix does not
license rewriting. Ordinary callback evaluation can retire the real rewrite;
returning from the parser adapter does not restore it. Callback usage cannot
borrow a suspended parser prefix merely because the word bytes match.

The selected C9 `array default` compiler hook accepts two or three operands
after `default`, invoking the admitted private worker directly. Its narrower
option arity is checked after ARRAY callbacks. Dynamic calls retain a real
public ensemble rewrite until evaluation withdraws it. Broad outer arity
failures remain generic public invocations. The native fixtures in
`runtime/rust/tests/data/native_array_default/usage.tsv` and
`rust/tcl-registry/tests/data/native_ensemble_rewrite` test these independent
lookup, execution and usage contracts.

### Original array searches

The selected `InvocationDialect::native_array_search_protocol` requires an
independently issued `NativeArraySearchAbi`; unsigned-long width is not the
variable hash width. `tcl_cmd_core::native_array_search::dispatch` performs
original handle conversion and delegates the cursor operation to the selected
physical array. Backends store `NativeArraySearchChain` on that array incarnation
and clear it at native mutation, releasing modern handle references immediately.
The chain owns no variable-cell pin and never resolves a replacement array by
name. Undefined trace-shell garbage collection preserves active searches.

`NativeArraySearchOperand` borrows the original handle together with the bytes
and cache observations reached on that same header. Pass it by reference to
`array_search_on_original` and `resolve`; neither the carrier nor its byte view
owns a handle or licenses lookup of a replacement array. Keep the selected
array, conversion timing and cache issuer independently validated.

C8 installs the resident-only handle primary after a valid decimal `strtoul`
parse, including before wrong-variable and missing-search failures. The original
string is preserved, along with its byte offset. C9 owns the returned handle and
checks its identity before CString spelling; it does not install the legacy
cache. Jim has no native Search members. Unknown native selection or unsupported
ABI withdraws before guest conversion. The fixture
`rust/tcl-syntax/tests/data/native_array_search` records the five actual C
engines; its error-state column is an ensemble observation rather than seeded
primitive-error preservation evidence.

The `native_array_search::tests`,
`cmd_array::tests::original_search_handle_cache_matches_all_native_columns`,
`cmd_array::tests::native_search_lifecycle_keeps_shell_gc_and_invalidates_real_mutations`
and `vars::tests::search_chain_owns_modern_handle_and_retires_on_original_table_mutation`
controls check the selected grammar, original cache effects, cursor lifecycle and
actual modern handle holds. Run them with the repository's normal package test
commands and oracle configuration.


### Native constructor root inventories

Select `InvocationDialect::native_bootstrap_protocol()` for the actual audited engine. `allocations(CreateInterpreter, has_platform_default_library)` gives producer order, including undefined error and precision cells. `MainArguments` is a separate C producer with `argv0`, `argc`, `argv`, and interactivity in that order; the Jim CLI producer is unavailable through that purpose. Neither plan initializes a script library or registers an extension's procedures.

`Vm::with_native_core` and `Interp::with_native_core` consume this root plan before adding library or application globals. Supply the native build's path bytes through `NativeBootstrapInputs`; a release cannot establish those paths. The ordinary constructors retain their embedding-host globals. Native-core constructors still register the backend's supported command implementations; they do not attest a complete native command/module roster. `registers_core_binary`, `registers_core_try`, `registers_core_throw` and `initializes_tcl_oo` gate their actual core registrations separately from distribution initialization; autoload procedures belong to the library initializer. Platform-member values come from the selected host. The inventory includes physical undefined cells, but cannot itself prove a native error read/unset hook or a private error-state update.

The fixed constructor fixture is `rust/tcl-registry/tests/data/native_bootstrap/core-roots.tsv`. Registry tests compare selected allocation order through the independent ABI/hash owner; VM and Runtime tests inspect actual retained root cells, including absence of CLI/library globals. Run the `native_bootstrap_tests` and `native_core_root_cells_match_six_original_constructor_inventories` filters with the six strict native oracle environment variables documented above.


Compiler lookup of an import must retain the raw imported command row separately
from its callable origin. `compiler_identity_at_lookup` returns the original
wrapper's token; ordinary command resolution follows the origin. The wrapper
retains its copied hook presence and private compiler identity. A current
origin hook cannot supply that missing evidence. No-op and ensemble compiler
prerequisites identify the raw wrapper, including after rename. Ensemble
configuration still comes from the actual shared ensemble token.

`NativeCompilationSnapshot::compiler_targets` projects compiler registrations;
its targets do not establish post-operand callable identity. An imported Set
compiler can remain selected while its origin invokes a different handler.
`SourceCompiledInvocationProof::compiler_prerequisite` retains the exact raw
registration and namespace incarnation. `native_operation_selection_plan`
forwards that receipt and any independent worker dependencies to
`NativeOperationSelectionSite`. The shared
`NativeCompilerSelectionPrerequisite::from_command_registration` projects
Command or Ensemble from the retained compiler payload, independently of the
callable. Execution checks that same typed prerequisite as explicit
compiler-selection sites. An ensemble receipt validates the exact public
registration and configuration; selected worker bindings retain their separate
guards. Failed initial validation replays the
original source once. Operand effects cannot trigger a second lookup of an
already admitted operation. Keep normal handler, diagnostic layout, and
compiler-selection consumers on their respective receipts.

When extending compiler metadata, update actual registration publication,
import copying, token relocation, source snapshot projection, retained guard
validation, and replay together. Tests must vary the raw hook and callable
origin independently, including a replaced or opaque origin, execution traces,
and renamed wrappers. Check both emitted-operation admission and ordinary
handler queries so a compiler receipt cannot donate callable semantics.

`CommandRegistry::native_compilation_for_registration` and
`native_registration_lookup` cache pure descriptor queries by the complete
`InvocationDialect` and canonical registration identity. Registry mutation
replaces the derived cache; an immutable snapshot retains its own generation.
Negative queries are cached too. A cached descriptor grants no command-token,
interpreter, namespace, object-storage, or execution authority. Keep those
checks at the consumer's actual boundary.

Use `NativeProcedureReference` only for a genuine command, frame, or primary
reference. Use `NativeProcedureBinding` to share one actual binding across
query, import, and dispatch transports. Read its current declaration explicitly;
a cached declaration handle does not follow client-data replacement. Memory
leases preserve allocation safety without retaining native resources. Take
resource cells before releasing native references so free hooks can reenter
without borrowing the draining owner.


## Fold substitutions from their original execution receipts

`word_subst::checked_lifted_calls` supplies the complete original substitution
inventory. A missing inventory or child carrier withdraws an applicable fold.
Select a child by its unique original source offset; source text, an unrelated
namespace tail and the module's final command map cannot supply its dispatch.
Expression-contained children use `lifted_calls_with_surface` with the same
retained carrier and actual lexer policy.

`ConstSubstCtx::fold_retained_call` selects the original execution target,
actual invocation dialect and shared effective operand mapping. Its
`retained_arguments` projection preserves written, expanded and captured
prefix origins. Frozen argument contents do not prove that their evaluation
can be erased. Variable operands require the original captured read and closed
physical read contexts; nested substitutions require their own retained fold.
Captured prefix objects additionally require the original native object-method
effects to be closed. Refined invocation effects retain unbounded callbacks
even when the selected handler and result bytes are known.

Applicable O103 procedure folds require the exact authored declaration
allocation. The shared formal-parameter owner validates native arity and binds
actual values, defaults and rest parameters. Jim caller links cannot be seeded
as values. An argument-independent constant return still requires the original
argument evaluation and native parameter binding to succeed.

Command arguments, interpolation, typed returns and method-frame substitutions
consume these same projections. Expression folds additionally use the original
prepared-expression and native result protocol owners. Method dispatch barriers
union the original class and procedure targets and their transitive reach;
missing targets, incomplete layouts and unbounded native callbacks widen reach
to every component. Advisory unpositioned lookup remains separate from these
execution and erasure proofs.


### Original diagnostic frame and caller-link advice

Original diagnostic layouts retain either an accepted source declaration and
its allocation, or the original root script's actual global frame and namespace.
The frame is independent from an entered body. A diagnostic consumer validates
an authored procedure through `DeclarationFlowReport::owns_original_procedure`;
an unavailable `executed_source` does not remove its original declaration.
These queries supply conditional diagnostic occurrences, never body execution,
physical variable reads, SSA definitions or permission to remove a store.

The declaration flow inventory retains immutable original dispatch points.
Selected candidate layouts use `effective_words_for_target` and
`frozen_argument_words`, so captured alias prefixes, evaluated var-lists and
known expansion elements use the same argv mapping as execution consumers.
Missing observations and unknown dispatch preserve their residual obligations.
Loop body bindings describe entry to that body; zero-trip execution remains
independent.

Unentered formal advice additionally consumes the declaration-flow report's
`conditional_handler_keeps_incoming` query at the exact original handler site.
The incoming-name set is independent of definedness and literal values: joins
intersect it, and original writes, removals, aliases, possible outputs and unknown
effects withdraw it. Require the declaration-owned variable word, frame and
typed namespace key, original literal earlier operands, selected native formal
plan and unanimous provenance. This query mints no actual `SourceVariableAccess`
and supplies no caller value or successful store. Test a preserved incoming
formal beside overwrite, alias, unknown-writer and conflicting-layout controls.

Use `collect_positioned_call_by_name_reads` for caller diagnostic suppression.
It selects the original callee allocation, binds original arguments through the
native formal grammar, follows symbolic incoming-formal provenance and consumes
registry `VariableCellAlias` transitions. The returned names cannot grant a
runtime alias, physical caller-cell use or executable store removal. A qualified
local procedure shadow, imported or renamed command and captured alias prefix
must be checked at the original invocation. Unpositioned spelling indexes are
assistance only and cannot resolve these positioned calls.

Stock constant-fold handlers explicitly declare their own native world effects.
`ConstSubstCtx::fold_retained_call` requires the actual selected handler,
its implementation lookup, native dialect, selected subcommand and argument
offset. Original variable operands additionally require closed native string
access; known text alone does not close an object's getter. Unknown object
conversion or execution callbacks retain the shared effect barrier.

`declared_absence_warning_occurrences` retains a separate conditional prefix
whose native layouts remain available. A later unknown loop backedge cannot
erase a first-entry occurrence; an unknown earlier writer cannot donate one.
The complete occurrence inventory, actual effects and executable store graph
retain all their alternatives.

Caller navigation uses `SourceCallerFrameInvocationTemplate::procedure_definition`
to identify the exact original declaration allocation. Its conditional source
frame and literal operand mapping remain independent from the actual procedure
reference and receiver-method execution receipt. A joined runtime frame may be
unknown while its immutable declaration observations agree. Consumers must
validate source, allocation, dialect, realm and native formal layout; this
navigation template cannot certify a caller-cell write or normal completion.
`namespace_context` retains an exact procedure-body key; a receiver preview
returns `None`. `SourceConditionalBodyEntry` follows the same distinction.
`SourceDeclaredReceiverBodyEntry::declaration_namespace_context` is declaration
provenance, not an entered receiver frame. A `RootScript` receipt requires both
global frame layout and the retained actual root identity. Restoring an
original body must filter expression preparation and math bindings by its exact
namespace key, independently of its presentation label.

`ExecutionNamespace::SourceContext` binds analysis through the retained exact
`SourceNamespaceKey`, using `ResolveContext::with_namespace_identity`. Preserve
that typed context when rebinding a method or restoring an original body.
Compatibility `for_head` projection declines a SourceContext, including an
absolute-looking displayed name; it cannot turn native, allocated or authored
identity into a text lookup. `RuntimeSelected` retains unknown context rather
than borrowing a namespace from the body or declaration's presentation.

O103 can use the completed original invocation's result bytes only with an
independent `original_invocation_completes_normally` receipt, the same actual
callee allocation, native-bound original arguments and closed effects. A
conditional normal result alone cannot replace an invocation. Constant-fold
descriptors are selected with the actual invocation dialect, including when
the registry contains several implementations of the same command.

`SourceDeclaredReceiverBodyEntry` retains the original native member declaration,
body source and formals without allocating a method implementation or receiver.
Caller navigation compares this exact scope for the call and read. A template
with `declared_receiver_body` has no `frame` or actual method-dispatch receipt.
Nested procedure bodies and neighbouring receiver declarations keep distinct
owners. Original layout observations validate the preview scope independently
from the entered runtime frame.

`conditional_declared_overwrite_advice` describes adjacent original literal
setters under an available declaration prefix. It carries source sites and a
warning span, never a physical cell, contents version, successful store or edit.
Unknown prefixes, dynamic reads and aliases withdraw this warning inventory.
An earlier known missing read does not turn the later setters into executed
stores; the diagnostic explicitly conditions its advice on reaching them.
After an abrupt source prefix, `record_unentered_declaration_suffix` retains
only original operand layouts against the actual joined abrupt lookup table.
It does not evaluate operands or publish dispatch, argument completion or
compiler visits. Literal operations whose declared native effects preserve
lookup can carry that table forward; unknown handlers, explicit callbacks,
lookup mutations, executable bodies and substituted operands stop retention.
Variable-observer alternatives remain outside this conditional declaration
projection. A computed read still withdraws overwrite advice for the complete
frame, and a return route still stops its declared successors. Use the
declaration report for these warnings; a missing execution receipt cannot be
replaced with a reconstructed setter or a physical contents assumption.

Argument-sensitive Registry forms refine both legacy and world-effect metadata.
The one-argument `subst` form uses the shared native template scanner to close
world effects only for Text components. Variable, command, Jim expression and
unavailable components retain their effect barrier. The two-argument `string is`
value form has no optional write operand; `-failindex` layouts retain their
variable effects. Original object coercions and callbacks remain separate
obligations for every form.

Native namespace binding selection consumes original `NativeCompilerWords`
through the shared source adapter. It retains the original image, input channel,
delimiter extents and complete compiler word vector. A binding-owned query
rejects changed, truncated or conflicting vectors before applying the native
grammar. The source-string issuer remains independent of the physical compiler;
a live entry with a missing issuer cannot borrow the authoring profile's recipe.

`namespace_binding_preparation_at` exposes the original compiler's ordered
declaration and operand-visit recipe under unanimous source, table, namespace
and policy guards. A later compiler decline retains its accepted prefix geometry.
Child compiler failures stop traversal in native operand order; the recipe does
not certify that every planned visit completed. This preparation grants no live
namespace alias, cell contents, successful store or emitted instruction. Generic
fallback and runtime binding keep their own owners.

`ProcedureProvenance::namespace_context` records the original body lookup
context independently of the displayed procedure name, parameters and body.
The VM procedure-artifact cache partitions candidates by exact component path
as well as their original source identity. Colliding namespace displays remain
separate candidates. `CompiledNamespaceContext::ConstructedPath` preserves that
geometry but cannot authorize native reuse; a missing context cannot be recovered
from the displayed declaration name. Reuse requires a `Native` context that
resolves to the selected interpreter and namespace incarnation at the query.
Deleting and recreating the same path cannot redirect an old artifact, while an
actually retained old namespace remains usable until its real owner retires.
This metadata adds no namespace reference, procedure role, literal registration
or compiler permission. Current procedure activation prepares its original body
without selecting these dormant module candidates; candidate provenance cannot
donate an original Bytecode header. Validate distinct paths with colliding displays, absent
context, foreign interpreter, same-path replacement and final owner retirement
when changing the cache key or admission query.

C8.5 monolithic `NamespaceLegacy` selection delegates to
`compile_native_namespace_upvar`. The common
`NativeNamespaceBindingCompilation` retains `Upvar`, the explicit original
namespace operand, ordered bindings and visits, and its Inline/Generic/Unknown
outcome. Selection requires the raw `upvar` member and the original procedure
compiler context. Quoted or braced SIMPLE_WORD local names are accepted; escaped
or substituted locals decline. Script and Direct contexts remain Generic, and
C8.4 has no monolithic hook. Later C releases use their separately selected
namespace workers rather than borrowing this C8.5 recipe.

Visit the original namespace first, then each target Word before its
`DeclareLocal`. A rejected array local can still allocate its base, and earlier
literal pools and LVT declarations remain preparation facts after Generic
instruction rollback. Partial compiled aliases must not execute on that decline.
The Compiler emits `NSUPVAR` only for the accepted recipe and retains its actual
BeforeArguments compiler-selection guard. The Runtime artifact keeps the same
original namespace object across target evaluation, then performs namespace
lookup after each target using the original object cache and checked alias
owner. A target substitution that deletes and recreates a namespace can select
the new incarnation; neither an earlier lookup nor a rendered path can replace
that operation. Use the native fifteen compiler-frontier and eight execution
controls in `native_namespace_upvar_compilation` when changing member grammar,
visit order, rollback, declaration order or callback-sensitive lookup. A pure
recipe supplies no live cell, completed operand, namespace owner or body entry.

Native procedure ownership distinguishes actual roles from query handles.
`NativeProcedureReference` owns a real command, active frame or primary role;
borrowing its declaration or sharing metadata adds no native reference. Runtime
`NativeCallableProcedure` queries commandless method Proc clientData; cloning
that transport does not prolong the native role. The original Method owner
publishes and retires the role, while `enter_method` retains the actual entered
clientData through replacement until the call finishes. Proc execution has its
independent active role. The original lambdaExpr primary owns its Proc and the same absolute namespace
object; duplicating that header acquires those two roles. TclOO method copying
creates a new Proc and string-only body while retaining the original defaults.
Use declaration-copy APIs instead of cloning snapshots to create a callable.
Retirement frees native resources at the final role even while metadata survives.

`operation_for_handler_identity` selects a mathop only from its attested registered
implementation. Rename, alias and import preserve that identity independently of
current argv. Dispatch does not stringify argv0 to select the operator; only the
actual native arity presenter consumes the original head. Validate changes with
`selected_mathop_identity_matches_all_60_native_name_and_argv_controls` on both
ports, including overridden handlers and operand effects.

`c_family_local_alias_name_bytes` supplies release-independent authored C
assistance, using the shared CString qualification scan. It accepts a qualified
tail or an unqualified name without NUL; an unqualified NUL-bearing input needs
an exact release and remains opaque. Registry `local_alias_name` uses this only
for explicitly unversioned Tcl-family assistance. F5 and unknown families remain
opaque. The local alias key and written target are separate: target qualification
uses the selected native/Jim owner and preserves literal-colon namespaces. This
projection grants no physical binding, native cache or release authority.

Package failures use the selected `NativePackageProtocol` error-code action at
the actual primitive boundary. Publish the private header before returning
completion metadata; a carried `-errorcode` dictionary alone does not update the
interpreter's original private state. The package publication controls seed a
prior guest code and inspect the original global read for bad members,
preferences and non-Error loader completions. Run
`package_error_publication_matches_all_21_original_native_controls` together
with the all-six ordinary package, original version-header and files-header tests.

## Native preparation and original object ownership

`NativeCompilationSpec::requires_original_word_preparation` identifies compiler
descriptors whose preparation consumes original source through a shared recipe.
Consumers use that capability instead of maintaining their own grammar lists.
It supplies no hook, selection or execution authority.

`native_instruction_plan` consumes the complete original `NativeCompilerWords`
vector under the actual compiler registration. For an independently selected
registered worker, use `native_registered_worker_instruction_plan`: the complete
vector still retains the public ensemble selectors, while the operand offset
identifies the actual worker arguments. Its validator uses registered-worker
grammar, without reinterpreting the public head as another worker selector.
Neither factory supplies registration, compiler-hook, or callable authority.
Its operands identify unchanged source words or parser-expanded literal members
with exact value spans.
`project_native_compiler_words` follows the selected C parser's pure TEXT list
expansion. Substituted, malformed and generated members retain their expansion
obligation. Runtime List conversion cannot replace that parser evidence.

Use `SourceCommandBindings::structured_compilation_at` at the original command
allocation. The query requires unanimity across retained compiler observations:
the complete unchanged source vector, image and channel, policy, namespace,
table and structured recipe must agree. `SourceNativeStructuredPreparation`
keeps `compilation_site()`, `recipe()` and `dependency()` together, so original
geometry cannot be detached from the compiler registration and lookup
incarnation that supplied it. The binding-owned
`SourceInvocationBinding::original_compiler_source` authenticates the complete
original vector before source/preflight consumers select a recipe. Transformed,
truncated or synthetic words supply no replacement authority.

Traverse the retained preparation in its native order. `Generic` keeps its
authenticated preparation prefix; `Rejected` carries the exact native compiler
failure and stops traversal at that failure. Missing evidence remains a typed
refusal. Neither outcome permits recovery of a body index from generic or
expanded runtime argv. Source/preflight follows original recipe operands and
compiler visits; the Runtime artifact separately instantiates actual literal
pools, local slots and control ranges. Preparation does not prove normal
handler effects, reached bodies or runtime values.

| Shared recipe | Preparation and execution contract |
| --- | --- |
| `NativeNamespaceBindingCompilation` | `visits` preserves implicit literals, local declarations and original operand compilation in order. A declined prefix retains its private pool and local allocations; generic fallback recompiles the original words with its own executable publication. `bindings` execute only for an admitted inline outcome. |
| `NativeReturnInstruction` | Original static options and ordered stack pairs use the shared physical return merger. Its control receipt records immediate code and pending level independently of the retained private options header; the actual procedure boundary settles a pending return. |
| `NativeErrorInstruction` | `compile_native_error` projects the complete original parser vector, including literal expansion member spans; `native_error_instruction` accepts already selected operand identities. Preserve `NativeErrorStep` order: original message, then each compiler-owned keyword and original value. C8.6/C9.0 construct List pairs; C9.1 inserts Dictionary pairs. Immediate Error Return uses code one and level zero independently of Return-command grammar. C8.4/C8.5 and unsupported arity or runtime expansion retain generic invocation. |
| `NativeSwitchInstruction` | The original subject precedes only arms whose `compile_body` is true. Pattern and body spans, fallthrough targets, duplicate masking and matching operations come from the selected recipe. Validation precedes subject effects. |
| `NativeTclOoInstruction` | Actual helper registration and original word geometry remain independent. Self/Next additionally require an issued method frame. `ObjectInfo { operation, operand }` evaluates the retained original operand before its selected object lookup/getter; it never rebuilds a name from a reporting label. Expansion and argument construction follow the selected C release; a helper spelling supplies no object or frame. |
| `NativeUnsetInstruction` | `compile_native_unset` validates flags and the known-word pass before preparing any variable. Compile each retained root/index operand immediately before its own unset; preserve `complain` and scalar/array/local/stack receiver choice. Dynamic expansions retain Generic invocation, C8.4/C8.5 have no unset hook, and unavailable source geometry remains a typed refusal. Selection grants no successful destruction or current cell value. |
| `NativeNamedInvocationInstruction` | Use `native_named_invocation_instruction` with the admitted private name, original words, consumed post-head count, protocol and canonical replacements. Keep `Original` operands and `Replacement` literals distinct, with aligned expansion flags. Direct compiles the private head before its arguments; Rewrite compiles original/canonical argv before the private head. Resolve the actual handler by ordinary lookup after operand effects; the pure layout grants no binding identity. |
| `NativeControlCompilation<T>` | Apply `preparations` in order, then consume `outcome`. `Inline` retains an instruction recipe; `Generic` retains completed preparation before generic compilation; `Rejected` carries the native compile-time failure. Missing geometry is a separate typed refusal, not generic permission. |
| `NativeControlInstruction` / `NativeCatchProtocol` | Probe literal Booleans with a fresh NULL object. Preserve if pruning, while body-before-test compilation and for start/body/next/test order. Catch retains release-selected range entry, stack order and result/options store order; C8.4 speculative rejection retains completed preparation while discarding body instructions. Keep `NativeControlBody::script` extents separate from its original dynamic operand. |
| `NativeExpressionProgram` | Keep the original operand, counted source, exact span, parser context and checked tree together. `native_expression_boolean_operator` separates Boolean literals from absent-string arithmetic results; `native_expression_private_logical_boolean85` selects the distinct C8.5 logical-fold registration lifetime. Folded producers retain the selected original result header and pool role. A proved syntax rejection is executable native failure; absent parser or source evidence is a refusal. Dynamic `NativeExpressionInstruction` operands retain their original EXPR_STK construction. |
| `NativeEachInstruction` / `NativeEachAuxiliary` | Preserve original iterator groups and exact body range. Compiler output carries actual variable indices and anonymous temporary indices, independently of their name bytes. Execution uses the release-selected `NativeCompiledEachStorage` owner. |
| `NativeTryInstruction` | Preserve original handler matcher, each clause's own binding slots, its non-dash body target and independent finally range. Capture actual result/options headers in stack roles or genuine anonymous cells; cleanup uses RETURN_STK and the original `-during` dictionary. |


Unset receiver selection keeps lookup purpose separate from variable geometry.
`QuietUnset` suppresses the selected guest missing-cell message, while unavailable
host ownership remains a refusal. An explicit root/index operation must use the
original root and element headers; a combined-name parsed-array cache on the
root object cannot substitute for those parts. Retain the originals through
callbacks, and complete one receiver's unset before evaluating the next target.
The compiler recipe supplies these ordered operands, not current cell contents
or completed callback effects. Runtime's `unset_original_c_variable` and
`unset_original_c_parts` share that original receiver owner with the compiled
artifact. Indexed receivers retain the actual selected slot. Undefined selected
trace shells still fire; detach the old registrations before callbacks and
retain the old member through them, including when a callback recreates the
same written root. Ignored unset callback errors do not prove successful later
operand evaluation.

Object introspection uses `NativeTclOoObjectInfo` independently of method-frame
helpers. VM and Runtime object getters consume the same original command header,
checking an eligible live command-name cache before a string getter and following
the actual imported command origin. Class results reuse the original cached
class-name String. Namespace results use `ObjectNamespace` with the actual
object namespace token; a rendered path cannot recover a deleted or replaced
incarnation. Standalone IsObject results retain the interpreter's execution
constant when `uses_execution_constant` selects it; a conditional-jump peephole
may consume truth without producing a result header. Missing-object error state
and predicate truth are independent: a false predicate preserves the selected
native private lookup code. CreationId uses the actual creation epoch and is
registered only for C9.1. Preserve the original lookup/result-owner tests and
operand-before-getter tests when changing either port.

`NativeVariableObserverPresence::permits_no_callbacks` accepts only a complete
Absent table or BoundedNativeOnly engine hooks. Opaque native observers and
retained active trace firing count as Present even without a script-trace entry.
Unknown or partial tables cannot supply negative callback proof. This receipt
proves neither contents, links, physical frame ownership nor compiler entry;
namespace variable membership is a separate captured inventory.

Error instruction selection requires independent proof of the installed native
compiler hook and the unchanged original source vector. Its pure options layout
cannot manufacture original object headers or grant source execution. Compiler
codegen and Runtime artifact preparation consume the same ordered recipe;
execution applies the actual immediate-return option/header owner. Test the
message, info and code operand evaluation order, literal expansion geometry,
release-selected List/Dictionary construction and generic decline separately
from Return-command parsing and procedure-boundary settlement.

`NativeInstructionPlan::expression_program` borrows the checked expression
program for an exact original operand. `body_error_context` and
`expression_error_context` select the native annotations for that recipe role;
an unrelated operand has no annotation. These queries do not parse new text,
resolve a command or enter a body. `NativeExpressionProgram::compiler_steps`
retains C8.4 fixed-function lookup before its argument visits, using the actual
fixed-function table independently of command lookup. Later C releases and Jim
supply no fixed-function-table lookup through that axis; original script
compilation visits and deferred syntax failures retain their independent
recipes. Unknown source geometry cannot be replaced by a reconstructed
expression or generic body layout.

`NativeExpressionProgram::compiler_name_effects` selects a separate source-only
compiler purpose. The inspected C8.5–9.1 FUNCTION branch constructs the counted
`tcl::mathfunc::` literal before runtime invocation; it does not look up or
execute the function while compiling. `expression_compiler_names_preserved`
additionally requires current ordinary-pool effects and quiet callback-free
lookup, because pooled string getters and reference release can invoke object
callbacks. Original argument bracket compilation retains its exact recursive
visits. C8.4/Jim Calls and Raw or foreign contexts remain unknown for this
purpose. No runtime function registration, handler completion, compiler
admission, cache or Normal result follows. See the [function-name compiler
contract](../analysis/name-resolution-proofs/expression-compiler-function-name-literals.md).

`SourceInvocationBinding::original_structured_compilation(tokens)` authenticates
Each preparation against the complete original source vector and compiler policy.
The executable CFG retains that invocation for Inline, Generic declaration
prefixes and Rejected outcomes. The native emitter visits declarations, original
value operands, the exact loop body/range and result literal in order; the
analysis CFG retains its structural loop graph. Changed or synthetic words and
conflicting source/compiler evidence supply no Each recipe. Preparation does not
imply handler entry or normal completion.

Catch's stack entry and protected instruction range are separate owners.
`Instruction::catch_start` marks the first protected instruction and `catch_end`
its exclusive end. Actual C8.4 opens BEGIN before dynamic body substitution, but
that substitution lies outside the protected range. C8.5+ performs substitution
before BEGIN. Preserve this distinction when lowering the recipe or entering a
Runtime range; a stack entry alone cannot catch an earlier substitution failure.
`NativeCatchProtocol::result_before_options` selects result-before-options stores
in C8.4/C8.5 and options-before-result stores in C8.6+. C9.1 also changes the stack
code/result order. These layouts do not grant native completion-header identity.

`NativeExpressionNumberLiteral` retains an Integer payload, exact Double bits or
arbitrary-precision sign/magnitude without formatting. Its `number()` projection
does not establish a native object. `register_private_expression_number` records
an ordered `PrivateExpressionNumber` slot; concrete literal owners authenticate
the same actual C8.5+ issuer before manufacturing the absent-string numeric
header. Ordinary Boolean results use the compiler's registered original `0` or
`1` literal. `native_expression_private_logical_boolean85` selects only a folded
C8.5 And/Or subtree. `register_private_logical_boolean85` records its ordered
`PrivateLogicalBoolean85` slot; the concrete temporary compiler acquires and
releases an actual registration before retaining that same original local header.
An independently live registration can therefore keep the same header shared.
C8.6+ resident folded Boolean results remain normally registered.

A folded result with resident bytes uses `LiteralTable::intern_expression_number`.
Its chronological `NativeLiteralAction::AdoptExpressionNumber` follows ordinary
registration and deduplication. Both concrete pool owners authenticate the actual
issuer and install the numeric representation only into that same untyped header;
resident bytes and an earlier typed cache remain intact. `NativeConstantResult`
keeps Number, resident bytes with optional number, guest failure and unavailable
operands distinct. A genuine arithmetic failure emits the selected Syntax packet;
an unavailable operand keeps the runtime operator. Runtime
`eval_compiler_constant` omits public final-string normalization. A selected
ternary TRY_CVT converts the same original result instead of replacing its header.

`native_logical_expression_compilation` selects C8.4 `NormalizedLeft84`
independently of the later C `BranchResult` recipe. Preserve original preparation
chronology: compile the left program, register the same pooled `0` then `1`,
normalize the left result and test its jump, then compile the right program for
eager LAND/LOR. Runtime `PreparedNativeExpression::logical_left84` retains the
actual pool indices and compiled-node execution borrows those original headers;
public whole-expression reparsing or byte-equal replacements supply no pool
identity. `native_expression_boolean_word84` distinguishes reached word-Boolean
conversion from registered numeric `0`/`1`. Primary-only C8.4 TRY_CVT preserves
the produced long/wide or nonnumeric original header. Fresh C8.4 conversion
uses `fresh_c84_conversion` only after the original cache miss, preserving the
selected reset and real C-call effects. Both float-error normalizers use
`c84_nonfinite_error`: finite/NaN handling preserves thread state, while infinity
requires authentic `NumericErrorState` EDOM/ERANGE facts. EDOM/NaN classification
precedes range/infinity, and unknown raw errno remains unknown. Absent host
facts produce typed refusal rather than borrowing unrelated runtime state. Jim and unknown syntax issue no C logical
compiler recipe.

`ExprOps::prepared_node` permits the selected producer to return its retained
original for the exact prepared tree before evaluating leaves. Its default
returns no replacement, so ordinary expression contexts continue through the
normal walker. Runtime `PreparedNativeExpression` retains folded original slots
or a deferred Syntax message/options packet. Neither a matching mathematical
value nor reconstructed result text licenses prepared-object reuse.

The Compiler and Runtime artifact builders preserve these same recipes and
ordered allocations. Source-analysis preparation retains their original
geometry and compiler dependencies independently of reached execution. A
shape-only query cannot manufacture an original vector, a completed local
layout or a runtime binding. Missing source, issuer, registration or frame
information remains a typed obligation.

`native_named_worker_instruction` selects a worker's own compiler from the
retained complete invocation, rather than introducing a synthetic private head.
For its command-name cache action, use
`native_compiled_selected_command_name_literal_from_lookup` with the original
`NativeCommandCompilerPrerequisite`. `CompilerSelected` retains the actual
public lookup/configuration and selected worker. Priming must not look up the
worker again by reporting bytes; later ordinary handler dispatch remains separate.

The Runtime named artifact retains the same compiled private head in its native
literal array. `native_compiled_command_name_literal_from_lookup` supplies the
actual issuer/context and independently settled lookup, without a synthetic
source word. Direct invocation preserves the private usage header; Rewrite keeps
original ensemble words and its genuine rewrite state. Dynamic rewrite expansion
without a fixed native argument extent is unavailable; unknown source ensemble
or path attestation remains a residual. Neither case permits generic execution
under an invented named-instruction receipt.

Original object consumers borrow the original header through coercion and
callbacks. `prepare_pattern_original`, `prepare_search_pattern_original` and
`compiled_match_original` retain the selected pattern artifact and original
subject storage. Native RegExp cache hits precede string access; flags, counted
character units and zero-range equivalent-glob execution belong to their shared
owners. Capture-range matching preserves the full native expression path.
Regexp and regsub output setters use the original destination object and settle
one reached assignment before another match or callback.

Jim's integer-program regex engine uses `tcl_regex::jim::Flags::new()` followed
by `with_nocase`, `with_lineanchor`, `with_linestop` and `with_expanded` with the
selected Boolean values. Its `key()` retains the actual Jim bitmask
(2, 4, 8 and 32), independently of C ARE flags. Build this key from the selected
Jim option owner; a C flag integer or an option spelling cannot authenticate
Jim's compiled program or its cache.

`NativeInstructionName::for_return` selects the original C8.6/C9.0/C9.1
`instname` primary for reached immediate-return and Syntax instructions.
Construction stores the native opcode and leaves bytes absent; the updater
supplies `returnImm` or `syntax`. Native free and duplicate hooks are NULL.
The VM and Runtime store this genuine primary rather than a descriptive String
or a type-name receipt. Their private `innerContext` is a real interpreter-owned
List containing that name and the same original stack operands, with real child
references. Its header mutation/COW owner is independent of `errorStack` and
exception snapshots. C8.4/C8.5 and Jim do not borrow this recipe. A portable
instruction number or rendered operand cannot establish a reached inner context.
`cargo xtask owner-resolution` checks both ports' selected return-purpose door,
shared instruction-name construction and original List mutation owner, together
with the Runtime descriptor's NULL hooks and shared updater bytes. Keep those
checks when changing the producer or primary; snapshot labels alone cannot
replace the ownership checks.

Namespace variable handlers use the original name getter, the shared Define
lookup and the subsequent Write lookup with their distinct flags and diagnostics.
Define can create the namespace root without creating a rejected array element.
Callbacks can replace a namespace binding between the value write and the local
alias settlement, so settlement performs the required second target lookup and
checks the actual local cell. A pre-callback address or a matching rendered name
cannot replace that lookup.

### Consume chronological compiler preparation without granting execution

`native_control_compilation` owns the common preparation vocabulary for
`native_control_instruction`, `compile_native_each` and `compile_native_try`.
Supply the original `NativeCompilerWords`, the selected compiler context and
its actual registration prerequisite. A release number or recognizable command
name alone cannot admit these recipes. Original and parser-expanded operands
retain their own indices and value extents; `native_control_body_span` reaches
the body in the same source image. Quote or backslash transformations that
prevent a direct mapping remain an explicit geometry refusal. Do not build a
new source image from a rendered argument to recover a static body.

Declaration, original word, literal, private Integer/List, script-context and
expression visits run in their recorded order. A later decline does not undo
completed declaration or literal effects. Stop at the first actual preparation
failure; a retained planned visit does not prove that it completed. The Compiler
emitters in `codegen/native_control.rs`, `native_each.rs` and `native_try.rs` and
the Runtime artifact preparation children consume these same visits. Source
analysis retains compiler obligations independently of normal-handler flow.
The VM executes admitted instructions and their original slot/range metadata;
it does not reselect a recipe from an instruction's displayed command name.

Variable Load/Store preparation visits the retained original variable target,
including its exact index arena, before a Store value. It does not visit the
selected command head as an argv operand. Preserve that order in compiler,
preflight and Runtime artifact consumers; command-selection guards remain
independent of the variable operand visits.

`NativeInstructionPlan::Uplevel` retains the selected C9.1
`NativeUplevelInstruction`: `level_word` is the original explicit level or the
compiler's literal `1`, and `script_words` is the original fragment range.
Evaluate those operands once. Emit `CONCAT_STK` only for multiple fragments,
then `UPLEVEL`. These compiler visits do not compile the script body; later
body preparation follows the actual selected variable frame and the shared
`EvalObjectPurpose::UpLevel` object policy. Retain frame restoration, original
error context and the actual compiler token/BeforeArguments guard separately.
Test explicit and default levels, single and multiple fragments, substitutions,
frame-selection failure and replacement public handlers against the selected
instruction recipe.

`project_native_local_scalar` delegates to
`NativeCompiledVariableRecipe::scalar_name`, returning a
`NativeCompiledScalarName`. Its `declaration` is a compiler allocation request;
its `scalar` field independently accepts the scalar operand. C8.4 rejects an
array before allocating its base. Later C compilers can reserve the base before
declining that operand. A declaration inventory therefore supplies neither a
successful store nor a scalar instruction. Keep formal declaration order,
compiler comparison and dynamic lookup keys under their separate owners.

`LocalVarTable::intern_anonymous` reserves a distinct slot without a source
name. Carry it as `None` in the native layout and install its actual frame cell
before execution. An empty source variable name remains a different named
slot. Anonymous values live in those cells until replacement or real frame
teardown; they have no dynamic table key, trace name or `info locals` entry.
Do not add a parallel owning vector of temporary values. Native iterator
auxiliary storage and protected-completion storage use the same cell owner.

### Preserve expression, iterator and protected-completion operands

`prepare_native_expression_program` parses exact original bytes once through
the checked expression parser. Keep `NativeExpressionTree::Parsed` and
`Rejected` distinct from `NativeExpressionProgramUnavailable`. A parsed tree
supplies syntax, not callable math-function identity, object coercion closure
or a reached result. Dynamic expression operands remain original object
construction, with spaces between multiple EXPR_STK inputs. Static leaf offsets
cannot be rebased through an advisory Unicode rendering.

`NativeEachAuxiliary` carries ordered variable-slot groups and release-checked
value-list/counter slots. `native_compiled_each_storage` selects C8.4 local
member refresh before each setter, C8.5 retained List copies for each group on each iteration, and
C8.6+ retained stack lists with shared-header copying at iterator entry.
Evaluate original value operands before entering the loop range. Native WRITE
callbacks can mutate a value-list or a temporary, so refresh at the selected
boundary rather than retaining a detached element vector. Collect only normal
body results for lmap; for foreach retain the selected empty result producer.
Generic foreach handlers use their own original-name/iterator contracts.
`EachLoopState::advance` owns preparation and cursor chronology; its
`entered_body()` flag changes only when it returns `EachLoopAction::Body`.
Setter failure or an exhausted zero-iteration input cannot be treated as body
entry. Feed the actual completion to `body_completion` before continuing or
collecting a result; elapsed cursor movement alone proves no successful body.
Jim iterator continuations borrow the original argv variable/value List roots;
allocation and continuation lifetime receipts add no native root references.
Foreach owns the interpreter's actual emptyObj as its result until settlement.
Lmap owns a separate result List and borrows emptyObj only for padding. Keep
that borrowed padding lifetime separate from the Foreach result-owner role;
cloning a value handle to bridge continuation storage can change the native
reference window. Validate these roles against the unchanged original 406
iterator controls.

`NativeTryHandler::target` identifies the body reached after a match; it does
not transfer the target clause's bindings. A dash clause writes its own result
and options locals before entering the target body. Preserve original headers
when capturing completion, binding variables and forming `-during`. Anonymous
cells own the stored headers; a protected-completion receipt borrows them only
within the same live activation. Finally has a separate protected range and
can replace the saved completion. Host refusal bypasses guest handlers.
RETURN_STK merges the original options operand through its physical return
owner, independently of a replaceable public `try` or `return` command.

### Retain actual TclOO method records and original selector caches

`native_tcloo_method_cache` owns the C8.6+ cache recipe.
`NativeTclOoMethodCacheStamp` contains actual receiver or class-representative
creation identity, Foundation/object mutation epochs and lookup flags.
`reusable` compares those observations; it does not mint a method record or
authenticate a serialized stamp. Jim and missing native issuers cannot borrow
a C TclOO cache.

Both runtimes' `cmd_oo/native_method_cache` children own `MethodTable`,
`MethodSlot`, `NativeMethodWorld`, `NativeMethodChain` and
`ReachedMethodChain`. A table owns a mutable method record. Replacement changes
that same record; deletion and recreation produce a distinct record. Reached
and cached chains retain their selected records and original table-key owners,
so retirement cannot redirect an active call to a newly created method.
Read the reached record at the appropriate call boundary rather than retaining
a copied declaration as a replacement owner.

The original selector's MethodName primary owns the actual shared chain.
Duplicating that native header shares the chain. An equal distinct selector
that hits a table does not thereby acquire a primary, and a type-name snapshot
supplies no lookup authority. Apply genuine Foundation/object mutation hooks
at the mutation itself, including changes made before a definition body later
errors; successful definition leave alone cannot invalidate caches correctly.
Property accessor lookup passes its retained original method-name child into
this same owner instead of reconstructing a selector from a displayed key.

For C alias locals, `NativeVariableNameProtocol::alias_local_is_element`
performs ObjMakeUpvar's CString shape test before
`alias_local_input` selects the simple-name lookup input. Full counted lookup
storage and the CString element-rejection extent remain separate. Thus bytes
past the first NUL cannot change the element test, while the selected original
name and table-key owners still survive through lookup, publication and traces.
Qualification is tested before the NUL boundary. The local key is counted on
C8.5+ and CString on C8.4. This local purpose installs no ParsedVariableName or
localVarName primary on the original local operand; the complete original
target separately uses normal Link lookup and cache rules. An actual C8.5+
dynamic or namespace key birth retains the same original unqualified local
operand, or a fresh tail header when qualified. A compiled local slot owns no
hash key. Neither pure projection creates a native cache or an actual alias cell.

### Keep expression evaluation separate from physical compilation

The runtime captures `ExpressionEvaluationPolicy` in
`NativeCompilationEntry.expression_policy` inside the selected activation. Its
issuer, profile, checked parser context and optional numeric simulation are
independent of `execution_point`, original compiler registrations and
`math_functions`. A supplied entry with no expression policy remains incomplete;
consumers must not reconstruct it from the catalogue or physical compiler point.
Host activation selects its own policy and source grammar and restores the
configured user policy on return. Entry comparison and compiler caches include
the retained policy.

Use the Registry queries for each purpose:

| Purpose | Shared query | Consumer obligation |
| --- | --- | --- |
| Prepare original native syntax | `prepare_native_expression_program` | Preserve the original counted bytes, operand, span and physical parser context. |
| Select expression emission | `expression_program_emission` | Distinguish `Native`, `AuthoredSource` and `Unavailable`; do not pool authored results as native compiler literals. |
| Admit dynamic original-source evaluation | `compilation_expression_source_evaluation_supported` | Retain the installed evaluator; function resolution remains a reached, independent obligation. |
| Select a function protocol | `expression_function_dispatch` or `compilation_expression_function_dispatch` | Retain the actual fixed table or command lookup required by that protocol. Parser and numeric providers grant neither. |
| Select an authored fixed function | `authored_math_functions::provider`, `arity`, `prepare` | Require the separately installed authored function capability and preserve checked call topology and operand order. |

`AuthoredSource` uses unchanged expression bytes through `EXPR_STK`, under
explicitly installed parser and numeric capabilities. The checked logical tree
must contain no unsupported raw node. A checked function call additionally
requires the independent `AuthoredMathFunctionProvider`; parser and numeric
capabilities alone do not admit it. This route
does not authenticate variables, substitutions, native object representations or
F5 function registrations. Physical compiler preparation and its dependencies
remain separate. An unavailable capability is a typed refusal, not a Tcl syntax
or missing-command result.

Exercise both installed and missing policy, the same physical host with distinct
logical policies, host restoration, child inheritance and cache withdrawal.
Keep real C Tcl and Jim observations separate from authored F5 simulation and
measured appliance results.

`AuthoredMathFunctionProvider::Tcl84Core` supplies the checked Tcl 8.4 core
function set through original-source evaluation. It grants no native fixed-table
registration, `CALL_BUILTIN_FUNC` admission or native result representation.
Installation requires the authored Tcl 8.4 parser and numeric policies. Use
`Vm::set_logical_math_function_provider` to install or withdraw it; that operation
invalidates captured compilation and evaluation caches.

Each actual interpreter owns its authored random stream and double-formatting
context. The initial seed of one and precision of twelve are explicit,
reproducible simulation choices. They are not measurements of automatic C Tcl
or BIG-IP initialisation. A double uses the current precision when first rendered
and retains that String afterwards. TMM recreation creates fresh interpreter
state; equal worker display names do not identify the same owner. Host activation
uses its installed native core and restores the user policy on return.

Tests must cover capability installation and withdrawal, original argument
evaluation, rejected calls, formatting before and after precision changes,
independent interpreter streams, and native host restoration. Compare the
function and conversion behaviour with real Tcl 8.4 while keeping authored
state initialisation and appliance observations explicitly separate.

### Preserve conditional declaration-body ownership

An original procedure declaration can retain lexical operand layouts even when
its installation is unknown. `DeclaredProcedureBody` owns its original source,
position, candidate, typed publication slot, formal parameters and namespace.
It is independent of an installed command allocation or entered body.

Consumers obtain this information through the existing declaration-layout advice
queries. They must keep it conditional: it supplies no runtime target, alias
cell, successful argument evaluation, normal completion or compiler admission.
Successor collection stops when substituted operands, changed lookup or unknown
effects prevent retention of the original lookup context. Explicit replacement
controls must withdraw the affected advice; an installed body keeps its separate
receipt.

### Select original compiler registration independently of runtime members

Use the captured original registration and the Registry's
`native_compilation_for_original_registration` query for physical compiler
selection. A monolithic command compiler receives its unchanged full source
word vector, including the member selector. A resolved runtime member can have
different release availability, compiler dependencies and body semantics.
Keep those member facts for their own runtime purposes; they cannot replace the
original root compiler. An unknown compiler hook still requires a provider.

Preserve indexed original `WordExpr` operands through native argument emitters.
Reparsing an unpositioned substitution does not recreate its original source
coordinates. Nested words retain their enclosing `Source`, `Derived` or
`Opaque` provenance; only genuinely positioned source words may read bytes from
the retained original source image. Test nested and quoted substitutions,
options and repeated operands, including an opaque-source negative.

The ordinary `upvar` and `info exists` compiler controls retain actual
registration receipts across C8.4–9.1. C8.4 has no hook for either command;
`compiler_hook_presence` remains `Some(false)` and accepted source stays Generic.
Its `{*}` cases retain the original close-brace parse failure and an empty
instruction window, without inventing a command or compiler selection. The
80 upvar and 60 existence windows count those failures as their native outcomes.
The accompanying 140 completion, 20 reached name-header and 15 quiet-observer
windows check actual ownership and result production independently of compiler
selection.

### Keep fresh activation contents separate from outward contents

`VariableBindings::enter_called_frame` establishes the contents world of an
actual fresh procedure or receiver-method activation. Parameters are defined;
other ordinary locals can be absent even when outward namespace contents are
unknown. This closure applies only to the selected activation's local keys.
It supplies no knowledge of globals, enclosing frames or namespace variables.

The fresh activation also retains its own closed variable-observer inventory,
independently of unknown incoming namespace registrations. A bounded newly
allocated receiver can retain a separate receipt for that exact allocation.
Local links resolve to their physical owner before consulting either receipt;
class labels, object names and declaration previews cannot issue one. Known
registrations still observe their exact cells. Unknown registrations, callbacks
and unresolved contents writes withdraw the affected fresh-owner receipts;
joins retain only receipts present on every path, and restoration preserves only
the exact live owner. These receipts supply no value, representation, compiler
registration or opcode permission.

Alias resolution precedes this projection. Unknown dynamic writes, callbacks,
incompatible joins and frame restoration withdraw or conservatively combine
the retained closure. A conditional declaration-body preview does not issue a
fresh activation. Test defined parameters, absent locals, upvar links, unknown
writes, callback withdrawal and differing frame identities. Negative fixtures
must explicitly supply an unknown entry: convenience analysis entry points
author a fresh source-entry contract.

### Resolve cached frame levels against the current caller chain

The native `levelReference` primary stores a parsed numeric level, not a frame
pointer or interpreter identity. C Tcl 8.5 can cache relative and absolute levels;
8.6–9.1 cache absolute levels. Tcl 8.4 has no such primary. Each use selects its
target from the current caller variable-frame chain. A missing, retired or
replaced frame must therefore be reconsidered even when the operand retains its
cache. Moving the same operand to another interpreter does not retain its old
target.

`NativeObjectCacheSnapshot::FrameReference` exposes only the actual release, relative
flag and level. Check the original header's lifetime and selected release before
using it. The native descriptor's free, duplicate, String-update and
set-from-any hooks are NULL; payload transport must preserve those facts without
inventing reference owners or a frame-generation stamp.

Test the same operand before and after entering, retiring and replacing a target
frame, across interpreters, after duplication and after releasing the original.
Keep foreign-release and retired-header refusals separate from ordinary Tcl
frame-selection errors.

### Export guest completions without changing their original owners

At byte-valued engine and browser boundaries, preserve the original guest
completion and its counted result bytes, including embedded NUL and non-Unicode
bytes. A Unicode presentation boundary must use the fallible Unicode accessor.
Host refusals remain host failures; terminal presentation cannot manufacture a
guest completion or guest error code.

When extracting `-errorcode` from completion options, borrow members of an
existing dictionary primary. Converting that same dictionary to a list changes
its primary and child owners. The selected list path remains appropriate for
original list options; String presentation is a separate, reached operation.
Test original stringless dictionaries, original member identities and ownership,
raw guest bytes, and missing native issuers before materialisation.

### Empty native literal registration observations

`NativeLiteralWorld::capture_empty_world` observes the actual shared registration
index without calling an object getter or retaining a native object. Its
`NativeEmptyLiteralWorld` receipt binds the interpreter, registration-world
identity, capture epoch and weak lifetime. Another capture, registration attempt,
registration acquisition/release, world replacement or retirement invalidates
that receipt before native callbacks or owner changes. Populated and foreign
worlds cannot supply it.

`NativeCompilationEntry.empty_literal_world` is optional. The VM captures its
own world at the original entry boundary; Runtime exposes the corresponding
concrete-world observation through `capture_native_empty_literal_world` using
its actual physical compiler epoch. A profile or engine point cannot create the
receipt. The source importer checks the original entry interpreter and epoch,
and source joins require the same current observation. Unknown effects withdraw
this provenance permanently within that source world.

The native observation closes only original literal object effects needed by
an exact fresh command publication. It does not issue a stock literal class,
numeric or container contents, child ownership, successful normal execution,
compiled instruction or command binding. Numeric and stock-literal consumers
retain their separate authored pool requirements.

Fresh authored Jim source retains an independent Text-word effect closure.
`SourceOrdinaryLiteralPool::text_effects_current` supplies this narrow internal
facet to original template preparation: the genuine complete `NativeWord`
arena must contain only Text components, its formed value must remain current,
and abrupt or unknown preparation still withdraws completion. The separate
`effects_current`, object and numeric facets retain their C pool obligations.
A supplied native entry requires its own issuer and cannot become an authored
Jim world. Shared opaque-effect invalidation withdraws the Text facet without
reseeding it. Command effects and selected invocation completion remain
independent. See the
[Jim Text-world contract](../analysis/name-resolution-proofs/original-jim-text-evaluation.md)
and its separate
[exact native source controls](../analysis/name-resolution-proofs/static-original-text-controls.md).

### Retire literal owners after releasing table borrows

`NativeLiteralWorld::release` retires the registration and returns the detached
final owner as `Option<V>`. A remaining registration lease returns `None`.
The caller disposes of a returned owner after releasing every mutable table or
pool borrow. The owner's destructor can release registrations from nested
bytecode in the same literal world.

```rust
let retired = world.borrow_mut().release(registration);
drop(retired);
```

Keep these as separate statements. An inline
`drop(world.borrow_mut().release(registration))` retains the `RefMut` through
the destructor and prevents a nested release from borrowing the world. Pool
adapters follow the same rule for outer pool borrows and inner world borrows;
local slot removal also detaches its value before disposal. Registration
observations are withdrawn before the detached owner can invoke cleanup.

The shared API and registered-bytecode controls test reentrant Rust ownership.
They supply no C Tcl or Jim private destructor, object identity or namespace
chronology claim. See the
[literal owner contract](../analysis/name-resolution-proofs/runtime-original-literal-owner-retirement.md).

## Retain original source values independently of display and cache state

`OriginalProducedNameValue` retains native data bytes, the independently selected
name policy and every admitted source producer. `OriginalWord`,
`OriginalVariableRoot` and readonly `OriginalValue` remain distinct inputs. A
lexical variable root is not its substituted value; a readonly evaluated value
cannot become an editable word or compiler operand.

The original-content stamp is independent of `representation_epoch`. Known
cell writes retire that cell's stored facet and read receipts, while already
frozen immutable data can remain current. Unknown content or observer effects
advance the content stamp; joins with different histories and exhausted stamps
do not revive old values. A normal read separately requires the current cell,
generation, origin, kind, frame and observer closure. Neither known bytes nor
a cache class supplies Normal completion.

`SourceInvocationBinding::original_written_name_input` checks the complete
original source/configuration/vector correspondence before returning a direct
word or frozen readonly value. Frozen values use written ordinals; alias
prefixes and expansion elements require independent effective-origin receipts.
The selected native List serializer is separate from list parsing, command
selection and physical object allocation. Representation-sensitive template
concatenation remains unavailable without its appropriate original operation
recipe; the source-data concatenation adapter admits ASCII non-NUL components.

`SourceImage` caches its byte digest to avoid rehashing immutable buffers.
Equality and ordering still compare complete bytes and the input channel,
including digest collisions. The cache supplies no source or lookup identity.

For static ASCII metadata on a Document channel,
`word_rules::original_static_word_ascii_presentation` uses the complete retained
lexer configuration without requiring or inventing a C/Jim naming policy. It
rejects dynamic/expanded words and grants presentation only, including for
BIG-IP catalogue assistance.

Readonly list and dictionary children count their complete nested producer graph against the shared finite lineage budget. A one-origin child may still retain a long parent chain; constructors and joins must decline when that whole graph exceeds the bound. This limit does not supply a current cell, physical object or successful evaluation.

Closed leaf completion has a separate operation owner. The native source driver
retains the exact source site, complete words, lexer configuration and semantic
operation only after a closed procedure installation or an independently proved
before-store Set operation. It consumes that certificate at the enclosing
invocation and clears it before returning. Normal child bodies, result bytes and
possible Ok routes cannot issue the same certificate.

`OriginalNormalValueWrite` checks the current receiver and selected container,
namespace or activation, write observers and release of the overwritten contents
before the store. Absence closes release without a value; defined contents needs
an independently sealed ordinary producer. A readonly name value or known
representation category does not establish that producer. The source handler
rechecks this receipt after argument and pre-handler effects, before applying the
write. Wrapper frames and callbacks retain their separate completion obligations.

`OriginalVariableInvocation` retains one aligned effective post-head vector for
reads, stores, outputs, alias targets and observer capture. Its entries retain
original words, readonly produced values or independently authenticated list
children; a missing entry never selects a name from a logical display. Registry
transition ordinals, including projected local alias tails, select that vector.
Captured alias prefixes need their own retained value producer.

An `OriginalCompiledVariableOperand` is issued only from the actual retained
inline instruction plan, its target-word ordinal, source site, full syntax
configuration and matching compiler guard. It preserves static compiler naming
separately from a frozen dynamic argument. A selected static operand whose index
or compiler recipe is unavailable remains unknown rather than acquiring runtime
whole-name lookup. Namespace local declarations retain their own
`OriginalCompiledNamespaceLocal` receipt; global/upvar targets still consume the
original materialised operand and independently selected namespace or caller
frame. The normal store certificate and physical native LVT admission remain
separate from these naming receipts.

Source variable effects and retained byte-value reads share this descriptor.
Known logical contents may be installed at its exact captured receiver, but
those Strings do not manufacture native name bytes, an ordinary object class or
Normal completion. Runtime alias destinations select the direct slot before
following older links. Alias targets follow the dialect's cell-versus-name link
policy, with generations and namespace incarnations retained independently.

An absolute-zero original upvar operand selects only a root namespace already
retained in the interpreter inventory. It does not construct a caller frame;
relative caller levels still require their actual selected frame receipt.
Trace registration and removal subjects use the selected trace CString extent,
separately from ordinary counted variable lookup and copied callback prefixes.
A Registry variable role without a selected receiver purpose remains an
original-name coverage obligation and cannot supply a source symbol.

`CommandSpec`, `SubCommand` and `CommandForm` may retain explicit
`variable_receivers`. Their indices belong to the selected descriptor and
`InvocationFacts::argument_offset` projects them into effective argv. Array
set/get/names/unset receivers use the selected combined name grammar. An
element-shaped array receiver can still be a guest error; this naming metadata
does not supply a successful handler or convert `WHOLE_ARRAY_ARG` into a
root-only input protocol. Wrong arity and incomplete roles withdraw the form.
The same descriptor is authorable as `variable_receivers {0 2}` in SpecTcl
and as an optional position list in Studio. Command, member and refinement
drafts preserve omission versus explicit `{}` withdrawal through both
renderers and the loader. It adds no native execution contract. The selected
facts retain the descriptor-local positions until `argument_offset` projects
them into the actual argv.

Source variable symbols are readonly navigation identities. Intrinsically
qualified roots preserve their native namespace or Jim flat-key geometry.
Bare namespace roots require unanimous original point-selected cell ownership;
the lexical scope cannot supply C8's namespace/global fallback. Procedure and
method locals retain an independent original body/frame recipe. C local names
must agree with their selected compiled primary, while Jim checks its own
direct scalar setter/formal installation. Known alias reads separately verify
their actual point against the sealed declaration relationship. An advisory
local alias spelling is distinct from its target symbol and its rename policy.

The analyser also retains static syntax occurrences before processing each
genuine complete source command vector. Those occurrences preserve the actual
root image, input channel, full configuration and source site even when an
executed result is unavailable. Conflicting or presubstituted vectors decline.
This map supplies no Registry role, namespace existence, entered cell, compiler
preparation or Normal completion; consumers retain those independent owners.

`OriginalVariableWriteAdvice` combines a retained original name producer with
unanimous conditional Registry write-role metadata. It supplies a source card
even when current cell identity is unavailable. Its optional frame comes from
the genuine original body entry, and its optional lexical namespace does not
select storage. Reads, unsets and alias declarations have separate metadata.
Navigation and rename still require `SignatureSourceVariableOccurrence` and
their complete source/alias coverage. Alias declaration navigation returns the
actual local operand span, independently of the remote cell target and whether
renaming the target changes that local spelling.

Static-word tail extents use `NativeCompilerWords::original_literal_extent`.
Literal channel units and complete scanner-selected escape results map through
the shared Syntax extent owners; an interior native byte cannot fabricate a
source boundary. Lexical substitutions additionally require their retained
root membership. Formal-list children retain their own original list geometry.
These readonly coordinates supply no replacement syntax or compiler capability.

Conditional SSA destruction advice retains the actual written removal ordinal
and projects its key through the original declaration snapshots, including
their typed alias bindings. It does not reparse the diagnostic `removes` names
in the caller's current table. The original-advice flag remains separate from
an actual Must destruction; missing input, primary agreement or unanimous
original cell ownership withdraws the conditional key.

Cross-event variable cells retain exact native name bytes and the separately
supplied interpreter, worker, epoch, connection and storage domain. A join of
unequal execution or interpreter owners withdraws contents, representation,
observer and binding certainty. Source navigation symbols cannot publish a
RULE_INIT value, a per-TMM value agreement or an event-runtime cell.


The namespace editor takes `OriginalNamespaceSymbol` and uses
`original_namespace_rename_edits` to collect authentic namespace inputs,
import/forget patterns, publication metadata, command lookups and qualified
variable occurrences. It compares byte geometry and complete source/configuration
currency before planning edits. `OriginalNamespaceSymbol::renamed` supplies
pure proposed namespace geometry for collision comparisons; it grants no
publication or namespace lifetime. Global command/variable simple tails are
excluded by their own naming purposes even when their spelling equals the
namespace being renamed.

Static words and readonly static-list children go through
`original_name_input_edits`, which reparses complete containers and preserves
unrequested values, cardinality and expansion markers. Lexical variable roots
use `original_variable_root_name_edit`, which checks their literal native units
and reparses the complete substitution while preserving wrappers and the
separate index. Neither path creates an editable word key for a computed value.

`namespace_name_unknowns` carries Registry assistance positions whose original
value producer is unavailable. These are May obligations only. The namespace
editor inspects original lexical arenas under the retained full grammar;
absolute fixed C prefixes can rule out unrelated namespaces. Unknown prefixes,
unsupported receiver contexts, exhausted lexical coverage and readonly computed
values that name the selected namespace refuse atomically. Deferred callback
prefixes require their independently selected receiver protocol rather than an
installer-head or list-child assumption. Jim absolute variable keys remain
counted root-flat keys, without a namespace/tail relationship donated by
CString reporting. Lexical-only compatibility editing is available only when
the analyser independently selected the absence of a native naming policy.


### Source class and method candidate inventories

`class_hierarchy::original_metadata::original_instance_metadata_order` joins
retained class relation operands to exact declaration publications before the
shared generic MRO walk. Its nodes are immutable declaration records. Missing
source providers, ambiguous publications and superclass cycles withdraw the
order; a registered root or a class-object maker is not invented from a label.

Core `original_oo` consumes original lookup inputs or independently retained
class allocations for hover and class-command declaration assistance. Method
labels and insertion words come from `OriginalSourceMemberLedger` bytes, with
source-channel rendering and whole-selector edits through `original_name_edit`.
The same byte candidate view feeds prefix and fuzzy completion. Registry object
specifications are an authored advisory surface kept separate from source
class metadata. Canonical receiver method navigation still requires a genuine
call-point `SourceReceiverMethodEntry`; final metadata and advisory inheritance
do not supply execution, native delegate attachment or method-table currency.

Registry `DefinitionMemberNamePurpose` separates TclOO counted method names
from generic definer source values. Registered/maker provider topology and
per-object member effects require their independent owners. This source-only
view does not fill those gaps with reporting maps or infer Normal execution.

`method_symbol::local_candidate` provides the readonly local Core method
selection used by definition, references and hover. It consumes the genuine
analysis inventory directly, without building a workspace. The Workspace
`method_symbol::candidate` adapter uses the same canonical declaration and lookup
kernel. Both retain the original class/selector inputs, `MemberSide`, complete
consumer source image and lexer configuration. A known original query whose
receiver or target is unavailable is terminal; `instance_classes` and displayed
member names cannot supply a missing owner.

An authentic temporal `SourceReceiverMethodEntry` selects its canonical source
member. A pure external class lookup instead supplies declaration advice after
checking earlier occupied procedure and class publications. The explicit
`member_metadata_candidate` door accepts an independently selected class record
and genuine selector input. Instance advice delegates to
`original_instance_metadata_order_for_inventory`, retaining each declaration's
URI, authentic relation lookup and earlier occupied non-class publications.
Class-object advice covers only the selected
source own table; it supplies no maker, object-mixin or native-delegate order.

`local_reference_spans` and Workspace `original_method_reference_rows` compare
canonical declarations, not displayed names. Readonly rows retain the actual
consumer image/configuration and optional declaration inclusion. Identical
complete views within one URI are deduplicated, while equal documents under
different URIs remain independent allocations. Nested members retain the
complete parent source image; an unrelated parent change withdraws selection.
Server queries recheck each current document before converting a span or
rendering hover. These candidates supply neither edit permission nor an entered
object or native dispatch.

### Property declaration advice

`CommandRegistry::native_class_factory_recipe` selects the stock constructor
and support roles under the actual retained release and command specification.
`OriginalClassFactoryState` keeps the current factory and support allocations
independent of class body metadata. Existing classes can retain a moved support
allocation, while a later configurable factory call must reselect the support
slot. Unknown native topology and custom factory state cannot be supplied by
bootstrap source metadata. The recipe establishes neither successful factory
completion nor a live native class hierarchy; its [source-analysis
contract](../analysis/name-resolution-proofs/tcloo-original-configurable-factory-support.md)
is separate from the captured configurable controls.

`OriginalClassFactoryTransfer::capture` independently requires the fresh
destination, current intrinsic factory/support allocations, exact static argv,
selected workers and complete stored definition script. `completed` then
checks the same source/site/configuration and exact resulting allocation and
publication after the canonical transfer. Only those complete pre/post
premises preserve the shared normal-handler certificate. Dynamic operands,
occupied destination cleanup, custom factories and unknown callbacks remain
unavailable. The [conditional transfer
contract](../analysis/name-resolution-proofs/tcloo-original-stock-class-factory-normal-transfer.md)
does not execute stored method bodies or supply a live native class, physical
object identity or CPP execution receipt.

`native_property_declaration_validity` selects the pure C9.0/C9.1 declaration
validation kernel. It returns name validity or the native byte diagnostic;
it does not issue a physical property lookup recipe for C9.0. Counted property
names and the validation's CString projection retain separate purposes.
Installation, accessor entry and cache currency still need their own owners.
The [declaration-validity
contract](../analysis/name-resolution-proofs/property-original-c9-declaration-validity.md)
binds this pure consumer independently of physical property observations.

`OriginalSourcePropertyLedger` retains each property's counted name, selected
receiver side, typed kind and independently retained custom body inputs.
`DefinitionBodyGrammar::source_property_declarations_bytes` owns the per-name
option layout: one property's options do not alter the following property.
`declarations` preserves canonical syntax, while `properties(side)` folds the
latest exact name and policy or reports unavailable coverage. Class and
own-object configuration inventories transport their own property deltas.

The selected grammar's `accessor_methods` supplies authored completion
proposals. These use the shared native source-word renderer and selector edit
planner. A proposal describes source declaration advice; property syntax alone
does not prove an eligible native target, an installed accessor, property
option lookup or a current own-object overlay. An ordinary class cannot acquire
configurable execution from a metaclass reporting label. Native property
admission and physical lookup remain separate purposes.

### Variable completion from original occurrences

`completion::original_variables` checks the full consumer image and grammar
before enumerating original variable occurrences. Local and formal candidates
require `original_variable_frame_in_source` at the cursor; a matching scope
label or an enclosing body range cannot supply a frame. Local alias spellings
come from `original_local_alias`, separately from the target symbol and link
execution. Namespace candidates retain their selected native geometry. Jim's
flat variable keys do not acquire a C namespace/tail partition.

A candidate insertion must reparse as an actual executable variable reference
and reproduce the selected native root and independent index. The literal-text
renderer roundtrips the input channel and string protocol. Whole-word numeric
escapes cannot stand in for substitution inside a braced variable root;
unrepresentable roots are omitted. Complete-word rendering remains available
for an index whose actual grammar performs substitutions. These checks grant
source insertion geometry, without a current value or a successful read.

The [empty array root observation](../analysis/name-resolution-proofs/empty-array-root-lexical-reference.md)
distinguishes C `$(k)` from Jim's expression syntax. The editor's C empty-root
geometry assertion remains separate from that native experiment. Explicit
lexical declaration advice requires
`AnalysisResult::allows_lexical_declaration_advice`. The actual retained input
selects this domain independently of occurrence inventory: hosted source policy
or ContextRegistry refuses compatibility even for an empty document. A failed
known-native candidate cannot use reporting `VarDef` names to recover completion.

### Outline declaration inventory

`document_symbols::original` uses original procedure, class, variable,
namespace, member and property metadata. Canonical declarations retain their
source geometry. Known member moves and deletions use the ordered source
ledger for their current labels; lexical containment and typed namespace homes
remain separate relationships. Registry test, matcher, constraint and event
cards retain their own selected declaration advice. Body ranges and labels
serve presentation, without command publication, method selection or execution.

Missing original metadata does not activate reporting-map identity. Computed
member declarations require their own readonly producer, and later object
configuration requires a separately current receiver before it supplies an
own-table overlay. The outline can preserve explicit logical-only reporting
when no native name recipe was selected.

### Semantic token roles

Semantic token declaration roles use complete original procedure/class words and canonical member and property declaration occurrences, including independently retained own-object configuration syntax. The original variable inventory supplies static declaration-operand roles. These roles survive reporting-map changes but grant no publication or dispatch authority. The token walk builds this source-geometry index once per request and checks the complete image and lexer configuration.

A source method-call role requires `receiver_identity::method_at_command` at its authentic post-argument point. Its selected original input travels with that receipt; a computed selector keeps its substitutions rather than becoming a static method token. Known original heads do not use the String class hierarchy to fill a missing source dispatch receipt. Registry object specifications remain separate advisory facts. Original class-head matching is declaration assistance, independent of whether a manufacturer executes.

### Readonly declaration identity and editor wire data

`original_declaration::OriginalDeclarationIdentity` is the common readonly
source identity for procedure, class, method, property and nameless lifecycle
syntax. Issuers require an actual original declaration record in the current
analysis, retain the owning URI, complete source image and lexer configuration,
and keep declaration and class-owner allocation sites separate. Effective
method routes select the canonical declaration, metadata and receiver side;
a moved selector does not become a new declaration. Constructor and destructor
identities use their original keyword, formal-list and body words without
inventing a method-name input.

`OriginalDeclarationDocument` carries each independently owned current source
and analysis to reference and hierarchy consumers. A carrier supplies no
complete-inventory or execution claim by itself. The server gathers original
procedure/class owner URIs, indexed documents, open buffers and the selected URI
through `original_declaration_documents`. An invalid, unreadable or stale owner
refuses the whole inventory, so removing an earlier occupied header cannot
create a later unique candidate. Equal text under different URIs retains
independent owners.

`OriginalClassInventory::from_documents` is the checked readonly class
inventory shared by type hierarchy and method implementation queries. It
validates every original class and procedure header in each complete current
source, preserves equal text under different URI owners, and retains non-class
publications that can occupy an earlier relation candidate. Duplicate URI
owners, changed source or changed scanner configuration refuse the inventory.
The server's `original_declaration_document_participates` checks the resolved
environment, contributed language identity and workspace source extensions
before requesting Tcl analysis. JavaScript buffers, BIG-IP configuration and APL
presentations supply no Tcl headers. An unknown relevant Tcl provider remains a
terminal obligation; exclusion does not supply a naming policy or permit a
cross-dialect relation join.

`original_indexed_location::OriginalIndexedSourceLocation` retains an indexed
variable occurrence or namespace fact together with its independent URI and
`WorkspaceDiagnosticSourceContext`. `for_analysis` supplies the same readonly
complete image/channel, full scanner configuration and advice mode used by
index capture. The context also retains the actual Registry structural semantic
key. Equal source and scanner configuration cannot preserve currency after that
Registry changes or becomes unavailable. Constructor success retains a candidate; it does not attest that
the current document still contains it.

After reading the owning document, call `validated_span(source, analysis)` with
that document's current analysis before returning a location or adding any
hover count. The validator requires complete source/configuration/Registry agreement,
the exact current original fact and bounded source geometry. Missing or
conflicting facts withdraw the span. Namespace membership uses original input,
scope, policy, declaration role and span; cleared display labels and optional
presentation ranges cannot select or erase the fact. An implicit parent uses
the existing authentic declaring-descendant prefix owner after membership is
validated. `uri`, `is_declaration` and `is_implicit_namespace_parent` preserve
ownership/classification only. This [source-location
contract](../analysis/name-resolution-proofs/original-indexed-source-location.md)
supplies neither lookup nor native cell, namespace existence, completion or edit
permission.

`invocation_targets_declaration` checks the actual original input against the
retained lookup and the reference's issuing command site. Its independent-owner
form checks both consuming and declaring complete sources. Direct editable
references also compare the called original slot and naming policy; call edges
may follow an already retained terminal implementation allocation. Neither form
constructs an input from a displayed qualified name. Missing original lookup is
terminal. `literal_name_matches_publication` is a separate API-value projection:
it applies the selected document ingress to a literal Unicode value and performs
root publication matching without evaluating script escapes or issuing a source
operand. Procedure tools and literal class queries share this projection.

`original_call_hierarchy` prepares source declaration candidates and groups
incoming/outgoing edges by genuine declarations and URI owners. Earlier occupied
non-procedure headers and ambiguous providers stop lookup. The method branch
uses the shared local or Workspace selector and exact original metadata order;
source candidates supply neither entered object nor native dispatch. Code lenses
enumerate original declarations independently of UI maps. Property and lifecycle
syntax retain source identities without inventing generated accessor references
or positional lifecycle call counts.

The bounded `OriginalDeclarationRegistry` retains full readonly receipts and
returns compact versioned handles. Hierarchy and lens wire data contains only
`originalDeclaration` locators. Resolution reselects the actual current
URI/source/configuration and declaration record; parsing a token, changing a label
or returning another range supplies no original input, publication, native entry
or edit permission. Stale or unknown locators decline, and a stale lens command
is cleared. Item rendering occurs after canonical selection in the target's own
current source document.

Type navigation uses a genuine original method candidate to select its canonical
declaring class. `method_implementations_in_inventory` shares the checked class
inventory and original relation kernel, keeps each target's URI, and separates
instance descendant overrides from class-object own-table declarations. Opaque
member labels and moved selector routes do not replace the canonical declaration
or receiver side. Procedure declaration advice similarly returns only a unique
original candidate; multiple redefinitions supply no advice by source order.
Positioned command references remain the separate owner of actual call targets.

### Original Jim script error locations

`Interp::eval_native_jim_script` retains the actual original Script filename,
line and current procedure level until error capture. A validated Script entry
resets automatic capture; parser failure uses its pre-frame Source location,
and an entered error is captured before the actual evaluation frame is popped.
Failed lookup retains its authentic empty invocation. Explicit traces and
completed earlier captures keep first-capture ownership. Do not reconstruct a
procedure frame after unwinding or substitute a rendered script's location.
The [pinned Jim source windows](../analysis/name-resolution-proofs/jim-original-source-entry.md)
explain source transport only; they issue no new execution, cache, compiler or
opaque filename result.

### Original variable read diagnostics

Retain the failed operation's original input independently of its reached
receiver. `report_native_c_variable_value_name` presents original whole or
separate compound parts through the selected C recipe; it does not reselect a
cell from a reporting name. The Runtime's `original_c_variable_failure_input`
and `body_read_original_parts` preserve the Read purpose through late failures.
Read and Write presenters cannot substitute for one another. A captured
increment retains its own original operands and selected failure protocol.

The C `ValueWrite`/`IsArray` failure tuple selects only the actual array-root
write site. `report_native_variable_diagnostic_at` keeps that operation and
failure site separate from Read and NameLookup. The finite source-file controls
record NONE for C8.4/8.5 and TCL WRITE VARNAME for C8.6–9.1; current Jim instead
replaces the array with NEW. These source results and the Rust site-table
comparison establish no original receiver identity, physical storage or
compiler admission. See the [array-root write question](../analysis/name-resolution-proofs/variable-original-array-root-write-diagnostic.md).

The [six Read source controls](../analysis/name-resolution-proofs/variable-original-read-diagnostic-input.md)
retain literal, dynamic-head and procedure outcomes for C array roots and Jim
dictionary roots separately from scalar-element failures. The
[recreated-element control](../analysis/name-resolution-proofs/variable-recreated-element-independent-read-trace.md)
retains the exact requested compound name independently of the captured old
receiver and new callback. Supplementary-character extent controls are Rust
ownership assertions; those ASCII captures establish no opaque native name or
private receiver layout.

### Original variable read replacement

`SsaSourceView::read_produces_value_at` checks the exact retained source
substitution and every original context alternative for value production.
`replaceable_read_at` additionally requires a quiet bounded scalar cell with a
current generation and the matching typed cell and SSA version. These queries
supply no editable extent, store-deletion permission, alias continuity or
permission to move a producing expression.

The inline-variable consumer retains the complete current image and scanner
configuration, a literal store whose contents survive, and one later same-block
operand read of that exact definition. It uses the read's original source
extent and independently checked value rendering. Reassignment, aliases,
observers, unknown residuals, stale source and evaluated values decline. The
native fixture `native_inline_variable_semantics` records the pure source,
Unicode grammar, reassignment, callee-upvar and read-observer controls
separately; its handwritten replacement scripts do not certify a Rust edit.
The file-script and interactive-stdin receipts preserve their own source,
startup, exit and raw streams. A zero interactive shell exit with an error in
stderr does not certify a successful variable read.


### Original enclosing-body selection

`selection_range` uses the analysis ingress's retained profile. The dialect
entry point accepts semantic body links only when that profile and the complete
current source/configuration correspond. Original procedure and class records,
named member declarations and nameless constructor/destructor words retain
their own source sites. Repeated declarations and byte-distinct names remain
independent even when their optional display labels coincide. Configuration
workers contribute their independently selected body syntax; a selection range
does not settle their class target or select an installed method.

Containing spans must form a nested source chain. Foreign records, missing
current source/configuration and conflicting geometry omit semantic body links.
Syntax word, command, line and document ranges remain independently available.
The explicitly selected logical declaration-advice branch is a separate
compatibility surface; unavailable Native metadata cannot activate it.

### Exact scalar source proposals

`native_scalar_source_spelling` accepts original bytes, the source channel,
complete `LexerConfig` and selected native name protocol. It requires unchanged
unqualified scalar interpretation, then validates its literal name word and
complete braced reference through the shared source renderer and variable
scanner. The proposal supplies source syntax only. A refactoring must separately
prove availability, binding/cell identity, value production, observer closure,
lifetime, editable source correspondence and permission for every changed
operation. Braced Unicode native controls support their exact source programs;
they do not turn a spelling proposal into an edit certificate.

`SsaSourceView::fresh_scalar_variable_proposal` adds a separate point-owned
absence receipt. It requires a genuine reached invocation, an actually entered
local activation, a complete tracked contents inventory and independently closed
activation observers. The selected name must retain unchanged native scalar
geometry, with no occupied or retired slot, alias, unknown binding or possible
read/write/unset callback. Global, namespace and iRules event storage cannot
borrow local freshness. This receipt neither selects a generated setter nor
proves its new value, Normal completion, source movement or edit; conditional
future entry needs its own source-assistance owner.

### Original variable operands and SSA type advice

The original handler boundary retains the complete effective
`OriginalVariableInvocation` once, including actual written/expanded/captured
origins and independent compiler/runtime receiver recipes. Conflicting
observations withdraw the descriptor. Normal definition and transfer consumers
use these exact operands for writes, aliases, traces and destruction; selected
original policy never reparses a missing operand's display label.

`SsaSourceView::original_definition_name` pairs an actual definition's byte cell
and binding phase with its genuine written receiver input. It grants source
correspondence only, without a value, future lifetime, Native representation,
Normal operation or rename permission. Variable type hints match the exact SSA
symbol/version and original declaration input under complete current source and
configuration. Readonly write-name cards remain a separate source inventory
when an executed cell cannot be selected.

Use `def_places_at` with the actual CFG block/index and retained point contexts
when deriving those definitions. A specialised `AssignConst` has no statement
token vector; its genuine `source_tokens_at` still retains the original native
operand and Normal continuation. The absence of statement tokens must not make
the first definition use authored escape text while later definitions use the
decoded native key. Original opaque accesses or missing point ownership remain
unknown. SSA symbol labels are retained only after that exact cell selection.

`SignatureSourceFormalParameters` exposes readonly native ParamList bytes and
the independently selected required/default/rest/link binding grammar. Its
analysis facade requires exact retained procedure metadata and a unanimous
Registry declaration recipe. A reporting parameter list cannot supply missing
original topology. This source signature neither installs formals nor creates
an activation, caller link, current cell or Normal result.

`OriginalReceiverBodyContext` retains the actual entered method's declaring
provider, explicit Instance/ClassObject/OwnObject axis, byte formal topology,
activation and complete original source/configuration. Unanimous entered
observations supply method IR facts; a lexical body alone supplies neither a
provider nor a receiver. Original method facts do not seed object variables from
a class-wide reporting-name union. Own-object and class-object variable
inventory remains unknown unless its own lower owner proves it. Original
contexts also preserve a withdrawn interpreter or worker/event domain instead
of filling it from a compatibility overlay.

`AnalysisResult::original_scalar_body_alpha_rename` is a separate semantic
equivalence query. Its bounded subset is a closed complete source containing
one surviving procedure, one required direct scalar formal and a sole selected
builtin `ReturnResult` whole-object fetch. It independently requires the genuine
declaration frame, exact topology, quiet read/command effects and an unchanged
handler allocation. The transformation preserves argv, activation and release
order and introduces no store or conversion. Full rename coverage and
authenticated formal/list-child and lexical-root edit geometry remain separate
obligations. Extra source/body commands, introspection, observers, default/rest/
link/array roles and missing ownership withdraw the receipt. Arbitrary external
procedure introspection is outside this closed-source contract.


### Original diagnostic dependencies

`WorkspaceDiagnosticSourceContext` retains actual document image/channel, full
scanner configuration, actual Registry semantic identity and independently
selected original/logical advice mode.
It supplies diagnostic invalidation currency only. Indexed diagnostic facts
compare original procedure/class rows and surviving source publications, with
their exact byte slots, policies and allocation sites. A display-name or arity
projection cannot retain an unchanged verdict after original ownership changes.

Explicit Logical slot advice requires the complete actual source-input token
validated by `SourceAnalysisOptions::retained_logical_source_input`. Entry,
baseline, cache and lowering retain that same token;
`logical_source_name_advice_input` supplies the readonly domain to presence and
diagnostic-presence queries. An original source site alone cannot select
Logical advice. Native, hosted, missing and foreign input remain separate from
definite lexical absence and from any loaded runtime callable.

`ResolvedAnalysisInput::has_hosted_source_name_context` selects the hosted naming
domain from the actual retained source policy or ContextRegistry. Empty and
unavailable original occurrence inventories do not enable Logical compatibility;
reporting dialect labels cannot alter this decision.
`AnalysisResult::allows_lexical_declaration_advice` is the shared branch owner,
and `WorkspaceDiagnosticSourceContext::for_analysis` captures that same decision.
Server `settle_cross_file_calls`, its per-call resolver,
`cross_file_arity_diagnostics` and `refine_workspace_index_w123` require this
explicit Logical branch before consulting reporting name sets or arity rows.
Original unresolved subjects remain present despite matching math-function
labels, opt-in project tails or supplied reporting call spans. The helpers
provide no independently loaded cross-file callable entry. Genuine original
declaration identity and readonly source candidates retain their separate
source/URI/configuration/Registry owners. See the
[workspace diagnostic contract](../analysis/name-resolution-proofs/original-workspace-diagnostic-refinement.md).

The server matches actual caller inputs and ordered original lookup dependencies.
Unknown, conflicting or stale relevant caller inventories conservatively wake
all relevant open Tcl sources. The shared participation owner excludes foreign
JavaScript, BIG-IP configuration and presentation surfaces before this query.
Scheduling supplies no callable command, runtime world, declaration selection,
cell value or edit permission. Reanalysis and publication retain those separate
obligations under each document's own current source/configuration.

`WorkspaceIndex::allows_lexical_rename_advice` is a separate compatibility
eligibility query for consumers that lack an analysis argument. Every active
document must have its own captured context and independently select lexical
advice. An empty inventory or a missing context refuses. Native and hosted
vendor contexts refuse even when their reporting dialect labels change; empty
hosted documents use the actual retained execution environment. This getter
does not validate current source or authorise edits. A consumer still needs its
independently selected complete rename plan. The [workspace eligibility
contract](../analysis/name-resolution-proofs/workspace-lexical-rename-advice-eligibility.md)
keeps this compatibility decision separate from command lookup and publication.

### Ordered source compiler locals

`SourceCompilerLocalInventory` retains complete ordered source compiler
allocation events, including represented anonymous positions, under the actual
source, configuration, frame and compiler context. `preflight_image` issues it
from the same authenticated traversal; `OriginalSourceVariableCompilation::local_inventory`
exposes it only after compilation correspondence. Unsupported attempted
allocations or missing preparation ownership withdraw completeness. The
inventory does not supply a physical local table or an activation.

`OriginalCompiledVariablePrimary` is privately issued from that inventory's
first counted small-index primary. `resolve_original_compiled_primary` then
uses the existing exact cell, alias, observer, declared-variable and generation
owner. Runtime dynamic names retain their independent addressing projection.
Large-index or missing results in an owned inventory decline instead of
silently selecting the requested runtime spelling. Independently complete
formal-prefix or byte-unique proofs keep their own premises. The [ordered
inventory](../analysis/name-resolution-proofs/ordered-compiler-local-inventory.md)
and [primary-cell correspondence](../analysis/name-resolution-proofs/ordered-compiler-primary-cell.md)
issue no successful read, write or Normal completion.

### Authored source interpretation reuse

`SourceAnalysisCacheKey::at_entry` retains complete source bytes and channel,
full scanner configuration, original frame, owned entry contracts, selected
policies, realm/compiler request and Registry semantic identity. Retained native
entries and oversized sources remain ineligible. `lookup` clones independently
mutable builders; the bounded worker memo remains outside semantic equality and
hashing. `SourceCommandBindings::analyse_source_in_frame_with_options` remains
the sole interpretation producer. The [memo contract](../analysis/name-resolution-proofs/authored-interpretation-memo.md)
supplies no physical provider freshness, compiler admission, table/activation,
Normal completion or timing guarantee.

### Completed source-world occupancy advice

`OriginalCompletedCommandWorld::command_slot_occupied` requires an independently
complete Normal root world, exact `ByteCommandSlot`, retained naming policy and
known namespace holder. It queries the same canonical source/imported/baseline
binding alternatives: definitely missing reports false, unanimously occupied
reports true, and unknown, empty or mixed alternatives decline. Registry
metadata cannot fill missing supplied native entries. This [occupancy
query](../analysis/name-resolution-proofs/command-original-completed-cell-occupancy.md)
supplies readonly collision advice; it does not select an implementation or
issue a new completion, insertion or edit permission.

### Original Registry header advice

`original_registry_invocation_assistance` authenticates the complete original
word vector and prefers an independently retained execution envelope. Its
conditional fallback uses the existing closed original declaration-layout
issuer for readonly metadata. An uncalled stored body can retain a signature
without an actual Normal result or completion receipt; independently available
conditional variable-transfer metadata remains separate. Changed syntax,
missing ownership or source-defined handler shadowing cannot borrow stock
Registry shape. `selected_registry_words`
keeps effective original argument origins through editor advice. This [header
contract](../analysis/name-resolution-proofs/original-registry-header-advice.md)
does not supply execution, stores or compiler admission.

Hosted source metadata uses `vendor_registry_invocation_shape` with the genuine
vendor head, independently selected context, complete source and scanner
configuration. Supported plain ASCII source units retain their actual operand
ordinals; unsupported values and expansion cardinality stay unknown. The
SymbolDef consumer declines expanded vectors rather than remapping them. This
[hosted metadata contract](../analysis/name-resolution-proofs/vendor-original-registry-metadata.md)
requires a matching actual analysis `ContextRegistry`; the shared
`resolve_structured_invocation_in_resolved_context` retains that complete
context, keyed versions, overlays and command-store identity. A static profile
context cannot replace the actual generation, and an explicit hosted context
without a matching registered owner declines. Readonly receipt equality compares
all retained context fields, original inputs and command-store content; equal
environment labels or a digest alone cannot identify it. This supplies source roles and
cards, with no C/Jim recipe, executable argv, shadow
closure, native lookup, appliance result or runtime completion.

### Atomic original command rename plans

`original_command_rename::select_in_documents` retains a genuine current
procedure/class identity or independently positioned direct reference.
`original_command_rename_edits` then requires every independent URI/source/
configuration, an unchanged original publication and its completed Normal
source world. The replacement literal value uses the retained native ingress
and command-component recipe. Every destination and ordered lookup alternative
must agree with canonical occupancy; a missing holder or unknown alternative
refuses the entire plan.

The complete actual source traversal requires declared transition knowledge and
selected executable-region ownership. Aliases, observers, callback prefixes,
reflection, command-data name references and readonly expression identifiers
retain separate edit obligations. Each declaration and direct call uses its own
static source container; missing containers, overlaps and ambiguous owners
refuse all edits. Prepare uses the same unchanged-tail obligations before
projecting a written component range. Server transport collects and revalidates
each current source and Registry owner before publishing the combined plan.
Native method-family edits without their own complete typed override authority
remain terminal. The [command plan contract](../analysis/name-resolution-proofs/original-command-rename-plans.md)
supplies editor replacements, with no runtime execution, insertion, allocation,
new Normal completion or arbitrary reflection-equivalence guarantee.

### Hosted source declaration consumers

`vendor_declaration::declarations` validates the complete consumer image and
scanner configuration against every retained hosted procedure, class and
Registry symbol header. `select_at_offset` selects only direct header geometry.
Supported vendor units or exact unavailable content spelling supply readable
source cards; neither supplies a C/Jim command slot or runtime publication.
`source_word` permits a suggestion only when supported units survive an
independent full-grammar parse under the same vendor producer.

Outline, procedure tools, direct definition/hover and completion share these
actual metadata rows. Semantic declaration roles require supported hosted units;
unavailable units stay readonly cards without fabricated roles or suggestions.
Hosted variable suggestions use the independent actual body/formal advice and
exclude other lexical homes. Clearing reports cannot erase authentic headers,
while changed complete source withdraws assistance. The [hosted consumer
contract](../analysis/name-resolution-proofs/vendor-original-source-declaration-consumers.md)
supplies no measured appliance build, Native lookup, runtime cell/class or edit
permission.

### Own-object method source ownership

`receiver_identity::original_own_method_metadata` joins an independently retained
own-object entry to its exact configuration allocation and canonical source
worker. `method_symbol` carries that object owner separately from any declaring
class. Its query retains the genuine selector input, actual entry generation
when available, complete consumer image and full scanner configuration. An
object-valued head supplies no reconstructed class input. A private own entry,
changed generation or unavailable receiver cannot borrow a same-named class
method or class-object table.

`OriginalMethodCandidate::own_object` exposes the readonly declaration owner.
`OriginalDeclarationIdentity::for_member_candidate` preserves it with the
independent URI and original declaration words. Definition, hover, references,
call hierarchy and lenses use that same canonical source identity. Wire handles
reselect it in the owning current document. Equal labels or source bytes under
another URI do not merge object allocations, and stale source/configuration
withdraws the handle. These APIs supply source navigation; they do not install a
method, establish native dispatch, reveal a current runtime object or issue edit
permission.

### Effective signature arguments

`original_invocation::selected_registry_words` retains unanimous selected
Registry advice with its genuine effective argument vector and producer origins.
`OriginalRegistryWords::active_argument_at` and `effective_argument_at` map the
cursor through those origins. Captured binding prefixes occupy argument slots
without receiving the caller's source anchor. An expanded list child requires
its independently retained extent and exact value correspondence. Ambiguous
cardinality or multiple matching cursor operands declines the ordinal.

Signature help checks the complete consumer image, full scanner configuration
and actually retained Registry/profile. Changing a reporting dialect or removing
a display declaration map cannot change that owner. A current parameter ordinal
is readonly signature advice; it supplies no future callback lookup, alias
availability, native entry, successful invocation or editable extent.

### Shared workspace references and lenses

The server's `original_declaration_identity_at` selects one canonical source
identity through `original_call_hierarchy::prepare`. Reference and lens consumers
share `original_declaration_reference_locations`, which uses the complete
participating document inventory and the same Core reference kernel. Declaration
inclusion is an explicit independent choice. Missing original lookup, duplicate
providers or unavailable relevant source withdraws the selection rather than
reparsing a reporting name.

Each returned span retains its own URI and source owner. After asynchronous
document reads, the server checks every complete source/configuration again
before projecting UTF-16 locations. `span_to_range` is a pure coordinate helper;
it supplies no declaration identity. A location or reference count neither
proves live native dispatch nor grants source editing.

## Plan minification by independent source and naming purposes

`minify::minify_with_analysis(source, analysis, MinifyOptions)` is the shared
entry for document consumers. It retains the actual resolved Registry/profile,
complete `SourceImage`, and every `body_lexer_config` axis. A stale or missing
owner returns unchanged source with a typed `MinifyRefusal`. CLI drivers that
already hold profile/store inputs use `minify_with_profile`, which installs
those exact inputs before passing the analyser its presentation label.

Native lexical compaction uses `SourceStructure::capture_for_syntax_compaction`
and `native_script_words_in` over the complete original image. Descendant script
roles require selected current source metadata and an independent original
operand lookup-preservation facet. Conditional or hosted schema advice,
declaration bodies and incomplete-body candidates cannot supply this facet.
The readonly `SourceStructure::capture` inventory remains independent. Compaction
changes only authenticated comment/separator gaps, including independently
selected descendant script gaps. Ordinary data, blocked roles and unsupported
cooked body geometry retain their written bytes. Every emitted region checks
command/word counts and original written words after those selected descendant
edits under the same complete configuration. Alpha output is reanalysed with
the same ResolvedAnalysisInput before selecting body geometry. Unknown owners,
malformed tails or changed correspondence refuse. Lexical correspondence
supplies no runtime reflection, selected handler, name, cell, frame or broader
edit equivalence.

Native scalar compaction consumes the separately issued
`AnalysisResult::original_scalar_body_alpha_rename` receipt. Its bounded source
contains one surviving procedure, one required scalar formal and one selected
whole-object return. `scalar_alpha_compaction` checks the exact declaration,
complete image/configuration, Registry semantic key and proposed native tail,
then separately calls `original_variable_rename_edits` for complete original
formal/list-child and lexical-root coverage. It introduces neither a store nor
movement and preserves the public procedure name. Reported symbol-map labels
only describe the proved change; they never select its declaration or frame.
Missing editable coverage reports `AlphaRenameGeometryUnavailable`. Source
syntax compaction subsequently checks the exact edited word sequence under the
same configuration; it does not rebuild a command world from the edited text.

Other name compaction, semantic transfer, alias insertion, and keyword selection
have separate eligibility obligations. Native planning returns
`MissingAlphaRenameContract`, `MissingAliasInsertionContract`,
`MissingSemanticRewriteContract`, and `MissingKeywordSelectionContract` when
those requested passes have no authentic receipt. An `isolated` flag, a fresh
name proposal, a typed source reference, or an SSA label cannot replace their
observer, store, motion, or insertion permission. Native refusal never enters
`Scope.variables`, `all_procs`, String call matching, or alias-preamble passes.
Isolated mode still reports missing global/procedure renaming permission even
when the independent scalar-formal pass succeeds. General dynamic alpha
renaming and inserted stores require their own future sealed contracts.

Compatibility transformations require independently selected lexical declaration
advice, supported complete configuration, equal analyser/unit profile policies
and a structurally matching full availability context over the retained store.
`MinifyEnv` borrows that actual `ContextRegistry` and command Realm; roles and
case layout use the shared structured resolver. Intermediate changed text is
analysed with the same complete `ResolvedAnalysisInput`. Unsupported context or
configuration returns the complete original source, an empty symbol map and
zero rewrites with a typed refusal. Reporting-map names in the supported branch
remain authoring advice without Native identity or execution authority.

`LogicalProcedureDefinition::{capture,apply}` authenticates the complete
positively selected Logical input and current Document origin before retaining
a model procedure slot. Static whole words, unchanged unprefixed Proc schema,
strict formals and known Authored namespace are checked again at the transfer.
The stored body's script validity remains separate from definition validity.
Unknown, rebound, dynamic, opaque, observer or foreign premises decline. This
model slot supports Logical diagnostic presence; it supplies no Native
publication, entered activation, compiler prerequisite or Native Normal
certificate. See the
[Logical definition contract](../analysis/name-resolution-proofs/logical-procedure-definition-model.md).

Logical formal navigation and isolated alpha-renaming share
`OriginalLogicalProcedureBindings` from the complete positively selected source
input, actual context/Registry, strict formal-list schema and unchanged parent
body. Literal defaults retain their own text; genuine repeated or compound
scalar reads keep their exact root spans. Readonly navigation does not establish
whole-body edit coverage. `compact_logical_formals` additionally requires the
caller's isolated policy and complete binding-name closure; aliases, local
writes, observers, reflection, frames, unknown operations, arrays and nested
evaluation block edits. The selected zero/one-result Return descriptor supplies
only this source binding axis: zero operands retain empty authored roles and
one operand retains Result at ordinal0. Native effects remain unknown and no
Normal result follows. Variadic,
cooked, opaque and duplicate formals remain outside alpha edits. See the
[Logical formal contract](../analysis/name-resolution-proofs/logical-formal-binding-alpha.md).
Logical head metadata comes from the complete authentic script word under the
current whole image, configuration and token vector. The shared static ASCII
projection accepts whole braced, quoted or escaped heads; substitutions,
expansions, opaque units and non-ASCII values remain unavailable. A partial
token cannot supply builtin body, lambda, expression or case roles. See the
[complete lexical-head contract](../analysis/name-resolution-proofs/minifier-complete-lexical-head.md).
Result consumers preserve `MinifyResult.refusals` independently of any emitted
syntax and cannot infer a naming change from the requested tier label.

Implementation questions: [syntax correspondence](../analysis/name-resolution-proofs/minifier-original-syntax-correspondence.md),
[naming permission](../analysis/name-resolution-proofs/minifier-original-naming-permission.md),
[scalar body alpha compaction](../analysis/name-resolution-proofs/minifier-original-scalar-body-alpha.md),
[lexical authoring](../analysis/name-resolution-proofs/minifier-lexical-authoring-passes.md),
and [retained lexical context](../analysis/name-resolution-proofs/minifier-retained-lexical-context.md).
Named assertions are coverage bindings; actual Rust execution and native
measurements remain separate evidence.

### Current diagnostic subjects and source actions

`OriginalDiagnosticInvocation` retains the authentic `OriginalRegistryWords`
and actual immutable ContextRegistry for Registry-driven syntax diagnostics.
`with_schema` revalidates that selection; variable-name positions, W212/W216
advice and W146 literal validation consume its effective roles and selected
literal values. `RegistrySourceDiagnosticSubject` keeps the purpose, effective
operand ordinal, original written ordinal and whole-word extent together.
Captured prefixes and unanchored expansion values receive no written span;
dynamic values and pre-substituted vectors cannot manufacture a literal.
`Diagnostic::registry_source` and the Core typed subject transport preserve this
source purpose independently of message text. They grant no handler, current
runtime value, physical cell, Native compilation, Normal completion or edit
permission. `SourceDeprecationAdvice` supplies current applicability, message
and review replacement metadata from that same selected descriptor. Complete
original words preserve braces, substitutions and multiline operands in the
proposal; captured or expanded layouts receive no replacement extent.
`RequiresReview` and independent current edit guards remain mandatory. A source
proposal supplies no BIG-IP replacement-handler equivalence. See the
[Registry diagnostic contract](../analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md).

Registry source lifecycle diagnostics use
`ResolvedInvocation::authored_source_diagnostic_options` and an explicit typed
option boundary under the actual retained context. Required nested operands
remain data; unknown dynamic selectors, widths and reserved tails cannot advance
the scan to a later flag. Version and range buffers retain the genuine written
operand with its typed Lifecycle subject. W004 disabled-option diagnostics and
W304 terminator suggestions share that selection. Captured prefix metadata has
no written anchor, and a dynamic source word supplies no current runtime value.
The [nested option source observations](../analysis/name-resolution-proofs/namespace-ensemble-original-nested-option-positions.md)
retain C release and Jim capability differences without granting handler,
compiler or edit authority.

Compiler warnings can retain `DiagnosticSourceContext` with the complete
original image/channel, full lexer configuration and actual immutable Registry
snapshot. `TaintDiagnosticSubject` retains the original sink and read subject
independently of message text. `ContextDiagnosticData` transports a bounded
process-local handle to that actual warning; decoding client JSON cannot create
a new subject. This transport supplies neither a native name nor edit permission.

Use `find_taint_warnings_for_function` for a retained FunctionUnit and
`find_taint_warnings_for_module_function` when its original Module is available.
Their shared `TaintSourceContext` checks the actual input, availability,
profiles and complete source grammar; a supplied dialect label cannot restore
missing metadata. `retain_diagnostic_source_context` attaches provenance after
final whole-document spans are known and checks the emitting Module's command
generation. The standalone warning API retains its explicit caller contract
and cannot replace unavailable actual unit context.

`context_diagnostic_actions_in_analysis` independently checks the diagnostic
code and range, complete source/configuration, and both current Registry owners.
It selects the actual source sink and read geometry before considering an
action. Native lexical roots require current analysis membership; taint labels
are never re-encoded into native keys. Helper occupancy uses retained declarations,
including braced names; comments supply no occupancy. Collection bootstrap
requires actual selected handler/payload metadata and a parsed top-level handler
boundary. A nested handler lookalike cannot become an insertion anchor.

The selected `subst -nocommands` hardening uses independently declared option
and original word geometry. Other Native wrappers still require their distinct
evaluation and insertion permissions. Hosted source hardening and explicitly
selected Logical authoring remain separate advice purposes. Profile suggestions
also use current selected source command/event requirements; their catalogue
orders presentation after selection and establishes no appliance availability.
See the [diagnostic source action contract](../analysis/name-resolution-proofs/original-diagnostic-source-actions.md)
and [profile source advice contract](../analysis/name-resolution-proofs/original-profile-source-advice.md).

### Tool refactors and independently permitted literal bracing

MCP refactor requests analyse the actual requested dialect and pass the retained
`AnalysisResult` to `if_to_switch`, `switch_to_dict` and `brace_expr`. Data-group
extraction consumes that analysis's own Registry/configuration and requires its
independent lexical declaration advice. A Native or hosted request cannot receive
that permission through a substituted iRules profile.

`source_rewrite::select` identifies current selected handler structure only.
Branch conversion, substituted expression rewrites and temporary dictionary
insertion require their own evaluation, control-flow, cell, observer and store
permissions. Missing permission returns an explicit disabled action without edits.

`original_exact_case_source_at_analysis(source, analysis, head_offset)` joins
selected exact, case-sensitive case layout to the genuine effective source
operands. `OriginalExactCaseSource::subject` retains an arbitrary whole original
word, including inert braces, quoted compounds and variable syntax;
`pairs` contains completed literal pattern/body data. Preserve that subject's
original spelling when proposing an operand. Data values have no original
word identity and must use the shared literal renderer before entering proposed
Tcl source. Unknown option selectors, selected case-insensitive matching,
malformed case data, known replacements, unavailable operands and stale
complete input refuse the receipt.

```rust
fn original_case_summary(
    source: &str,
    analysis: &tcl_compiler::analyser::AnalysisResult,
    head_offset: u32,
) -> Option<(String, usize)> {
    let case = tcl_lsp_core::refactor::original_exact_case_source_at_analysis(
        source, analysis, head_offset,
    )?;
    Some((case.subject().try_text().ok()?.to_owned(), case.pairs().len()))
}
```

This example returns descriptive source syntax and a pair count. Dictionary
conversion still consumes the independent `RewriteObligation`; the case
receipt cannot enable Native edits. Data-group advice uses the separate
`original_exact_switch_source_at_analysis` scalar projection: exactly one
original variable component, the selected `whole_var_ref` grammar and shared
root/closed-element partition must agree. Its subject preserves the complete
reference spelling, including spaces, quotes, literal dollar/backslash and
Unicode. Arrays, compound substitutions and inert braced data remain outside
that scalar projection. Explicit standalone data-group authoring retains its
supplied Registry/configuration contract. Both proposed-script consumers check
complete generated lexical syntax without claiming BIG-IP runtime equivalence.
See the [exact case contract](../analysis/name-resolution-proofs/original-exact-case-source.md)
and [data-group subject contract](../analysis/name-resolution-proofs/original-datagroup-variable-subject.md).

For an equality operand extracted from a source expression, use
`scalar_variable_source_syntax(fragment, config)` with the caller's exact
`LexerConfig`. `ScalarVariableSourceSyntax` returns only the scalar root and
unchanged reference spelling. It shares the selected whole-reference kernel
with the genuine original-word subject, without retaining its input or operand
provenance. Core data-group, MCP data-group and `if_to_switch` consumers retain
their bounded equality/OR shapes and use this projection instead of character
classes or regex variable names. Preserve `reference()` when proposing a word;
rebuilding `$` plus `name()` loses braces, quotes and literal name bytes. The
fragment does not establish expression truth, a native receiver or permission
to change dispatch. Generated syntax checks and `RewriteObligation` remain
independent. See the
[selected IF scalar contract](../analysis/name-resolution-proofs/selected-if-scalar-source-syntax.md).

For one static original expression operand,
`SourceInvocationBinding::original_literal_expression_bracing` checks the exact
original vector, current unobserved stock handler, complete source/configuration,
Registry, independent compiler-name effect closure and checked native expression.
The selected native constant evaluator must independently establish a finite
normal result. Equal argument values alone supply no rewrite permission. The
whole original source must be one root/global command: procedure bodies and
later source observers, including `info body`, remain outside this bounded
permission. Variables, scripts, functions, expansion and ambiguous operands
refuse the issuer. A separately authored braced word must decode to the exact
same counted value under the same grammar. Core replaces only that original
operand span after revalidating source and Registry; the receipt supplies no
compiler admission, physical object identity, relocation or insertion.
See the [tool context contract](../analysis/name-resolution-proofs/original-mcp-refactor-context.md),
[source rewrite contract](../analysis/name-resolution-proofs/refactor-original-source-rewrite-permissions.md)
and [literal bracing contract](../analysis/name-resolution-proofs/original-literal-expression-bracing.md).

### Workspace procedure suggestions and instance member edits

`completion::original_workspace` derives the requesting document's protocol
from genuine current source inputs. Native candidates preserve independent
original declaration owners and render source spelling under that consumer's
compatible protocol. Hosted suggestions retain their actual source policy;
unavailable units remain readable cards without generated suggestions. Labels
are presentation and supply no reached command or runtime existence.

`original_member_rename_edits` has a separate bounded Instance-table purpose.
It requires one ordinary current source class, its genuine method entry and
complete source roster, including builtins and selected default visibility.
Every declaration/call uses its canonical original selector and static edit
container. Overrides, configurations, class-object or own-object tables,
generated links, future callbacks, incomplete coverage and ambiguous owners
refuse the complete plan. Server uses the common typed command rename tier and
revalidates all independent owners before publishing edits.
See [workspace completion](../analysis/name-resolution-proofs/original-workspace-procedure-completion.md)
and [instance member rename plans](../analysis/name-resolution-proofs/original-instance-member-rename-plans.md).

### Readonly resident name labels

`native_string::resident_name_label` presents complete resident bytes after the
consumer independently selects its semantic source owner. Printable Unicode
stays readable; backslashes, controls and invalid bytes use distinct escaped
labels. It supplies no source-roundtrip, equality, naming policy, native ingress,
lookup or edit permission. Keep the original byte/policy key beside the display;
never parse a label back into a key or source word. See the
[resident label contract](../analysis/name-resolution-proofs/resident-name-label.md).

### Original package operands and manifest spelling

`SignaturePackageRequire` retains the actual package name input/key and each
requirement value at its independently selected original argument ordinal.
Foreground `original_package_operand` can retain a genuinely known evaluated
value; the background scanner keeps its own static original operands and leaves
unknown values unavailable. A literal dollar or backslash is never inferred
from a reporting requirements string. Package names use their selected
package-purpose key; requirement values remain counted and receive no package
C-string projection.

`NativePackageNameKey::manifest_atom` renders under the retained native policy
and full configuration, then reparses the eventual Document-channel atom through
the shared original word owner. The resulting key and policy must agree with
the retained package key. CLI `original_document_requirements` validates the
whole original source/configuration and reads these producers directly.
Unavailable or unrepresentable operands stay unresolved. Optimised-text
compatibility requires independent lexical source advice and supplies no
original argument ordinal. This is manifest source advice; package loading,
availability and runtime table membership require their own owners. See the
[package manifest contract](../analysis/name-resolution-proofs/original-package-manifest-source-advice.md).

### Typed SpecTcl notice actions

`SpecPackNoticeSubject::from_loader` retains the actual original pack source,
producer parsing configuration, typed Property/Flag subject and unique literal
word geometry. `ContextDiagnosticData::from_spec_pack_notice` transports an
opaque bounded handle while independently retaining the resource consumer's
configuration and Registry semantic key. Client data can reselect an issued
subject but cannot construct one from a diagnostic message.

`spec_pack_quick_fixes` requires actual current Analysis and checks complete
source/channel, both configurations, Registry, notice kind and exact line/word
geometry. Changed presentation does not select another subject. Ambiguous,
forged, stale or missing subjects decline. This DSL source advice supplies no
Native input or runtime/evaluation permission. See the
[typed notice contract](../analysis/name-resolution-proofs/typed-spec-pack-notice-actions.md).

### Conditional Registry metadata and possible hosted taint

`original_conditional_registry_metadata` consumes the actual Native original
head and namespace source coordinates. The separate
`original_conditional_vendor_registry_metadata` consumes genuine hosted
`VendorSourceNameInput`, complete original words and the actual ContextRegistry.
The two share a command-cell barrier classifier while retaining their own name
policies. Known custom, missing, moved or document-override cells block nominal
catalogue metadata. Unknown alternatives remain conditional and retain explicit
applicability obligations; they establish no reached builtin handler.

`HostedSourceTaintContext` is retained by actual lowering and transported on
`CommandTokens`. It carries the whole image/channel, full configuration and
structural context with its Registry generation. Diagnostic compilation passes
its actual ContextRegistry through `CompilationUnit::build_with_context_registry`;
a profile label cannot replace a foreign generation. Unchanged original words
are revalidated through the guarded hosted issuer. Getter metadata retains only
possible taint and sink metadata can produce conditional source warnings.
Sanitiser certainty, storage effects, execution reachability, Normal, compiler
admission and edit equivalence remain independent. See the
[conditional metadata contract](../analysis/name-resolution-proofs/conditional-registry-source-metadata.md)
and [hosted taint contract](../analysis/name-resolution-proofs/hosted-source-taint-descriptor.md).

### Auditable local naming recipes

`cargo xtask owner-resolution` checks a finite inventory of typed Core naming
facades for local namespace/NUL delimiter recipes and lossy resident-byte
conversion. The token scan excludes comments, literals and actual test items
while retaining production code that follows tests. Each facade uses the shared
purpose owner for input selection, byte equality, namespace geometry and source
extent. Native Syntax issuers and explicitly selected lexical compatibility have
separate responsibilities. The gate detects its stated local recipes; it is not
a proof of arbitrary data-flow safety. The inventory and contract are in
[shared utility contracts](../contracts/shared-utility-contracts-rust.md).

### Trace subject names and value effects

`VariableReceiverOperandForm::TraceSubject` selects the original receiver at the
actual modern or supported legacy trace argument ordinal. It uses the native
trace-query extent before the independent combined root/index projection.
Source symbols retain the ignored original suffix and their own namespace,
local-frame or alias coordinates. A trace naming role does not read or overwrite
the subject's value; ordinary value reads keep their observer requirements.
SpecTcl and Studio preserve the typed receiver form and reject malformed,
duplicated or out-of-range coordinates instead of supplying a default. See the
[trace source-purpose contract](../analysis/name-resolution-proofs/trace-source-receiver-purpose.md).

### Reference and highlight selection

`references_in_program` and `document_highlights_in_program` share the current
namespace, variable, method and original declaration selectors. A selected
original query with missing or stale evidence is terminal. Independent original
declarations and canonical call edges survive erased reporting maps; public
String-facing method-reference helpers require explicit lexical advice.
Readonly reference locations supply no writable-reference consensus or edit
permission. See the
[reference selection contract](../analysis/name-resolution-proofs/original-reference-highlight-selection.md).

### Expression source positions

`expr_context::source_expression_operand_at` retains the actual original word
selected by complete current source/configuration and Registry roles.
`original_invocation::selected_registry_words` supplies the effective role and
written producer origin; hosted source uses the separate guarded
`selected_vendor_registry_words_at` and its actual structural context.
`source_expr_arg_context_at` uses that same operand for completion and excludes
inner command-substitution positions. Reporting heads and expression-shaped
data or comments cannot supply a role.

`OriginalMathFunctionOccurrence` retains each checked function identifier's
original byte extent and tree ordinal. A modern C command-table occurrence can
also retain a sealed relative function-name value and immutable original lookup
observations. `diagnostic_math_function_presence_at` gives actual reached
observations precedence, including Unknown; source presence is available only
when that actual inventory is absent and the independent literal-operand,
ordinary-pool, quiet-observer and callback-free lookup premises close. Fixed
function tables retain their separate selected rows. Neither projection creates
a written command head, evaluated argv, registration token or compiler entry.
See the [original function contract](../analysis/name-resolution-proofs/original-function-navigation.md).

C8.4 and Jim fixed-table expressions use the independent
`OriginalFixedMathFunctionLookup` source purpose. It retains a checked counted
ASCII identifier, selected fixed-table dialect and agreeing quiet original
snapshots under independently current parser-text provenance.
`SourceOrdinaryLiteralPool::text_effects_current` supplies that text purpose;
it grants no C object pool or commandName effects. The separate command-table
path retains its ordinary-object effects check. Neither projection creates an
implicit command name or slot reference. A supplied
actual entry table is terminal, including missing or Unknown. Only an absent
actual entry can use the independently selected fresh fixed roster. Actual
reached Unknown, stale input, operand side effects, opaque mutation and an
expression-handler replacement retain refusal. This presence classification
proves no function token, operand execution or Normal result. Jim native Pi
syntax errors remain separate from this readonly source classification. See
the [fixed-function source-presence contract](../analysis/name-resolution-proofs/original-fixed-function-source-presence.md).

`expr_rewrite_actions` requires the proposed selection inside that original
operand. Native and hosted operator proposals keep a disabled
`missing-expression-operator-rewrite-permission` reason with no edits until an
independent evaluation/replacement contract is supplied. Explicit lexical
authoring is a separate branch. Source position advice provides neither an
evaluated math-function target nor operator equivalence. See the
[expression selection contract](../analysis/name-resolution-proofs/original-expression-source-selection.md).


## Structural source graphs and diagrams

Compiler `source_graph::current_analysis` checks the complete document image and
retained lexer configuration. `invocation_targets_declaration_in` and
`reference_matches_declaration` compare genuine original call inputs, lookup
policy and allocation sites against independently current declaration records.
Core declaration, reference and graph consumers use these shared checks while
retaining each declaration's URI separately. Equal source in independent URIs
does not merge owners.

`procedure_body_for_declaration` additionally requires the actual module image,
configuration and Registry snapshot. It joins the current declaration allocation
to the procedure's executed body source and namespace. A matching displayed
procedure name cannot supply the join. `event_body_for_procedure` requires the
retained `SourceIrulesEventBody` descriptor and exact procedure/body context;
a `when`-shaped internal label supplies no event identity or TMM entry.

`tcl_diagram::diagram_data_for_analysis` checks those owners against the actual
supplied `ContextRegistry`. `diagram_data_for_dialect` retains that supplied
Registry through `snapshot().shared_registry()` before creating analysis; it
does not reconstruct a default command catalogue. Original declarations receive
independent source cards even when their labels collide. Missing allocation or
body correspondence leaves `flow` unavailable. Actual original calls retain
canonical declaration IDs. Action labels are presentation; completion hints
require independently selected handler metadata. `OriginalIrulesSourceContext`
retains the complete source/configuration, structural ContextRegistry and actual
event descriptors. Attachment and cross-rule reports consume its guarded source
schema and the shared original source-object kind/ordinal descriptors. Literal
source references retain their original spans; dynamic targets remain
unconstrained rather than inheriting nearby `set` values. These are conditional
source candidates, so neither an attachment pattern nor a possible event edge
certifies runtime reachability, observed behavior or deletion safety. See the
[structural diagram](../analysis/name-resolution-proofs/original-structural-diagrams.md),
[iRules context](../analysis/name-resolution-proofs/original-irules-source-context.md)
and [attachment candidate](../analysis/name-resolution-proofs/original-source-attachment-candidates.md) contracts.

## Conditional child-interpreter source visibility

`SourceInterpreterVisibilitySnapshot` retains the complete original child
creation/evaluation vectors, authentic body container, full immutable editing
input and separate C scoped slots and hidden-token source relationships. Typed
Registry transitions update this conditional ledger; an unowned nested child
masks its enclosing source context. Deferred-body reuse hashes the complete
snapshot and retains original source correspondence.

Use `Diagnostic::conditional_interpreter_visibility` and
`DiagnosticSubjectData` to transport W129 source knowledge and its explicit
applicability obligations. Presentation messages cannot recreate the subject.
Possible source effects and edges remain even when the source ledger reports
unavailability. Successful source operations, Registry initialization, closed
interpreter mutation/observers, entered body and original operand
materialization are separate obligations. Current hidden-table membership,
selected callability and runtime hide/expose require their actual independent
allocation owners. See the [source visibility contract](../analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md).

Child source reporting retains its own C report domain through namespace and
procedure descendants. A synthetic interpreter display scope cannot replace the
parent file's source root; W113 publication subjects retain original geometry.
Conditional Apply/lambda advice requires the genuine original descriptor or
retained list producer and an independent Apply schema. An omitted namespace is
Apply-rooted; an explicit original namespace list element is resolved by the
separate root-relative C owner. This geometry supplies no entered lambda frame,
receiver allocation or successful body execution.

Runtime adapters obtain the complete original namespace object string through
its actual getter before calling `NativeNameProtocol::lambda_namespace_input`.
That C8.5–9.1 operation retains counted units and supplies the exact root prefix;
`lambda_namespace_path` applies the separate namespace lookup extent afterwards.
Compiler `namespace_for_lambda_operand` consumes the retained
`OriginalVariableInvocation` value and its native list children. Exactly two
children prove omission; a third supplies the original namespace bytes. Static
containers retain the whole source image and full lexer configuration, and the
selected path must match a unique current namespace inventory. Opaque native
units never pass through a displayed namespace string. C8.4 cannot borrow the C
Apply recipe, and Jim retains its separate original object and source path.
These projections supply no object header, callable, entered frame or completion.

Lambda values additionally retain their actual constructor channel. The
[CLI source question](../analysis/name-resolution-proofs/lambda-original-namespace-object-prefix.md)
uses binary-produced values and literal UTF8 source; the [original SDK question](../analysis/name-resolution-proofs/lambda-original-namespace-constructor-and-getter.md)
compares counted String and C ByteArray third elements. Raw String00 and a
ByteArray's materialized C080 result cannot share an inferred lookup extent.
The SDK samples resident-string/type fields and then held getter bytes in its
exact recorded order; those fields grant no general cache/header or entered
frame correspondence. Jim preserves its independent complete counted String
result, and its ByteArray constructor remains untested.

## Original document links and completion contexts

`document_links_from_analysis` consumes the actual current Analysis, source,
configuration and Registry. Its shared invocation traversal enters retained
script/body roles and preserves genuine effective argument origins. A source
file link also requires the selected `SourceFileGrammar`; a package link retains
its original package-purpose key. Unknown or shadowed handlers, stale owners
and unavailable filesystem units decline. Explicit lexical path evaluation is
independent. Link construction proves neither file existence nor execution.

Registry context completion uses the actual innermost source vector before
querying metadata. Missing or blocked inner metadata cannot borrow an outer
command's roles. Alias captures and expanded children retain their effective
ordinals and independently owned source geometry. Hosted candidates use the
separate guarded vendor metadata issuer and full `ContextRegistry`; no C/Jim
name recipe is constructed from vendor source units. Schema advice supplies no
handler, effect, normal-completion or insertion permission.

Compiler `OriginalSourceScopedBody` retains the original body word, exact
content, complete image/configuration and selected conditional context.
Core `scoped_env_at` consumes that inventory after current-owner validation.
Known shadowed/deleted handlers and stale complete sources decline; source
regions or labels cannot manufacture a body receipt. Closed document grammar
completion retains every lexer configuration field, including BOM policy,
when selecting authored vocabulary. These are source advice contracts,
separate from entered frames and native execution. See the
[document link](../analysis/name-resolution-proofs/original-document-link-selection.md),
[context completion](../analysis/name-resolution-proofs/original-registry-context-completion.md),
[scoped-body](../analysis/name-resolution-proofs/original-scoped-body-completion.md)
and [document grammar](../analysis/name-resolution-proofs/original-document-grammar-completion.md)
contracts.

## Shared source structure and formatter inputs

`SourceImage` shares its immutable derived byte digest and line index across
clones. `SourceMap::from_image` reuses that exact byte/channel geometry; cache
storage does not enter full byte/channel equality, hashing or ordering.
`retained_document_image` reuses a realm image only after the complete Document
bytes and full configuration agree. Full input/context/Registry and vector
checks remain independent. Position reuse supplies no role, name key, selected
handler or edit authority, and this contract attaches no timing benchmark. See
the [immutable image-geometry contract](../analysis/name-resolution-proofs/original-immutable-image-geometry.md).

Compiler `registry_invocation::source_structure` owns `OriginalRegistryWords`
and `OriginalOperandSource`. Each sealed vector retains complete current
source/configuration/Registry and its actual selected, conditional Native or
guarded hosted applicability. Effective argv ordinals remain independent of
written producer origins. A captured prefix has no written anchor; an expanded
child can retain a readonly extent without becoming a whole original word.
`with_source_schema` checks the actual complete ContextRegistry before exposing
Registry-authored roles or argument presentation. Supported UTF-8 controls may
supply source schema without changing the independently retained original byte
values. Ordinary unknown hosted words occupy one slot; unknown expansion
cardinality remains unavailable.

`ScriptTimingResolver` receives `InvocationArguments` from the selected
original descriptor. `CommandPrefixArguments::from_invocation_arguments`
retains those actual facts without compatibility spellings. Use
`authored_source_script_arguments` and
`authored_source_command_prefix_arguments` for exact effective positions.
Generic executable positions include `LambdaLiteral` for its own list grammar.
Produced plain scripts instead use `authored_source_plain_script_arguments`
or the positively Logical counterpart: selected Body and CommandPrefix
positions qualify, while LambdaLiteral and ReferenceOnly remain excluded.
The Logical role kernel retains only unanimous authored frame layout, with
Native frame/effect facts independent. In both purposes,
unknown callback payloads keep their slots, while unknown selectors, option
widths and expanded cardinality refuse. `ReferenceOnly` operands supply no
executable source position. A selected ListArguments descriptor may retain a
finite ASCII list value through the shared serializer and its complete original
producer; it grants no native result object. Child-source consumers use the
same FrameLevel role grammar and original value capsule for static list-produced
scripts. Dynamic heads, shadows and inert data cannot supply a visibility
conclusion. See the [structured script-timing contract](../analysis/name-resolution-proofs/original-structured-script-timing.md).

Analyser `retain_original_static_source_names` captures readonly lexical
vectors before command dispatch gates. Genuine
`NativeWord::executable_parts` Command-body spans, including array-index
children, use the shared `native_script_words_in` owner. Independently selected
whole-word Body and case receipts retain script descendants through the same
original vector issuer. Each complete
child vector keeps the original whole image, full configuration and source
origin; an unknown parent can retain that syntax without supplying a reached
child. Braced data and escaped brackets remain opaque. A malformed tail has no
complete vector; independently complete prefix commands remain readonly syntax.
Use this inventory for source cards,
then require the separate current schema and consumer-purpose projection;
lexical ownership alone supplies no handler, Normal world, publication or edit.
See the
[original source-input contract](../analysis/name-resolution-proofs/vendor-original-source-context-input.md).

`OriginalSourceRegistryContext::capture` owns a fresh standalone readonly
source image, complete editing input and the same sealed Analyser Realm. Its
`words` query shares conditional catalogue, future-body and authored-transition
factories, including source-order alias and deletion barriers. These candidates
retain conditional applicability and cannot preserve runtime lookup, supply a
native entry, certify complete references or authorize rename. A consumer with
an existing analysis uses `source_registry_words` and its retained owner instead.
Hosted body ancestry requires that actual analysis; a standalone native grammar
cannot issue it.

Core `SourceStructure::capture` checks the complete analysis generation and
walks original command, bracket and body regions iteratively. Declaration
ledgers supply their actual body owners; shared source roles supply conditional
script/expr/case geometry. Registry case layout selects exact effective
ordinals and the independently chosen list grammar. Its separate presentation
projection can retain an odd complete list without granting executable case
layout. Original lambda owners retain the parent word and selected native list
geometry. Decoded values do not acquire cooked-source offsets, writable child
keys or runtime entry. Folding and selection consume this same structure;
plain data, blocked roles and stale owners cannot borrow a sibling body.

Public readonly syntax consumers use `SourceSyntaxStructure::capture(source,
analysis)`. `commands` exposes genuine whole-document command geometry;
`lexical_regions` exposes each `SourceSyntaxRegion::span` and its already
joined lexical `tokens`. Lexer `source_region_tokens_in` owns checked region
slicing and complete-image token rebasing. Consumers do not add local offsets,
recapture a child input, infer a body from newlines or use a reporting command
head to select roles. CLI ANSI/HTML highlighting and MCP data-group source
traversal consume this same geometry. ReferenceOnly syntax remains readable
without potential-evaluation authority; missing or stale input refuses capture.
These spans and tokens grant neither entered bodies nor Native names or edits.
See the [highlight geometry contract](../analysis/name-resolution-proofs/cli-original-source-highlight-geometry.md).

`format_tcl_with_input` accepts the caller's actual `ResolvedAnalysisInput`.
`FormatterConfig::for_resolved_input` keeps its complete lexer configuration,
profile and structural ContextRegistry while retaining independent style
options. Formatting maps actual effective roles back to their own written
origins and uses separate case/list presentation. It does not format captured
prefixes as if they were document operands. Keyword transformations require
their own permission; a literal expression rewrite additionally requires the
bounded original-expression equivalence receipt. See the
[source structure](../analysis/name-resolution-proofs/original-editor-body-structure.md)
and [formatting](../analysis/name-resolution-proofs/editor-original-source-formatting.md)
contracts.

Produced command-prefix source advice retains an independent selected
Body/CommandPrefix parent, one complete original command-substitution builder
and its original target words. The actual full-context builder traits and
ListArguments-from-zero schema, source graph and target alias lineage remain
separate premises. Captured words retain BindingPrefix origins and receive no
later call-site anchors. Compound/multiple-command substitutions, unknown or
rebound builders/targets, expansion, stale context and conflicting receipts
refuse advice. Lambda values remain a separate purpose, and unowned native
nested frames supply no source namespace. This receipt creates no evaluated
list object, future invocation, handler availability, frame, Normal or syntax
compaction. See the [produced-prefix source contract](../analysis/name-resolution-proofs/original-produced-command-prefix.md).

`OriginalDeclaredCommandWords` is a separate sealed source contract for
document/workspace declarations. It retains the full immutable DeclaredSurface
and provenance, authentic complete input and whole direct-written invocation.
Inline document declarations require the canonical `stubs-begin`/`stubs-end`
block; an orphan stub line supplies no contract.
`DeclaredCommand::supplied_argument_roles` supplies optional-slot roles, while
`source_arity` retains only finite declared counts. Typed declared diagnostics
preserve those roles, counts, whole written anchors, provenance and explicit
applicability through Compiler/Core transport; optional expression roles do not
concatenate operands. Genuine current document/workspace contracts enter every
document realm through `SourceDeclaredCommandContracts`. Deferred Logical
procedure and static namespace bodies retain their whole source ancestry and
conditional namespace/frame geometry, including qualified names and ordered
global fallback. Computed namespaces, unowned bodies, known replacement,
move/delete barriers, dynamic heads, expansion and inserted alias operands
refuse that advice. A builtin-spelled stub supplies no builtin effects, handler,
actual namespace/frame, Native entry, Normal or edit permission. See the
[declared word contract](../analysis/name-resolution-proofs/source-original-declared-command-word-contract.md).

`original_procedure_source_descriptor` retains the genuine same-document
implementation allocation, immutable declaration, actual full input and
independently selected formal grammar before argument-count success.
`SignatureSourceFormalParameters::argument_count_shape` uses the same retained
formal topology and selected C/Jim grammar before binding. An invalid count can
retain descriptive signature syntax. The richer
`OriginalProcedureArguments` projection then requires complete effective argv
and successful shared parsed-formal binding for argument topology. Neither
receipt supplies callee entry, a runtime frame or Normal completion. Inlay
labels and formal-role colours use only their exact written argument anchors:
captured prefix slots, omitted defaults and frozen list values without current
source children do not borrow spans. A present projection missing its original
correspondence remains terminal. See the
[procedure argument contract](../analysis/name-resolution-proofs/original-procedure-argument-topology.md).

Selected Apply source arity retains its genuine LambdaLiteral operand and
trailing argument count. `FormalArgumentCountShape` shares the independently
chosen C or Jim formal grammar with the strict binder and original source
signature; defaults and rest positions are not inferred from a reporting
profile. Captured literal alias prefixes retain their authenticated declaration
producer separately from the written call-site words and supply no call-site
anchor. Dynamic values, unknown expansion width, malformed formals and builtin
shadows decline. The [source diagnostic contract](../analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md)
keeps those descriptive premises separate from the [native formal-count observations](../analysis/name-resolution-proofs/procedure-original-formal-count-shape.md),
which cover only seven exact successful definitions, argc0..5 and the recorded
Apply/setup/call controls. Neither purpose supplies entered lambda frames or
native object/header identity.

Formal and lambda spacing uses native child list values and complete original
word ownership. The formatter decodes a proposed whole source spelling under
the same complete configuration and verifies exact native list elements,
nested formals and optional lambda namespace before emission. Opaque or
unrepresentable units preserve the original whole word; no formal installation
or runtime frame follows. See the
[formal formatting contract](../analysis/name-resolution-proofs/original-native-formal-list-formatting.md).

`CommandRegistry::authored_document_member_grammar` projects readonly nested
vocabulary from an actual selected document family, an admitted member and its
same-family child grammar. SpecTcl notice actions use this source projection
without asking whether the DSL word executes as a Native command. The loader's
typed subject, full current source/configuration and Registry identity remain
independent action premises; diagnostic prose cannot select a subject. See the
[typed notice contract](../analysis/name-resolution-proofs/typed-spec-pack-notice-actions.md).

Diagnostic edit consumers capture `DiagnosticEditSource::for_analysis` from
the current Document text and its actual retained analysis. The guard reuses
`WorkspaceDiagnosticSourceContext` for complete source, every lexer axis and
Registry semantic currency. `code_actions_in_program` requires it before range
actions and matches each published analyser fix against a current code/span/fix;
message and title changes remain presentation. `check_diagnostic_actions`
requires the guard and each compiler issuer’s matching `DiagnosticSourceContext`.
Compiler and DB aggregates attach that context after whole-document span rebasing.
Suppression edits obey the same currency checks. Server bulk selection captures
the guard for each freshly analysed buffer, preserving independent safety and
non-overlap rules. Valid source extents must also lie on actual UTF8 byte
boundaries. None of these checks supplies Native naming, handler, evaluation or
rewrite equivalence; structured subjects and each fix’s semantic obligations
remain independent. See the
[diagnostic edit currency contract](../analysis/name-resolution-proofs/original-diagnostic-edit-currency.md).

MCP iRule test generation and path reporting reuse `OriginalIrulesSourceContext`.
The original source remains exact inside a safely authored list element.
Generated outcomes are comments requiring independent observation, and taint
warnings join actions through their actual emitting spans. Event descriptors,
source action labels and conditional taint do not prove scenario feasibility,
TMM execution or expected behavior. See the
[source obligation contract](../analysis/name-resolution-proofs/original-irule-test-source-obligations.md).

Original script regions use the compiler-owned
`OriginalRegistryWords::source_script_bodies` projection. It selects genuine
`ArgRole::Body` and case-clause regions through the retained full context and
Registry schema. Direct bodies retain their complete original `NativeWord`;
case-list children retain the real parent word and independently selected child
extent. Expression and lambda grammars remain separate. Cooked quoted words,
malformed clauses, inert data and multiple concatenated script operands cannot
lend original script positions. Core body discovery and the hosted read-only
source-vector inventory consume this same projection.

A stored hosted child retains sealed source ancestry independently of whether
its parent body is entered. `StoredScriptBodyApplicability` records that premise.
A present child lookup must pass its own source-cell classifier; a blocked or
stale lookup cannot fall through. When the child has no lookup snapshot or
runtime/compiler source attachments, its authentic original occurrence and body
receipt can supply read-only source schema with `UnknownCurrentLookup` and
`UnknownSourceNamespace`. The missing attachments remain absent. The shared
source-barrier classifier checks the actual retained ancestor script regions.
No parent frame, command table, live variable cell, ILX handle, Normal result or
syntax-compaction authority follows. See the
[original body-structure contract](../analysis/name-resolution-proofs/original-editor-body-structure.md)
and the
[hosted source-input contract](../analysis/name-resolution-proofs/vendor-original-source-context-input.md).

Readonly ILX method cards consume the shared complete `source_vectors`
inventory under the actual guarded hosted schema. Their private full image,
configuration, structural context and Registry identity prevent stale source
reuse. Core reselects the exact current card before navigation. `definition` returns
`IlxTarget::SourceCandidate` for a genuine inline constructor and JavaScript
registration path, with the independently unresolved handler and handle-result
obligations in `IlxSourceLocation`. This does not fill `IlxMethodCall::target`.
`source_candidates` and `source_candidates_from_registration` explicitly list
readonly source/path advice under configured mappings or the directory
convention. Identity `references` retains only the own site for an unavailable
handle; equal method or constructor labels cannot issue a shared handle.
Primary Server Tcl requests obtain the actual document analysis through
`analysis_for` and pass it to `IlxContext::with_analysis`.
`OriginalIrulesSourceContext::from_source_analysis` validates its full image,
configuration, structural context and Registry, retaining readonly source
vectors without rebuilding a lowered module or granting event entry. A present
stale or unsupported analysis declines the query. The explicit standalone
source API captures its own complete input; it cannot replace a retained
document configuration. `Analyser::file_lexer_config` preserves the supplied
whole-file configuration, including `LeadingBom`, before original vectors are
retained. Only default ingress chooses the profile source-file BOM rule. This
document boundary does not select the grammar of an independently entered
nested script. Stored proc/control/case source descendants use the shared original Body/case
projection. Missing, cooked, inert or blocked body geometry remains unavailable. See the
[ILX source contract](../analysis/name-resolution-proofs/original-ilx-method-source-candidates.md).

Registry source hover uses the current complete source/configuration and the
retained profile and Registry. Shared source schemas supply actual command,
subcommand and option-prefix roles at their effective original operands.
Captured prefixes have no written cursor span; expansion children keep their
own original extents. Scoped documentation requires its genuine body receipt,
not a reporting region. Known shadows and deleted cells block nominal docs;
unknown applicability remains conditional documentation. Head subcommand lists
use the actual retained `ResolvedContext` availability owner and the selected
schema's owning-package lifecycle floor. Selected subcommands, nested operations
and option-prefix occurrences use those same availability owners; caller
profile labels cannot widen the lists. These are documentation facts, without
an interpreter command-table or successful execution claim. See the
[Registry source hover contract](../analysis/name-resolution-proofs/original-registry-source-hover.md).

`OriginalSourceScopedCommandSchema` joins a genuine scoped-body issuer to
independently selected `ScopedCommand` vocabulary. Its inert source descriptor
retains parent availability and uses the common Registry selector/form query.
Complete original child words, whole body/image/configuration and actual
ContextRegistry/Registry correspondence remain mandatory; copied reporting
regions and global head lookup establish no scoped issuer. This schema grants
no package installation, safe-interpreter alias, Native handler or frame entry.
See the [scoped source contract](../analysis/name-resolution-proofs/original-scoped-command-schema.md).

Expression diagnostics use
`ResolvedInvocation::authored_source_expression_arguments` to retain the
selected effective expression roles and whole-tail concatenation grammar.
`OriginalDiagnosticInvocation` maps those roles to genuine written whole words
and transports a typed Expression subject. Unknown ordinary values keep their
source slots; unknown argv cardinality, captured prefixes and expanded words
cannot donate anchors. A single original quoted word can receive a brace
proposal requiring review. A mixed multiword tail retains its whole source
extent without that edit. Source literal values supply no runtime expression or
rewrite equivalence. See the [Registry diagnostic contract](../analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md).

Disabled-command diagnostics use `OriginalSourceCommandAvailability`, a
separate negative metadata receipt. `command_source_unavailability` selects
excluded rows under the actual immutable full context, related provider rows
and invocation realm. The receipt retains complete original Native, positively
selected Logical or hosted source ownership and independent lookup/body
barriers. Known declarations and custom cells prevent nominal exclusion advice;
reporting dialects and maps cannot recreate its applicability. W002/IRULE2004
and Core transport preserve the typed command, classification and source domain
without parsing messages. Negative metadata supplies no admitted schema,
selected handler, runtime command absence or successful operation. See the
[source-unavailability contract](../analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md).

Instance and widget diagnostics obtain
`source_registered_instance_words_at` from the current analysis.
`OriginalSourceRegisteredInstance` retains the genuine mandatory named factory
and unchanged original naming operand. A scalar handle separately retains its
complete selected setter and actual written singleton RHS construction; a
captured alias RHS is not a new setter-time construction. Complete method argv,
alias/move lineage, full image/configuration/input and actual ContextRegistry
must agree. Shared descriptor method, prefix, lifecycle, arity and option
queries use that same Registry-owned factory, without inverse class-label
lookup. Unknown source widening retains the authentic original factory/name
conditionally with explicit mutation/lookup obligations and clears private
symbolic setter handles. Known replacement/deletion, stale handles and
unsupported selectors refuse. Typed instance source subjects preserve factory
identity and applicability through Core transport; they prove no construction,
runtime object/value, entered receiver, Normal or edit authority. See the
[registered-instance source contract](../analysis/name-resolution-proofs/source-original-registered-instance-words.md).

Append, Unset and case-body advice uses the shared selected source shape
queries. Unknown payloads retain effective slots, while actual C/Jim Unset
option grammar and known case flags own position selection. Dash-leading
flag-free case patterns remain data. W104/W106/W217 use genuine whole words
and executable lexical parts. A captured variable can preserve a warning on
a written padded payload, but an lappend proposal requires exactly two direct
unexpanded written operands and review. Typed AppendList, CaseBody and
OptionOnly purpose survives Core transport.

W306 obtains selected Regex pattern positions from the same source schema.
An unknown initial option-or-pattern word does not establish a pattern slot;
`--` supplies an explicit boundary. Braced, escaped and variable patterns keep
their original lexical distinctions. A bracket quoter exemption requires its
genuine unchanged child geometry under full configuration and a separately
selected source descriptor. REGEX_LITERAL metadata supplies no evaluated
quoter result or sanitising effect. Captures cannot donate call-site spans;
source proposals prove no successful matching or handler equivalence. See the
[Registry diagnostic contract](../analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md).

Control diagnostics use the already selected source descriptor through
`authored_source_clause_issue`, `authored_source_context_gate` and
`authored_source_optional_trailing_names`. `ClauseShapeChecker` and
`ContextGate` receive `InvocationArguments`: unknown payloads retain positions,
while unknown selectors and unproved expanded counts cannot select a grammar.
`ClauseShapeIssue` carries the defect and an optional `ClauseShapeRepair`
authored by that same grammar walk. `authored_source_clause_shape` projects
only the defect for consumers that need no proposal. Consumers read the typed repair anchor;
they never search condition values for a clause keyword. Diagnostic-only pack
hooks carry no repair anchor. E004 requires its genuine written operand;
captured conditions supply no call-site anchor, and clause proposals require
consecutive ordinary words. Edits use complete `NativeWord` spans, including
compound-word suffixes and closing delimiters.
W302 retains a selected Catch body and appends optional result names at its
whole written endpoint. Only an independently selected original child teardown
descriptor suppresses that hint. W142 uses lexical event context and count; an
f5-irules authoring input proves no TMM execution. Typed ClauseShape,
ErrorCapture and ContextGate purpose survives Core transport. See the
[control source-advice contract](../analysis/name-resolution-proofs/diagnostic-original-control-advice.md).

Channel diagnostics select `channel_arguments` through the current original
Registry schema or genuine block-framed declared contract. A channel role and
a written whole operand are separate requirements: captured prefixes and
unknown optional-selector layouts receive no invented anchor. Literal dollar
words remain data. `scalar_variable_reference` retains only a complete original
scalar part and its actual span; the diagnostic matches the token access at
that span and uses the original SSA read identity/version, rather than selecting
a cell by a displayed name. Typed Registry or Declared ChannelArgument purpose
survives Core transport. This source type advice supplies no current channel
allocation, existence or successful I/O. See the
[channel source-ownership contract](../analysis/name-resolution-proofs/diagnostic-original-channel-source-ownership.md).

Package advice selects `OriginalSourcePackageReference` from the genuine
complete source schema and actual immutable context. A `package provide NAME`
query supplies no provision; version-supplying provision, selected options and
exact count grammar remain required for source ordering. W120/H301 retain typed
package requirement/purpose and independent Native package keys instead of
reparsing invocation or requirement labels. Core preserves that purpose when
messages or fixes change. These metadata facts prove no package installation,
loader/callback success or eventual command presence. See the
[package source-advice contract](../analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md).

Package actions and W120 use the same
`original_package_require_insert_offset` owner for genuine root command
geometry under the complete current input/configuration. Split-line commands,
shebangs and EOF retain their original insertion extents; nested requirements,
data and provide-name queries cannot become root headers. The actual
ContextRegistry owns the package catalogue. Native actions retain their
genuine unresolved-command subject and lookup. Logical actions require a
positively sealed complete source vector and shared positioned absence; an
unsupported Native or hosted input cannot fall back to that advice. These
reviewed source proposals establish no installed package or successful loader.
See the [package action-context contract](../analysis/name-resolution-proofs/core-original-package-source-action-context.md).

Command suggestion candidates are filtered through the actual full context.
`original_command_suggestion_fix` authenticates the emitting original subject
and renders one faithful ASCII source word at the complete written head extent
under the selected grammar. Unsupported value domains withdraw the proposal;
review remains required. The heuristic candidate proves no target cell,
handler, current presence or edit equivalence. See the
[command suggestion source contract](../analysis/name-resolution-proofs/diagnostic-original-command-suggestion-source.md).

Source signature diagnostics use `authored_source_arity` and
`authored_source_count_for_arity` for the selected count axis and each actual
version window. Unknown expansion widths retain only their proved lower bound;
mandatory pre-option operands stay data. Extra-argument proposals require a
consecutive suffix of genuine written whole words and `RequiresReview`.
`authored_source_option_relationships` uses the same selected topology and
complete-negative boundaries. Typed source package metadata identifies
applicability without proving package installation. These descriptive source
facts establish no accepted runtime argv or handler result. See the
[Registry diagnostic contract](../analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md).

Logical procedure models select
`Registry::authored_source_semantics_in_context` from the actual complete
ContextRegistry. That authored descriptor roster remains distinct from source
InvocationDialect and Native release availability. A quiet stock Proc schema,
static original words/formals, known Authored namespace and current positive
Logical input can retain a model definition; this does not issue a Native
publication or entered frame. See the [Logical definition contract](../analysis/name-resolution-proofs/logical-procedure-definition-model.md).

Selected static instance forward declarations retain a nonempty original prefix
separately from target dispatch. Source-order replacement and the bounded
superclass inventory can mask script or builtin advice without entering the
forward target. A current occupied forward override blocks stock variable-linker
advice; absence of a script body alone cannot establish stock eligibility.
`closed_inherited_member` joins the authentic current unaliased superclass
allocation, implementation generation, lifecycle and factory-grammar receipt.
Computed prefixes and unknown method chains decline. The
[source contract](../analysis/name-resolution-proofs/tcloo-original-forward-registration-prefix.md)
and [native source observations](../analysis/name-resolution-proofs/tcloo-native-forward-registration-prefix-boundaries.md)
keep definition-scope substitution, registration and future invocation separate;
the measured PREFIX_CAPTURE difference supplies no physical prefix-storage or
successful source-transfer proof.

Pattern Hover and semantic tokens use
`ResolvedInvocation::authored_source_pattern_arguments` under the actual retained
ContextRegistry. Selected descriptors, available options and effective ordinals
identify embedded languages; only each genuine current written operand receives
its original cursor or token anchor. Positively sealed Logical input follows the
same source inventory. Captured alias values, uncertain selectors, expanded
cardinality, known shadows and stale source/context decline the applicable
pattern fact. This source projection supplies no match result, native value,
handler or edit authority. See the [pattern context contract](../analysis/name-resolution-proofs/original-pattern-retained-context.md).

Source whitespace layout reads complete original word plans under every lexer
configuration field. Only trivia gaps between genuine command words can be
trimmed or collapsed; quoted/braced/indexed data keeps its original spelling.
Selected formatting ranges require complete words in their owned script region.
Body command thresholds use the same whole-configuration lexical plan and keep
unavailable syntax separate from a successful zero-command count. See the
[source whitespace contract](../analysis/name-resolution-proofs/original-source-whitespace-geometry.md).

Conditional hosted taint retains the invocation's actual `SourceOriginId`
whole image after original-token stamping. A positioned lowering body buffer
may contain padding; it cannot replace that source owner. Shared guarded
metadata revalidates the complete original vector, configuration, structural
ContextRegistry and Registry generation before source/sink advice. A sole
variable operand can remain MAY unknown when no independent positioned SSA
read exists; source metadata creates no cell, contents version, Native handler
or Normal receipt. See the
[hosted taint contract](../analysis/name-resolution-proofs/hosted-source-taint-descriptor.md).

The hosted document Realm retains `vendor_source_input` as the actual complete
`ResolvedAnalysisInput` through source entry, baseline, cache and lowering.
Its independent source InvocationDialect and full ContextRegistry select the
source catalogue. A C-compatible Registry profile does not supply C execution
or variable storage policy for that hosted input. An explicitly supplied
execution policy remains a separate owner. Concatenating descriptors retain a
contiguous body receipt only for one genuine whole original word at the final
effective argument; a Body annotation on the first fragment of several eval
operands supplies no such receipt.

Inlay, Hover embedded-format, semantic format and Compiler version-gate
consumers use `ResolvedInvocation::authored_source_format_arguments` through the
same sealed source schema. Actual ContextRegistry availability, effective
argument_offset and original origins select metadata. Compiler
`Analyser::original_format_templates` collects each selected Binary or
Sprintf/Scan template once with its formed literal bytes, effective ordinal and
genuine written whole-word span. Captured prefixes, expansion children without
a whole word, dynamic values and pre-substituted vectors supply no template
anchor; reporting command labels cannot select this grammar.

Builtin parameter hints, Registry source cards and semantic format/head
projections share this whole-source descriptor for Native and positively sealed
Logical inputs. `original_static_word_ascii_source_offset` maps unchanged
literal interiors and decoded escape endpoints back to their original word;
decoded interiors, transformed braced views and ambiguous coordinates refuse.
Static source bytes follow the complete selected escape grammar. In particular,
C8.4/C8.5 greedy x25b is a bracket rather than percent-b; the explicit x25
followed by u0062 boundary is independent. The
[source escape observations](../analysis/name-resolution-proofs/original-source-static-escape-values.md)
keep CLI binary-scan projection separate from counted SDK result getters;
their different zero-byte outputs supply no original object identity.

Ordinary semantic colours share `OriginalRegistryWords` for each positively
retained source domain. The selected descriptors supply option boundaries,
nested selectors, enum operands, inline cases and definition-body grammar;
format and pattern roles precede generic option colours. Reserved trailing
values stay data, captured alias prefixes receive no source anchor, and a
shadowed head supplies no nominal builtin role. Logical definition bodies need
their positive complete-input/head seal; user and OO overlays retain their
independent source owners. Declared command roles use their separate immutable
DeclaredSurface contract and never acquire builtin traits. See the
[source-role contract](../analysis/name-resolution-proofs/original-inlay-retained-context.md)
and [declared word contract](../analysis/name-resolution-proofs/source-original-declared-command-word-contract.md).

An ordinary head with missing or refused authentic source schema receives
only generic Function colour, without builtin library or keyword modifiers.
`definition_member_head` independently requires a complete genuine static word
and its selected definition-body grammar under the current full source, input,
context and configuration; it supplies no ordinary builtin fallback.

Explicit Logical list roles use `OriginalSourceListElement` and
`original_static_word_list_elements` from the genuine complete original parent
word and selected full list/escape grammar. Unicode lexical fields retain only
proved original extents. Cooked intermediate values cannot fabricate interior
coordinates. Native list/formal receipts remain independent and take precedence
where available; lambda bodies retain their existing original owner. This list
geometry creates no Native object, frame or Normal completion. See the
[body-structure contract](../analysis/name-resolution-proofs/original-editor-body-structure.md).

Signature rendering likewise consumes `OriginalRegistryWords` for
positively retained Logical input and Native source advice. Missing or foreign
source/configuration/context cannot fall through a nominal catalogue query.
See the [inlay contract](../analysis/name-resolution-proofs/original-inlay-retained-context.md)
and [signature contract](../analysis/name-resolution-proofs/original-signature-schema-rendering.md).

Body-free declaration/database headers retain `SourceFormalCountProjection`:
count, original/authored/unknown origin and selected `ParameterGrammar` only.
`SourceDeclarationSignature`, `ItemSig` and method headers keep the projection
separate from body bytes and mutable reporting parameter labels. Original-key
and reporting arity maps consume this same count; body-only edits preserve the
header and project-arity equality. Complete ProcDef, MethodDef and SignatureProc
records retain `SourceFormalCount`; body-free consumers take
`.formal_count_projection()` and use its `.arity()`. Origin and parameter grammar
remain descriptive metadata, and method `params_computed` remains independent.
Unknown remains open with no selected grammar. This Eq projection cannot
recreate original words, a whole-source
receipt, successful binding or an entered callee. See the
[formal-count header contract](../analysis/name-resolution-proofs/database-original-formal-count-header.md).

`OriginalSourceReceiverBodyDeclaration` retains one genuine original TclOO
method with complete parameter/body words and unchanged borrowed ASCII body
under the full current image, configuration and input. Scope keeps this source
owner independently of native deferred-body entry. `owns_invocation` requires
the exact original body extent and complete parser vector; a contained partial
vector does not qualify. Computed, recooked, conflicting or foreign owners
decline. This supplies readonly invocation/body applicability without method
allocation, executing receiver namespace, entered frame or Normal completion.
See the [receiver-body declaration contract](../analysis/name-resolution-proofs/tcloo-original-source-receiver-body-declaration.md).

Caller-variable navigation consumes `caller_frame_bindings` and
`original_caller_read_spans` under the actual complete input and namespace.
`NavigationVariable` keeps declaration-backed original symbols separate from
sealed caller-frame source candidates. The latter can supply readonly cards
and same-frame reference extents without granting entered upvar links,
physical cells or original-only rename coverage. Native/hosted static procedure
assistance separately requires a genuine complete original call, selected
Procedure implementation, empty captured prefix and matching original
allocation; `proc_for_definition` supplies metadata for that same allocation.
Unpositioned String names, final procedure reports and caller labels cannot
replace it. See the [caller navigation contract](../analysis/name-resolution-proofs/original-caller-frame-navigation.md)
and [static procedure contract](../analysis/name-resolution-proofs/original-static-procedure-selection.md).

Readonly C array-name assistance uses a separate
`SignatureSourceNameValue::from_original_variable_operand_root` purpose. The
shared original `CompoundArray` boundary retains a fixed root such as
`::N::array` while leaving `$index` unknown. Complete original vector, whole
image, full configuration and independently matching C policy must agree. This
origin supplies no produced value, complete word key, static list container,
index evaluation, entered alias or element cell. Dynamic roots and escaped
opening delimiters decline. See the [array root contract](../analysis/name-resolution-proofs/original-array-operand-root.md).

Conditional variable-alias navigation queries
`AnalysisResult::original_variable_alias_templates_in_source` under the complete
current image, full retained input and configuration. `OriginalVariableAliasTemplate`
keeps the genuine declaration frame, original local producer, target producer,
source order and unresolved applicability obligations.
`OriginalSourceVariableAliasOperands` separately retains the authentic complete
Registry source schema and actual ContextRegistry. Typed VariableCellAlias
subject/local ordinals map only through Written effective origins and whole
NativeWords; captured prefixes and expanded element values supply no written
anchor. Per-item reuse and clearing preserve that separate source facet. A fixed
compound target root supplies no evaluated index or element cell. `NavigationVariable::Alias`
shares that readonly relationship across definition, hover and references.
Physical alias matching, editable references and rename coverage require their
independent owners. Dynamic newer targets or conflicting declarations withdraw
later source anchors. See the [alias source-template contract](../analysis/name-resolution-proofs/original-alias-source-template.md).

Namespace command-path transfer uses `original_namespace_path::select` on the
selected SetPath operand. Under an independently selected C8.5–9.1 policy,
all elements of the complete original List must resolve to unique existing
namespace incarnations before the source path is replaced. Element order and
duplicates remain exact; an empty complete List clears the path. Missing,
ambiguous, stale, malformed or unsupported operands leave unknown path state
and allocate no namespace or runtime token. C8.4 and Jim provide no transfer
through this owner. The separate native ASCII path result establishes its
recorded runtime question only. See the [path transfer contract](../analysis/name-resolution-proofs/namespace-original-counted-path-transfer.md).

## Authored alias and rename source advice

`SourceCommandBindings::original_source_transition_advice_tape` owns conditional
root-script alias and rename advice. Genuine complete `NativeWord` vectors select
Registry-authored transition subjects. `SourceAdviceNameInput::Native` retains
the independently selected command lookup, alias publication and rename
source/destination inputs. Consumers must not normalize labels into command
slots. `SourceAdviceNameInput::Logical` instead retains the whole original word
and shared ASCII source presentation under a positively validated immutable
`logical_source_input`. It issues no Native naming key, value object or policy;
both purposes use the same source-order graph and transition recipes. The tape preserves source
order, captured prefix producers, known shadows, deletions and rebinding.
Aliased or moved transition operations also retain their genuine earlier
source-schema selection receipts independently of the current complete
operation vector and subject producers. Unknown
source operations withdraw derived aliases and moves, retaining their exact
original vectors as unresolved applicability premises.

The immutable `CommandBindingRealm` shares this preparation under the complete
`ContextRegistry`. Compiler `source_registry_words` issues the same sealed
`OriginalRegistryWords` to source consumers. `OriginalRegistrySource::SourceTransitions`
retains any written transition lineage and unknown applicability obligations.

Native baseline source descriptors come only from visible Registry rows in an
authentic closed root entry with the same complete name policy. Missing rows,
opaque or conflicting implementations, known shadows and deletions remain
barriers. The retained source receipt keeps `NativeBaselineSourceApplicability`
and unavailable/unknown lookup obligations; it never changes actual bindings.
`OriginalRegistryWords::written_argument_roles` supplies only genuine Written
whole-word positions, without assigning coordinates to captured prefixes or
expanded children. See the [conditional baseline contract](../analysis/name-resolution-proofs/native-baseline-conditional-source-roles.md).

The shared `OriginalSourceTransitionAdviceLookup` distinguishes `Advice`,
`Refused` and `Unrepresented`. A represented refusal is terminal. Only an
unrepresented site may ask an independently authenticated conditional
declaration owner; an absent result cannot reopen a stock schema fallback.
Explicit Logical advice also retains `logical_source_input()` and
`ExplicitLogicalSourceApplicability`; check that input against the consumer's
actual full editing input and require `original_head().logical_input()` before
using Logical presentation permissions. A missing Native operand input does
not select the Logical domain. The actual ContextRegistry allocation must
match the validated Logical owner before advice preparation or projection.
Effective captured prefixes have no current written operand anchor; current
arguments retain their own complete original words and source spans. Formatting,
hover and source roles consume `with_source_schema` under that same context,
and body presentation uses the shared `source_script_bodies` projection.

Logical source metadata additionally requires
`ResolvedAnalysisInput::has_logical_source_name_context` and the complete
positively retained input; an unversioned authored simulation is not a Native
execution recipe. A genuine root Procedure descriptor and unchanged original
body may retain an own conditional child inventory. Each child keeps the same
whole source, configuration and actual context. Mutations affect only that
inventory, without publishing an entered body or parent command-table facts.
Cooked body coordinates and unsupported holder/frame recipes remain unavailable.
Readonly roles do not authorize keyword editing when earlier mutation or callback
uncertainty remains. See the [complete Logical metadata contract](../analysis/name-resolution-proofs/minifier-complete-logical-metadata.md).

Whole-source correspondence uses `AnalysisResult::matches_original_source_image`
and the shared `source_registry_words` ingress. The document Realm producer
privately seals the independently supplied full `ResolvedAnalysisInput`,
including profiles, grammar, hosted policy and immutable context generation.
Copying public `resolved_input` fields cannot reproject those retained facts.
Repeated walks over the same complete hosted vector preserve its already
sealed body ancestry; an independent diagnostic lexical vector supplies no
child body role or source schema on its own.

A snapshot with unbounded lookup and no selected target may retain conditional
source advice with explicit unavailable-current-lookup obligations. Native
known targets or definite absence remain authoritative and cannot fall through.
Explicit Logical advice requires any known target to agree with the canonical
Registry descriptor and captured prefix; known shadows, deletions and foreign
input generations decline. Even a matching Logical target grants source
presentation only.
`original_source_registry_barrier` keeps a known Shadowed/Deleted source cell
or closed alias cycle distinct from a missing or unrepresented catalogue.
Conditional catalogue and future-body advice cannot override that barrier; an
unloaded catalogue alone establishes no shadow.

Implicit current-interpreter constants belong to the selected Registry
transition recipe; they never receive manufactured source inputs.
This source issuer handles original root-script transitions only; computed
transition subjects, nested namespace-body operations, cross-interpreter aliases
and supplied live native entries stay unavailable. Its advice always has
`operands_preserve_source_lookup=false`. Source advice supplies no successful
handler, physical command table, native compilation, Normal result, current
variable cell, rename edit or syntax-compaction authority. Changes to transition
handling belong in the shared Registry recipe and byte-purpose owners, with
source-order, shadow, deletion, prefix, stale-source, full-configuration and
foreign-context controls. See the
[authored transition advice contract](../analysis/name-resolution-proofs/authored-command-transition-advice.md).

Immediate original Body/case regions and command substitutions are inspected
through the shared authored source graph before later source advice is retained.
Whole-image words, complete configuration and the actual ContextRegistry own
that traversal. Possible rename, deletion and rebinding withdraw affected source
cells; speculative child publications never enter the enclosing graph. Unknown
callbacks retain their exact original child vectors and `UnknownEarlierMutation`.
Dynamic operands may keep current pre-operand grammar while withdrawing later
advice. A literal command spelling does not close channel, execution or variable
observers. A synchronous descriptor with `Reentrancy::Never` still retains
unknown future variable, command or execution observer state when the selected
body metadata requires it. This readonly inspection supplies no entered frame
or runtime mutation receipt. See the [immediate-body source advice contract](../analysis/name-resolution-proofs/original-immediate-body-mutation-advice.md).

`OriginalLexicalMemberContext` separately retains the canonical instance-side
method, constructor or destructor declaration, complete original ParamList/Body
words and the actual full analysis input. `source_class` joins that same source
publication and declaration; class/member display labels are not split into an
identity. `OriginalLexicalNextCall` consumes common Registry source argv, the
source relation MRO, counted member ledger and original formal topology for
readonly signature advice. A genuine selected factory and closed declaration
workers retain those original source method bodies even when the independent
inherited runtime-chain check refuses overlapping mixins. Absolute target
geometry stays separate from a relative target that lacks an actual object
namespace. This context grants no
entered receiver, runtime next provider, Normal completion or editable coverage.
See the [lexical-member source contract](../analysis/name-resolution-proofs/tcloo-original-lexical-member-context.md).

`AnalysisResult::original_declared_self_method_template` supplies a separate
readonly declared-self relationship for an unentered constructor. It joins the
authentic lexical class/member/body, current full input, common original
Registry vector, source MRO and byte formal topology. Caller-variable navigation
uses that original callee body and conditional source template. A class label
cannot replace these premises, and the template grants no entered receiver,
successful installation, physical caller cell, Normal completion or edit.
See the [declared-self template contract](../analysis/name-resolution-proofs/tcloo-original-declared-self-method-caller-template.md).

Receiver caller-name traits require their separate
`OriginalCompilationLookupAdvice::closed_receiver_trait_lookup` and
`receiver_source_trait_invocation` source purpose. The actual declaration body,
preview frame and source namespace must agree with unanimous stock Registry
candidates, quiet observers, accepted arity and the complete original vector.
The resulting parameter and caller-name recipe supplies no future private
runtime namespace or entry. Unknown or absent candidates, custom handlers,
foreign body bytes and incomplete vectors decline. Runtime closed lookup,
definite invocation, entered receiver and physical caller-cell obligations
remain independent. A declared receiver formal may pass an unrelated alias
prefix only when its retained original closed prefix proves that every alias
targets the immediate caller; same-frame, unknown and retargeted aliases refuse.
The independent prefix remains required when enumerable alias maps are empty: an unresolved dynamic alias can withdraw it without installing a visible row. The unchanged formal-slot, incoming-value and observer requirements still apply.
See the [receiver caller-trait contract](../analysis/name-resolution-proofs/tcloo-original-declared-receiver-caller-traits.md).

`SourceLoad` retains the selected Registry path subject ordinal through
`source_file::path_candidate`. `OriginalInterpreterSourceLoad` joins that whole
original path word, complete child argv and immutable input without executing a
file. Workspace source indexing and Server inheritance validate the retained
analysis before publishing possible source edges. Procedure Define geometry
likewise requires a unique visible declaration with a matching whole original
name word; literal-colon names remain distinct from global-root names. These
source relationships establish no actual child command or procedure allocation.
The independent [parent-command observations](../analysis/name-resolution-proofs/interpreter-original-created-parent-command.md)
retain C success and the tested Jim form refusals as separate provider facts.

`OriginalSourceInterpreterHandleBody` retains the complete original creation
and call vectors, exact source order and canonical single braced eval body.
A created child source cell remains separate from a Registry descriptor.
An exact original move retains that creation and original child path while
recording the current parent command spelling and move lineage separately.
One, repeated and qualified moves use the same source-order graph. Old heads,
shadowing, deletion, prefix aliases, unknown callbacks, transformed body text
and stale full input withdraw advice. Current child visibility and possible
source-load edges require the same child ledger. Successful creation,
materialisation and child entry remain independent; this body supplies no
runtime handle, allocation, frame or Normal. The [source contract](../analysis/name-resolution-proofs/interpreter-original-created-handle-source-body.md)
and [public move observations](../analysis/name-resolution-proofs/interpreter-original-created-parent-command-move.md)
retain separate implementation and native purposes.

`element_writes_observed_with_contexts` consumes the existing exact point
owners for selected original array commands. It retains the genuine array
operand and whole-array read inventory before classifying an element store as
observed. Raw written labels cannot replace that selected child expression.
The [array observer source controls](../analysis/name-resolution-proofs/original-array-observer-renamed-alias-controls.md)
measure only finite returned values; the [point-observer contract](../analysis/name-resolution-proofs/variable-original-point-element-observers.md)
retains the separate Rust registration and store/read ownership premises.

Runtime original object reads and assignments delegate the positively selected
observed naming purpose to the actual entered event-frame receiver with
complete counted bytes. `read_original_named_variable`,
`assign_original_named_variable` and `store_original_named_variable` preserve
read, assignment and result-publication distinctions. A retired frame cannot
supply a receiver. The [observed assignment contract](../analysis/name-resolution-proofs/variable-observed-original-object-assignment.md)
checks internal Rust ownership; it supplies no appliance frame or native cache,
header, pointer or compiler observation.

For original substitutions under the positively selected observed name
purpose, `observed_substitution_receiver` retains the complete combined name
and actual current event-frame level. Object-passthrough and interpolated-byte
paths share this receiver while keeping their result forms separate. A
separately supplied root/index or retired frame refuses; no trace registration
or upvar capability follows. See the [observed substitution contract](../analysis/name-resolution-proofs/variable-observed-original-substitution-receiver.md).

An uncalled child receiver body can retain possible visibility and SourceLoad
through its independent `OriginalSourceReceiverBodyDeclaration`, current full
input and exact complete invocation membership. A native deferred-body receiver
entry remains separate and may be absent. Its executing namespace stays
`None`, with `UnknownExecutingReceiverNamespace` and
`NoEarlierReceiverNamespaceBinding` obligations. A conditional global-fallback
source slot does not establish a private receiver namespace or actual callable;
inert braced method data supplies no body owner. See the [source visibility
contract](../analysis/name-resolution-proofs/interpreter-original-source-visibility-advice.md).

Deferred package bodies use their genuine contiguous `SourceFutureBodyInventory`
and `SourceCommandBindings::original_future_source_catalogue_candidate` to
obtain conditional source schema through the same `source_registry_words`
owner. `UnknownFutureEntry` remains explicit; the deferred invocation never
becomes selected document lookup. The current source, full configuration,
context generation, original declaration and complete vector must agree.
The authored global-entry descriptor is C-family-only and supplies no scheduling
or callback observation. Static Source filename words can retain source advice;
list-produced loaders without a genuine script receipt retain directory hints.
See the [file candidate contract](../analysis/name-resolution-proofs/original-package-file-candidate-provenance.md).

## Source snippet choices and canonical member syntax

Completion snippet variable choices use `original_variables::source_references`,
the same current visibility and complete source-reference decoder as variable
completion. Unknown Native or hosted spellings remain unavailable; only the
explicit lexical advice branch may read its reporting inventory. Snippet
escaping preserves complete reference spellings. `EventHandlerFacts::from_analysis`
retains actual source/configuration/ContextRegistry and authentic hosted event
cards through shared source structure. Event templates require the current
root script; a source event label supplies no worker reachability or binding.
See [snippet source context](../analysis/name-resolution-proofs/original-snippet-source-context.md).

Method syntax consumers use canonical `OriginalSourceMethodMetadata` worker
words: `parameters_word`, `body_word` and independently retained `source_dialect`.
Whole source/configuration and exact token ancestry remain mandatory. Missing
or incompatible source dialect supplies no formal grammar. Semantic colours
consume these operands after reporting maps are erased; these are syntax roles,
without evaluated ParamList, formal binding, method activation, entered body,
Normal or edit permission. See [member source roles](../analysis/name-resolution-proofs/tcloo-original-member-body-source-roles.md)
and [member colours](../analysis/name-resolution-proofs/original-member-body-colours.md).

The original instance-member rename planner uses that same whole `body_word`
and requires it in the actual current definition worker's Native words.
`NativeWord::content_span` owns the delimiters; `NativeCompilerWords` owns the
formed static value. Shared `native_source_literal_bytes` applies the selected
source-channel ingress before comparing content with that value, preserving
original document coordinates for Unicode and CRLF. A cooked body without this
original script correspondence remains unavailable. Reporting `body_span`
cannot supply script geometry. The member roster, canonical calls, visibility,
complete coverage and atomic current-owner obligations remain independent.
See the
[instance-member rename contract](../analysis/name-resolution-proofs/original-instance-member-rename-plans.md).

### Conditional hosted reads and package advice

`CompilationUnit::build_with_context_registry` consumes the caller’s retained
whole-source configuration and complete ContextRegistry for hosted diagnostics.
A profile label cannot recreate that input. `HostedSourceRead` combines guarded
source metadata with the actual positioned `SsaSourceView`: only its independent
`read_reference` receipt can select a cell and represented version. An exact
original sole-variable operand with unavailable contents remains MAY unknown.
A reporting SSA name or missing taint entry cannot make it clean. Literal,
braced and unsupported compound geometry stays separate from this read facet;
known source barriers remain in the shared conditional metadata owner. This
path creates no SSA symbol, variable presence, store, effect or Normal receipt.

Conditional formal incoming provenance reads the declaration graph's genuine
argument-entry point before operand evaluation. Its available-prefix paths
remain independent of later handler selection. Whole source/configuration,
immutable formal topology, quiet fresh scalar receiver and complete original
operand effects must still match; preceding uncertain writers, stores, aliases
and observers withdraw this advice. It grants neither an actual read nor an
entered frame, caller value or Normal result. See the
[formal argument-entry contract](../analysis/name-resolution-proofs/original-formal-argument-entry.md).

An independently supplied `TrustedPackageLoader` needs the selected original
Require, complete argv/schema/arity, actual context/Registry and a quiet known
source world. `OriginalTrustedPackageTransfer` authenticates those premises
before unresolved callback widening; it cannot restore unknown state.
`preload_dependencies_hold` and `publication_keys` share the fixed C-family
provider namespace and publication-purpose geometry. Existing provision,
revoked registrations, shadowed dependencies and physical Native entry decline.
The transfer retains model command slots, namespace metadata and its normal
source continuation while widening incoming variable values independently.
An attested package-only script can retain a complete source world. A following
namespace import independently retains its abrupt alternative: authentic
qualified/imported Normal-path slot correspondence does not certify completion
of the whole root script.
It supplies no physical namespace token, compiler prerequisite or runtime
package execution. See the
[attested loader contract](../analysis/name-resolution-proofs/package-attested-source-loader-transfer.md).

`DocumentFloor` reads `ResolvedAnalysisInput::availability_context` and genuine
original package-purpose and version operands. A completion caller’s profile
or changed reporting fields cannot donate a require or replace ambient context.
Version alternatives retain their own bounds and conditional requests cannot
establish an unconditional suggestion. These are catalogue and floor source
candidates, independent of a package loader, installed version or current
command publication. The exact contracts are
[hosted-source-taint-descriptor](../analysis/name-resolution-proofs/hosted-source-taint-descriptor.md)
and [original-package-completion-advice](../analysis/name-resolution-proofs/original-package-completion-advice.md).

`OriginalPackageFileCandidate` keeps filename advice provenance explicit.
Only a genuine selected loader body and shared Source schema supply a static
filename's whole original operand and conditional roles. Deferred dir/path text
has no future variable value receipt; directory entries have no filename word
or source schema. Inert data and produced-list loaders without a script/value
receipt cannot lend Source navigation. All candidates remain independent of
package provision, file existence, load, execution, Normal and edit permission.
See the [file-candidate contract](../analysis/name-resolution-proofs/original-package-file-candidate-provenance.md).



### Package-index paths, hints and readonly labels

Original package-index reachability consumes the actual complete loader Analysis,
retained ContextRegistry and Registry generation through the common source schema.
Exact written package registration operands remain distinct from filename
candidates. Literal conditions and completion options prune only with their own
closed source lookup and operand-effect premises. Conditional applicability,
unknown commands and unavailable body geometry preserve Undecidable paths. The
running interpreter release does not prove its current Tcl package provision;
version-dependent conditions retain that missing premise. See the
[package source-control contract](../analysis/name-resolution-proofs/original-package-reachability-source-controls.md).

Inlay consumers normalize their caller profile and Registry to the actual retained
analysis after validating its whole source and full configuration. Nested format
roles share SourceStructure, and formal labels use the same original effective
argv/child mapping as semantic colours. Independent variable and parameter proofs
remain required; a source hint creates no value, entered frame or edit. The
[retained hint contract](../analysis/name-resolution-proofs/original-inlay-retained-context.md)
keeps those inputs explicit.

`native_string::resident_name_label` is a presentation owner for counted resident
bytes. Escaped controls, invalid bytes and literal escape spellings remain distinct
in a readonly card. A consumer must retain the independent original input, purpose
and geometry for lookup, collision checks or edits; decoding this label cannot
replace any of those inputs.

## Original class-call and constructor signature source advice

`source_constructor_call_at` obtains the genuine whole current call under the
retained Analysis input, source, configuration and ContextRegistry.
`OriginalSourceClassDeclaration` retains its selected definer, canonical name
operand/site and source move/alias lineage. `OriginalSourceConstructorCall`
composes actual effective operands and captured prefixes without assigning
written spans to captures. The declaration-owned family grammar and original
member ledger select manufacturer shape/visibility; report metaclass strings
supply neither. Unknown expansion length stays unknown.

`OriginalConstructorArityAdvice` retains the assessed expected signature and
actual count for Core transport, independently of diagnostic messages. Genuine
TclOO constructor parameter words and mandatory object-name positions remain
distinct from provider-authored Snit bare construction. Known replacement,
delete, alias cycles, private/overridden manufacturers and foreign whole source
retain refusal. These are source-purpose joins, with no installed class,
successful construction, filter closure, runtime entry, Native admission or
Normal. See the [source class-call contract](../analysis/name-resolution-proofs/source-original-class-constructor-call.md).

## Current original source region and value purposes

`visit_analysis_executable_commands` and
`innermost_analysis_executable_region_at` retain the actual Analysis source,
full lexer configuration, Registry, resolved input and immutable realm.
Shared original Body/case/lambda/member geometry preserves source ancestry;
method/receiver temporal identity is independent. A profile-only walker cannot
replace this Native or hosted owner. Its positive Logical/Tk compatibility
branch retains its separate scope. See the [Core region contract](../analysis/name-resolution-proofs/core-original-executable-region-context.md).

`OriginalSourceScriptPurpose::Syntax` retains reference-only body text for
source assistance. `PotentialEvaluation` queries the same selected script
timing at each original effective ordinal and excludes ReferenceOnly; Core
executable traversal and mutation coverage request that purpose explicitly.
Potential evaluation supplies no reached body, frame, effects closure or
Normal. See the [script-purpose contract](../analysis/name-resolution-proofs/original-script-region-purpose.md).

A genuine original future procedure call can walk its borrowed body in a
private conditional graph using the later current factory metadata. Original
formal/body parents, full source/config/context, known source replacements,
C declaration namespaces and Jim flat source naming remain independently
validated. Local handles do not borrow parent values or escape into the
caller; depth and total work are finite. This source walk does not materialise
formals, enter a frame or publish body effects. See the [future body contract](../analysis/name-resolution-proofs/source-original-future-procedure-body.md).

`DynamicVariableNameSite.resolved` remains compatibility source-value advice.
`site_resolution_rules_out` may narrow a refusal only in positively selected
Logical lexical declaration analysis; Native/Jim/hosted sites require their
independent original value/cell receipts and cannot borrow this presentation
field. Complete coverage, collision and edit gates remain separate. See the
[value-purpose contract](../analysis/name-resolution-proofs/core-original-dynamic-name-value-purpose.md).

## Original source naming, data and formal purposes

`source_command_publication_at` locates an original naming declaration under
the actual selected schema, full source/config/context and option/member
layout. It retains the complete naming word and source-produced bytes, rather
than an installed command. `source_handle_class_advice_at` joins the actual
setter and its single original constructor substitution to that same factory
receipt. Compatibility class labels grant no stored handle, allocation,
lifetime, Normal or rename authority. See the [source publication contract](../analysis/name-resolution-proofs/original-command-name-publications.md).

Core variable cursor consumers pass the actual `AnalysisResult` to
`definition::substituting_var_at_position` and `lookup_var_read_at`.
`inert_text::offset_in_data_brace_in_analysis` shares original whole-word and
Body/case/lambda/expr geometry through `SourceStructure` and the current
`OriginalRegistryWords` source schema. Effective argument mapping uses written
origins; captured prefixes cannot own a document span. `Some(true)` is data
under that schema, `Some(false)` is original non-data source, and `None` is
unavailable ownership or layout. Preserve `None`; do not rebuild a static
Registry from a dialect label or treat it as a live variable reference.

This source classification grants no runtime substitution, variable cell,
entered frame, complete reference inventory or edit permission. Scope-based
lookup requires positive `allows_lexical_declaration_advice`; Native naming
consumers use their original variable owner. The caller-frame cursor adapter
delegates to this same gate, and lexical caller-reference scans include only
`Some(false)` source positions. Declaration-span selection remains independent,
so literal variable-name words can retain their own declaration without
inventing an unrelated `$` reference. Expression string literals keep their
shared lexical purpose; cooked lambda bodies and case patterns have no borrowed
script offsets. See the [source classification contract](../analysis/name-resolution-proofs/core-original-data-brace-classification.md).

Method and receiver command traversal uses
`executable_regions::visit_analysis_executable_commands`; signature cursors use
`innermost_analysis_executable_region_at`. Pass the complete retained analysis
and its current Registry. These facades derive source/configuration/context and
Realm from that owner; a source mismatch, foreign Registry or changed complete
configuration withdraws the traversal. Potential source regions do not grant
execution or runtime frame authority. See the [original executable region
contract](../analysis/name-resolution-proofs/core-original-executable-region-context.md).

Multi-contributor concatenated expressions retain unknown until an original
joined expression mapping is supplied. Do not independently reinterpret each
contributor as a complete expression or literal data.

Name-only formal lowering selects both `WordValueRules` and `ParameterGrammar`
from the actual source ingress and uses `parse_formal_parameters_in` for
procedure and member headers. Missing grammar declines; Jim qualified,
array-shaped and link-spelled names retain their own source syntax. Syntax
acceptance supplies no ordinary C positional/default/rest or caller-link
activation recipe. See the [readonly formal contract](../analysis/name-resolution-proofs/original-readonly-formal-topology.md).

## Selected formal syntax and script-purpose consumers

`parse_param_list_strict_in` requires independently selected word-value rules
and parameter grammar. It validates syntax only; the compatibility
`parse_param_list_strict` has the authored C purpose. E006 formal/lambda
diagnostics and original procedure replay select their actual grammar, and an
unknown axis declines. The ordinary C inliner and bounded lambda activation
require the C parameter axis independently; Jim names and reference headers
are not C frame evidence. Structural prefix counts keep the central finite
arity representation. See the [readonly formal contract](../analysis/name-resolution-proofs/original-readonly-formal-topology.md).

The original body inventory defaults to `Syntax`, including reference-only
source. Executable regions, conditional interpreter-body source visibility and
lexical store suppression explicitly request `PotentialEvaluation`. A
reference-only Body role cannot supply those purposes. Native/hosted rename
coverage and nested reference-dispatch regions pass the retained actual
analysis to the same source walker. Method/callable temporal identity and
complete edit coverage remain separate. See the [script-purpose contract](../analysis/name-resolution-proofs/original-script-region-purpose.md)
and [Core retained context](../analysis/name-resolution-proofs/core-original-executable-region-context.md).

## Empty TclOO constructor lifecycle

An original empty constructor body is a lifecycle definition, rather than an
installed zero-command procedure. The [finite original source-file captures](../analysis/name-resolution-proofs/tcloo-empty-lifecycle-definition.md)
retain C8.6/9.0/9.1 own-constructor removal before formal validation, nonempty
space/comment arity and inherited constructor selection. Those ASCII process
results do not grant a source carrier, current callable, entered frame, object
cache, compiler recipe or arbitrary callback closure. C8.4/8.5/Jim stock
processes report unavailable TclOO; external BIG-IP remains untested. Source
constructor diagnostics retain their independent original declaration and
arity receipts.

## Retained metadata, UI and constructor source advice

`InvocationMetadataContext` takes the actual full availability context and
command generation. Use the shared metadata-context entry for role, body,
symbol and variable-name queries. Positioned completion and representation use
`resolved_tokens_invocation_in_context` and
`normal_representation_invocation_in_context` with the actual ContextRegistry.
Use `registry_invocation_assistance_in_context` for candidate/residual metadata
and `resolved_statement_invocation_in_context` for genuine original Statements.
`normal_transfer_invocation_in_context` selects supported normal variable
transfer; `possible_variable_name_operands_in_context` retains possible name
positions and unknown alternatives. Neither projection supplies body entry.
Diagnostic constant-harvest operands use the selected argument_literal grammar.
These shared entries retain independent original binding, native/normal-handler
and operand requirements; availability supplies none of their missing proof.
A display profile or refused availability cannot supply a replacement generation. Standalone Tk source modelling also
retains the explicitly supplied profile/full grammar and provided Registry
store in one actual analysis. Captured and expanded operands cannot acquire
later written widget spans. See the [metadata contract](../analysis/name-resolution-proofs/original-analysis-metadata-context.md)
and [Tk source contract](../analysis/name-resolution-proofs/core-original-tk-source-context.md).

For an IR consumer, take the context from `Module::source_metadata_input`,
which the actual Lowerer retains from its configured ingress. For an editor
consumer, take it from `AnalysisResult::resolved_input`. Preserve the selected
availability, Registry store and full word grammar together. Missing input
returns unavailable; `Module::dialect_profile`, a display label or a catalogue
snapshot cannot construct replacement input.

```rust
fn selected_metadata(
    module: &tcl_compiler::ir::Module,
    tokens: &tcl_compiler::ir::CommandTokens,
) -> Option<tcl_compiler::registry_invocation::ResolvedStatementInvocation> {
    let context = module.source_metadata_input.as_ref()?.context_registry();
    tcl_compiler::registry_invocation::resolved_tokens_invocation_in_context(
        &context, tokens,
    )
}
```

Choose the purpose-specific query rather than widening this example's result.
Source body, symbol and diagnostic consumers use their conditional original
source carriers; native codegen, runtime effects and writable references retain
their independent admission, handler, frame and edit requirements. Tests pair
available metadata with refused availability, a foreign store and missing
original binding under the same genuine ingress.

Acquire `source_constructor_call_at` from an actual retained child command-head
offset. For a setter RHS, `source_handle_construction_at` retains its genuine
selected setter and whole `original_single_command_substitution_words` vector.
Do not trim or split a reporting value. `original_source_body` owns the selected
enclosing body. Logical class-call syntax retains Logical names and body
applicability while `constructor_shape` abstains. Native constructor shape
requires its separate original class/member and naming-input join. Top-level
manufacturer visibility uses only effects before the call; deferred bodies
retain a separate conditional final-source horizon. Uncalled original procedure
inventory carries `FutureOriginalProcedureSourceApplicability`, without a
written call, parent value, entered frame or permanent created-command set.
See the [constructor source contract](../analysis/name-resolution-proofs/source-original-class-constructor-call.md)
and [future body contract](../analysis/name-resolution-proofs/source-original-future-procedure-body.md).

Selected definition-member formal consumers call
`source_member_formal_validation_applicability` or the special-member companion
with the genuine selected grammar and static counted original body. The shared
lifecycle disposition returns `Required`, `SkippedRemovedLifecycle`, or unknown.
An empty word differs from missing evidence; spaces, comments and counted NUL
values remain nonempty. Dynamic bodies cannot borrow empty source spelling.
`member_formal_parameter_indices` supplies the E006 formal ordinals. Original
empty declarations remain source records while inherited signature selection
is independent. The [native lifecycle record](../analysis/name-resolution-proofs/tcloo-empty-lifecycle-definition.md)
retains its exact fixed ASCII process observations separately from linked
source implementation controls and unmeasured counted-NUL values.

### Retain metadata across bodies, callbacks and CFGs

`InvocationMetadataContext::from(&ContextRegistry)` borrows the supplied full
availability and command-store generation. Retain packages, authoring scope,
realm and dialect axes through each query. The shared `_in_context` adapters
join that metadata to the original invocation; they do not select an otherwise
unknown handler. Internal `_with_metadata_context` kernels accept the same
borrowed carrier. An explicitly static `SemanticContext` is a separate caller
contract, not a replacement for unavailable analysis input.

Source-body consumers acquire `OriginalRegistryWords` from the complete source
and actual analysis. Select `OriginalSourceScriptPurpose::Syntax` for readonly
syntax, including reference-only operands. Select `PotentialEvaluation` for
possible source evaluation; it excludes reference-only bodies and still grants
no actual frame, assignment, completion or effects. A returned region keeps its
whole parent word and genuine extent. An empty vector supplies no region for
this projection, not proof that execution has no children.

```rust
fn potential_source_bodies(
    source: &str,
    analysis: &tcl_compiler::analyser::AnalysisResult,
    offset: u32,
) -> Option<Vec<tcl_compiler::registry_invocation::OriginalSourceScriptBody>> {
    use tcl_compiler::registry_invocation::{
        source_structure::source_registry_words_at, OriginalSourceScriptPurpose,
    };
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let words = source_registry_words_at(source, analysis, offset)?;
    Some(words.source_script_bodies_for(
        &context, OriginalSourceScriptPurpose::PotentialEvaluation,
    ))
}
```

Callback source advice additionally needs the genuine registration horizon and
original prefix, rather than a name or scalar argument count. Join the shared
`source_callback_procedure_target_at` receipt through
`SourceCallbackSignatureLookup::from_original_lookup`. Its canonical local
header, held alias target and structured refusal remain separate. Keep alias
captures, the callback's own baked prefix and its appended suffix as three
quantities, with their original operand origins. Known replacement, deletion,
cycles, foreign interpreter and unavailable relative trigger scope are terminal.
An absolute C source-name recipe can support its bounded source join without
supplying the future trigger frame. A source prefix's lookup geometry alone
cannot establish current target occupancy or an installed callback.

For IR consumers, `Module::source_metadata_input` retains the configured
Lowerer ingress. It participates in module equality and must travel through
secondary body builders, call-site scans and lattice requests. The module's
lexer configuration, original command bindings and source entry remain
independent required inputs. A Registry snapshot or profile label cannot
recreate an absent `ResolvedAnalysisInput`.

| CFG metadata mode | Entry | Behaviour |
| --- | --- | --- |
| Supplied analysis | `PreparedCfgContext::from_source_input(context, registry, Some(input))` | Uses full retained availability; a foreign command store withdraws metadata. |
| Unavailable analysis | `PreparedCfgContext::from_source_input(context, registry, None)` | Keeps metadata unavailable; does not enter standalone mode. |
| Explicit standalone body | `build_cfg_function_with_upvars_and_config` | Uses the caller's independently selected Registry context and exact lexer configuration. This contract cannot fill missing document input. |

Prepare module facts once and share the same `PreparedCfgContext` across its
ordinary and method CFGs. The public ordinary-body entry is
`build_cfg_function_with_prepared_context`; module builders retain the same
command classes for every body. For example:

```rust
fn top_level_cfg(module: &tcl_compiler::ir::Module) -> tcl_compiler::cfg::Function {
    use tcl_compiler::cfg_builder::{
        build_cfg_function_with_prepared_context, prepare_cfg_context_with_registry,
        PreparedCfgContext,
    };
    let registry = module.resolved_registry();
    let facts = prepare_cfg_context_with_registry(module, registry);
    let prepared = PreparedCfgContext::from_source_input(
        facts, registry, module.source_metadata_input.as_ref(),
    );
    build_cfg_function_with_prepared_context(
        "::", &module.top_level, true, registry, false, &prepared, module.lexer_config,
    )
}
```

The prepared metadata mode supplies no interpreter allocation, entered frame,
native opcode or Normal effects. CFG body/effect consumers still use their
original invocation and admission queries. Context-sensitive changes belong at
that shared owner, with every metadata consumer preserving its modes and
refusals. Controls distinguish the same catalogue under available and unavailable
actual contexts, a foreign store, missing input and known source replacement;
a warning count or display-name match cannot prove the join.
See the [metadata contract](../analysis/name-resolution-proofs/original-analysis-metadata-context.md),
[source-body purpose](../analysis/name-resolution-proofs/original-script-region-purpose.md)
and [callback target contract](../analysis/name-resolution-proofs/source-original-callback-procedure-target.md).

### Executable IR and semantic metadata

Use `build_linear_executable_ir_with_metadata_context` for executable source
metadata. It accepts the actual `InvocationMetadataContext`; missing or foreign
input returns `ExecutableMetadataBuildDecline::ContextUnavailable` before source
visitation. A source shape outside the supported projection returns
`ExecutableMetadataBuildDecline::Source`. Recursive bodies and original
substitutions retain the supplied availability. A profile label or matching
catalogue cannot recreate this owner.

`FunctionUnit::invocation_metadata_context` validates its retained input against
its actual command store and source grammar. A consumer combining a function
with its Module uses `invocation_metadata_context_for_module`; both retained
inputs must share the actual `ContextRegistry`, analyser and unit profiles and
hosted source policy. Each grammar is checked against its own input, including
nested-body BOM policy and lexer overlays. Clearing the Module input cannot be
repaired by a surviving function copy. For example:

```rust
fn executable_source_metadata(
    unit: &tcl_compiler::compilation_unit::CompilationUnit,
    registry: &tcl_registry::CommandRegistry,
) -> Result<
    tcl_compiler::executable_ir::ExecutableFunction,
    tcl_compiler::executable_ir::ExecutableMetadataBuildDecline,
> {
    use tcl_compiler::executable_ir::{
        build_linear_executable_ir_with_metadata_context, ExecutableFunctionId,
    };
    let module = &unit.ir_module;
    let context = unit.top_level.invocation_metadata_context_for_module(registry, module);
    build_linear_executable_ir_with_metadata_context(
        registry, context, ExecutableFunctionId::new(0), &module.top_level,
    )
}
```

`SemanticAnalysisBundle::build_with_metadata_context` retains full semantic
facts under this owner. Its interactive companion,
`build_for_interactive_analysis_with_metadata_context`, retains source
invocations while preserving the selected dispatch entry and world-state cost
contract. Function-unit builders use `with_retained_semantic_analysis` with the
original body and their retained input. `CompilationUnit::with_retained_deep_semantic_analysis`
uses the supplied full builder across its top level, procedures, methods and
body units. Lattice and memoised body callers retain
the genuine nested input; they preserve vendor policies and availability while
projecting the body's lexer coordinates.

When no body is supplied,
`SemanticAnalysisBundle::unavailable_with_metadata_context(registry, context)`
retains a valid metadata owner and reports `SourceUnavailable`. Missing or
foreign context reports `ContextUnavailable`. Readonly metadata queries remain
independent of the absent executable body.

For a supplied bundle, `metadata_context()` exposes the complete retained
`ResolvedContext`, including overlays and package availability. `context()`
belongs to the explicitly standalone scalar `SemanticContext` API and remains
`None` for supplied bundles. In an explicit standalone bundle,
`metadata_context()` exposes the exact resolved context of the already retained
scalar handle. The getter supplies no default context for missing input. The
explicit standalone builders remain separate contracts and cannot serve as a
fallback for withheld document input.

Metadata availability supplies no Native command entry, actual activation,
compiler admission, variable receiver, successful normal completion or effect
closure. Codegen retains those independent proofs and the original source
words. A structured lowering node's operation label cannot recreate consumed
command tokens or donate a binding. Tests distinguish same-store availability,
missing and foreign owners, stale grammar, conflicting original carriers and
nested lexer overlays. See the
[analysis metadata contract](../analysis/name-resolution-proofs/original-analysis-metadata-context.md).

## Typed original callback signatures

Retain `OriginalCallbackPrefix` independently of mutable invocation display
names, scalar counts and spans. It owns the readonly original name child, baked
words, selected suffix-count descriptor and lookup scope. An unavailable trigger
frame does not borrow the invoking frame. Use
`SourceCallbackAritySubject::from_source_signatures` with exact original
procedure headers and selected formal grammar. Finite alternatives and lower
bounds remain distinct; unknown counts or headers decline. The typed issue owns
the mismatch before presentation is constructed. An unavailable decoded child
substring falls back only to the whole genuine readonly parent operand, never
an edit span.

The database `original_lookup_command_signatures` query retains ordered original
slot/policy alternatives and body-free current headers. A qualified callback
cannot borrow a different namespace's same-tailed declaration. Body-only edits
preserve the header contract; formal edits update it. Core transports source
purpose, counts, lookup scope, declarations and typed issue independently of the
message. No callback registration, actual argv, future callable, activation or
entered frame is established. See the [callback signature contract](../analysis/name-resolution-proofs/diagnostic-original-callback-signature-subject.md).

Inherited constructor source arity uses the actual provider's own original
factory grammar, naming policy and formal dialect. It does not borrow the
child's axes. Static empty own lifecycle removal proceeds to inherited/default
source signature; missing or dynamic body remains unknown. These source controls
are distinct from the [fixed ASCII native lifecycle observations](../analysis/name-resolution-proofs/tcloo-empty-lifecycle-definition.md).

## Original comments, definers and receiver consumers

Comment consumers pass the actual retained Analysis into
`comment_facts_from_analysis`. The shared whole-image physical projection and
common Syntax body inventory preserve full grammar/configuration/context and
Registry generation. Genuine declared Body syntax with unspecified timing and
ReferenceOnly remain Syntax; they cannot supply PotentialEvaluation. W115
corrections use complete physical lines and refuse inline command prefixes beside
body comments. Cooked, compound, dynamic, stale and unmapped coordinates decline.
See the [comment source contract](../analysis/name-resolution-proofs/core-original-comment-source-context.md).

Family source handlers consume `OriginalClassDefinerSource` from the selected
factory schema/effective argv and canonical original class declaration/name.
Moved and captured prefixes retain their original producer anchors. Reporting
class or metaclass labels cannot select family grammar.

Readonly receiver consumers use `source_class_instance_words_at` under the
actual whole source/configuration/context. `OriginalSourceClassInstanceWords`
retains constructor, selected declaration, optional genuine setter, full later
receiver vector and conditional obligations. Direct named receivers follow
original moves; named symbolic handles refuse changed original slots rather than
updating a variable value. Anonymous results have no fabricated slot. Private or
overriding manufacturers, terminal replacement/deletion and stale ownership
refuse. Pending method assistance and whole receiver inventories share this
facade; reporting created-command maps do not supply the join. These are source
contracts with no successful construction, stored value, allocation, runtime
entry, Normal or edit authority. See the [receiver contract](../analysis/name-resolution-proofs/source-original-class-instance-receiver.md).

## Original callback source purpose and native controls

Ordinary and registered-instance callback positions use the authentic selected
source schema, exact prefix producer and `OriginalCallbackPrefix` issuer.
`OriginalSourceRegisteredInstanceWords::argument_input` retains the genuine
producer's policy domain; a Native consumer requires its independent
`native_input`. Captured operands preserve earlier source anchors and acquire no
later written span. Source callback inventory retains no actual argc or
navigation/edit authority.

`SourceDeclarationSignature::from_original_procedure` is the public body-free
header issuer from canonical original procedure metadata. Header formal origin,
count, grammar and publication geometry remain independent of body bytes and
installed identity. `SourceCallbackAritySubject` consumes those headers and its
own original prefix lookup. Unavailable relative TriggerFrame lookup declines;
a selected C absolute head can retain source target-slot geometry without a
caller namespace. It supplies no future callable or entered trigger frame.
Unrelated package/unresolved/interpreter subject accessors return None for
CallbackSourceArity; Core requires typed issue/code correspondence before
transport, independently of message text.

The [absolute trace captures](../analysis/name-resolution-proofs/callback-absolute-trace-source-lookup.md)
retain five C public processes: relative cb selects B, absolute ::Dest::cb selects
Dest, and both report triggering caller ::B. Variable/command suffix counts are
three; execution enter/leave counts are two/four. Jim catches unavailable trace
for all six installations. Independent release-source excerpts retain CString
Tcl_Eval for C8.4/C8.5 execution callbacks, counted Tcl_EvalEx(flags0) for C8.6+
execution callbacks, and counted EvalEx(flags0) for all five variable/command
callbacks. No original object/header or compiler-body authority follows.

The [alias captures](../analysis/name-resolution-proofs/callback-alias-capture-source-control.md)
retain FIXED, then BAKED, then the two public comparator inputs; nested captures
retain INNER before OUTER. Moving the alias retains capture; moving its target
fails late target-name lookup. A four-formal replacement needs BAKED, with the
exact release-specific error label preserved. These native public-source controls
remain independent of source signature advice and give no internal native
argv/object/header/cache, current installation, runtime frame or edit authority.

## Positioned local callee body availability

Pass the original `Statement` to
`collect_positioned_opaque_callee_name_args`. `UnitCommandResolver::resolves`
requires its retained original source binding and independently proved execution
target. Registry targets retain authentic identity under the actual generation
and context; procedure targets require their exact implementation allocation site
in the canonical original declaration inventory. Final reporting names, future
procedures, stubs, factories, aliases and namespace tails cannot supply a Native
local body. Positive Logical source naming selects its separate procedure-name
compatibility. Unavailable local bodies retain literal variable-name argument
candidates for opaque-callee assistance. Body availability does not close callee
name effects or establish an entered frame, Native execution, Normal or edits.
See the [positioned callee contract](../analysis/name-resolution-proofs/original-positioned-callee-body-availability.md).

## Source instance completion and authentic callback scanning

`InstanceCompletionSource` retains a complete original conditional receiver
receipt and canonical class declaration or a separately positioned current
receiver with genuine written words. Primary/fuzzy method candidates and property
options share effective argument producers under the complete current source,
configuration and context. Captured selectors own no later written replacement;
method position requires the whole original selector or selected intra-command
whitespace/continuation. Later arguments and separate commands decline. Narrow
completion spelling replacement retains independent original name/source edit
checks and supplies no member rename identity. Positive Logical compatibility
keeps class and method bucket together; Native rename hazards require actual
positioned receiver identity independently of source candidates or display maps.
See the [source instance completion contract](../analysis/name-resolution-proofs/core-original-source-instance-completion.md).

Background callback inventories emit only authentic `OriginalCallbackPrefix`
metadata through `reported_source_head`. A matching original inventory is
terminal, including refusal; replaced/deleted installers cannot borrow a nominal
header callback role. Header-only scans retain independently selected authoring
ingress and evaluator-checked readonly lineage with lookup absent. Foreground and
background callback argc remains unavailable and rename_safe remains false.
These source controls establish no reached callback or native argv/frame.

## Original next-line comments and sealed member regions

Physical comment consumers with an `AnalysisResult` call `script_comment_facts_from_analysis(source, analysis)` or `parse_noqa_line_suppressions_from_analysis(source, analysis)`. These retain the actual immutable input, complete source image, full lexer configuration and Registry generation. `None` means unavailable ownership; consumers preserve that refusal instead of resolving a dialect label or treating an unknown region as script. Core's comment/style facade delegates to the same Compiler owner. Standalone source-only APIs accept an explicit fresh source/configuration/Registry scope.

`original_script_comment_facts` owns exact physical comment slices and whole-image line prefixes. Original Registry bodies use `OriginalSourceScriptPurpose::Syntax` for comment/navigation inventory. `PotentialEvaluation` requires the independently authored timing owner and excludes `ReferenceOnly`. A declared `Body` without timing supplies Syntax only. These source issuers supply no entered frame, current binding, Native compiler admission, completion or rewrite equivalence. A W115 correction uses genuine syntax applicability and complete physical edit lines.

Definition-member consumers retain an `OriginalSourceScriptBody` selected through the genuine parent source schema, then call `definition_member_region(context, actual_current_script_span)` once for that region. The sealed `OriginalSourceDefinitionMemberRegion::script_bodies` accepts only a complete argv in its retained original lexical plan. Its selected member grammar uses the actual availability query and real byte-valued option/wrapper selectors. Bodies retain whole `NativeWord` containers; dynamic selectors, expansion, cooked/compound bodies, foreign words and trimmed argv do not acquire replacement geometry. Only an explicitly selected wrapper block returns `definition_parent()` for a descendant region. Ordinary method, lifecycle, accessor and lambda scripts do not inherit definition vocabulary.

`DefinitionBodyGrammar::source_member_script_arguments_in` and the static member-role/body projections share one layout kernel. Extend the Registry descriptor and that kernel for member grammar changes; update original-word geometry tests and all purpose-specific callers together. Consumers do not render names, formals or bodies to recover layout. Unknown selectors remain unavailable even if an unrelated reported string looks useful.

`OriginalRegistryWords::source_expression_script_bodies(actual_input)` owns command-region spans inside a whole original expression operand. It uses the same full configuration and actual profile's expression grammar. Multiple expression fragments, cooked words, missing ownership and unknown expression grammar decline. Core SourceStructure, executable traversal and the comment inventory reuse this projection rather than tokenising expressions independently.

Next-line suppression inventory is initialised after the analyser retains its actual input, declared contracts and original command realm and before command diagnostics are emitted. Whole-file, chunked, command-snapshot and per-item entry points use the same owner. Tests compare those entry points and separately check custom Registry bodies, declared syntax-only bodies, ReferenceOnly syntax, known replacements, Unicode spans and refusal boundaries. These Rust bindings do not constitute native interpreter observations.

## Conditional callback procedure targets and signature counts

`source_callback_procedure_target_at` retains the genuine registration source
horizon and a structured target/refusal. Canonical local procedure metadata is
joined by original declaration site/name under the complete source context. An
authored alias held external target retains its own source slot/name and policy;
it is separate from a missing direct local target. Nested captures retain their
producer words and inner-before-outer order without flattening a native argv.

`SourceCallbackSignatureLookup` carries the retained source selection and
body-free canonical local header. `SourceCallbackAritySubject::from_source_lookup`
keeps alias-captured, baked-prefix and appended-suffix counts separate, including
expansion uncertainty and checked arithmetic. Database external headers use the
held target slot/policy or genuine prefix lookup only when UnknownSourceTarget
permits it. Known deletion/replacement/nonprocedure/cycle barriers, unavailable
relative trigger frame, external interpreter and ambiguous source horizons stay
terminal. Core transports target kind, slots, counts, declarations, obligations
and typed issue independently of report messages. No current registration,
future callable, entered frame or internal native argv/object/header follows.
See the [callback target contract](../analysis/name-resolution-proofs/source-original-callback-procedure-target.md)
and the independent [finite native alias controls](../analysis/name-resolution-proofs/callback-alias-capture-source-control.md).

Family base-class list source layout uses
`DefinitionBodyGrammar::source_base_class_list_argument` from the selected
Registry grammar. Reporting member labels do not select that layout. W123
retains its sole original positioned presence issuer; quoted diagnostic message
text and nominal callable populations supply no post-filter.

## Original class-reference producer and consumer

`source_class_reference_at` joins a genuine readonly original list child at its
authentic consuming command. Registry base-list layout supplies Jim BASES
ordinals. An earlier two-word procedure retains its original whole name input
and declaration namespace separately from a later adopting class factory;
its original conditional slot must still be current. Current class moves retain
the canonical original declaration; command aliases, class delete/shadow and
moved-procedure adoption decline. Keep the complete immutable source, lexical
configuration, selected context and unresolved mutation obligations. A display
name split cannot issue this child. `class_declaration().source_class(analysis)`
selects canonical original metadata without a final reporting-map lookup. This
source reference grants no runtime inheritance, class object, frame, dispatch,
navigation identity or edits. See the [original class-reference contract](../analysis/name-resolution-proofs/source-original-class-reference.md).

Core W123 emission uses its original positioned presence issuer. Server
workspace reporting refinement has a separate, independently Logical-only
compatibility branch; it cannot settle Native or hosted unresolved subjects.

## Canonical class source metadata and original navigation

After a genuine class call/reference selects its canonical original declaration,
`retain_original_source_class_update` accumulates the freshly walked metadata
under the exact original occurrence/name and creation name/body geometry. An
ambiguous source publication declines; reporting maps cannot select the row.
Readonly consumers keep using the canonical original member ledger. Genuine
two-word procedures use the shared `OriginalSourceProcedureBody::source_procedure`
join, also used by callback targets. Their authentic class reference and exact
readonly second list child retain the selected Registry SourceValue purpose,
original ParamList/Body words, independent procedure dialect and producer site.
A readonly child cannot become a complete static naming word. Call-source
horizons and known mutation barriers remain explicit. The [native Jim chronology](../analysis/name-resolution-proofs/class-jim-source-member-chronology-control.md)
answers only its retained public processes; successful public rows do not grant
these source carriers a Native object, table, MRO, frame or edit capability.

`declaration` preserves the selected original alias-declaration span and sends
other Native/hosted navigation to the sealed definition variable/caller-template
owner. Missing original roots or command realm, copied reports and a changed
whole image cannot select a fresh nominal scan. Only actual explicit Logical
input admits the separate compatibility scanner. Call-hierarchy preparation
likewise treats a retained variable cursor as terminal before selecting a
genuine independent canonical method declaration. See the [declaration navigation contract](../analysis/name-resolution-proofs/core-original-declaration-navigation.md).

## Shared original definition vocabulary

`OriginalSourceScriptBody::definition_parent_for` owns source definition
vocabulary. An authored definition-body schema selects its genuine body. A nested
ordinary body retains its source parent only with actual Plain/current metadata
and SameInvocation timing for every Body position, under matching full context,
image/configuration and containment. Structural, deferred, ReferenceOnly,
unrelated and unavailable bodies decline. This supplies source vocabulary,
independently of PotentialEvaluation, Native handler, entered frame, completion
or edits.

Retain `OriginalSourceDefinitionMemberRegion` for the genuine current region.
Obtain its complete argv through `original_command_words_at`, then ask
`member_keyword` for actual source availability and `script_bodies` for layout.
Keyword applicability and body layout are separate. The selected Registry member
layout owns optional positions and wrapper/body ordinals. Wrappers retain the
genuine parent; method/procedure/lambda bodies clear it. Comments and next-line
suppressions, executable traversal, semantic tokens and readonly SourceStructure
share this issuer and parent rule. Declared Body syntax retains its separate
source contract without borrowing a Registry implementation or missing timing.

Semantic colours retain an escaped member keyword as one complete physical
source token and use LineIndex Unicode coordinates. Actual retained realm is
required; copied reports cannot select a nominal replacement. Readonly structure
keeps its authentic parent queue independently of emitted-span deduplication;
readonly wrapper/declaration geometry supplies no descendant syntax compaction.
The named Rust controls test these source obligations, not Native execution.

## Actual input for declaration-abort and pending reporting advice

Use `AnalysisResult::allows_retained_logical_declaration_advice` for both class
and per-object W315 reporting admission. It requires the actual retained
ResolvedAnalysisInput Logical purpose together with independently selected
lexical advice and source/vendor exclusions. Native/hosted/missing input and
copied labels/flags decline; both emitters drain candidate reports before
consulting compatibility tables. Authored Native retraction tombstones remain
source records. Genuine Logical advice retains its bounded receiver,
conditional-state and same-name model. Declaration/name/member order and String
reports establish no selected Native worker, closed own-method table, guaranteed
Native abort, object/class allocation or entered frame. See the separate
[class](../analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md)
and [per-object](../analysis/name-resolution-proofs/diagnostic-original-object-abort-admission.md)
admission contracts.

Pending scalar-variable constructor geometry retains the genuine setter and
whole source words. Constant labels and final reporting class maps select only
positively retained explicit Logical compatibility. Missing actual
result.resolved_input cannot be replaced with a reconstructed default or copied
advice flag. Genuine original class call/reference/receiver carriers remain
independent of these reporting helpers.

## Original factory slots and user-name units

Query `DefinitionBodyGrammar::source_two_word_factory_initialisers` under the
genuine selected Jim class grammar and pinned InvocationDialect. The shared
source graph installs those conditional slot barriers before original member
adoption. Earlier canonical procedures retain birth metadata but cannot donate
current member or callback headers through a known replacement slot. Genuine
later raw declarations can retain their own independent producer. The original
BASES list supplies exact empty/nonempty/unknown conditions; unavailable count
withholds old-header advice without claiming executed replacement.

`NativeNameProtocol::jim_two_word_member_publication_slot` composes the original
class units, a space and selected Registry member, then delegates the existing
procedure or alias publication purpose. Alias original/selected bytes retain their spelling. Both alias and procedure
command-table comparison strip the leading root run through the shared owner;
their namespace qualification remains separately selected. This pure owner grants no source word, current class,
installed worker or Native header. The [source class-reference contract](../analysis/name-resolution-proofs/source-original-class-reference.md)
keeps the [native factory observations](../analysis/name-resolution-proofs/class-jim-source-factory-initialiser-replacement.md)
independent.

Original Native class, instance and setter-variable user-name units use their
selected publication/variable owner without ASCII reporting normalisation.
Whole original producer geometry, explicit empty/NUL refusal and separately
bounded Logical naming remain. Registry schema selectors and mandatory factory
naming slots retain their own independent descriptors. A source name receipt
supplies no successful class/object construction or stored handle contents.

Registered-factory candidates have a separate purpose: selected Jim source
layouts use the shared counted command lookup comparison at the retained root,
without asserting the factory publication operation. C/Logical generic registered-factory candidates are restricted to the shared
ASCII authored-source geometry. Unsupported non-ASCII/empty/NUL candidates
yield no receiver advice; consumers preserve that refusal rather than infer an
actual publication API or use a reporting class label.

## Property option values and original member geometry

`DefinitionBodyGrammar::source_property_declarations_where` owns one shared
width-aware layout for names, options and script bodies. A selected option
consumes its following value once; that value is not rescanned as an option.
Following property names start separate declarations and repeated getter/setter
selectors retain the final script. Unknown script values preserve ordinal
selection; unknown selectors, invalid kinds and uncertain expansion refuse the
complete layout. `OriginalSourceDefinitionMemberRegion::script_bodies` retains
the full original argv and genuine parent carrier, then projects complete body
operands regardless of option quoting. Accessor scripts do not inherit
definition vocabulary. Source geometry grants no installed accessor or frame.
The [native option/value observations](../analysis/name-resolution-proofs/tcloo-property-option-value-boundary.md)
remain independent of dynamic, Unicode and opaque source controls.

## Shared project callback source diagnostics

Use `project_callback_diagnostics_for_analysis` with the complete current source,
actual retained AnalysisResult and current Project for supplied-analysis callers.
The tracked `project_callback_diagnostics` query delegates to the same owner.
`retained_callback_invocations` reissues the lower canonical source lookup before
augmentation, so copied invocation labels/counts or a foreign image/horizon
cannot donate advice. Known local barriers remain terminal. Exact held external
target slots and permitted unknown-target prefix lookups query current project
headers; legacy scalar/tail advice requires positive actual Logical input.

Server push publication, full pull and code actions share
`Backend::project_callback_diagnostics_if`. Keep feature state, source image and
header currency independent of typed subject/range transport. A readonly source
signature diagnostic cannot create an edit, registration, future callable,
actual argv or entered frame. The [project projection contract](../analysis/name-resolution-proofs/database-original-project-callback-projection.md)
records these source controls separately from native callback observations.

## Alias publication at the actual entry namespace

Runtime local and child installation use `Namespace::alias_publication_at`,
which delegates the selected NativeNameProtocol AliasPublication purpose. Pass
the actual original entry namespace: C relative-qualified alias names use its
namespace, plain names use the root publication purpose and explicit roots keep
their own geometry. Child publication uses its supported provider API and keeps
creation/query return conventions separate. Jim retained original spelling and
root-stripped command comparison remain distinct projections; equal public call
results cannot establish private key/cell or cross-API output equivalence. The
[current-holder](../analysis/name-resolution-proofs/alias-c-current-namespace-publication-holder.md)
and [child](../analysis/name-resolution-proofs/alias-original-child-interpreter-publication.md)
records retain exact original sources and provider rows.

## Original command source reporting extent

`CommandSpan::source_extent` owns the original terminator-delimited physical
extent, including the final closer and trailing trivia while excluding the
terminator and leading indentation. `NativeScriptCommandWords::source_extent`
retains that optional extent in the genuine whole image. Structural word spans
remain separate; an assembled word vector has no containing-source ownership.
Runtime interpreted parsing and the C body artifact reuse that source extent.
`CodegenCtx::native_command_source_span` selects it only under the actual C
source-log recipe; Jim and vector-only replay keep their independent represented
spans. Physical source geometry supplies no compiler entry, Native handler or
entered frame. The [native reporting record](../analysis/name-resolution-proofs/runtime-original-command-source-extent.md)
measures ASCII public excerpts independently of opaque implementation fixtures.
Read hidden C error globals through the public Read owner, such as `set ::errorInfo`; raw storage access cannot attest public observer state.

## Positive Logical analysis admission and actual suppression context

`Analyser::selected_logical_declaration_advice` selects the actual positive
Logical ResolvedAnalysisInput consistently at all five analysis entrypoints.
Keep captured Native entry, NativeRecipe and hosted/mixed-input refusal
independent of that source purpose. Plain Logical input can retain its authored
C8.6 name simulation without a Native name protocol; the simulation does not
disable compatibility or supply Native binding, frame or formal grammar.
Missing result input cannot be replaced by a copied flag or display dialect.
Plain tcl retains no formal parameter grammar until a dialect is independently
selected. Declaration navigation and reporting-advice consumers use this
positive domain without recapturing Native source ownership.

Undefined-variable alias-tail suppression passes its actual
UndefSuppressionSemantics.context into the shared original Statement invocation
owner. Use selected availability and known source barriers rather than the
command catalogue profile. Qualified/unqualified tail text is source metadata;
it establishes no executed alias link, variable cell, runtime contents or frame.

## Conditional procedure source suggestions

Use `source_structure::source_procedure_publications` and its `candidates` for
readonly final root procedure suggestions. Canonical original headers/body and
naming operands remain separate from conditional current slots, move lineage
and exact unknown-operation obligations. Known represented delete/replacement
withdraws the old suggestion; canonical names cannot restore it. Genuine
unrepresented namespace/deferred declarations remain cards with no publication
receipt. Local completion, workspace indexing/completion and package inventories
share this selector with full actual input/image/configuration/Registry context.
An independently completed command world remains separate. These candidates
grant no Native dispatch/frame, successful load, navigation or edit identity.
See the [procedure source contract](../analysis/name-resolution-proofs/source-original-procedure-publications.md).

## Source installer, optional callback scope and readonly properties

Retain `OriginalSourceCallbackRegistration` beside the prefix. It owns the actual
installer site, whole command or registered-method schema, factory ancestry and
applicability; captured operands keep their own original producer. Source target
finalisation consumes this receipt while future Native lookup stays independent.
`OriginalCallbackPrefix.scope` may be unavailable although layout/count advice
survives. Preserve UnavailableCallbackLookupFrame. Only applicable absolute C
source correlation is frame-independent; relative unknown and Jim purposes
cannot borrow a frame. Actual package availability/source scope creates no
blanket Tcllib registration frame.

Readonly property completion calls
`OriginalSourcePropertyMetadata::matches_original_declaration_source` with the
actual canonical provider. Complete Document input/configuration, genuine class
receipt and private original property-ledger membership must agree. Foreign
same-source providers and stale images refuse. Entered-definition-frame/edit
matching is independent of this readonly correspondence.

## Actual context for source actions and unknown-handler advice

Source actions resolve event/bootstrap/taint/helper metadata through actual
retained availability and matching command-store identity.
`SourceIrulesEventBody::event_word` owns the complete original event operand;
a first body statement cannot replace its header geometry. Missing operand,
image, configuration or Registry correspondence refuses without Native event
activation/TMM lifecycle authority.

Unknown-handler secondary lowering/recursive original Statement queries retain
actual ContextRegistry, lexical configuration and unit profile. Standalone
assistance requires positive actual Logical input; missing/Native/hosted ingress
cannot be replaced by display-profile or reporting Registry labels. Report flags
establish no executed unknown worker. See the [unknown-handler source contract](../analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md).

Called iRules bodies join genuine vendor declaration worker vectors to shared
complete body/content spans. ProcDef token endpoints/report maps cannot clip
those bodies; ProcedureBindingUnavailable and dormant-body exclusion remain.
Constructor source signatures separately require selected formal grammar:
plain authored-name simulation supplies no missing parameter grammar, whereas
explicit C9 controls select their own source topology.

## Immutable compiler proof reuse and Logical source storage

`SourceCompilerNameEffectsCache` shares completed descendant name-effect proofs
only at the same immutable source lookup point. Preserve exact source image,
channel/span/configuration, origin, namespace, Registry/realm and compiler
context; the point retains its original baseline/lookup/observer world. Detached
Clone/join resets this derived memo and its population is not semantic Eq/Hash.
Physical Native entry remains uncached because its currency can change outside
the source snapshot. Reused booleans grant no command execution, frame, inline
admission or Normal completion. Fixed parsed-region controls are independent
of elapsed-time validation. See the [proof reuse contract](../analysis/name-resolution-proofs/source-original-compiler-descendant-proof-reuse.md).

`logical_original_substitution_access` requires the actual positive Logical
entry and whole Authored Document arena/site/configuration correspondence,
with matching current source variable frame/namespace. It delegates existing
symbolic storage to `resolve_substitution_access`; chosen model values are not
executed provider stores. Observers/dynamic uncertainty remain open. Native
original roots/lexical compilation retain a separate path. Missing, Native or
stale ownership cannot acquire Logical fallback or physical read authority.
See the [Logical substitution contract](../analysis/name-resolution-proofs/logical-original-variable-substitution.md).

`AnalysisResult::allows_lexical_declaration_advice` itself requires actual
retained positive Logical input; its retained-report wrapper delegates. Copied
flags/profiles cannot reopen missing/Native/hosted admission. Variable completion
uses `None` for this explicitly admitted Scope compatibility only; unavailable
or malformed original geometry remains terminal. Workspace symbols require
current whole source context before original versus Logical presentation.

Native ensemble public outputs retain exact provider differences. The separate
pinned source appendix records NS_DYING/NS_DEAD entry checks and purpose-specific
parent/current routing; it does not expose tokens/flags from those executions.
`retained_logical_class_definer_grammar` selects source diagnostic family advice
only from actual positive Logical input, complete image/configuration/context
and observed source class/provider provenance. Stand-ins, defaults and stale
inputs refuse. This purpose supplies no Native class identity, formal grammar
or dispatch worker; Native/hosted canonical source selection remains separate.

Registry recipe and Runtime/VM source controls remain coverage links until
independent actual software receipts execute them.


## Current dispatch regions and selected procedure fallback

Core references, caller-frame scans, rename hazards and refactors obtain readonly
regions through `retained_dispatch_context`, `nested_dispatch_regions` and
`frame_shifted_dispatch_regions`. Keep the actual retained input, complete image,
lexer configuration, current Registry/availability and matching command realm.
Use the selected original source schema and script purpose for moved/captured
operands. `ReferenceOnly`, known source replacement/deletion and missing/stale
context return no region; consumers preserve that refusal. A potential source
body is not an actual entered frame, receiver identity or an editable reference.
See the [dispatch-region contract](../analysis/name-resolution-proofs/core-original-dispatch-region-context.md).

For Runtime procedure calls, `run_selected_procedure_body` keeps the actual
preselected C artifact or Jim original body when an AOT entry declines or is
absent. Procedure entry retains frame/formal ownership. Compiled activation leave
keeps the existing guest error episode for subsequent public error-variable Read;
that read must not start another evaluation. Manual ABI source logging retains
source error text independently of original provider instruction-labelled
errorstack. A matching source trace supplies no body/header/ABI/frame/compiler
admission. See the [selected fallback contract](../analysis/name-resolution-proofs/compiled-procedure-fallback-and-source-log.md).

## Selected source producers and current metadata

Command hook/symbol/body/variable/math/event/self-shape/manufacturer metadata
uses the actual retained ContextRegistry command store. Missing current context
cannot borrow a stashed default store. Already-selected schema roles are pure
Registry projections; handler selection and reached effects remain independent.
Package callback receipts also require actual selected package availability.

For list-built source assistance use `original_list_built_script_command` with
the complete actual input/realm and selected parent/builder/target schemas. It retains the sealed selected builder and its original producing spans. Captured alias values
cannot borrow written spans; dynamic/multiple heads, shadows and missing source
ownership refuse. Producing expressions are not substituted values or an entered
script. Pending constructor head advice retains its real variable/selector
geometry. Source-only Native/hosted class advice has no reporting-label fallback;
separately retained positive Logical advice follows canonical lexical candidates.
Missing input/copying flags cannot reopen that compatibility domain. Positioned
procedure compatibility uses the same positive Logical eligibility and canonical
qualified candidates, independently of genuine Native source-body ownership.

Native public source errors, counted variable same-object comparisons and the
strict formatter archive/image pair retain separate question records. Their
finite observations supply only their measured purpose; Runtime/VM/Core source
controls and executed validation receipts remain independent.


## Quiet operations in the retained Logical source model

Use the shared Logical `original_static_invocation_words` admission and
`logical_operation::transfer` for bounded symbolic operations. Retain positive
actual Logical input, whole original Document/vector, current image/configuration,
source frame/namespace, selected Registry target and actual availability. Static
captured alias arguments compose with genuine written values under that same
selected operation. The Proc-specific definition transfer remains independent.

The selected scalar store updates quiet symbolic contents only; dynamic
bindings/traces, observers, arrays, physical storage identities and missing/stale
ownership refuse. Command moves/deletions/alias transitions require bounded
selected effects, known source targets and admissible destinations/current
interpreter operands. These authored continuations retain no Native Normal,
entered frame, argv, compiler admission or actual store receipt. Preserve every
refusal instead of filling it from reporting state. See the
[Logical operation contract](../analysis/name-resolution-proofs/logical-original-operation-transfer.md).

Shallow loop and bounds metadata also consume the actual retained ContextRegistry.
A catalogue store does not donate metadata unavailable in the selected context.
The strict integer-formatter build-pair control keeps its complete independently
captured 812-row archive/image observations; a linked source control is separate
from an executed validation result or admission of another build.


## Original comments, member references and final removal guards

Analysis-aware comment cursor consumers use
`offset_in_physical_comment_in_analysis` and retained Compiler comment facts.
Keep the complete actual image/configuration/input. A second physical line inside
a quoted word can contain a live substitution despite a leading hash character;
only genuine physical comment facts classify that comment purpose. Absence from
the inventory does not classify all other inert data. Use the independent
original word/data purpose and preserve missing/stale ownership refusal.

For member-reference inventory obtain the genuine selected parent region, then
use its `original_commands` and `references` APIs. Whole-command membership and
actual availability select `source_member_ref_indices_in`; control words and
resident name values retain distinct purposes. Returned readonly operands are
source advice, not installed member identity or editable rename permission.
The [member-reference contract](../analysis/name-resolution-proofs/core-original-member-reference-hazard.md)
keeps those independent requirements explicit.

Optimiser finalisation consumes selected original CFG/binding/normal definition
operands and canonical place projections for the resurrection guard. Missing
source/store/configuration/binding withdraws removal; atomic groups remain whole.
Quoted names, literal dollars, scalar parentheses and captured alias operands
retain their selected values without a reporting-argv reconstruction. This
source removal gate does not grant Native rewriting or instruction/frame entry.
See the [finalisation guard](../analysis/name-resolution-proofs/optimiser-original-definition-resurrection-guard.md).

C command teardown order uses the pure selected release frontier recipe. The
[five pinned C source inspections](../analysis/name-resolution-proofs/namespace-command-teardown-frontier.md)
are separate from original public provider completion records. Neither recipe
nor inspected source text manufactures an actual namespace holder or command
token.


## Retained member hazards and child host refusals

For a class-configuration source target use `source_configured_class_at` and its
`OriginalSourceConfiguredClassReference`. Keep the whole selected configuration
consumer, genuine written/captured operand, canonical pretransition source class
and every consumer/factory obligation. Class-layer target ordinals must agree.
Callable aliases and object-layer configuration cannot select class identity;
known barriers, missing input, changed source and ambiguous Logical metadata
refuse. This source correspondence grants no current class object or successful
configuration. See the [class reference contract](../analysis/name-resolution-proofs/source-original-class-reference.md).

Core member hazards consume genuine parent/member-reference regions after full
source/input/realm and actual ContextRegistry checks. Preserve custom schemas,
selected wrapper/immediate caller vocabulary and moved/captured source targets.
Missing/stale/unowned metadata returns no hazard words. A matching display method
name filters authenticated source candidates; it cannot grant an installed member
or safe edit. Caller-frame reference projection retains that same current Registry
and source input rather than rebuilding a nominal scan.

Original child evaluation checks its retained host failure before capturing guest
completion. `transport_host_refusal_from` keeps reached typed admission/access
fields, preserves existing parent fields and leaves the parent result unchanged.
No guest options or cross-interpreter value/lookup identity is manufactured. The
[host transport contract](../analysis/name-resolution-proofs/interpreter-original-child-host-refusal-transport.md)
links an authored host-worker control separately from native child observations.

Dictionary conformance preserves all observed completion comparisons. The isolated
C8.5 write-error lifetime edge compares defined diagnostic/caller-read-status
fields independently of later freed-value contents. Whole public windows in other
rows are observations, not lifetime certification. The VM's safe live-header/member
policy has its own control and supplies no native freed-content identity.

Callback call-graph and opaque-caller consumers share the genuine positioned
source inventory and actual Lowerer metadata input. Keep a whole selected list
builder separate from literal list data and callback target identity. A retained
registration/builder supplies no executed value, future lookup or callback frame.

Compiled command currency uses `NativeCompilationGuard::for_native_dialect`
from the actual selected C engine. C8.4 retains `ChunkEntry`; C8.5–9.1 retain
`BeforeArguments`. Keep compiler and namespace resolver epochs independent.
For the latter guard, `revalidate_source(captured, current)` checks the actual
artifact stamp against current epochs before operand preparation.
Runtime `execute_body_instruction` obtains those epochs through
`native_compiler_cache_epochs` and re-enters the same original command source
when required. The captured selection remains fixed during its argv effects.
Missing current epochs refuse; neither a guard nor source text creates a
captured opcode or reached execution receipt. The
[currency contract](../analysis/name-resolution-proofs/command-native-command-entry-currency.md)
keeps Logical/Jim/hosted no-guard cases explicit. Controls distinguish a change
before operands from a replacement inside an operand.

Completed original Array declarations stay available through the shared selected
worker plan even when its dispatch is Generic. Declaration facts supply no inline
instruction or admission. Unchanged primitive projection keeps its actual
reached result/header/cache.

Jim alias failures retain their actual typed cause and original operand units.
The shared presenter does not reconstruct a rejection from text. Evaluated-command
logging also keeps its selected protocol and independent evaluator flag; original
public error reads preserve that existing episode. Whole-array teardown captures
old member registration IDs before root callbacks and retires only those IDs, so
new registrations on a replacement generation retain their own lifetime.

### Jim interpreter copies and child initialisation

`NativeJimInterpreterCopyPurpose` selects the original API's string extent.
`Object` retains all counted bytes for scripts, alias arguments and results.
`ChildStartupVariable` selects the CString extent used by
`JimInterpCopyVariable`: a runtime NUL ends the copied value. This purpose only
selects bytes; it grants no source or target interpreter ownership.

Runtime's shared Jim object bridge obtains both actual live contexts,
materialises the original string, creates a fresh target String and binds it
only to the target owner. Directly binding the parent object to the child still
refuses. Source/Script/List headers, filename objects, cache children and their
interpreter allocations do not cross with the string. Child eval uses Jim's
Concat even for one argument; alias invocation duplicates the retained
parent-owned prefix List, appends copied child arguments and evaluates that
actual parent List. Results copy back, with Jim's raw completion code kept
separate from C return-option transport.

`NativeBootstrapProtocol::child_initialisation` supplies the selected child
recipe. Jim's actual factory calls its static-extension initializer and copies
exactly `argv`, `argc`, `argv0`, `jim::argv0`, `jim::exe` and `jim::lineedit`
from the parent global home. Procedure locals cannot replace those operands;
absent globals stay absent. C child initialization has its independent recipe
and supplies no Jim variable-copy plan.

The Runtime child installs its supported Jim distribution subset. Namespace,
package, interp and array are present through its supported core installers;
`cmd_binary::install` selects Jim's real scripted binary ingress and compound
helpers. A static initializer named binary cannot justify installing the C
binary ensemble. The pinned native distribution also reports pack, unpack and
class in the bounded presence probe; those observations do not establish
backend handlers or complete static-roster parity.

Child construction checks actual parent and child host refusals before
publishing a child allocation or command. Parent typed state remains
independent of guest results; genuine child causes use
`transport_host_refusal_from`. Jim and C construction consumers return a
retained constructor failure before writing a successful handle. A refused getter's `None` is not
an absent-global observation. The constructor-worker controls cover retired
Jim ownership, missing observed frame/purpose and actual nested C owner
transport; they are host API assertions, not observed native process failures.

Keep the [counted crossing proof](../analysis/name-resolution-proofs/jim-original-object-crossing.md),
[child presence and source/API contract](../analysis/name-resolution-proofs/interpreter-original-child-bootstrap-context.md)
and [startup copy extent proof](../analysis/name-resolution-proofs/jim-original-child-variable-copy-extent.md)
separate. The public native rows prove their exact queried values, completion
codes and presence, while source anchors describe the original API's ownership
and copy order. Equal output cannot prove private object identity or admit a
foreign context.

## Explicit native scripted-library entry

`native_scripted_distribution::procedures(dialect, library)` owns the audited
source roster selected by `NativeScriptedLibrary::ALL`, including the namespace-info dependency.
It retains original command, formal and body source. Selecting that roster
establishes no installed command, current binding, compiler hook or entered
frame. Native core construction and explicit scripted-library installation are
separate entry purposes; a test for a loaded library must select that library.

Runtime bootstrap uses `cmd_proc::install_stock_scripted_library`; VM bootstrap
uses `install_scripted_library`. For example, an internal VM distribution
fixture loads the namespace library explicitly:

```rust
vm.install_scripted_library(
    tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceEnsemble,
);
```

The installers use real procedure registration and preserve existing host/user
bindings. Calls still resolve the current mutable command table. Jim namespace
routing uses `InvocationDialect::native_namespace_scripted_helper` over the
original selector's selected native extent, then ordinary lookup through
Runtime `dispatch` or VM `invoke_command_value_at`. Do not replace a renamed,
deleted or overridden helper with a catalogue body, or donate C ensemble
configuration semantics from its spelling. Core absence, explicit library
presence and subsequent replacement need separate controls.

## Fallible in-place list append

`ValueOps::try_list_append_in_place` returns `Result<bool, ValueError>`.
`Ok(true)` means the selected physical owner completed the append. `Ok(false)`
permits the shared caller's ordinary copy-on-write path; the default VM
capability returns that value. `Err` retains conversion/storage or host
unavailability and must propagate before any fresh-list reconstruction.

Use `tcl_cmd_core::var::lappend_value` for the shared value computation:

```rust
let updated = tcl_cmd_core::var::lappend_value(ops, original, operands)?;
```

That owner retains completed in-place operands and rebuilds only after
`Ok(false)`. Runtime's capability checks both original objects' liveness,
selects the actual string/list protocol and delegates to
`list::append_prepared_native_elements`. Jim also requires its actual object
context. Preserve authentic List storage/capacity metadata for the next native
mutation; equal member bytes cannot restore missing allocation authority.
Never turn a typed refusal into `false`, swallow it with `unwrap_or`, or retry
through the untyped list mutator. The positive storage-currency control appends
again through the native owner; its unavailable-storage control requires an
error with the original object and members unchanged. These are implementation
controls, separate from the original native provider observations.

## Literal variable diagnostics and reviewed source proposals

A resolved variable name is literal data. Use
`tcl_syntax::naming::split_array_name_braced(name, true)` or the corresponding
retained-style owner to distinguish a complete `root(index)` from a scalar
whose name contains an opening parenthesis. A literal leading dollar stays in
the name. Use the sigil-aware reference parser only for actual source variable
references; read spans come from the original source and retained lexer
configuration, including the complete braced wrapper.

Possible object results with an owning Module use
`registry_invocation::retained_module_metadata_context(registry, module)`.
That shared adapter applies `InvocationMetadataContext::for_module` before
joining availability to the command store: the Module's unit profile, complete
lexer policy and retained input must agree. The input-only
`retained_source_metadata_context` adapter serves requests with no owning
Module. Consumers then call `advisory_value_assignments_in_context` and
`advisory_return_values_in_context`. The analyser independently verifies the
exact source image and its correspondence with `Module.source_metadata_input`.
Missing, foreign or mismatched context supplies no factory advice. A possible
result remains a possible result; it cannot donate a completed store, live
object, current class or entered command.

The regexp/scan no-match diagnostic selects its layout from genuine original
Registry words and typed conditional output operands. Captured alias arguments
share that same effective layout. Its bounded literal-result check cannot
select a command by spelling. A no-match preserves an earlier output value:
SSA entry versions, intervening stores and branch dominance must independently
establish the value at the later read. Logical source advice requires a
positively retained Logical input. Native diagnostics additionally require the
original read-presence alternatives to establish possible undefinedness; the
matcher metadata supplies no current cell or completed getter.

For W216 source proposals, use
`tcl_syntax::backslash::literal_quoted_source_fragment` for literal root
characters and `substituting_quoted_source_fragment` for the separately retained
index template. The latter uses complete shared lexical configuration,
retains the original variable/command/expression source components and checks
that projection preserves their parsed meaning. Incomplete or malformed
components supply no proposal. `native_literal_source_word` shares the same
literal quoting implementation but still requires its independent native
string and source-channel roundtrip.

A fallback `[::set "root(index)"]` proposal requires the inserted `::set` to
retain its stock variable-reading meaning. This unwritten command has no
original invocation receipt: the diagnostic and fix description state that
premise, and the fix remains `RequiresReview`. A root containing a literal
opening parenthesis has no such combined-name array projection. Neither
successful lexical rendering nor a matching public native example grants
safe-edit permission, live variable identity or index evaluation.

The [resolved-name advice](../analysis/name-resolution-proofs/diagnostic-original-resolved-variable-name-advice.md),
[matcher source contract](../analysis/name-resolution-proofs/diagnostic-original-matcher-output-retention.md)
and [W216 source projection](../analysis/name-resolution-proofs/diagnostic-original-w216-literal-source-replacement.md)
records retain their precise questions and consumer bounds. Native public
value observations are independent records; their source channel, exact input
and provider coverage cannot be inferred from a source/API control.

## Selected scalar reference emission and Jim inventory

`optimiser::helpers::expr_simplify::substitute_expr_constants_in_context`
consumes complete `ExprParseContext` axes. The shared expression lexer and
`tcl_syntax::naming::variable_reference_root_bytes` use the same selected
grammar and original token source span. A scalar name such as `scalar(open`
stays whole. The scalar constant map supplies no selected array-element key:
separate indexed references and braced combined-element names therefore
retain their original source. A legacy profile wrapper delegates to the same
emitter; Jim uses its retained context without a catalogue profile. This is
source projection only. `substitute_expr_constants_for_execution` still
requires independently matching native operand proofs and original checked
evaluation before emitting any replacement.

For native Jim inventory, retain `NativeJimInfoScope` from the original
dispatch. Only the original exact `-nons` option selects `DirectCore`; rooting
an `::info` head keeps namespace-aware forwarding. Pass original member
operand count and first operand bytes, together with the actual retained frame
namespace, to `command_inventory`. `NamespaceHelper` invokes the current
single-name `namespace info` command with the original selector and operands;
its independent grammar receives `-all` unchanged. `Flat` enumeration uses
`jim_core_command_list` and `jim_core_command_names` with the actual table.

A retained `NativeJimCommandTableKey` keeps `report_bytes()` separate from
`comparison_bytes()`. Same-slot replacement uses `retain_for_replacement`;
rename and fresh publication select their own name purposes. Neither reported
spelling nor matching bytes supplies a private original object or generation.
Distribution bootstrap consumes the common `NativeScriptedLibrary::ALL`
roster. Core-only entry and explicit single-library fixtures remain separate
purposes. The [native scripted-library consumer guide](../../../rust/tcl-registry/src/native_scripts/README.md)
provides the current installer and forwarding APIs and links their independent
public observations. The [scalar source-emitter contract](../analysis/name-resolution-proofs/expression-scalar-reference-constant-source-projection.md)
records its exact parsing and constant-key bounds.

## Function metadata and conditional write footprints

Use `FunctionUnit::invocation_metadata_context(registry)` for a function's
complete retained metadata, and `FunctionUnit::source_lexer_config()` for its
exact source grammar. The supplied Registry must match the input's immutable
command generation. A missing or foreign input is unavailable; a catalogue
profile cannot replace it. A function used with its Module calls
`invocation_metadata_context_for_module(registry, module)`, which checks both
producers against the same actual input, availability generation and profile.
The Module's retained input must travel through secondary body builders and
function-lattice requests. Module-level metadata joins use
`InvocationMetadataContext::for_module`; `retained_module_metadata_context`
returns its checked ContextRegistry for internal consumers that need the owned
availability handle. Keep `for_source_input` for requests that have their full
input and exact source configuration but no Module.

Readonly compiler field reviews use
`durable_inventory::{FUNCTION_UNIT_FIELDS, COMPILATION_UNIT_FIELDS}` and
`assert_durable_field_inventory(unit)`. The owner declares each field roster
beside its exhaustive no-`..` witness, including private retained input and
configuration fields. Explorer checks that every field has one current review
and that no obsolete row survives. Reviewers use the existing checked unit
accessors to consume metadata; a field-name string exposes no private value and
grants no authority to replace it, select availability or admit execution.

```rust
fn function_metadata<'a>(
    function: &'a tcl_compiler::compilation_unit::FunctionUnit,
    registry: &tcl_registry::CommandRegistry,
    module: &tcl_compiler::ir::Module,
) -> Option<tcl_compiler::registry_invocation::InvocationMetadataContext<'a>> {
    function.invocation_metadata_context_for_module(registry, module)
}
```

Global-write summaries select their metadata through `for_module` and retain
its exact lexer configuration and shared original command bindings. Structural IR
destinations use `structural_variable_write_projection`; generic calls use
the retained selected invocation and Registry operand roles. Original alias
captures remain before the written arguments. `uplevel`/`upvar` body consumers
share `script_value_write_projection_with_metadata_context` with the selected
namespace, actual metadata and exact configuration. Unknown body, frame level,
argument role or context contributes an opaque footprint rather than an empty
write set. Embedded expression calls also participate in the source procedure
summary closure. These are conditional source footprints, not measured writes
or selected Native frames and cells.

Diagnostic definition places and existence guards take the same function
metadata. Missing/foreign input, refused availability and a mismatched source
grammar withdraw advice. Existence-query operands describe a conditional guard;
they cannot establish the current query result, physical storage contents or
permission to remove an original read. See the [metadata contract](../analysis/name-resolution-proofs/original-analysis-metadata-context.md)
and [existence metadata contract](../analysis/name-resolution-proofs/retained-existence-metadata.md).

Read-before-set suppression for a concatenating script retains the genuine
installer's whole source vector, original lookup state and namespace, exact
source image and complete metadata input. The materialized text is scanned by
`script_value_possible_writes_with_metadata_context`; it receives possible
names and an opaque residual, without an authored child word, source span,
Native evaluation entry or current write. Select immediate bodies from the
already selected Registry descriptor, including its `BodyKind::Plain`, current
interpreter and same-invocation timing. A scoped descriptor keeps its own
selection; querying the global catalogue by the displayed command name cannot
replace it.

Expression suppression starts with the original Expr operand content base.
`OriginalRegistryWords::source_expression_script_bodies_at` selects its unique
complete source operand, including a particular operand among multiple Expr
roles. Each original bracketed command then uses its own retained lookup
position. The outer assignment's post-substitution state cannot replace that
position: an earlier `[set x value]` stays independent of a later rename in the
same expression. Captured aliases retain their declaration values and held
target names. Known replacements and unavailable descriptors supply no borrowed
stock writes. CFG condition footprints use the same retained context and
literal-name owner; they remain analysis facts. The [materialized footprint
contract](../analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md)
records the bounded source/API controls independently of provider execution.

## Conditional reads and substitution purity

Hidden-read consumers use the original CFG carrier at each actual block and
statement index through `Function::source_tokens_at`. A consumed typed statement
has the same source carrier requirement as an ordinary call. Each child in
`CommandTokens::nested_bindings` supplies its own `original_recorded_command`,
lookup state, source origin and grammar. The outer command's post-substitution
world cannot stand in for the child's earlier lookup horizon. Known replacement,
a held alias target moved away, refused availability or conflicting original
observations must remain visible.

`OriginalSourceMaterializedFootprint::invocation_reads` combines conditional
by-name roles with selected expression and immediate-body reads.
`reevaluated_reads` selects only those expression and body reads, for a parent
whose ordinary variable operands are already represented. Alias-captured
operands retain their original values and origins. Materialized text supplies
possible names and an opaque residual; it supplies no authored child word or
span, Native entry, physical read, represented SSA use or normal completion.
Lexical interpolation uses `VarReferenceScanner::with_config` with the retained
source configuration, preserving protected braces and literal variable-name
syntax independently of selected command roles. Missing or foreign metadata is
opaque. Elimination uses opaque coverage to retain possible stores; expanding
that coverage to existing symbolic SSA names does not create read occurrences
or SSA versions.

Body-local diagnostic suppression uses
`OriginalSourceMaterializedFootprint::body_name_ownership` with the genuine
original whole body. It retains the installer's source lookup and checks the
body's full source, selected metadata and interpreter scope. The shared
materialized engine separates by-name input operands from ordinary
interpolation: `set name` may supply conditional local ownership, while
`puts $name` supplies no binding. Literal operand names preserve a leading `$`
and an unmatched final parenthesis. Known alias targets, captured operands and
replacement barriers remain at the installer's original horizon. A named child
requires its independently retained command world; source geometry cannot
substitute the parent's table. Missing or derived body ownership refuses this
projection. Its names suppress a source warning only; they prove neither an
entered child nor a completed store.

Existential procedure-read advice selects
`SourceCommandBindings::original_procedure_read_declaration` from the complete
original declaration observation and actual supplied input. The Native branch
retains the original `OriginalFormalTopology` and independently selected native
parameter grammar; the Logical branch retains its separate sealed
`LogicalProcedureDeclaration`. Re-query selected descriptors with the complete
availability context. A displayed `proc` head, a procedure label or a catalogue
profile cannot supply that header. Renamed factory calls and captured formal
operands keep their original producers. Verify the original body image, offset,
formal value and parameter correspondence before delimiting diagnostic reads.
This purpose requires no successful namespace publication or body activation,
and supplies no Native formal installation, frame or current read.

Substitution purity uses the same original child receipts and the function's
Module-aware metadata. The Native branch requires closed execution alternatives
and the independently proved normal handler. The Logical branch uses
`original_logical_operation_invocation_with_metadata_context`, which validates
the exact retained Logical input and resolves original alias captures and the
selected subcommand. Use `OriginalCompilationLookupAdvice::closed_logical_source_lookup` only for
the positively retained authored namespace and original source targets. Its
closure is distinct from `closed_lookup` and grants no physical variable frame.
Catalogue traits attached to a displayed head are
insufficient. Conditional procedure summaries additionally join the original
implementation allocation to one matching declaration source site; a procedure
name is a summary key only after that join. Receiver-method labels do not supply
a method receipt.

A literal with no evaluation can remain pure without a command query. An
expression `Command` node already identifies evaluated script syntax even when
its text lacks bracket delimiters; it consumes original receipts directly.
Purity describes the selected implementation's effect contract, separately from
whether evaluating its operands can raise, whether traces or external accesses
observe the store, and whether the original source is editable. DSE, ADCE and
constant-store coupling keep those independent error, observer and source-edit
gates. Tests distinguish each dependency using aliases, held and moved targets,
replacement, availability, exact lexer configuration, and missing or foreign
Module/function input. The [metadata proof contract](../analysis/name-resolution-proofs/original-analysis-metadata-context.md)
records the bounded source controls separately from native process observations.

## Typed write-chain source proposals

O104/O130 select variable operations through actual retained invocation facts,
including the Registry's typed lowering and analyser hooks. Original alias
captures and moved commands keep their selected static operands. A frozen
value obtained by a substitution cannot replace an original static operand:
dropping the substitution could remove an observer. Source roles, accepted
arity, exact grammar and the same supplied generation remain required.

Literal variable names use the closed `split_element_ref` owner. A leading
`$` or unmatched final parenthesis remains part of a scalar name. Complete
array operands require an independent index/cell proof and do not acquire one
from the scalar chain recipe. Shared literal source renderers quote the exact
selected name; reference-sigil normalisation is not a destination-name parser.
A traced or aliased interleaved write ends the chain because its callback can
observe the accumulator.

The proposed rooted setter is derived from the selected setter descriptor.
Its independent whole-unit command-mutation guard remains required because
the unwritten head has no original invocation receipt. Genuine retained
Logical compatibility can emit grouped source rewrites. A Native handler
layout supplies a hint only: source pattern, static values and quiet source
hazards do not certify Native intermediate-store erasure or result-object
equivalence. `hint_only` prevents those proposals from being applied.

## Original selector and procedure usage rendering

`InvocationDialect::native_info_original_option_protocol` selects the
original C8.4 legacy option door. The independently selected
`native_info_original_missing_selector_usage` supplies each supported C
provider's missing-selector usage suffix. Runtime and VM consume these shared
purpose queries; Jim retains its separate `NativeJimInfoProtocol`. Version
labels and quoted display names do not select another provider's grammar.
See the [original selector dispatch observations](../analysis/name-resolution-proofs/info-original-missing-and-empty-selector-dispatch.md).

Procedure usage names and messages use
`NativeUsageProtocol::render_procedure_name` and `render_procedure_message`.
The original called-name bytes, selected header bytes and formal usage suffix
remain separate inputs. Jim's called-name rendering preserves the original
raw bytes, while its assembled formatter result has the independently selected
CString extent. Rendering does not establish command lookup, original object
identity, a live procedure header or interpreter entry.

## Declaration arguments and retained frame slots

Use `OriginalDeclaredProcedureArgumentSlots` for the argument positions of a
newly compiled original procedure. Its shared issuer requires `WholeModule`
scope, the actual declaration image and body, complete original formal topology,
fixed scalar arguments, and a selected compiled-variable protocol supporting
`DeclareProcedure`. Counted names and compiler first-primary comparison select
the proposed argument ordinal. Defaults, rest arguments, caller links,
collisions, a missing protocol or a replaced source producer withdraw it.

`MaterialisableSlotAuthority::SourceFormal` retains this declaration receipt
and the exact SSA activation cell. Every represented use must match the
original topology and activation, with independently closed observers. A
parent Native compilation entry does not turn the new declaration's arguments
into slots borrowed from that parent's frame.

`CommonAotProofPlan::declared_arguments()` keeps declaration argument decisions
separate from `materialisable_slots()`. A `DeclaredArgumentIdentity` selects the
canonical original procedure and counted argument ordinal. Its evidence retains
the declaration receipt, independently selected singleton shape, and every exact
`SsaIncomingSlotRead`. Distinct reached activations keep their physical cell
identities; they need not share a scalar SSA value. Missing owners, changed
contents, observers, unresolved caller types or conflicting formal topology
withdraw the decision.

Use `declared_argument_for_incoming_read` to join a typed incoming operand to
this evidence. The query compares the complete source read, counted slot and
physical alternatives under the canonical declaration. The WASM operand adapter
also requires the direct call's complete original formal receipt to agree.
This permits a compiler allocated argument position without inventing a version
zero SSA value or borrowing the root frame. It does not erase operand coercion,
original object sharing, body admission, overflow, caller evaluation or the
required trace and interpreter policy guards.

Use `OriginalNativeFrameSlot` for an existing root frame. It requires the
retained layout owner and incarnation, actual compiled primary and ordinal,
selected procedure activation, namespace identity and independent entry naming
policy. Every represented use must retain those owners. A source declaration,
escape label or calling-convention ordinal cannot supply this receipt.

`SealedProgramCell` describes independently owned program storage proved by
closed-program statement coverage; it cannot replace the frame contract of a
supplied Native entry. None of these allocation purposes discharges retained
Native body admission, current contents, trace or alias absence, numeric
representation, or runtime observer guards. The [slot-purpose contract](../analysis/name-resolution-proofs/aot-original-slot-purpose.md)
keeps declaration positions, authored storage and retained frames separate.
