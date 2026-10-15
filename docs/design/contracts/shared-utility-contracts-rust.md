# Shared-utility contracts (Rust workspace)

The low-level logic every crate must share rather than reimplement, and why.
Equivalent low-level logic (namespace-name splitting, number parsing,
unique-prefix option matching, canonical error texts, backslash decoding, list
codec) belongs to its shared owner. Local copies risk subtle, user-visible drift: a naive
`rsplit("::")` mishandles colon *runs* (`namespace tail foo:::` must be
`""`, and `foo:::bar` must dispatch `foo::bar`), a
hand-rolled integer parser accepts `--5` or misses the 9.0 `0d`/`_`
forms, and a local prefix matcher resolves `""` against a one-entry
table where `Tcl_GetIndexFromObj` errors.

## Operational context

The Rust workspace splits shared logic by dependency altitude: the
grammar crates (`tcl-lexer`, `tcl-syntax`) at the bottom, the portable
command cores (`tcl-cmd-core`) above them, and the two runtimes
(`rust/tcl-vm`, `runtime/rust`), the compiler, and the LSP server as
consumers. Each utility below has exactly **one** owner; every other
crate calls it, wraps it, or (for `&str`/byte-slice duality) adapts it —
never re-derives it.

## Owners

The machine-checked manifest below is the canonical source-to-owner map. An
owner row names the source files that implement the owner, the public entry
points consumers use, the semantic axis that must be threaded, and the drift
gate (if one exists). `cargo xtask owner-resolution` fails when a listed file,
entry point, or gate moves without this contract being updated.
The retained naming-proof rows use `check-name-resolution-proofs`: the gate
requires its exact full Python checker recipe and live script. Structural-only
audits and inventory-writing modes do not satisfy that route.

<!-- owner-resolution-manifest -->
| Surface | Owner source paths | Checked entry points | Dialect/release axis | Drift gate |
| --- | --- | --- | --- | --- |
| Debugger current-frame variable reporting | `rust/tcl-vm/src/interp.rs`; `rust/tcl-debugger/src/backend.rs`; `rust/tcl-vm/src/debug.rs` | `Vm::debug_snapshot`; `DebugBackend::evaluate`; `DebugVar` | actual current frame local bindings and arena VarIds select scalar/array state before native name bytes are rendered; undefined and linked bindings retain their existing exclusion; arrays retain empty root values; valid UTF-8 stays literal and opaque bytes use full existing escape rendering; captured labels carry reporting purpose only and replay requires one matching label, declining collisions without dropping displayed cards; this grants no native variable-name acceptance, new frame selection or live evaluation | `xtask-owner-resolution` |
| Logical declaration source views | `rust/tcl-lsp-core/src/workspace_index.rs`; `rust/tcl-lsp-server/src/lib.rs` | `WorkspaceIndex::logical_declaration_source_names_at` | complete current Logical source/input/configuration and actual document URI select original declaration occurrences across retained indexed views; actual typed publication slots retain repeated sites and absolute names; optional checked spellings report the selected slots, and missing/stale/unrepresented views decline without constructing seed labels; no Native loading, dispatch or edit authority follows | `xtask-owner-resolution` |
| Current Logical member-provider correspondence | `rust/tcl-lsp-core/src/call_hierarchy.rs`; `rust/tcl-lsp-core/src/workspace_index.rs`; `rust/tcl-lsp-server/src/lib.rs` | `LogicalCallTarget::capture`; `WorkspaceIndex::logical_member_provider_matches_target`; `LogicalCallTarget::matches_indexed_member_provider` | positive Logical full-source input, current image/configuration/registry, original class occurrence and original member declaration/side are retained together; actual indexed provider ownership and declaring URI are required; changed source, detached copied provider, ambiguous source publication or missing member owner declines; class display labels do not establish provider identity and this issues no Native dispatch or edit authority | `xtask-owner-resolution` |
| Logical workspace command candidates | `rust/tcl-compiler/src/command_binding/source_transition_advice/logical_navigation.rs`; `rust/tcl-compiler/src/registry_invocation/source_structure.rs`; `rust/tcl-compiler/src/analyser/types/logical_procedures.rs`; `rust/tcl-compiler/src/analyser/types/logical_members.rs`; `rust/tcl-lsp-core/src/call_hierarchy.rs`; `rust/tcl-lsp-core/src/workspace_index.rs`; `rust/tcl-lsp-server/src/lib.rs`; `rust/tcl-lsp-db/src/lib.rs` | `source_logical_command_candidates_at`; `OriginalLogicalCommandCandidates::coordinates`; `LogicalSourceProcedureDeclaration::source_candidate_coordinate`; `LogicalSourceClassDeclaration::source_candidate_coordinate`; `WorkspaceIndex::logical_command_candidates`; `WorkspaceIndex::logical_command_candidate_is_current`; `OriginalLogicalCommandCandidates::original_head`; `OriginalLogicalCommandCandidates::argument_count`; `OriginalLogicalCommandCandidates::has_lookup_operand_effects` | complete current Logical input/image/configuration, exact original command vector and independently retained source-body namespace/path/import coordinates issue per-occurrence candidate geometry; original selected procedure Name/scope and class factory Name/body joins issue target coordinates independently of report labels; the class factory coordinate retains its own original descriptor/name/body/current-ClassDef correspondence independently of the separate Module-issued class-body syntax facet, and cannot donate that body word when the Module proof is unavailable; first matching coordinate preserves all duplicates across procedures and classes, unique navigation selection remains conditional, unknown paths or unsupported scopes refuse, earlier unknown mutations retain applicability, and current declaring revision plus exact source bytes must survive range conversion; no Native lookup, installation, body entry, closure, dispatch or edit authority follows; exact original head offset selects the already retained immutable whole-vector tape under matching full input, image, tape map key and scope; source suffix reparsing across nested closers cannot substitute another command, and this coordinate grants no frame, inventory absence or lookup authority; genuine Document-root source recipes remain distinct from retained declared-body recipes and unavailable method namespaces; original head and syntactic argument count provide source geometry only, expansion leaves cardinality unknown, and operand effects before lookup remain a separate applicability obligation; DB direct-call advice demands each actual typed slot independently and preserves first occupied empty class/nonprocedure cutoff, refusing earlier unknown mutation or lookup-operand effects before strict arity or suppression; a present original Native source owner stays terminal; the positive retained Logical domain selects its source-header path even when it carries native-shaped name geometry; only an independently selected non-Logical original Native lookup can make the Native branch terminal, and geometry alone cannot switch source purpose | `xtask-owner-resolution` |
| Native rename diagnostic operand reporting | `rust/tcl-syntax/src/naming/native.rs`; `runtime/rust/src/cmd_alias.rs`; `rust/tcl-vm/src/interp/native_rename.rs`; `rust/tcl-vm/src/command.rs` | `report_native_name_bytes`; `NativeNameReportPurpose::RenameCommandError`; `NativeNameProtocol::rename_diagnostic`; `NativeRenameFailure`; `NativeRenameDiagnostic` | audited original C8.4/8.5/8.6/9.0/9.1 retains counted original operand bytes for missing-source and occupied-destination messages; original latest Jim formats its selected operand through CString reporting; shared failure-stage reporting selects no explicit tuple for C8.4/8.5 and Jim, exact TCL LOOKUP COMMAND tuples for later-C missing rename/delete and TCL OPERATION RENAME TARGET_EXISTS for occupied destinations; adapters retain their neutral-error path when no tuple is selected; lookup operands, command generations and retirement/check ordering remain independent | `xtask-owner-resolution` |
| Original hidden-command invocation options | `rust/tcl-syntax/src/naming/hidden_invocation.rs`; `runtime/rust/src/cmd_alias.rs`; `runtime/rust/src/interp/namespace_address.rs`; `rust/tcl-vm/src/command.rs`; `runtime/rust/src/interp/native_hidden_invocation.rs`; `rust/tcl-vm/src/interp.rs` | `NativeNameProtocol::hidden_invocation_grammar`; `NativeHiddenInvocationGrammar`; `NativeHiddenInvocationGrammar::names`; `NativeHiddenInvocationGrammar::byte_names`; `NativeHiddenInvocationGrammar::option`; `NativeHiddenInvocationOption`; `NativeHiddenInvocationGrammar::uses_global_variable_frame` | actual retained C release selects its original native-index table and ordinal meanings: C8.4 has -global/--, later C adds -namespace, Jim refuses; original argument getters, abbreviation/index caches and safe-interpreter checks remain independent; Runtime namespace creation uses the selected native namespace address from the actual GLOBAL token, C8.4 selects the existing global variable frame, later C pushes a non-procedure namespace frame for an explicit context; Runtime and VM retain their actual frame selectors, original target and mandatory cleanup independently of this pure policy; grammar grants no lookup, target command, frame or execution authority | `xtask-owner-resolution` |
| Native command visibility diagnostics | `rust/tcl-syntax/src/naming/native.rs`; `runtime/rust/src/cmd_alias.rs`; `runtime/rust/src/interp.rs`; `rust/tcl-vm/src/interp.rs` | `NativeNameProtocol::command_visibility_diagnostic`; `NativeCommandVisibilityFailure`; `NativeCommandVisibilityDiagnostic` | actual retained C naming protocol and independently selected hide/expose/invokehidden failure; names retain the selected C-string report extent; C8.4 uses its original expose wording and neutral codes, C8.5 retains delegated missing-visible-command lookup codes while other refusals use the neutral adapter, C8.6–9.1 use exact structured tuples; Jim refuses this unavailable C purpose; reporting preserves original lookup, retirement and observer order and cannot select command slots or issue Native authority | `xtask-owner-resolution` |
| MCP hook-context source names | `rust/tcl-mcp/src/spectcl.rs`; `rust/tcl-lexer/src/word_parts.rs` | `spectcl_check`; `scan_var_ref` | actual requested target profile supplies the full LexerConfig for source-only variable-reference extents; complete bare/braced names and array roots are read through the shared lexer, never private name-byte boundaries; literal dict keys are bounded textual advice and opaque uses remain unattributed; the hook inputs declaration independently determines cacheability, and this report grants no executed hook, cache acceptance, Native binding or writable identity | `xtask-owner-resolution` |
| Contextual catalogue case advice | `rust/tcl-registry/src/model/context.rs`; `rust/tcl-lsp-server/src/lib.rs` | `ResolvedContext::resolve_spec_case_advice` | literal catalogue query and actual command store under the selected full availability context; exact admitted lookup wins, unique case-folded admitted descriptor supplies advice, and distinct descriptor ambiguity declines; spaces and empty names remain data in the shared query; descriptor metadata is returned directly without reconstructing identity from its display label; UI advice supplies no Native binding, dispatch, presence, runtime case policy or edit grant | `xtask-owner-resolution` |
| F5 XC original source descriptor joins | `rust/f5-xc/src/translator.rs`; `rust/tcl-compiler/src/source_graph.rs`; `rust/tcl-compiler/src/registry_invocation/source_structure.rs`; `rust/tcl-registry/src/registry.rs` | `registry_words_for_original_call`; `registry_words_for_original_substitution`; `source_registry_words_at`; `OriginalRegistryWords::with_source_schema`; `spec_pack_of` | actual source/analysis/Module retains the complete supplied Registry store, configuration, input and original source bindings; actual IfClause condition_base plus ExprOffsets and Switch subject span/braced facts supply original operands; existing sole-child and source occurrence owners select canonical descriptors and static operands, including separately selected xc_translatable availability; actual selected descriptor pointer provenance must belong to the shipped tcl or irules source pack, authored overrides cannot borrow XC templates even with a true or absent flag, and explicit false declines; missing/stale/foreign/malformed or literal-only ownership declines advice rather than reconstructing coordinates or borrowing shipped rows; display labels cannot select a getter/action, and conditional source metadata supplies no F5/TMM Native lookup, successful completion or effect equivalence | `xtask-owner-resolution` |
| F5 XC source getter geometry | `rust/f5-xc/src/translator.rs`; `rust/tcl-lexer/src/native_script_words.rs`; `rust/tcl-compiler/src/registry_invocation/source_structure.rs`; `rust/tcl-syntax/src/word_rules.rs` | `native_script_words_in`; `original_single_source_command_substitution`; `original_static_word_unicode_value` | original complete words under the translator's independently selected f5-irules grammar; static Unicode command words and sole executable child commands retain source image/configuration; expanded or dynamic heads, malformed tails, multiple commands, concatenating suffixes and braced literal brackets decline scalar getter advice; class-match criteria read the actual getter operand rather than a substring; authored translation advice supplies no F5 runtime lookup, TMM availability or Native dispatch, and requires separate current source/descriptor applicability | `xtask-owner-resolution` |
| Authored F5 configuration object paths | `rust/tcl-bigip/src/object_names.rs` | `ObjectNamePolicy`; `ObjectNameCandidates`; `object_name_candidates`; `ObjectNameCandidates::first_in_order`; `ObjectNameCandidates::first_exact_in_order_or_unique`; `ObjectNameCandidates::unique`; `owner_partition_reference_paths`; `reference_without_numeric_service`; `EmptyServiceSuffix` | literal authored slash-domain paths; caller-filtered actual configuration/module/type and input order; explicit exact, partition/Common, suffix and leaf policies; ordered selections retain duplicate exact rows without unique identity, unique fallback declines ambiguity; service suffix grammar remains purpose-specific; independent of Tcl namespace paths, C/Jim lookup and unmeasured BIG-IP execution | `xtask-owner-resolution` |
| Original linked-member source ownership | `rust/tcl-compiler/src/analyser/types/original_linked_member.rs`; `rust/tcl-syntax/src/naming.rs`; `rust/tcl-lsp-core/src/definition.rs`; `rust/tcl-lsp-core/src/hover.rs`; `rust/tcl-lsp-core/src/references.rs`; `rust/tcl-lsp-core/src/rename.rs` | `OriginalSourceLinkedMember`; `original_source_linked_members_for_class`; `original_source_linked_member_at`; `original_source_linked_member_reference_spans`; `OriginalSourceLinkedMember::editable_target_span` | authentic original class/member/body/side and full source input; selected original TCLOO_BINDS_METHOD_ALIAS schema; original list-field alias publication geometry remains separate from literal target method data; ambiguous or unreadable source aliases decline; editable spans additionally require exact unchanged raw bytes/static list ancestry, coupled-name call spelling and complete reference coverage; no entered alias/receiver, Native helper/private worker, runtime publication or general edit grant | `xtask-owner-resolution` |
| Original conditional case source geometry | `rust/tcl-compiler/src/registry_invocation/source_structure.rs`; `rust/tcl-compiler/src/analyser/oo.rs`; `rust/tcl-compiler/src/static_loops/original_simulation.rs`; `rust/tcl-compiler/src/sccp/original_value_facts.rs` | `OriginalSourceCase`; `original_logical_source_case_for_tokens`; `original_native_source_case_for_statement`; `StaticLoopSource`; `analyse_for_statement_with_source_dependencies`; `summarise_for_statement_with_source_dependencies` | full original vector/input/configuration and actual selected descriptor; readonly Logical case roles retain possible/no-match/unknown selection, patterns and original subject references; Native branch requires an already typed Switch and closed actual handler; original whole words own geometry while captured prefix values keep their own origins; source-aware loop analysis retains the actual CFG metadata and fold Registry, exact subject data or whole scalar-reference grammar, original literal patterns and ordered contiguous original body extents; glob/regexp/nocase, changed subjects, missing input and mismatched body mapping decline; readonly source correspondence cannot erase Native Switch evaluation without an independent retained execution-obligation carrier; no handler completion, compiler admission, Native frame or executable rewrite grant | `xtask-owner-resolution` |
| incomplete variable completion source geometry | `rust/tcl-syntax/src/naming.rs`; `rust/tcl-lsp-core/src/completion.rs` | `array_variable_completion_prefix` | editor fragment already excludes the substitution opener; preserves name/index source prefixes without stripping arbitrary braces; supplies no evaluated variable name, complete reference, cell or read proof | `xtask-owner-resolution` |
| scalar variable source-reference proposals | `rust/tcl-syntax/src/naming/variable_spelling.rs`; `rust/tcl-syntax/src/naming.rs`; `rust/tcl-lsp-core/src/completion.rs`; `rust/tcl-lsp-core/src/refactor/extract_proc.rs`; `rust/tcl-lsp-core/src/refactor/extract_variable.rs`; `rust/tcl-lsp-core/examples/tcloo_diag.rs`; `rust/tcl-lsp-core/src/minify.rs` | `scalar_source_reference`; `whole_scalar_variable_reference_name` | exact literal name data and complete selected LexerConfig; the shared whole-scalar-reference scanner must reproduce the entire proposed bare/braced reference and identical name; actual Logical completion source/scope currency remains independently required; qualification, prefix/fuzzy enumeration, local/cross-namespace candidates and current Logical snippet variable choices, and Logical extraction call/upvar parameter reads and proposed extracted-variable references use the same source-only constructor, with existing name restrictions and independent insertion/store/movement checks retained; the standalone TclOO diagnostic categorises variable receivers through the same configured whole-scalar scanner, without private sigil/brace stripping or a binding claim; Logical minifier scalar emission checks each genuine original token extent/name range, uses the same constructor and shared whole-literal next-component reference scan, and retains whole body source when emission is unavailable; original array references remain intact and quote removal uses the actual retained brace grammar; array-element data and incomplete or changed scalar proposals decline, without a Native recipe, cell, current value, read or edit grant | `xtask-owner-resolution` |
| Original Logical array-index candidates | `rust/tcl-compiler/src/analyser/scope/logical_array_indices.rs`; `rust/tcl-compiler/src/analyser/scope.rs`; `rust/tcl-syntax/src/naming/variable_spelling.rs`; `rust/tcl-syntax/src/naming.rs`; `rust/tcl-lsp-core/src/completion.rs` | `LogicalVariableScopeAdvice::array_index_candidates`; `LogicalArrayIndexCandidate`; `LogicalArrayIndexKind`; `LogicalArrayIndexCandidate::source_reference`; `array_source_reference` | exact current Logical source/input/configuration and selected declaration; actual definition/reference occurrences must reborrow a genuine original whole word and independently select the same declaration through their own source scope; static name operands additionally require the shared selected VarRead/VarWrite role and original content/value correspondence, while active variable components retain their actual index extent; literal key data and computed original index syntax remain distinct, computed syntax requires the same source frame/namespace, and braced insertion declines computed indices; literal proposals reproduce both root and index through the shared configured scanner/static decoder; legacy untyped index observations cannot issue insertion candidates; no Native name recipe, current cell/value/read, completed substitution or edit permission follows | `xtask-owner-resolution` |
| Logical variable source-scope eligibility | `rust/tcl-compiler/src/analyser/scope.rs`; `rust/tcl-compiler/src/analyser/types.rs`; `rust/tcl-compiler/src/analyser/handlers.rs`; `rust/tcl-lsp-core/src/definition.rs`; `rust/tcl-lsp-core/src/completion.rs`; `rust/tcl-lsp-core/src/rename.rs`; `rust/tcl-lsp-core/src/refactor/inline_variable.rs` | `LogicalVariableScopeAdvice`; `AnalysisResult::logical_variable_scope_advice`; `LogicalVariableScopeAdvice::eligible_scopes`; `LogicalVariableScopeAdvice::root_names_require_qualification`; `LogicalVariableScopeAdvice::cross_namespace_variable_names` | independently retained Logical input, exact whole image/channel/configuration and checked cursor; shared smallest-body descent and Registry frame-level grammar; distinct namespace/local/root/current/unknown-caller source tables; missing namespace coordinates retain only their own body tables without guessed qualification or bare root fallback; interpreter source bodies and foreign domain owners decline document-root borrowing; cross-namespace insertion coordinates come from each declaration’s own current retained namespace and must reselect that same declaration, independently of reporting scope names; variable inlining requires the actual written setter Name token to equal the selected declaration span and its sole reference to reselect the identical declaration, without a physical-line or flattened-scope search; consumers retain table/cell selection and independent edit checks; Native/hosted bodies and original readonly aliases cannot issue this source-only selector | `xtask-owner-resolution` |
| Source activation-local variable geometry | `rust/tcl-syntax/src/naming.rs`; `rust/tcl-compiler/src/inlining/frame.rs`; `rust/tcl-compiler/src/registry_invocation/authored_controls.rs`; `rust/tcl-compiler/src/analyser/param_traits.rs` | `source_variable_name_is_frame_local`; `source_variable_name_is_frame_local_for_all_policies` | actual retained VariableLookupPolicy and independently owned source-name extent; the empty name is local, C qualifiers select namespace storage, Jim relative qualified names stay local, and initial Jim root markers select global storage; inlining authenticates unchanged Module/metadata and positive Logical purpose, then reads the retained authored naming recipe’s variable lookup policy; unavailable recipes and Native/unknown purpose decline; an Authored source scalar without a selected lookup policy requires the intersection of both existing policy predicates, while a selected policy remains authoritative; no Native frame, lookup, input extent, presence or edit permission is issued; parameter-copy and caller-template consumers use actual literal name data with the retained variable policy, or agreement across source policies when no policy is held; empty, Unicode and literal sigil names are data, arrays remain separate, and no caller frame or completed write follows | `xtask-owner-resolution` |
| Original conditional word contents | `rust/tcl-compiler/src/command_binding/logical_word_values.rs`; `rust/tcl-compiler/src/command_binding/frozen_arguments.rs`; `rust/tcl-compiler/src/command_binding/declaration_preview.rs`; `rust/tcl-compiler/src/lowering/mod.rs` | `LogicalOriginalWordValue::capture_invocation`; `LogicalOriginalWordValue::is_current`; `LogicalOriginalWordValue::value`; `SourceInvocationBinding::original_authored_written_argument_value` | Logical values retain the actual source/input/frame/namespace/variable snapshot and original word; scalar substitutions require same-source quiet WrittenAt provenance, while arrays, commands and unknown contents decline; authored values independently require retained Standalone/AuthoredSimulation purpose, exact original frozen ordinal and evaluated-value agreement without dynamic storage/traces or Native coordinates; selected declaration bodies retain distinct evaluated source origin and base zero rather than borrowing variable-token extents; no Native read/frame, Normal completion, callable publication, quiet store or executable erasure grant | `xtask-owner-resolution` |
| Original procedure summary ownership | `rust/tcl-compiler/src/cfg_builder/summary_candidates.rs`; `rust/tcl-compiler/src/cfg_builder/mod.rs`; `rust/tcl-compiler/src/compilation_unit.rs`; `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/command_binding/retained_source.rs`; `rust/tcl-compiler/src/object_types.rs` | `OriginalProcedureSummaryOwners`; `OriginalProcedureSummaryOwners::for_module`; `CfgContext`; `PreparedCfgContext`; `PreparedCfgContext::from_source_input`; `PreparedCfgContext::from_metadata_input`; `LatticeRequest`; `LatticeRequest::original_metadata_context`; `Module::original_source_procedure_bodies`; `Module::original_source_procedure_for_target` | the fifth CFG context field retains the matching original Module and Registry; prepared metadata preserves explicit Standalone, Supplied, full supplied source and Unavailable purposes; the lattice request rechecks its issued body/header, current Module/input, nested configuration and effective profile before retaining that purpose; missing supplied input remains unavailable, including from_source_input(None); upvar, parameter and outer-write maps use issued ProcedureId coordinates; source inventory includes procedures and authentic OriginalDeclaration units under the current issuer/allocation/image; lambdas and installed-only units retain separate purposes; unique original target allocations and effective operands select summaries, including retained child calls; Native source_procedure_for_target remains independent; missing, changed or foreign owners widen effects rather than joining display names; raw memo identity retains the Arc owner independently of certified registry-only templates; no Native entry, entered frame, committed write or completion grant; object candidate harvesting, body spans, return candidates and flow edges reborrow the actual same Module ProcedureId/Procedure inventory without changing strict maps or supplying missing Native FunctionUnits | `xtask-owner-resolution` |
| Original Logical source frame projection | `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-compiler/src/registry_invocation/logical_frame.rs`; `rust/tcl-compiler/src/cfg_builder/upvar_info.rs`; `rust/tcl-compiler/src/command_binding/original_body_namespace.rs`; `rust/tcl-compiler/src/lowering/mod.rs` | `authored_logical_source_frame_argument_layout`; `OriginalLogicalFrameInvocation`; `original_logical_frame_invocation_with_metadata_context`; `OriginalLogicalFrameInvocation::source_body`; `SourceInvocationBinding::original_logical_frame_body_namespace`; `Lowerer::original_logical_body_for_module`; `original_logical_body_geometry_for_module`; `original_logical_frame_body_namespace_for_module`; `OriginalLogicalFrameInvocation::unknown_caller_variable_candidates` | complete original Logical input, selected descriptor, argv origins and realm; consensus source frame grammar preserves valid, invalid and unknown layouts; actual Module/positioned realm and shared OriginalRegistryScriptBody select only genuine original written children; current/root namespace recipes retain actual source-scope ancestry and exact selected-parent body correspondence independently of reached entered-body/Native receipts; namespace selection declines relative unknown-caller and dynamic levels; a separate unchanged-parent/Module-input body-geometry query supplies only context-admitted Registry literal MAY vocabulary for an unknown caller and always retains opaque read/write hazards; it cannot select current/root tables or entered-body coordinates; literal alias names remain data; Native handlers, numeric recipes, physical frames, completed links, captured/materialized child source and Normal completion remain independent | `xtask-owner-resolution` |
| Retained source body identity | `rust/tcl-core-types/src/body_identity.rs`; `rust/tcl-compiler/src/ir/procedure_identity.rs`; `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/source_graph.rs` | `ProcedureId`; `ProcedureIdIssuer`; `ProcedureInventory`; `Module::retain_procedure`; `Module::retain_body_unit`; `procedure_body_for_declaration` | private source_procedure_for_target and source_procedure_for_allocation select original allocating owners; module-scoped opaque body IDs retain repeated displays; clones share issuance, separate modules cannot join equal ordinals; exact allocating instruction/source/image/configuration and independently retained implementation body select originals; ordinal is presentation only, never Native lookup or entry authority | `xtask-owner-resolution` |
| Retained iRules event consumers | `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/source_graph.rs`; `rust/tcl-compiler/src/optimiser/unused_procs.rs`; `rust/tcl-compiler/src/analyser/diagnostics.rs`; `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs` | `event_body_for_procedure`; `event_body_for_function`; `SourceIrulesEventBody::event` | actual Module-issued body and Registry-selected original event; function joins require the same body ID, current Module/function metadata, source/configuration and retained event Arc; library detection, worker-state warnings, global-name fixes and racy cross-event diagnostics receive the selected event rather than parsing procedure labels; top level carries no event; missing owners decline event-only suppression and do not issue Native activation, worker or edit authority | `xtask-owner-resolution` |
| Original source-read catalogue | `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/command_binding/source_read_scope.rs`; `rust/tcl-compiler/src/command_binding/source_transition_advice.rs`; `rust/tcl-compiler/src/command_binding/source_transition_advice/body_effects.rs`; `rust/tcl-compiler/src/command_binding/source_transition_advice/logical_body.rs`; `rust/tcl-compiler/src/realm/source_transition_advice.rs`; `rust/tcl-compiler/src/registry_invocation/declaration_flow.rs` | `SourceCommandBindings`; `OriginalSourceCommandTransitionAdvice`; `OriginalSourceCommandTransitionAdvice::matches_source`; `OriginalSourceCommandTransitionAdvice::matches_context` | private SourceReadScope, OriginalSourceCataloguePurpose and DeclarationSchemaOrigin retain completed-prefix source schemas with genuine body issuers, original argv and current Registry/context/image/configuration; complete conditional body lookup effects distinguish exact command-resolution mutations from separately retained variable/observer uncertainty; immediate bodies and incomplete layouts keep their existing effect projection; they cannot override actually selected targets or issue reached runtime applicability, current stores or entered frames | `xtask-owner-resolution` |
| Package namespace diagnostic advice | `rust/tcl-registry/src/private_tcl_namespaces.rs`; `rust/tcl-compiler/src/analyser/diagnostics/validity.rs`; `rust/tcl-registry/src/model/context.rs` | `required_package_namespace_advice_covers`; `ResolvedContext::resolve_spec` | inline stub shadow advice selects descriptors from the actual analysis ContextRegistry/store with exact metadata names and full availability; literal package names remain data while command-prefix advice delegates shared written-name parsing; existing original user-resolution/source/order/path checks remain independent; no package loading, runtime binding or Native lookup is supplied | `xtask-owner-resolution` |
| Unattributed namespace edit hazards | `rust/tcl-syntax/src/naming.rs`; `rust/tcl-lsp-core/src/namespace_rename.rs` | `unattributed_rooted_name_candidates` | conservative text fragments retain literal colons and separate possible array roots; the broad delimiter/sigil scan can refuse an edit but supplies no lookup, variable access, source correspondence or editable span | `xtask-owner-resolution` |
| Logical original alias prefix provenance | `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/command_binding/logical_prefix.rs`; `rust/tcl-compiler/src/command_binding/logical_operation.rs`; `rust/tcl-compiler/src/command_binding/source_read_scope.rs`; `rust/tcl-compiler/src/registry_invocation.rs` | `SourceCommandTarget::original_logical_prepended_name_inputs` | positively retained Logical input only; complete original alias argv and source allocation, exact image/configuration/ContextRegistry and unanimous authored name policy retain captured original words across the existing target traversal, snapshots and joins; changed or missing provenance withdraws source roles and values; Native and hosted entries keep their independent original prefix owner; no Native object, Normal completion, runtime binding, frame or edit permission is issued | `xtask-owner-resolution` |
| Logical selected-frame source bodies | `rust/tcl-compiler/src/command_binding/source_declared_command/logical_body.rs`; `rust/tcl-compiler/src/command_binding/source_transition_advice/logical_body.rs`; `rust/tcl-registry/src/resolved_invocation.rs` | `OriginalDeclaredLogicalBodyContext`; `OriginalDeclaredSourceBodyFrame`; `OriginalDeclaredLogicalBodyContext::namespace_recipe`; `OriginalDeclaredLogicalBodyContext::frame_recipe`; `OriginalDeclaredLogicalBodyContext::scope_body`; `OriginalDeclaredLogicalBodyContext::body`; `ResolvedInvocation::authored_logical_source_frame_argument_layout` | actual retained Logical input, complete original invocation and independently selected Registry frame descriptor; relative zero preserves the current source coordinate, absolute zero retains its original body/level-word/ordinal recipe and selects an independent Logical root coordinate; original declaration remains ancestry; exact image/configuration/context and selected-body containment survive reuse; invalid, dynamic and unavailable caller coordinates decline; immediate child mutation coverage carries the same actual owner through Registry-selected root/current frames and reads its policy-specific source roles; possible child mutations and unknown path/export states withdraw subsequent source advice; no entered or Native frame, Module-root fallback, completed execution or edit authority | `xtask-owner-resolution` |
| Checked namespace emission geometry | `rust/tcl-syntax/src/naming.rs` | `checked_namespace_path_key` | exact retained byte namespace components; UTF-8 and a lossless constructed-key round trip are required; opaque or colliding components decline, and the report key grants no Native lookup, namespace liveness or compiler entry | `xtask-owner-resolution` |
| Whole source variable-reference shapes | `rust/tcl-syntax/src/naming.rs`; `rust/tcl-compiler/src/value_shapes.rs`; `rust/tcl-compiler/src/uri_split.rs`; `rust/tcl-compiler/src/unit_scope.rs`; `rust/tcl-compiler/src/analyser/handlers.rs`; `rust/tcl-compiler/src/rendered_properties.rs`; `rust/tcl-compiler/src/path_concat.rs`; `rust/tcl-compiler/src/native_integer_proof.rs`; `rust/tcl-compiler/src/interprocedural.rs`; `rust/tcl-compiler/src/sccp.rs`; `rust/tcl-compiler/src/value_transfer.rs`; `rust/tcl-compiler/src/sccp/original_value_facts.rs`; `rust/tcl-compiler/src/const_subst.rs`; `rust/tcl-compiler/src/tcl_expr_eval.rs`; `rust/tcl-compiler/src/optimiser/propagation.rs`; `rust/tcl-compiler/src/interval_bounds.rs`; `rust/tcl-compiler/src/analyser/diagnostics/proven.rs`; `rust/tcl-compiler/src/codegen/values.rs`; `rust/tcl-compiler/src/cfg_builder/cfg_lower.rs`; `rust/tcl-compiler/src/lowering/mod.rs`; `rust/tcl-compiler/src/dispatch_proof.rs`; `rust/tcl-compiler/src/analyser/diagnostics/usage.rs`; `rust/tcl-compiler/src/analyser/param_traits.rs`; `rust/tcl-compiler/src/analyser/diagnostics/var_command.rs`; `rust/tcl-compiler/src/realm.rs` | `is_whole_variable_reference`; `whole_scalar_variable_reference_name`; `whole_variable_reference_literal_name`; `LatticeDriver::source_reference_lexer_config`; `split_braced_var_ref` | actual complete reference bytes and explicit LexerConfig delegate whole-reference recognition to the Lexer; literal-name extraction refuses separate indices but preserves combined name data for an independently selected lookup; scalar extraction additionally refuses combined array references while retaining legal empty, literal sigil and unmatched-parenthesis scalar names; syntax cannot establish source ownership, cell selection, read/version contents or Native proof; SCCP/iteration consumers retain purpose-checked metadata and an actual selected profile through the Driver getter, while supplied missing or unprofiled input cannot borrow a default grammar; CFG recognition across either braced-reference close style is representation-only, while source-backed Codegen and constant lookup retain actual lexer configuration and their existing arenas/read/local-map guards; the dispatch proof fixpoint carries its actual metadata input grammar into complete-reference scanning and independent element/index classification, while missing or invalid grammar and dynamic or escaped separate indices retain hazards; Native evaluated-name literal projection and trace obligations stay independent; diagnostic expression-atom and namespaced ensemble-head consumers retain the actual original whole word, complete image and configuration; ensemble-head consumers additionally check the exact original parent head offset; scalar recognition declines arrays and compounds, while a namespace suffix must remain unchanged contiguous Text after its original scalar or Command component; RequiresReview, conditional values, current command-slot and literal command-tail eligibility remain independent; parameter traits require exact source/SegmentedCommand/full nested configuration correspondence before reborrowing original NativeWords; a sole typed Variable and shared whole-scalar grammar select reads, while arena variable/index components and the existing lexical nested-command scanner retain component geometry with role projection disabled; literal braced or escaped data and captured values cannot become parameter reads or source-frame authority | `xtask-owner-resolution` |
| Original variable token reference forms | `rust/tcl-compiler/src/var_refs.rs`; `rust/tcl-compiler/src/analyser/scope.rs`; `rust/tcl-lexer/src/word_parts.rs`; `rust/tcl-syntax/src/naming.rs` | `original_variable_reference_form` | the actual complete source/configuration and genuine Var token must agree with the shared scanner's original token extent and name range; the checked whole spelling uses the existing Syntax presentation owner, preserving literal name closers, sigils, empty names and separate array-index syntax; token content geometry retains literal-braced provenance through ordinary, nested command and expression reads; incompatible token/configuration geometry refuses, while caller scope/current-source and read authority remain independent | `xtask-owner-resolution` |
| Original source value components | `rust/tcl-syntax/src/word_rules.rs`; `rust/tcl-compiler/src/const_subst.rs`; `rust/tcl-compiler/src/analyser/handlers.rs`; `rust/tcl-compiler/src/registry_invocation/original_word_data.rs`; `rust/tcl-compiler/src/static_loops/original_simulation.rs`; `rust/tcl-compiler/src/codegen/wasm/backend.rs`; `rust/tcl-compiler/src/native_lowering/lower.rs`; `rust/tcl-compiler/src/executable_ir.rs`; `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs`; `rust/tcl-compiler/src/lowering/structured.rs`; `rust/tcl-compiler/src/lowering/mod.rs` | `original_word_unicode_value_with_variables`; `ConstSubstCtx::fold_original_source_substitution`; `original_scalar_variable_with_prefix`; `original_literal_argument_data_for_module`; `original_literal_argument_data`; `CellReference::from_name_data`; `original_name_role_values`; `OriginalSourceArgument`; `original_source_argument_for_module`; `original_source_argument_for_script`; `OriginalSourceArgument::scalar_reference_name` | original document word, full lexer configuration, grouping and executable-part arena remain together; callbacks receive genuine variable components, unsupported command/index/value cases decline, and conditional readonly values supply no Native cell, observer absence, executable erasure or editable reference; manufacturer-name syntax requires one nonempty original scalar component and an allowed decoded literal prefix; genuine complete words and current document lexer preserve grouping and exact variable-name bytes, while escaped sigils, arrays, commands and multiple components decline; this syntax query supplies no actual receiver, variable read, runtime publication or name lookup; the shared original-argument datum decoder requires the exact whole vector, retained point grammar and immutable source; its Module variant additionally checks current positioned correspondence and attachment ownership; it refuses synthetic/dynamic/NUL/foreign input; Native/WASM and effect-footprint consumers retain independent operation, dispatch, trace and cell eligibility; completed-name partitioning preserves literal empty/sigil/element bytes without reparsing them as source; selected original name roles require complete direct argument layouts and a literal decoded original word; LoopVarList parses that value with the retained list grammar, while VarWrite preserves one complete name; executable Foreach/Lmap cursors independently require actual descriptor/span/iterator correspondence, and file-line analysis supplies no list-cursor execution contract; the immutable original-argument view retains the original typed word, configured NativeWord and complete lexer configuration; Module issuance keeps current Module/Registry/attachment checks, while Script issuance independently requires actual statement membership, unanimous original carrier, same retained source instance, unchanged body bytes/extent and exact command-span/lowered-operand correspondence; literal data remains separate from sole scalar-reference syntax; source simulation copies only exact whole scalar reads or decoded literal data, while compound words, arrays, commands and missing or changed Script owners decline; Native assignment/increment adapters also match actual operation provenance/span/argv and keep existing read/trace guards; the immutable view does not revalidate later Module changes or grant Native entry; selected expression operands retain exact scalar-reference bytes through the shared whole-reference scanner under their complete preparation grammar, with absent grammar refusing a missing-byte operand lookup and existing-byte dependencies unchanged | `xtask-owner-resolution` |
| Original lowered expression name footprints | `rust/tcl-syntax/src/expr/ast.rs`; `rust/tcl-compiler/src/registry_invocation/original_word_data.rs`; `rust/tcl-compiler/src/command_binding/original_name_value.rs`; `rust/tcl-compiler/src/executable_ir.rs` | `ExprNode::evaluation_nodes`; `original_lowered_expression_syntax`; `SourceInvocationBinding::original_variable_reference_policy` | typed expression traversal retains word operands and opaque boundaries; exact whole original AST and expression base must match the selected original operand or one unique genuine checked child, retaining its Registry, full source vector and lexer configuration; variable identity uses original reference text with separate index substitutions and uncertainty; combined-name lookup uses only an independently retained selected policy, while Logical syntax cannot supply a Native recipe; this symbolic footprint proves neither an executed read nor a physical cell, store or Native entry | `xtask-owner-resolution` |
| Contextual metadata classifier selection | `rust/tcl-registry/src/registry.rs`; `rust/tcl-registry/src/command_descriptor_key.rs`; `rust/tcl-compiler/src/var_escape/helpers.rs`; `rust/tcl-compiler/src/auto_path_eval/path_constants.rs`; `rust/tcl-compiler/src/analyser/bounds_checks.rs` | `CommandRegistry::get`; `CommandSpec::descriptor_key`; `is_frameless_runtime_command_in` | exact-first Registry metadata selection owns optional-root fallback before exact effective-semantics lookup; BoundsMetadataContext retains actual context availability and supplied labels for role routing; authored path inventory retains its separate policy and lexical scope, while original path collection remains independently source-owned; `is_frameless_runtime_command_in` is a `cfg(test)` compatibility metadata classifier and supplies no production handler eligibility; missing metadata keeps refusal or barriers and supplies no current Native binding, frame or compiler entry | `xtask-owner-resolution` |
| Conservative local-slot metadata narrowing | `rust/tcl-compiler/src/var_escape/slot_resolution.rs`; `rust/tcl-registry/src/registry.rs` | `assign_local_slots`; `populate_local_slots`; `CommandRegistry::resolve_call`; `typed_local_slot_name` | accepted escape summaries and actual Module-issued BodyFrameId joins independently gate slots; source-backed calls require the original Script vector/current Registry and strict Normal frameless handler with complete selected roles and effects; typed scalar destinations require original Normal operand/descriptor correspondence and exact IR name data, while unavailable owners and dynamic/array or nonlocal names under the actual retained lookup policy decline; catalogue resolve_call supplies no physical handler, entered frame, cell or observer absence | `xtask-owner-resolution` |
| Self-dispatch source-table disturbance | `rust/tcl-compiler/src/source_command_observation.rs`; `rust/tcl-compiler/src/analyser/diagnostics/var_command.rs` | `SelfDispatchSourceTableProbe`; `SelfDispatchSourceTableProbe::for_selected_descriptor`; `SelfDispatchSourceTableProbe::any_reported_key` | actual contextual self-dispatch metadata independently selects the descriptor; authored alias/rename/deletion and declaration report tables retain their interpreter domains; exact bare/root report aliases are available only when the shared authored projection retains descriptor geometry, otherwise diagnostics withdraw; a negative table query grants no current binding absence, receiver, Native policy, entered frame or method-table authority | `xtask-owner-resolution` |
| Source-owned analysis cache correspondence | `rust/tcl-compiler/src/body_analysis_capsule.rs`; `rust/tcl-compiler/src/compilation_unit.rs`; `rust/tcl-compiler/src/command_binding/retained_source.rs`; `rust/tcl-compiler/src/source_graph.rs`; `rust/tcl-compiler/src/common_aot_plan/authored_source.rs`; `rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs`; `rust/tcl-compiler/src/command_binding/analysis_body_correspondence.rs` | `CompilationUnit::function_for_body`; `CompilationUnit::original_procedure_for_body`; `CompilationUnit::original_procedure_cfg_for_body`; `CompilationUnit::accepts_original_procedure_lattice`; `RetainedSourceModuleBindings::matches_original_procedure_summary`; `RetainedSourceModuleBindings::original_logical_source_slot_for_body`; `LatticeRequest::original_source_procedure`; `BodyAnalysisCapsule`; `BodyAnalysisCapsule::retain`; `BodyAnalysisCapsule::registry_snapshot`; `BodyAnalysisCapsule::has_closed_readonly_source_correspondence`; `BodyAnalysisCapsule::body_id`; `BodyLatticeInputs`; `BodyLatticeInputs::retain`; `BodyLatticeInputs::rebind_cached`; `BodyLatticeInputs::certified_native_source_body_id`; `BodyLatticeCommandBindings`; `BodyLatticeCommandBindings::bindings`; `BodySummaryInputs`; `BodySummaryInputs::retain`; `BodyLatticeInputs::certified_readonly_source_body_id` | genuine compilation, Module issuer, body ID, FunctionUnit and original source/input/configuration own every analysis; original Named and independently retained declaration bodies use the shared source inventory; ID-keyed source correspondence retains original header/formals/body source, direct statement geometry, body kind, publication and declaration site while excluding only the qualified reporting label; source graph/AOT/call/formal and memo consumers require their independent current Module/input/owner joins; current prepared CFGs are retained before lattice analysis and rejected memo callbacks supply no downstream baseline; Hosted functions retain only the exact current source card Arc, without borrowing an event, allocation, ABI or entered frame; exact source and variable relocation or independently checked Logical, positioned Native ReadFlow and Native Source-schema dependency certificates permit analytical comparison; the Source-schema certificate retains actual current input/entry/card/body/issued ID, original schema words/roles/namespace/path/applicability and actual command observers/source-step state, refusing unsupported effects or source shapes; full capsule comparisons additionally retain the actual function, summary, module and semantic facts with a unique original namespace policy/slot; private normalized carriers are cleared only after dependency capture, and cached results restore the current owner, proofs and issued body ID; an authentic current strong Named Logical header or held Logical declaration-card coordinate may retain a raw body with no command bindings or certificate and keeps full-context validation; missing/changed dependencies recompute, and analytical equality supplies no Native compiler, callable, frame, object or execution authority; the capsule retains the exact immutable Registry snapshot only after its actual current body/source/CFG checks, with that snapshot included in equality and hashing; procedure summaries, taint cascade and body-optimisation memo consumers borrow this same validated snapshot rather than an unrelated lattice key solely for Registry access, while producer eligibility, summary dependencies and full-source/trace refusal gates remain independent; explicit Standalone Authored source retains a distinct actual issued card, root grammar and current Module/body with a closed scalar declaration-scope dependency certificate; complete original vectors, configuration, quiet observers, source steps and registered effects precede normalization; readonly restoration restores current source ownership and issued ID independently of the Native-specific certificate and Native publication, without execution, frame or cell authority; readonly source optimisation requires an actual closed source certificate while retaining complete unit mutation and trace hazards in its dependency/equality value; under such hazards the per-body memo route requires nonempty capsules and every selected capsule certified, and retains authentic full-unit analysis before body filtering/rebasing; uncertified or empty source routes and the existing other-world, method, cross-function-read and argument-sensitive guards keep whole-unit fallback, without inferring physical quietness or Native execution | `xtask-owner-resolution` |
| Typed event-body connection scope | `rust/tcl-compiler/src/connection_scope.rs` | `EventCell`; `ConnectionScope`; `ConnectionScope::handler_source_names`; `ConnectionScope::observes_cross_event_place`; `build_connection_scope`; `build_connection_scope_with_registry` | actual CompilationUnit/Module-issued handler IDs and independently retained event metadata preserve repeated event labels; resolved cells and supplied worker/interpreter/storage domains remain separate from report unions; written protection labels project only from the matching handler keys and establish no arbitrary event order or current value | `xtask-owner-resolution` |
| Declared frame-effect metadata coordinates | `rust/tcl-registry/src/model/declaration.rs`; `rust/tcl-compiler/src/declared_frame_effects.rs`; `rust/tcl-compiler/src/dynamic_names.rs`; `rust/tcl-compiler/src/deferred_writes.rs` | `DocumentCommandSurface::plain_call_frame_effect_descriptor_keys`; `collect_declared_frame_effects`; `declared_frame_effect_for_metadata` | genuine selected document descriptor labels retain exact optional-root metadata coordinates; distinct colliding declarations withdraw assistance and selected registry commands retain precedence; standalone catalogue readers use this owner while original-source and entered Native frame evidence remain independent | `xtask-owner-resolution` |
| Selected lowering descriptor transport | `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/interprocedural.rs`; `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/lowering/mod.rs`; `rust/tcl-compiler/src/cfg_builder/execution_regions.rs`; `rust/tcl-compiler/src/native_lowering/lower.rs` ; `rust/tcl-compiler/src/command_binding/logical_operation.rs`| `InvocationFacts::descriptor_key`; `ResolvedInvocation::descriptor_key`; `AdmittedNativeCompilerInvocation::descriptor_key`; `SourceInvocationBinding::evaluated_written_argument_value` ; `RegistryInvocationShape::possible_method_instance_variable_names`; `SourceInvocationBinding::original_conditional_logical_scalar_store`; `ModuleCommandMutations::trusts_unobserved_descriptor`| genuine retained selected facts supply exact metadata labels to admitted and Logical hook selection and generic role reporting; private SelectedLoweringBindingIdentity separates actual admitted Native registration requirements from Logical descriptor metadata and delegates their one-root transport to Syntax; catalogue baseline and interprocedural invoker metadata delegate the original selected label directly to exact-first Registry get, retaining method rootability refusal; selected operation/dependency and mutation-report registration fields delegate one-root transport to Syntax, while CFG original lookup namespaces delegate to Runtime API in_rooted_namespace; native expression shape receives a genuine selected descriptor key before its unchanged mutation/trace guards; compiler admission, original argv/context/guard and reached handler remain independent; method-variable MAY protection reads exact decoded literal VarWrite operands from the selected source schema, including captured prefixes, empty names and literal sigils/braces; class declaration unions likewise decode original member tokens under the retained lexer configuration before slot/role selection, keep exact literal names and widen known-name protection on unknown operands; unavailable advice retains separate unparsed source-text protection instead of proving absence; explicit body-text shape classification selects exact-first Registry metadata and never issues Native lookup or frame authority; local body-literal retention requires the genuine ConditionalLogicalScalarStore quiet positioned source receipt, actual constant-assignment IR and matching original tokens; retained literal names decode with the current lexer through value_transfer and exact literal/array-root writes invalidate their entries; Logical map reads use the selected lexer whole-variable reference owner and reject separate or combined array elements and compound source; Native invocations cannot borrow that Logical receipt; independently retained original Native entries validate exact stamped source purpose before reading evaluated_written_argument_value at the original source ordinal, with compiler/body/context/guard admission unchanged; generic calls and evaluated expressions/values/increments withdraw the map because reported targets do not prove absence of computed, caller-frame or observer writes ; original candidate membership and shared frozen effective words support standalone/Native/Logical MAY method-name protection without borrowing source-only or reached authority; quiet Logical store retention uses the actual original operand snapshot through the existing scalar owner, independently of public address-phase reports; the shared module mutation owner checks selected descriptor eligibility against its exact optional-root MAY report coordinate and unchanged mutation/observer inventories, independently of actual handler, compiler and trace owners; retained binding operands distinguish genuine original written whole scalar references under their full configuration and unchanged argv from captured literal values; alias-pair summaries use the selected Registry successful layout and exact actual effective-argument cardinality before formal membership, without converting captured data to reads or weakening Native literal eligibility | `xtask-owner-resolution` |
| Conditional variable observation names | `rust/tcl-compiler/src/variable_observation_names.rs` | `VariableObservationNameSet`; `VariableProtectionPurpose`; `VariableWriteObservationPurpose`; `module_observation_key`; `variable_name_may_be_observed`; `record_variable_write_name` | explicit textual MAY inventories select module report-key, mixed exact/report or one-root native-lowering protection separately; deferred/global and outer-scope write projections retain their own aliases and array-root policies; decoded literal sigils, braces and empty variable names remain data, unknown inventories protect every name; exact VariableCellKey checks and genuine entered-Native observer guards remain independent, with no interpreted lookup, cell identity, callback-free state, absence or erasure permission | `xtask-owner-resolution` |
| APL variable convention | `rust/tcl-bigip/src/apl/variable_names.rs` | `tcl_var_to_apl_name`; `apl_name_to_tcl_var` | pure field convention delegates written root/qualifier geometry to Syntax; rooted and simple roots preserve lone colons while namespace-qualified roots decline; conventional export text is reporting only and supplies no appliance publication, native variable lookup or source-edit authority | `xtask-owner-resolution` |
| External script command inventory | `rust/tcl-syntax/src/naming/script_name_query.rs`; `rust/tcl-syntax/src/naming/script_command_inventory.rs`; `rust/tcl-cli/src/commands/spec_probe.rs` | `original_script_name_query_support`; `original_script_command_inventory_support`; `render_script` | real shell observations select query grammar without Native authority; C original simple words retain namespace context and checked root addressing; Jim uses its independent flat table; opaque or ambiguous inventory refuses as a whole; scanned registration literals remain unresolved authoring inputs | `xtask-owner-resolution` |
| Original alias registration lifetime | `rust/tcl-syntax/src/naming/native.rs`; `rust/tcl-registry/src/native_alias_registration.rs`; `rust/tcl-vm/src/interp/native_alias_registration.rs`; `runtime/rust/src/interp/native_alias_registration.rs` | `NativeNameProjection::alias_registration_key`; `NativeNameProjection::alias_registration_replacement_key`; `NativeAliasRegistrationTable` | C query, target, deletion and inventory retain actual backend tokens across relocation; C8.4 replacement and later-C unique tokens have separate publication policies; physical entry order retains the original registration objects; callbacks validate original input and current generations; Jim uses its independent command-object policy | `xtask-owner-resolution` |
| names / namespaces | `rust/tcl-syntax/src/naming.rs`; `rust/tcl-syntax/src/naming/reporting.rs`; `rust/tcl-syntax/src/naming/native.rs`; `rust/tcl-syntax/src/naming/native/routing.rs`; `rust/tcl-cmd-core/src/namespace.rs`; `rust/tcl-registry/src/registry.rs`; `rust/tcl-registry/src/source_name_ownership.rs`; `rust/tcl-lsp-core/src/rename.rs`; `rust/tcl-spectcl/src/loader/eval.rs` | `qualifier_component_extents`; `written_separator_extent`; `written_command_tail`; `qualifier_segments`; `qualify`; `qualify_namespace`; `canonical_written_command`; `key_tail`; `key_holder_and_tail`; `key_segments`; `key_suffixes`; `reported_namespace_contains`; `reported_namespace_strictly_contains`; `ReportedQNameNamespacePurpose`; `reported_qname_namespace`; `root_unrooted_key`; `unroot_rooted_key`; `autoload_command_candidates`; `native_autoload_command_candidates`; `CommandLookupBase`; `command_lookup_bases`; `NativeCommandSlotProjection::lookup_bases`; `NativeCommandSlotProjection::lookup_uses_namespace_path`; `command_resolution_candidates`; `command_resolution_candidates_from_namespace_keys`; `qualifiers`; `tail`; `exists`; `exists_bytes`; `parent`; `parent_bytes`; `children`; `children_bytes`; `which_request_in`; `which_command`; `which_command_bytes`; `which_command_bytes_checked`; `which_variable_bytes_checked`; `which_variable`; `variable_fqn`; `variable_fqn_bytes`; `import_pattern`; `origin_bytes_checked`; `origin_from_command_checked`; `TclStringHashOrder`; `TclStringHashOrder::statistics`; `is_static_variable_source_word`; `WrittenCommandParts`; `written_command_parts` | safe unqualified Logical rename replacement proposals use the shared qualify constructor over their existing constructed namespace prefix, with unsafe public workspace replacement inputs refused before lookup/edit construction and all source/target/collision checks retained; shared current/path/root order; native C8.4 excludes namespace path, while Jim retains independent flat-key lookup; base coordinates grant no liveness or closed-table proof; written input, authored constructed-text projection and retained native component geometry remain distinct; `key_segments` and holder/tail helpers choose an analytical split, not an inverse of colliding native displays; `which_variable`'s alternate global candidate is release-selected and 9.0 drops it; reporting QNames use pure typed prefix geometry independently of those name owners; inclusive report containment compares equality after removing one root marker, then uses the strict literal `::` prefix boundary; neither query recovers namespace identity; the static source-word constructor accepts only the fixed ASCII identifier subset for unquoted emission, separately from Tcl name acceptance, source-reference parsing and lookup; selected Global option-variable operands retain their actual effective scope/arity/role/opacity guards while the shared root/unroot helpers preserve already-rooted literals or add exactly one marker to the complete unrooted literal; array partitioning remains independently selected and this metadata supplies no namespace lookup, cell, Native frame or current value; authored SpecTcl static-head routing consumes counted written root and first literal tcl qualifier through the shared parts view; sandbox and extra-command policies remain independent, without canonical lookup, installed worker or Native admission | `xtask-resolution-drift` |
| written namespace separator predicate | `rust/tcl-core-types/src/naming.rs`; `rust/tcl-syntax/src/naming.rs`; `rust/tcl-lexer/src/highlight.rs` | `has_namespace_separator`; `is_qualified` | dependency-free counted-byte presence of two consecutive colons under the caller-selected extent; Syntax qualification and Lexer HTML/range classification delegate this same predicate; separator presence supplies no canonical name, C/Jim lookup policy, namespace identity, argument grammar or Native grant, and Lexer acquires no higher-layer Syntax authority | `xtask-owner-resolution` |
| alias chain topology | `rust/tcl-syntax/src/naming/alias_chain.rs` | `alias_chain_loops` | caller-owned actual command identities, independently selected global lookup and interpreter lifetime; a missing or non-alias target terminates, repeated nodes form a loop, and lookup uncertainty remains an error; Jim cycle policy is selected by each actual runtime adapter | none |
| authored path constant scope and transport | `rust/tcl-compiler/src/auto_path_eval.rs`; `rust/tcl-compiler/src/auto_path_eval/path_constants.rs`; `rust/tcl-lsp-core/src/workspace_index.rs` | `PathConstantAssignments`; `PathConstantAssignments::naming_policy`; `PathConstantAssignments::before`; `FoldedPathConstants`; `FoldedPathConstants::at`; `FoldedPathConstants::agreement`; `constant_path_assignments_with_naming_policy`; `SourceResolver` | retained authored naming issuer even on empty inventories, original namespace body intervals and source offsets, selected local/global homes and compatible batch/import lifecycles; only global homes export, equal values require equal issuers on every route; unsupported scopes withdraw rather than borrow enclosing values; navigation advice grants no native cell, frame or completion authority | `xtask-owner-resolution` |
| authored naming policy and signature publication | `rust/tcl-registry/src/native_string_materialization.rs`; `rust/tcl-syntax/src/naming/native.rs`; `rust/tcl-compiler/src/signature_scan/mod.rs`; `rust/tcl-compiler/src/signature_scan/original_name.rs`; `rust/tcl-lsp-core/src/namespace_import.rs`; `rust/tcl-lsp-core/src/workspace_index.rs`; `rust/tcl-compiler/src/signature_scan/scope.rs`; `rust/tcl-compiler/src/signature_scan/types.rs`; `rust/tcl-compiler/src/signature_scan/ctx.rs`; `rust/tcl-compiler/src/signature_scan/handlers.rs`; `rust/tcl-compiler/src/signature_scan/walker.rs`; `rust/tcl-compiler/src/signature_scan/factory.rs`; `rust/tcl-compiler/src/analyser/types.rs`; `rust/tcl-compiler/src/analyser/class_hierarchy.rs` | `InvocationDialect::authored_name_policy`; `NamePolicyAuthority::AuthoredSimulation`; `extract_signatures`; `SignatureScanResult`; `SignatureNamespaceScope`; `SignatureSourceCommand`; `SignatureSourceLookup`; `SourceNamespacePattern::matched_imported_command_tail`; `SourceCommandPublication`; `procedures_for_written_name`; `SignatureCommandAliasTarget::selected_global_name`; `SignatureCommandAliasTarget::reported_global_key`; `SignatureCommandAliasTarget::checked_global_key` | private ScanCtx::with_selected_schema retains selected schema transport; pure authored C release/Jim naming, original procedure publication and namespace/rename/alias purposes; unversioned Tcl explicitly selects the C8.6 analysis abstraction, while vendor compatibility does not issue a native policy; C8.4/8.5 reject colon-prefixed procedure tails outside root; procedure/class declaration inventories retain authored slots independently of reported labels; class relations retain original caller lookup contexts, TclOO object publication differs from explicit registry ProviderAdvice, and neither grants a runtime class/provider token; colliding label maps abstain, checked written lookup uses retained C components or Jim flat keys, and WrittenCaller aliases have no global target key; scan records supply source assistance only, without command tokens, compiler selection, entered frames or runtime lookup authority; main and factory scans share selected ClausePlan, original semantic keyword values and actual supplied source availability/config/inventory; positional handlers require exact direct written-origin mapping; captured/expanded operands cannot borrow body tokens; supplied missing/foreign/replacement refusal is terminal and header-only compatibility is explicit; import source advice selects C namespace/tail matching or Jim canonical flat-command enumeration through its actual retained publication owner; the matching SignatureSourceCommand supplies the exact tail consumed by Core, without interpreting a Jim flat key as a C path or completing conflicts, alias replacement, census or lifecycle | `xtask-owner-resolution` |
| original name producers and declaration metadata | `rust/tcl-compiler/src/signature_scan/original_name.rs`; `rust/tcl-compiler/src/signature_scan/name_value.rs`; `rust/tcl-compiler/src/analyser/types.rs` | `SignatureSourceNameKey`; `SignatureSourceNameValue`; `SignatureSourceNameInput`; `SourceOriginalNameOccurrence`; `SourceDeclarationMetadata`; `AnalysisResult::original_procedure_declarations`; `AnalysisResult::original_class_declarations`; `AnalysisResult::original_namespace_scope_at` | original input channel, full lexical configuration, explicit byte policy and purpose; readonly values cannot donate a complete original word, physical object, positioned implementation or editable reference | `check-name-resolution-proofs` |
| original workspace declaration transport | `rust/tcl-lsp-core/src/workspace_index.rs` | `WorkspaceIndex::original_procedure_declarations`; `WorkspaceIndex::original_class_declarations`; `WorkspaceInvocation` | exact original byte publications and workspace URI ownership survive display collisions; declaration assistance remains separate from actual call allocation and command lifetime | `check-name-resolution-proofs` |
| structured diagnostic naming subjects | `rust/tcl-compiler/src/analyser/diagnostic_subject.rs`; `rust/tcl-lsp-core/src/diagnostic_subject.rs` | `DiagnosticSubject`; `SourceUnresolvedCommandSubject`; `DiagnosticSubjectData`; `DiagnosticSubjectData::from_diagnostic`; `DiagnosticSubjectData::from_value`; `diagnostic_subject_data` | independently emitted byte keys and source coordinates; LSP, CLI and MCP share versioned reporting transport; missing, malformed and future data decline without parsing messages or recreating semantic authority | `check-name-resolution-proofs` |
| namespace variable declaration grammar | `rust/tcl-registry/src/native_variable_declaration.rs`; `runtime/rust/src/cmd_var.rs`; `rust/tcl-vm/src/command.rs` | `NativeVariableDeclarationProtocol`; `NativeVariableDeclarationProtocol::arguments`; `InvocationDialect::native_variable_declaration_protocol` | authenticated C8.4/8.5 require at least one operand, C8.6+ permit an empty declaration, and Jim accepts one name with an optional value, refusing a third operand; grammar admission precedes cell declaration and does not issue a variable lookup capability; exact usage and six-provider conformance captures are in `experiments/name-handling/results/native-variable-array-modes` | `xtask-owner-resolution` |
| original array name enumeration modes | `rust/tcl-cmd-core/src/array.rs` | `NativeArrayNamesProtocol`; `NativeArrayNamesProtocol::from_name_policy`; `NativeArrayNamesProtocol::wrong_arguments_suffix`; `dispatch_bytes_at` | one pure mode grammar over the selected naming issuer; C supports a separate mode word while Jim accepts only a glob pattern; original option lookup, checked key order, exact CString extent and `NativeNameGlobPurpose::ArrayNamesSearch` remain distinct owners; regular-expression enumeration refuses until its original worker is available; six-provider captures are in `experiments/name-handling/results/native-variable-array-modes` | `xtask-owner-resolution` |
| original array search chains | `rust/tcl-core-types/src/native_array_search.rs`; `rust/tcl-syntax/src/native_array_search.rs`; `rust/tcl-registry/src/native_variable_table.rs`; `rust/tcl-runtime-api/src/native_hash_abi.rs`; `rust/tcl-cmd-core/src/native_array_search.rs` | `NativeArraySearchAbi`; `NativeArraySearchCache`; `NativeArraySearchChain`; `NativeArraySearchProtocol`; `native_array_search_protocol`; `supported_backend_array_search_abi`; `NativeArraySearchBackend`; `NativeArraySearchOperand`; `dispatch`; `resolve` | actual C unsigned-long and int layouts are independent receipts; borrowed original handle/bytes/cache carrier preserves conversion phase without acquiring a header role; C8 resident cache retains signed ID and byte offset while C9 retains the original handle; the owning chain belongs to the original array incarnation, drops handles at native invalidation and does not pin variable cells; undefined-shell collection does not invalidate; C8 descriptors have NULL free, duplicate and updater hooks, while Rust cache cleanup belongs to the sole object-allocation auxiliary owner; Jim has no native Search members | `xtask-owner-resolution` |
| original Jim command and variable lookup caches | `rust/tcl-syntax/src/native_jim_lookup.rs`; `rust/tcl-registry/src/native_jim_lookup.rs`; `rust/tcl-vm/src/interp/native_jim_lookup.rs`; `rust/tcl-vm/src/value/native_jim_lookup.rs`; `rust/tcl-vm/src/frame.rs`; `rust/tcl-vm/src/vars.rs`; `runtime/rust/src/interp/native_jim_lookup.rs`; `runtime/rust/src/frame/jim_lookup.rs`; `runtime/rust/src/obj/native_jim_lookup.rs` | `NativeJimLookupProtocol`; `native_jim_lookup_protocol`; `command_is_current`; `variable_is_current` | actual pinned Jim issuer; command cache owns original namespace only and validates native procedure epoch, counted namespace equality and actual live binding/invocation; an invocation retains its selected node through synchronous or NRE completion while the original command cache never owns the callable; variable cache owns no cell/value/frame and validates selected current/top frame incarnation; new unqualified variable table key owns the original object; absolute key is separate; misses preserve the original primary; metadata/type names grant no lookup authority; internal implementation: `read_original_named_variable`, `store_original_named_variable`, `WeakJimVariableCell` | `xtask-owner-resolution` |
| original Jim local commands and upcalls | `rust/tcl-syntax/src/native_jim_local.rs`; `rust/tcl-registry/src/native_jim_local.rs`; `rust/tcl-vm/src/interp/jim_local.rs`; `rust/tcl-vm/src/interp/native_jim_lookup.rs`; `runtime/rust/src/interp/jim_local.rs`; `runtime/rust/src/namespace/jim_local.rs` | `NativeJimLocalProtocol`; `native_jim_local_protocol`; `changes_epoch`; `follows_previous`; `accepts_upcall`; `accepts_rename` | actual pinned Jim issuer and owning previous-node edges; occupied local publication transfers the previous worker and changes procedure epoch; original top command cache is installed before live upcall traversal; frame cleanup retains original result-name objects and restores bindings in reverse order without an epoch change; nonempty rename with a previous edge refuses; pure policy and cache metadata grant no node lifetime authority | `xtask-owner-resolution` |
| native root bootstrap purposes | `rust/tcl-registry/src/special_vars.rs`; `rust/tcl-vm/src/interp.rs`; `runtime/rust/src/interp.rs` | `NativeBootstrapPurpose`; `NativeBootstrapVariable`; `NativeBootstrapInputs`; `NativeBootstrapProtocol`; `native_bootstrap_protocol`; `allocations`; `registers_core_binary`; `registers_core_try`; `registers_core_throw`; `initializes_tcl_oo`; `with_native_core` | actual constructor root births are separate from Tcl_Main arguments, real script-library initialization, Jim extensions and ordinary embedding-host defaults; undefined cells remain physical births; core-only command gates exclude distribution-owned binary/try/throw/autoload and unsupported OO initialization independently of root variables; build path bytes are independent inputs; logical/vendor profiles cannot issue physical inventory | `xtask-owner-resolution` |
| native stock ensemble registration | `rust/tcl-registry/src/invocation_words.rs`; `rust/tcl-registry/src/commands/tcl/clock_.rs`; `runtime/rust/src/interp/stock_ensembles.rs`; `runtime/rust/src/cmd_clock.rs` | `EnsembleImplementationFamily`; `NativeEnsembleBootstrap`; `InvocationDialect::clock_bootstrap_ensemble`; `InvocationDialect::ensemble_implementation_namespace`; `InvocationDialect::ensemble_implementation_command_address`; `InvocationDialect::ensemble_bootstrap_deferred_members` | selected C release and registry-owned implementation paths; C8.6 clock retains deferred calendar targets without publishing worker tokens, while reading hooks retain independent compiler descriptors. Hook presence supplies no handler-effect, opcode or completed-world authority. Current map membership and deferred dispatch agree with C8.6 controls; ordered map/worker registration and calendar support remain separate gaps; primitive Clock bootstrap shares the actual stock-family publication owner; its selected descriptor retains the real public generation with no ensemble configuration, members or namespaces, and refresh checks this publication through displaced configuration retirement without issuing namespace or compiler authority | `xtask-owner-resolution` |
| direct native variable callbacks | `rust/tcl-runtime-api/src/native_variable_trace.rs`; `rust/tcl-registry/src/native_variable_trace.rs`; `rust/tcl-vm/src/interp/native_variable_observers.rs`; `runtime/rust/src/interp/native_variable_observers.rs` | `NativeVariableObserver`; `NativeVariableTraceAccess`; `NativeVariableTraceOperation`; `NativeVariableTraceToken`; `NativeVariableTraceProtocol`; `NativeTraceStatePurpose`; `NativeTraceStateRecipe`; `native_variable_trace_protocol`; `native_trace_state_recipe`; `saves_chain_result`; `saves_script_result`; `add_native_variable_observer`; `remove_native_variable_observer` | selected actual C issuer and original stable cell; callbacks share newest-first trace ordering, active-chain suppression, removal and unset retirement, but remain absent from guest trace introspection; direct callbacks create no script frame or result reset; C8.5+ preserves the interpreter state once per chain; C8.4 script callbacks transfer the result independently, with command-script return code preserved separately | `xtask-owner-resolution` |
| native hidden error-variable callbacks | `rust/tcl-registry/src/special_vars.rs`; `runtime/rust/src/interp/native_error_variables.rs`; `rust/tcl-vm/src/interp/native_error_variables.rs`; `rust/tcl-cmd-core/src/native_append.rs` | `NativeErrorVariableProtocol`; `NativeErrorVariableRead`; `native_error_variable_protocol` | C8.5+ interpreter creation registers hidden callbacks on actual root cells; reads copy retained private objects only under the legacy-copy flag; unset recreates an undefined cell; reset publishes code before info and releases private owners; C8.4 and Jim have no such callbacks; both backends retain original private error objects and carried option values; modern trace chains preserve their selected interpreter state, script callback entry resets the selected error state, and nonempty errorInfo mutations use the shared counted-byte append owner | `xtask-owner-resolution` |
| native variable entry order | `rust/tcl-core-types/src/native_hash_order.rs`; `rust/tcl-runtime-api/src/native_hash_abi.rs`; `rust/tcl-registry/src/native_variable_table.rs`; `rust/tcl-runtime-api/src/lib.rs` | `NativeHashAbi`; `NativeHashOrder`; `NativeEntryLedger`; `supported_backend_hash_abi`; `NativeVariableTableProtocol`; `native_variable_table_protocol`; `authored_variable_table_protocol`; `Frames::var_names_bytes_checked`; `Namespaces::vars_in_bytes_checked` | actual plain-char promotion and unsigned-int/size_t widths are independently issued by the supported backend; C8.4/8.5 variable hashing uses plain char, C8.6 unsigned bytes with unsigned int, C9 unsigned bytes with size_t; Jim requires retained seed and lazy table allocation; authored simulation has independent authority; real entry birth/retirement includes undefined shells, while compiled declaration slots remain ordered and distinct | `xtask-owner-resolution` |
| independently issued execution naming | `rust/tcl-syntax/src/naming/execution_policy.rs`; `rust/tcl-registry/src/f5/naming.rs`; `rust/tcl-runtime-api/src/native_compilation.rs`; `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/var_resolve.rs` | `ExecutionNamePolicy`; `ExecutionVariableNameProjection`; `ObservedBigIpNamePolicy`; `MeasuredBigIpNameScope`; `ObservedVariableNamePurpose`; `observed_name_policy`; `execution_name_policy`; `command_name_policy` | naming selection is independent of producer-string policy; native C/Jim and authored recipes remain distinct from the operation-bounded counted exact-build TMM HTTP variable recipe; unsupported qualification, empty/nested grammars, forms and contexts withdraw; observed projections grant no storage receiver, compiler-local provider, command cache or procedure-header compiler; source and native entry transports retain the issuer independently | `xtask-owner-resolution` |
| purpose-selected variable namespace geometry | `rust/tcl-syntax/src/naming/variable_geometry.rs`; `runtime/rust/src/state_traits.rs`; `rust/tcl-vm/src/interp.rs`; `rust/tcl-cmd-core/src/namespace.rs`; `rust/tcl-lsp-core/src/definition.rs` | `VariableLookupBase`; `variable_namespace_lookup_bases`; `NativeVariableNamespaceProjection`; `NativeNameProjection::variable_namespace_projection`; `NativeVariableNamespaceProjection::namespace_query_bases`; `NativeNameProtocol::variable_root_geometry` | C original purpose and selected extent precede common component geometry; only NamespaceVariableQuery grants the C84–86 current/global query policy, never VariableRoot or Jim; actual token equality suppresses duplicate root coordinates without proving liveness; explicit Logical callers supply their bounded fallback policy; physical table/cell presence, procedure frames, aliases, read/write flags and dying namespace lifetimes remain independent | `xtask-owner-resolution` |
| selected invalid namespace creation address | `rust/tcl-syntax/src/naming/native.rs`; `runtime/rust/src/interp/namespace_address.rs`; `runtime/rust/src/cmd_oo.rs`; `rust/tcl-vm/src/interp.rs` | `NameProjectionUnavailable::EmptyNamespaceOutsideRoot`; `NativeNameProtocol::namespace_address_path`; `report_native_namespace_creation_error`; `report_native_namespace_creation_conflict`; `Interp::namespace_creation_conflict_error`; `NativeNameProtocol::namespace_creation_reuses_parent` | selected empty relative C address outside root yields lookup absence or its independently selected creation diagnostic; C84/85 retain primitive error-code state, C86+ supply CREATEGLOBAL; unsupported contexts/recipes remain refusals, and pure error bytes do not grant native String production or frame entry; occupancy diagnostics retain the actual original C-string operand independently of the selected namespace address; C86+ supplies CREATEEXISTING while C84/85 retain primitive error-code state; Runtime namespace publication and explicit OO namespace creation share the same report adapter without changing their own lookup, class, lifecycle or getter checks; the pure C parent-return query requires the original CString trailing separator and absent actual delete callback after an independently selected dying final token; Runtime/VM reuse that exact selected NsId, without descendant creation, publication, frame or cleanup authority; Jim and callback-bearing requests decline | `xtask-owner-resolution` |
| original script command presence | `rust/tcl-syntax/src/naming/script_command_presence.rs` | `ScriptRootCommandPresencePlan`; `original_command_presence_support` | explicit root context and selected C8.4–9.1/Jim0.84 query grammar; C literal qualifier plus escaped simple tail versus Jim escaped flat pattern; unknown grammar or unavailable original-byte transport refuses; original autoload/invocation word stays separate from inventory reports | `xtask-owner-resolution` |
| script namespace-variable snapshots | `rust/tcl-syntax/src/naming/script_namespace_snapshot.rs` | `original_namespace_variable_snapshot_support` | retained namespace context plus original simple variable operand; explicit global query/read/array/trace operations; conservative refusal of ambiguous reports and unsupported enumeration; coordinates grant no Native namespace incarnation or variable-cell identity | `xtask-owner-resolution` |
| native name input and reporting | `rust/tcl-syntax/src/naming/native.rs`; `rust/tcl-syntax/src/naming/jim_enumeration.rs`; `rust/tcl-syntax/src/naming.rs`; `rust/tcl-syntax/src/naming/aliases.rs` | `NativeNameProtocol`; `NativeNameProtocol::jim_two_word_member_publication_slot`; `NativeNameProtocol::rename_alias_loop_name`; `NativeNameContext`; `NamePolicyProtocol`; `NativeNameProjection`; `NativeVariableProjection`; `command_publication_slot`; `command_c_api_publication_slot`; `variable_root_input`; `combined_variable_input`; `separate_variable_input`; `trace_registration_input`; `namespace_pattern_parts`; `NativeNamespacePatternParts`; `NativeNamespacePatternSource`; `namespace_subcommand_input`; `interpreter_subcommand_input`; `namespace_variable_query_input`; `jim_namespace_canonical_input`; `jim_namespace_absolute_canonical_input`; `jim_namespace_command_query_candidates_from_canonical`; `jim_namespace_construction`; `NativeJimNamespaceConstruction`; `jim_info_command_names`; `jim_procedure_namespace`; `ensemble_publication_slot`; `report_native_namespace_lookup_error`; `report_native_namespace_operation_error`; `report_native_dictionary_missing_key`; `oo_method_input`; `oo_object_publication_slot`; `package_key`; `formal_storage_name_input`; `formal_name_input`; `c_family_local_alias_name_bytes`; `hidden_token_input`; `report_native_name_bytes`; `report_native_variable_access_trace_names`, `report_native_variable_diagnostic`; `report_native_variable_diagnostic_at`; `native_constant_failure_verb`; `checked_command_slot_utf8`; `checked_namespace_path_utf8`; `native_command_source_spelling`; `native_namespace_source_spelling`; `native_jim_namespace_source_spelling` | actual audited engine/build, explicit authored simulation authority, purpose, original byte input form and actual namespace context; command/C API publication, scalar, combined/separate element, trace receiver and report remain separate; checked analytical aliases require exact native round-trip; release-independent authored C alias projection declines unqualified NUL-bearing names and grants no physical lookup or release authority; Jim absolute canonical projection needs no namespace bytes and retains its original operand with CString suffix extent, while relative input declines that branch; checked query-candidate factoring requires the same private protocol and canonical purpose; these byte projections grant no namespace holder, caller frame or operation authority, and the existing contextual entry still requires namespace bytes | `xtask-owner-resolution` |
| native TclOO variable declarations | `rust/tcl-syntax/src/naming/oo_variables.rs`; `rust/tcl-syntax/src/native_glob.rs` | `NativeOoVariableSlotOperation`; `NativeOoVariableSlotSelection`; `NativeOoVariableError`; `native_oo_variable_slot`; `validate_native_oo_variable`; `apply_native_oo_slot_records`; `apply_native_oo_variable_slot` | actual C8.6/C9 variable slot methods; full counted ObjHash membership, original selected declaration object, CString validation and reporting are independent; properties use their own object comparison owner | none |
| original C9 class-variable links | `rust/tcl-registry/src/native_tcloo_method_definition.rs`; `runtime/rust/src/cmd_oo.rs`; `runtime/rust/src/interp/native_variable_names.rs` | `NativeTclooClassVariableProtocol`; `NativeTclooClassVariableHeadStage`; `classvariable_original`; `Interp::prepare_original_c_namespace_link_target` | actual C9.0/C9.1 method activation, declaring class facet and original namespace token; selected head String stage and all-name validation precede every target lookup/local link; original arguments retain allocation safety without native references, and callback withdrawal stops later binding through the existing C name/cell kernel; C8.6 compatibility is independently supported and supplies no C9 worker purpose | none |
| original source literal arrays | `rust/tcl-runtime-api/src/native_literal.rs`; `rust/tcl-vm/src/literal_pool.rs`; `runtime/rust/src/interp/native_literal_pool.rs`; `rust/tcl-registry/src/native_eval_object.rs` | `command_literal_partition`; `registered_c84_long`; `NativeSourceLiteralAction`; `source_literal_action`; `source_literal_empty_result_is_unshared`; `permits_direct_source_operands`; `NativeLiteralWorld::register_using`; `NativeLocalNameTable::create_using` | actual source object identity, matching literal allocation issuer and real interpreter registrations; C8.6/C9 replaces identical source literals with fresh string-only objects; C8.5 uses hidden command literals and an unshared empty-script result; C8.4 retains each source slot registered in the same live world and withdraws registered local slots at actual interpreter table cleanup; both ports own original ordered local-array members and release actual registration leases before local native refs; registered C8.4 decimal primaries consume the explicit physical native-long width; private constant List members are fresh original String objects; array size grants no ownership authority; equal bytes alone grant no source identity; registration getters run outside world/procedure borrows using allocation-only original header projections, actual entry birth and a mutation episode; each acquisition, insertion and release advances that episode, and exhausted owner counts refuse before cloning; original compiler target/frame and procedure/local-table slots are checked before publication, with the first typed cause retained before unused objects and partial arrays retire; selected C8.4 direct evaluation retains a plain source control plan and creates fresh original string operands per reached word, with no native literal registration, compiled layout or compiler cache; internal implementation: `NativeRuntimeLiteralArray`, `create_native_literal_array`, `create_native_literal_array_with_actions`, `NativeRuntimeLiteral::UnsharedOriginal`, `finalize_original_source_pool`, `direct_source_unit` | `xtask-owner-resolution` |
| original C variable-name primaries and canonical local names | `rust/tcl-syntax/src/native_variable_name.rs`; `rust/tcl-registry/src/native_variable_name.rs`; `rust/tcl-runtime-api/src/native_literal.rs`; `rust/tcl-vm/src/interp/native_variable_names.rs`; `runtime/rust/src/interp/native_variable_names.rs`; `runtime/rust/src/obj/native_variable_name.rs`; `runtime/rust/src/frame/native_variable_names.rs`; `runtime/rust/src/frame/materialisation.rs`; `runtime/rust/src/vars.rs` | `NativeVariableNameProtocol`; `NativeVariableNameLookupPurpose`; `NativeVariableNameProtocol::alias_local_input`; `alias_local_is_element`; `native_variable_name_protocol`; `NativeParsedVariableName`; `NativeLocalVariableName`; `NativeLiteralWorld`; `NativeLocalNameTable` | actual matching native C name policy; C8.4 failed simple lookup retires only a primary with a physical free hook, C8.5/8.6 retire all, C9 preserves a prior primary on a miss; parsed array parts own original root plus release-specific element storage; compiled local caches validate the actual procedure or canonical name and indexed cell; read, write, link and unset prepare the original cache under their actual frame and lookup purpose; QuietUnset suppresses guest missing-cell presentation while preserving host refusal, and explicit root/index lookup cannot borrow the combined-name parsed-array primary; C8.5+ scalar/root hash entries retain their original key independently of the variable cell; canonical names share one reusable procedure header and unpartitioned global registrations; the first simple/indexed selection retains only weak cell, binding, table/frame birth and link observations through the same checked primary retirement; reached receiver and member projections consume that selection without a second name classification, and typed refusal is retained before unadopted cache parts retire; C8 retirement still precedes the first simple lookup while C9 postlookup retirement checks the original selected cell; internal implementation: `native_c_variable_name_protocol`, `prepare_native_name_cell`, `retain_native_key`, `NativeOriginalNameCell`, `prepare_original_c_indexed_name_cell`, `capture_original_c_selected_receiver`, `discard_native_internal_representation_with_stage_check` | `xtask-owner-resolution` |
| retained C scalar aliases and array keys | `runtime/rust/src/frame/native_scalar_alias.rs`; `runtime/rust/src/frame/native_element_entry.rs`; `runtime/rust/src/frame/native_variable_names.rs`; `runtime/rust/src/frame/destruction.rs`; `rust/tcl-vm/src/vars.rs`; `rust/tcl-vm/src/interp/native_variable_names.rs`; `rust/tcl-runtime-api/src/lib.rs`; `rust/tcl-vm/src/interp.rs`; `runtime/rust/src/interp.rs` | `Frames::link`; `VarStore::get_bytes`; `VarStore::set_bytes`; `Vm::try_eval_source_image`; `Interp::eval_str` | each new scalar alias holds its exact variable entry; borrowed link views share that hold; undefined unset and recreation preserve the entry and original key until its final alias retires; namespace retirement releases table ownership independently; C8.4 array keys own bytes; C8.5+ object-key tables retain the original separate index, while combined C8.5/8.6 parsed bytes create a fresh CString key header and C9 retains the original counted parsed index; each element alias holds its exact entry without another key-header reference; single-element unset preserves an undefined aliased entry and key, while whole-array retirement releases each key at actual hash-entry destruction and aliases retain only the disconnected Var until their final hold ends; retired array generations cannot redirect aliases into replacements; internal implementation: `NativeScalarAliasEntry`, `NativeElementAliasEntry`, `retain_native_scalar_alias`, `retain_element_alias`, `prepare_original_native_element`, `retain_original_array_key`, `retire_released_native_alias_entries` | `xtask-owner-resolution` |
| native compiled-variable selection | `rust/tcl-syntax/src/naming/compiled_variables.rs`; `rust/tcl-registry/src/native_compiled_variables.rs`; `rust/tcl-bytecode/src/lib.rs` | `NativeCompiledVariableProtocol`; `NativeCompiledVariableRecipe`; `NativeCompiledVariableEnvironment`; `NativeCompiledVariableLookup`; `CompiledVariableTarget`; `compiled_local_names_equal`; `dynamic_local_names_equal`; `substitution_lookup`; `command_lookup`; `supports_environment`; `native_compiled_variable_protocol`; `authored_logical_compiled_variable_protocol`; `NativeCompiledScalarName`; `NativeCompiledVariableRecipe::scalar_name`; `LocalVarTable::intern_anonymous`; `LocalVarTable::from_native_names`; `from_native_slot_names`; `native_slot_names`; `set_native_protocol`; `find_native`; `intern_native`; `is_source_local_native` | independently selected actual compiler or explicit authored logical provider; counted original names and token geometry; compiler length-plus-CString comparison, dynamic frame-local comparison and dynamic hash keys remain distinct | none |
| native control compiler preparation | `rust/tcl-registry/src/native_control_compilation.rs` | `NativeControlBooleanProbe`; `NativeControlBooleanProbe::matches_original`; `NativeControlPreparationStep`; `NativeControlOutcome`; `NativeControlCompilation`; `NativeLocalScalarProjection`; `project_native_local_scalar`; `native_control_body_span` | original compiler words, operand indices and exact body spans under the selected C compiler/frame; ordered original Boolean-probe metadata (no allocation or child visit), local, anonymous-slot, word, literal, private Integer/List, script-context, speculative script and expression visits remain independent from Inline, Generic or Rejected outcomes; a decline retains completed declarations and allocations; unavailable geometry is a typed refusal | `xtask-owner-resolution` |
| native conditional and loop instructions | `rust/tcl-registry/src/native_control_instructions.rs`; `rust/tcl-bytecode/src/lib.rs` | `NativeControlBody`; `NativeControlTest`; `NativeConditionalClause`; `NativeControlInstruction`; `NativeCatchProtocol`; `result_before_options`; `Instruction::catch_start`; `Instruction::catch_end`; `NativeControlInstructionUnavailable`; `native_control_instruction` | original if/while/for/catch compiler selection; static script extents and dynamic original operands are distinct; fresh NULL Boolean probes determine pruning and compiler visits; for retains separate body/next loop ranges; catch retains release-selected substitution/range order, stack layout and result/options store order; `Instruction::catch_start` and `catch_end` separate BEGIN lifetime from protected body extent, and C8.4 speculative rejection preserves completed preparation; recipes grant no callable or completed operand authority | `xtask-owner-resolution` |
| f5-string-predicates | `rust/tcl-syntax/src/expr/operators.rs`; `rust/tcl-registry/src/native_expression_program.rs` | `MeasuredBigIpExpressionObservation`; `AuthoredF5StringPredicateProvider`; `AuthoredF5StringPredicateProvider::predicate`; `F5StringPredicate`; `F5StringPredicate::evaluate`; `authored_f5_string_predicate_provider` | one pure authored string recipe after independent parser and checked operand admission; bare `matches` and `matches_glob` use case-sensitive whole-string glob matching; exact BIG-IP 21.1.0.1 build 0.0.26 observations cover HTTP_REQUEST, tmsh CLI, iApp implementation and triggered iCall, and grant no live capability; compiler, VM and Runtime retain typed authored policy; native C/Jim and unavailable contexts withdraw | `xtask-owner-resolution` |
| expression evaluation activation | `rust/tcl-runtime-api/src/expression_policy.rs`; `rust/tcl-registry/src/native_expression_program.rs` | `ExpressionEvaluationOrigin`; `ExpressionEvaluationPolicy`; `native_expression_evaluation_policy`; `authored_expression_evaluation_policy`; `expression_function_dispatch`; `compilation_expression_function_dispatch`; `compilation_expression_source_evaluation_supported`; `ExpressionProgramEmission`; `expression_program_emission`; `expression_program_emission_for_policy`; `expression_tree_is_call_free`; `expression_boolean_probe_matches`; `control_boolean_probes_match` | exact activation profile and independently installed parser/numeric issuer, separate from physical compiler registration and original native preparation; native function dispatch uses the actual engine; authored source emission retains unchanged bytes and grants no native pooled result or fixed-function table; checked function calls additionally require their separately installed authored provider; every reached original Boolean-pruning predicate retains its operand/value, including omitted false clauses; authored BooleanValue parsing must agree with the physical decision, with no native header allocation or child visit; missing or conflicting capabilities refuse; entry-world and VM/Runtime artifact cache equality retain the evaluation policy | `xtask-owner-resolution` |
| authored Tcl84 fixed functions | `rust/tcl-runtime-api/src/expression_policy.rs`; `rust/tcl-registry/src/authored_math_functions.rs`; `rust/tcl-vm/src/interp/authored_math.rs` | `AuthoredMathFunctionProvider`; `authored_math_functions::provider`; `authored_math_functions::arity`; `authored_math_functions::prepare`; `Vm::set_logical_math_function_provider` | explicitly installed checked function set, independent of native registrations and compiler admission; original operand chronology and authored numeric conversions; each actual interpreter owns its seed-one Park–Miller stream and precision-twelve lazy double formatter; initial state is a reproducible simulation choice, not an appliance measurement; installation and withdrawal invalidate captured policies and caches; native host activation retains its own state | `xtask-owner-resolution` |
| native expression operand programs | `rust/tcl-registry/src/native_expression_program.rs` | `NativeExpressionTree`; `NativeExpressionProgram`; `NativeExpressionProgram::compiler_steps`; `NativeExpressionProgramUnavailable`; `NativeExpressionInstruction`; `native_expression_boolean_operator`; `NativeLogicalExpressionCompilation`; `native_logical_expression_compilation`; `native_expression_boolean_word84`; `native_expression_instruction`; `prepare_native_expression_program` | exact counted original expression bytes and direct span mapping with independently selected parser/diagnostic axes; one checked tree or proved syntax rejection, static single SIMPLE_WORD versus original dynamic EXPR_STK operands; original C8.4 fixed-function lookup and child compiler visits stay ordered through `compiler_steps`, independently of command lookup; C8.4 logical preparation registers original pooled 0 then 1 after the left program and before the right program; runtime normalizes the selected left original and preserves eager LAND/LOR versus branch-result recipes; no math-function binding or object-cache authority | `xtask-owner-resolution` |
| original compiled expression number headers | `rust/tcl-bytecode/src/lib.rs`; `rust/tcl-syntax/src/expr/eval.rs`; `rust/tcl-registry/src/native_expression_program.rs`; `rust/tcl-compiler/src/codegen/native_control.rs`; `rust/tcl-vm/src/literal_pool.rs`; `runtime/rust/src/interp/native_body_artifact.rs`; `runtime/rust/src/interp/native_body_artifact/native_control.rs` | `NativeExpressionNumberLiteral`; `NativeExpressionNumberLiteral::number`; `NativeLiteralAllocation::PrivateExpressionNumber`; `LiteralTable::register_private_expression_number`; `NativeLiteralAction::AdoptExpressionNumber`; `LiteralTable::intern_expression_number`; `ExprOps::prepared_node`; `native_expression_boolean_operator`; `native_expression_private_logical_boolean85`; `NativeLiteralAllocation::PrivateLogicalBoolean85`; `LiteralTable::register_private_logical_boolean85` | actual C8.5+ issuer and independently admitted constant operator subtree; exact Integer, Double bits or arbitrary-precision magnitude becomes an absent-string original numeric header in an ordered private slot; resident folded results use chronological cache transfer into the same untyped registered header after deduplication, preserving resident bytes and any earlier typed primary; ordinary Boolean results use registered original zero/one literals; a folded C8.5 logical subtree retains the temporary compiler's same original Boolean header after its registration retires, while C8.6+ resident folds remain registered; prepared-node reuse addresses the exact retained tree before leaves, while ordinary evaluation declines; deferred Syntax packets retain original message/options headers; payloads and mathematical values grant no compiler or object authority | `xtask-owner-resolution` |
| original generic foreach and lmap scheduling | `rust/tcl-runtime-api/src/native_each_loop.rs`; `rust/tcl-registry/src/native_each_loop.rs`; `rust/tcl-cmd-core/src/native_each_loop.rs`; `rust/tcl-vm/src/cmd_control.rs`; `rust/tcl-vm/src/exec.rs`; `runtime/rust/src/cmd_control.rs` | `NativeEachLoopKind`; `NativeEachLoopRecipe`; `NativeEachLoopProtocol`; `native_each_loop_protocol`; `EachLoopState`; `EachLoopState::advance`; `EachLoopState::entered_body`; `EachLoopState::body_completion`; `preparation`; `setter_failure` | actual C/Jim preparation, refresh/setter/body order and completion; cursor advance precedes callbacks, and entered-body progress is recorded only on the Body action; concrete adapters retain original variable/member objects and distinguish lifetime transport from native owners; Jim argv List roots remain borrowed, Foreach owns actual emptyObj as result, and lmap owns its result List while borrowing emptyObj only for padding; native compiler auxiliary storage remains a separate owner | `xtask-owner-resolution` |
| native compiled iterator preparation and storage | `rust/tcl-registry/src/native_each_compilation.rs`; `rust/tcl-bytecode/src/lib.rs` | `NativeEachCollection`; `NativeCompiledEachStorage`; `native_compiled_each_storage`; `NativeEachGroup`; `NativeEachInstruction`; `NativeEachUnavailable`; `compile_native_each`; `NativeEachAuxiliary` | actual C foreach/lmap compiler, original variable-list members/value operands and exact protected body extent; declaration prefix survives decline; real auxiliary slot indices select C8.4 per-setter local refresh, C8.5 per-group List copy, or C8.6+ retained stack lists; value-list/counter temporaries are unnamed physical frame cells, independently of generic iterator handlers | `xtask-owner-resolution` |
| native compiled try handlers and cleanup | `rust/tcl-registry/src/native_try_compilation.rs` | `NativeTryCondition`; `NativeTryCondition::code`; `NativeTryHandler`; `NativeTryInstruction`; `NativeTryUnavailable`; `compile_native_try` | actual C8.6+ compiler and original protected body/handler/finally operands; clauses retain their own result/options bindings and fallthrough body target; release-selected matcher literals/private objects, anonymous completion slots, numeric table and exception ranges belong to compiler preparation; execution preserves original result/options headers, RETURN_STK and original -during dictionaries | `xtask-owner-resolution` |
| original TclOO method records and call-chain caches | `rust/tcl-registry/src/native_tcloo_method_cache.rs`; `rust/tcl-vm/src/cmd_oo/native_method_cache.rs`; `rust/tcl-vm/src/value/native_method_name.rs`; `runtime/rust/src/cmd_oo/native_method_cache.rs`; `runtime/rust/src/obj/native_method_name.rs` | `NativeTclOoMethodCacheProtocol`; `NativeTclOoMethodCacheStamp`; `native_tcloo_method_cache_protocol`; `NativeTclOoMethodCacheProtocol::strings`; `flags`; `reusable` | actual C8.6+ receiver/class creation, Foundation/object mutation epochs and public/private/class-cache flags; actual method tables own mutable records, replacement preserves a record while deletion/recreation creates a new one; idle caches must observe exact original records weakly and actual entered chains retain ALL original records through next; Runtime and VM separate these roles; VM uses its existing neutral original-operation Scope independently of the actual target/cache stamp, and its Value/Rc semantics do not attest physical C clientData counts; same-record replacement changes mutable payload independently of entered-record lifetime, and Body/Forward/Property execution roles require their own original purpose; original table keys and same selector primary own their cache container, duplicate shares it; equal distinct headers and metadata never issue a primary or lookup authority; internal owners: MethodTable, MethodSlot, NativeMethodWorld, NativeMethodChain, ReachedMethodChain | `xtask-owner-resolution` |
| original TclOO entered call-chain reporting | `rust/tcl-registry/src/native_tcloo_method_definition.rs`; `runtime/rust/src/cmd_oo.rs`; `runtime/rust/src/cmd_oo/native_method_cache.rs` | `NativeTclooMethodPayloadType`; `NativeTclooCallReportKind` | actual selected records carry current private flags and payload type; actual chain producers retain filter, unknown, declaring facet and Named/Constructor/Destructor roles independently of selector text; reporting uses the retained record name or genuine Foundation header and the existing selected List producer, with no eager String or method-table relookup; stock installers issue builtin and slot descriptors explicitly; implicit stock dispatch has a selected report terminus without a synthetic Method record or cache grant; unavailable original declaring facets refuse rather than adopting a reborn class; Runtime physical roles, VM value semantics, native Class lifetime and Body/Forward/Property execution obligations remain independent | none |
| native string materialisation | `rust/tcl-syntax/src/native_string.rs`; `rust/tcl-syntax/src/native_tcl_utf.rs`; `rust/tcl-syntax/src/scalar_getter.rs`; `rust/tcl-vm/src/value.rs`; `rust/tcl-vm/src/interp/native_numeric_ops.rs` | `NativeStringProtocol`; `NativeStringInput`; `materialize_native_string`; `NativeStringUnavailable`; `native_string_bytes_checked_for_value`; `OriginalNumericOps::string`; `OriginalNumericOps::string_value` | resident string bytes preserve exact extent independently of ByteArray backing; pure C ByteArray maps each byte through native U+00XX units; Jim pure ByteArray unavailable; object cache installation and physical empty storage remain adapter facts; no numeric/compiler authority | none |
| original native string trim | `rust/tcl-registry/src/native_string_trim_compilation.rs`; `rust/tcl-syntax/src/native_string_trim.rs`; `rust/tcl-cmd-core/src/string/native_trim.rs`; `rust/tcl-cmd-core/src/string.rs`; `rust/tcl-compiler/src/codegen/statements/native_string.rs`; `rust/tcl-vm/src/exec.rs`; `runtime/rust/src/interp/native_body_artifact/native_string_trim.rs` | `NativeStringTrimOperation`; `NativeStringTrimInstruction`; `select_original`; `instruction`; `default_trim_set`; `trim_range`; `string::compiled_trim`; `string::trim` | actual C86+ private compiler registration selects original subject then explicit/default character operand; characters are converted before subject at execution, no-cut instructions preserve the same subject, and effective cuts create fresh NULL-primary string objects; interpreted C commands always create a fresh result; counted native UTF boundaries and release-specific defaults remain pure Syntax geometry, with no header or CPP authority; C84/85 and Jim retain their genuine generic compiler paths | `xtask-owner-resolution` |
| original script bytes and lexical boundaries | `rust/tcl-lexer/src/source_map.rs`; `rust/tcl-lexer/src/lexer.rs`; `rust/tcl-lexer/src/script.rs`; `rust/tcl-lexer/src/parse_cut.rs`; `rust/tcl-lexer/src/native_word.rs`; `rust/tcl-runtime-api/src/lib.rs`; `rust/tcl-bytecode/src/lib.rs`; `rust/tcl-compiler/src/compile_service.rs`; `rust/tcl-compiler/src/command_binding/executed_script_source.rs`; `rust/tcl-compiler/src/parsing/syntax/build.rs`; `rust/tcl-compiler/src/segmenter.rs`; `rust/tcl-compiler/src/codegen/emit.rs` | `SourceImage`; `SourceChannel`; `NativeWord`; `NativeWord::from_group`; `ExecutedScriptSource`; `build_document_image`; `segment_commands_image_with_offset_and_config`; `Emit::emit_command_image`; `SourceMap::from_image`; `source_bytes`; `token_bytes`; `Lexer::with_bytes`; `with_source_image`; `group_commands_bytes`; `first_parse_cut_bytes`; `first_parse_cut_image`; `ScriptCompileTargetBytes`; `ProcedureCompileTargetBytes`; `LocalVarTable::from_names`; `intern_bytes`; `CompileService::script_command_plan_bytes_with_entry` | immutable original bytes and explicit native-value/document channel; selected grammar remains independent of bytes and interpreter authority; exact token spans and constructed namespace segments; actual formal storage keys enter one native LVT; checked Unicode views cannot replace an opaque buffer; unsupported byte services return typed refusal; original byte/channel/name and nested parse-cut controls | none |
| executable substitution arena | `rust/tcl-lexer/src/executable_parts.rs`; `rust/tcl-lexer/src/word_parts.rs`; `rust/tcl-lexer/src/native_word.rs` | `ExecutablePartArena`; `ExecutablePart`; `SpannedExecutablePart`; `ExecutableText`; `PartListId`; `decompose`; `decompose_template`; `NativeWord::executable_parts`; `source_span` | one exact SourceImage, original byte extents, flags and separately selected word/template grammar; existing scanners queue arbitrary index nesting into flat ordered lists; Text distinguishes original from decoded bytes; whole spans and native token spans remain distinct; IDs belong to their arena; syntax errors and geometry refusal remain explicit; no command/value/object/effect authority from lexical geometry; advisory tree budget cannot truncate execution; fixed 2,000-index opaque-byte native control, 64 KiB construction/drop, decoded/raw/token span and Jim template discriminants | none |
| retained native namespace context and source keys | `rust/tcl-runtime-api/src/native_compilation.rs`; `rust/tcl-runtime-api/src/lib.rs`; `rust/tcl-compiler/src/command_binding/namespace_context.rs`; `rust/tcl-bytecode/src/lib.rs`; `rust/tcl-vm/src/interp/native_procedure_artifacts.rs` | `NativeNamespaceContext`; `NativeCompilationEntry::retained_namespace_context`; `NativeCompilationEntry::command_lookup_cursor`; `NativeCommandLookupCursor::next_candidate`; `NativeCommandLookupCandidate`; `CompiledNamespaceContext`; `CompiledNamespaceContext::path`; `CommandBindingIdentity::with_namespace_context`; `ProcedureBindingIdentity::with_namespace_context`; `SourceNamespaceKey`; `SourceNamespaceKey::to_compiled_context`; `SourceNamespaceKey::source_artifact_path`; `Instruction::source_namespace_context`; `NativeOperationSelectionSite::replay_namespace_context`; `ProcedureProvenance::namespace_context` | actual interpreter owner, namespace incarnation and exact component boundaries; optional compiled binding/replay context is authoritative when supplied; Native retains actual identity, ConstructedPath retains geometry without a native token; stop-aware lookup validates fallback only when reached; VM binding checks and replay validate native interpreter/token/path/lifetime without display parsing or substitute lookup; procedure-artifact keys retain exact body component geometry and source identity independently of displayed declaration names; constructed paths partition candidates but native reuse requires the selected actual incarnation, and metadata adds no owner reference or dormant-candidate activation; source_artifact_path supplies component geometry for emitted source artifacts and cannot establish an actual namespace incarnation; authored contexts and source allocations remain distinct; internal implementation: `SourceCommandKey`, `SourceCommandTable`, `SourceNamespaceMap`, `SourceNamespaceSet` | `xtask-owner-resolution` |
| immutable native compiler byte lookup | `rust/tcl-runtime-api/src/native_compilation.rs`; `rust/tcl-compiler/src/native_byte_compilation.rs` | `NativeCompilationEntry::lookup_command_bytes`; `NativeCommandLookupUnavailable` | original bytes and namespace incarnation resolve against one immutable actual snapshot; unique registration, closed absence and typed residual remain distinct; C CString/path/global lookup and Jim namespace-object keys are selected independently; constant heads require absent compiler hooks while dynamic heads never borrow runtime hooks; every bracket child retains its own admission obligation; no runtime handler/effect or opcode authority; opaque names, hook presence, path search, retained deletion and nested-child certificate controls; internal implementation: `closed_generic_byte_compilation` | none |
| native command-source error logging | `rust/tcl-registry/src/native_error_log.rs`; `rust/tcl-syntax/src/native_tcl_utf.rs`; `runtime/rust/src/interp/native_object_vector.rs` | `NativeErrorLogProtocol`; `native_error_log_protocol`; `excerpt`; `observes_error_words`; `object_vector_command`; `command_log_excerpt`; `character_complete`; `Interp::eval_original_object_vector` | actual C release and original counted source; original argument updaters at public object-vector failure, release-selected already-logged observation, C-string formatting extent, native incomplete-character rules and clipping; no Jim, authored-grammar or host UTF-8 fallback; the receiving OriginalArguments operation remains held through native vector entry, reached getters and mandatory frame/List/argv-pin cleanup; the real C error List retains its original children through reached command logging and retires afterward, while release-specific NativeObjectVectorProtocol completion and independent Script-source rules remain unchanged | none |
| runtime command-table identity | `rust/tcl-core-types/src/lib.rs`; `rust/tcl-core-types/src/name_bytes.rs`; `rust/tcl-runtime-api/src/command_identity.rs` | `NameBytes`; `ByteNamespacePath`; `ByteCommandSlot`; `NativeByteCommandSlot`; `CommandSlot`; `StaticNamespacePath`; `StaticCommandSlot`; `display_namespace`; `display_command`; `encode_command_slot`; `CommandId`; `NsId`; `OoId` | invariant; written-name parsing remains selected at ingress by the document/runtime dialect | none |
| VM native command placement | `rust/tcl-vm/src/interp.rs`; `rust/tcl-vm/src/interp/native_name_world.rs`; `rust/tcl-vm/src/embed.rs`; `rust/tcl-runtime-api/src/lib.rs` | `Namespaces::find_command_bytes_checked`; `Namespaces::find_namespace_bytes_checked`; `Namespaces::command_name_bytes`; `Vm::command_names_bytes`; `Vm::retain_commands_bytes`; `Vm::command_names`; `NativePublicationService::prepare`; `Vm::native_publication_service`; `Vm::registered_command_token_bytes` | one interpreter-owned `Rc<RefCell<NativeNameWorld>>`; exact namespace token and `NameBytes` select placement; private String keys are storage indexes, never written names; slot, display and generation queries return owned results before lifecycle callbacks or interpreter switches; reserved slots without generations are absent; token handles cannot follow a same-named replacement; Unicode enumeration returns a typed decoding error for unrepresentable names; Unicode whitelists remove opaque unlisted names; callbacks consume owned entries without table guards; command-trace operands use byte list quoting and the shared source ingress, whose unsupported byte source remains an explicit refusal; internal implementation: `NativeNameWorld`, `command_slot`, `command_storage_key_at_slot`, `live_command_generation`, `command_display_key_bytes`, `command_sidecar_display_bytes`, `resolve_command_bytes_checked`, `namespace_object_lookup`, `native_procedure_publication` | none |
| Runtime command registration and identity | `runtime/rust/src/interp.rs`; `runtime/rust/src/interp/stock_ensembles.rs`; `runtime/rust/src/namespace.rs` | `Interp::register_builtin`; `Interp::register_stock_builtin`; `Interp::register_guarded_builtin`; `Interp::register_spec_builtin`; `Interp::bind_builtin`; `Interp::attested_command_identity`; `Interp::stock_ensemble_registry_identity`; `Interp::stock_ensemble_implementation_is_current`; `Interp::synchronise_command_environment_guards`; `Interp::backing_report` | actual publication generation separates host registration from stock provenance; host spelling grants no stock availability or compiler identity; explicit semantic attestation belongs only to the returned generation; root/worker/map and selected release remain independent current facts; actual namespace mutators share a checked policy revision and exhaustion poisons all three lookup guard domains; original callback-created replacement and first refusal cannot be adopted by a name query | none |
| positioned command navigation | `rust/tcl-compiler/src/command_binding/command_reference.rs`; `rust/tcl-compiler/src/command_binding/linked_definition.rs`; `rust/tcl-compiler/src/signature_scan/types.rs`; `rust/tcl-compiler/src/analyser/types.rs` | `SourceCommandReference`; `SourceCommandReferenceBinding`; `SourceCommandDefinition`; `SourceCommandDefinitionKind`; `command_reference`; `evaluated_command_reference`; `linked_definition`; `written`; `retain_reference`; `clear_positioned_reference`; `proc_for_definition`; `class_for_definition` | exact post-argument slot and current implementation allocation, full source origin and selected grammar; evaluated command receipts are independent of source-exact contributor edits; called definition differs from terminal navigation, observed/opaque alias edges decline; no opcode, normal-completion or object-class licence | none |
| actual native compilation entry and artifact guards | `rust/tcl-runtime-api/src/native_compilation.rs`; `rust/tcl-runtime-api/src/lib.rs`; `rust/tcl-compiler/src/compile_service.rs`; `rust/tcl-compiler/src/codegen/emitter/mod.rs`; `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/command_binding/source_analysis_entry.rs` | `NativeCompilationEntry`; `NativeCompilationEntry::same_compilation_world`; `NativeNamespaceVariableTable`; `NativeCompilationBinding`; `NativeCompilationNamespace`; `NativeCommandImplementation`; `NativeInterpreterIdentity`; `CommandBindingGuard`; `CommandBindingIdentity`; `CompileError`; `CompileService`; `BytecodeCompileService::native_entry_options`; `native_entry_config`; `SourceAnalysisOptions::native_compiler_dialect`; `SourceAnalysisOptions::logical_invocation_dialect`; `SourceAnalysisOptions::native_lexer_config`; `SourceCompilationScope`; `ModuleEmissionScope`; `codegen_module_with_emission_scope`; `codegen_procedure_module_with_emission_scope` | typed runtime targets exclude deferred declaration analysis and lowering before CFG preparation and emit only entered source; explicit AOT retains owned procedure inventory; direct options and adapter share exact live-policy precedence; measured source grammar retains driver coordinates/modes independently from logical absence; independently retained source profile, logical invocation policy and physical engine point; missing logical/physical evidence never borrows the other or editor assistance; original argv materialisation retains logical lexer/word rules; actual interpreter identity and command allocations; alias and namespace-unknown prefix capture retains exact argv presence without materialising stored objects; distinct chunk-entry and before-argument validation; independently captured original namespace variable-root tables contain allocation membership only, including undefined and linked entries; source-cache equality includes these rows without changing native compiler epochs or running-frame freshness; unsupported adapters decline optimized compilation; typed compile-service admission refusal reaches the retained host channel without becoming a guest completion; internal implementation: `compile_script_with_entry`, `compile_procedure_with_entry`; internal implementation: `InterpState::source_profile` | none |
| original namespace-name objects | `rust/tcl-syntax/src/native_namespace_name.rs`; `rust/tcl-registry/src/native_namespace_name.rs`; `rust/tcl-runtime-api/src/native_namespace_name.rs`; `rust/tcl-vm/src/interp/native_namespace_names.rs`; `runtime/rust/src/interp/native_namespace_names.rs`; `runtime/rust/src/namespace/native_namespace_name.rs`; `runtime/rust/src/obj/native_namespace_name.rs`; `rust/tcl-cmd-core/src/namespace.rs`; `rust/tcl-engine-api/src/lib.rs`; `rust/tcl-runtime-api/src/lib.rs`; `rust/tcl-syntax/src/native_glob.rs` | `NativeNamespaceNameRecipe`; `native_namespace_name_protocol`; `authored_namespace_result_recipe`; `NativeNamespaceNameToken`; `NativeNamespaceNameCache`; `NamespaceObjectBackend`; `current_original`; `parent_original`; `children_tokens_checked`; `children_original`; `namespace_objects_original`; `NamespaceDeleteBackend`; `delete_original`; `OriginalObjectResult::NamespaceName`; `namespace_children_lookup`; `Namespaces::find_namespace_child_bytes_checked` | actual original primary, closed namespace incarnation and reference context, physical lifecycle and separate parent detachment; C84 updater and unresolved conversion remain distinct from later no-updater primaries; actual C producers and explicitly authored logical result factories grant separate permissions; serialized bytes and metadata never grant cache authority; internal implementation: `native_namespace_object_lookup` | `xtask-owner-resolution` |
| whole-unit native compiler replay | `rust/tcl-runtime-api/src/native_compiler_pass.rs`; `rust/tcl-bytecode/src/lib.rs`; `rust/tcl-registry/src/native_compiler_pass.rs`; `rust/tcl-compiler/src/codegen/native_compiler_pass.rs`; `rust/tcl-compiler/src/codegen/emitter/mod.rs`; `rust/tcl-vm/src/interp/native_command_names.rs`; `runtime/rust/src/interp/native_body_artifact/native_compiler_pass.rs` | `NativeCompilerPassOwner`; `NativeCompilerPassEnvironment`; `NativeCompilerPassProcedure`; `NativeCompilerPassHazard`; `native_compiler_replays`; `LiteralTable::discarded_native_passes`; `LiteralTable::retain_discarded_native_pass` | actual live interpreter identity, parent and enabled command/time limits; original first-pass emitted opcode classes select C86+ whole-unit replay, while C84/85 retain one pass; only an actual command-owned procedure namespace supplies the `::tcl` library exception, never an anonymous/source candidate; the second configured emission moves the same original local table and appends its actual anonymous slots, omits executable command markers, and retains independent original operation/epoch validation; earlier ordered literal arrays are constructed, primed and released before the final array, without first-array source finalization or a retained pool pin; Runtime validates the independent compiler/context stamp after first-array callbacks; VM final literal arrays retain the planned replay environment and revalidate the same state owner, root and enabled limits after first-array callbacks and release, refusing stale realization through the typed host boundary before final-array allocation; root/limit facts select only reached compilation and are excluded from warm Bytecode cache equality; Runtime discards first-pass executable/literal/auxiliary preparation while retaining the same local table, and materializes the final array; missing/foreign/retired environments refuse replay and metadata supplies no callable/body/numeric authority | `xtask-owner-resolution` |
| native command-object cache and compiler literal actions | `rust/tcl-registry/src/native_command_literal.rs`; `rust/tcl-runtime-api/src/native_command_name.rs`; `rust/tcl-vm/src/interp/native_command_names.rs`; `rust/tcl-bytecode/src/lib.rs`; `rust/tcl-engine-api/src/lib.rs` | `NativeCommandNameProtocol`; `native_command_name_protocol`; `native_compiled_command_literal`; `native_compiled_command_name_literal`; `native_compiled_command_name_literal_from_lookup`; `native_compiled_selected_command_name_literal_from_lookup`; `NativeCommandNamePrimingAuthority`; `cache_is_current`; `preserves_primed_cache`; `invalidates_path_before_object_create`; `NativeLiteralContext`; `NativeCommandNameCache`; `NativeLiteralAction`; `NativeLiteralAction::RetainSyntaxErrorInfo`; `LiteralTable::retain_syntax_error_info`; `LiteralTable::intern_native_command_bytes`; `prime_native_command_name`; `hide_native_literal`; `OriginalObjectResult::CommandName` | actual C release, original object, retained interpreter/namespace incarnation, node and reference epochs; ordered allocation/priming/hiding remains independent of byte deduplication; C91 Syntax retention validates original local message/options array slots and installs that SAME message as the private Dict error-info member; CompilerSelected retains the original public registration/configuration and selected worker, without lookup by its reporting bytes; Jim and explicit logical simulation use selected lookup without C cache; reporting bytes and serialized snapshots grant no live node or original-object authority; internal implementation: `native_command_from_original`, `native_namespace_command_name` | `xtask-owner-resolution` |
| ensemble usage rewrite reset order | `rust/tcl-registry/src/native_ensemble_rewrite.rs`; `runtime/rust/src/interp/stock_ensembles.rs` | `NativeEnsembleRewriteProtocol`; `EnsembleRewriteResetEvent`; `native_ensemble_rewrite_protocol`; `ensemble_rewrite_protocol` | actual engine authority; C8.5 clears after successful ordinary lookup, C8.6+ before lookup and bytecode entry; internal invocation preserves, parser usage adaptation has separate exact-object scoped lifetime; C8.4/Jim have no C rewrite state, explicit logical provider grants no native authority | none |
| original Boolean operand purposes and ordered effects | `rust/tcl-syntax/src/native_boolean_truth.rs`; `rust/tcl-registry/src/native_boolean_truth.rs`; `rust/tcl-cmd-core/src/native_boolean_truth.rs`; `rust/tcl-syntax/src/expr/eval.rs`; `runtime/rust/src/typed_value/native_boolean_truth.rs`; `runtime/rust/src/codegen_abi.rs` | `NativeBooleanTruthPurpose`; `NativeBooleanTruthProtocol`; `NativeBooleanExpressionResultProduction`; `NativeBooleanExpressionResultProtocol`; `native_boolean_truth_protocol`; `native_boolean_expression_result_protocol`; `native_logical_final_operand_purpose`; `NativeBooleanTruthOps`; `original_boolean_truth`; `NativeBooleanExpressionResultOps`; `original_boolean_expression_result`; `original_logical_operand_failure`; `ExprOps::to_bool_for_purpose`; `ExprOps::logical_right_truth`; `tcl_value_get_bool_for_purpose`; `tcl_value_get_expression_bool` | actual selected scalar engine and reached instruction purpose independently of mathematical truth, original object lifetime and expression-result production; C8.4 logical-not, conditional jump and LAND/LOR have separate cache/String/probe recipes; Jim original terms publish before ExprBool and intermediate failed getters publish before later probes and can remain observable after success; original first Host cause wins before each later probe/publication, with the final guest record returned for one publication; the [checked current Tcl 8.6.18/9.0.4/9.1.0 optimizer source interpretation](../analysis/name-resolution-proofs/numeric-current-optimizer-boolean-result-production.md) retains listed next-consumer rewrites and the 9.1 build guard without observed opcode or source-to-library build equivalence; public expression API copy and inline result conversion are separate producer descriptors, the concrete result worker performs the reached neutral numeric probes, current original COW and API copy before returning its owned operand; one instruction diagnostic worker settles actual Number, Dictionary, length, String and List operations; the shared continuation retains original lhs through rhs and dispatches explicit purpose, while tags and pure recipes grant no physical object or completed normalisation | none |
| selected primitive scalar getter | `rust/tcl-syntax/src/scalar_getter.rs`; `rust/tcl-syntax/src/scalar_getter/float.rs`; `rust/tcl-syntax/src/scalar_getter/errors.rs`; `rust/tcl-syntax/src/native_tcl_utf.rs`; `rust/tcl-registry/src/invocation_words.rs`; `rust/tcl-registry/src/native_numeric_error.rs`; `rust/tcl-registry/src/native_numeric_error/command.rs` | `native_scalar_getter_protocol`; `NativeScalarGetterProtocol`; `materialize`; `cached_conversion`; `fresh_conversion`; `fresh_conversion_with_range_error`; `NativeScalarGetterConversion::into_parts`; `failure_presentation`; `NativeScalarGetterErrorCode`; `NativeTclUtf`; `expression_operand_error_code_policy`; `native_numeric_command_protocol`; `NativeNumericCommandPurpose`; `NativeReadCountProtocol` | actual native engine/build and independent getter kind, original RawString/ByteArray materialization, full cache magnitude and preparation before mutation; Jim signed-boundary Wide requires retained native range state or abstains; primitive getter presentation and expression-stage codes remain separate; no parser, effects, expression source, numeric constant or opcode authority | none |
| conditional original expression advice | `rust/tcl-registry/src/conditional_expression.rs`; `rust/tcl-compiler/src/command_binding/normal_result.rs`; `rust/tcl-compiler/src/type_infer.rs`; `rust/tcl-registry/src/commands/tcl/string_.rs` | `ConditionalExpressionEvaluation`; `ConditionalExpressionEvaluation::prepare`; `ConditionalExpressionEvaluation::tree`; `ConditionalExpressionEvaluation::lexer_grammar`; `ConditionalExpressionEvaluation::context`; `ConditionalExpressionEvaluation::numeric_reentry_is_idempotent`; `ConditionalExpressionEvaluation::normal_result_representation`; `ConditionalExpressionPoolState`; `string_is_member_type_for_dialect` | checked original syntax and retained input occurrences; OriginalSource analytical advice meets only same-original source/frame/namespace issuers, while handler-time EvaluatedSource or materialised observations neither donate nor invalidate it; conditional normal representation is separate from reached completion, PreparedExpressionWitness, fixed math registration, native CPP and physical headers; source templates retain Conditional reachability; TypeInfer uses the original stamped source/declaration receipt for result, AssignExpr and Return advice; current authored numeric-pool state meets independently of stock object class and withdraws on representation-changing getters, numeric command-name conversion or unknown callbacks; unknown functions/scripts and unsupported syntax decline; selected original StringIs and written sole-variable origin under actual metadata; Tcl family/release class policy is independent of numeral grammar; unavailable class, failindex and Jim refuse this type projection, with no successful Native intrep/cache/frame/store effect | `xtask-owner-resolution` |
| actual native numeric thread state | `rust/tcl-platform/src/numeric_environment.rs`; `rust/tcl-cmd-core/src/native_numeric.rs`; `rust/tcl-syntax/src/expr/errors.rs` | `NumericEnvironment`; `NumericErrorState`; `c84_nonfinite_error`; `fresh_c84_conversion`; `NativeFloatError::classify` | reached actual host errno, independent EDOM/ERANGE facts, selected reset and unsigned/double calls; original cache lookup precedes fresh conversion; finite/NaN observations preserve thread state, infinity requires authentic domain/range state; raw unknown errno stays unknown, and absent capability refuses rather than inventing baseline state; both normalizers use this owner and selected C8.4 arithmetic error publication retains an actual String header | `xtask-owner-resolution` |
| reached original numeric operand cache | `rust/tcl-registry/src/native_numeric_conversion.rs`; `rust/tcl-registry/src/runtime_expr_validation.rs` | `integer_relational_operand_conversion`; `integer_index_operand_conversion`; `NativeOperandNumericCacheProduction`; `with_original_cache_class` | retained prepared C8.5+ numeric `<` getter branch or selected runtime scalar index stage, exact original object/current cache class and successful integer-only conversion; Double preservation differs from fresh String/List/ByteArray conversion, unknown class stays Numeric; immediate bytecode indices and grouped children cannot donate original cache; no canonical-string, value, effects or opcode proof | none |
| native integer arithmetic | `rust/tcl-dialect/src/arithmetic.rs`; `rust/tcl-registry/src/invocation_words.rs`; `rust/tcl-syntax/src/expr/wide.rs`; `rust/tcl-syntax/src/number_tower.rs`; `rust/tcl-compiler/src/tcl_expr_eval.rs` | `NativeArithmetic`; `arithmetic`; `normalizes_expression_result`; `WideError`; `literal`; `binary`; `unary`; `BigIntOps`; `FoldPolicy`; `for_retained_entry`; `with_invocation_dialect`; `InvocationDialect::characters` | selected native integer tower and embedding character model independently of catalogue and numeral grammar; retained lexer overlay does not replace native engine axes; explicit undefined-native outcomes and unchanged-string results | none |
| native double string representation | `rust/tcl-dialect/src/double_string.rs`; `rust/tcl-syntax/src/number.rs`; `rust/tcl-registry/src/special_vars.rs`; `rust/tcl-vm/src/value.rs`; `rust/tcl-vm/src/interp.rs`; `rust/tcl-registry/src/invocation_words.rs`; `rust/tcl-dialect/src/profile.rs` | `DoubleStringPolicy`; `DoubleFormat`; `double_string_policy`; `format_double_selected`; `parse_double_precision`; `SpecialVariableHook`; `name_in`; `changes_read_value` | exact native engine identity; thread-shared C8.x precision and lazy first string materialisation; C9 shortest and Jim fixed twelve digits; implicit read/write hooks are storage-identity observers; internal implementation: `DoubleFormatContext`, `native_double`, `with_native_double_format`, `double_representation` | none |
| selected script and prefix source navigation | `rust/tcl-registry/src/selected_script_timing.rs`; `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-registry/src/invocation_words.rs`; `rust/tcl-registry/src/spec.rs`; `rust/tcl-compiler/src/registry_invocation.rs` | `InvocationFacts::deferred_script_argument_indices`; `ResolvedInvocation::authored_source_script_arguments`; `authored_logical_source_script_arguments`; `authored_source_command_prefix_arguments`; `ScriptTimingResolver`; `CommandPrefixArguments::from_invocation_arguments` | retained selected command/subcommand/form, structured actual argument facts, exact cardinality, independently selected Native/Logical source roles and available option grammar; unknown ordinary payloads keep selected positions, while unknown selectors, expanded count, option widths or concatenated payloads retain unknown coverage; ReferenceOnly supplies no executable source position; actual ingress supplies no compatibility spelling stand-in; mapped original operands are navigation metadata only, never callback absence, future entry, erasure or edit-completeness permission | none |
| typed callback input metadata | `rust/tcl-registry/src/registry.rs`; `rust/tcl-compiler/src/taint.rs`; `rust/tcl-compiler/src/registry_invocation.rs` | `callback_taint_inputs_words`; `instance_callback_taint_inputs_words`; `callback_taint_inputs`; `ResolvedStatementInvocation::with_argument_words` | exact evaluated argv cardinality and selected deferred positional body; unknown captured prefixes preserve slots; unproved option layout or expansion abstains without placeholder bytes | none |
| checked completion transport | `rust/tcl-runtime-api/src/lib.rs`; `runtime/rust/src/interp.rs`; `runtime/rust/src/completion.rs` | `Commands::dispatch`; `Commands::dispatch_id`; `Interp::eval_completion`; `Interp::eval_sourced_completion` | Guest completion codes and first Host refusal remain independent; public Runtime entry establishes original scope before byte-preserving result/options capture, selected String access and owning-reference retirement with currency checks; internal implementation: `completion::capture_bytes`, not a public entry or permission | `xtask-owner-resolution` |
| neutral native execution refusal | `rust/tcl-runtime-api/src/lib.rs`; `rust/tcl-runtime-api/src/native_execution_error.rs`; `runtime/rust/src/interp/native_compilation.rs`; `rust/tcl-vm/src/interp.rs`; `rust/tcl-vm/src/exec.rs`; `rust/tcl-vm/src/embed.rs`; `rust/tcl-engine-api/src/lib.rs`; `rust/tcl-engine-tclvm/src/lib.rs` | `NativeExecutionError`; `NativeExpressionRefusal`; `NativeExpressionFailure`; `CompileError::Message`; `CompileError::NativeCompilationAdmission`; `CompileError::Unsupported`; `NativeCompileServiceRefusal`; `NativeHostCommandRefusal`; `refuse_host_command`; `EngineError`; `EngineError::ExecutionFailure`; `EngineError::native_execution_failure`; `VmCompilationError`; `try_compile_function`; `try_define_procedure`; `try_invoke_function`; `try_eval_source`; `try_eval_expr`; `try_invoke_command`; `try_run_module`; `try_run_function` | unresolved native compilation admission and reached unsupported expression domains remain host errors; immutable expression bytes/full profile/frame/interpreter/namespace diagnostic context, without a resumable continuation; engine retention survives interpreter switches; prior effects stay applied, guest catch/try/finally, completion writes and traces cannot consume refusal; Runtime host-refusal display observes only an already available namespace coordinate and otherwise reports explicit unavailability, without acquiring an original namespace getter; actual interpreter/frame/token identity and the first refusal remain independent of display bytes | none |
| original pre-handler argument rejection | `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/command_binding/pre_handler_failure.rs`; `rust/tcl-compiler/src/registry_invocation.rs` | `SourceCommandBindings` | internal implementation: `original_arguments_rejected_before_handler`, `logical_structured_invocation`; unchanged original words and source occurrence, exact typed namespace/frame and actual-only all-Error operand routes without normal continuation; actual handler dispatch or unrepresented possible body entry withdraws normal-transfer rejection; unknown namespace uncertainty covers every context, exact known keys filter selected views; declaration previews supply neither actual rejection nor uncertainty; original AST/declaration grammar remains analytical without physical entry, CPP, effects or read-value authority | none |
| source and IR command bindings | `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/command_binding/source_analysis_entry.rs`; `rust/tcl-compiler/src/command_binding/retained_source.rs`; `rust/tcl-compiler/src/command_binding/body_template.rs`; `rust/tcl-compiler/src/command_binding/compiler_inventory.rs`; `rust/tcl-compiler/src/realm.rs`; `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/command_binding/compiled_invocation.rs` | `SourceCommandBindings`; `RetainedSourceModuleBindings`; `SourceAnalysisOptions`; `SourceAnalysisEntry`; `TrustedPackageLoader`; `SourceInvocationBinding`; `proved_class_definition_factory`; `SourceRuntimeReachability`; `native_compiler_dialect`; `native_compilation_admission_selection`; `SourceCommandTarget`; `SourceCommandTarget::registry_identity`; `analyse_command_binding`; `CommandBindingRealm`; `variable_accesses_for_invocation_args`; `document_realm_bindings`; `document_realm_bindings_with_config`; `restore_script_bindings`; `BodyProofScope`; `NativeBodyTemplate`; `BodySourceProofs`; `prepare_native_body_template`; `restore_cfg`; `restore_statement`; `for_each_statement`; `for_each_script_mut`; `child_scripts`; `child_scripts_mut`; `tokens_mut` | selected runtime family, positioned execution namespace, exact invocation offsets, separate compiler coverage and runtime reachability, explicit entry and loader contracts; registry_identity retains the original semantic descriptor identifier independently of renamed/imported live slots and grants no lookup spelling; compiler-only recipes cannot provide reached effects; editor projections retain uncertainty; immutable lowering projections are reused only with exact source image, grammar, registry semantic key, entry policy, compilation scope and frame identity; internal implementation: `source_analysis_frame` | none |
| positioned command-slot advice | `rust/tcl-compiler/src/command_binding/command_presence.rs`; `rust/tcl-compiler/src/command_binding/future_bodies.rs` | `SourceCommandSlotPresence`; `selected_slot_presence`; `selected_slot_diagnostic_presence`; `diagnostic_slot_presence_at`; `SourceFutureBodyInventory::invocation_at` | exact evaluated head, raw alias/import wrapper, independent namespace paths and retained document origin; default autoload possibility remains separate, custom/unknown fallback withdraws absence advice; no execution or registry-key licence | none |
| declared command assistance | `rust/tcl-registry/src/model/declaration.rs` | `DeclaredCommand`; `DeclaredSurface`; `DocumentCommandSurface`; `declared_command`; `declared_names` | authored argument shape and retained trust provenance; immutable entry contracts retain the raw declaration set only for the surface; inventory and contract reads use the surface; slot applicability comes from the source owner, independently of runtime command presence | none |
| original declaration navigation | `rust/tcl-compiler/src/registry_invocation/declaration_assistance.rs`; `rust/tcl-compiler/src/command_binding/declaration_layout.rs`; `rust/tcl-compiler/src/realm.rs` | `OriginalDeclarationAssistance`; `DeclarationArgument`; `CommandBindingRealm::original_declaration_assistance`; `stamp_original_tokens` | unchanged source and retained declaration frame; only OriginalDeclaration layouts supply declaration advice, with unanimous allocation, words, grammar and typed context; EnteredActivation observations remain independent and cannot donate or veto declaration semantics; frozen alias arguments and mapped written positions; no prefix-only or expansion token; no execution, physical cell or store authority | none |
| effective invocation words | `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/word_subst.rs` | `EffectiveCommandWords`; `InvocationWordOrigin`; `ResolvedStatementInvocation`; `RegistryInvocationAssistance`; `RegistryInvocationShape`; `written_argument_roles`; `invocation_argument_role_assistance`; `invocation_argument_role_consensus`; `CatalogueInvocationAssistance`; `registry_invocation_assistance`; `catalogue_invocation_assistance`; `DeclaredInvocationAssistance`; `resolved_declared_assistance`; `resolved_statement_invocation`; `resolved_tokens_invocation`; `effective_command_words`; `argument_spellings`; `argument_presentations`; `with_argument_words`; `argument_literal`; `written_argument`; `resolve_command_tokens`; `resolve_word_exprs`; `resolve_word_exprs_with_dialect`; `inherit_nested_bindings`; `lifted_source_expressions` | selected escape/list/brace rules and implementation provenance; statement facts and effective arguments share one proof; alias values have no written argument span; role assistance unions retained candidates and applicable declarations without granting execution or contents reads; definite grammar roles require every actual candidate, a complete accepted argv layout and no opaque or absent residual; selected invocation role mapping retains its existing proof strength; internal implementation: `evaluated_arguments`, `source_expression_from_invocation`; private body-assistance facade maps original operand roles, candidate case-list/definition grammars and nominal loop operations for navigation, while scoped definitions and alias/value-copy summaries require its separate accepted definite invocation; offset-free assistance explicitly withdraws every definite field; name-effect traversal uses original lifted child receipts and preserves an opaque residual for unsupported or truncated substitution inventories, while braced data never becomes a script; internal implementation: `checked_lifted_calls` | none |
| evaluated expression source | `rust/tcl-compiler/src/command_binding/executed_expression_source.rs`; `rust/tcl-compiler/src/ssa.rs` | `ExecutedExpressionSource::from_arguments`; `ExecutedExpressionSource::variable_source`; `SsaSourceView::read_executed_expression_variable` | selected native expression argument grammar and shared concatenation; exact piecewise original operand read sites; decoded, captured, expanded and cross-piece references have no authored-site proof; grants neither compiler preparation nor source edit eligibility | none |
| nested normal handler lookup | `rust/tcl-registry/src/native_handler_path.rs`; `rust/tcl-registry/src/native_compilation.rs` | `NativeHandlerLookupPaths::select`; `NormalHandlerImplementationLookup::RequiredPath`; `SuccessfulHandlerSpec::stock_native_workers` | exact frozen selector and cardinality choose an authored ordered ensemble path; each intermediate map, command token and terminal worker requires independent live identity proof; stock inventory enumerates every eligible edge without selecting a path; neither projection supplies compiler operation, hook, result or observer proof; native handler assertions are explicitly excluded by SpecTcl and Studio | none |
| temporal constructor entry | `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/command_binding/deferred_method.rs`; `rust/tcl-registry/src/definer.rs` | `SourceConstructorEntry`; `SourceInvocationBinding::constructor_entry`; `is_instance_constructor`; `preserves_local_constructor_entry` | current class incarnation and original constructor declaration, formals, evaluated body and receiver frame; original native manufacturer payload and shared formal arity; later redefinition, argument callbacks and unknown inherited/filter chains abstain; advisory parameter edges confer no manufactured class or completion proof | none |
| temporal receiver navigation | `rust/tcl-compiler/src/command_binding/receiver_self.rs`; `rust/tcl-compiler/src/command_binding/object_instance.rs`; `rust/tcl-compiler/src/command_binding/deferred_method.rs`; `rust/tcl-lsp-core/src/receiver_identity.rs`; `rust/tcl-compiler/src/registry_invocation/source_receiver_names.rs`; `rust/tcl-compiler/src/command_binding/original_receiver_body_context.rs`; `rust/tcl-compiler/src/compilation_unit.rs`; `rust/tcl-compiler/src/analyser/handlers.rs` | `SourceReceiverMethodEntry`; `is_exported`; `SourceMethodReceiver`; `class_definition_method_entries`; `object_receiver_method_entry`; `named_object_receiver_method_entry`; `receiver_self_method_entry`; `frozen_object_receiver_method_entry`; `SourceReceiverNameOperand`; `original_source_receiver_name_operand`; `OriginalReceiverBodyContext` | exact class allocation; internal consumer projections: `original_class`, `method_at_command`, `method_at_cursor`, `class_at_read` and original member source/frame and declaring class independently of actual receiver class; static byte-exact selector required for edits, separate Class/Instance tables, original variable-object read or installed named command retained through argv; actual analysis realm and lexer map only original local declarations; definition, hover, references, rename and type-definition share the projection; nominal completion and absent method-name inventories supply no editing or execution licence; original receiver syntax distinguishes literal name data from one actual scalar substitution, refusing expansion, compound words and arrays; genuine entered receiver context retains declaring provider allocation, activation/formal frame and source independently of class labels or declaration previews | none |
| original definition method-name operands | `rust/tcl-compiler/src/command_binding/definition_method_references.rs`; `rust/tcl-compiler/src/command_binding/definition_reference_inventory.rs`; `rust/tcl-compiler/src/command_binding/deferred_method.rs` | `SourceDefinitionMethodReference`; `definition_method_reference_inventories`; `definition_method_references_for_class`; `definition_method_reference_at` | accepted native class allocation and incarnation; original private worker and unchanged source operand; Instance/Class receiver and original method declaration at that definition phase; missing declarations never borrow later methods; complete empty coverage is distinct from unknown/conflicting coverage; retained original source survives class retirement and supplies no dispatch, visibility, body entry, result or compiler licence | none |
| receiver-local builtin hazards | `rust/tcl-compiler/src/command_binding/receiver_self.rs`; `rust/tcl-registry/src/definer.rs` | `SourceReceiverBuiltinCandidates`; `receiver_self_builtin_candidates`; `BuiltinObjectMethodOperation::ObjectVariableLinks` | exact original receiver frame, native release/reach policy and current class/base dependencies; actual allocation and private-dispatcher generation are separate from generic May residual; declared overrides exclude the builtin; no normal alias, visibility, completion or compiler licence | none |
| normal allocated object links | `rust/tcl-compiler/src/command_binding/receiver_self.rs`; `rust/tcl-compiler/src/allocated_instance.rs`; `rust/tcl-compiler/src/place.rs` | `link_allocated_instance_variables`; `retire_allocated_instance_variables`; `CellOwner::AllocatedInstance` | actual reached receiving allocation, original private dispatcher and current class/base dependencies select the authored native variable-link operation; sequential links preserve partial errors, unknown destinations remain opaque, typed heap cells survive frame return; generic deferred receiver inventory cannot supply normal aliases | none |
| native iteration entry | `rust/tcl-registry/src/iteration_entry.rs`; `rust/tcl-compiler/src/command_binding/source_loop.rs` | `InvocationFacts::iteration_entry`; `IterationEntry` | actual selected paired-list operation, frozen cardinality and native list parsing distinguish empty, required, invalid and unknown entry; sequential physical stores retain their own errors; mandatory first normal/continue successor precedes the repetition join; no compiler, callback or variable-value licence | none |
| conditional native variable outputs | `rust/tcl-registry/src/variable_output.rs`; `rust/tcl-compiler/src/variable_bindings.rs`; `rust/tcl-compiler/src/command_binding/variable_outputs.rs` | `NativeVariableOutputSpec`; `VariableOutputCommitment`; `successful_variable_output_commitments` | authored selected native handler, exact accepted argv and bounded native matching distinguish written, unchanged and conditional output slots; actual ordered physical addresses, observers and partial errors remain independent obligations; no stored value, opcode or unconditional handler licence | none |
| native expression result recipes | `rust/tcl-registry/src/runtime_expr_validation.rs` | `native_expression_constant_pooling`; `PreparedExpressionWitness::numeric_result_recipe`; `native_expression_pooled_subtrees` | actual native compiler syntax selects pooled versus executed/normalised result instructions; source numeric provenance and codegen share this policy; maximal constant subtrees supply coercion footprints only; mathematical constants and pooled values establish no current object representation | none |
| normalised native numeric result strings | `rust/tcl-syntax/src/number.rs`; `rust/tcl-registry/src/runtime_expr_validation.rs`; `rust/tcl-compiler/src/native_numeric.rs`; `rust/tcl-compiler/src/var_resolve.rs` | `canonical_numeric_bytes_may_equal`; `NativeExpressionNumericResultProduction::normalises_result_string` | private selected setter/new-object recipe, separate from current numeric category; conservative native output language includes signed zero, NaN and infinity; only sharing disjointness, no value, exact bytes, object identity or erasure; unknown/overlapping bytes retire shape and every coercion still advances frozen-receipt epoch; unary forwarding and exponentiation shortcuts decline; internal implementation: `SourceNativeNumericShape::string_bytes_may_equal` | none |
| closed integer conversion contents | `rust/tcl-compiler/src/native_numeric.rs`; `rust/tcl-compiler/src/var_resolve.rs`; `rust/tcl-compiler/src/increment_rewrite.rs` | `SourceIntegerConvertibleContents` | sealed fresh ordinary source-pool cache/string consistency or actual Integer producers, closed under the same selected C bignum protocol; successful live physical read; conversion/effects only, strict representation/value/object unknown; Double/custom/missing/incompatible axes and generic coercions withdraw; O114 still requires exact original read/store/handler schedule; direct Increment needs current Integer, otherwise the audited C bignum unary recipe retains original shared-input normalisation and validates new expression preparation/dependencies; unknown incoming class/lineage cannot license Increment; cache subtype remains independent from retained contents acceptance; internal implementation: `contents_integer_increment_conversion_at` | none |
| original complete argument evaluation | `rust/tcl-compiler/src/command_binding/expression_preparation.rs`; `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/optimiser/elimination.rs` | `original_arguments_complete_normally` | privately certified literal/scalar-read and selected total C bignum arithmetic evaluation, through the original source/formal binder and callee frame; exact written words and unobserved argv boundary; no SSA definitions, result value/object, handler success or purity grant; unknown reads, callbacks, noninteger operands, division/shifts, expansion, conflicting paths and relocation withdraw; dead-store consumers retain independent effect and observation gates | none |
| nested native arithmetic normalization | `rust/tcl-registry/src/runtime_expr_validation.rs`; `rust/tcl-compiler/src/math_function_binding.rs` | `numeric_reentry_is_idempotent`; `ExpressionMathBindings::for_module_statement` | exact original outer/inner C preparation and native Expr handler envelopes; a fresh inner arithmetic normal result has idempotent outer numeric normalization; conditional result only, no totality or value; inner errors remain; script/call/string-forwarding recipes and unknown observers decline; internal implementation: `nested_numeric_normalisation` | none |
| original read/store observations | `rust/tcl-compiler/src/command_binding/read_store_schedule.rs`; `rust/tcl-compiler/src/increment_rewrite.rs` | `SourceInvocationBinding::sole_rhs_read_store_observations`; `SourceReadStoreObservation` | privately captured full expression/operand source identity, known scalar address and distinct pre-RHS/pre-write contexts for every evaluation; address/order only, no conversion/category/value/erasure grant; missing/conflicting coverage, unknown generation, callbacks, relocation, templates and another destination withdraw; consumers match every original read and prospective setter, never a first alternative or equal unknown addresses | none |
| deferred captured-prefix source references | `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-lsp-core/src/receiver_identity.rs` | `deferred_script_argument_indices`; `deferred_script_source_argument_indices` | selected structured timing and original written index mapping, `None` unknown; actual captured receiver/member/source selector receipt; references and edits only, no future callback dispatch or call-graph edge; empty source projection cannot close alias-prefix residual; captured references require retained registration and role/value/dispatcher identity; disagreements withdraw the projection; internal implementation: `captured_method_reference_spans` | none |
| named external cell dependencies | `rust/tcl-compiler/src/ssa.rs` | `SsaSourceView::externally_mutable_by` | canonical storage keys and exact point source bindings match named alias/observer dependencies; display labels never identify storage; absent point evidence with nonempty dependencies remains unknown; SCCP and type inference share the query; internal SSA owner; no SpecTcl surface | none |
| native missing-command fallback | `rust/tcl-registry/src/command_lookup.rs` | `CommandLookupOrigin`; `UnknownHandlerNamespace`; `NativeLookupFallbackPolicy`; `native_lookup_fallback_policy` | actual native interpreter protocol and release, distinct caller/lookup namespaces; C8.5 alias uses lookup namespace, later C uses caller for handler selection; handler head always resolves in lookup namespace; F5 authored 8.4 interpreter contract is independent of its unknown compiler hooks; actual C five-release/Jim VM alias matrix | none |
| native captured variable updates | `rust/tcl-registry/src/native_rmw.rs`; `rust/tcl-registry/src/native_numeric_error.rs`; `rust/tcl-vm/src/interp.rs`; `runtime/rust/src/interp/captured_rmw.rs`; `runtime/rust/src/frame/rmw.rs` | `NativeRmwOperation`; `NativeRmwReadPolicy`; `NativeRmwReadFailure`; `NativeRmwAmountGrammar`; `NativeRmwAmountValidation`; `NativeRmwFailureErrorCode`; `NativeIntegerErrorPresentation` | retain the selected physical receiver through callbacks; scalar-root reset, element retirement and namespace retirement differ; C8.4 validates the amount before reads, modern C converts contents first; Jim amounts use safe integer expressions; frozen root generations govern trace reentry; internal implementation: `increment_captured`, `CapturedVariableUpdate` | none |
| native scalar numeric input bytes | `rust/tcl-syntax/src/number.rs`; `rust/tcl-registry/src/invocation_words.rs`; `runtime/rust/src/value_ops.rs`; `rust/tcl-vm/src/value.rs` | `NativeScalarNumericInputPolicy`; `InvocationDialect::scalar_numeric_input_policy`; `for_point`; `input_bytes` | actual native C family/version or exact core point; C byte projection applies to guest-object inputs; raw host strings require independently selected storage/materialization; Jim084 first-NUL prefix before checked Unicode access; original bytes and object/cache identity remain retained; unknown/foreign engines use a host capability refusal; no expression-source, equality, membership, names or error-presentation permission | none |
| native expression conversion error formatting | `rust/tcl-registry/src/native_numeric_error.rs` | `NativeExpressionOperandStage`; `NativeExpressionOperandErrorPresentation`; `InvocationDialect::expression_operand_error_presentation`; `non_numeric_unary_message` | selected failed conversion stage and original bytes; exact Jim0.84 only, C/unknown abstain; high bytes retained, native first-NUL formatting, guest NONE code; no acceptance, evaluation or host-refusal conversion; pinned floating/integer/boolean and raw-NUL native controls; unary +/- boolean-success nonnumeric errors use the separate operator-specific presenter | none |
| staged variable destruction | `rust/tcl-runtime-api/src/variable_destruction.rs`; `rust/tcl-registry/src/native_variable_destruction.rs`; `runtime/rust/src/frame/destruction.rs`; `runtime/rust/src/interp.rs`; `rust/tcl-vm/src/interp.rs` | `VariableDestructionPhase`; `VariableDestructionProtocol`; `InvocationDialect::variable_destruction_protocol`; `RetainedArrayCell` | C distinct arrays detach the root lookup and root registrations before root callbacks; original member values and trace guards survive until each member retires immediately before its frozen callbacks; recreated names and registrations cannot redirect old receivers; table member order grants no portable source topology; internal implementation: `begin_array_destruction` | none |
| explicit authored TMM static publication | `rust/tcl-runtime-api/src/authored_tmm.rs`; `rust/tcl-runtime-api/src/native_compilation.rs`; `rust/tcl-vm/src/interp/authored_tmm_static.rs` | `AuthoredTmmStaticPolicy`; `AuthoredTmmWorkerTopology`; `TmmStaticExecutionContext`; `AuthoredTmmStaticRecipient`; `AuthoredTmmStaticCompilationContext`; `Vm::install_irules_static_simulation` | explicit logical RULE_INIT publication independent of physical engine/source/naming providers; enrolled actual namespaces and selected topology, resolved alias/attached member cells, ordered original Values and real journal storage; ordinary globals/event/timer stores remain per worker; recipient callbacks retain normal effects/errors; compilation entry retains event context, recipient namespace/observer epochs and outward observer class, with missing context retaining unknown effects; no concrete Runtime TMM backend or F5 appliance proof | `xtask-owner-resolution` |
| variable cell identity and overlap | `rust/tcl-compiler/src/place.rs`; `rust/tcl-compiler/src/var_resolve.rs`; `rust/tcl-compiler/src/raw_binding.rs`; `rust/tcl-compiler/src/allocated_instance.rs`; `rust/tcl-compiler/src/var_resolve/cell_key.rs` | `RawBindingArena`; `RawBindingSlotId`; `capture_callable_statics`; `install_callable_statics`; `retarget_raw_binding`; `raw_static_unset_error`; `detach_retained_binding`; `AllocatedInstanceLinkOutcome`; `link_allocated_instance_variables`; `retire_allocated_instance_variables`; `CellOwner`; `CellGeneration`; `CellIdentity`; `Place`; `overlap`; `places_read_to_form`; `ResolveContext`; `ContentsOrigin`; `VariableExecutionFrame`; `NamespaceCellPresence`; `canonical_variable_name`; `canonical_namespace_variable`; `VariableCellKey`; `VariableCellKeyQuery`; `VariableCellTable`; `VariableCellSet`; `VariableNamespaceSet`; `compatibility_name`; `canonical_binding_value_key`; `canonical_place_key`; `canonical_literal_variable_key`; `namespace_contains`; `cell_key`; `resolve_place`; `resolve_access`; `resolve_literal_access`; `resolve_substitution_access`; `resolve_target_access`; `resolve_alias_destination_slot`; `enter_called_frame`; `selected_frame_context`; `restore_execution_frame`; `project_access`; `resolve_dict_path` | selected Tcl/Jim lookup policy, actual activation owner, cell lifetime and trace observation; `ReceiverMethod` has a known procedure-local activation while receiver namespace/instance ownership remains unknown; actual bounded First/Second object allocations may separately supply typed instance-cell owners, sequential local links and exact-owner retirement; fresh actual activations and reached allocations retain independent exact-owner observer closure despite unknown outward namespace registrations; unknown registrations, callbacks or unresolved writes withdraw these receipts, and joins retain unanimous live owners; no contents, representation or compiler permission is donated; outward frame restoration and alpha relocation preserve those cells from retained typed slots, never class or object names; native NamespaceIdentity retains actual interpreter/incarnation/components; primary fact queries require exact typed keys, while string queries address only Authored storage; compatibility names are diagnostic presentation and cannot recover native, retained, activation or lifetime identities | none |
| point-specific variable bindings | `rust/tcl-compiler/src/variable_bindings.rs`; `rust/tcl-compiler/src/place_bridge.rs` | `PointResolveContexts`; `build_point_resolve_contexts`; `build_point_resolve_contexts_with_entry`; `before_statement`; `after_statement`; `before_terminator`; `source_tokens_at`; `transfer_statement`; `read_places`; `def_places`; `write_observed_by_unknown_access`; `uncertain_variable_output_addresses`; `source_variable_write_places_with_output_order`; `transfer_source_namespace_cells_with_output_order` | registry-owned binding/frame/trace transitions and CFG successful/abrupt joins under the selected entry; actual selected output order can invalidate later physical destinations after an earlier observer, while unknown order retains every possibility and namespace destruction remains a physical clobber without an invented SSA value | none |
| original statement source carriers | `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/variable_bindings.rs` | `retained_source_tokens_for_statement`; `source_input_tokens_at` | owned Script statements or exact CFG block/index; one private selection helper requires original direct and consumed carriers to agree; typed synthetic evaluation phases retain their own explicit input carrier independently of runtime replay sites; Script assistance rejects synthetic phases | none |
| structured source word references | `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/word_expr.rs`; `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/native_lowering/cells.rs` | `SourceVariableAccess`; `SourceStockLiteralObject`; `WordExpr::from_word`; `sole_variable_substitution`; `native_lexer_config`; `SourceCommandBindings::variable_accesses_for_invocation_args`; `SourceCommandBindings::variable_accesses_during_expression_invocation` | selected ingress lexer grammar and original component source extent; variable components retain verbatim spelling independently of compatibility argv text; quoted sole references retain their individual sites, compound and braced data words supply no sole read; substituted and literal index syntax stays distinct without proving physical scalar/element storage; actual native-family/frame lookup remains separate; current physical read receipts additionally require unique original site and spelling; private frozen literal RHS/store/read carriers retain effect-only stock object classes without strict representation, numeric contents or freshness permission; unknown effects and missing joins withdraw them; private prepared-argv read receipts retain original source/site/spelling, selected word mapping, physical contents origin and full source attestation; command observer boundaries withdraw them and native leaf consumers never re-evaluate index words; private per-context selected addresses freeze evaluated index parts before later effects and before the outer read observer, relocate actual cells and contexts together, and withdraw conflicting or unknown keys; sole-child expression type advice validates exact evaluation-parent ownership, unchanged source, literal preceding words and declaration-local scope before referencing an existing SSA version; earlier effects, formal inputs, aliases, traces and unavailable declaration flow withdraw it; it adds no SSA use, normal fact or executable read; no execution-admission authority; internal implementation: `declaration_expression_reads`, `DeclarationExpressionReadAdvice::diagnostic_version`, `variable_word_place`, `variable_reference_place` | none |
| source-owned formal caller advice | `rust/tcl-compiler/src/command_binding/formal_call_advice.rs`; `rust/tcl-compiler/src/command_binding/formal_value.rs`; `rust/tcl-compiler/src/analyser/diagnostics/var_command.rs`; `rust/tcl-compiler/src/analyser/diagnostics.rs`; `rust/tcl-compiler/src/interprocedural.rs` | `Analyser::emit_cfg_ssa_diagnostics_with_cu`; `collect_call_by_name_reads` | original callee allocation, accepted declaration frame and incoming/copy provenance; direct unentered projection requires exact conditional handler-site incoming provenance, separate from definedness/literals, intersected at joins and withdrawn by writes/removes/aliases/possible outputs/unknown effects; original caller argument-entry flow, native formal binder and effective written or captured-prefix operand origin; literal candidates support W307 absence advice only; unknown external calls remain possible; prior unknown effects, changed allocation, overwritten formals and unsupported bindings withdraw candidates; no actual activation, selected target, SSA value or executable authority; internal implementation: `declaration_formal_call_values`, `DeclarationFormalCallValue::value_in_source`, `symbolic_declaration_formal_value_at`, `symbolic_declaration_formal_components_in_word`, `symbolic_formal_value_at`, `conditional_handler_keeps_incoming` | none |
| reachable cell contents | `rust/tcl-compiler/src/ssa.rs`; `rust/tcl-compiler/src/def_use.rs`; `rust/tcl-compiler/src/cell_state_ssa.rs`; `rust/tcl-compiler/src/memory_ssa.rs`; `rust/tcl-compiler/src/variable_bindings.rs`; `rust/tcl-compiler/src/var_resolve.rs`; `rust/tcl-compiler/src/compilation_unit.rs` | `SsaFunction`; `SsaStatement::destruction_defs`; `SsaSourceView`; `SsaReadReference`; `is_positioned`; `read_spelling`; `source_tokens`; `read_reference`; `read_word`; `read_expression_variable`; `read_expression_incoming_slot`; `SsaIncomingSlotRead`; `read_completion_at`; `read_contents_presence_at`; `SsaReadContents`; `read_contents_at`; `read_word_contents`; `read_expression_variable_contents`; `source_symbols_at`; `at_statement`; `at_terminator`; `symbol`; `source_names`; `source_symbols`; `reaching_binding`; `reaching_bindings`; `context_before`; `DefUseResult`; `build_ssa_with_context`; `DefUseFacts`; `DefUseFacts::cell_key`; `DefUseFacts::symbol`; `DefUseFacts::physical_value_key`; `DefUseFacts::chain_for_cell`; `cell_key`; `cell_keys`; `cell_name`; `cell_names`; `cell_symbol`; `var_name`; `var_symbol_at`; `var_symbol_at_terminator`; `chain_for`; `build_cell_state_ssa`; `build_memory_ssa_with_cfg`; `VariableProofRelocation`; `relocated_variable_proofs`; `restored_source_proofs` | point-specific cell address and generation; reachability, traces and unknown alias exposure; SSA cell_key/cell_symbol retain primary storage identity; the immutable DefUseResult kernel uses function-local Symbol/Version coordinates, while DefUseFacts retains the complete cell inventory for facts and equality, while each physical query checks its selected symbol key against the current SSA; cell_name/var_name are presentation/source projections; MemoryLocation.storage_key is separate from name, and deterministic presentation ordering supplies no identity; destruction versions invalidate contents without values, representations or WrittenAt store provenance; conditional transitions retain predecessor use and may-def membership, and taint tombstones preserve predecessor influence | none |
| actual read success, store contents and current representation | `rust/tcl-compiler/src/var_resolve.rs`; `rust/tcl-compiler/src/ssa.rs`; `rust/tcl-compiler/src/contents_source.rs`; `rust/tcl-compiler/src/command_binding/normal_variable_continuation.rs`; `rust/tcl-compiler/src/command_binding/source_representation.rs`; `rust/tcl-compiler/src/command_binding/container_coercion.rs`; `rust/tcl-compiler/src/command_binding/native_result.rs`; `rust/tcl-compiler/src/native_numeric.rs` | `read_produces_value`; `contents_already_native_numeric_at`; `contents_native_string_access_closed_at`; `contents_native_numeric_category_at`; `SourceNativeNumericShape`; `read_word_produces_value`; `normal_store_contents_preserved`; `normal_variable_continuation`; `read_word_representation`; `read_word_representation_alternatives`; `read_expression_representation`; `read_expression_representation_alternatives`; `ClosedContainerRepresentations` | actual native scalar/array or dictionary-container policy, defined current contents, cell incarnation, full-origin attestation and observer closure; read success is independent of an SSA address/version; the normal store receipt preserves semantic contents only, while current representation requires unanimous exact read evidence and may change without a contents write; private frozen result receipts carry representation independently of bytes and require a known unchanged shared representation epoch; frame restoration retains the issuance high-water mark, while divergent joined histories permanently withdraw further stamps; a typed immediate constructor result may pass its native result boundary and publish its actual shape to the final unobserved store without becoming a frozen receipt; selected successful Increment stores, actual native ListLength count results and private-minted prepared runtime arithmetic/bitwise result descriptors publish numeric intrep independently of unknown bytes before callbacks, which may withdraw it; literal/pooled/forwarded/Boolean expression results cannot borrow the arithmetic production; ordinary Value formals may transfer only an actual frozen result receipt to their own Incoming activation slot; the point-local numeric shape intersects all live normal producer alternatives of the same engine, retaining equal Int/Double categories or Numeric for disagreement independently of joined writer origin; missing/nonnumeric/opaque alternatives withdraw it; this can exclude simultaneous ordinary List/Dict identity but supplies no concrete numeric value, operand, epoch or object identity; exact frozen operand bytes may exclude disjoint current container objects from a coercion footprint, without claiming unique native allocation; closed List/Dict alternatives are conversion-cost advice only, require every original read to succeed in a live unobserved cell, and keep strict representation Unknown when alternatives disagree; selected native store/result numeric shape may only exclude an ordinary container at a successful current read, retaining strict representation Unknown and no concrete numeric contents | none |
| reached original numeric operand cache | `rust/tcl-registry/src/native_numeric_conversion.rs`; `rust/tcl-compiler/src/command_binding/numeric_operand_cache.rs`; `rust/tcl-compiler/src/native_numeric.rs` | `NativeOperandNumericCacheProduction`; `NativeNumericOperandClass`; `integer_relational_operand_conversion`; `integer_index_operand_conversion`; `with_original_cache_class` | actual prepared C85+ relational numeric branch or selected runtime scalar index getter, original word/site and physical read, integer-only numeric branch, independent current cache class and closed conversion effects; only the same live normal continuation publishes current cache; original bytes can coexist with Double and cannot prove Int or a value; initial lindex requires original Integer cache, excludes grouped child conversion; Inline literal indices and unknown compilation decline; C84/Jim relational recipe abstains; no primitive Wide, value, epoch, completion, freshness or erasure donation; private source capture/finish keeps pre-getter evidence separate from result setters and read/store schedules | none |
| captured read representation cost | `rust/tcl-compiler/src/ssa.rs`; `rust/tcl-compiler/src/shimmer/commit.rs`; `rust/tcl-compiler/src/shimmer/hints.rs`; `rust/tcl-compiler/src/shimmer/use_site.rs`; `rust/tcl-compiler/src/shimmer/expr.rs` | `CommitWalker`; `CommitFacts::walker`; `compute_commit_facts`; `SsaSourceView::read_word_representation`; `SsaSourceView::read_expression_representation` | one common captured physical read projection and cost reconciliation; actual current ordinary List/Dict/String supersedes earlier SSA commitment, closed container alternatives remain May advice, native numeric shape is independent of unknown container representation; native named operands require post-argv read inventory; no constant, erasure, object identity or compiler admission licence; internal implementation: `captured_read_representations`, `cost_at_read`, `at_captured_representation`, `representation_cost_type`; internal implementation: `SsaReadRepresentationAdvice`, `read_word_representation_advice`, `read_expression_representation_advice`, `native_read_representation_advice`, `RepresentationCost`, `cost_for_word`, `cost_for_expression`, `cost_for_native_read` | none |
| temporal contents folding and existence | `rust/tcl-compiler/src/sccp.rs`; `rust/tcl-compiler/src/existence_query.rs`; `rust/tcl-compiler/src/var_resolve.rs`; `rust/tcl-compiler/src/place_bridge.rs`; `rust/tcl-compiler/src/tcl_expr_eval.rs` | `existence_constant_branches_with_ssa`; `ContentsPresence`; `def_places_with_continuation`; `QueryReading`; `ConditionReading` | exact nested dispatch, frozen target names and actual read/write observers; future effects cannot erase earlier SSA versions; actual `in_expr_at` stays separate from original diagnostic existence advice, whose name dependencies assert no contents and grant no SCCP branch erasure; internal implementation: `in_expr_for_diagnostics_at`, `in_tokens_for_diagnostics`; complete actual CFG input, selected lexer policy and per-site original parent/base travel together; supplied missing/foreign/geometry/configuration refusal is terminal; detached text is explicitly standalone; conditional names/types supply no Native presence, evaluated branch, closed read or erasure proof | none |
| evaluated script source | `rust/tcl-compiler/src/command_binding/executed_script_source.rs` | `ExecutedScriptSource`; `ExecutedScriptMapping`; `from_word`; `materialised`; `list_element`; `base` | exact frozen script bytes and semantic origin; literal word/list byte equality proves contiguous mapping; decoded, substituted and concatenated bodies retain distinct materialised origins and origin-qualified source proofs | none |
| cardinality-only argument roles | `rust/tcl-registry/src/spec.rs`; `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-spectcl/src/loader.rs` | `ArgRoleResolverInput`; `ArgRoleCountResolver`; `InvocationSemantics::arg_role_resolver_input`; `CommandSpec::arg_role_count_resolver`; `SubCommand::arg_role_count_resolver` | exact evaluated argc independently of operand values; foreach body position and scan/binary-scan output slots use this axis even when input or format values are unknown; unknown expansion cardinality retains incomplete roles; count callbacks receive no fabricated values; SpecTcl accepts known native descriptors and excludes unsupported Tcl count hooks; Studio field coverage and SpecTcl round-trip tests | none |
| structured argument layouts and consensus | `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-registry/src/registry_role_consensus.rs`; `rust/tcl-registry/src/spec.rs`; `rust/tcl-registry/src/invocation_words.rs` | `ArgRoleLayoutResolver`; `InvocationOptions::available`; `arg_role_assignments_consensus`; `RoleOperandAlternatives` | exact argv cardinality and selected option availability; unknown values remain unknown; closed bounded alternatives donate only unanimously identical roles, never a singleton value, operation or runtime handler; value/count/layout resolver declarations are mutually exclusive; native regex availability and layout-conflict controls; SpecTcl/Studio round trips | none |
| deferred script entry frame and possible callers | `rust/tcl-registry/src/body_execution.rs`; `rust/tcl-compiler/src/command_binding/future_bodies.rs`; `rust/tcl-compiler/src/unit_scope.rs`; `rust/tcl-compiler/src/ir.rs` | `BodyExecutionSpec::DeferredGlobalScript`; `deferred_entry_frame`; `DeferredBodyFrame`; `SourceFutureBodyInventory`; `SourceFutureCallSite`; `Module::future_call_sites` | selected exact Body operands and actual native scheduling policy; native after/default and idle enter the global frame later; registering-call normal effects and independently possible future effects remain separate; unknown host policy supplies no frame proof; exact possible future bindings retract caller seeds without producing them; no normal continuation/body-entry licence | none |
| procedure analysis body inventories | `rust/tcl-compiler/src/command_binding/generated_procedures.rs`; `rust/tcl-compiler/src/lowering/installed_bodies.rs`; `rust/tcl-compiler/src/specialise_factories.rs`; `rust/tcl-compiler/src/command_binding/origin_inventory.rs` | `SourceInstalledProcedureBody`; `SourceProcedureImplementationBody`; `SourceCommandBindings::selected_source_in_context`; `installed_procedure_body_units`; `original_declaration_body_units`; `specialise_factories_with_cap` | installed inventory requires exact current singleton allocation/source instance; original declaration inventory independently retains each original allocation/source/key despite later deletion, replacement or opacity, transported by Module.original_declaration_body_units; neither inventory belongs to executable roots or grants entry/CPP; evaluated native formals and authoritative namespace_key distinct from namespace/command presentation; exact selected_source_in_context retains point/compiler/read observations only in that original context; analysis-only body coverage and bounded metadata, original factory calls retained, no callable installation or compilation licence; Script.namespace_context transports the body key into CFG and the central function_source_entry, while original-name projections remain diagnostic | none |
| conditional provider definition installation | `rust/tcl-registry/src/definer.rs`; `rust/tcl-compiler/src/command_binding/snit_definition.rs`; `rust/tcl-compiler/src/command_binding.rs` | `DefinitionCommandInstallation`; `DefinitionDispatcher`; `TrustedPackageLoader::definition_dispatchers`; `lookup_dependencies`; `installed_lookup_dependencies`; `SourceInvocationBinding::nominal_definition_name_result` | driver-attested exact factory recipe, live installed token/generation; preload lookups must already resolve before loading, installed lookups are checked against the actual published commands after loading, and both sets remain current prerequisites; private/core lookup, namespace-state and execution-observer prerequisites; distinct SnitType/ItclClass installation recipes and qualified/written name-result protocols; normal returned name is advisory callable-name possibility only, with no object existence, class, constructor closure, opcode or specialization permission | none |
| actual source-file entry and caller correlation | `rust/tcl-registry/src/source_file.rs`; `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/unit_scope.rs`; `rust/tcl-compiler/src/command_binding/source_analysis_entry.rs` | `SourceFileGrammar`; `SourceFileSelection`; `select`; `completion_route`; `TrustedSourceModuleLoader`; `SourceOriginId::loaded`; `SourceAnalysisEntry::trusted_source_modules`; `CommandAllocation::matches_source_declaration`; `scan_source_call_sites_with_source_entry` | selected native file argv and return grammar; driver-attested path, encoding, EOF-decoded bytes and file implementation identity; reached loader runs in the caller frame with a retained Loaded origin; declaration correlation preserves actual allocation and grants no executable or edit proof | none |
| native script compilation | `rust/tcl-registry/src/native_compilation.rs` | `NativeCompilationMode`; `NativeCompilationFrame`; `NativeCompilationGrammar::ProcedureHookFrom`; `NativeCompilationGrammar::NamespaceOrigin`; `NativeCompilationGuard`; `NativeCompilationWordShape`; `NativeCompilationContext`; `with_inline_exception_range`; `NativeCompilationSelection`; `NativeCompilationFailure`; `NativeCompilationFailureScope`; `NativeCompiledBodyContext`; `NativeCompiledBodyOperand`; `entered_context`; `NativeCompiledBodies`; `NativeCompilationStep`; `NativeCompilationSteps`; `NativeCompiledExpressionOperand`; `NativeCompiledExpressionErrorContext`; `NativeExpressionCompilation`; `compilation_steps`; `validate`; `compiled_substitutions`; `NativeCompilationGrammar`; `NativeAppendKind`; `validates_empty_result`; `NativeBodyCompilation`; `NativeCompilationSpec`; `select`; `failure_for_selection`; `compiled_bodies`; `body_context`; `body_context_for_operand` | actual direct/bytecode entry and compiler-local-table capability, separately from physical variable frames; selected complete no-body roles establish empty compiler traversal independently of runtime expanded argv length, while unresolved body roles and compiler selection retain uncertainty; C 8.4 chunk guard vs later before-argv guard; Jim late dispatch | none |
| native compilation error presentation and admission | `rust/tcl-runtime-api/src/native_compilation_error.rs`; `runtime/rust/src/interp/native_compilation.rs`; `rust/tcl-bytecode/src/lib.rs`; `rust/tcl-compiler/src/codegen/native_failure.rs` | `NativeCompilationError`; `NativeCompilationErrorCommand`; `NativeCompilationPreflight`; `NativeCompilationAdmissionError`; `error_info_for_procedure`; `validate_native_compilation_entry`; `native_compilation_admission_error` | proved result/error code plus ordered raw compiler-command contexts and authored expression/child-body notes; actual procedure name supplied at pre-formal admission; missing presentation retains a host preflight obligation; failed commands retain native guard dependencies; UTF-8-safe native 150/50-byte excerpts; internal implementation: `retain_dependencies`; retained rejection stops before executable source emission, retaining reached original prefix registrations/enclosing compiler selections/fixed math as chunk-entry prerequisites; later dependencies stay outside rejected traversal; an earlier unresolved compiler visit withdraws later guest error presentation and retains native provider admission | none |
| native compiler backend admission | `rust/tcl-compiler/src/native_compilation_admission.rs`; `rust/tcl-runtime-api/src/native_compilation.rs`; `rust/tcl-compiler/src/codegen/wasm/pipeline.rs`; `rust/tcl-runtime-api/src/native_compilation_error.rs` | `NativeCompilationAdmission`; `NativeCompilationAdmissionScope`; `NativeCompilationAdmissionPlan`; `script_admission_plan`; `retained_script_admission`; `NativeMathFunctionPrerequisite`; `NativeMathFunctionTable`; `NativeMathFunctionBinding` | exact executed source origin, compiler-token dependencies and actual fixed math registration identity; unresolved provider obligations remain independent of proved failures; complete host script entry precedes stores and procedure-table installation; source procedure admission precedes formal binding; unavailable source refuses host execution | none |
| original procedure body creation | `rust/tcl-registry/src/native_procedure_body.rs`; `rust/tcl-vm/src/interp.rs`; `runtime/rust/src/interp/native_procedure_body.rs` | `NativeProcedureBodyCreationProtocol`; `NativeProcedureBodyAction`; `native_procedure_body_creation_protocol` | actual supported native string owner, original sharing observed before temporary ownership and counted resident spelling; shared ordinary C bodies become fresh string-only objects before formal parsing, unshared bodies and Jim retain the same original; precompiled ProcBody objects require separate authority; compilation attaches to the chosen body, never a bytes-equal input substitute; internal implementation: `capture_procedure_declaration_body`, `capture_procedure_body_value_in_original_name`, `choose_original_procedure_body` | `xtask-owner-resolution` |
| private error object headers | `rust/tcl-registry/src/native_error_objects.rs`; `rust/tcl-vm/src/interp/native_error_stack.rs`; `runtime/rust/src/interp/native_error_headers.rs` | `NativeErrorObjectsProtocol`; `native_error_objects_protocol` | actual C constructor and return producer; C8.6+ owns the original private List header and C8.5+ return producers own the original private Dict header; saved interpreter state retains headers once without retaining children again; mutation copies shared headers, restore consumes the saved header, and public snapshots retain the same original List; CALL List members borrow the actual procedure, apply, method, constructor, next and namespace invocation vectors before acquiring their real native member refs; TclOO teardown frames carry the actual empty vector and log no CALL; reset and shifted-frame metadata hold no duplicate original child inventory; internal implementation: `NativeErrorStack`, `NativeReturnOptions`, `adopt_original`, `original_error_stack_value` | `xtask-owner-resolution` |
| original C return-instruction options | `rust/tcl-registry/src/native_return_options.rs`; `runtime/rust/src/interp/native_return_instruction.rs`; `runtime/rust/src/interp/native_error_headers.rs` | `NativeReturnOptionsApplication`; `native_return_options_application` | actual C85+ Immediate/Syntax instructions retain the same merged Dict header; Stack selects the separate original List/merge path; original private Code is retained without its string getter, private stack COW precedes aliased operand extraction, and NULL-interpreter line probes preserve reached cache effects; Syntax captures actual private Info/Code with numerical controls and its selected stack omission; C85/86 empty getter append copies shared Info while C90/91 retains it; compiler reset retires genuine original fields after literal acquisition; internal implementation: `process_original_c_return_options`, `capture_original_c_syntax_options`, `reset_original_c_compiler_result` | `xtask-owner-resolution` |
| native procedure installation | `rust/tcl-registry/src/native_procedure.rs` | `NativeProcedureDefinitionSpec`; `NativeProcedureDefinitionSelection`; `NativeProcedureDefinition`; `ProcedureDefinitionResult`; `select_native_procedure_definition`; `StaticVariableInitialiser`; `StaticVariableDeclaration`; `StaticVariableError`; `parse_static_variables` | proved core protocol under selected C/Jim parameter grammar; evaluated argv positions and persistent static storage capture | none |
| actual variable observer entry | `rust/tcl-runtime-api/src/native_compilation.rs`; `rust/tcl-vm/src/interp.rs`; `rust/tcl-compiler/src/command_binding/runtime_entry.rs` | `NativeVariableObserverPresence`; `permits_no_callbacks`; `NativeCompilationEntry` | complete actual observer table separate from contents/frame/cell provenance; Absent and BoundedNativeOnly alone permit callback-free reasoning; Present includes opaque native callbacks and active trace firing, independently of script trace introspection; bounded hooks grant no contents, links or physical frame facts, and foreign/partial tables remain Unknown; internal implementation: `install_runtime_entry` | none |
| actual stock registration compiler descriptor | `rust/tcl-registry/src/registry.rs`; `rust/tcl-vm/src/interp.rs`; `rust/tcl-vm/src/cmd_info.rs`; `rust/tcl-vm/src/cmd_array.rs`; `rust/tcl-vm/src/cmd_namespace.rs` | `native_compilation_for_registration` | actual canonical registration identity; selected native engine point; private parent plus frozen member prefix; conflicting reverse mappings retain unknown; does not establish live implementation provenance; unknown or absent descriptors require provider evidence; internal implementation: `native_hook_for_registry_identity`, `register_stock_namespace_ensemble` | none |
| native inline operation lifetime | `rust/tcl-runtime-api/src/native_compilation.rs`; `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/codegen/mod.rs`; `rust/tcl-bytecode/src/lib.rs`; `rust/tcl-vm/src/exec.rs` | `NativeOperationSelectionPlan`; `NativeOperationSelectionDecline`; `native_operation_selection_plan`; `NativeOperationSelectionSite`; `compiler_selection_prerequisite`; `NativeCompilerSelectionPrerequisite::from_command_registration`; `validate_native_compilation_entry` | original source allocation, exact replay/namespace, actual Command/Ensemble compiler registration independent of callable and worker guards, separate chunk and before-argv premises; once-selected operation survives argument effects; malformed metadata refuses admission; missing source/proof requires native provider; internal implementation: `retain_native_operation_selection` | none |
| native frame-evaluation argv | `rust/tcl-registry/src/native_compilation.rs`; `rust/tcl-registry/src/frame_effect.rs`; `rust/tcl-compiler/src/codegen/emitter/bytecoded.rs`; `rust/tcl-vm/src/command.rs`; `rust/tcl-registry/src/native_instruction_plan.rs`; `rust/tcl-compiler/src/codegen/statements.rs`; `runtime/rust/src/interp/native_body_artifact.rs` | `NativeUplevelOperands`; `NativeInstructionPlan::Uplevel`; `NativeUplevelInstruction`; `uplevel_operands`; `FrameEffectSpec::resolve_arguments` | original known frame-word selection and C9.1 procedure compile capability separate from runtime frame existence; exact once argv and shared script concatenation; C91 recipe preserves explicit original level or literal 1 and original script range, CONCAT_STK only for multiple fragments then UPLEVEL; body preparation follows actual frame selection, not operand compiler visits; earlier releases/script frames/substituted first words remain generic; internal implementation: `uplevel_cmd`, `cmd_uplevel` | none |
| native source-word emission | `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/codegen/hook_operands.rs`; `rust/tcl-compiler/src/codegen/cmd_subst.rs`; `rust/tcl-compiler/src/codegen/emitter/bytecoded.rs` | `compiled_local_name_word`; `compiled_local_name_value` | original source eligibility and evaluated name are separate; inert private selectors have no runtime evaluation; native list segments retain bracing, expansion, prior prefix values and original operand coordinates; compiler-selected release protocol remains independent of handler normal effects; internal implementation: `original_hook_argument`, `emit_native_argument_list` | none |
| native inline body compiler context | `rust/tcl-registry/src/native_compilation.rs`; `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/codegen/control_flow.rs` | `body_context_for_invocation_operand`; `proved_native_inline_body_context` | selected actual native compiler, frozen effective argv and original body word shape; exception context scoped independently per operand; unproved inline selection cannot supply context; internal implementation: `with_inline_body_operand` | none |
| native persistent variable storage | `runtime/rust/src/frame.rs`; `runtime/rust/src/vars.rs` | `VarTable` | raw cell ownership is separate from name bindings; static copies snapshot values while captures retain mutable alias wrappers; Jim alias wrappers resolve the selected logical level/name on each access; internal implementation: `StaticVariables`, `RetainedVariableCell`, `capture_cell`, `install_statics`, `capture_static_source` | none |
| formal parameter activation | `rust/tcl-syntax/src/formal_params.rs`; `rust/tcl-dialect/src/version.rs`; `rust/tcl-dialect/src/profile.rs` | `ParameterGrammar`; `parameter_grammar`; `FormalParameter`; `FormalArgumentBinding`; `FormalArityError`; `parse_formal_parameters_in`; `bind_formal_arguments`; `formal_parameter_usage`; `message_for_definition`; `message_for_definition_bytes`; `skips_empty_body_activation` | selected native C/Jim list grammar, optional/rest/reference allocation and definition-time release diagnostics | none |
| lambda invocation selection | `rust/tcl-registry/src/lambda_invocation.rs`; `rust/tcl-syntax/src/naming/native.rs`; `rust/tcl-vm/src/command.rs`; `runtime/rust/src/cmd_proc.rs`; `rust/tcl-vm/src/interp.rs`; `rust/tcl-vm/src/exec.rs`; `rust/tcl-vm/src/interp/namespace_context.rs`; `rust/tcl-vm/src/interp/native_original_lookup.rs`; `rust/tcl-vm/src/interp/native_jim_lookup.rs` | `LambdaInvocation`; `LambdaInvocationSelection`; `select_lambda_invocation`; `NativeNameProtocol::lambda_namespace_input`; `NativeNameProtocol::jim_lambda_namespace_input`; `NativeNamePurpose::JimLambdaNamespace`; `Vm::namespace_operation_error`; `formal_parameters`; `Vm::preflight_original_lambda_activation` | native family, selected list grammar, formal parameters, body, explicit/default namespace and trailing arguments; C namespace construction retains its root prefix and independent namespace lookup extent; the Jim-only namespace projection preserves exact counted original bytes without canonicalisation, lookup or creation, while the VM separately retains the actual List child and original namespace execution context; original getters, callable ownership and actual frame entry remain independent; VM usage/arity and empty-body preflight precede namespace access and temporary binding, with its completed flag preventing a repeated body getter; the VM retains the same original Jim namespace object through Script preparation, parsing and local binding, obtaining its getter only at a reached namespace operation; cold command lookup obtains namespace before command bytes even for qualified heads, while a valid same-object cache comparison obtains neither namespace getter; current-context native compilation requires an already materialised authenticated coordinate and declines without obtaining one; independently selected explicit known targets retain their own checks; Runtime independently retains original namespace holders and the actual apply dummy argv through its frame and reached-getter owners; internal VM implementation: NamespaceExecutionContext, OriginalJimNamespaceContext; C missing-namespace result and errorCode reuse the shared ObjectLookup report and selected native tuple/string emitter, preserving original namespace constructor/getter channels independently of Jim | none |
| diagnostic source and overwrite advice | `rust/tcl-compiler/src/script_binds.rs`; `rust/tcl-compiler/src/registry_invocation/store_advice.rs`; `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/analyser/diagnostics.rs`; `rust/tcl-compiler/src/realm.rs` | `Analyser::emit_cfg_ssa_diagnostics_for_function_full`; `Analyser::emit_cfg_ssa_diagnostics_with_cu`; `CommandBindingRealm::original_declaration_assistance` | original source image/channel and byte geometry; unchanged accepted authored declaration/formals for potential local-read diagnostics; declaration-owned candidate layout for conditional lexical bodies, preserving opaque runtime alternatives; exact adjacent original setters, same local cell lifetime and previous contents writer; strict overwrite advice requires closed execution/variable/object observers, while conditional overwrite advice retains object-callback uncertainty; missing or conflicting premises decline; diagnostic advice only; no reached activation, physical undefined contents, executable body entry, successful dispatch or dead-store elimination; internal implementation: `script_image_binds_name`, `authored_procedure_read_advice`, `AuthoredProcedureReadAdvice::owns`, `overwritten_local_store_advice`, `conditional_overwritten_local_store_advice`, `conditional_unread_local_store_advice`, `conditional_body_topology_advice`, `SourceCommandBindings::declaration_operand_layout_advice`, `SourceInvocationBinding::declaration_operand_layout_advice`, `SourceInvocationBinding::declaration_read_occurrences`, `DeclarationReadOccurrenceAdvice::diagnostic_version`, `SourceInvocationBinding::declaration_flow_report`, `DeclarationFlowReport::invocation_may_be_reached`, `conditional_index_access_advice`, `SourceInvocationBinding::original_compilation_lookup_advice` | none |
| Logical frame-alias source projection | `rust/tcl-registry/src/state_transition.rs`; `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-registry/src/commands/tcl/upvar_.rs`; `rust/tcl-spectcl/src/loader.rs`; `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-registry/src/source_name_ownership.rs` | `LogicalSourceStateTransitionResolver::FrameAliases`; `StateTransitionDescriptor::logical_source_resolver`; `ResolvedInvocation::authored_logical_source_state_transitions`; `ResolvedInvocation::authored_logical_source_variable_aliases` | actual stock or DSL metadata producers register explicit readonly source dispatch; positively retained Logical input and current selected schema remain caller-owned; authored frame-layout consensus preserves original alias operands, while dynamic/divergent layouts abstain; descriptor replacement withdraws inherited source dispatch and extension retains it; custom resolvers, commit annotations and dynamic widenings remain independent; source facts grant no Native resolver identity, entered frame, successful link or completion | `xtask-owner-resolution` |
| conditional literal body name ownership | `rust/tcl-registry/src/source_name_ownership.rs`; `rust/tcl-compiler/src/script_binds.rs`; `rust/tcl-compiler/src/ir_helpers/materialized_footprint.rs`; `rust/tcl-compiler/src/command_binding/declaration_body_ownership.rs`; `rust/tcl-compiler/src/deferred_writes.rs`; `rust/tcl-compiler/src/cfg_builder/global_write_info.rs`; `rust/tcl-compiler/src/deferred_writes/supplied.rs`; `rust/tcl-compiler/src/command_binding/materialized_footprint.rs` | `SourceRolePurpose`; `SourceNameOwnershipPurpose`; `SourceVariableNameOwnership`; `ResolvedInvocation::authored_source_name_ownership`; `original_literal_body_ownership`; `DeclarationLayoutObservation::source_body_name_ownership`; `Scan::lambda`; `own_frame_global_writes_with_metadata_context`; `own_frame_global_writes_standalone`; `SourceInvocationBinding::original_materialized_footprint_for_module`; `OriginalSourceMaterializedFootprint::deferred_namespace`; `SourceScan::original_registration`; `selected_values`; `SourceScan::lambda` | original selected installer and effective whole words, full actual availability/store/config and source lookup horizon; named bindings/by-name reads/scope aliases stay distinct from interpolation; empty names, literal dollar sigils, Unicode and scalar parentheses retain their exact shared array-partition roots; dynamic/unavailable/undeclared effects preserve opaque residuals; explicit standalone is separate; no Native word, body/frame entry, current cell, read exclusion or erasure grant; literal-lambda body advice retains actual Module/nested source input and lexer policy, while independent installer/operand selection remains required; missing/foreign/stale supplied input keeps opaque write residuals; standalone unprofiled catalogue is an explicit source mode, never missing-input recovery or Native frame/completed-write authority; Module-aware footprints retain the original installer snapshot and complete point input or genuine nested projection/source ancestry; only authored GlobalFrame scope supplies a source namespace, with unknown invoking/triggering future scopes refused; final bindings and mechanical SourceImage transport confer no frame/callback grant; supplied deferred scans select authentic installer roles/timing/effective argv and complete point input under its immutable source horizon; nested lambda values retain actual input; unknown future scopes remain opaque; legacy Scan::lambda is explicit Standalone only, with no missing-owner recovery or callback/frame authority; installer Module matching additionally retains exact entry, compilation scope, namespace/frame kind and incoming formal owner independently of a valid metadata handle; changed source ownership refuses conditional body names | none |
| evaluated body execution | `rust/tcl-registry/src/body_execution.rs`; `rust/tcl-compiler/src/execution_region.rs`; `rust/tcl-compiler/src/executable_ir.rs`; `rust/tcl-compiler/src/semantic_analysis.rs`; `rust/tcl-registry/src/native_compilation.rs`; `rust/tcl-compiler/src/registry_invocation.rs` | `BodyExecutionSpec`; `CapturedLifecycleSpec`; `BodyExecutionSelection`; `ExecutionRegionDependency`; `EvaluatedBodyRegion`; `ExecutionPhase`; `RegionSelection`; `RegionTarget`; `WrapperEffectProjection`; `EvaluatedRegionCompletion`; `PossibleBodyRegion`; `ConditionalBodySource`; `PossibleBodyCondition`; `PossibleBodyTopology`; `possible_body_invocation`; `GenericInvoke`; `registry_specialisation_arguments_exact`; `evaluated_regions` | selected literal argument grammar, exact provider version/hook prerequisites, ordered caller-frame phases and residual effects | none |
| case-list possible body locations | `rust/tcl-registry/src/case_bodies.rs`; `rust/tcl-registry/src/spec.rs`; `rust/tcl-compiler/src/command_binding/case_bodies.rs`; `rust/tcl-registry/src/script_body_flow.rs`; `rust/tcl-registry/src/native_compilation.rs` | `CaseBodyOperands`; `CaseListSpec::possible_body_operands`; `script_body_flow_in_registry`; `ScriptBodyFlow::CaseBodies`; `PossibleBodyTopology::CaseAlternatives` | shared authored option/clause/list grammar; frozen effective argv and native list policy; each body retains its original argument and decoded list element; unknown subject preserves selection/error residual; actual entered scripts retain exact source origin; partial unknown option layouts remain unresolved | none |
| original event callbacks and wait triggers | `rust/tcl-registry/src/native_event.rs`; `rust/tcl-registry/src/native_eval_object.rs`; `rust/tcl-cmd-core/src/event.rs`; `rust/tcl-syntax/src/native_array_search.rs`; `rust/tcl-vm/src/interp/native_event_context.rs`; `runtime/rust/src/interp/native_event_context.rs` | `NativeEventProtocol`; `NativeEventWait`; `EvalObjectPurpose::AfterCallback`; `EventQueue`; `EventTurn`; `native_c_decimal_unsigned` | independently selected C/Jim callback, global wait, option and identifier purposes; original script handles; current service turns retain cancellation and defer later registrations; C84 counted source entry remains separate from modern object evaluation; global write/unset traces differ from Jim value comparison; no callback-free, native compiler, notifier-completeness or ordinary completion grant | none |
| invocation completion routing | `rust/tcl-registry/src/completion_route.rs`; `rust/tcl-registry/src/registry.rs`; `rust/tcl-registry/src/value_transfer/completion.rs`; `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/cfg_builder/mod.rs` | `ReturnStateEffect`; `native_return_state_effect`; `ReturnCompletionRoute`; `InvocationCompletionRoute`; `ProcedureCompletionPolicy`; `procedure_completion_policy`; `invocation_completion_words`; `exact_invocation_completion_words`; `invocation_completion_knowledge`; `invocation_completion_route`; `normal_possible`; `abrupt_possible`; `immediate_code`; `through_procedure_boundary`; `through_procedure_boundary_in`; `ReturnInvocationGrammar`; `ReturnInvocationGrammar::for_dialect`; `ReturnInvocationFacet`; `decode_return_words_in`; `ReturnCompletion`; `original_native_source_conditional_completion`; `CfgCommandClasses::conditional_source_completion_route` | native option grammar retains eventual code and unwind level; only procedure boundaries settle pending returns; unknown retains normal and abrupt continuations; C/Jim raw control-code boundaries differ  original-source and evaluated-worker facets remain distinct; actual argv release selects conversion width independently of lexical Numbers; unknown or disputed C85/Jim policies decline; original direct result/errorcode ordinals differ from a merged dictionary errorcode, with no object/header/frame or compiler entry grant; conditional Native source topology additionally requires full retained input/Registry/configuration and original literal argv, accepted arity and an actual source schema or independent closed declaration/compiler layout; a retained Native source-owner refusal is terminal, missing/dynamic/expanded/unclosed input declines, and Logical source routing remains separate from reached dispatch, Normal results and codegen continuation permission; | none |
| active VM artifact identity | `rust/tcl-runtime-api/src/manifest.rs`; `rust/tcl-vm/src/interp.rs`; `rust/tcl-vm/src/exec.rs` | `ArtefactIdentityManifest::refused_rungs`; `Vm::function_identity_claims_match`; `Vm::function_live_command_bindings_match`; `Vm::run_compiled_unit`; `Frame` | actual retained manifest and current coherent pack stamps at entry/live epoch checks; disagreement refuses dependent rungs, Generic without dependent claims remains independent; missing manifest cannot supply pack facts; source replay refusal retains admitted epoch and pc; OverlayMiss retains current identity; software selection keys give no Native compiler/handler/body/frame/roster authority and do not replace existing original guards | none |
| native package database and original option lookup | `rust/tcl-registry/src/native_package.rs`; `rust/tcl-registry/src/native_index_lookup.rs`; `rust/tcl-core-types/src/native_index.rs`; `rust/tcl-vm/src/interp/native_index_lookup.rs`; `runtime/rust/src/interp/native_index_lookup.rs`; `runtime/rust/src/obj/native_index.rs`; `rust/tcl-vm/src/interp/native_package_files.rs`; `rust/tcl-vm/src/interp.rs`; `rust/tcl-vm/src/cmd_package.rs`; `rust/tcl-vm/src/cmd_package/original.rs`; `rust/tcl-vm/src/interp/authored_package.rs`; `runtime/rust/src/cmd_package.rs` | `NativePackageProtocol`; `native_package_protocol`; `c_members`; `member_usage`; `retains_version_object`; `tracks_files`; `failure_error_code`; `hash_recipe`; `NativeIndexLookupProtocol`; `NativeStaticIndexTable`; `native_index_lookup_protocol`; `NativeIndexLookupFlags` | actual C8.4–9.1/Jim protocol independent of logical grammar; entered VM package currency, allocation-only operand leases and physical table receipt remain through original String getters, callbacks, result construction and same-target loader/file/table cleanup; embedding provide selects that actual protocol; full first Host cause precedes Guest projection; native byte version conversion and copied script buffers; native bad-member, preference, version and loader-completion failures publish the selected private error-code header before returning carried metadata; C86+ original version headers, C9 original private files List; ABI-aware STRING_KEYS births and removal; original Index cache hit before getter and foreign-origin refusal; retained immutable table authority independent of spelling; exact native table stride, C9 TEMP_TABLE and NULL_OK; canonical Index usage without replacing the original header; internal implementation: `OriginalPackage`, `PackageTableReceipt`, `with_package_table_checked`, `NativeStaticLookup`, `original_static_option_lookup`, `native_index_from_original_checked`, `native_index_operand_checked`, `native_index_usage_bytes_checked`, `record_package_source_file`, `package_version_object` | `xtask-owner-resolution` |
| authored namespace component geometry | `rust/tcl-syntax/src/naming/authored_namespace_component.rs`; `rust/tcl-syntax/src/naming.rs`; `rust/tcl-lsp-core/src/namespace_rename.rs` | `AuthoredNamespaceOccurrencePurpose`; `AuthoredNamespaceComponent`; `authored_namespace_component_extent` | exact written occurrence, independently selected resolved namespace reports and Namespace/Entity/Variable purpose; variable syntax uses the actual LexerConfig; Written retains only the matching original span, Inherited supplies no editable span; computed or unavailable geometry declines; caller currency, identity, hazards and edit permission remain independent | `xtask-owner-resolution` |
| semantic bundle Registry correspondence | `rust/tcl-compiler/src/semantic_analysis.rs`; `rust/bpf-tcl-ir/src/semantic_bridge.rs` | `SemanticAnalysisBundle::matches_registry` | genuine context-valid builders retain the actual complete Registry semantic key; the audit-only BPF bridge checks that key before descriptor projection, preserving earlier source/context declines and returning RegistryUnavailable for a missing or foreign key; this comparison grants no source correspondence, Native/Normal/frame facts or executable backend admission | `xtask-owner-resolution` |
| frame selector and package policy | `rust/tcl-registry/src/frame_effect.rs`; `rust/tcl-registry/src/state_transition.rs`; `rust/tcl-registry/src/invocation_words.rs`; `rust/tcl-dialect/src/profile.rs`; `rust/tcl-dialect/src/version.rs` | `FrameLevel`; `FrameLevelWord`; `FrameArgumentResolution`; `FrameEffectSpec`; `VariableAliasDestination`; `VariableAliasFrame`; `is_active_in_frame`; `is_active_in_frame_with_policy`; `resolve_arguments`; `FrameLevelPresence`; `CompletionOptionsPolicy`; `catch_positional_arity`; `completion_options_policy`; `VariableLookupPolicy`; `VariableLookupPolicy::container_model`; `VariableLookupPolicy::link_binding`; `VariableLinkBinding`; `variable_link_binding`; `VariableContainerModel`; `variable_container_model`; `PackageProtocol`; `NamespaceImportBinding`; `upvar_level_presence`; `uplevel_level_presence`; `package_protocol`; `namespace_import_binding`; `validate_version_for`; `validate_requirement_for`; `compare_versions_for`; `select_package_version_for` | optional-level presence separate from numeral grammar; C release and Jim runtime family remain independent; selected variable lookup policy owns pure container/link mappings, while physical Runtime configuration still requires the actual native name protocol and an authored provider only checks compatibility with the existing model; no-runtime policies abstain | none |
| selected VM execution frame and saved target | `rust/tcl-vm/src/interp/selected_frame.rs`; `rust/tcl-vm/src/interp/native_operation_currency.rs`; `rust/tcl-vm/src/interp.rs` | `SelectedFrameRestore`; `Vm::select_execution_frame`; `Vm::restore_execution_frame`; `SavedExecutionTarget`; `Vm::select_execution_frame_checked`; `Vm::eval_value_at_level_checked` | actual interpreter/context identity and descriptive original currency captured before frame hiding; same existing frame, namespace-tail and shifted-error-stack restoration runs only on that physical target under mandatory saved cleanup; callback-selected child storage is preserved and the full first Native cause survives cleanup; saved metadata grants no fresh invocation, Native recipe or generic activation-abort closure | none |
| lists | `rust/tcl-syntax/src/list.rs` | `find_element`; `split_list`; `list_element`; `join_list`; `append_list_element`; `junk_fragment` | invariant | none |
| native list-result serialization | `rust/tcl-syntax/src/list_result.rs`; `rust/tcl-registry/src/native_result.rs`; `rust/tcl-registry/src/spec.rs`; `rust/tcl-registry/src/invocation_words.rs` | `NativeListResultSerialization`; `InvocationDialect::list_result_serialization`; `ordinary_range_literal_result`; `constant_range_literal_result`; `run_const_fold_in` | actual C84/C85+/Jim native list-object bytes after shared element parsing and selected index grammar; portable folds require byte agreement; rendering grants values independently of current input class, callbacks, completion and erasure | none |
| dictionary scope binding | `rust/tcl-compiler/src/dictionary_bindings.rs`; `rust/tcl-compiler/src/var_resolve.rs`; `rust/tcl-registry/src/dictionary_scope.rs`; `runtime/rust/src/interp/native_dictionary.rs` | `enter_dictionary_scope`; `finish_dictionary_scope`; `scope_marker_effects`; `DictionaryScopeId`; `DictionaryScopePlan`; `DictionaryWritebackFailure`; `writeback_failure_route`; `stock_scripted_wrappers`; `DictionaryScopeRead`; `dictionary_scope_variable_read` | frozen original keys/path and live physical cells at completion-sensitive epilogue; actual C/Jim completion policy and exact joins/relocation retain the source allocation; Runtime original-name getters hold the selected receiver through read callbacks, preserve native initial/quiet/creating lookup flags and save genuine private error headers at the corresponding writeback boundary | none |
| conditional dictionary source phase | `rust/tcl-registry/src/dictionary_scope.rs`; `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-compiler/src/registry_invocation/source_scoped_body.rs`; `rust/tcl-compiler/src/command_binding/logical_operation.rs` | `AuthoredDictionaryScopeSourcePhase`; `authored_dictionary_scope_source_phase`; `additional_effects`; `original_written_script_body_is_empty` | actual selected accepted dictionary grammar retains direct mapping and explicit additional effects separately; source-only empty-body continuation requires original lexical geometry, selected current same-invocation body, quiet owned scalar and complete trace-free inventory; unsupported additional writes and scripted helpers refuse, unknown alias keys retain contents clobber; generic UNKNOWN facts, Native handler/frame/store and Normal completion remain independent | none |
| dicts | `rust/tcl-syntax/src/list.rs`; `rust/tcl-syntax/src/value.rs`; `rust/tcl-cmd-core/src/dict.rs` | `find_element`; `split_list`; `canonical_dict_slots`; `ValueOps::dict_pairs`; `ValueOps::dict_hash_bucket_count`; `ValueOps::new_dict_with_hash_bucket_count`; `worded_parse_error`; `dict::info` | invariant | none |
| native glob matching | `rust/tcl-syntax/src/native_glob.rs`; `rust/tcl-syntax/src/native_glob_case_mapping.rs`; `rust/tcl-syntax/src/raw_string.rs`; `rust/tcl-cmd-core/src/info.rs` | `NativeGlobProtocol`; `NativeGlobProtocol::export_filter_applies`; `NativeGlobObject`; `NativeNameGlobPurpose`; `match_native_name_pattern`; `match_native_glob_objects`; `match_c_string_glob`; `equal_c_strings`; `name_pattern_uses_exact_lookup`; `info::command_list` | independently selected actual or authored issuer; original object type/cache/resident bytes; C counted Unicode, pure byte array and CString dispatch; exact name lookup separate from scans; pinned native simple casing and Jim numeric units; explicit CString entry points preserve terminated extents and opaque bytes without selecting an object getter; absent storage or unsupported reached access refuses; command enumeration applies the original C CString extent before namespace qualification, then uses InfoCommandsSearch for exact-key or native UTF scan semantics; Jim retains its flat-table helper and counted pattern owner; fixed native object/CString matrices and 110 original pattern-object windows across six engines; selected export filtering applies C ordered export verdicts or Jim no-export-filter semantics only, independently of command existence, completed import or aliases | none |
| Unicode glob compatibility | `rust/tcl-syntax/src/glob.rs`; `rust/tcl-lsp-core/src/workspace_index.rs` | `string_match`; `string_match_bytes`; `string_case_match`; `is_literal` | Unicode analysis/display strings and Rust casing; no native byte/object or lookup proof; workspace import classification delegates escape-aware literal-pattern recognition before its existing typed source-slot and export checks; syntax classification supplies no installed lookup or absence proof | none |
| switch body grammar | `rust/tcl-syntax/src/switch_body.rs` | `tokenise_switch_body`; `parse_braced_pairs` | invariant | none |
| native C switch option grammar and usage | `rust/tcl-cmd-core/src/switch.rs`; `rust/tcl-vm/src/cmd_switch.rs`; `runtime/rust/src/cmd_switch.rs` | `parse_options`; `usage`; `Options`; `Mode` | actual C release selects the static original-option table, scan bound, repeated-mode policy and inline/list usage; C8.4 scans all leading options and accepts the last mode, while C8.5+ reserves subject/body operands and rejects repeated modes; adapters retain original option headers and select the same usage owner for empty case Lists | `xtask-owner-resolution` |
| opaque native invocation effects | `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/cfg.rs`; `rust/tcl-compiler/src/ssa.rs`; `rust/tcl-compiler/src/place_bridge.rs`; `rust/tcl-compiler/src/variable_bindings.rs`; `rust/tcl-compiler/src/taint.rs`; `rust/tcl-compiler/src/taint_interproc.rs` | `Statement::has_opaque_native_accesses`; `Function::has_opaque_native_accesses`; `SsaFunction::has_opaque_native_accesses`; `statement_mutation_places`; `read_places`; `def_places` | retained original byte invocation; explicit unknown reads, writes and callback residual independent of empty named SSA sets; clobber without definite named definition; withdraw optimization, clean taint and source-edit proofs | none |
| source edit ownership | `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/cfg.rs` | `Script::is_authored_source`; `Script::is_fully_authored_source`; `Statement::source_edit_span`; `statement_source_edit_span`; `terminator_source_edit_span` | exact per-node executed source instance, affine authored range, and typed command eligibility; diagnostic ranges, unknown semantic lookup and lexical source ownership are separate axes | none |
| compiler-selected private names | `rust/tcl-compiler/src/command_binding/named_invocation.rs`; `rust/tcl-compiler/src/registry_invocation.rs` | `SourceNamedInvocationProof`; `named_invocation`; `native_named_command_words` | original compiler/map prerequisites, captured private name, original written operand indices, and independent post-argv handler lookup | none |
| reached implicit math binding | `rust/tcl-compiler/src/math_function_binding.rs`; `rust/tcl-compiler/src/command_binding/origin_inventory.rs` | `ExpressionMathBindings`; `for_origin`; `resolved_call_for_value_analysis`; `resolved_call`; `proves_intrinsic_for_erasure`; `ResolvedMathFunctionCall`; `proved_invocation`; `implicit_math_invocation_at`; `implicit_math_invocations_for_script` | exact source origin and AST call coordinate after reached operands; actual mutable dispatch or interpreter-owned fixed implementation; identity-only analysis remains separate from erasure requiring captured operand callback-effect closure; unknown and skipped lazy calls retain no donated stock identity | none |
| fixed-function diagnostic presence | `rust/tcl-registry/src/mathfunc.rs`; `rust/tcl-compiler/src/command_binding/command_presence.rs` | `fresh_fixed_function_presence`; `diagnostic_math_function_presence_at` | ready native fixed-table call coordinate; closed fresh C8.4 or measured Jim registration roster, superseded by actual runtime table evidence; unknown entry, opaque mutation or conflicting observations withdraw advice; no command QName, registration token, arity, completion or compiler authority | none |
| retained procedure implementation bodies | `rust/tcl-compiler/src/command_binding/generated_procedures.rs` | `SourceProcedureImplementationBody`; `procedure_implementation_body`; `procedure_implementation_bodies` | full original allocation and incarnation, evaluated source, formals and installed namespace; retained analysis inventory requires a separately proved current handler allocation before conditional result typing; no final-name lookup, body-entry or compiler authority | none |
| source-owned whole-expression preparation | `rust/tcl-compiler/src/command_binding/expression_preparation.rs`; `rust/tcl-compiler/src/math_function_binding.rs`; `rust/tcl-compiler/src/cfg.rs` | `SourceExpressionPreparation`; `expression_preparations_for_script`; `with_preparations`; `preparation_for_context`; `retain_expression_preparations` | exact source origin, full-expression parser base and native term tree; reached preparation independently of original chunk compiler traversal; consumed fixed-table prerequisites survive removal even when no lazy call was reached | none |
| consumed implicit math artifact dependencies | `rust/tcl-compiler/src/math_function_binding.rs`; `rust/tcl-compiler/src/static_loops.rs`; `rust/tcl-compiler/src/codegen/mod.rs`; `rust/tcl-compiler/src/cfg.rs` | `native_fold_dependency`; `NativeMathFoldDependency`; `StaticLoopSummary`; `summarise_for_statement_with_dependencies`; `retain_math_invocations` | actual reached call guards survive expression removal; independently closed object callback effects required; fixed table owner and original relative command lookup remain distinct; commit atomically and refuse unrepresented obligations; frozen alias-prefix bytes do not prove effect-free object coercion | none |
| native opcode specialisation | `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/lowering/mod.rs`; `rust/tcl-compiler/src/codegen/mod.rs` | `proved_native_admitted_inline_operation`; `admitted_native_compiler_invocation`; `native_site_binding_requirement` | retained immutable admission recipe and complete native guard/replay premises; compiler-only wrapper exposes no normal effects; strict `proved_native_inline_operation` remains required for erasure; Generic handler convergence cannot mint an opcode or a live opcode guard; captured-name path and unresolved provider obligations remain separate | none |
| namespace directive metadata | `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-registry/src/state_transition.rs` | `NamespaceDirectiveFootprint`; `namespace_directive_footprint`; `NamespaceTransition::export_pattern_operands` | converged handler and frozen argument layout; exact export control word and literal patterns; directive metadata cannot supply imported binding, physical mutation, compiler, or normal-execution proof | none |
| possible variable-name operands | `rust/tcl-compiler/src/registry_invocation.rs` | `PossibleVariableNameOperands`; `possible_variable_name_operands`; `variable_name_operands`; `phased_operands`; `phased_unresolved_roles` | retained candidate identity union, frozen effective name operands and explicit unknown residual; advisory read/write hazard roles only, with destruction kept per candidate even for unresolved operand layouts, including receiver-method candidates; no physical stores, successful transfer, purity, body traversal or compiler operation proof | none |
| decoded original command lookup value | `rust/tcl-compiler/src/registry_invocation.rs` | `static_command_word`; `effective_invocation_word` | retained escape and word rules; original static head only; no implementation, compiler-hook or dispatch proof; dynamic/expanded heads decline | none |
| native compiler operands and cache mutation | `rust/tcl-registry/src/native_compilation.rs`; `rust/tcl-registry/src/native_instruction_plan.rs`; `rust/tcl-registry/src/native_procedure.rs`; `rust/tcl-registry/src/native_ensemble.rs`; `runtime/rust/src/namespace.rs`; `runtime/rust/src/interp/native_compilation.rs` | `NativeCompilationSpec::requires_original_word_preparation`; `native_instruction_plan`; `native_registered_worker_instruction_plan`; `NativeInstructionPlan`; `native_compiler_cache_invalidated`; `configuration_compiler_mutations`; `Interp::native_compiler_cache_epochs` | original complete compiler words, retained physical selection/context, actual raw compiler attachment, independent interpreter/compiler and namespace/resolver counters; copied import attachments, genuine token retirement, real setter transactions and first/last execution trace transitions; public and actual registered-worker grammar validation retain distinct operand purposes over the same complete original vector; recipe projection supplies no indexed-local or executable capability | none |
| original compiled named invocation | `rust/tcl-registry/src/native_instruction_plan.rs`; `rust/tcl-registry/src/native_command_literal.rs`; `runtime/rust/src/interp/native_body_artifact/native_named.rs` | `NativeNamedInvocationInstruction`; `NativeNamedInvocationWord`; `native_named_invocation_instruction`; `NativeInstructionPlan::NamedInvocation`; `NativeNamedWorkerCompilation`; `native_named_worker_instruction`; `native_compiled_command_name_literal_from_lookup` | independently admitted selection, retained original operands/expansion flags and canonical replacement layout; same private pooled head at compilation, Direct before operand preparation and Rewrite after original/canonical argv preparation; actual handler resolves by ordinary lookup after argv effects; pure layout grants no binding or callable identity, and unavailable dynamic rewrite extent remains a typed residual | `xtask-owner-resolution` |
| native compiler parser word projection | `rust/tcl-registry/src/native_compiler_word_projection.rs` | `NativeCompilerWordOperand`; `NativeProjectedCompilerWord`; `project_native_compiler_words` | original C8.5+ pure TEXT argument expansion, native source channel and exact list member spans; malformed, backslash-generated and substituted expansions remain expanded; no runtime List header, callback or executable authority | none |
| native namespace binding compilation | `rust/tcl-registry/src/native_namespace_binding_compilation.rs`; `rust/tcl-registry/src/native_namespace_upvar_compilation.rs`; `rust/tcl-compiler/src/codegen/native_namespace_upvar.rs`; `runtime/rust/src/interp/native_body_artifact.rs` | `NativeNamespaceBindingKind`; `NativeNamespaceBindingCompilation`; `NativeNamespaceBindingOutcome`; `NativeNamespaceBindingVisit`; `compile_native_namespace_bindings`; `compile_native_namespace_upvar` | actual C8.5–9.1 procedure compiler; release-specific final TEXT tails and counted local names; global/variable declare locals before name/value visits, while C8.5 monolithic Upvar visits its original namespace then each target before local declaration; raw member and SIMPLE_WORD locals gate selection; prefix pools/LVT survive Generic instruction rollback without executing partial bindings; Runtime retains the original namespace across targets and repeats its native lookup after each target; handler identity, live aliases and original name caches remain independent | none |
| original native compiler source selection | `rust/tcl-compiler/src/registry_invocation/native_compilation_source.rs`; `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/command_binding/compiler_inventory.rs`; `rust/tcl-lsp-core/src/refactor/mod.rs` | `native_compilation_syntax`; `SourceCommandBindings::switch_compilation_at`; `SourceCommandBindings::namespace_binding_preparation_at` | exact full parser-word vector and original source channel; purpose-specific namespace and switch recipes retain original spans and compiler visits, while changed, truncated or missing source withdraws selection; internal implementation: `OriginalNativeCompilerInvocation`, `OriginalNativeCompilerPreparation`, `original_native_compilation`, `original_native_compiler_words` | none |
| original structured compiler preparation | `rust/tcl-compiler/src/command_binding/compiled_invocation.rs`; `rust/tcl-compiler/src/command_binding/compiler_inventory.rs`; `rust/tcl-registry/src/native_instruction_plan.rs` | `SourceCommandBindings::structured_compilation_at`; `SourceInvocationBinding::original_structured_compilation`; `SourceNativeStructuredPreparation::compilation_site`; `SourceNativeStructuredPreparation::recipe`; `SourceNativeStructuredPreparation::dependency`; `NativeInstructionPlan::expression_program`; `NativeInstructionPlan::body_error_context`; `NativeInstructionPlan::expression_error_context` | unanimous original allocation, complete unchanged word vector, source image/channel, compiler policy, namespace, table and actual registration dependency; ordered preparation survives Generic decline and exact Rejected failures remain distinct from unavailable evidence; original operand projections supply checked expression trees and native error-context annotations without generic-argv body-index recovery, normal-handler effects or reached execution authority | none |
| original namespace compiler preparation | `rust/tcl-compiler/src/command_binding/compiled_invocation.rs`; `rust/tcl-compiler/src/command_binding/compiler_inventory.rs` | `SourceNativeNamespaceBindingPreparation`; `namespace_binding_preparation_at` | original complete source vector, exact allocation and independent compiler dependency; retained declaration geometry includes declined prefixes and grants no live alias, store or executable operation | none |
| native return compilation and original option merge | `rust/tcl-registry/src/native_return_compilation.rs`; `rust/tcl-cmd-core/src/native_return_merge.rs` | `NativeReturnInstruction`; `NativeReturnOptionsOperand`; `NativeReturnExit`; `native_return_instruction`; `NativeReturnMergeObjects`; `MergedNativeReturnOptions`; `merge`; `merge_stack` | actual C8.5–9.1 compiler selection, original static or stack options, version-specific recursive Dict/List merge and retained private headers; control receipts remain separate from procedure-boundary settlement | none |
| original Error compiler instruction | `rust/tcl-registry/src/native_error_compilation.rs`; `rust/tcl-registry/src/native_instruction_plan.rs`; `rust/tcl-compiler/src/codegen/native_error.rs`; `runtime/rust/src/interp/native_body_artifact/native_error.rs` | `compile_native_error`; `native_error_instruction`; `NativeErrorInstruction`; `NativeErrorStep` | original parser vector and expansion-member geometry; selected C8.6/C9.0 List pairs or C9.1 Dictionary insertion, message then keyword/value operand order and immediate Error Return; C8.4/C8.5 decline remains generic; pure recipe supplies no installed compiler hook, source attestation, original header or Return-command grammar authority | `xtask-owner-resolution` |
| original native coroutine compilation and transport | `rust/tcl-registry/src/native_coroutine_compilation.rs`; `rust/tcl-registry/src/native_instruction_plan.rs`; `rust/tcl-compiler/src/codegen/native_coroutine.rs`; `runtime/rust/src/interp/native_body_artifact/native_coroutine.rs`; `rust/tcl-vm/src/cmd_coro.rs`; `runtime/rust/src/cmd_coro.rs` | `compile_native_coroutine`; `NativeCoroutineInstruction`; `NativeCoroutineStep`; `NativeCoroutineCompilationUnavailable` | actual C8.6–9.1 original words, frame, arity and expansion; release-specific namespace capture and original head/target literal roles; unchanged namespace-prefixed List and value headers cross dispatch or serialised suspension; direct creation argv has no generated source authority; wire probe/inject APIs and unavailable wasm suspension remain separate; compiler hook, header producer and executable coroutine capability require their own issuers  InfoCoroutine separately retains the original C8.6/C9 zero-worker-argument Name recipe from pinned registration/worker source; no live compiler token, holder or public completion follows from that source fact;| `xtask-owner-resolution` |
| native switch compilation | `rust/tcl-registry/src/native_switch_compilation.rs` | `NativeSwitchInstruction`; `NativeSwitchArm`; `NativeSwitchMode`; `NativeSwitchMatch`; `native_switch_instruction` | actual selected C compiler, original subject and exact pattern/body geometry, ordered arm compilation and version-specific matching operations; omitted or masked bodies supply no preparation authority | none |
| native regexp cache and equivalent-glob conversion | `rust/tcl-syntax/src/native_regex.rs` | `NativeRegexpRecipe`; `NativeRegexpRange`; `NativeRegexpCache`; `equivalent_glob` | actual C8.4–9.1 RegExp flags and retained compiled artifact, counted TclReToGlob conversion and selected zero-range subject access; equivalent glob does not replace full capture-range matching | none |
| original object append and working transports | `rust/tcl-syntax/src/native_object_append.rs`; `rust/tcl-registry/src/native_object_append.rs`; `rust/tcl-cmd-core/src/native_append.rs`; `rust/tcl-cmd-core/src/native_cat.rs`; `runtime/rust/src/value_ops.rs`; `runtime/rust/src/interp/native_append.rs`; `runtime/rust/src/frame/pending_scalar.rs`; `runtime/rust/src/frame/rmw.rs` | `NativeObjectAppendProtocol`; `NativeObjectCatProtocol`; `native_object_append_protocol`; `NativeAppendObjects`; `NativeCatObjects`; `concatenate_jim`; `concatenate_jim_checked`; `PreparedAppendValue`; `append_object`; `append_unicode_units`; `append_counted_bytes`; `append_published_counted_source_checked`; `NativeAppendVariable`; `append_operands`; `RuntimeAppendValue` | actual native string issuer, original source/receiver primary and resident storage, selected Unicode/binary append and real copy-on-write ownership; working transports keep borrowed originals separate from retained prepared objects, with private constructors and no external native-reference authority; the selected C8.4 counted variable worker publishes a shared duplicate as an allocation-only pending role, retires the actual old cell reference outside borrows, then takes cell +1 only for the same live slot/ticket; unshared receiver ref1 and the counted message retain their genuine roles, while ordinary checked reads refuse an unsettled role | none |
| original regexp compilation and matching | `rust/tcl-cmd-core/src/regex.rs`; `rust/tcl-syntax/src/native_tcl_utf.rs`; `rust/tcl-regex/src/jim.rs` | `NativeRegexSource`; `NativeRegexObjects`; `PreparedOriginalRegex`; `prepare_pattern_original`; `prepare_search_pattern_original`; `execute_pattern_original`; `compiled_match_original`; `NativeTclUtf`; `TclUtfUnit`; `jim::Flags::new`; `with_nocase`; `with_lineanchor`; `with_linestop`; `with_expanded`; `jim::Flags::key` | selected native engine, original pattern cache and subject primary, flags and counted character units; cache hits precede getters and reached output setters retain original target objects; Jim Flags builder retains actual 2/4/8/32 bits independently of C ARE flags and cannot grant compiled-program identity | none |
| native TclOO helper compilation and registration | `rust/tcl-registry/src/native_tcloo_compilation.rs`; `rust/tcl-registry/src/native_tcloo_registration.rs`; `rust/tcl-syntax/src/native_namespace_name.rs` | `NativeTclOoHelper`; `NativeTclOoInstruction`; `NativeTclOoObjectInfo`; `NativeTclOoObjectInfo::uses_execution_constant`; `select_original`; `instruction`; `definition_identity`; `compilation`; `NativeNamespaceObjectProducer::ObjectNamespace` | actual qualified helper or definition worker registration, original compiler words and selected C8.6–9.1 expansion grammar; method helpers independently require an issued frame, while ObjectInfo retains its original operand and selected object getter; Class uses the retained original class-name String, Namespace uses the actual object namespace incarnation, standalone IsObject uses interpreter execution constants and CreationId is C9.1; spelling and pure instruction geometry supply no object, frame or cache authority | none |
| original native unset compilation | `rust/tcl-registry/src/native_unset_compilation.rs`; `rust/tcl-compiler/src/codegen/native_unset.rs`; `runtime/rust/src/interp/native_body_artifact/native_unset.rs`; `runtime/rust/src/interp/native_variable_names/native_unset.rs` | `compile_native_unset`; `NativeUnsetInstruction`; `NativeUnsetVariable`; `NativeUnsetReceiver`; `NativeUnsetUnavailable` | actual C8.6+ hook and complete original parser projection; flag/known-word validation precedes preparation, each original root/index operand precedes its own unset, and complain semantics remain explicit; grouped words and literal expansion members retain different geometry; old-release no-hook and native declines stay Generic, unavailable geometry is a typed refusal; no live cell, runtime cache, trace order or successful destruction follows from selection | `xtask-owner-resolution` |
| original stock TclOO factory/bootstrap roles | `rust/tcl-registry/src/native_tcloo_bootstrap.rs`; `rust/tcl-compiler/src/command_binding/registered_class_factory.rs`; `rust/tcl-registry/src/registry.rs` | `NativeClassFactoryRecipe`; `NativeConfigurableSupportRecipe`; `CommandRegistry::native_class_factory_recipe`; `OriginalClassFactoryState` | release-selected native constructor roles, original support class and persistent definition namespace/path recipe; source-created classes retain actual selected factory/support allocations independently of source bodies, current class topology, compiler preparation and successful completion; native snapshots do not inherit missing class roles from Registry metadata | none |
| intrinsic original stock class creation | `rust/tcl-compiler/src/command_binding/original_class_factory_transfer.rs`; `rust/tcl-compiler/src/command_binding/deferred_method.rs` | `OriginalClassFactoryTransfer::capture`; `OriginalClassFactoryTransfer::completed`; `closed_definition_member` | quiet selected stock constructor, exact static original argv and complete native declaration workers, fresh valid public slot and same resulting class/allocation/publication; only the shared pre/post transfer preserves Normal, independently of source inventories and stored body execution | none |
| native property lookup and invalidation | `rust/tcl-registry/src/native_property_lookup.rs` | `NativePropertyLookupProtocol`; `NativePropertyInvalidation`; `native_property_lookup_protocol`; `native_property_declaration_validity` | independently selected C9.0/9.1 pure declaration validity; actual C9.1 property declaration, original counted member and accessor names, TEMP_TABLE lookup and direction-specific mutation invalidation; property List ownership remains independent of variable declaration tables | none |
| accepted integer contents conversion | `rust/tcl-compiler/src/native_numeric.rs`; `rust/tcl-compiler/src/command_binding/expression_preparation.rs` | `SourceExpressionPreparation`; `SourceInvocationBinding::expression_preparation`; `SourceInvocationBinding::original_arguments_complete_normally` | exact completed physical read, closed current cell/formal and selected C8.5–9.1 integer conversion; accepted contents remain separate from current numeric primary and allocation identity; grants no Boolean, String, List, comparison, result-normalisation or observer authority; internal implementation: `SourceIntegerContentsRead`, `source_read_integer_contents` | none |
| native interpreter option tables | `rust/tcl-registry/src/native_interpreter_options.rs` | `NativeInterpreterOptionProtocol`; `native_interpreter_option_protocol` | original root/child command declarations and independent C9 secondary table order; recognised unavailable handlers retain their native option positions and cache behavior | none |
| native Jim enum and compared-string caches | `rust/tcl-registry/src/native_jim_enum.rs`; `rust/tcl-core-types/src/native_jim_enum.rs` | `NativeJimEnumProtocol`; `NativeJimOptionCache` | actual Jim enum table identity and every flags bit; original cache hit precedes string access, failures retain existing primary; compared-string caches have their own identity and supply no C Index authority | none |
| original Jim index expression and cache | `rust/tcl-syntax/src/native_jim_index.rs`; `rust/tcl-vm/src/value_ops.rs`; `rust/tcl-vm/src/interp.rs`; `runtime/rust/src/value_ops.rs`; `runtime/rust/src/builtins.rs` | `JimIndex`; `JimIndexExpression`; `JimIndexEvaluationError`; `evaluate_index` | actual safe GetWideExpr on the same original ordinary operand, or a fresh counted end-expression suffix; existing Int bypass and Index hit-before-getter remain distinct; expression completion and host refusal retain separate channels; successful conversion installs the same-header Jim Index without C Index authority | none |
| original C switch handler selection | `rust/tcl-cmd-core/src/switch.rs`; `rust/tcl-syntax/src/native_glob.rs`; `rust/tcl-vm/src/cmd_switch.rs`; `runtime/rust/src/cmd_switch.rs` | `parse_options`; `usage`; `select_original`; `body_is_fallthrough`; `equal_c_strings`; `match_c_string_glob` | actual C release, original native string getters and CString default/Exact/Glob/body extents; opaque bytes remain native inputs; Regexp and Integer retain independent original object conversions; handler comparison does not grant compiler primitive authority | `xtask-owner-resolution` |
| original Jim switch selection | `rust/tcl-syntax/src/native_jim_switch.rs`; `rust/tcl-registry/src/native_jim_switch.rs`; `rust/tcl-cmd-core/src/native_jim_switch.rs`; `rust/tcl-vm/src/cmd_switch/native_jim.rs`; `runtime/rust/src/cmd_switch/native_jim.rs` | `NativeJimSwitchProtocol`; `NativeJimSwitchOption`; `native_jim_switch_protocol`; `NativeJimSwitchObjects`; `Immediate`; `select`; `Selection`; `Failure` | actual pinned Jim two-byte option scan, original command and current borrowed case List members; last mode and sticky regexp end-option flag; genuine default/dash ComparedString comparison; integer callback result and unchanged completion, with same-List refetch before code inspection; actual empty-result reset before original body execution; typed host refusal remains separate | `xtask-owner-resolution` |
| original Jim lsearch selection and callbacks | `rust/tcl-cmd-core/src/native_jim_lsearch.rs`; `rust/tcl-cmd-core/src/regex.rs`; `rust/tcl-vm/src/cmd_list.rs`; `runtime/rust/src/cmd_list.rs`; `rust/tcl-vm/src/cmd_regexp.rs`; `runtime/rust/src/cmd_regex.rs` | `NativeJimLsearchObjects`; `JimLsearchError`; `lsearch` | actual Jim issuer, exact Enum table, default exact search, original Index conversion before List descent and borrowed original children; selected List/head native holds and result publication precede their release; callback integer is preserved for equality and Boolean inversion, and callback failure becomes lsearch Error; internal implementation: `invoke_jim_match_command` | `xtask-owner-resolution` |
| original receiver declarations and caller navigation | `rust/tcl-compiler/src/command_binding/conditional_body.rs`; `rust/tcl-compiler/src/command_binding/declaration_layout.rs` | `SourceDeclaredReceiverBodyEntry`; `SourceDeclaredReceiverBodyEntry::declaration_namespace_context`; `SourceCallerFrameInvocationTemplate`; `SourceCallerFrameInvocationTemplate::namespace_context` | original native formals and declared receiver body scope retained for navigation; procedure entries retain their exact body namespace key, receiver previews have no entered namespace context; declaration namespace provenance grants no method implementation, receiver allocation, physical caller or successful store | none |
| conditional declared overwrite advice | `rust/tcl-compiler/src/command_binding/declaration_flow.rs`; `rust/tcl-compiler/src/command_binding/declaration_layout.rs`; `rust/tcl-compiler/src/registry_invocation/declaration_flow.rs`; `rust/tcl-compiler/src/registry_invocation/store_advice.rs`; `rust/tcl-compiler/src/analyser/diagnostics.rs` | `Analyser::emit_cfg_ssa_diagnostics_for_function_full` | exact adjacent literal setters, retained source/frame/prefix/path and warning-only declaration flow; an abrupt read retains later original layouts under its actual lookup table until an unknown, substituted or lookup-changing operation; no reached arguments, compiler visits, physical Place, current contents or executable removal authority; internal implementation: `conditional_declared_overwrite_advice`, `record_unentered_declaration_suffix` | none |
| native compiler selection and exact replay | `rust/tcl-runtime-api/src/native_compilation.rs`; `rust/tcl-bytecode/src/lib.rs`; `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/codegen/cmd_subst.rs` | `NativeCompilerSelectionPrerequisite`; `NativeCommandCompilerPrerequisite::matches_registration_with`; `NativeEnsembleCompilerPrerequisite`; `NativeCompilerSelectionSite`; `native_compiler_replay_source` | actual procedure header or ensemble compiler/configuration independent of runtime identity; selected worker compiler capability retained at admission; public configuration/cache validation independent of live worker-hook changes; chunk/before-argv guard once, late name dispatch and exact original-command replay; internal implementation: `emit_native_compiler_selection` | none |
| actual selected ensemble worker compiler | `rust/tcl-registry/src/native_selected_worker.rs`; `rust/tcl-registry/src/native_compilation.rs`; `rust/tcl-registry/src/native_ensemble.rs` | `compile_original_selected_worker`; `compile_original_selected_worker_path`; `OriginalSelectedWorkerInvocation`; `OriginalSelectedWorkerCompilation`; `OriginalSelectedWorkerPathCompilation`; `original_ensemble_selector_at`; `NativeCompilationSpec::select_registered_worker_native_words` | actual retained worker registration/compiler and complete original source words; recursive selection retains original map-prefix objects and every intermediate compiler/configuration prerequisite before operands, with the final leaf independent of late handler lookup; execution-trace veto keeps public Generic, absent hooks and present-hook decline use the release-selected named rewrite policy, and admitted worker grammars retain exact direct named or operation preparation; unknown hooks/geometry decline; no public lookup, configuration, runtime binding or callable authority follows | `xtask-owner-resolution` |
| original native dictionary lookup compilation | `rust/tcl-registry/src/native_dictionary_compilation.rs`; `rust/tcl-registry/src/native_dictionary.rs` | `compile_native_dictionary_lookup`; `NativeDictionaryCommand::select_original_lookup`; `NativeDictionaryLookupInstruction`; `NativeDictionaryLookupKind`; `NativeDictionaryCompilationUnavailable` | actual command compiler-hook release, original dictionary then ordered keys then default, nonempty key count and parser-owned literal expansion geometry; dynamic expansion and declined native arity remain Generic, unavailable operation/source remains distinct; no dictionary parsing, getter, callback, binding or operand completion occurs during selection | `xtask-owner-resolution` |
| runtime command guard ownership | `rust/tcl-runtime-api/src/guard.rs`; `rust/tcl-vm/src/interp.rs`; `runtime/rust/src/interp.rs` | `GuardManager`; `OwnedGuardManager`; `OwnedGuardManager::poison` | shared checked epoch and token allocation; exact live command allocation captured at issuance; equal semantic identities on replacement allocations cannot revive issued guards; namespace and command mutations invalidate dependent snapshots while untouched allocations retain their attested identities | `xtask-owner-resolution` |
| retained module semantic ingress | `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/lowering/mod.rs` | `Module::resolved_profile`; `Module::resolved_registry`; `Module::number_syntax`; `Module::word_values` | immutable actual registry snapshot, resolved custom profile, exact lexer configuration and independent invocation policies; explicit legacy compatibility fallback only where ingress carriers are absent | none |
| complete retained Module source correspondence | `rust/tcl-compiler/src/command_binding/retained_source.rs`; `rust/tcl-compiler/src/optimiser/propagation.rs`; `rust/tcl-compiler/src/optimiser/method_barrier.rs`; `rust/tcl-compiler/src/registry_invocation/source_structure.rs`; `rust/tcl-compiler/src/analyser/class_lattice.rs`; `rust/tcl-compiler/src/analyser/diagnostics/var_command.rs` | `RetainedSourceModuleBindings::matches_module`; `retained_substitution_calls`; `proc_dispatch_facts`; `original_registry_words_for_module` | original source image, entry/input/configuration, printed/native namespace, typed top_level_namespace_context, top-level kind and Registry semantic key are independent owner premises; nested substitution inventory and dispatch use actual availability/grammar and refuse changed owner; a valid metadata handle supplies no Native holder/frame/handler or executable erasure; conditional original OO receiver and Return operand analysis uses complete current Module input, positioned retained attachments, unchanged original words/argv and full configuration before shared source-schema projection; only selected Written operands become scalar-source advice, with Normal completion, callable topology, entered frame and Native lookup still independent | none |
| current source refactor and CLI reduction metadata | `rust/tcl-lsp-core/src/refactor/extract_variable.rs`; `rust/tcl-lsp-core/src/refactor/extract_proc.rs`; `rust/tcl-lsp-core/src/refactor/source_rewrite.rs`; `rust/tcl-cli/src/commands/minimize.rs`; `rust/tcl-syntax/src/naming.rs` | `original_assignment_candidate`; `select`; `minimize_diagnostic_with_input`; `collect_rename_edits`; Syntax `is_qualified` and `root_unrooted_key` | complete AnalysisResult source/input/configuration/availability and original lookup govern setter/hook and guarded variable plans; changed text is reanalysed under the same input; symbol plans with shared containers are atomic; missing/foreign/stale ownership refuses; conditional advice grants no Native inserted store, source motion or executable replacement; qualification and fresh unrooted-key projection delegate Syntax | none |
| original selected Core dispatch and namespace hazards | `rust/tcl-lsp-core/src/references.rs`; `rust/tcl-lsp-core/src/namespace_rename.rs` | `RetainedDispatchContext`; `retained_dispatch_context`; `scan_my_method_sites`; `namespace_rename_hazard` | complete actual source image/input/configuration, retained Realm/Registry and availability select original custom dispatch traits and written namespace roles; known replacements and missing/foreign/stale owners refuse method references or retain namespace blockers; conditional source regions and hazard inventories supply no live receiver, Native method/frame/holder or executable-edit licence | none |
| nested codegen lexical source operands | `rust/tcl-compiler/src/codegen/statements.rs` | `CodegenCtx::nested_command_tokens` | retained ingress LexerConfig and genuine whole command-substitution source geometry preserve nested value tokens independently of catalogue profile; only the owning invocation inherits exact-site source bindings, while reparsing or absent parent keeps lookup unproved; lexical shape grants no Native selection, emitted instruction or body/frame admission | none |
| supplied document compiler findings and unindexed tokens | `rust/tcl-lsp-db/src/lib.rs`; `rust/tcl-lsp-server/src/lib.rs` | `compilation_unit_uncached_from_analysis`; `compiler_check_diagnostics_uncached_from_analysis`; `Backend::compiler_diagnostics_for`; `unindexed_semantic_tokens`; `Backend::semantic_tokens_core_data`; `code_action_report` | current whole document, complete AnalysisResult input/configuration/actual Registry availability and typed generation status own the uncached source build; workspace pack roles and diagnostic subjects/source contexts remain retained by the same analysis; missing/foreign/stale/unavailable supplied ownership refuses compiler/token advice, with no default fallback, Native entry or executable-edit grant | none |
| original source index diagnostics | `rust/tcl-registry/src/source_index_bounds.rs`; `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-compiler/src/analyser/bounds_checks/source_index.rs`; `rust/tcl-lsp-core/src/diagnostic_subject.rs`; `rust/tcl-spec-studio/src/render_spectcl.rs` | `SourceIndexBounds`; `SourceIndexBoundsInvocation`; `ResolvedInvocation::authored_source_index_bounds`; `original_index_diagnostics`; `DiagnosticSubjectData::from_diagnostic`; `render` | selected operation, exact effective source count/offset and retained availability/dialect own conditional literal index advice; supplemental values remain distinct from original words and captured/written geometry; typed purpose transports independently of messages and authored descriptors survive render/reload; no current container, evaluated argv, Native handler/frame or erasure grant | none |
| supplied unindexed viewport and deep compiler input | `rust/tcl-lsp-server/src/lib.rs` | `unindexed_semantic_tokens_for_range`; `compute_compiler_diags_after_base`; `compute_compiler_diags` | full/range tokens and the no-SourceFile deep worker borrow the same current document/base input and actual workspace Registry; missing/stale/unavailable source metadata withholds advice, while indexed snapshot work, cross-file evidence and cancellation remain separate; no donor profile or Native entry | none |
| original embedding publication and lossless fact transport | `runtime/rust/src/engine.rs`; `runtime/rust/src/interp/native_host_publication.rs`; `rust/tcl-syntax/src/scalar_getter/carrier.rs` | `PublicationService`; `authenticate`; `consume`; `RuntimeEngine::command_publication_service`; `RuntimeEngine::restrict_commands`; `host_command_proc`; `to_obj`; `snapshot_value`; `fail`; `capture_answer`; `import_scalar`; `export_scalar`; `import_storage`; `export_storage` | actual interpreter/scope/purpose/occupancy own one-use immediate receipts and retained host/unit generations; typed host refusal stays outside guest completion; exact counted callback bytes and full scalar facts preserve independent origin/storage tags; codecs grant no publication or cache adoption authority; unavailable original-object/deferred-overlay and guest retirement work refuse; software controls supply no external Native table/header/getter/frame equivalence | none |
| neutral retained scalar and resident-storage carriers | `rust/tcl-core-types/src/native_value.rs`; `rust/tcl-syntax/src/scalar_getter/carrier.rs` | `NativeScalarCache`; `NativeCVersion`; `NativeIntegerRadix`; `NativeStringStorageIdentity`; `import_scalar`; `export_scalar`; `import_storage`; `export_storage` | neutral enums retain full payload, independent descriptor release and explicit storage tags; shared codecs preserve these recorded facts without narrowing magnitude or inferring allocation from bytes; the receiving engine separately validates actual origin/storage/object authority; no additional numeric getter, native cache production, live header, updater or publication permission | none |
| reached original error-context ownership | `rust/tcl-vm/src/value.rs`; `rust/tcl-vm/src/native_list_backing.rs`; `rust/tcl-vm/src/interp/native_error_stack.rs` | `Value::capture_native_error_context`; `NativeListItems::owns_native_header`; `NativeErrorStack::begin_inner`; `NativeErrorStack::begin_instruction`; `NativeErrorStack::restart_inner` | actual reached capture alone turns borrowed original invocation members into an owning List; live header/selected protocol validation and typed first Host cause remain exact; software reference counts supply no external private header or Native entry/frame/instruction grant | none |
| selected authored command-prefix data | `rust/tcl-compiler/src/command_binding/source_transition_advice/produced_prefix.rs`; `rust/tcl-compiler/src/registry_invocation/source_structure.rs` | `OriginalSourceAuthoredCommandPrefix`; `source_authored_command_prefix` | current genuine list builder and conditional target roles/origins retain whole source/configuration/actual availability; data-only applicability has no deferred Produced parent, future lookup/value, Native frame/dispatch or edit authority | none |
| checked tracked document source input | `rust/tcl-compiler/src/analyser/state.rs`; `rust/tcl-lsp-db/src/lib.rs`; `rust/tcl-lsp-server/src/lib.rs` | `Analyser::prepare_analysis_input`; `document_analysis_input`; `Backend::analyse_with_workspace_classes_from_analysis` | one checked complete editing input owns source walk, memoised procedure grammar and class-hint reanalysis under real resource settings; keyed pack availability and typed OverlayMiss stay independent of equal store identity; missing/stale/foreign/unavailable refuses before cache advice | none |
| installed embedding handles and typed settlement | `runtime/rust/src/engine.rs`; `runtime/rust/src/interp.rs`; `rust/tcl-vm/src/embed.rs`; `rust/tcl-engine-tclvm/src/lib.rs` | `RuntimeHandle`; `Interp::install_proc_original_storage`; `Vm::begin_embedding_call`; `Vm::finish_embedding_result` | actual installed generation/interpreter owns compiled handle entry; complete original Guest result/options and typed first Host cause remain separate through variable/package/Engine/stub transport; no new Native cache/header/table/frame or callback proof | none |
| original Logical procedure completion and no-error source calls | `rust/tcl-compiler/src/command_binding/retained_source.rs`; `rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs`; `rust/tcl-compiler/src/interprocedural/completion.rs`; `rust/tcl-compiler/src/optimiser/elimination.rs`; `rust/tcl-compiler/src/inlining/frame.rs` | `RetainedSourceModuleBindings::matches_original_procedure_summary`; `original_logical_procedure_calls_for_module`; `original_procedure_formal_count_shape`; `original_procedure_scalar_bindings`; `CompletionWalk`; `procedures_complete`; `RaiseProof::calls_complete`; `SourceContext::matches_procedure` | complete actual Logical Module/point, exact original allocation/header/body/formals and authentic effective argv/child horizons own conditional completion/read/count advice; definition reach, source purity, observers and edit/admission requirements remain independent; Native-only, missing/foreign/changed owners decline rather than borrowing labels or frames | none |
| retained expression assistance and unavailable callback source joins | `rust/tcl-compiler/src/analyser/diagnostics/usage.rs`; `rust/tcl-compiler/src/analyser/diagnostics/validity.rs`; `rust/tcl-compiler/src/analyser/commands.rs`; `rust/tcl-compiler/src/analyser/types.rs`; `rust/tcl-compiler/src/analyser/diagnostic_subject/callback.rs` | `Analyser::original_expression_parser_context`; `gated_operator_assistance_context`; `Analyser::expr_function_calls`; `AnalysisResult::matches_original_source_image`; `AnalysisResult::allows_retained_logical_declaration_advice`; `SourceCallbackSignatureLookup::from_original_lookup` | actual checked lexer/variable/numeral grammar and genuine spans own operator/function recognition; recognition vocabulary cannot donate evaluation; typed unavailable generation withdraws original signature/source/Logical compatibility even with copied complete input and populated receipts; no Native callback/math Handler, frame or edit authority | none |
| current structural diagram source metadata | `rust/tcl-diagram/src/data.rs` | `DiagramContext::metadata_context`; `registry_command`; `command_completion` | whole original source, actual analysis/input/configuration and matching availability select contextual command/completion advice independently of labels; unavailable or changed source owner refuses; structural metadata is not an executed completion or reachability observation | none |
| retained analyser semantic ingress | `rust/tcl-compiler/src/analyser/input.rs`; `rust/tcl-compiler/src/analyser/state.rs`; `rust/tcl-compiler/src/analyser/types.rs`; `rust/tcl-registry/src/model/assembly.rs`; `rust/tcl-lsp-core/src/lib.rs` | `ResolvedAnalysisInput`; `analyser_profile`; `unit_profile`; `context_registry`; `lexer_config`; `with_resolved_input`; `resolved_analysis_input`; `resolved_profile`; `resolved_registry`; `retained_command_realm`; `with_command_store`; `profile_for_analysis`; `registry_for_analysis` | exact analyser/unit profiles, body grammar and immutable availability generation; full/chunked/incremental/isolated inputs and cache keys retain the same axes; original temporal realm compares semantic inventories rather than derived caches; editing metadata remains independent of native execution admission; name assistance only for legacy results without retained input; missing temporal realm remains unknown | none |
| attested native mathop handler selection | `rust/tcl-cmd-core/src/mathop.rs`; `rust/tcl-vm/src/cmd_mathop.rs`; `runtime/rust/src/cmd_mathop.rs` | `operation_for_handler_identity` | only an attested registered mathop implementation identity selects the operation; rename, alias, import and current argv remain independent; successful dispatch never reads argv0 for the operation, while the selected native arity presenter retains the original head | none |
| native procedure references and retirement | `rust/tcl-runtime-api/src/native_procedure_roles.rs`; `rust/tcl-vm/src/command/native_procedure.rs`; `runtime/rust/src/interp/native_procedure_resources.rs`; `runtime/rust/src/obj/native_lambda_expression.rs` | `NativeProcedureRoleLedger`; `NativeProcedureReference`; `NativeProcedureBinding` | actual command, frame and primary roles independent of allocation-only transports; final resource retirement; same-binding client-data replacement; borrowed queries acquire no role; commandless methods retain genuine Proc clientData and apply retains its original lambdaExpr primary plus absolute namespace object; a lambda-header duplicate acquires real Proc/namespace roles, while method-copy declarations create a new Proc and string-only body with the same original defaults; internal implementation: `NativeCallableProcedure`, `duplicate_lambda_role`, `duplicate_method`, `LambdaExpression` | none |
| actual empty procedure compilation | `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-runtime-api/src/native_compilation.rs`; `rust/tcl-bytecode/src/lib.rs`; `rust/tcl-registry/src/native_procedure.rs` | `NativeProcedureNoOpPlan`; `native_procedure_noop_plan`; `NativeProcedureHeaderPrerequisite`; `NativeCompilerSelectionSite`; `procedure_header_compilation` | actual callable/header and namespace allocation, original lookup word, body-object compilation state, chunk/before-argv validation and retained selection across substitutions | none |
| closed BPF conditional syntax | `rust/tcl-registry/src/bpf_op.rs`; `rust/tcl-registry/src/registry.rs`; `rust/tcl-compiler/src/lowering/mod.rs`; `rust/bpf-tcl-ir/src/source.rs` | `BpfOpKind`; `CommandRegistry::bpf_conditional_operands`; `source_nesting_limit` | independently authored static language descriptor and shared validated clause grammar; actual original literal words and affine source bytes, no expansion or dynamic source; existing construction bound and actual lexer configuration; derived bodies retain syntax spans and discard independent Tcl execution/admission receipts; unexpanded conditions are explicitly rejected | none |
| structured script operand extraction | `rust/tcl-compiler/src/codegen/mod.rs`; `rust/tcl-compiler/src/lowering/structured.rs`; `rust/tcl-compiler/src/optimiser/helpers/tokens.rs`; `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/optimiser/mod.rs` | `extract_body_text`; `Module::lexer_config`; `PassContext::lexer_config`; `CodegenCtx::lexer_config` | complete written-word ranges; exact retained ingress grammar; original static word value; verbatim evaluated script bytes and completion-preserving splice; internal implementation: `script_word_span` | none |
| Standalone static source fragments | `rust/tcl-syntax/src/word_rules.rs`; `rust/tcl-compiler/src/codegen/emitter/bytecoded.rs` | `standalone_static_word_fragment_value` | caller retains explicit post-grouping text, braced fact and actual LexerConfig; braced fragments apply only brace continuation, while unbraced fragments require wholly literal/escape components; empty and literal sigil bytes remain data; source-backed or native-entry hooks cannot borrow this source-absent model, and the syntax value grants no original source receipt, handler, cell or slot authority | `xtask-owner-resolution` |
| Normalised compiled-word protocol | `rust/tcl-syntax/src/word_rules/compiled_word.rs`; `rust/tcl-compiler/src/codegen/values.rs`; `rust/tcl-vm/src/subst.rs` | `CompiledWordSubstitution`; `compiled_word_substitution`; `push_compiled_word`; `subst_word_bytes` | already normalised bytes and actual LexerConfig select LiteralValue or Deferred; bare dollar bytes remain data, while `${`, `[` and configured Jim `$(` markers retain runtime scanning, including malformed deferred input; finished values use exact literal emission and original source uses its own grammar; the independent VM whole-braced-word gate remains earlier, and marker selection grants no name lookup, variable read, frame or successful evaluation | `xtask-owner-resolution` |
| word values (brace / list axes, braced-word recognition) | `rust/tcl-syntax/src/word_rules.rs` | `WordValueRules`; `collapse_braced_word`; `split_list`; `split_list_tolerant`; `split_word_names`; `whole_braced_word` | `LexerGrammar::brace_backslash_newline` and `list_parse` per dialect, carried together because a word-shaped list asks both; brace *balance* is release-invariant, so `whole_braced_word` takes no rules | none |
| numbers | `rust/tcl-syntax/src/number.rs`; `rust/tcl-dialect/src/expr_number.rs`; `rust/tcl-dialect/src/grammar.rs` | `parse`; `parse_whole_with`; `is_expr_number`; `scan_expr_number`; `scan_nan_payload`; `native_int32_low_bits`; `NumberSyntax` | `NumberSyntax` and expression-word grammar per release; low signed 32-bit projection follows an independently selected native getter, validated operand range and physical width, and grants no conversion success or width authority | `xtask-number-drift` |
| backslash escapes | `rust/tcl-lexer/src/substitution.rs`; `rust/tcl-syntax/src/backslash.rs`; `rust/tcl-dialect/src/grammar.rs` | `backslash_subst`; `backslash_subst_in`; `decode_bytes_in`; `backslash_fragment_in`; `BackslashFragment`; `EscapedInputUnit`; `native_source_escape_in`; `native_source_string_bytes_in`; `native_source_braced_word_map`; `native_source_braced_word_extent`; `NativeBracedWordSourceMap`; `NativeSourceUnavailable`; `EscapeSyntax` | selected lexical grammar and independently selected native original-unit decoder/encoder; native source token extent and bounded value decoding, separately from Unicode presentation and subst processing | none |
| boolean words | `rust/tcl-syntax/src/boolean.rs` | `parse_boolean_word`; `truthiness_with` | fixed boolean vocabulary; number axis per release | none |
| quotes / braces / word spans | `rust/tcl-lexer/src/ranges.rs`; `rust/tcl-lexer/src/expr_lexer.rs` | `close_quote_offset`; `word_closer_offset`; `word_span_at`; `braced_var_name_end` | configured nested-expression consumers retain complete LexerConfig through shared quote/command closers; `${...}` closing rules and tmsh brace mode remain release/dialect-selected | none |
| array-index source scan | `rust/tcl-lexer/src/ranges.rs`; `rust/tcl-dialect/src/grammar.rs` | `scan_array_index`; `ArrayIndexSyntax` | `LexerGrammar::array_index` per release | none |
| word substitution components | `rust/tcl-lexer/src/word_parts.rs` | `decompose`; `decompose_spanned`; `scan_var_ref`; `scan_expression_sugar`; `command_subst_close`; `quoted_word_close`; `SubstFlags`; `WordPart`; `SpannedPart`; `WordBody`; `VarRef`; `RawVarRef`; `MISSING_QUOTE`; `MISSING_CLOSE_BRACKET`; `MISSING_CLOSE_BRACE`; `MISSING_PAREN`; `EXTRA_AFTER_CLOSE_BRACE` | `LexerConfig` per emulated release (`${...}` close rule, array-index source mask, escape grammar); compiled-word vs source-word `$` spelling | none |
| runtime compatibility substitution callbacks | `runtime/rust/src/subst.rs` | `resolve_with`; `resolve_with_expression` | caller-supplied variable, command and independent expression callbacks; no retained invocation evidence; unresolved callbacks contribute no value; actual Runtime evaluation owns completion/refusal propagation | none |
| indices | `rust/tcl-cmd-core/src/index.rs` | `read_with`, `read_under`; `drill` | grammar- or release-parameterised, inheriting the number axis and reading each integer in the release's range | none |
| binary field grammar | `rust/tcl-cmd-core/src/binary.rs` | `specifiers`; `Specifier`; `is_specifier`; `signedness_available`; `specifier_min_version`; `FormatValueOps`; `format_values`; `integer_value`; `ScanValue`; `scan_values` | typed scalar/list numeric operands retain internal representations; field letters per release (`t n m r R q Q` are 8.5+); the TIP 275 unsigned suffix (`u` only, after any field letter) per resolved release | none |
| native binary argument diagnostics | `rust/tcl-registry/src/native_binary_usage.rs`; `rust/tcl-registry/src/invocation_words.rs`; `rust/tcl-cmd-core/src/ensemble.rs` | `NativeBinaryRootDispatch`; `binary_root_dispatch`; `binary_root_index_error_code`; `NativeBinaryArgumentUsage`; `InvocationDialect::binary_argument_usage`; `NativeArgumentUsageHeader`; `argument_usage_header_style`; `ArgumentUsageRewrite`; `rewrite_argument_usage` | actual registered handler supplies the operation and invocation layout; selected C/Jim policy supplies the usage tail and raw-word versus list-word header encoding; runtimes preserve bytes through their actual list encoder; public, private, alias and configured ensemble prefixes retain their dispatch rewrite chain; native Binary usage comparisons | none |
| binary objects and decoder inputs | `rust/tcl-registry/src/native_binary_value.rs`; `rust/tcl-cmd-core/src/binary.rs`; `rust/tcl-vm/src/value.rs`; `runtime/rust/src/bytearray.rs` | `NativeBinaryByteConversion`; `NativeBinaryDecodeSource`; `NativeBinaryDecodeInput`; `DecodeArguments`; `decode_argument_layout`; `hex_decode_with_strict`; `base64_decode`; `uu_decode_with_strict`; `UuDecodeError` | selected native character model controls legacy narrowing; Tcl9 encode/scan proper-byte conversion is separate from format narrowing; decoder diagnostics retain actual pure-byte payload versus Unicode source and original byte offsets; exact option selection precedes data conversion; strict uuencode short data is distinct from an invalid character; native decoder/value comparisons; VM values retain raw Jim bytes; unported Unicode-only consumers keep an explicit reached host refusal | none |
| native string length representations | `rust/tcl-registry/src/native_string_length.rs`; `rust/tcl-vm/src/value.rs`; `runtime/rust/src/value_ops.rs` | `NativeStringLengthRepresentation`; `InvocationDialect::string_length_representation` | actual length access converts C8.4/8.5 to string; C8.6 retains any byte-array payload; C9 retains only pure proper byte arrays; modern short existing strings bypass conversion; Jim retains its native cached character count; numeric length alone cannot prove a representation change | none |
| scripted Binary root dispatch | `rust/tcl-registry/src/native_binary_value.rs`; `rust/tcl-vm/src/cmd_binary.rs`; `rust/tcl-vm/src/compiled.rs`; `runtime/rust/src/interp/stock_ensembles.rs` | `NativeBinaryScriptedIngress`; `InvocationDialect::binary_scripted_ingress` | pinned Jim0.84 root is a real procedure tailcalling the exact canonical compound command; rename of the root does not rewrite the compound target; compound replacement remains live; bootstrap source requires real lazy compilation before executing; native Jim compound replacement/frame comparisons; internal implementation: `CompilerProvenance::UncompiledSource` | none |
| decoded native literals | `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-bytecode/src/lib.rs`; `rust/tcl-compiler/src/codegen/values.rs`; `rust/tcl-vm/src/exec.rs` | `EffectiveInvocationWord::ByteLiteral`; `literal_bytes`; `NativeStringLiteral::bytes`; `unicode`; `LiteralTable::intern_bytes`; `push_lit_bytes_exact` | authored `WordExpr`/`WordPart` source spelling and spans remain unchanged; the canonical selected escape decoder produces exact bytes; invalid Unicode remains one Dynamic registry operand without text/name/object authority; literal pool equality and dedup compare bytes; decoded PUSH is verbatim; unsupported byte-native consumers retain host execution refusal outside guest catch; source byte scripts/names require their own represented protocol | none |
| invocation signature count | `rust/tcl-registry/src/arity.rs`; `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-registry/src/spec.rs`; `rust/tcl-registry/src/hover.rs` | `ArityCount`; `with_positionals`; `count_invocation_arguments`; `argument_count_for_arity`; `arity_accepts_frozen_arguments`; `leading_option_word_count_for_arguments`; `value_word_count_for_arguments` | explicit raw-argv versus positional count; frozen cardinality; availability-filtered exact/alias/minimum-prefix grammar; known terminators retain dynamic trailing values; unknown count cannot prove rejection or successful transfer; native compiler source-word arity remains independent; internal implementation: `frozen_argument_count` | none |
| option words / subcommands | `rust/tcl-cmd-core/src/prefix.rs`; `rust/tcl-cmd-core/src/ensemble.rs`; `rust/tcl-registry/src/hover.rs`; `rust/tcl-registry/src/spec.rs` | `OptionTable`; `OptionSpec`; `SubCommand`; `first_positional_index`; `ensemble::EnsembleToken`; `ensemble::InvocationLayout`; `ensemble::invocation_layout`; `ensemble::UNKNOWN_DELETED_MESSAGE`; `ensemble::UNKNOWN_DELETED_ERROR_CODE`; `ensemble::CREATE_OPTIONS`; `ensemble::CONFIG_OPTIONS`; `ensemble::SUBCOMMANDS`; `ensemble::resolve_subcommand`; `ensemble::subcommand_choices`; `ensemble::unknown_subcommand_message`; `ensemble::validate_map_targets` | option surface per release/dialect; ensemble token lifecycle and invocation layout invariant | `xtask-option-registry-drift` |
| trace argument decoding | `rust/tcl-cmd-core/src/trace.rs` | `TraceKind`; `resolve_option`; `resolve_type`; `parse_ops`; `parse_legacy_variable_ops`; `legacy_ops_letters`; `callback_op_word` | option surface per release (the 8.x-only `variable`/`vdelete`/`vinfo` forms) | none |
| sort numeric parsing | `rust/tcl-cmd-core/src/sort.rs` | `parse_wide`; `parse_real` | `NumberSyntax` per release | none |
| generated host refusal transport | `rust/tcl-runtime-api/src/codegen_abi.rs`; `runtime/rust/src/codegen_abi.rs`; `rust/tcl-compiler/src/codegen/wasm/native_emit.rs`; `rust/tcl-compiler/src/codegen/wasm/backend.rs` | `CodegenAbiImportId::requires_host_refusal_check`; `CodegenAbiImportId::HostRefusalPending`; `NATIVE_PROC_STATUS_RAN`; `NATIVE_PROC_STATUS_DECLINED`; `NATIVE_PROC_STATUS_HOST_REFUSED`; `tcl_codegen_host_refusal_pending` | reached operation refusal is a retained host channel; query before out-slot adoption or guest completion handling; unwind ownership without guest capture/finally or replay; status2 leaves completion out untouched; transport-only exemptions are explicitly authored; other operations require a refusal check | none |
| native byte and checked Unicode access | `rust/tcl-syntax/src/value.rs`; `rust/tcl-syntax/src/raw_string.rs`; `rust/tcl-cmd-core/src/error.rs` | `ValueOps::as_bytes`; `ValueOps::new_bytes`; `ValueOps::new_jim_string`; `ValueOps::try_as_str`; `ValueOps::try_char_len`; `ValueOps::string_character_model`; `RawString`; `UnicodeAccessError`; `NativeStringAccessError`; `NativeMaterializationLimitError`; `NativeValueAccessRefusal`; `ValueError::native_access_refusal`; `CmdError::native_access_refusal`; `CmdError::message_bytes`; `CmdError::error_code_bytes`; `CmdError::into_byte_details` | actual native bytes and independently selected character units; Unicode projection, native seek/copy failures and eager backend materialization quotas retain the host-only channel; actual Jim string results preserve their prescribed cached count independently of raw byte length; adapters inspect refusal before constructing any guest completion or options; byte-capable operations preserve original bytes | none |
| Jim string comparison and simple case mapping | `rust/tcl-syntax/src/raw_string.rs`; `rust/tcl-syntax/src/jim084_case_mapping.rs`; `rust/tcl-cmd-core/src/string.rs`; `rust/tcl-lexer/src/substitution.rs` | `RawString::jim084_compare`; `RawString::jim084_case_mapped`; `JimCaseMapping`; `encode_jim084_unicode` | selected Jim084 fast case-sensitive equality compares exact bytes; ordered comparison and limited/nocase equality compare decoded numeric units using actual cached counts; simple case mapping uses the pinned BMP tables, preserves supplementary units, stops at native NUL, and re-encodes malformed or surrogate units; no host Unicode case folding or blanket replacement of every equality operation | none |
| Jim native substring search and repeat preparation | `rust/tcl-syntax/src/raw_string.rs`; `rust/tcl-cmd-core/src/string.rs` | `RawString::jim084_first`; `RawString::jim084_last`; `prepare_repeat_count`; `repeat_with_count` | first searches decoded-unit starts using native leading-byte seeks; last scans exact bytes within the exclusive native prefix and can match continuation bytes; retained storage bounds preserve reached host refusal; hosts normalize repeat counts once before charging exact byte allocation, including Jim safe integer expressions | none |
| Jim byte and numeric-unit glob matching | `rust/tcl-syntax/src/raw_string.rs`; `rust/tcl-cmd-core/src/string.rs` | `RawString::jim084_matches`; `string::string_match` | exact pinned Jim084 decoding, simple uppercase and native bracket grammar; explicit depth-first star continuations retain native empty-subject and byte-extent rules without host recursion; the owned terminating NUL is available, but reached decoding beyond it retains host refusal; no compiler, handler selection or expression equality permission | none |
| Jim trim byte cuts and physical result | `rust/tcl-syntax/src/raw_string.rs`; `rust/tcl-syntax/src/value.rs`; `rust/tcl-cmd-core/src/string.rs`; concrete VM/Runtime value adapters | `RawString::jim084_trim_plan`; `JimStringTrimPlan`; `ValueOps::jim_string_trim_result`; `string::trim` | exact Jim084 default set and numeric-unit membership; native backward start-byte scanning; checked original getters preserve subject-before-characters for left/full trim and characters-before-subject for right trim; the concrete owner converts the SAME fresh left-cut header before full trim's right cut, retains cached counts on unshared suffix reuse, and creates fresh NULL-primary objects for shared cuts and all-trim results; diagnostic argv shares the actual argument owner and copies values only at error capture; no compiler, purity, source or semantic type grant | none |
| command errors | `rust/tcl-cmd-core/src/error.rs` | `CmdError`; `wrong_args`; `bad_choice`; `with_error_details` | invariant | none |
| channel output configuration / encoding | `rust/tcl-platform/src/lib.rs`; `rust/tcl-cmd-core/src/channel.rs`; `rust/tcl-registry/src/commands/tcl/fconfigure_.rs` | `SystemEncoding`; `Host::system_encoding`; `ChannelConfig`; `StandardChannelConfigs`; `OpenAccess`; `resolve_open_access_mode`; `ChannelDirection`; `ChannelEncoding`; `EncodingProfile`; `OutputTranslation`; `resolve_fconfigure_option`; `config_list`; `config_value`; `set_config_value`; `encode_output`; `encode_output_bytes`; `EncodedOutput`; `EILSEQ_ERROR_CODE` | system encoding per host locale and interpreter tree; option availability per dialect profile; open-access validation and profile/binary defaults per Tcl release; mutable direction-specific state per channel | none |
| original byte encoding names and source order | `rust/tcl-platform/src/lib.rs`; `rust/tcl-host-native/src/lib.rs`; `runtime/rust/src/mem_fs.rs`; `rust/tcl-registry/src/native_encoding.rs`; `rust/tcl-cmd-core/src/encoding/original_names.rs` | `EncodingCatalogue`; `EncodingCatalogueUnavailable`; `Host::encoding_catalogue`; `SourceFile`; `Filesystem::open_source_bytes`; `InvocationDialect::native_encoding_name_protocol`; `InvocationDialect::native_source_file_protocol`; `NativeEncodingNameProtocol`; `NativeEncodingNamePurpose`; `NativeSourceFileProtocol`; `OriginalSourceFileOps`; `PreparedSourceFile`; `resolve_original_encoding_name`; `prepare_source_file` | original counted String getters precede source backend access; catalogue lookup follows successful access and uses literal-zero C extent without Unicode repair; supported codec kernels share the channel owner, actual catalogue misses remain separate from unavailable knowledge, and an empty system name restores the separately captured Host default; Runtime OriginalArguments and VM Name scopes retain original receiver/header/currency on both stage outcomes through evaluation and mandatory cleanup; Jim retains filename-only source and has no encoding command; exact provider defaults and backend handles do not establish additional codec, callback or reference-count authority | `xtask-owner-resolution` |
| native original concat | `rust/tcl-dialect/src/version.rs`; `rust/tcl-syntax/src/value.rs`; `rust/tcl-cmd-core/src/list.rs`; `rust/tcl-cmd-core/src/list/native_concat.rs`; `rust/tcl-vm/src/value_ops/native_concat.rs`; `runtime/rust/src/value_ops/native_concat.rs` | `ConcatPolicy`; `ValueOps::concat_policy`; `NativeConcatListShape`; `NativeConcatFirstElement`; `ValueOps::native_concat_list_shape`; `ValueOps::native_concat_empty_list`; `ValueOps::native_concat_copy_list`; `ValueOps::native_concat_append_list`; `ValueOps::native_concat_first_bytes`; `ValueOps::release_native_concat_first`; `ValueOps::native_concat_string_bytes`; `ValueOps::native_concat_string_result`; `ValueOps::discard_native_concat_result`; `list::concat_selected` | actual release selects pure/canonical/abstract List eligibility, original header copy and borrowed member append independently of string bytes; C8.6+ later hash-head checks preserve original getter order and retire partial results; Runtime arithmetic-series Index children are fresh rc0 temporaries bounced after append; VM has no corresponding actual abstract primary; C8.5+ slow results own unknown-count String caches and allocated bytes, including empty results; successful string concat supplies no canonical List guarantee | none |
| Original script-object evaluation | `rust/tcl-registry/src/native_eval_object.rs`; `rust/tcl-vm/src/cmd_control.rs`; `runtime/rust/src/interp.rs`; `runtime/rust/src/list.rs` | `InvocationDialect::native_eval_object_protocol`; `InvocationDialect::eval_object_protocol`; `EvalObjectPurpose`; `LogicalEvalObjectProvider`; `NativeEvalObjectProtocol::dispatches_list` | actual engine and evaluation purpose select original List dispatch independently of script grammar and physical string materialisation; retained backing protects original element identity through callbacks without extra child references; C8.4 direct eval and concatenated namespace bodies require absent resident bytes; C8.4 single namespace bodies and uplevel use source evaluation; Jim requires absent resident bytes, while later C accepts its retained canonical List flag; source paths retain counted bytes, source location and the selected physical frame; internal implementation: `NativeListBacking`, `eval_original_body_framed` | none |
| Original-object return options / completion-code conversion | `rust/tcl-cmd-core/src/return_options.rs`; `rust/tcl-registry/src/native_return_options.rs` | `InvocationDialect::return_options_protocol`; `ReturnOptionsOps`; `prepare_return`; `prepare_option_pairs`; `prepare_prefix_error_options`; `parse_completion_code` | actual engine selects merge/sequential conversion, original list storage, primitive Int/Long probes and completion-code cache; internal dictionary ingestion is a distinct purpose; an explicit logical F5 provider grants simulation only | none |
| original native instruction-name error contexts | `rust/tcl-syntax/src/native_instruction_name.rs`; `rust/tcl-vm/src/value.rs`; `rust/tcl-vm/src/interp/native_error_stack.rs`; `runtime/rust/src/obj/native_instruction_name.rs`; `runtime/rust/src/interp/native_error_headers.rs` | `NativeInstructionName`; `NativeReturnInstructionName`; `NativeInstructionName::for_return`; `opcode`; `string_bytes` | actual C8.6/C9.0/C9.1 instname primary stores native opcode with absent bytes, NULL duplicate/free hooks and selected updater; interpreter innerContext is a real List retaining SAME original stack operands, independently of errorStack and snapshot roles; pure descriptor metadata grants no reached instruction or object authority | `xtask-owner-resolution` |
| Tcl completion options / structured error stacks | `rust/tcl-runtime-api/src/completion_options.rs`; `rust/tcl-runtime-api/src/error_stack.rs` | `completion_options::plan`; `completion_options::plan_with_origin`; `completion_options::retained_array_read_options`; `completion_options::ControlOptionPolicy`; `completion_options::ErrorOptions`; `completion_options::OptionValue`; `error_stack::ErrorStack`; `error_stack::validate_error_stack`; `error_stack::ErrorStackValueError` | standard option overlay follows completion code/level; control commands independently select inherited/fresh body options and forwarded/settled success options; TIP 348 `-errorstack` is available from Tcl 8.6; shifted contexts use the concrete runtime's frame count | none |
| native original Jim key producer handoff | `runtime/rust/src/interp/native_command_names.rs`; `runtime/rust/src/interp/native_ensemble_objects.rs`; `runtime/rust/src/interp.rs`; `runtime/rust/src/namespace.rs`; `runtime/rust/src/cmd_namespace.rs` | `OriginalByteCommandRegistration`; `Interp::remember_original_byte_command_registration`; `Interp::publish_original_jim_registration_keys`; `Interp::prune_original_byte_command_registrations`; `original_jim_command_enumeration_keys` | byte registration retains actual input, publication namespace/tail and generation; selected Jim key construction accepts only unchanged root slots and the shared C-API publication projection; stale receipts retire at registration, repin and command-slot cleanup; helper enumeration borrows actual current physical Jim hash keys, then native List/local/alias-prefix owners retain those same objects outside namespace borrows; helper qualifier/tail and target reporting read the reached original object spelling under current original arguments; comparison bytes do not replace an original key or join a same-name replacement; cold observation and selected-dead-key refusal remain independent | none |
| native automatic Jim error capture | `rust/tcl-runtime-api/src/jim_error_stack.rs`; `rust/tcl-runtime-api/src/script_source_location.rs`; `rust/tcl-registry/src/invocation_words.rs`; `rust/tcl-test-support/src/automatic_errors.rs`; `runtime/rust/src/namespace.rs`; `runtime/rust/src/interp/native_jim_lookup.rs`; `runtime/rust/src/interp.rs`; `rust/tcl-vm/src/native_jim_script.rs`; `rust/tcl-vm/src/exec/native_jim_script.rs`; `rust/tcl-vm/src/exec.rs` | `NativeErrorStackProtocol`; `JimEvaluationFrame`; `JimScriptLocation`; `capture_jim_error_frames`; `JimErrorFrameSelection`; `select_jim_error_frame_sources`; `JimErrorStack`; `JimErrorTrace`; `ScriptSourceLocation`; `error_stack_protocol`; `JIM_AUTOMATIC_ERROR_CASES`; `NativeJimScriptState::new_for_evaluation_frame`; `NativeJimScriptState::current_script_location` | actual selected Jim 0.84 protocol, separate procedure/evaluation nesting and selected variable frames; actual command generation retains the last checked cold lookup original Jim hash key under a neutral allocation lifetime lease, without changing native reference counts or reviving retirement; hot lookup does not replace it; the pure shared frame-coordinate selector materializes only selected original keys/argv/locations before unwind, reached retired keys refuse and unselected keys are not read; no FQN or argv-name fallback; explicit traces remain raw; literal source locations are retained at value creation; VM native Script evaluation borrows its actual ordinary backing filename and validated current LINE, including missing-script failure, rather than bytecode metadata; Subst has no ordinary Script layout; filename transport adds no native reference; shared actual Jim corpus in VM and standalone Runtime | none |
| VM scheduled tailcall activation | `rust/tcl-vm/src/frame.rs`; `rust/tcl-vm/src/interp.rs`; `rust/tcl-vm/src/exec.rs`; `rust/tcl-registry/src/native_compilation.rs` | `NativeTailcallStack` | request belongs to the actual issuing frame; catch observes return code 2 while the request remains scheduled; empty tailcall cancels; only successful procedure settlement dispatches the replacement after leave callbacks; retained lookup namespace is independent of the caller variable frame; unresolved native compiler protocol requires provider evidence; internal implementation: `CallFrame::tailcall`, `TailcallReq`, `schedule_tailcall`, `dispatch_tailcall` | none |
| authored rule callable and counted-string simulation | `rust/tcl-registry/src/f5/rule_identity.rs`; `rust/tcl-vm/src/interp/authored_rule_callables.rs` | `RuleIdentity`; `RuleIdentity::partition`; `RuleProcedureTarget::resolve`; `Vm::install_irules_rule_callable_simulation`; `Vm::install_irules_counted_string_simulation` | explicitly installed logical F5 capability and retained rule identity; relative callable paths select the partition root, local targets select the current rule, absolute paths retain their owner; private actual command-table publication is separate from root body execution namespace and the original caller frame; configuration removal retains activated callables, while full reset retires them; user string length counts original encoded or pure bytearray bytes, independently of physical host Unicode length; no F5 CPP, ABI, native StringLength operation, Unicode index or hash-key identity grant | `xtask-owner-resolution` |
| exact measured original iRule loader images | `rust/tcl-registry/src/irules_policy.rs` | `MeasuredIrulesLoaderProfile`; `MeasuredIrulesSourceRefusal`; `MeasuredIrulesLoaderProfile::original_source_refusal` | opt-in BIG-IP 21.1.0.1 build 0.0.26 original source classification; exact complete attested Unicode images retain their own accepted or rejected outcome, changed or unlisted non-ASCII images remain unsupported; literal source NUL refusal is independent of runtime-generated counted values and names; no general Unicode grammar, normalization, native parser, compiler registration or key identity authority | `xtask-owner-resolution` |
| original child completion handoff | `rust/tcl-vm/src/exec.rs` | `Vm::try_invoke_command` | actual parent activation, retained Subst/Each/Expression/JimScript/Control driver and original Completion result/options; synchronous tailcall and ordinary popped-child results share ordered driver consumption; parent compiler and namespace liveness are validated before getters, reads or callbacks consume the result; no reconstructed commands, options or object headers; internal implementation: `fold_child_completion`, `deliver_tailcall_completion` | `xtask-owner-resolution` |
| independent BIG-IP execution-context evidence | `rust/tcl-dialect/src/model/bigip_execution_context.rs`; `rust/tcl-registry/src/f5/execution_context.rs`; `rust/tcl-registry/src/f5/evidence.rs` | `BigIpExecutionContext`; `BigIpExecutionContext::build_profile`; `BigIpExecutionContext::environment_name`; `BigIpExecutionContext::core_profile`; `EmbeddedRuntimeEvidence`; `RuntimeFact`; `RuntimeFactKind`; `measured_fact`; `assistance_fact` | exact context, appliance release/build and original observation provenance; ParserCase retains source hex, completion code and result hex; semantic queries require the exact measured build, while nearest-known assistance stays labelled and in the same context; APL contexts remain unmeasured, iCall has no inferred catalogue environment, and missing tmsh/iCall width remains Unknown; host Tcl evidence and reported patchlevels grant no embedded parser, compiler, object or ABI authority; internal implementation: `evidence/resolution_contexts.rs` | `xtask-owner-resolution` |
| trace-aware array enumeration | `rust/tcl-runtime-api/src/lib.rs`; `rust/tcl-cmd-core/src/array.rs`; `rust/tcl-syntax/src/value.rs` | `ArrayTarget`; `ArrayElementRead`; `ArrayReadMiss`; `ArrayReadFailure`; `ArrayInvalidation`; `VarStore::array_target`; `VarStore::array_read_elem_at`; `VarStore::array_keys_checked_at`; `VarStore::array_key_bytes_checked_at`; `VarStore::array_read_elem_bytes_at`; `VarStore::unset_elem_bytes_at`; `ArrayReadFailure::new_bytes`; `VarStore::variable_container_model`; `ValueOps::pin_value`; `ValueOps::unpin_value`; `array::dispatch_at` | Tcl variable-cell identity and array-before-element trace ordering; linked-element recovery/reporting follows the selected release | none |
| live native compilation entry | `rust/tcl-runtime-api/src/native_compilation.rs`; `rust/tcl-compiler/src/compile_service.rs`; `rust/tcl-runtime-api/src/lib.rs` | `NativeCompilationEntry`; `NativeCompilationBinding`; `NativeCompilationNamespace`; `NativeNamespaceVariableTable`; `NativeVariableObserverPresence`; `CompileService::compile_script_with_entry`; `BytecodeCompileService::native_entry_options`; `native_entry_config` | source profile, logical `invocation_policy` and physical engine point retained separately; compiler hooks use the physical projection while handlers/values/frames use the logical contract; incomplete live axes remain unknown; interpreter owner/arena, command implementation generations, namespace tokens, independent actual variable-observer closure, trace state and mutation epoch; absent command slots require a closed actual command table; variable-root absence needs its independently captured exact namespace table, and grants no values, trace closure or CPP authority | none |
| selected native scalar math objects | `rust/tcl-registry/src/mathfunc.rs`; `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-registry/src/commands/tcl/mathfunc_generated.rs`; `rust/tcl-compiler/src/command_binding/source_representation.rs` | `NativeScalarMathProtocol`; `NativeScalarMathInputRequirement`; `NativeMathNumericOperandPolicy`; `normal_scalar_math_protocol`; `fixed_scalar_math_protocol`; `refine_scalar_math_input_effects` | exact reached implementation or retained fixed-table registration identity/arity; Sqrt supports C/Jim and Double supports C with exactly one operand; normal Double and OK/Error after operands remain independent of custom string-updater effects; actual C numeric subtype preservation versus Jim coerced-integer weakening; operand world closure requires current numeric or original closed stock-object input; no number, result freshness, compiler route, erasure or observer licence | none |
| original completed source result | `rust/tcl-compiler/src/command_binding/normal_result.rs` | `SourceInvocationBinding::original_normal_result`; `SourceNormalResult::text`; `SourceNormalResult::representation`; `SourceInvocationBinding::original_invocation_completes_normally`; `SourceCommandBindings::conditional_procedure_normal_result` | exact original allocation, unchanged words, pre-handler frame/target/frozen argv and post-handler observer settlement; all Normal alternatives must agree on bytes, while representation joins independently; unknown Normal values withdraw bytes and abrupt alternatives withdraw totality; declaration results meet only original body observations with unknown formals; called declarations use an isolated collector and merge only conditional results, never actual dispatch inventories or caller constants; selected default Return remains pending until the actual procedure boundary; no physical object/cache or compiler permission | none |
| native result dependencies / representation evidence | `rust/tcl-registry/src/native_result.rs`; `rust/tcl-syntax/src/value.rs`; `rust/tcl-registry/src/resolved_invocation.rs` | `NativeResultContract`; `NativeResultSelection`; `NativeNumericStoreProduction`; `NativeNumericResultProduction`; `normal_numeric_store_production`; `InvocationFacts::normal_numeric_result_production`; `ListResultConstruction`; `ordinary_list_range_representation`; `ordinary_range_literal_result`; `VariableResultPhase`; `ValueRepresentation` | selected native implementation and argv grammar; range shape additionally requires a current frozen original ordinary input or an exact successful physical read followed only by original literal argv words, actual length and selected nonempty index range before operand coercion; compiled list construction additionally requires actual compiler selection, compilation context and original operand shapes, preserving pooled literal uncertainty; capture selected cell before observers, consult its post-observer contents without retriggering reads; string contents and list internal representation are independent; Increment numeric store shape requires the authored producer contract, exact cardinality and selected arithmetic, publishes before write observers, and supplies no value, object identity or post-observer result proof; native ListLength normal-result integer shape requires its explicit result contract, selected operation/handler and exact accepted argv; unlike list construction, audited C compilers always execute LIST_LENGTH even for literal operands, and the count result supplies no bytes, callback-effect closure, completion guarantee or allocation freshness | none |
| native successful-handler effects | `rust/tcl-registry/src/native_compilation.rs` | `NativeCompilationSpec::successful_handler_effects`; `SuccessfulHandlerEffects`; `SuccessfulHandlerSpec`; `NormalHandlerImplementationLookup`; `VariableOperandBindingPhase` | exact stable original handler across compilation and both argv boundaries; frozen native argv and actual physical variable frame; normal effects and authored variable-address binding phase only; ensemble leaf contracts additionally require the actual release-specific private mapping/worker. Compiler failure/body/operation selection stays separate | none |
| normal variable transfer under an unknown compiler protocol | `rust/tcl-compiler/src/registry_invocation.rs` | `NormalTransferInvocation`; `normal_transfer_invocation`; `variable_binding_phase` | strict execution proof first; otherwise stable handler and authored successful-handler equivalence on frozen argv/physical frame; private facts expose only variable reads/stores/normal transfer, and observer-free setter value location; cannot license opcodes, script bodies, completion or removal of compiler errors | none |
| normal handler representation effects | `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-registry/src/resolved_invocation.rs` | `NormalRepresentationInvocation`; `normal_representation_invocation`; `normal_statement_representation`; `NormalIndexAccess`; `NormalIndexAccessKind`; `index_access`; `list_constructor_words`; `InvocationFacts::argument_type_hint`; `NormalRepresentationInvocation::argument_type_hint` | shares the normal-transfer owner's exact converged handler and frozen effective argv projection; accepted cardinality/selected arity; positional argument hints retain the selected static command/member/form table and its actual available-option boundary in InvocationFacts, including transparent_from; normal consumers read that retained table without re-querying a later registry, and unknown options/cardinality/selectors or rejected arity decline; exposes only representation hints/mutation, successful result types, byte-result/payload metadata and positioned operand words; ordered List constructor operands retain separate original contributor spans without an object or execution guarantee; diagnostic index access retains actual container/index words, frozen values and selected index grammar/append policy without a success or erasure guarantee; typed `IterationBindings` carries producer-minted list/dict input expectations without command dispatch; no general invocation facts or compiler/body/completion licence; exception edges discard normal conversion guarantees; shimmer commitment, use-site, expression, sharing and byte provenance | none |
| normal handler taint source properties | `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-registry/src/resolved_invocation.rs` | `NormalTaintInvocation`; `normal_taint_invocation`; `source_colour` | shares the accepted normal handler proof, including actual worker/provider prerequisites and frozen argc; source attributes come from the retained selected descriptor, with getter safety bits restricted to its zero-argument result; replacements, unknown targets and unloaded catalogue candidates cannot inherit those bits; exposes no sanitizer, effects, completion or opcode licence; command-substitution taint results | none |
| conditional ordinary list-length completion | `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-registry/src/representation.rs`; `rust/tcl-compiler/src/command_binding/container_coercion.rs` | `InvocationFacts::ordinary_list_length_completion` | selected ListLength operation and normal handler, accepted frozen argv, actual native family/release, current original final operand with unanimous ordinary List/Dict evidence; bounds codes to OK/Error, preserving error and abstract/unknown residuals; representation metadata alone grants no completion, result, store or opcode | none |
| native list-object and conversion effects | `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-registry/src/list_object_methods.rs`; `rust/tcl-registry/src/representation.rs`; `rust/tcl-registry/src/native_result.rs`; `rust/tcl-compiler/src/command_binding/object_callbacks.rs`; `rust/tcl-compiler/src/command_binding/source_representation.rs`; `rust/tcl-compiler/src/registry_invocation.rs` | `InvocationFacts::list_length_object_protocol`; `InvocationFacts::native_list_method_requirements`; `NativeListObjectProtocol`; `NativeListMethodRequirement`; `ListLengthObjectProtocol`; `InvocationFacts::normal_list_method_provider`; `NativeResultContract::normal_list_method_provider`; `NativeListMethodProvider` | selected ListLength operation, normal handler and accepted frozen operand layout; all-engine stock conversion obligations versus actual C9 native length-method dispatch; independently current ordinary input or audited arithmetic-sequence result provider is required before preserving the mutable world; provider result can be ordinary empty or stock arithseries; explicit Duplicate/Length/Index/Slice/Elements capabilities close root interpreter-world effects independently of receiver/cache mutation; StringAccess requires separate stock or numeric object-class proof; actual compiler selection separates runtime foreach varlists from compile-time preparation, and grouped/nested indices retain residuals; no producer effects, bytes, representation, class, freshness or completion | none |
| original literal-pool object effects | `rust/tcl-runtime-api/src/native_literal_observation.rs`; `rust/tcl-runtime-api/src/native_literal.rs`; `rust/tcl-compiler/src/command_binding/literal_object_pool.rs` | `NativeEmptyLiteralWorld`; `SourceStockLiteralObject` | authored fresh Tcl pool or an independently observed empty actual shared registration world, bound to the original interpreter, world, capture epoch and weak lifetime; every subsequent capture, registration attempt, acquisition, release or retirement invalidates the native observation before getters/owner changes; missing, populated, foreign or expired worlds refuse; source-root identity intersects on joins and unknown effects withdraw it without constructor recovery; the native observation permits only effect-only original literal capture for exact fresh publications, and supplies no stock class, numeric/container contents, child ownership, result, normal completion or compiler admission; stock literal and numeric consumers retain their independent authored requirements; VM and Runtime capture only their actual concrete shared world; source importer validates the original entry interpreter/epoch | none |
| shared-object representation coercion | `rust/tcl-registry/src/representation.rs` | `RepresentationEffect::coercion_selection`; `RepresentationCoercionSelection`; `successful_ordinary_container_coercions`; `OrdinaryContainerCoercion` | selected native implementation, actual family/release, exact argv layout and independent ordinary-object representation proof; list-length, dictionary-size and foreach value slots expose normal-success conversion policies; C 9 abstract-list objects remain excluded; empty/unknown bytes can retain a May preservation branch; shared aliases join representations rather than gain a Must result shape; expression-source parsing and reached lazy operands can shimmer every alias without changing bytes or cells | none |
| typed binary packing / unpacking | `rust/tcl-cmd-core/src/binary.rs`; `rust/tcl-syntax/src/value.rs` | `FormatValueOps`; `format_values`; `integer_value`; `ScanValue`; `scan_values` | field/count grammar is shared; byte, integer, double and list coercions use the selected runtime; numeric fields retain native values without materialising a precision-dependent string | none |
| native fixed-function expression preparation | `rust/tcl-syntax/src/expr/jim_function_tree.rs`; `rust/tcl-registry/src/runtime_expr_validation.rs` | `NativeFunctionTree`; `prepare_jim_function_tree`; `requires_fixed_function_preparation`; `preparation_needs_actual_table`; `prepare_fixed_function_expression`; `PreparedExpressionWitness`; `ExpressionPreparationProof`; `prepare_expression_witness`; `NativeExpressionResultRecipe`; `numeric_result_recipe`; `native_expression_constant_pooling` | exact expression bytes and full parser axes; actual selected native parser and installed function identity/arity; entry validation includes runtime-lazy branches, prepared native tree and interpreter/table prerequisite separately from reached calls; warm caches revalidate actual table evidence | none |
| expression grammar / evaluation | `rust/tcl-syntax/src/expr/parser.rs`; `rust/tcl-syntax/src/expr/eval.rs`; `rust/tcl-registry/src/expr_surface.rs`; `rust/tcl-syntax/src/expr/checked.rs` | `parse_expr`; `ExprParseContext`; `CheckedExprParse`; `native_diagnostic_with_context`; `eval`; `RuntimeExprSurface` | complete lexer/operator/host/native parser axes; proved syntax failure distinct from unsupported recovery, limits or unknown syntax; `RuntimeExprSurface` per release | none |
| original branch expression source grammar | `rust/tcl-compiler/src/optimiser/branch_folding.rs`; `rust/tcl-compiler/src/optimiser/mod.rs`; `rust/tcl-syntax/src/expr/parser.rs` | `branch_original_expression`; `PassContext::lexer_config`; `branch_execution_equivalent`; `PassContext::expression_rewrite_equivalence_at` | original source lexer grammar selects AST variable boundaries independently of numeral/operator/runtime policy; shared contextual equivalence retains separate operand/preparation and execution requirements; standalone missing-profile runtime numerals are explicit; source topology supplies no Native object, compiler instruction, activation or editing licence | none |
| original expression terms and Jim preparation | `rust/tcl-lexer/src/expr_lexer.rs`; `rust/tcl-syntax/src/expr/native_objects.rs`; `rust/tcl-syntax/src/expr/jim_function_tree.rs`; `rust/tcl-registry/src/runtime_expr_validation.rs`; `rust/tcl-syntax/src/expr/eval.rs`; `rust/tcl-syntax/src/scalar_getter.rs`; `rust/tcl-syntax/src/expr/checked.rs` | `expression_term`; `expression_terms`; `ExprTerm`; `ExprTermKind`; `JimExpressionCacheAction`; `JimExpressionObjects`; `JimExpressionTermValue`; `jim_integer_expression_message`; `JimScriptPreparation`; `jim_expression_number_for_kind`; `prepare_fixed_function_expression_bytes_with_preparation`; `prepare_expr_bytes_checked_with_context`; `ExprOps::command_bytes_at`; `ExprOps::literal_bytes_at`; `ExprOps::string_bytes_at` | exact original token/body extents and raw native line deltas; asynchronous requests retain inclusive AST positions; Jim cache actions distinguish tokenization, completeness and tree outcomes independently of displayed errors; numeric term construction retains the original scanner-selected integer/double kind and is separate from existing-object getters; fixed-function preparation retains the same independent primary action; pure receipts do not issue interpreter filename identity; the concrete prepared holder owns all original terms by span, including lazy branches; safe Jim integer expressions consume original token bodies and preserve reached primary/cache effects | none |
| full Jim Script token ownership | `rust/tcl-lexer/src/lexer.rs`; `rust/tcl-lexer/src/jim_script_tokens.rs`; `rust/tcl-syntax/src/jim_script_layout.rs` | `jim_script_tokens`; `jim_script_syntax_tokens`; `JimScriptTokens`; `JimScriptTokenKind`; `JimScriptLine`; `prepare_jim_script_layout`; `JimScriptLayout`; `JimScriptLayoutEntry` | original native source image, complete consumed spans and undecoded token bodies; separators and malformed tails precede command-cut filtering; raw native line deltas, parser completeness line and missing marker remain separate; the pure WORD/LINE projection retains original roster indices and native expansion counters rather than conventional word counts; genuine expansion prefixes are omitted from prepared backing; native escape decoding and original filename/interpreter object ownership belong to the concrete interpreter; translated document input cannot issue native token ingress; the readonly syntax sibling preserves its original channel and missing marker without object or execution authority | none |
| concrete Jim Script object ownership | `rust/tcl-syntax/src/jim_script_objects.rs`; `rust/tcl-vm/src/value_script.rs`; `runtime/rust/src/native_script.rs`; `runtime/rust/src/interp/native_script.rs` | `JimScriptObjects`; `JimScriptObjectConstruction`; `JimScriptCommand` | full selected native roster and layout; original filename object, signed source baseline, actual interpreter empty/null-script identities and weak context origin; LINE has resident empty bytes and a ScriptLine primary, WORD has a pure integer primary; Script owns each real original token once; active execution retains the same backing across parent shimmer and restores it on guest completion; Jim duplicate retires Script to an untyped resident string; indexed-variable and specialized interpolation cache access remain checked capability boundaries; original native Script storage fixtures; internal implementation: `prepare_native_jim_script`, `native_script::prepare`; original Script execution enters the actual receiving interpreter operation before token preparation and retains that receiver and allocator currency through reached preparation and mandatory execution/backing cleanup; currency is checked before preparation use and return; cross-interpreter invocation keeps its existing counted-String crossing and rejects foreign allocation, without an extra getter, namespace coordinate or handler admission | none |
| Jim interpreter retirement | `rust/tcl-runtime-api/src/jim_interpreter.rs`; `rust/tcl-vm/src/interp/jim_teardown.rs`; `runtime/rust/src/interp/jim_teardown.rs` | `JimInterpreterTeardown`; `teardown_jim_interpreter`; `JimInterpreterObjectRole`; `release_jim_call_frame_objects` | selected actual Jim ownership runs stored frame defers while command storage is live; normal and retiring frames release local commands, original procedure arguments, original procedure body, original namespace object and variables in that order; namespace indexes hold weak references after transferring their original object to a declaration or frame; interpreter retirement releases frames and commands before the interpreter's individual original object references, then invalidates the native procedure epoch and releases package storage; weak context associations and numeric access decline after retirement; storage is taken under short guards and released outside them; no association-data or trace-command object is installed in these backends; ordering and lifetime controls | none |
| selected normal operand cache costs | `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/shimmer/hints.rs`; `rust/tcl-compiler/src/shimmer/commit.rs`; `rust/tcl-compiler/src/shimmer/use_site.rs` | `stock_length_operand_cache` | selected actual Length protocol plus original captured native class; C86 numeric conversion versus C9 cache preservation reused by warning, replay and loop cost accounting; missing/semantic-only class or wrong operand cannot close cost; no value, completion or erasure grant; paired selected-Length cost/replay tests cover cache preservation, conversion and missing-class refusal; internal implementation: `operand_preserves_captured_cache` | none |
| reached native math-function folding | `rust/tcl-compiler/src/math_function_binding.rs`; `rust/tcl-compiler/src/tcl_expr_eval.rs`; `rust/tcl-syntax/src/expr/eval.rs`; `rust/tcl-compiler/src/sccp.rs`; `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/command_binding/math_occurrence.rs`; `rust/tcl-compiler/src/command_binding/origin_inventory.rs` | `ExpressionMathBindings`; `resolved_call_for_value_analysis`; `resolved_call`; `proves_intrinsic_for_erasure`; `proved_invocation`; `SccpResult`; `eval_tcl_expr_with_resolved_math_bindings`; `eval_tcl_expr_with_math_bindings`; `ExprOps::call_at`; `SourceImplicitMathInvocation`; `SourceMathInvocation`; `SourceMathInvocation::reached`; `SourceMathInvocation::conditional`; `SourceConditionalMathInvocation`; `math_invocations_for_script`; `implicit_math_invocations_for_script` | original occurrence carrier retains Reached or Conditional purpose; explicit reached() selection precedes actual identity, fold or erasure; conditional topology retains exact source/frame/namespace/arity without registration, entry, header or totality and keeps MathBindingPrerequisiteRequired; recorded previews remain Conditional; exact source origin and AST call offset after reached operands; mutable command identity, namespace lookup and observer absence; independently stamped fixed-table implementation and interpreter prerequisite; stamped global contract spelling is projected through registry `global_command_bare_name`, never a tail or arity match; only reached calls with independently closed operand callback effects consume fold dependencies; implementation-only analysis and pure intrinsic mathematical evaluation cannot establish erasure | none |
| reached expression operand coercion | `rust/tcl-compiler/src/tcl_expr_eval.rs` | `RetainedNativeOperandProof`; `NativeOperandProofs`; `eval_tcl_expr_with_proved_operands`; `analyse_tcl_expr_with_resolved_math_bindings`; `FoldEvaluation`; `NativeCoercionObligation`; `NativeExpressionResultDependency` | value constants do not prove native object identity or representation; fresh operation values are separate from retained objects; numeric, truth, numeric comparison, membership and final-result paths preserve required conversions; already native numeric reads require actual interpreter/object incarnation and exact dialect evidence; consumers retain and validate object evidence before replacing execution; skipped lazy operands never require conversion proofs; original-AST analysis values retain reached coercion obligations independently of erasure; Jim selected-object results retain original bytes/sharing obligations; synthetic literal substitution requires actual operand evidence; source/SSA currently carry live cell List/Unknown evidence, not native numeric object identity; execution folding declines required retained-object conversions until the owner supplies that evidence; internal implementation: `FoldValue` | none |
| grouped variable-store rewrite eligibility | `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/optimiser/store_packing.rs` | `StatementResultUse`; `Script::statement_result_use` | normal result discard is separate from abrupt completion, ordered physical outputs, failure text, observers and shared object representations; replacement command candidates require exact retained lookup and native engine point; synthetic boundaries do not consume a result; existing metadata does not close grouped-store schedule/object equivalence, so O119 remains hint-only; C84 foreach and set have different array-store error text; internal implementation: `assess_grouped_store_rewrite`, `StorePackingAssessment`, `StorePackingDecline` | none |
| purpose-aware scalar value facts | `rust/tcl-compiler/src/sccp.rs`; `rust/tcl-compiler/src/compilation_unit.rs`; `rust/tcl-compiler/src/static_loops.rs`; `rust/tcl-compiler/src/sccp/original_value_facts.rs` | `ValueFactInputs`; `execution_value_facts`; `semantic_value_facts`; `SemanticValueFacts`; `SemanticValueProjection`; `FunctionUnit::semantic_values`; `ExpressionEvaluationPoint`; `ExpressionAnalysis`; `SemanticValueFacts::loop_analysis`; `StaticLoopAnalysis` | one shared phi/store/read/reachability solver; immutable analysis contents preserve original producer coercion, result-object, preparation and reached-handler obligations; existing SCCP values remain execution-erasure proof; Jim raw selected-result bytes remain separate from numeric interpretation; bounded loop analysis retains ordered expression and increment obligations; caller contents enter analysis through exact closed ordinary Incoming reads across every retained activation, without manufacturing a Symbol/version; diagnostic consumers use the typed immutable view; semantic constants never enter legacy erasure maps without producer proof; internal implementation: `DiagnosticValueFacts` | none |
| selected native constant command substitution | `rust/tcl-compiler/src/const_subst.rs` | `ConstSubstCtx::fold_cmd_subst_in`; `ResolvedConstSubst` | retained `InvocationDialect` selects grammar, native query and numeric release independently of catalogue profile and row order; recursive substitutions retain the same axes and exact command dependencies; binding trust remains a separate required contract; explicit unknown native identity declines rather than selecting a catalogue fallback; source rewrites additionally retain the original operand/evaluation effects; known bytes do not attest native object representation | none |
| reached native substitution template | `rust/tcl-registry/src/substitution.rs`; `rust/tcl-compiler/src/command_binding/substitution_template.rs`; `rust/tcl-registry/src/body_execution.rs` | `BodyExecutionSpec::SubstitutionTemplate`; `SubstitutionTemplateSelection`; `TemplateParseErrors`; `ResolvedInvocation::native_substitution_template` | actual frozen argv cardinality, retained available option table and prefix grammar; selected native family and exact template origin; ordered scalar reads preserve observer effects and partial C parse errors; result bytes carry no numeric or object representation proof; command substitutions, array-index substitutions and unmodelled Jim parser acceptance retain unknown effects; the original command remains executable | none |
| pattern operand source projection | `rust/tcl-registry/src/registry.rs`; `rust/tcl-compiler/src/regex_source.rs`; `rust/tcl-compiler/src/registry_invocation.rs` | `arg_indices_for_role_words`; `pattern_source_argument_index` | selected registry layout, original structured source words and actual retained `InvocationDialect`; dynamic option positions and expansions decline; fixed option values retain their occupied slots; normal-handler diagnostics map only exact written origins, never captured alias-prefix values into source spans; role selection supplies no opcode, store or rewrite licence; internal implementation: `source_pattern_index` | none |
| reached native numeric representation | `runtime/rust/src/obj.rs`; `rust/tcl-registry/src/mathfunc.rs` | `native_math_protocol` | actual reached conversion updates shared internal representation while preserving existing source bytes and lazy double context; catalogue spelling and a separately constructed numeric result cannot license conversion | none |
| expr math functions and the `rand` generator | `rust/tcl-syntax/src/expr/mathfunc.rs`; `rust/tcl-syntax/src/expr/rand.rs` | `NumValue`; `dispatch`; `dispatch_with_backend_int_width`; `try_dispatch_with_backend_int_width`; `try_dispatch_with_backend_protocol`; `NativeMathProtocol`; `jim_numeric_operand`; `IntWidth`; `MathFuncError`; `MathFuncSince`; `spec`; `all`; `added_in`; `seed_from_wide`; `next_draw`; `seed_and_draw` | `MathFuncSince` per release for the function surface; selected native math protocol separates C floating power/domain errors from Jim integer power/wrap/NaN; integer parsing and conversion remain selected native axes; dispatch policy does not prove handler presence; the Park-Miller generator is release-invariant | none |
| command / word segmentation | `rust/tcl-lexer/src/script.rs`; `rust/tcl-lexer/src/native_script_words.rs`; `rust/tcl-lexer/src/native_word.rs`; `rust/tcl-compiler/src/segmenter.rs`; `rust/tcl-compiler/src/parsing/syntax/build.rs`; `rust/tcl-compiler/src/parsing/syntax/segment.rs` | `group_commands`; `group_commands_bytes`; `CommandSpan`; `CommandSpan::source_extent`; `NativeScriptCommandWords`; `WordSpan`; `WordKind`; `NativeWord::from_group`; `SegmentedCommand`; `SegmentedScript`; `SegmentedParseCut`; `segment_commands`; `segment_script_image_with_offset_and_config` | exact original image, full `LexerConfig` and typed `WordKind`; braced/quoted groups own their outer delimiter, while bare concatenations use the final component's configured closer and retain enclosing body delimiters separately; lexical geometry grants no selected command or execution | `xtask-segmentation-drift` |
| nested command-substitution syntax | `rust/tcl-compiler/src/word_subst.rs` | `nested_command_words`; `NestedWordsDecline`; `lifted_calls`; `LiftedCall` | original lexical extent and exactly one complete command through the shared configured segmenter; syntax and word grouping only, with no entered handler, effect closure, parent completion or erasure authority | none |
| parse-error cut | `rust/tcl-lexer/src/parse_cut.rs` | `first_parse_cut`; `first_parse_cut_in`; `first_parse_cut_image_with_descent_checked`; `ParseCutDescent`; `ParseCut`; `EXTRA_AFTER_CLOSE_QUOTE` | `LexerConfig` per emulated release, inherited from the segmentation and word-component owners it walks; explicit enclosing-script versus recursive command-body syntax descent, independent of physical script entry | `xtask-segmentation-drift` |
| script completeness / reparse windows | `rust/tcl-lexer/src/structural_index.rs` | `script_is_complete`; `command_boundaries`; `reparse_window`; `BracketIndex`; `BraceIndex`; `ExprParenIndex`; `ParenBalance` | dialect-blind by construction: one byte scan of stock 8.6/9.x brace, quote, `${…}`-nesting, comment and terminator structure, so an editor keystroke costs no tokenise. The two grammar axes that really do move a command boundary — the F5 `BraceLineContinuation::Continues` next-line-`{` rule and the 8.x `BracedVarStyle::FirstClose` name rule — are pinned as measured divergences by `differential_boundaries`, never silently absorbed | `xtask-segmentation-drift` |
| iRules execution boundaries and placement | `rust/tcl-syntax/src/event_handler.rs`; `rust/tcl-registry/src/events.rs`; `rust/tcl-registry/src/registry.rs`; `rust/tcl-irules/src/when_block.rs`; `rust/tcl-irules/src/executable.rs` | `event_handlers`; `event_handlers_with_head_predicate`; `script_commands`; `top_level_when_handlers_with_registry_and_head_resolver`; `IrulesDeclarationArguments`; `IrulesExecutionContext`; `IrulesCommandPlacement`; `IrulesTopLevelDeclaration`; `IrulesTopLevelEffect`; `CommandRegistry::irules_command_placement`; `CommandRegistry::irules_event_declaration`; `CommandRegistry::irules_top_level_declaration`; `CommandRegistry::irules_top_level_declaration_shape`; `CommandRegistry::irules_top_level_effect`; `when_blocks`; `irules_executable_commands` | caller-supplied `LexerConfig`; offset-keyed resolved command identity; exact single-braced declaration body; declaration-only top level; known-event roots; call-reachable procedure bodies; stateful priority (`0..=1000`, default 500) | `xtask-gen-ai-diagnostics` |
| text similarity | `rust/tcl-compiler/src/text.rs` | `edit_distance`; `rank_suggestions`; `rank_containment_suggestions` | invariant | none |
| per-command knowledge | `rust/tcl-registry/src/spec.rs`; `rust/tcl-registry/src/hooks.rs`; `rust/tcl-registry/src/registry.rs` | `CommandSpec`; `SubCommand`; `CommandRegistry`; `RegistrySnapshot`; `RegistrySemanticKey`; `semantic_key`; `CommandRegistry::snapshot`; `RegistrySnapshot::registry`; `RegistrySnapshot::shared_registry` | per release/dialect; immutable full registry equality with authored spec order, surface layers, full profile key, ambient/own package floors, special-variable rows, overlay, pack origins, reference texts and document grammar; fingerprint buckets still require full equality | `xtask-command-backing` |
| constant evaluation and value transfers | `rust/tcl-registry/src/value_transfer/mod.rs`; `rust/tcl-registry/src/value_transfer/declaration.rs`; `rust/tcl-registry/src/value_transfer/route.rs`; `rust/tcl-registry/src/value_transfer/const_ops.rs`; `rust/tcl-registry/src/value_transfer/cell_write.rs`; `rust/tcl-registry/src/value_transfer/keyed_update.rs`; `rust/tcl-compiler/src/value_transfer.rs` | `CommandSemantics`; `SemanticsDeclaration`; `resolve_semantics`; `ResolvedSemantics`; `EvalRoute`; `NativeEvalId`; `ConstOps`; `CellWriteSemantics`; `KeyedUpdateSemantics`; `AnalysisContextKey` | the resolved invocation's binding, selected form, and target profile, carried in one analysis context; a route declares the release axes it needs | `xtask-value-transfers` |
| command vocabulary outside the registry | `rust/tcl-registry/src/registry.rs`; `rust/tcl-registry/src/clause_grammar.rs`; `rust/tcl-registry/src/definer.rs`; `rust/tcl-registry/src/special_vars.rs`; `rust/tcl-lsp-core/src/semantic_tokens.rs` | `CommandRegistry`; `manufacturer_method_for_descriptor`; `clause_keywords`; `DefinitionBodyGrammar`; `DefinitionBodyGrammar::implicit_receiver_vars`; `DefinitionBodyGrammar::is_implicit_receiver_variable`; `SPECIAL_VARS` | per dialect surface: the vocabulary is every word the registry declares across every loadable dialect and the shipped `.tclspec` packs; selected manufacturer descriptors distinguish the registered factory command from its manufactured-class defaults and provide source layout/default visibility only, without Native lookup, entry or execution; selected definition-body receiver variables are an explicit subset of implicit_vars, not all implicit vocabulary; semantic method colouring retains its actual instance-callable/current-body/sole-word and Native gates, and this metadata grants no receiver identity, variable read or entered frame | `xtask-registry-axes` |
| dialect / release facts | `rust/tcl-dialect/src/profile.rs`; `rust/tcl-dialect/src/grammar.rs`; `rust/tcl-dialect/src/version.rs`; `rust/tcl-dialect/data/reference-toolchains.tsv` | `DialectProfile`; `DialectProfileKey`; `DialectProfile::cache_key`; `DialectProfileKey::profile`; `DialectProfile::intern`; `LexerGrammar`; `TclVersion`; `TclVersion::patchlevel`; `TclVersion::reference_source_tag`; `TclVersion::has_error_stack`; `find` | the resolved dialect/release/build and complete lexer/runtime/availability/library snapshot; profile caches use value keys, bytecode retains its supplied strict handle; exact pinned reference patchlevel/source tag | `xtask-editor-extensions` |
| reference interpreter discovery and execution | `rust/tcl-test-support/src/lib.rs`; `rust/tcl-test-support/src/jim.rs`; `rust/tcl-test-support/jim-reference.txt` | `reference_patchlevel`; `reference_source_tag`; `locate_tclsh`; `available_tclshs`; `witness_tclsh`; `REQUIRE_TCLSH`; `required_tclshs`; `run_script`; `run_script_file`; `run_script_fixture`; `locate_source_tree`; `Tclsh`; `TclSourceTree`; `ScriptOutcome`; `jim_reference`; `locate_jimsh`; `require_jimsh`; `JimReference`; `Jimsh`; `JimCapability` | exact C interpreter/source agreement; deliberately pinned current Jim revision and measured extension capabilities | none |
| executable conformance vectors | `rust/tcl-syntax/src/execution_conformance.rs` | `ExecutionDomain`; `ExecutionVector`; `vectors`; `RewriteCase`; `rewrite_cases`; `FilesystemExecutionCase`; `filesystem_cases` | C release expectations and independently measured Jim results/feature absences | none |
| interpreter platform bootstrap | `rust/tcl-platform/src/lib.rs`; `rust/tcl-cmd-core/src/platform.rs` | `bootstrap::Values`; `bootstrap::Snapshot`; `bootstrap::snapshot`; `bootstrap::entries`; `bootstrap::auto_path`; `platform::auto_path_installation`; `platform::auto_path_environment`; `bootstrap::HOST_ARRAYS`; `bootstrap::HOST_PATH_GLOBALS`; `bootstrap::safe_scrub_keys`; `bootstrap::SHARED_LIBRARY_EXTENSION` | selected-host snapshot, keys, rebootstrap clear, safe scrubbing and canonical Unix shared-library suffix; auto-path byte entries retain existing multiplicity and deduplicate new additions; environment expansion needs actual host HOME and refuses unavailable named-user lookup; runtime identity supplied per engine | none |
| shared plain types | `rust/tcl-core-types/src/lib.rs`; `rust/tcl-core-types/src/diag_code.rs` | `OoId`; `DiagCode` | interpreter-local OO identity and diagnostic codes are invariant across dialects | `xtask-diag-tables` |
| diagnostic suppression directives | `rust/tcl-compiler/src/analyser/utils.rs` | `parse_file_suppression`; `parse_noqa_marker`; `parse_noqa_line_suppressions_for_dialect`; `apply_preceding_noqa`; `line_suppressed`; `FILE_SUPPRESS_KEY` | directive shapes are release-invariant; the noqa pre-scan segments under the document dialect's `LexerConfig` | none |
| SslicTcl declaration model | `rust/tcl-sslictcl/src/model.rs` | `SslicModel`; `TlsFacts`; `Policy` | vocabulary version (`dsl::SUPPORTED_VOCABULARY`); no Tcl release axis — the document is never evaluated | none |
| SslicTcl document loading | `rust/tcl-sslictcl/src/dsl.rs`; `rust/tcl-sslictcl/src/vocabulary.rs` | `load_with_diagnostics`; `DslDiagnostic`; `DECLARATIONS` | vocabulary version; open/closed block rule per declaration | none |
| SslicTcl finding identity | `rust/tcl-sslictcl/src/policy.rs` | `evaluate_policy`; `PolicyFinding` | invariant `(check id, endpoint)` identity; the `grade` id is reserved | none |
| SslicTcl embedded source data | `rust/tcl-sslictcl/src/trust.rs` | `embedded_dataset` | pinned upstream revisions, recorded with hashes and licences in `data/provenance.json` | `xtask-sslictcl-data` |
| SslicTcl declaration surface | `rust/tcl-registry/src/commands/sslictcl/mod.rs`; `rust/tcl-registry/src/definer.rs` | `sslictcl_command_specs`; `SSLICTCL_GRAMMARS` | the `sslictcl` authoring surface (`SpecSurface::SSLICTCL`); Tcl 9.0 core underneath | none |
| SslicTcl editor projection | `rust/tcl-lsp-core/src/sslictcl_diagnostics.rs`; `rust/tcl-lsp-core/src/declaration_outline.rs` | `applies_to`; `diagnostics`; `SUPERSEDED_ANALYSER_CODES`; `is_declaration_document`; `declarations` | resolved authoring surface (the `sslictcl` package) per document | none |
| native frame-level cache | `rust/tcl-registry/src/frame_effect.rs`; `rust/tcl-syntax/src/native_object.rs`; `rust/tcl-vm/src/value.rs`; `runtime/rust/src/obj.rs` | `NativeFrameLevelCache`; `NativeFrameLevelProtocol`; `native_frame_level_protocol`; `NativeObjectCacheSnapshot`; `native_frame_level_cache_in`; `install_native_frame_level_cache` | original live header and actual C release; C8.5 relative/absolute and C8.6+ absolute numeric caches reselect the current caller chain on every use; NULL descriptor hooks, duplication and lifetime transport grant no frame pointer, interpreter identity or generation authority | `xtask-owner-resolution` |
| original compiled list-index operands | `rust/tcl-cmd-core/src/index.rs`; `rust/tcl-syntax/src/native_compiled_index.rs`; `rust/tcl-registry/src/native_list_index_compilation.rs`; `rust/tcl-bytecode/src/lib.rs` | `compiled_list_index_in`; `NativeCompiledListIndex`; `NativeCompiledListIndex::resolve`; `NativeListIndexInstruction`; `NativeListIndexOperation`; `compile_native_list_index` | actual C compiler, original ordered operands, release-selected immediate encoding and independently retained width uncertainty; native end and outside coordinates are separate from portable encodings; the recipe grants no compiler registration, list primary, member ownership or successful getter | `xtask-owner-resolution` |
| original native List storage and ranges | `rust/tcl-cmd-core/src/native_list_storage.rs`; `rust/tcl-vm/src/native_list_backing.rs`; `rust/tcl-vm/src/value/native_list_storage.rs`; `runtime/rust/src/list.rs`; `runtime/rust/src/list/native_list_storage.rs` | `NativeListStringState`; `NativeListStringState::from_empty`; `NativeListRangeStorage`; `NativeListRangeAction`; `range_action`; `range_storage_action`; `NativeListReplaceStorage`; `NativeListReplaceLayout`; `replace_layout`; `NativeListItems` | actual C release, original getter and header sharing sampled before result ownership; visible window, owning member inventory and issued allocation extent remain distinct; C9 full/prefix/span/new-store order and integer division are shared; internal `Value::native_list_range`, Runtime `native_list_range` and `replace_prepared_native_elements_using` consume those recipes; memory snapshots add no native member roles; generation rejects changed vectors; ordinary ranges and actual append/replacement are separate from nested LSET and abstract slice providers, compiler admission and argument chronology | `xtask-owner-resolution` |
| original scalar compiler operands | `rust/tcl-registry/src/native_scalar_compilation.rs`; `rust/tcl-registry/src/native_compilation.rs` | `NativeScalarScope`; `NativeScalarOperation`; `NativeScalarInstruction`; `compile_native_scalar` | original public monolithic or private worker registration, complete source vector and actual C release; equality and string/list length preserve operand evaluation before their selected native getters; written command names cannot issue a private compiler scope | `xtask-owner-resolution` |
| original namespace string compiler operands | `rust/tcl-registry/src/native_namespace_string_compilation.rs`; `rust/tcl-cmd-core/src/string/native_compiled.rs`; `rust/tcl-syntax/src/value.rs` | `NativeNamespaceStringOperation`; `NativeNamespaceStringInstruction`; `compile_native_namespace_string`; `compiled_find`; `compiled_index`; `compiled_range`; `compiled_tail`; `compiled_qualifiers`; `ValueOps::native_unicode_string_result` | complete original compiler words and separately selected C8.6+ Tail/Qualifiers registrations; actual haystack-before-needle Unicode getter, counted index byte result and shared selected UTF encoder, retained Unicode range backing and independently checked backend constructor; direct CString worker and older no-hook controls stay separate; pure geometry grants no compiler token, frame, name lookup or Normal completion | `check-name-resolution-proofs` |
| counted resident name presentation | `rust/tcl-syntax/src/native_string/name_display.rs` | `resident_name_label` | exact complete resident bytes; printable UTF-8 remains readable and invalid/control/backslash bytes retain distinct labels; no source roundtrip, comparison, lookup or edit grant | `xtask-owner-resolution` |
| original external UTF-8 conversion | `rust/tcl-cmd-core/src/encoding.rs`; `rust/tcl-syntax/src/value.rs`; `runtime/rust/src/value_ops.rs`; `rust/tcl-vm/src/value_ops.rs` | `external_utf8_octets`; `convert_to_utf8`; `Utf8EncodingError`; `ValueOps::native_external_utf8_result` | actual C release and native string-getter authority remain independent of the backend binary-result issuer; counted native units select external UTF-8 bytes, C9 strict failures retain separate character index and source-byte offset, and successful binary results retain their genuine storage; Jim, foreign and authored-only issuers decline; arbitrary encoding, profile, streaming and private object identity require separate capabilities | `xtask-owner-resolution` |
| original exact case layout and scalar subject | `rust/tcl-lsp-core/src/refactor/datagroup/source_subject.rs` | `OriginalExactCaseSource`; `OriginalExactCaseSource::subject`; `OriginalExactCaseSource::pairs`; `original_exact_case_source_at_analysis`; `OriginalExactSwitchSource`; `OriginalScalarVariableSubject`; `original_exact_switch_source_at_analysis`; `ScalarVariableSourceSyntax`; `scalar_variable_source_syntax` | actual complete analysis, original source image, retained Realm/config and effective argv select exact case-sensitive case layout; the general carrier retains an arbitrary genuine original subject word and completed literal case data, while scalar advice independently requires one genuine substitution with no array index; explicit fragment scalar syntax retains its own selected grammar and complete spelling but supplies no actual document input or genuine original operand; original syntax and literal values supply no native read, entered body, execution equivalence or edit permission | `xtask-owner-resolution` |
| durable compiler field inventory | `rust/tcl-compiler/src/durable_inventory.rs` | `assert_durable_field_inventory` | compiler-owned exhaustive field witnesses share their declaration with published FunctionUnit and CompilationUnit field-name inventories; consumers review every name while private retained inputs stay private; field names and coverage accounting supply no source/configuration value, mutable sidecar or native execution authority | `xtask-owner-resolution` |
| counted ensemble map prefix construction | `rust/tcl-syntax/src/naming/native.rs` | `NativeNameProtocol::ensemble_map_prefix_input` | independently selected C8.5–9.1 purpose, original counted bytes and actual namespace report/parent fact; rooted head is unchanged, relative construction appends every byte and never applies later command-lookup extent; Runtime/VM obtain the parent from their actual namespace owner; reporting text, pure API inputs and finite public results supply no private holder/header identity, C84/Jim worker or compiler/body admission | `xtask-owner-resolution` |
| retained source taint metadata | `rust/tcl-compiler/src/compilation_unit.rs`; `rust/tcl-lsp-db/src/lib.rs`; `rust/tcl-compiler/src/taint/source_context.rs`; `rust/tcl-compiler/src/taint.rs`; `rust/tcl-compiler/src/taint/instance_classes.rs`; `rust/tcl-compiler/src/interprocedural.rs`; `rust/tcl-compiler/src/command_binding/retained_source.rs`; `rust/tcl-registry/src/resolved_invocation.rs` | `CompilationUnit::original_interproc_taints_for_body`; `TaintSourceContext`; `TaintSourceContext::for_module_function`; `find_taint_warnings_for_module_function`; `RetainedSourceModuleBindings::matches_module`; `ResolvedInvocation::authored_source_callable_factory_class`; `global_instance_class_source` | complete actual input/configuration/availability and independently matching Module/FunctionUnit owner select conditional lattice and wrapper advice; missing/foreign/stale input withholds metadata or retains uncertainty; Module-bearing replay selects the actual body/top-level identity and requires the interprocedural global-receiver owner and complete source input to match that same Module; missing or foreign correspondence cannot be reconstructed from labels or a function-only context; DB retains its original baseline on refusal, while function-only and explicit standalone ingresses stay separate, with no executed read/output, current taint value, sanitiser certainty or Native frame grant; receiver candidates use authentic selected factory/lifecycle schemas and original alias operands/literal names; global setup maps retain exact Module input and interpretation owner, source order and independent availability; function-only equality, foreign/stale carriers or replacements cannot donate a constructor, object, Native method or sanitisation fact; conditional receiver candidates also retain actual owner-validated OriginalDeclaration body IDs, using only setup preceding each declaration site; callable eager horizons, publication, completion and Native admission remain separate; retained sink warnings first use execution candidates and separately query existing closed original-source header advice only under an actual matching Module owner; this conditional source candidate supplies neither an executed dispatch nor Native/effect admission | none |
| original inlining source and alpha-rename geometry | `rust/tcl-compiler/src/inlining/frame.rs`; `rust/tcl-compiler/src/inlining/mod.rs`; `rust/tcl-compiler/src/inlining/rename.rs`; `rust/tcl-lexer/src/word_parts.rs`; `rust/tcl-syntax/src/naming.rs`; `rust/tcl-compiler/src/inlining/heads.rs` | `SourceContext::for_module`; `SourceContext::invocation`; `reads_only_bound_names`; `build_inlinable_map`; `rewrite_value_string`; `RawVarRef::name_range_in`; `RawVarRef::index_range_in`; `names_a_command`; `substitutes`; `root` | complete retained Logical Module input/image/config/store and selected invocation govern source eligibility; shared strict Tcl formal syntax and exact declaration names remain independent; selected checked scanner/root and borrowed component extents preserve scalar literals, braced literal indices and separate enabled index substitutions; whole-word emission and source geometry grant no Native frame, caller cell, compiled header, body admission or editing equivalence; head namespace hazards use actual Module LexerConfig with shared checked WordPart/index traversal, preserving literal escapes and combined indices; unknown decomposition conservatively refuses rewriting | none |
| original diagnostic protocol peers | `rust/tcl-compiler/src/source_graph.rs`; `rust/tcl-compiler/src/signature_scan/scope.rs`; `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs` | `procedure_body_for_declaration`; `SignatureSourceCommand::body_scope`; `SignatureNamespaceScope::is_strict_ancestor_of`; private `Analyser::original_dispatch_peers`, `original_dispatch_site`, `Analyser::dispatch_protocol_signatures` | current original declarations and actual Module-issued IDs retain all bodies; distinct authentic publication slots count peers; selected policy/body-scope geometry and unexpanded complete source argv establish conditional diagnostic suppression; ambiguous/unavailable scope and reporting labels grant no dispatcher, Native frame or execution | `xtask-owner-resolution` |
| Original formal name source geometry | `rust/tcl-compiler/src/signature_scan/formal_parameters.rs`; `rust/tcl-compiler/src/command_binding/declaration_preview.rs`; `rust/tcl-compiler/src/command_binding/logical_definition.rs`; `rust/tcl-compiler/src/analyser/types.rs`; `rust/tcl-compiler/src/interprocedural/transfer.rs` | `OriginalSourceFormalName`; `OriginalSourceFormalName::name`; `OriginalSourceFormalName::source_span`; `OriginalSourceFormalName::quoted_marker`; `SignatureSourceFormalParameters::name_sources`; `AnalysisResult::original_procedure_formal_name_sources`; `ModuleProcedures::original_source_module` | exact original declaration/allocation and current source/input; Native uses its retained formal topology, Logical its independent selected schema and original effective-to-written word; decoded name bytes, optional tight readonly source span and quoted-marker convention remain separate; unmapped/escaped parents withhold spans; no Native activation, cell or edit permission | `xtask-owner-resolution` |
| original declaration read metadata | `rust/tcl-compiler/src/command_binding/declaration_preview.rs`; `rust/tcl-compiler/src/script_binds.rs` | `LogicalProcedureDeclaration`; `OriginalProcedureReadDeclaration`; `original_procedure_read_declaration`; `authored_procedure_read_advice` | exact source/formal/body joins and full supplied owner; Native original topology and conditional Logical recipe remain separate; no installation, physical activation, entered body or successful invocation | none |
| original per-point CFG lookup metadata | `rust/tcl-compiler/src/registry_invocation/metadata_context.rs`; `rust/tcl-compiler/src/cfg_builder/mod.rs`; `rust/tcl-compiler/src/command_binding/cfg_lookup_context.rs` | `InvocationMetadataInput`; `OwnedInvocationMetadataContext`; `InvocationMetadataInput::retain`; internal implementation: `CfgLookupContexts` | shared tagged standalone, actual availability-only, complete supplied source and terminal unavailable modes across source baselines and CFGs; availability-only contexts cannot manufacture full source input, grammar or entry policy; checked source projections retain actual profile/config/store; the internal CfgMetadataContext alias does not create another owner; supplied original holders/head/grammar retain point binding/replay/rebound/wildcard queries; reporting names cannot reconstruct Native namespaces, missing/foreign/conflicting carriers refuse; no Native invocation/frame/completion or rewrite grant | none |
| captured Runtime byte storage | `runtime/rust/src/vars.rs`; `runtime/rust/src/vars/entered_container.rs`; `runtime/rust/src/frame.rs`; `runtime/rust/src/frame/container.rs`; `runtime/rust/src/frame/jim_link_store.rs`; `runtime/rust/src/frame/rmw.rs`; `runtime/rust/src/frame/native_variable_names.rs`; `runtime/rust/src/interp.rs` | `EnteredVariablePlace`; `CapturedContainerModel`; `CapturedByteStore`; `capture_byte_store_entered`; `VarTable::publish_scalar`; `VarTable::publish_elem`; `VarTable::publish_dictionary_root`; `capture_native_compiled_temporary_receiver`; `Interp::store_captured_byte_variable`; `Interp::store_native_compiled_temporary_checked` | read, defined/array queries, ensure and store share the same classifier/link steps; reached model conversion and dictionary getters run after releasing cell, frame and namespace borrows, retaining original table and frame incarnation through the selected backing checks; one reached byte-name resolution captures the original retained receiver or weak cell/table binding under inherited operation currency; the stored old native owner retires after frame, namespace and cell borrows end, followed by the original context and binding checks before traces or output; anonymous stores also retain their genuine frame incarnation, and input allocation leases add no native reference; addressed ports and owned release-global publication remain trace-free; selected Jim dictionary paths retain their independent recipe and generic compatibility copying grants no native path or aggregate C/Jim pointer chronology | none |
| captured Runtime variable removal | `runtime/rust/src/vars/entered_container.rs`; `runtime/rust/src/frame/retired_variable.rs`; `runtime/rust/src/frame/destruction.rs`; `runtime/rust/src/frame/rmw.rs`; `runtime/rust/src/interp/native_variable_teardown.rs`; `runtime/rust/src/interp/native_variable_names/native_unset.rs`; `runtime/rust/src/interp/native_jim_links.rs`; `runtime/rust/src/namespace.rs`; `runtime/rust/src/frame.rs` | `RetiredVariableParts`; `EnteredVariablePlace::remove`; `VariableReceiver::publish_unset`; `VarTable::publish_remove_after_default`; `VarTable::publish_remove_elem`; `publish_array_destruction`; `finish_destruction_with_check`; `Interp::remove_captured_byte_variable`; `Interp::retire_namespace_variables` | the same original classifier/link steps select the table, frame incarnation and physical root before removal; cell bookkeeping and owner extraction finish before any removed value, default or key retires outside storage borrows; the carrier transfers only actual stored references, and original currency plus table/frame checks precede later callbacks or publication; selected C unset keeps its transferred dummy scalar owner through unset traces, then retires it, while generic byte compatibility and Jim key/value removal retain independent schedules; detached arrays and saved namespace cleanup release every already selected owner after first refusal, stop later optional traces and never adopt a replacement table; input allocation leases supply no native reference; metadata/link preparation and occupied command publication remain separate consumers, and Runtime model transition supplies no stock Native chronology | none |
| original dictionary path publication | `rust/tcl-registry/src/native_dictionary.rs`; `rust/tcl-vm/src/interp/native_dictionary.rs`; `runtime/rust/src/dict.rs`; `runtime/rust/src/frame/jim_lookup.rs`; `runtime/rust/src/interp/native_jim_increment.rs`; `runtime/rust/src/cmd_dict.rs` | `InvocationDialect::native_dictionary_path_publication`; `InvocationDialect::native_dictionary_root_protocol`; `NativeDictionaryRootPurpose`; `EnteredNativeDictionary::prepare_original`; `CapturedJimDictionaryCell`; `OriginalJimDictionaryRoot`; `Interp::mutate_original_jim_dictionary_path`; `Vm::jim_dictionary_path_update_bytes` | actual selected C original-name versus Jim variable/array purpose; Runtime Jim retains the exact weak cell, original root, frame incarnation and retained alias/member joins under inherited currency; path sharing precedes conversion and copied children publish immediately, while array sharing follows conversion and an unshared read follows member lookup; missing generic paths publish an empty root, required sugar unset refuses; allocation leases add no native reference before sharing, and compatibility copying remains separate; finite public319 values and original SDK schedules do not attest software callback retirement timing, header identity or VM root coverage | none |
| selected possible option source metadata | `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-compiler/src/registry_invocation.rs` | `AuthoredSourceOptionArguments`; `ResolvedInvocation::authored_source_possible_option_arguments`; `conditional_option_arguments_with_metadata_context` | selected actual availability, effective original argv, admitted value widths/reserved operands/terminator and possible ordinals; dynamic words retain admitted continuations; no handler, evaluated value, Normal or editable correspondence | none |
| complete supplied SSA metadata scans | `rust/tcl-compiler/src/ssa.rs`; `rust/tcl-compiler/src/cfg_builder/mod.rs` | `SsaInvocationContext`; `switch_must_defines_with_source_input` | actual supplied owner through definition/use/phi/nonlocal/destruction and recursive role scans; missing metadata preserves independent lexical substitutions and withholds descriptor-derived roles/definitions; explicit standalone wrappers remain separate | none |
| original substitution purity metadata | `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/command_binding/materialized_footprint.rs`; `rust/tcl-compiler/src/optimiser/elimination/source_purity.rs`; `rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs` | `original_substitution_purity_with_metadata_context`; `SourceInvocationBinding::original_logical_procedure_call_targets`; `substitutions_are_pure` | authentic nested bindings, whole effective operands and full source owner; Native requires independent selected handler alternatives, Logical uses its source recipe and exact procedure allocation/declaration join; literal AssignConst has no substitution query; no evaluation, no-raise, observer or erasure grant | none |
| original summary source dependencies | `rust/tcl-compiler/src/taint_interproc.rs`; `rust/tcl-lsp-db/src/lib.rs` | `command_subst_callees`; `summary_deps_key`; `proc_summary_cascade` | actual current Module-issued body and retained positioned source/Registry correspondence select original substitution heads under the nested Module lexer configuration; unresolved or incomplete candidates retain every known issued summary dependency and tracked document edits retain memo currency; dependency IDs supply no selected execution edge, runtime command/receiver or Normal completion | none |
| original declared argument read bridge | `rust/tcl-compiler/src/var_escape/original_slots.rs`; `rust/tcl-compiler/src/common_aot_plan/declared_arguments.rs`; `rust/tcl-compiler/src/common_aot_plan.rs` | `OriginalDeclaredProcedureArgumentSlots::incoming_source_read`; `DeclaredArgumentIdentity`; `DeclaredArgumentEvidence`; `CommonAotProofPlan::declared_arguments`; `CommonAotProofPlan::declared_argument_for_incoming_read` | authentic original declaration/topology/image, fixed counted argument owner and complete reached source-read/cell alternatives; proposed convention ordinals stay separate from physical SSA and materialisable-slot identity, while observer/content/caller-type/source-metadata/runtime-guard/body-admission premises remain independent; missing owners, changed source/read receipts and unproved callers refuse | none |
| supplied original SSA definition mapping | `rust/tcl-compiler/src/ssa.rs`; `rust/tcl-compiler/src/registry_invocation.rs` | `SsaSourceView::original_definition_name_with_metadata_context`; `normal_transfer_invocation_with_metadata_context` | complete actual availability/source grammar and independent original SSA definition cell/phase/effective receiver; actual AnalysisResult/Module/FunctionUnit consumers withdraw missing, foreign or incompatible metadata; explicitly standalone sibling is separate; no read completion, contents, native cache or edit grant | none |
| diagnostic policy | `rust/tcl-lsp-core/src/diagnostic_policy.rs`; `rust/tcl-lsp-core/src/diagnostic_report.rs`; `rust/tcl-lsp-core/src/config_ini.rs` | `apply`; `Policy`; `PolicyBuilder`; `Report`; `Finding`; `Directives`; `dialect_overlaps`; `document_report`; `standalone_findings`; `optimise_under_policy`; `settings_from_ini`; `merge_settings`; `global_layer`; `project_layer_for` | the dialect's overlap table; the document's configuration layers, resolved per code; the step order is release-invariant | none |
| checked original numeric stages | `rust/tcl-platform/src/numeric_environment.rs`; `runtime/rust/src/interp/native_operation_currency.rs`; `rust/tcl-vm/src/value.rs`; `rust/tcl-vm/src/interp/native_numeric_ops.rs`; `rust/tcl-vm/src/interp.rs` | `CheckedNumericEnvironment`; `native_scalar_probe_checked_with_string`; `native_number_probe_checked_with_string`; `native_scalar_getter_checked_with_string`; `OriginalNumericOps`; `Vm::authored_numeric_context`; `Vm::numeric_policy_context` | independently supplied actual numeric environment, callback-neutral before/after checker and ABI observation cell; conversion kernels retain selected recipe; original Runtime/VM operation owner preserves the full typed first cause, while neutral availability grants no interpreter, object, frame or handler authority; primary retirement is checked outside its representation borrow | `xtask-owner-resolution` |
| Original variable read roles and lexical references | `rust/tcl-compiler/src/var_refs.rs`; `rust/tcl-compiler/src/analyser/scope.rs` | `original_variable_name_role_words`; `VarReferenceScanner::scan_lexical_word`; `VarReferenceScanner::scan_standalone_word`; `VarReferenceScanner::scan_lexical_script`; `VarReferenceScanner::scan_standalone_script` | original complete image/channel, full resolved analysis input, exact lexer configuration and retained receiver/word provenance; roles follow effective-to-written ordinals, literal name values and original content spans; empty names and literal sigils remain distinct from missing/dynamic input; lexical components grant no receiver roles, standalone catalogue assumptions only protect liveness, and neither grants Native execution, completed reads or editing authority | `xtask-owner-resolution` |
| Original Logical source effect and frame carriers | `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/realm.rs`; `rust/tcl-compiler/src/registry_invocation/source_structure.rs`; `rust/tcl-compiler/src/registry_invocation/logical_frame.rs`; `rust/tcl-compiler/src/cfg_builder/global_write_info.rs`; `rust/tcl-compiler/src/cfg_builder/upvar_info.rs`; `rust/tcl-compiler/src/word_subst.rs` | `SourceInvocationBinding::has_original_source_layout_owner`; `SourceInvocationBinding::retain_original_logical_source_advice`; `SourceInvocationBinding::original_logical_source_words`; `original_logical_registry_words_for_module`; `original_logical_frame_invocation_for_module` | positive retained Logical input and whole immutable source tape own readonly source words; current Module entry/configuration/input/context/store/image and unchanged whole argv are revalidated; any reached/compiler/declaration layout owner keeps its existing route or refusal; branch joins retain only equal complete original advice; genuine recorded nested children retain their own coordinates; Global/Upvar and expression readers consume conditional MAY source roles, while frame arguments retain actual literal payloads and written origins through a separate Source carrier; missing/stale/foreign/Native/Hosted/unknown inputs refuse, with no strict invocation, Normal transfer, execution, entered frame or Native grant | `xtask-owner-resolution` |
| Original Hosted declaration body joins | `rust/tcl-compiler/src/command_binding/hosted_procedure_body.rs`; `rust/tcl-compiler/src/source_graph.rs`; `rust/tcl-compiler/src/lowering/hosted_source_bodies.rs`; `rust/tcl-compiler/src/compilation_unit.rs`; `rust/tcl-compiler/src/analyser/diagnostics.rs` | `OriginalHostedProcedureBody`; `hosted_procedure_body_for_procedure`; `hosted_procedure_body_for_function`; `hosted_procedure_body_for_declaration` | actual original RootScript/Global RuleLoader declaration retains complete source/input/Registry, original argv, formal/body coordinates, every alternative and actual Lowerer-issued Script/current entry; Module issuer and actual current body/card are required; declaration joins additionally retain the actual analysis row, original name/purpose/site/complete words and body extent, select a unique current body unit and preserve source-only cards when a body is unavailable; and function joins additionally require matching current metadata and exact held card Arc; changed, missing, foreign and unsupported qualified declaration owners refuse; report labels cannot select bodies, and declaration correspondence supplies no event activation, Native publication, physical frame or source editing permission | `xtask-owner-resolution` |
| Original Authored declaration and child source bodies | `rust/tcl-compiler/src/registry_invocation/authored_source_bodies.rs`; `rust/tcl-compiler/src/registry_invocation/authored_body_scope.rs`; `rust/tcl-compiler/src/lowering/authored_source_bodies.rs`; `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/ir.rs` | `OriginalAuthoredProcedureBody`; `original_authored_procedure_body`; `OriginalAuthoredSourceBodyScope`; `SourceInvocationBinding::retain_original_authored_source_body_scope`; `SourceInvocationBinding::has_original_authored_source_body_name_role_owner`; `SourceInvocationBinding::original_authored_source_body_scope_for_name_roles` | root Standalone grammar retains the actual immutable Registry, complete original header/body words, configuration and namespace recipe; Lowerer retains the authentic declaration span and whole Script under the current Module entry/profile, and issued inventory joins revalidate header/site, formals/body, source/store and readonly kind/publication; independent child scopes retain the whole original lexical plan and exact child vector/store, with terminal source-purpose refusal and unknown name coverage; missing scope stays unowned, labels do not select bodies, qualified coordinates prove no namespace existence, and these owners grant no Native entry, Normal effect, executing body or physical frame | `xtask-owner-resolution` |
| Original Native-name declaration source bodies | `rust/tcl-compiler/src/command_binding/generated_procedures.rs`; `rust/tcl-compiler/src/command_binding/declaration_preview.rs`; `rust/tcl-compiler/src/command_binding/source_transition_advice/native_declared_body.rs`; `rust/tcl-compiler/src/command_binding/source_transition_advice/source_procedure_publications.rs`; `rust/tcl-compiler/src/lowering/installed_bodies.rs`; `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/compilation_unit.rs`; `rust/tcl-compiler/src/source_graph.rs` | `OriginalNativeProcedureBody` | separate authentic Native-name read-flow or frame-free original source declaration recipes retain complete source/configuration/input/Registry, original name/formals/body and actual Lowerer-issued Script/current entry; frame-free source recipe uses actual transition-tape declaration/schema/ParamList topology and byte slot namespace geometry without a physical namespace token; Module issuer and current body/card select readonly original source inventory, keeping repeated declarations distinct and excluding source cards from executable roots; changed/missing/foreign or incomplete ownership refuses, reporting labels do not select bodies, and the card supplies no installed allocation, successful publication, callable body, Native formal ABI or entered frame | `xtask-owner-resolution` |
| Original Logical procedure MAY dependencies | `rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs`; `rust/tcl-compiler/src/cfg_builder/summary_candidates.rs`; `rust/tcl-compiler/src/command_binding/retained_source.rs` | `original_logical_procedure_may_dependencies_for_module`; `OriginalProcedureSummaryOwners::may_calls`; `RetainedSourceModuleBindings::matches_original_procedure_summary` | complete current Module/input/source/configuration/Registry, held original caller vector and BodyWalk scope join actual issued Named or Logical declaration body IDs through original header/body/formal/site geometry; retained body kind/publication/declaration site remain truth while qualified reporting labels cannot select bodies; only named-only source descriptors without alias or mutation uncertainty can exclude a procedure dependency; Global/CFG consume body IDs and opacity independently of arguments, return values, selected calls or Native entry | `xtask-owner-resolution` |
| Original procedure MAY dependencies | `rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs`; `rust/tcl-compiler/src/command_binding/declaration_preview.rs`; `rust/tcl-compiler/src/command_binding/generated_procedures.rs`; `rust/tcl-compiler/src/command_binding/original_command_table.rs`; `rust/tcl-compiler/src/taint_interproc.rs` | `OriginalProcedureMayDependencies`; `original_procedure_may_dependencies_for_word_for_module` | existing implementation-backed body selection keeps its own complete owner; separate original Native-name static lookup and byte declaration slots/policies join only authentic current Module-issued source cards, preserving repeated body IDs and open handler/alias alternatives; dependency readers retain every known ID when alternatives or dynamic heads are open, without reducing returns, argv, call edges or Native admission | `xtask-owner-resolution` |
| Original var-escape substitution inventory | `rust/tcl-compiler/src/var_escape/helpers.rs`; `rust/tcl-compiler/src/var_escape/walker.rs`; `rust/tcl-compiler/src/var_escape/cfg_propagation/walker.rs`; `rust/tcl-compiler/src/var_escape/handlers.rs`; `rust/tcl-compiler/src/var_escape/cfg_propagation/handlers.rs`; `rust/tcl-compiler/src/word_subst.rs`; `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs` | `OriginalSubstitutionInvocation`; `OriginalSubstitutionInventory`; `original_substitution_invocations`; `OriginalAuthoredSourceSubstitutionWords`; `checked_authored_source_substitution_words`; `original_authored_substitution_name_roles` | actual retained invocation metadata/configuration/source/whole words feed the shared lexical and selected Expr collectors; tree/CFG consumers retain original literal payloads and typed introspection roles, while CFG conditions must match the retained original expression; unavailable/incomplete/source-less owners widen escape facts, synthetic companions are excluded, and frameless classification requires its independent Normal handler owner; Authored child scopes retain actual lexical/selected Expr substitution vectors and the immutable store, reject expansion before original role projection, and always retain unknown coverage even for an empty inventory; every such child stays non-frameless, with no reconstructed Point/input or stronger purpose; test-only info roster/value wrappers preserve original fixture assertions and supply no production, Native or entered-frame coverage | `xtask-owner-resolution` |
| Original Authored control-flow source grammar | `rust/tcl-compiler/src/registry_invocation/authored_controls.rs`; `rust/tcl-compiler/src/registry_invocation/authored_source_structure.rs`; `rust/tcl-compiler/src/registry_invocation/authored_body_scope.rs`; `rust/tcl-compiler/src/lowering/mod.rs`; `rust/tcl-compiler/src/lowering/structured.rs` | `AuthoredSourceInvocation`; `original_authored_control_invocation` | actual closed Standalone header or retained readonly child scope owns complete original words and immutable Registry; original NativeWord expansion refuses before ordinal mapping, selected Plain/current/no-frame hooks require accepted arity/complete roles and every Body has SameInvocation timing and original static script geometry; present source scope stays terminal, missing readonly ownership remains unavailable, and AuthoredSourceSchema supplies analysis grammar independently of strict metadata, Normal completion, physical frames, Native admission or bytecode hooks | `xtask-owner-resolution` |
| Original source case-clause grammar | `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-registry/src/case_bodies.rs`; `rust/tcl-compiler/src/registry_invocation/hosted_structure.rs` | `ResolvedInvocation::authored_source_case_clause_projection` | already selected CaseListSpec and actual option availability own exact source argv count/layout; KnownBytes become literal grammar only when lossless UTF-8 without NUL, and list parsing uses the actual source WordValueRules without inventing an InvocationDialect; dynamic selection/no-match alternatives remain explicit, Native case projection preserves its independent dialect query, and clause locations supply no executable role completeness, body entry or Normal result | `xtask-owner-resolution` |
| Original Hosted control-flow source grammar | `rust/tcl-compiler/src/registry_invocation/hosted_structure.rs`; `rust/tcl-compiler/src/registry_invocation/source_taint.rs`; `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/lowering/mod.rs`; `rust/tcl-compiler/src/lowering_hooks.rs` | `SourceStructuredInvocation`; `original_hosted_structured_invocation`; `original_hosted_source_registry_metadata` | actual Lowerer source issuer retains complete input and exact context Arc/configuration with explicit policy concordance; whole original words and actual supplied Registry select bounded If/Switch hooks with accepted arity/static bodies and no expansion; If requires complete source roles, while Switch retains selected CaseListSpec clause geometry from actual full-input WordValueRules and selected option availability, with unknown selection/no-match alternatives separate from executable roles; Logical and Hosted source carriers remain distinct, Hosted metadata requires RuleLoader and supplies Unknown completion, no error context or ScopedCommandEnv; missing/changed/foreign input or divergent explicit vendor policy refuses; bytecode admission and actual Native/Normal runtime invocation paths cannot borrow Hosted source grammar | `xtask-owner-resolution` |
| Original taint replay reports | `rust/tcl-compiler/src/taint.rs`; `rust/tcl-compiler/src/compiler_checks.rs`; `rust/tcl-lsp-db/src/lib.rs`; `rust/tcl-lsp-server/src/lib.rs`; `rust/tcl-explorer/src/serialise.rs`; `rust/tcl-lsp-core/src/graphs.rs` | `TaintReplaySourceSnapshot`; `UnavailableTaintReplayReason`; `UnavailableTaintReplay`; `TaintReplayAnalysis`; `TaintWarningReport`; `TaintReplaySourceSnapshot::matches_module`; `UnavailableTaintReplayReason::as_str`; `TaintReplayAnalysis::is_complete`; `TaintReplayAnalysis::unavailable`; `TaintReplayAnalysis::unavailable_replays`; `TaintReplayAnalysis::merge`; `taint_warning_report_for_cu`; `taint_warning_report_for_module_function`; `CompilerCheckReport`; `compiler_check_report_with_generic_patterns`; `compiler_check_report_with_solved_and_patterns` | proven warnings remain separate from required replay availability; actual selected descriptor pointers and typed constructor transitions own source scaffolds; missing original source/constructors retain explicit refusals, not empty successful setup; residuals retain full source entry and actual Module owner, original bounds or genuine registration anchors, and unmapped synthetic bytes have no original source snapshot; DB/direct/memo/server and display consumers retain scoped reports, with whole-CU stored callbacks independent of per-function command-head coverage; stale/missing correspondence supplies no document coordinates or execution authority | `xtask-owner-resolution` |
| Original Native declaration child source grammar | `rust/tcl-compiler/src/command_binding/source_transition_advice/native_declared_body.rs`; `rust/tcl-compiler/src/command_binding/generated_procedures.rs`; `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/command_binding/source_transition_advice/body_effects.rs`; `rust/tcl-compiler/src/command_binding/source_transition_advice.rs`; `rust/tcl-compiler/src/lowering/installed_bodies.rs`; `rust/tcl-compiler/src/lowering/mod.rs`; `rust/tcl-compiler/src/registry_invocation/hosted_structure.rs`; `rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs` | `AdviceInvocationContext::retain_native_expression_syntax_inventory`; `OriginalNativeSourceBodyChildren`; `OriginalNativeProcedureBody::source_children`; `SourceInvocationBinding::original_native_source_words`; `SourceInvocationBinding::original_native_source_may_owner`; `SourceInvocationBinding::has_original_native_source_command`; `original_native_source_structured_invocation` | exact declaration source-origin Arc and complete retained input issue immutable original child/tape/vector receipts; present Native child receipt selects a terminal MAY source reader before legacy lookup; actual requested original name bytes and separate deferred-body/vector purpose receipt must agree, missing/poisoned purpose or known barriers/reached-target mismatch refuses; current actual Module/card/body/issued ID owns MAY dependencies, which retain every original inventory ID with unknown alternatives; original schema words preserve actual descriptor/body roles and reached-target compatibility, refusing expansion, foreign issuers and changed vectors; separate NativeSourceSchema supplies readonly analysis hooks only, no physical namespace/frame, Native compiler admission, Normal effects, installed allocation or selected call; complete selected Expr roles and original nonexpanded procedure words may retain independently owned child syntax while speculative effects/advice remain discarded; divergent branch receipts clear | `xtask-owner-resolution` |
| Original source SSA read associations | `rust/tcl-compiler/src/ssa.rs`; `rust/tcl-lsp-core/src/hover.rs` | `OriginalSourceSsaRead`; `OriginalSourceSsaRead::function`; `OriginalSourceSsaRead::span`; `OriginalSourceSsaRead::symbol`; `OriginalSourceSsaRead::version`; `original_source_read_at_unit` | private existing represented read references and independent readonly source occurrences retain exact original Variable/Template substitution extent and the current point FunctionUnit/SSA symbol; source-only occurrences always withhold represented Version, while an independently matching represented read keeps its own precedence; complete current Module/source/input/configuration/Registry/origin, keyed retained body truth and prepared CFG must agree, and present source-purpose refusal is terminal; unavailable represented Version supplies no contents annotation or definition fallback; hover uses the exact selected ValueKey for type, taint and first-use advice, without joining function/variable display names or granting physical read/value/edit permission | `xtask-owner-resolution` |
| Original Authored body-name candidates | `rust/tcl-compiler/src/registry_invocation/source_body_names.rs`; `rust/tcl-compiler/src/registry_invocation/authored_source_structure.rs`; `rust/tcl-compiler/src/var_escape/known_names.rs` | `original_authored_body_name_candidates` | actual complete Authored parent header/vector/configuration/store and selected SameInvocation script role retain each original body word/full image/content extent; shared NestedScript/lexical/role/list geometry supplies possible names with unknown coverage always retained, including empty/sigil data; bounded vocabulary supplies pessimistic spill candidates without entering a frame, closing lookup, supplying Normal facts or reparsing reconstructed scripts | `xtask-owner-resolution` |
| Authored literal namespace-prefix hints | `rust/tcl-syntax/src/naming.rs`; `rust/tcl-mcp/src/irule_gen.rs` | `authored_namespace_hint_tail` | exact literal namespace prefix retains the written remainder, including empty/repeated separators and rooted exclusion; actual source-reference or selected variable-role/full-input owners remain independent; hint geometry supplies no qualification, static cell/storage, event execution or Native grant | `xtask-owner-resolution` |
| Original literal variable-name operands | `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs`; `rust/tcl-compiler/src/var_escape/handlers.rs`; `rust/tcl-compiler/src/var_escape/cfg_propagation/handlers.rs`; `rust/tcl-compiler/src/var_escape/state.rs`; `rust/tcl-compiler/src/var_escape/cfg_propagation/state.rs` | `OriginalVariableNameOperand`; `OriginalVariableNameOperand::as_str`; `SourceNameRoleInventory::original_variable_name_operand`; `ResolvedStatementInvocation::original_variable_name_operand`; `original_assignment_name_operand` | an actual selected VarRead/VarWrite ordinal and original literal value issue the opaque name datum; tree/CFG possible-name transfer preserves legal empty and literal sigil names with reasons, independently of legacy raw empty/dynamic sentinels; scalar assignment transfer additionally requires selected FIRST_ARG_VARNAME plus VarWrite and exact existing IR name-field correspondence, retains the unchanged source residual, and uses actual Script or SSA-point tokens; missing roles/values and stale owners refuse, unknown source coverage remains separate, and a name datum grants no physical cell, completed store, frame, Normal result or catch/try receiver semantics | `xtask-owner-resolution` |
| Original source analysis attachments | `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/command_binding/retained_source.rs`; `rust/tcl-compiler/src/command_binding/body_template.rs`; `rust/tcl-compiler/src/command_binding/source_transition_advice/native_declared_body.rs`; `rust/tcl-compiler/src/ssa.rs` | `OriginalSourceAnalysisOwner`; `SourceInvocationBinding::original_analysis_attachment`; `SourceCommandBindings::original_analysis_owner`; `RetainedSourceModuleBindings::owns_original_analysis_attachment` | actual source-analysis entry retains its source-origin Arc, configuration and complete metadata; fact-cache hits get distinct private attachments while ordinary clones/projections preserve theirs; original and nested tokens retain the issuing attachment, and Native child/current SSA token joins require that exact attachment plus existing whole input/source/vector/card guards; missing or foreign attachments refuse even for identical input; certified neutral body templates clear attachments only after independent dependency capture and restore current original tokens before analysis reuse; attachment identity grants no callable publication, represented value, Native entry or physical frame | `xtask-owner-resolution` |
| Selected codegen wrapping traits | `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/codegen/cmd_subst.rs`; `rust/tcl-compiler/src/codegen/control_flow.rs` | `AdmittedNativeCompilerInvocation::traits`; `CodegenCtx::selected_invocation_traits` | source-backed wrapping requires the actual specialization scope, complete original head/argv and independent compiler admission, with terminal refusal before catalogue lookup; source-less metadata keeps its actual surface query and exact authored head rule; traits supply wrapping metadata only, with no successful handler, Normal completion, frame or erasure grant | `xtask-owner-resolution` |
| Original dictionary destination data | `rust/tcl-compiler/src/lowering/structured.rs` | `Lowerer::original_dict_write_name` | actual original selected Dict hook/VarWrite role and effective-to-written ordinal ledger require direct source slot1 before retaining a literal destination, including empty/sigil data; only the directly mapped original written slot supplies the name, inserted/expanded or differently mapped operands cannot donate another argument's name, and unavailable literal ownership declines to original generic dispatch without static defs; safe-on-uninit uses that same selected descriptor/effective words/context/retained realm and defaults false, while body mutator/barrier behavior remains independent; names supply no physical cell, frame or completed write | `xtask-owner-resolution` |
| Original control receiver name data | `rust/tcl-compiler/src/lowering/structured.rs` | `Lowerer::original_control_receiver_value` | actual selected original source literal supplies catch/try name data, while bytecode retains its independent Native written-value query; empty/sigil/array spelling is preserved without VarRef normalization, handler lists split decoded values once under current WordValueRules, and unavailable dynamic data remains runtime dispatch; names grant no physical cell, completed output or frame, and engine-specific empty output behavior retains its separate protocol | `xtask-owner-resolution` |
| Original source name-role inventory | `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs`; `rust/tcl-compiler/src/var_escape/known_names.rs`; `rust/tcl-compiler/src/var_escape/helpers.rs`; `rust/tcl-compiler/src/var_escape/handlers.rs`; `rust/tcl-compiler/src/var_escape/cfg_propagation/handlers.rs` | `SourceNameRoleInventory`; `original_source_name_roles` | actual complete original vector/site/input/configuration and immutable Registry retain separate source and stronger layout purposes; present Logical source ownership is terminal, incomplete role coverage/unknown obligations retain pessimism, and literal empty/sigil values remain data; tree/CFG name and introspection consumers reuse the same role ordinals/list grammar and actual store through children; metadata supplies no Normal completion, physical frame/cell or slot eligibility | `xtask-owner-resolution` |
| Configured single-command lexical projection | `rust/tcl-compiler/src/word_subst.rs`; `rust/tcl-compiler/src/codegen/cmd_subst.rs`; `rust/tcl-compiler/src/codegen/control_flow.rs`; `rust/tcl-compiler/src/codegen/statements.rs` | `SingleCommandSourcePart`; `SingleCommandSourcePurpose`; `single_command_source_parts`; `parse_cmd_parts`; `parse_cmd_parts_expand` | actual supplied source bytes and LexerConfig select one complete lexical command, with distinct Script and CommandSubstitution wrapper purposes; shared NativeWord content/grouping preserves braces, escapes and expansion markers, while malformed, noncontiguous or multiple-command geometry declines to runtime EVAL; public default standalone wrappers retain their independent presentation purpose; lexical tuples supply no command binding, original analysis input, Native admission or handler completion | `xtask-owner-resolution` |
| Original written word extents | `rust/tcl-compiler/src/word_expr.rs`; `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/registry_invocation/native_compilation_source.rs`; `rust/tcl-compiler/src/command_binding/compiled_preflight.rs` | `WordExpr::original_written_span`; `WordExpr::original_command_substitution_span` | actual Source provenance and bounds retain typed extent conventions: BracedLiteral/Variable use native inner-closer widening, Command requires exact whole bytes matching retained spelling, and a sole quoted command component remains separate from its wrapper; replay/header/preflight/literal-body/Logical body consumers keep independent current binding/input/frame/card/admission checks; lexical geometry supplies no identity or execution permission | `xtask-owner-resolution` |
| Original handler slot eligibility | `rust/tcl-compiler/src/var_escape/slot_resolution.rs` | `call_disables_slots` | actual parent Script's original tokens, immutable Registry and complete current metadata select strict Normal frameless handler facts with complete role cardinality/arity and descriptor/realm correspondence; literal name payloads preserve empty/sigil data, while unknown owners/observers/dynamic operands or unsupported introspection reject slot eligibility; Source MAY names cannot prove physical absence or slots | `xtask-owner-resolution` |
| CFG structural variable-name data | `rust/tcl-compiler/src/ir_helpers.rs`; `rust/tcl-compiler/src/cfg_builder/mod.rs` | `structural_variable_write_projection`; `structural_write_roots` | typed Assign/Incr destinations retain evaluated literal name data, including empty and sigil names; possible array-element writes project roots only for definition/invalidation coverage, never scalar constants; for-init scalar publication requires an actual literal scalar destination, element writes withdraw the root value and opaque writes clear constants | `xtask-owner-resolution` |
| Compatibility code-lens member keys | `rust/tcl-lsp-core/src/code_lens.rs`; `rust/tcl-compiler/src/analyser/types.rs` | `lens_qname_exists`; `class_member_key`; `class_property_key`; `class_constructor_key`; `class_destructor_key` | presentation existence compares actual declared members and effective constructor/destructor using the analyser recipes shared with lens emission; original nonempty member guards remain and rendered keys grant no original declaration identity, invocation or reference-count coverage | `xtask-owner-resolution` |
| Logical generated property completion | `rust/tcl-lsp-core/src/completion.rs`; `rust/tcl-compiler/src/analyser/class_hierarchy.rs` | `generated_property_method_items`; `source_method_name_word`; `ClassHierarchy::property_accessor_methods` | current completed Logical class and all inherited providers must match their retained source owners; the supplied Registry must match the analysis Registry semantic snapshot before shared property recipes supply generated name data; each item quotes one ASCII word under the actual retained input/lexer configuration, and unavailable owners, foreign stores or unrepresentable data decline without granting a Native method, lookup, dispatch or edit permission | `xtask-owner-resolution` |
| Exact native object allocation births | `rust/tcl-runtime-api/src/native_object_incarnation.rs`; `runtime/rust/src/obj.rs`; `runtime/rust/src/interp/native_arguments.rs`; `rust/tcl-vm/src/value.rs` | `NativeObjectIncarnation`; `NativeObjectIncarnation::fresh`; `NativeObjectIncarnation::same_as` | actual Runtime/VM object allocations retain an opaque birth identity; exact original-argument, primary/procedure and checked-lifetime readers compare that birth as well as their existing currentness; an address or neutral lease cannot identify a replacement incarnation, and birth identity grants no native reference, getter, allocator/pool permission or resident-header following | `xtask-owner-resolution` |
| Logical per-object member insertion and masks | `rust/tcl-compiler/src/analyser/types/logical_object_members.rs`; `rust/tcl-compiler/src/analyser/handlers.rs`; `rust/tcl-lsp-core/src/completion.rs` | `LogicalSourceObjectMemberState`; `LogicalSourceObjectMembers`; `AnalysisResult::logical_source_object_member_state`; `LogicalSourceObjectMembers::name_operand`; `LogicalSourceObjectMembers::source_word`; `LogicalSourceObjectMembers::export_names`; `LogicalSourceObjectMembers::unexport_names`; `LogicalSourceObjectMemberState::capture` | actual completed per-object source state, membership pointer and full Realm/image/input/metadata select the view; exact method operands and independent source visibility statements supply names/masks under the retained grammar; missing or foreign states decline and an external catalogue cannot replace their masks; insertion data supplies no Native receiver, method dispatch or editing permission | `xtask-owner-resolution` |
| Original definition-script source coordinates | `rust/tcl-compiler/src/ir/definition_script.rs`; `rust/tcl-compiler/src/lowering/mod.rs` | `OriginalDefinitionScriptSource`; `Module::original_definition_script_source`; `OriginalDefinitionScriptSource::original_words`; `OriginalDefinitionScriptSource::body_word`; `OriginalDefinitionScriptSource::content_span`; `OriginalDefinitionScriptSource::declaration_span`; `OriginalDefinitionScriptSource::family` | actual definition-script allocation retains its issued ID, whole source/input/entry/Registry/configuration, original definer vector and exact literal body extent; current reborrow also requires the same selected descriptor, definition grammar and written Body role; duplicate/foreign/stale or expanded inputs refuse; coordinates identify no callable body, namespace publication, physical frame, variable cell or represented SSA version | `xtask-owner-resolution` |
| Logical object assignment and factory candidates | `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/object_types.rs`; `rust/tcl-compiler/src/object_types/logical_source_candidates.rs`; `rust/tcl-lsp-core/src/semantic_tokens.rs` | `AdvisoryValueAssignment`; `advisory_value_assignments_in_context`; `original_advisory_value_assignments_with_metadata_context`; `advisory_object_factories_with_metadata_context`; `object_handle_facts_for_analysis`; `original_logical_constructor_class`; `logical_source_object_handle_classes`; `logical_source_unit_input` | positive complete Logical metadata selects MAY assignments and original factory source schemas; possible original schema candidates may coexist with unknown handlers only under both complete Logical metadata and Logical source purpose from the actual retained binding, retaining complete word/origin structure, actual candidate membership and current input/Registry/layout/configuration; closed unanimous schema selection for other purposes stays independent; current source/Registry/context/original argv and accepted arity retain exact mandatory publication layout independently of nominal result class; optional/control layouts withhold named publication, unknown contents remain open, and original sole variable substitutions retain configured scalar grammar; scoped candidates and reporting unions supply no completed store, physical read, runtime class, Native/Normal or dispatch facts; the actual AnalysisResult seeds only readonly original constructor/shape/class joins, Script and Call routes share one candidate harvester, readonly procedure-scope joins require current issued bodies and FunctionUnit metadata with duplicate/span guards, and strict maps and Native/hosted adapters retain independent owners; semantic tokens reuse the source-aware nominal union under the same complete Logical source/input/Module/configuration/Registry admission as retained candidates; Native/unknown-entry sources refuse this reader, unavailable ownership is not an empty successful union, and the generic Native reader remains independent | `xtask-owner-resolution` |
| Logical object-fact report correspondence | `rust/tcl-compiler/src/object_types/report_comparison.rs`; `rust/tcl-compiler/src/analyser/diagnostics.rs`; `rust/tcl-compiler/src/analyser/types.rs` | `ObjectHandleFacts::retain_report_source`; `ObjectHandleFacts::analysis_report_eq` | current Logical full input/image/Registry and Module owner retain unique original procedure/method/definition coordinates; report comparison maps only body/frame/definition IDs through those checked coordinates and compares scope/class/return/candidate reports, preserving exact physical read, proven-read and Normal-result facts; changed/missing/duplicate correspondence refuses, absent correspondence on both sides uses ordinary exact equality; this reporting comparison does not change runtime owner equality, grant physical identity or establish execution/SSA contents | `xtask-owner-resolution` |
| Original source Return effect envelope | `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-registry/src/world_effect.rs`; `rust/tcl-registry/src/state_transition.rs`; `rust/tcl-registry/src/commands/tcl/return_.rs`; `rust/tcl-compiler/src/command_binding/analysis_body_correspondence.rs` | `ResolvedInvocation::authored_source_effects_and_state_transitions`; `WorldEffectDescriptor::source_resolver`; `LogicalSourceStateTransitionResolver::CompletionState` | actual selected immutable invocation arguments and independently retained source dialect supply conditional effect/state facts; registered source resolvers retain unknown/expansion alternatives and legacy effect coverage, and Native source dependency consumers still require complete current body/card/input/vector correspondence; source CompletionState does not supply a Native name protocol, completed handler/effect or Normal result, and strict native Return effects keep their independent Native protocol requirement | `xtask-owner-resolution` |
| Original Authored scalar source grammar | `rust/tcl-compiler/src/registry_invocation/authored_controls.rs`; `rust/tcl-compiler/src/command_binding/declaration_layout.rs`; `rust/tcl-compiler/src/lowering/mod.rs`; `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs`; `rust/tcl-compiler/src/command_binding.rs` | `AuthoredSourceInvocation`; `original_authored_scalar_invocation`; `SourceInvocationBinding::original_authored_declaration_scalar_layout`; `SourceInvocationBinding::retain_original_authored_scalar_body_scope`; `SourceInvocationBinding::original_authored_scalar_body_scope` | actual Standalone declaration layout and held scalar source-body scope must agree on namespace/store, exact analysis attachment and unchanged whole vector; Lowerer keeps scalar tokens only after the complete scalar factory succeeds, with other role/control owners retaining their own terminal readers; complete selected Set/Return scalar grammar requires a Local source model without execution context, aliases or traces, exact descriptor/roles and no expansion; typed source operations retain independent represented SSA requirements and create no cell, version, Native handler or Normal result | `xtask-owner-resolution` |
| Original Native child syntax coordinate | `rust/tcl-compiler/src/command_binding/source_transition_advice/native_declared_body.rs`; `rust/tcl-compiler/src/word_subst.rs` | `SourceInvocationBinding::original_native_source_syntax_offset` | actual held declaration/tape, original source/full input/configuration, attachment and unchanged child vector own only a syntax offset; unavailable handler schema remains unknown, and syntax coordinates grant no selected command, frame, store, Native or Normal facts | `xtask-owner-resolution` |
| Configured original taint receiver names | `rust/tcl-compiler/src/taint.rs` | `unique_instance_class`; `original_scalar_reference_name` | callers retain actual original receiver word, lexer configuration and exact source point; shared complete scalar-reference grammar preserves literal empty/sigil names and refuses array references, compound/expanded/NUL references; only one class from the existing point candidate inventory is returned, missing or widened candidates abstain, and source root correspondence/standalone warnings cannot mint SSA/cell/version or mitigation authority without independent represented read ownership | `xtask-owner-resolution` |
| Configured substitution variable candidates | `rust/tcl-syntax/src/naming/substitution_candidates.rs`; `rust/tcl-compiler/src/taint.rs` | `SubstitutionVariableCandidates`; `substitution_variable_candidates`; `SubstitutionVariableCandidates::names`; `SubstitutionVariableCandidates::has_unknown_residual`; `SubstitutionVariableCandidates::may_contain`; `arg_var_names_with_config` | the shared configured executable arena retains possible variable roots through nested array indexes and command/Expr text in original order; malformed/depth/byte/NUL residuals cannot establish absence, taint readers use actual source configuration and widen known candidates when unavailable, and mitigation cannot close unknown coverage; candidates supply no definite read/value, cell, frame or Native result | `xtask-owner-resolution` |
| Standalone typed IR and source-word write coverage | `rust/tcl-compiler/src/ir_helpers.rs`; `rust/tcl-compiler/src/ssa.rs`; `rust/tcl-registry/src/registry.rs` | `standalone_typed_statement_writes_are_covered`; `variable_write_effects_from_commands`; `original_invocation_possible_name_writes`; `CommandRegistry::variable_write_projection_for_source_words` | explicit primitive/control typed-IR coverage requires all source/token/sidecar/admission evidence absent and refuses calls, raw expressions, math-function calls or substitutions; genuine source tokens remain terminal and instead require current selected descriptor/provider/effect/trace correspondence; independent source-word MAY effects and name candidates retain unknown residuals rather than establishing Native/Normal effects, physical cells, values or definite SSA versions | `xtask-owner-resolution` |
| Logical completed class/member source declarations | `rust/tcl-compiler/src/analyser/types/logical_members.rs`; `rust/tcl-compiler/src/analyser/oo.rs`; `rust/tcl-lsp-core/src/call_hierarchy.rs`; `rust/tcl-lsp-core/src/workspace_index.rs` | `LogicalSourceClassDeclaration`; `LogicalSourceMemberDeclaration`; `AnalysisResult::logical_source_class_declaration`; `LogicalSourceClassDeclaration::member`; `LogicalSourceClassDeclaration::capture`; `LogicalSourceMemberInsertion`; `AnalysisResult::logical_source_member_insertion`; `LogicalSourceMemberInsertion::name`; `LogicalSourceMemberInsertion::source_word` | the actual completed Logical class walk retains its genuine Realm source image/channel and full input; bounded class geometry and complete current ClassDef metadata must match, with only qualified UI label excluded and actual all_classes pointer required; exact side and unique MethodDef issue readonly member membership, and Core provider joins retain current URI/input/source correspondence; missing/stale/detached/Native owners refuse without creating original Native word/name ledgers, dispatch or editing authority; insertion additionally requires the selected current method pointer, genuine original name operand and exact static ASCII data under the retained grammar, then quotes one source word | `xtask-owner-resolution` |
| Original declaration source syntax memo | `rust/tcl-compiler/src/lowering/source_body_syntax.rs`; `rust/tcl-compiler/src/lowering/mod.rs`; `rust/tcl-compiler/src/compilation_unit.rs`; `rust/tcl-lsp-db/src/lib.rs` | `SourceBodySyntaxRequest`; `SourceBodySyntaxRequest::lower_neutral`; `SourceBodySyntaxTemplate`; `SourceBodySyntaxCache`; `Lowerer::with_source_body_syntax_cache`; `CompilationUnit::build_memoized_with_source_body_syntax_cache`; `SourceProcBodyKey`; `lower_proc_body_source` | a genuine current Native/Logical/Authored declaration card, original image/channel/body extent/namespace, input/entry/configuration/store and complete literal source words issue a pre-lowering syntax request; only selected no-hook generic calls without variable/body/frame/factory/transition dependencies qualify; neutral syntax contains no original token/runtime receipts, compares independently captured dependencies and exact shape, then restores the current request's original tokens/offset; unsupported requests lower fresh, with no computed output as key or physical body-cache authority | `xtask-owner-resolution` |
| Typed instance-variable write candidates | `rust/tcl-compiler/src/interprocedural.rs`; `rust/tcl-compiler/src/taint/instance_classes.rs`; `rust/tcl-compiler/src/ir_helpers.rs` | `structural_variable_write_projection` | typed literal write destinations preserve empty/sigil data and array-root partition, while an unknown destination conservatively includes all actual declared ivars or withdraws class state; array elements cannot publish scalar handle classes, and actual supplied source/input/Registry/invocation or explicit standalone purpose remains required; possible write/class metadata grants no Native cell, frame, Normal result or sanitization | `xtask-owner-resolution` |
| Named declaration source SSA ownership | `rust/tcl-compiler/src/command_binding/retained_source.rs`; `rust/tcl-compiler/src/command_binding/declaration_preview.rs`; `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/ssa.rs` | `RetainedSourceModuleBindings::owns_original_body_tokens`; `DeclaredProcedureBody::matches_original_procedure_source`; `LogicalStructuredInvocation::written_variable_name_operand` | exact current issued body/header/formals/source snapshot, full input/store, original attachment and unanimous declaration rows own the unchanged original vector; stronger Native/Logical child purposes remain terminal, and selected VarWrite data requires one genuine Written origin; read/word/definition selectors retain their independent current FU/CFG and represented Symbol/Version guards, with no new cell, version, Normal result or erasure permission | `xtask-owner-resolution` |
| Original analysis SSA lexical words | `rust/tcl-compiler/src/ssa.rs`; `rust/tcl-lsp-core/src/minify.rs` | `OriginalAnalysisSsaWords`; `OriginalAnalysisSsaWords::words`; `OriginalAnalysisSsaWords::config`; `original_analysis_words_for_statement` | exact current Module/function/keyed body/prepared CFG/full input and retained original tokens supply the complete document-coordinate lexical vector and actual configuration; present Native or Logical child purposes refuse terminally when changed; lexical geometry grants no represented read value, Normal result or erasure, and minifier independently requires replaceable-read plus the same current FunctionUnit/Symbol/SomeVersion and existing SCCP/taint checks | `xtask-owner-resolution` |
| Original analysis SSA definitions | `rust/tcl-compiler/src/ssa.rs`; `rust/tcl-lsp-core/src/inlay_hints.rs` | `OriginalAnalysisSsaDefinition`; `OriginalAnalysisSsaDefinition::function`; `OriginalAnalysisSsaDefinition::span`; `OriginalAnalysisSsaDefinition::symbol`; `OriginalAnalysisSsaDefinition::version`; `original_analysis_definition_at_unit`; `current_analysis_function`; `current_authored_analysis_function`; `original_authored_analysis_definition_at_unit` | the supplied-input path retains complete current Module/source/input/Registry/origin and actual function/keyed body/prepared CFG to select an exact original VarWrite operand, that point's symbol and definite represented SSA definition/version; present source-purpose refusal remains terminal, ambiguous or may-defined associations decline, and readonly Logical analysis definitions remain separate from Native physical definition receipts; a separate inputless path requires positively selected standalone entry, exact current Module/function/configuration/keyed Authored card/prepared CFG, actual card-owned scope ancestry and issuing header attachment, with no fabricated input or Native/Logical fallback; both paths require an existing definite SSA version, so generic bodies without represented definitions remain unavailable; inlay consumers also require the actual source-selected function pointer before reading that exact Symbol/Version type; neither a represented analysis version nor source geometry grants a physical cell, successful store, Normal result or editing permission | `xtask-owner-resolution` |
| Original declared source SSA definitions | `rust/tcl-compiler/src/ssa.rs`; `rust/tcl-compiler/src/command_binding/source_transition_advice/native_declared_body.rs`; `rust/tcl-compiler/src/registry_invocation/hosted_structure.rs`; `rust/tcl-lsp-core/src/inlay_hints.rs` | `OriginalSourceSsaDefinitionName`; `OriginalSourceSsaDefinitionName::span`; `OriginalSourceSsaDefinitionName::original_name_input`; `OriginalSourceSsaDefinitionName::body_id`; `OriginalSourceSsaDefinition`; `OriginalSourceSsaDefinition::function`; `OriginalSourceSsaDefinition::name`; `OriginalSourceSsaDefinition::symbol`; `OriginalSourceSsaDefinition::version`; `original_source_definition_at_unit`; `SsaSourceView::original_source_definition_for_body` | actual current Module/card/body/issued ID, exact FunctionUnit/SSA and full function metadata/configuration own a unique original VarWrite definition and its authored symbol/version; child source receipt and unqualified scalar name geometry must match, multiple or may-defined owners refuse; missing/stale/foreign inputs supply no definition; readonly source definition extent supplies no source-read selector, physical cell, Normal store, Native value or edit permission | `xtask-owner-resolution` |
| Original library navigation advice | `rust/tcl-lsp-core/src/definition.rs`; `rust/tcl-lsp-core/src/package_resolver.rs`; `rust/tcl-lsp-server/src/lib.rs` | `OriginalAutoloadRequest`; `OriginalAutoloadRequest::files`; `original_autoload_request_at` | complete current original C invocation head, name operand, caller input/configuration and retained binding select genuine auto_index files through the shared resolver; display labels cannot select a request or declaration; checked publication waits configuration/rehoming gates, revalidates closed source and actual open revisions/generations atomically with index writes; missing/stale/argument/unknown scope refuses, and source identities revalidate actual declarations before navigation; library advice supplies no successful autoload, installed Native command or callable grant | `xtask-owner-resolution` |
| MCP path source categories | `rust/tcl-mcp/src/irule_test.rs`; `rust/tcl-mcp/src/irule_gen.rs` | `PathSourceKind`; `PathSource` | actual diagram Event/Procedure category is retained separately from rendered labels through path traversal and test-generation data; only explicit event category selects event advice, missing/unknown category refuses; procedure-like event labels and event-like procedure labels cannot change category, and source advice supplies no F5 execution or Native event grant | `xtask-owner-resolution` |
| Per-function first-use representation commitments | `rust/tcl-compiler/src/shimmer/mod.rs`; `rust/tcl-compiler/src/shimmer/source_context.rs` | `first_use_commitments_for_function`; `first_use_value_commitments_for_function` | production advice uses actual FunctionUnit metadata/store and exact SSA ValueKey commit computation; the Symbol projection is test-only and requires agreement across represented versions; reporting labels cannot select production advice, missing/foreign metadata refuses, and commitments cannot donate a source-read selector, physical cell or another function's definition | `xtask-owner-resolution` |
| Exact native registration descriptor selection | `rust/tcl-registry/src/registry.rs`; `rust/tcl-registry/src/native_string_compilation.rs` | `CommandRegistry::native_registration_exact_descriptor` | shared exact authored registration key uses one-root transport, actual by_name visibility and exact descriptor identity label; original C84 string-member refinement reuses this same query, including rooted-only descriptors and excessive-root refusal; catalogue metadata supplies no live Native registration, admission or handler guarantee | `xtask-owner-resolution` |
| Logical callback candidate metadata | `rust/tcl-compiler/src/signature_scan/types.rs`; `rust/tcl-lsp-db/src/lib.rs` | `SignatureCommandInvocation::logical_callback_candidate_keys` | positively selected current Logical analysis callers retain the original compatibility projection; actual original callback/name producers keep their separate terminal lookup, resolved slots take priority, full candidates and rooted-qualified/bare assistance remain unchanged; display candidates supply no original lookup, Native callback or edit grant | `xtask-owner-resolution` |
| Current editor source inventory | `rust/tcl-lsp-core/src/original_context.rs`; `rust/tcl-lsp-core/src/document_symbols.rs`; `rust/tcl-lsp-core/src/implementation.rs`; `rust/tcl-lsp-core/src/rename.rs`; `rust/tcl-lsp-core/src/receiver_identity.rs`; `rust/tcl-lsp-core/src/references.rs` | `CurrentSourceContext::capture` | the actual retained Realm Document image, exact source bytes, configuration and complete input must match before cached document inventories or Logical scopes are read; rename preparation/execution, references/highlights, workspace method targeting and original method-cursor fast paths require that same full current editor owner before retained declaration metadata is projected; an unavailable rename execution owner is a terminal refusal rather than an empty edit fallback; shifted/missing/foreign owners refuse, and source currency grants no lookup, identity or editing permission | `xtask-owner-resolution` |
| Original variable completion extents | `rust/tcl-lsp-core/src/source_structure.rs`; `rust/tcl-lsp-core/src/completion.rs`; `rust/tcl-lsp-core/src/completion/original_variables.rs` | `source_variable_reference_end` | shared ExecutablePartArena under the actual original image and lexer configuration selects a unique variable component at the original start/cursor; the selected component owns brace/array closers according to that release, incomplete syntax retains the cursor fallback, and each completion branch independently requires its current original or Logical scope owner | `xtask-owner-resolution` |
| Unattributed dynamic alias hints | `rust/tcl-syntax/src/naming.rs`; `rust/tcl-compiler/src/var_escape/state.rs`; `rust/tcl-compiler/src/var_escape/cfg_propagation/state.rs` | `unattributed_variable_alias_hint` | an explicit standalone grammar parses one complete variable reference through whole_var_ref and declines array/compound references; a bounded assignment-value spelling is only a candidate, and both escape consumers retain unknown/all-known alternatives; present source owner refusal is terminal and cannot borrow this compatibility hint as a represented read, frame, cell or closed target | `xtask-owner-resolution` |
| SCCP phased write and destroy inventory | `rust/tcl-compiler/src/sccp/original_value_facts.rs`; `rust/tcl-compiler/src/registry_invocation.rs` | `scan_defined_and_unset`; `retain_source_call_name_coverage` | actual CFG/full-input correspondence and selected phased candidate roles retain literal name data, array-root coverage and each candidate's destruction effect; unresolved roles or source-only coverage preserve opaque write/destroy hazards; explicit standalone nominal metadata stays separate, and the inventory only protects existing existence folds without granting new fold eligibility or physical effects | `xtask-owner-resolution` |
| Shimmer scalar sharing destinations | `rust/tcl-compiler/src/shimmer/sharing.rs`; `rust/tcl-compiler/src/ir_helpers.rs` | `copy_destination_name`; `mutation_target` | shared structural name data supplies a scalar destination with array and opaque writes excluded; copy advice additionally requires the actual selected Normal first operand, positioned symbol/read/version and existing place/alias/current-metadata checks; mutation names remain exact selected literal data and cannot donate a different cell or represented value | `xtask-owner-resolution` |
| Original current variable reference extents | `rust/tcl-lsp-core/src/source_structure.rs`; `rust/tcl-lsp-core/src/minify.rs` | `OriginalVariableReferences`; `OriginalVariableReferences::capture`; `OriginalVariableReferences::reference_at` | exact current analysed source and retained configuration feed shared source structure and configured executable parts; reference geometry preserves the complete token, name and array-index extents; the minifier requires every selected reference to match that same source/name before accepting complete edits, with independent declaration, editing, read-version and erasure guards; geometry supplies no value, Native lookup or rewriting permission | `xtask-owner-resolution` |
| Constructed native command address bytes | `rust/tcl-syntax/src/naming/native.rs`; `rust/tcl-registry/src/invocation_words.rs`; `rust/tcl-compiler/src/codegen/cmd_subst.rs`; `rust/tcl-compiler/src/codegen/emitter/bytecoded.rs`; `runtime/rust/src/cmd_oo/native_context.rs` | `constructed_command_address_bytes`; `InvocationDialect::ensemble_implementation_command_address` | actual retained namespace components and selected command tail construct counted absolute input without reparsing reporting labels; Registry-owned private ensemble namespaces retain their existing engine/release selection, constructed key boundaries and literal counted tail bytes; codegen String/Dict/Namespace/Array worker inputs retain independent selected hook, original argument, entered-binding and dispatch guards, with public invocation fallback when the implementation namespace is unavailable; OO exact-before-unique selection checks the current command generation and ends the table borrow before constructing the replacement head; construction supplies no lookup success, original object header, command identity or Native admission, and reporting projections keep their separate purpose | `xtask-owner-resolution` |
| Original Jim interpreter string crossings | `rust/tcl-registry/src/special_vars.rs`; `rust/tcl-vm/src/interp/native_jim_interpreter_crossing.rs`; `rust/tcl-vm/src/command.rs`; `runtime/rust/src/interp/native_children.rs` | `NativeJimInterpreterCopyPurpose`; `Vm::copy_original_jim_value_to`; `Vm::capture_original_jim_parent_alias_prefix` | actual scoped source String materialisation and live destination constructor context copy counted object bytes or the independently selected child-startup CString extent; temporary destination headers remain unowned until real publication; parent-owned original alias prefix Lists are retained separately from neutral member snapshots, and child arguments/results cross by new destination Strings; actual source/destination currency and raw Jim completion remain independent of C return options, allocator identity or caller-name references | `xtask-owner-resolution` |
| Original compiler variable operands | `rust/tcl-compiler/src/codegen/values.rs`; `rust/tcl-compiler/src/codegen/emitter/bytecoded.rs`; `rust/tcl-syntax/src/native_variable_words.rs` | `CodegenCtx::original_variable_operand`; `NativeVariableWordOperand` | admitted original compiler words and one exact written operand supply selected version/protocol and literal root/index or executable compound index geometry; unset validates all operand projections before emitting sequential receiver effects, with actual local-slot/option/hook gates retained; captured or ambiguous operands decline to the existing generic invocation, and selected syntax grants no live variable cell, successful destruction or additional callback permission | `xtask-owner-resolution` |
| Original fixed scalar-formal alpha equivalence | `rust/tcl-compiler/src/analyser/types.rs`; `rust/tcl-compiler/src/command_binding/scalar_body_alpha.rs`; `rust/tcl-lsp-core/src/minify/original.rs` | `AnalysisResult::original_scalar_body_alpha_rename`; `AnalysisResult::original_fixed_scalar_body_alpha_rename`; `OriginalScalarBodyAlphaRename`; `ScalarAlphaFormalSelection`; `AnalysisResult::original_scalar_alpha_for_formals` | exact current closed original declaration/body and complete required fixed-scalar formal topology select the same-frame symbol and collision-safe native tail under the quiet whole-object return and closed observer contract; singleton selection preserves the old multi-formal refusal, fixed selection retains every actual formal/name-field obligation; minifier additionally requires complete original rename edits and source/config/Registry correspondence; local variables, multiple commands/procedures, defaults/rest/caller links, reflection and alias insertion remain unavailable | `xtask-owner-resolution` |
| Original positioned method-call syntax | `rust/tcl-lsp-core/src/source_structure.rs`; `rust/tcl-lsp-core/src/definition.rs`; `rust/tcl-lsp-core/src/hover.rs`; `rust/tcl-lsp-core/src/references.rs`; `rust/tcl-lsp-core/src/rename.rs` | `original_method_site_at`; `original_instance_method_at_cursor` | complete current analysed source/input/configuration and selected executable body structure choose the innermost complete original command, unchanged head and static selector words; expansion, computed selectors, physical comments, non-selector positions and malformed tails refuse; sole variable/command-substitution receiver syntax stays separate from decoded literal command data; definition, hover, references and rename retain independent actual receiver, dispatch, visibility, provider and editing owners, with no Native or Normal authority from this syntax projection | `xtask-owner-resolution` |
| Original literal-local store alpha equivalence | `rust/tcl-compiler/src/command_binding/scalar_body_alpha.rs`; `rust/tcl-compiler/src/analyser/types.rs`; `rust/tcl-lsp-core/src/minify/original.rs` | `OriginalLocalStoreAlphaRename`; `OriginalLocalStoreAlphaRename::selected_symbol`; `OriginalLocalStoreAlphaRename::new_native_tail`; `OriginalLocalStoreAlphaRename::declaration_site`; `OriginalLocalStoreAlphaRename::matches_source`; `OriginalLocalStoreAlphaRename::matches_registry`; `AnalysisResult::original_local_store_alpha_rename` | separate zero-formal literal scalar store followed by whole-object ReturnResult equivalence; actual current declaration/publication/frame/Registry and complete alias/global/observer/trace receiver inventories own one scalar replacement; minifier additionally requires complete original symbol and edit coverage; fixed-formal and singleton owners retain their own eligibility, and this receipt issues no Normal/store/native opcode or reached-call grant | `xtask-owner-resolution` |
| Original Logical class-body syntax | `rust/tcl-compiler/src/ir/definition_script.rs`; `rust/tcl-compiler/src/analyser/types/logical_members.rs`; `rust/tcl-compiler/src/analyser/diagnostics.rs`; `rust/tcl-lsp-core/src/source_structure.rs` | `Module::original_class_definition_body_source`; `AnalysisResult::retain_logical_class_body_sources`; `LogicalSourceClassDeclaration::original_body_word` | actual issued definition-script/current Module descriptor and grammar select original name/body geometry; the completed current Logical class snapshot retains that genuine body word under exact image/input correspondence; Core source structure consumes syntax only, without class allocation, member execution, editing or report-identity permission | `xtask-owner-resolution` |
| Package scaffold namespace construction | `rust/tcl-syntax/src/naming/scaffold.rs`; `rust/xtask/src/gen_editor_catalogs.rs`; `editors/vscode/src/nameGeneration.ts`; `editors/vscode/src/scaffold.ts` | `GeneratedScaffoldNamespace`; `ScaffoldNamespacePolicy`; `PACKAGE_SCAFFOLD_NAMESPACE_POLICY`; `PACKAGE_SCAFFOLD_NAMESPACE_CASES`; `scaffold_namespace_from_package_name` | explicit generated ASCII namespace construction from UTF-16 input units; shared policy and cross-language cases drive the editor projection, preserving unsupported-unit replacement, empty-segment removal and fallback spelling; the lossy result permits collisions and supplies no existing Tcl identity, lookup, creation, uniqueness, Native or frame permission | `xtask-gen-editor-catalogs` |
| Original iRules variable presentation | `rust/tcl-syntax/src/naming.rs`; `rust/tcl-irules/src/source_context.rs` | `var_reference_for_style` | independently authenticated whole original variable-token spans and retained complete source configuration supply presentation labels under the selected braced-reference close rule; array/index source text stays intact and literal sigils remain data; no selected cell, evaluated index/name, read, namespace lookup or Native grant follows from a label | `xtask-owner-resolution` |
| Original rendered result properties | `rust/tcl-compiler/src/rendered_properties.rs`; `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/compilation_unit.rs` | `propagate_rendered_props_with_config`; `NormalRepresentationInvocation::rendered_value_traits` | the actual FunctionUnit lexer configuration and retained CFG complete metadata owner must agree with the unanimous original statement vector, recorded command extent and source-point input; successful normal handler metadata alone supplies path/unescape/normalised hints; original effective assignment words retain alias correspondence, and nested commands require one actual whole command word under inherited original bindings/configuration; missing, changed, unavailable or compound source refuses to conservative properties without a standalone fallback; explicit standalone catalogue analysis remains separate; these annotations grant no invocation, Native capability, completion or rewrite | `xtask-owner-resolution` |
| native deferred Jim namespace and invocation context | `runtime/rust/src/namespace.rs`; `runtime/rust/src/cmd_proc/native_jim_apply.rs`; `runtime/rust/src/interp/native_jim_namespace.rs`; `runtime/rust/src/interp/native_jim_lookup.rs`; `runtime/rust/src/interp/native_compilation.rs`; `runtime/rust/src/interp/native_name_queries.rs`; `runtime/rust/src/interp.rs` | `JimNamespaceName`; `Namespaces::retain_unmaterialized_jim_namespace`; `Namespaces::record_jim_namespace_materialized`; `Namespaces::remember_jim_namespace_owner`; `Interp::materialize_jim_namespace`; `Interp::jim_canonical_namespace_object`; `OriginalJimLookupNamespace`; `CallMeta::original_frame_arguments`; `Interp::original_alias_prefix_value_at` | actual independently allocated original Jim object context, explicit absent name bytes, reached guarded getter and physical frame birth; weak actual procedure/frame roles preserve native decrements; same-pointer command-cache hits skip getters, different pointers retain original cached/current getter order; frame preparation retains the actual apply dummy head and original argument headers without obtaining lambda or argument Strings; original alias-prefix lookup validates the actual issuer and delegates to the original lookup owner without requiring pre-existing namespace bytes; same-pointer cache hits may remain stringless, while cold lookup reaches its own guarded getter and MissingCommand/NotAlias stay distinct; direct canonicalisation acquires the original name first, selects an absolute fresh CString suffix without obtaining namespace bytes, and reaches the guarded actual namespace getter only for relative construction; original operation currency, physical frame and allocation liveness remain independent checks; namespace display follows the actual retained Jim object or C tree kind across policy changes, requires reached resident bytes for a nonGLOBAL Jim object, and supplies no current operation or namespace coordinate; the permanent real GLOBAL C root remains independent; no C coordinate from Jim-only context or unsupported policy, no duplicate naming recipe | `xtask-owner-resolution` |
| Original failed-operand update syntax | `rust/tcl-compiler/src/registry_invocation/source_operand_evaluation.rs`; `rust/tcl-compiler/src/lowering/mod.rs`; `rust/tcl-compiler/src/lowering_hooks.rs` | `OriginalOperandEvaluationUpdate`; `original_operand_evaluation_update`; `try_lower_operand_evaluation_update_with_binding` | authentic before-handler operand-failure receipt plus exact Execution source purpose/full input and closed original descriptor/direct written layout; static original VarWrite names and absent expression/body/frame/deferred-script grammar select only an attempted scalar-update operand plan; all original tokens/head/namespace/guard remain attached and generic fallback remains unchanged; the existing handler/body rejection stays terminal, and the syntax receipt establishes no completed argv, successful handler, entered frame, Native value, committed store or executable-erasure authority | `xtask-owner-resolution` |
| Original refactor command geometry | `rust/tcl-lsp-core/src/refactor/mod.rs`; `rust/tcl-lsp-core/src/refactor/extract_proc.rs`; `rust/tcl-lsp-core/src/refactor/source_command.rs`; `rust/tcl-lsp-core/src/refactor/source_rewrite.rs`; `rust/tcl-lsp-core/src/refactor/inline_proc.rs`; `rust/tcl-lsp-core/src/refactor/extract_variable.rs`; `rust/tcl-lsp-core/src/refactor/inline_variable.rs`; `rust/tcl-lsp-core/src/refactor/brace_expr.rs`; `rust/tcl-lsp-core/src/refactor/if_to_switch.rs`; `rust/tcl-lsp-core/src/refactor/switch_to_dict.rs`; `rust/tcl-lsp-core/src/refactor/datagroup.rs` | `configured_command_span_offsets`; `configured_command_words`; `body_words`; `FrameWalk::command_span_offsets`; `FrameWalk::native_words`; `FrameWalk::variable_references` | unchanged complete image and retained full configuration; existing original whole-word spans supply complete command endpoints including variable closers; extraction selection, role positions, block/tail boundaries and insertion anchors, original command selection and rewrite traversal, inline call ranges, value-word extraction, variable-inline value/reference ranges, expression operand edits, and selected if-to-switch/switch-to-dictionary replacement endpoints share this geometry; current-analysis data-group extraction passes the same checked endpoint into rendering without a legacy geometry fallback, while explicit standalone convenience parsing independently reborrows configured original whole-word geometry without manufacturing an AnalysisResult or borrowing its edit guards; explicit standalone command walks reborrow the configured original braced word and shared content span, and lambda descent uses that same original word's list grammar and braced body content; source offsets and representative tokens cannot donate body delimiters; already-decoded case and clause script values enter their selected configured parser directly, without peeling literal command-name braces or trimming data a second time; selected literal Set/Return proposal arm values also use the same configured whole-word vector, preserving complete variable, compound, quoted and empty source words without token-closer guessing; their actual original-operation and edit guards remain independent, and the legacy token endpoint helper is test-only; individual variable components and their unique unchanged containing word select reference spans; an array-index child cannot borrow scalar-splice permission without frame, capture or edit permission | `xtask-owner-resolution` |
| Original caller-frame scalar read geometry | `rust/tcl-lsp-core/src/refactor/mod.rs`; `rust/tcl-lsp-core/src/caller_frame.rs`; `rust/tcl-syntax/src/naming.rs`; `rust/tcl-lexer/src/word_parts.rs` | `FrameWalk::components`; `whole_scalar_variable_reference_name`; `RawVarRef::name_range_in` | actual current source/input/configuration and selected Logical caller source frame bound the executable-command walk; original selected word/expression components own escaping and inert data, the shared whole-scalar scanner excludes array roots and differently delimited names, and the borrowed original name range supplies interior reference geometry; other source frames and unavailable components cannot donate reads; Native caller templates and original read owners remain independent, and no entered frame, cell, value or edit permission follows | `xtask-owner-resolution` |
| Original source handle inventories | `rust/tcl-lsp-core/src/executable_regions.rs`; `rust/tcl-lsp-core/src/semantic_tokens.rs` | `visit_analysis_executable_commands` | complete retained analysis/image/configuration and independently selected potential body roles; callbacks return true only to stop, while handle collectors continue with false through later roots and child regions; each collected loop/constructor/member keeps its own selected source roles, factory/body vocabulary and original whole words; no executed binding, entered frame, current receiver, stored value or Native grant | `xtask-owner-resolution` |
| Original expression child source catalogue | `rust/tcl-compiler/src/command_binding/source_transition_advice/body_effects.rs`; `rust/tcl-compiler/src/command_binding/source_transition_advice.rs`; `rust/tcl-compiler/src/realm/source_transition_advice.rs`; `rust/tcl-compiler/src/registry_invocation/source_structure.rs` | `OriginalSourceExpressionCommand`; `OriginalSourceCatalogueBody`; `AdviceInvocationContext::retain_original_expression_catalogue` | exact retained parent source schema, full input/configuration/Registry and written effective Expr operand select checked expression command extents; current-interpreter Plain/no-frame/nondeferred layouts retain only immutable child catalogue receipts in a discarded branch, with original source vector/namespace/target/barrier checks unchanged; inert expression literals and missing or replaced parent/child descriptors cannot donate a child; this source owner supplies no script Body role, reached lookup, handler/frame, effects, Native value, normal completion or edit authority | `xtask-owner-resolution` |
| Original Logical definer name geometry | `rust/tcl-registry/src/definer.rs`; `rust/tcl-lsp-core/src/semantic_tokens/original.rs` | `DefinitionBodyGrammar::command_installation` | actual selected definition schema installation name ordinal, independently of generic Name roles; existing original head/input, whole written operand and literal-source value correspondence govern Logical class-body context and source class-name decoration; dynamic/captured geometry refuses and no created class, current object or Native method dispatch follows | `xtask-owner-resolution` |
| Expression function command labels | `rust/tcl-registry/src/mathfunc.rs`; `rust/tcl-syntax/src/naming.rs`; `rust/tcl-compiler/src/codegen/expressions.rs` | `mathfunc::qualified_name`; `unroot_rooted_key` | the shared authored math-function namespace and unchanged literal function tail construct the label; removing exactly one constructed root marker retains the caller-relative command-table input, while fixed-table/unknown native dispatch still follows its existing independent gate; label construction grants no original function identifier, lookup success, registration, execution, arity or Native authority | `xtask-owner-resolution` |
| Native source warning candidates | `rust/tcl-compiler/src/registry_invocation/source_taint.rs`; `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/taint.rs` | `NativeSourceTaintInvocation`; `native_source_taint_invocation` | current retained Module and issued Native declaration card retain full actual input/configuration/Registry and unchanged original command words; direct written nonexpanded accepted-arity schemas supply conditional warning candidates independently of reached, Logical and Hosted invocation purposes; existing positioned read-occurrence ValueKeys may supply taint, while missing reads or values retain unknown tainted MAY and no symbol/version is invented; conditional option topology uses the same retained command/effective vector/current argv and Registry with zero argument offset, through the actual full-input source schema and shared authored possible-option grammar; existing read-occurrence taint, diagnostic spans and fix ownership remain independent; absent, foreign, dynamic, captured-prefix or expanded ownership refuses without a weaker source fallback; no Normal handler, completed argv, sanitisation, frame, cell or execution authority | `xtask-owner-resolution` |
| Expanded list scalar source words | `rust/tcl-lexer/src/native_script_words.rs`; `rust/tcl-syntax/src/naming.rs`; `rust/tcl-compiler/src/codegen/cmd_subst.rs`; `rust/tcl-compiler/src/codegen/values.rs` | `native_script_words_in_with_purpose`; `whole_scalar_variable_reference_name`; `CodegenCtx::try_list_expand_concat`; `CodegenCtx::emit_variable_reference` | complete selected LexerConfig and one original nested command supply the exact head, two written expansion operands and whole scalar references; malformed, compound, array and non-expanded words decline before emission; original reference syntax uses the existing operand emitter, while whole-unit trust, selected List hook and retained command binding remain independent; lexical shape grants no source attachment, native cell, read contents or successful invocation | `xtask-owner-resolution` |
| Original Jim selected command and tailcall transport | `runtime/rust/src/interp/native_jim_tailcall.rs`; `runtime/rust/src/interp/native_jim_lookup.rs`; `runtime/rust/src/interp.rs`; `runtime/rust/src/frame.rs`; `runtime/rust/src/namespace.rs`; `runtime/rust/src/namespace/jim_local.rs`; `runtime/rust/src/interp/jim_teardown.rs` | `JimCommandReference`; `JimCommandReferenceData`; `JimSelectedTailcall`; `PendingTailcall`; `Interp::capture_jim_command_reference`; `Interp::retained_jim_command_reference`; `Interp::retain_active_jim_command`; `Interp::schedule_jim_tailcall`; `Interp::invoke_selected_jim_tailcall`; `Interp::invoke_bound_with_jim_reference` | actual selected node, hidden slot or still-owned same physical binding supplies one born Jim command reference; an allocation-only weak index supplies no callable or interpreter lifetime; shared active/pending transport transfers the same native reference without another ledger acquisition; reached lookup retains the original namespace/cache owner and exact header, then the original List owns untouched arguments through issuer cleanup; invocation uses the selected allocation without another spelling/FQN/namespace lookup and preserves its raw completion; uninvoked pending retirement releases selected command before List, while a consumed evaluation releases List before selected command; both orders run under saved original operation cleanup, retain the first refusal, and retire only captured previous edges independently of ambient policy; frame and deferred-request teardown settle those real owners before command tables, with no strong interpreter cycle; existing C namespace-prefixed List transport remains separate | `xtask-owner-resolution` |
| Authored simulator mock command addresses | `rust/tcl-registry/src/simulator_mock_name.rs`; `rust/xtask/src/gen_irule_test_data.rs`; `rust/tcl-irule-test/src/pure_functions.rs` | `SimulatorMockName::for_label`; `SimulatorMockName::command_address` | the existing authored ASCII label recipe retains suffix/category/action and explicit collisions; its immutable publication namespace and literal suffix construct the complete counted command address through the shared Syntax owner; generated exact-label tables and pure-function registration consume that same address, while authored suffixes remain separate decision-log data; construction grants no registration presence, F5 binding, actual lookup, current worker, frame or Native authority | `xtask-owner-resolution` |
| Configured path rewrite terms | `rust/tcl-compiler/src/path_concat.rs`; `rust/tcl-syntax/src/naming.rs` | `build_file_join_fix_with_config`; `whole_scalar_variable_reference_name` | the W201 consumer supplies its existing complete LexerConfig; the shared scanner recognises whole scalar source words before the separate conservative path-rewrite spelling restrictions; standalone C8.6 fixtures remain separate, and proposed replacement text supplies no selected command binding, physical variable, successful evaluation or edit permission | `xtask-owner-resolution` |
| Original object ownership count observation | `rust/tcl-vm/src/value.rs`; `rust/tcl-irule-test/src/live/original_projection_tests.rs`; `rust/tcl-irule-test/src/pure_functions/original_projection_tests.rs` | `Value::native_object_reference_count` | borrowed observation of the actual modelled ownership reference count subtracts lifetime-only bridge pins from the existing strong count; the query acquires no clone, getter, object-liveness, invocation or Native authority, and test observations remain independent of real provider header counts | `xtask-owner-resolution` |
| Original Jim command reset and unknown header | `runtime/rust/src/interp.rs`; `runtime/rust/src/native_source.rs`; `runtime/rust/src/interp/native_script.rs`; `runtime/rust/src/interp/native_jim_tailcall.rs` | `NativeJimObjectContext::unknown_object`; `Interp::dispatch_prebound_inner`; `Interp::dispatch_missing_command`; `Interp::prepare_missing_command` | actual Jim lookup issuer selects the common command entry without C's pre-lookup result reset; the independently retained Script owner still resets at source entry, while vector/pure-List command lookup and the actual selected command reference precede worker reset; an actual miss publishes its diagnostic and checks original operation, physical frame and caller namespace before later unknown lookup; the checked live interpreter Unknown role supplies its same original header, allocation-only lookup acquires no native reference, and entered argv owns the original header through invocation; no fresh String/list parsing substitutes that role or hides withdrawal; C retains its own result/handler-prefix stages; the separately scoped resident retiring-result getter does not cover stringless updaters or reentrant result mutation; unknown recursion/frame/error-stack and Jim trace producers retain independent obligations | `xtask-owner-resolution` |
| Original completion argument separators | `rust/tcl-lsp-core/src/original_oo/instance_completion.rs`; `rust/tcl-lsp-core/src/original_oo.rs`; `rust/tcl-lsp-core/src/completion/original_context.rs` | `completion_gap` | actual complete LexerConfig and original head-end/cursor select intra-command separators and document-channel backslash continuations; class and method selector completion and actual Registry/vendor trailing argument slots share the instance-completion grammar, preserving empty head-boundary handling and existing current input, class, original selector and captured-operand guards; newlines, command separators and comments cannot donate another selector position; syntax supplies no receiver, dispatch or edit authority | `xtask-owner-resolution` |
| Original NativeLowering descriptor queries | `rust/tcl-registry/src/registry.rs`; `rust/tcl-compiler/src/native_lowering/original_invocations.rs`; `rust/tcl-compiler/src/native_lowering/lower.rs` | `CommandRegistry::native_lowering_for_descriptor`; `OriginalInvocations::for_module`; `OriginalInvocations::command`; `OriginalInvocations::substitution` | exact independently selected descriptor metadata and supplied availability windows; unchanged Module bytes/full input/configuration/Registry, actual analysis attachment and original source vectors bind direct Invoke words/binding/span and lexical argument child carriers; full original source invocation facts are independently re-resolved and direct facts must agree completely; absent or changed supplied source stays generic, while strictly source-absent authored compatibility requires positive Standalone metadata and known entry history; existing Native compiler admission, point dispatch/mutation/trace, original argv and read/store guards remain independent; no lookup, successful handler, body/frame/effect or Native value follows from source inventory or descriptor metadata | `xtask-owner-resolution` |
| Original method caller-frame source correspondence | `rust/tcl-compiler/src/command_binding/retained_source.rs`; `rust/tcl-compiler/src/cfg_builder/upvar_info.rs`; `rust/tcl-compiler/src/optimiser/method_barrier.rs`; `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/cfg_builder/global_write_info.rs` | `RetainedSourceModuleBindings::matches_original_method_summary`; `method_reaches_caller_frame` | actual Lowerer retention keeps Module-issued method header/kind/formals/execution namespace, complete immutable original Script and actual receiver/formal contexts for comparison, excluding class/member reporting labels; current issuer, Registry, positioned source owner and unchanged method correspondence precede an empty-body answer or namespace-aware shared source-frame collection; missing RuntimeSelected source namespace stays opaque; explicit Standalone structural analysis requires actual complete original Script words/configuration and current argv correspondence, with selected Registry frame layout over literal/dynamic/expanded operands; shared whole-scalar syntax and actual formal membership replace private sigil/brace parsing, and stale or missing original geometry stays opaque without a supplied-source-to-Standalone fallback; retained signatures supply no method frame, receiver, namespace donor or Native execution authority; actual SourceNamespaceKey values pass from the issued body namespace through recursive source collection and conditional writes, including nested SourceContext bodies; constructed list lookup uses NamespaceKeyQuery on the retained key without inverting advisory text | `xtask-owner-resolution` |
| Original Logical callback source candidates | `rust/tcl-compiler/src/analyser/item_tree.rs`; `rust/tcl-lsp-db/src/lib.rs`; `rust/tcl-lsp-db/src/project_source_inputs.rs`; `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-compiler/src/command_binding/source_transition_advice/logical_navigation.rs`; `rust/tcl-compiler/src/registry_invocation/source_structure.rs`; `rust/tcl-compiler/src/command_binding/source_transition_advice/logical_body.rs`; `rust/tcl-compiler/src/command_binding/source_transition_advice/body_effects.rs` | `ItemSig`; `ResolvedInvocation::authored_logical_source_command_prefix_arguments`; `OriginalLogicalCallbackCandidates`; `OriginalLogicalCallbackCandidates::registration`; `OriginalLogicalCallbackCandidates::head`; `OriginalLogicalCallbackCandidates::coordinates`; `OriginalLogicalCallbackCandidates::appended_arity`; `OriginalLogicalCallbackCandidates::baked_argument_count`; `OriginalLogicalCallbackCandidates::matches_source`; `source_logical_callback_candidates_at`; `source_logical_callback_candidates`; `LogicalNamespaceScope::body_scope` | selected original installer and complete actual Logical input/schema retain exact original whole words, shared original list fields and callback descriptor topology; explicit Document or genuine Declared source scope and descriptor GlobalFrame select conditional ordered coordinates, while unavailable method scope refuses; ordinary namespace source bodies retain the selected namespace and genuine parent ancestry through the same body-scope constructor; registration applicability, original source/input/configuration and selected body containment remain independent; body-free ItemSig headers retain independently checked Logical source coordinates, while the DB typed per-slot source arity query preserves ordered first occupied cutoff including empty class/nonprocedure entries; actual current supplied-file configuration and complete command-like header coverage remain required, earlier unknown mutations withdraw new callback arity advice, and report-tail fallback or Native publication cannot supply a missing coordinate; no empty result proves a complete callback census or absence, and no callback invocation, installed command, method frame, Native recipe or edit grant follows; exact original head offset selects the already retained immutable whole-vector tape under matching full input, image, tape map key and scope; source suffix reparsing across nested closers cannot substitute another command, and this coordinate grants no frame, inventory absence or lookup authority | `xtask-owner-resolution` |
| Original Jim retiring resident-result observation | `runtime/rust/src/interp.rs`; `runtime/rust/src/interp/native_jim_result.rs`; `runtime/rust/src/obj.rs`; `runtime/rust/src/value_ops.rs` | `RetiringResultHeader`; `RetiringResultHeader::capture`; `RetiringResultHeader::resident_bytes`; `JimResultRetirement`; `JimResultRetirement::enter`; `RetiringJimResult`; `Interp::retiring_jim_result_bytes`; `Interp::native_string_primary_admission` | actual selected Jim result replacement keeps the prior slot until its genuine release returns; a scoped weak-interpreter/currency receipt, unchanged original primary-admission result, exact original header incarnation and primary descriptor, allocation-only pin, native refcount zero and active free phase permit the interpreter's existing String getter to copy only its same current retiring result's resident bytes; generic retired-header liveness still refuses, original context stays retired, and the receipt grants no native reference, updater, conversion, mutation, resurrection or foreign/post-phase access; changed currency retains the first Host cause; C keeps its independent publish-before-release order | `xtask-owner-resolution` |
| Original autoload index filename data | `rust/tcl-lsp-core/src/package_resolver/original_names.rs`; `rust/tcl-lsp-core/src/package_resolver/original_names/index_loader.rs`; `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-registry/src/source_navigation.rs`; `rust/tcl-registry/src/source_file.rs`; `rust/tcl-registry/src/source_path.rs`; `rust/tcl-syntax/src/word_rules.rs`; `rust/tcl-cmd-core/src/path.rs`; `rust/tcl-lsp-core/src/package_resolver.rs` | `IndexLoader::new`; `IndexLoader::filename`; `ResolvedInvocation::authored_source_list_arguments`; `authored_source_navigation_operand`; `source_file_grammar`; `authored_source_path_operation`; `original_static_word_unicode_value`; `original_word_unicode_value_with_variables`; `path::join`; `parse_pkg_index`; `parse_tcl_index`; `IndexLoader::standalone`; `IndexLoader::collect_word` | unchanged complete original index source/input/configuration/Registry and actual Set schema retain the original auto_index key; literal loader data owns only nested script-value grammar, and actual list producers require authentic retained child schema and conditional list-argument ordinals; future Source metadata remains conditional original-value advice, never reached lookup/body/frame/Native argv or effect proof; Registry Source operand grammar and typed path operation select exact filename data without trimming spaces, quotes or sigils; shared whole-word value projection receives explicit directory-text advice rather than an observed dir value; missing, stale, expanded, unknown, captured-prefix or replaced original producer refuses, and real source-path effective arguments require identity written correspondence; existing C autoload naming policy and filesystem/directory provenance remain independent, without private helper whitelist, Native/load success or Jim autoload grant; positively selected Standalone source-data parsing remains independent of the actual analysis-backed input and never substitutes for missing supplied ownership; complete actual NativeWord source/configuration, shared list/key/value/path geometry and conditional Encoding compatibility layout preserve counted data and conventional directory-text advice; filesystem existence/listing and original key observations remain separate from reached autoload or Native engine permission; conditional Registry vocabulary, member and option selection uses the shared static Unicode word projection with its validated Literal facet; dynamic or expanded operands stay unknown, without a native-byte object, reached Source/file command or execution grant | `xtask-owner-resolution` |
| Original authored corpus scalar evidence | `rust/tcl-spec-studio/src/corpus.rs`; `rust/tcl-compiler/src/ir.rs`; `rust/tcl-syntax/src/naming.rs`; `rust/tcl-spec-studio/src/infer.rs` | `Stmt::scalar_read`; `statements`; `WordExpr::sole_variable_substitution`; `CommandTokens::from_segmented`; `whole_scalar_variable_reference_name`; `original_body_data`; `scan_in` | actual authored body and supplied environment configuration retain each genuine segmented original word; a sole Source substitution, exact source span and shared whole-scalar grammar select literal parameter data; braced or escaped data, compound, array, expanded and opaque operands decline; parameter membership and advisory corpus arity/callback inference remain independent of actual current argv, lookup, Native execution, frame and cell authority; an actual imported procedure declaration and Registry-authored layout select genuine Written Name/Body data through the shared script-body extent and static word Unicode value, preserving literal closers and sigils; corpus advice retains the same Registry and full nested configuration; supplied owner refusal supplies no body advice, while standalone scan remains an explicit authoring ingress and Module-issued summary facts remain independent | `xtask-owner-resolution` |
| Logical implicit command namespace paths | `rust/tcl-compiler/src/analyser/scope.rs`; `rust/tcl-lsp-core/src/references.rs` | `implicit_command_namespace_path_at` | the actual innermost analysed scope selects Registry-authored implicit command path metadata; readonly compatibility reference candidates compare its exact one-root namespace keys instead of copying a namespace label; original positioned bindings, current-source entry checks, shadowing and edit eligibility remain independent, with no Runtime namespace, entered frame or Native lookup grant | `xtask-owner-resolution` |
| Explicit Standalone source authoring ingress | `rust/tcl-compiler/src/lowering/mod.rs`; `rust/tcl-compiler/src/compilation_unit.rs` | `Lowerer::for_standalone_source` | only source-entry, supplied context and supplied input all absent select this authoring constructor; the actual Registry, full configuration and optional profile retain pure authoring dialect recipes without acquiring an availability ContextRegistry; supplied or driver ingress keeps its own purpose, including missing or unknown metadata, and cannot reopen this branch; no certificate, Native source entry, entered frame or executed declaration follows; the explicit isolated procedure-body authoring entry uses the same constructor with its actual Registry, complete configuration and optional profile; this preserves its positive Standalone purpose without supplying executed-source or Native compilation admission | `xtask-owner-resolution` |
| Original Jim existence queries | `rust/tcl-registry/src/commands/tcl/exists_.rs`; `runtime/rust/src/cmd_jim_exists.rs`; `runtime/rust/src/interp/native_jim_lookup.rs`; `runtime/rust/src/interp/native_jim_links.rs` | `NativeJimExistsKind`; `NativeJimExistsProtocol::table`; `NativeJimExistsProtocol::selected`; `NativeJimExistsProtocol::usage`; `InvocationDialect::native_jim_exists_protocol`; `install`; `exists_original`; `Interp::exists_original_jim_variable`; `Interp::read_original_named_variable_reported`; `OriginalJimVariableReport` | actual selected Jim backend installs the registered core worker; original argument ownership and same-object enum protocol retain option identity before existing original command lookup or silent original variable-object read; actual Proc/Alias record kind supplies only that flag, and missing or proved nonchannel records remain distinct from an unavailable existing-command channel flag; silent mode reuses actual VarVal, alias/link and cached-object ownership without a byte-name fallback, preserving the ordinary missing-error read mode; host refusal remains terminal, and handler presence or pinned source-invocation observations establish no complete channel, option, trace or Runtime parity | `xtask-owner-resolution` |
| Original declaration body content | `rust/tcl-lsp-core/src/declaration.rs`; `rust/tcl-lsp-core/src/refactor/mod.rs`; `rust/tcl-lexer/src/native_word.rs` | `original_declaration_body_content`; `FrameWalk::native_words`; `NativeWord::content_span` | actual current source/input/configuration and executable traversal select the exact retained scope-body word before its shared content extent is segmented; independently issued lambda content is not unwrapped a second time; selected declaration grammar, visible scope, original name spans and related variable identity remain required, with no reached body, method namespace, Native frame, physical alias or edit grant; explicit standalone grammar controls remain test-only | `xtask-owner-resolution` |
| Original variable reference tail proposals | `rust/tcl-syntax/src/naming/variable_spelling.rs`; `rust/tcl-syntax/src/naming.rs`; `rust/tcl-lsp-core/src/rename.rs`; `rust/tcl-lexer/src/word_parts.rs` | `variable_source_reference_with_tail`; `RawVarRef::name_range_in`; `variable_reference_root_bytes`; `written_command_tail` | the complete original reference and actual LexerConfig select the borrowed literal name extent and written tail; a shared exact splice preserves qualifier, wrappers and literal or computed index bytes, then the complete proposal must reproduce the intended name, root and original index shape under that same grammar; unrepresentable qualification or braced-name proposals decline; current source/component selection, safe replacement, variable identity, collisions and edit eligibility remain independent, with no Native recipe, frame, cell, read or edit grant | `xtask-owner-resolution` |
| Original Logical method procedure candidates | `rust/tcl-lsp-core/src/call_hierarchy.rs`; `rust/tcl-compiler/src/registry_invocation/source_structure.rs`; `rust/tcl-compiler/src/analyser/types/logical_procedures.rs`; `rust/tcl-compiler/src/analyser/types/logical_members.rs` | `logical_method_procedure_at`; `source_logical_command_candidates_at`; `source_logical_absolute_command_candidate_at`; `LogicalSourceProcedureDeclaration::source_candidate_coordinate`; `LogicalSourceClassDeclaration::source_candidate_coordinate` | actual current Logical source/input/configuration and each original head retain an independently issued ordered namespace recipe or root-independent absolute body-syntax coordinate; current original target headers select conditional source candidates, preserving duplicate declaration occurrences and first occupied class cutoff; class reporting labels supply no method namespace, unavailable relative method scope stays unresolved, and earlier unknown mutations retain unresolved alternatives even for a unique local source candidate; unrepresented alias/ensemble occupancy and actual lookup remain unknown, without a complete command census, installed call, method frame, Native or edit grant | `xtask-owner-resolution` |
| Original absolute Logical source command coordinate | `rust/tcl-compiler/src/command_binding/source_transition_advice/logical_navigation.rs`; `rust/tcl-compiler/src/registry_invocation/source_structure.rs`; `rust/tcl-compiler/src/realm/source_transition_advice.rs`; `rust/tcl-compiler/src/command_binding/source_transition_advice/body_effects.rs` | `source_logical_absolute_command_candidate_at`; `OriginalLogicalAbsoluteCommandCandidate::coordinates`; `OriginalLogicalAbsoluteCommandCandidate::matches_source`; `OriginalLogicalAbsoluteCommandCandidate::has_earlier_unknown_mutations`; `OriginalLogicalAbsoluteCommandCandidate::has_lookup_operand_effects` | actual pre-handler Script/Member/Expression body syntax traversal retains complete original words/current Logical input/image/config/body bounds and literal rooted coordinate; immutable exact-head tape map and realm currentness recheck applicability; root-independent syntax needs no relative method namespace and grants no installed lookup, binding, frame, dispatch or absence; unknown mutation alternatives remain explicit | `xtask-owner-resolution` |
| Original Jim namespace scripted helpers | `rust/tcl-registry/src/native_jim_lookup.rs`; `runtime/rust/src/interp/stock_ensembles.rs`; `runtime/rust/src/cmd_namespace.rs` | `NativeJimNamespaceHelperKind`; `NativeJimNamespaceHelperKind::ALL`; `NativeJimNamespaceHelper`; `NativeJimNamespaceHelper::kind`; `NativeJimNamespaceHelper::accepts_public_arguments`; `NativeJimNamespaceHelper::usage`; `NativeJimNamespaceHelper::name`; `NativeJimNamespaceHelper::parameters`; `NativeJimNamespaceHelper::body`; `InvocationDialect::native_jim_namespace_helper`; `Interp::install_stock_jim_namespace_helpers`; `Interp::install_stock_jim_namespace_helper`; `Interp::dispatch_stock_jim_namespace_helper` | immutable pinned Which/Code/Origin declarations and exact public Jim_ParseSubCmd arity supply constructor input without callable, physical slot, compiler identity or admission; the receiving issuer owns genuine global installation or preservation of the actual replacement and generation; public adapters perform original arity checks before fresh original helper heads and untouched operands enter ordinary lookup; direct helper frame, original argument identity, cache and getter branches remain independent, snapshots gain no getter, and other scripted helpers remain separate; Origin uses the same selected source descriptor, exact public arity and genuine physical helper publication; the original helper owns canonical, Exists, Info alias and selected tailcall operations, with no manual byte-name or alias loop replacing it and no full alias-cycle or reentrant lifecycle claim | `xtask-owner-resolution` |
| Decoded refactor script indentation | `rust/tcl-lsp-core/src/refactor/mod.rs`; `rust/tcl-lsp-core/src/refactor/if_to_switch.rs`; `rust/tcl-lsp-core/src/refactor/datagroup.rs` | `reindent_body` | actual selected clause/case container values are already decoded script data; indentation preserves their literal command-name braces and performs no second source-word unwrap; original source, Registry, operation and edit eligibility remain independent, and the explicit written-word fixture first obtains the shared original NativeWord content span | `xtask-owner-resolution` |
| Original method hierarchy presentation | `rust/tcl-lsp-core/src/original_declaration.rs`; `rust/tcl-lsp-core/src/original_call_hierarchy.rs` | `OriginalDeclarationIdentity::reporting_key` | independently current readonly identity and actual original class/member names supply qualified method display; unavailable class presentation retains the short label, while identity, geometry and call edges stay independently selected; no reporting key can supply a caller namespace, binding, frame or edit grant | `xtask-owner-resolution` |
| Original caller-template literal name data | `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/analyser/param_traits.rs` | `InvocationBodyAssistance::caller_template_literal_data` | the selected invocation must be the actual retained caller-template projection and equal its held original invocation/tokens; Written operands use original literal data, while actual captured-prefix values retain their separate literal-byte origin; dynamic, expanded, detached, changed or unavailable correspondence declines; parameter traits and alias templates retain their independently selected argument/formal/role and local-scalar policy checks, without source-reference interpretation, actual caller cell, reached alias, completed write, execution or Native authority | `xtask-owner-resolution` |
| Original procedure report labels | `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/interprocedural.rs`; `rust/tcl-compiler/src/registry_invocation/authored_source_bodies.rs` | `Module::original_procedure_report_label` | actual Module-issued body ID, unique original procedure and current positioned retained Module/Registry source correspondence precede report projection; an Authored card retains its checked original root Name spelling, while equal report labels keep separate body identities; reporting selects no callable, publication, namespace, entered frame or Native entry | `xtask-owner-resolution` |
| Original Logical body-word geometry | `rust/tcl-compiler/src/command_binding/source_transition_advice/logical_word_geometry.rs`; `rust/tcl-compiler/src/command_binding/source_transition_advice/body_effects.rs`; `rust/tcl-compiler/src/command_binding/source_transition_advice.rs` | `OriginalLogicalBodyWords`; `OriginalLogicalBodyWords::capture`; `OriginalSourceTransitionAdviceTape::logical_body_original_word_at_span` | actual Script, Member or Expression body syntax retains complete original NativeWords before dynamic head or Registry selection; original origin allocation, full Logical input/context/configuration, whole vector and body bounds authenticate immutable word geometry; exact tape merging preserves ambiguity and the span query independently requires selected-advice unanimity; geometry supplies no schema, handler, lookup result, namespace, frame or Native authority | `xtask-owner-resolution` |
| Conditional original source semantic metadata | `rust/tcl-compiler/src/semantic_analysis.rs`; `rust/tcl-compiler/src/semantic_analysis/source_metadata.rs`; `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs` | `SemanticAnalysisBundle::conditional_source_metadata`; `ConditionalSourceMetadata::source_input`; `ConditionalSourceMetadata::invocations`; `ConditionalSourceInvocationMetadata::descriptor_key`; `ConditionalSourceInvocationMetadata::span`; `ConditionalSourceInvocationMetadata::roles`; `ConditionalSourceInvocationMetadata::roles_complete`; `ConditionalSourceInvocationMetadata::direct_written_arguments`; `ConditionalSourceInvocationMetadata::has_unknown_residual` | actual full supplied input/store/configuration, authentic retained Script carrier, recorded command span and unchanged original whole words retain each conditional Registry schema independently; existing stronger purposes are terminal, while separately authenticated unresolved-point vocabulary always retains an unknown residual; effective role coordinates and direct Written correspondence remain distinct; partial inventories prove no census, absence, reached handler, Normal result, frame, Native value or executable rewrite, and source transport independently requires its actual certificate/token/span map while preserving complete facts and residual equality | `xtask-owner-resolution` |
<!-- end-owner-resolution-manifest -->

### Internal owner entries

The five-column manifest requires public declarations or an exact checked source
registration below.
`signature-type` checks an explicit parameter/return type in the current function
declaration only; it supplies no value, receiver or body coverage. Signature
macros, aliases, type/const generics and conditional declarations remain refused. Private entries retain their actual visibility and owning
crate/module/declaration/consumer. Runtime receiver registration remains
independently checked through the following internal table.
The separate table supports only explicit `crate` scope in direct `lib.rs` file
modules; missing or unknown scopes and unsupported module routes fail closed.
The gate checks the consumer's declared method and direct `self.<getter>()`
receiver through `std::cell::Ref<'_, T>`, with one unaliased import of the owning
module's actual `T`. Generic methods, aliases, globs and ambiguous receivers do
not establish a route. Contributing declarations, imports and local bindings
must be unconditional: only documentation attributes and struct/enum derives
are supported on these nodes. Enclosing file/crate attributes must preserve
all declarations; lint settings remain neutral even inside `cfg_attr`.
Other attributes, including availability-changing `cfg`/`cfg_attr`, withdraw
the route. This does not widen any API's visibility.

<!-- owner-resolution-internal-manifest -->
| Surface | Owner source | Crate | Owner module | Entry | Scope | Consumer source | Consumer module | Consumer | Receiver getter |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Runtime lookup-policy revision | `runtime/rust/src/namespace.rs` | `tcl-runtime` | `crate::namespace` | `Namespaces::guard_lookup_environment_revision` | crate | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::synchronise_command_environment_guards` | `Interp::namespaces` |
<!-- end-owner-resolution-internal-manifest -->

### Source owner declarations and consumers

Source registration checks the actual crate, nested file-module route, exact
declaration and visibility against its named consumer. Qualified static and
type references, typed parameters, same-implementation self calls and bounded
constructor/field/iterator receivers retain their original AST routes. Exact import and re-export aliases follow their checked canonical paths. Missing,
ambiguous, inaccessible, conditional, shadowed or macro-derived routes and
unresolved aliases supply no coverage. Explicit passive declaration attributes retain the
source route; availability and path-changing attributes refuse. Generic owners, producers and receiver routes retain their strict type/constant-generic refusal; lifetime-only types retain their real owner. A static call in a consumer with unrelated plain bounded type parameters may resolve an independent concrete owner, with generic identifiers shadowing nominal paths. Actual local module declarations take precedence over unrelated module globs; explicit same-name imports, duplicate or conditional declarations, imported-only glob identities and block-local glob ambiguity remain refused. Plain local constants may contain only checked literal data, including exact no-argument, non-generic byte-string-literal `.as_slice()` coercion; their initializers supply no owner call or receiver proof. Constant names shadow the whole block before uses, and nested block identities restore their enclosing scope. Conditional, macro, callable or unsupported initializers remain refused. This check does not widen private APIs or issue a
receiver, Native frame, handler, runtime lookup or completed-effect permission.
The Runtime internal typed getter table and its restrictions stay independent.

<!-- owner-resolution-source-manifest -->
| Surface | Owner source | Crate | Owner module | Entry | Visibility | Consumer source | Consumer module | Consumer entry | Route |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Source-owned analysis cache correspondence | `rust/tcl-compiler/src/command_binding/retained_source.rs` | `tcl-compiler` | `crate::command_binding::retained_source` | `RetainedSourceModuleBindings::matches_original_procedure_summary` | pub(crate) | `rust/tcl-compiler/src/source_graph.rs` | `crate::source_graph` | `procedure_body_for_original_call` | `parameter-field:module:retained_source_bindings` |
| Configured original taint receiver names | `rust/tcl-compiler/src/taint.rs` | `tcl-compiler` | `crate::taint` | `unique_instance_class` | pub(crate) | `rust/tcl-compiler/src/taint.rs` | `crate::taint` | `instance_taint_source_colour` | `static` |
| Configured original taint receiver names | `rust/tcl-compiler/src/taint.rs` | `tcl-compiler` | `crate::taint` | `original_scalar_reference_name` | private | `rust/tcl-compiler/src/taint.rs` | `crate::taint` | `unique_instance_class` | `static` |
| Original Authored scalar source grammar | `rust/tcl-compiler/src/command_binding.rs` | `tcl-compiler` | `crate::command_binding` | `SourceInvocationBinding::original_authored_scalar_body_scope` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/authored_controls.rs` | `crate::registry_invocation::authored_controls` | `original_authored_scalar_invocation` | `parameter-field:tokens:source_binding` |
| Original definition-script source coordinates | `rust/tcl-compiler/src/ir/definition_script.rs` | `tcl-compiler` | `crate::ir::definition_script` | `OriginalDefinitionScriptSource` | pub | `rust/tcl-compiler/src/ir/definition_script.rs` | `crate::ir::definition_script` | `Module::original_definition_script_source` | `type` |
| Original definition-script source coordinates | `rust/tcl-compiler/src/ir/definition_script.rs` | `tcl-compiler` | `crate::ir::definition_script` | `OriginalDefinitionScriptSource::capture` | private | `rust/tcl-compiler/src/ir/definition_script.rs` | `crate::ir::definition_script` | `Module::retain_definition_script_source` | `static` |
| Original definition-script source coordinates | `rust/tcl-compiler/src/ir/definition_script.rs` | `tcl-compiler` | `crate::ir::definition_script` | `Module::retain_definition_script_source` | pub(crate) | `rust/tcl-compiler/src/lowering/mod.rs` | `crate::lowering` | `Lowerer::extract_definer_members` | `field:self.module` |
| Logical object-fact report correspondence | `rust/tcl-compiler/src/object_types/report_comparison.rs` | `tcl-compiler` | `crate::object_types::report_comparison` | `ObjectHandleFacts::retain_report_source` | pub(crate) | `rust/tcl-compiler/src/analyser/diagnostics.rs` | `crate::analyser::diagnostics` | `Analyser::produce_object_handle_facts` | `producer:crate::object_types::object_handle_facts_for_analysis` |
| Logical per-object member insertion and masks | `rust/tcl-compiler/src/analyser/types/logical_object_members.rs` | `tcl-compiler` | `crate::analyser::types::logical_object_members` | `LogicalSourceObjectMembers` | pub | `rust/tcl-compiler/src/analyser/types/logical_object_members.rs` | `crate::analyser::types::logical_object_members` | `AnalysisResult::logical_source_object_member_state` | `type` |
| Logical per-object member insertion and masks | `rust/tcl-compiler/src/analyser/types/logical_object_members.rs` | `tcl-compiler` | `crate::analyser::types::logical_object_members` | `LogicalSourceObjectMemberState::capture` | pub(crate) | `rust/tcl-compiler/src/analyser/handlers.rs` | `crate::analyser::handlers` | `Analyser::fold_objdefine_block` | `static` |
| Original Authored scalar source grammar | `rust/tcl-compiler/src/registry_invocation/authored_controls.rs` | `tcl-compiler` | `crate::registry_invocation::authored_controls` | `AuthoredSourceInvocation` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/authored_controls.rs` | `crate::registry_invocation::authored_controls` | `original_authored_scalar_invocation` | `type` |
| Original Authored scalar source grammar | `rust/tcl-compiler/src/registry_invocation/authored_controls.rs` | `tcl-compiler` | `crate::registry_invocation::authored_controls` | `original_authored_scalar_invocation` | pub(crate) | `rust/tcl-compiler/src/lowering/mod.rs` | `crate::lowering` | `Lowerer::logical_structured_invocation` | `static` |
| Original Authored scalar source grammar | `rust/tcl-compiler/src/command_binding/declaration_layout.rs` | `tcl-compiler` | `crate::command_binding::declaration_layout` | `SourceInvocationBinding::original_authored_declaration_scalar_layout` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/authored_controls.rs` | `crate::registry_invocation::authored_controls` | `original_authored_scalar_invocation` | `parameter-field:tokens:source_binding` |
| Original Native child syntax coordinate | `rust/tcl-compiler/src/command_binding/source_transition_advice/native_declared_body.rs` | `tcl-compiler` | `crate::command_binding::source_transition_advice::native_declared_body` | `SourceInvocationBinding::original_native_source_syntax_offset` | pub(crate) | `rust/tcl-compiler/src/word_subst.rs` | `crate::word_subst` | `original_call_context` | `parameter-field:tokens:source_binding` |
| Configured substitution variable candidates | `rust/tcl-compiler/src/taint.rs` | `tcl-compiler` | `crate::taint` | `arg_var_names_with_config` | private | `rust/tcl-compiler/src/taint.rs` | `crate::taint` | `ordered_argument_candidates` | `static` |
| Standalone typed IR and source-word write coverage | `rust/tcl-compiler/src/ir_helpers.rs` | `tcl-compiler` | `crate::ir_helpers` | `standalone_typed_statement_writes_are_covered` | pub(crate) | `rust/tcl-compiler/src/ssa.rs` | `crate::ssa` | `nested_writes` | `static` |
| Standalone typed IR and source-word write coverage | `rust/tcl-compiler/src/ir_helpers.rs` | `tcl-compiler` | `crate::ir_helpers` | `variable_write_effects_from_commands` | pub(crate) | `rust/tcl-compiler/src/ir_helpers.rs` | `crate::ir_helpers` | `defs_from_expr` | `static` |
| Standalone typed IR and source-word write coverage | `rust/tcl-compiler/src/ir_helpers.rs` | `tcl-compiler` | `crate::ir_helpers` | `original_invocation_possible_name_writes` | pub(crate) | `rust/tcl-compiler/src/ir_helpers.rs` | `crate::ir_helpers` | `original_statement_possible_name_writes` | `static` |
| Logical generated property completion | `rust/tcl-lsp-core/src/completion.rs` | `tcl-lsp-core` | `crate::completion` | `generated_property_method_items` | private | `rust/tcl-lsp-core/src/completion.rs` | `crate::completion` | `method_items` | `static` |
| Logical generated property completion | `rust/tcl-lsp-core/src/completion.rs` | `tcl-lsp-core` | `crate::completion` | `source_method_name_word` | private | `rust/tcl-lsp-core/src/completion.rs` | `crate::completion` | `generated_property_method_items` | `static` |
| Named declaration source SSA ownership | `rust/tcl-compiler/src/command_binding/retained_source.rs` | `tcl-compiler` | `crate::command_binding::retained_source` | `RetainedSourceModuleBindings::owns_original_body_tokens` | pub(crate) | `rust/tcl-compiler/src/ssa.rs` | `crate::ssa` | `original_source_read_at_unit` | `parameter-field:unit:ir_module.retained_source_bindings` |
| Named declaration source SSA ownership | `rust/tcl-compiler/src/command_binding/declaration_preview.rs` | `tcl-compiler` | `crate::command_binding::declaration_preview` | `DeclaredProcedureBody::matches_original_procedure_source` | pub(super) | `rust/tcl-compiler/src/command_binding/retained_source.rs` | `crate::command_binding::retained_source` | `RetainedSourceModuleBindings::owns_original_body_tokens` | `producer-field:crate::command_binding::declaration_layout::original_declaration_layouts:entry:variant:DeclaredProcedure:0` |
| Named declaration source SSA ownership | `rust/tcl-compiler/src/registry_invocation.rs` | `tcl-compiler` | `crate::registry_invocation` | `LogicalStructuredInvocation::written_variable_name_operand` | pub(crate) | `rust/tcl-compiler/src/ssa.rs` | `crate::ssa` | `original_analysis_definition_at_unit` | `producer:crate::registry_invocation::logical_structured_invocation_with_metadata_context` |
| Original conditional word contents | `rust/tcl-compiler/src/command_binding/logical_word_values.rs` | `tcl-compiler` | `crate::command_binding::logical_word_values` | `LogicalOriginalWordValue::capture_invocation` | pub(super) | `rust/tcl-compiler/src/command_binding/logical_definition.rs` | `crate::command_binding::logical_definition` | `LogicalProcedureDefinition::capture` | `static` |
| Original conditional word contents | `rust/tcl-compiler/src/command_binding/logical_word_values.rs` | `tcl-compiler` | `crate::command_binding::logical_word_values` | `LogicalOriginalWordValue::is_current` | pub(super) | `rust/tcl-compiler/src/command_binding/logical_definition.rs` | `crate::command_binding::logical_definition` | `LogicalProcedureDefinition::apply` | `field:self.values` |
| Original conditional word contents | `rust/tcl-compiler/src/command_binding/logical_word_values.rs` | `tcl-compiler` | `crate::command_binding::logical_word_values` | `LogicalOriginalWordValue::value` | pub(super) | `rust/tcl-compiler/src/command_binding/logical_definition.rs` | `crate::command_binding::logical_definition` | `LogicalProcedureDefinition::capture` | `producer:crate::command_binding::logical_word_values::LogicalOriginalWordValue::capture_invocation` |
| Original conditional word contents | `rust/tcl-compiler/src/command_binding/frozen_arguments.rs` | `tcl-compiler` | `crate::command_binding::frozen_arguments` | `SourceInvocationBinding::original_authored_written_argument_value` | pub(crate) | `rust/tcl-compiler/src/lowering/mod.rs` | `crate::lowering` | `Lowerer::original_authored_written_argument_value` | `producer-field:Lowerer::cmd_tokens:source_binding` |
| Original procedure summary ownership | `rust/tcl-compiler/src/ir.rs` | `tcl-compiler` | `crate::ir` | `Module::original_source_procedure_bodies` | pub(crate) | `rust/tcl-compiler/src/cfg_builder/global_write_info.rs` | `crate::cfg_builder::global_write_info` | `detect_global_write_bodies_with_registry` | `parameter:module` |
| Original procedure summary ownership | `rust/tcl-compiler/src/ir.rs` | `tcl-compiler` | `crate::ir` | `Module::original_source_procedure_for_target` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs` | `crate::registry_invocation::original_procedure_call` | `original_source_procedure_ids_for_word_for_module` | `parameter:module` |
| Original Logical source frame projection | `rust/tcl-compiler/src/registry_invocation/logical_frame.rs` | `tcl-compiler` | `crate::registry_invocation::logical_frame` | `OriginalLogicalFrameInvocation` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/logical_frame.rs` | `crate::registry_invocation::logical_frame` | `original_logical_frame_invocation_with_metadata_context` | `type` |
| Original Logical source frame projection | `rust/tcl-compiler/src/registry_invocation/logical_frame.rs` | `tcl-compiler` | `crate::registry_invocation::logical_frame` | `original_logical_frame_invocation_with_metadata_context` | pub(crate) | `rust/tcl-compiler/src/command_binding/original_body_namespace.rs` | `crate::command_binding::original_body_namespace` | `SourceInvocationBinding::original_logical_frame_body_namespace` | `static` |
| Original Logical source frame projection | `rust/tcl-compiler/src/registry_invocation/logical_frame.rs` | `tcl-compiler` | `crate::registry_invocation::logical_frame` | `OriginalLogicalFrameInvocation::source_body` | pub(crate) | `rust/tcl-compiler/src/cfg_builder/upvar_info.rs` | `crate::cfg_builder::upvar_info` | `record_logical_frame_script` | `parameter:invocation` |
| Original Logical source frame projection | `rust/tcl-compiler/src/command_binding/original_body_namespace.rs` | `tcl-compiler` | `crate::command_binding::original_body_namespace` | `SourceInvocationBinding::original_logical_frame_body_namespace` | pub(crate) | `rust/tcl-compiler/src/command_binding/original_body_namespace.rs` | `crate::command_binding::original_body_namespace` | `original_logical_frame_body_namespace_for_module` | `parameter-field:parent:source_binding` |
| Original Logical source frame projection | `rust/tcl-compiler/src/lowering/mod.rs` | `tcl-compiler` | `crate::lowering` | `Lowerer::original_logical_body_for_module` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/logical_frame.rs` | `crate::registry_invocation::logical_frame` | `OriginalLogicalFrameInvocation::source_body` | `static` |
| Original Logical source frame projection | `rust/tcl-compiler/src/registry_invocation/logical_frame.rs` | `tcl-compiler` | `crate::registry_invocation::logical_frame` | `original_logical_body_geometry_for_module` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/logical_frame.rs` | `crate::registry_invocation::logical_frame` | `OriginalLogicalFrameInvocation::unknown_caller_variable_candidates` | `static` |
| Original Logical source frame projection | `rust/tcl-compiler/src/registry_invocation/logical_frame.rs` | `tcl-compiler` | `crate::registry_invocation::logical_frame` | `OriginalLogicalFrameInvocation::unknown_caller_variable_candidates` | pub(crate) | `rust/tcl-compiler/src/cfg_builder/upvar_info.rs` | `crate::cfg_builder::upvar_info` | `walk_stmt` | `producer:crate::registry_invocation::logical_frame::original_logical_frame_invocation_with_metadata_context` |
| Original conditional case source geometry | `rust/tcl-compiler/src/registry_invocation/source_structure.rs` | `tcl-compiler` | `crate::registry_invocation::source_structure` | `OriginalSourceCase` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/source_structure.rs` | `crate::registry_invocation::source_structure` | `original_logical_source_case_for_tokens` | `type` |
| Original conditional case source geometry | `rust/tcl-compiler/src/registry_invocation/source_structure.rs` | `tcl-compiler` | `crate::registry_invocation::source_structure` | `original_logical_source_case_for_tokens` | pub(crate) | `rust/tcl-compiler/src/analyser/oo.rs` | `crate::analyser::oo` | `walk_unknown_stmt` | `static` |
| Original conditional case source geometry | `rust/tcl-compiler/src/registry_invocation/source_structure.rs` | `tcl-compiler` | `crate::registry_invocation::source_structure` | `original_native_source_case_for_statement` | pub(crate) | `rust/tcl-compiler/src/analyser/oo.rs` | `crate::analyser::oo` | `walk_unknown_stmt` | `static` |
| conditional literal body name ownership | `rust/tcl-compiler/src/script_binds.rs` | `tcl-compiler` | `crate::script_binds` | `original_literal_body_ownership` | pub(crate) | `rust/tcl-compiler/src/ssa.rs` | `crate::ssa` | `braced_word_owns_name` | `static` |
| original Logical procedure completion and no-error source calls | `rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs` | `tcl-compiler` | `crate::registry_invocation::original_procedure_call` | `original_logical_procedure_calls_for_module` | pub(crate) | `rust/tcl-compiler/src/source_graph.rs` | `crate::source_graph` | `procedure_body_for_original_call` | `static` |
| original Logical procedure completion and no-error source calls | `rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs` | `tcl-compiler` | `crate::registry_invocation::original_procedure_call` | `original_procedure_formal_count_shape` | pub(crate) | `rust/tcl-compiler/src/interprocedural/completion.rs` | `crate::interprocedural::completion` | `procedures_complete` | `static` |
| original Logical procedure completion and no-error source calls | `rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs` | `tcl-compiler` | `crate::registry_invocation::original_procedure_call` | `original_procedure_scalar_bindings` | pub(crate) | `rust/tcl-compiler/src/interprocedural/completion.rs` | `crate::interprocedural::completion` | `procedures_complete` | `static` |
| original Logical procedure completion and no-error source calls | `rust/tcl-compiler/src/interprocedural/completion.rs` | `tcl-compiler` | `crate::interprocedural::completion` | `procedures_complete` | pub(super) | `rust/tcl-compiler/src/interprocedural.rs` | `crate::interprocedural` | `build_interprocedural_analysis_inner` | `static` |
| original inlining source and alpha-rename geometry | `rust/tcl-compiler/src/inlining/frame.rs` | `tcl-compiler` | `crate::inlining::frame` | `SourceContext::for_module` | pub(super) | `rust/tcl-compiler/src/inlining/mod.rs` | `crate::inlining` | `build_inlinable_map` | `static` |
| current source refactor and CLI reduction metadata | `rust/tcl-lsp-core/src/refactor/extract_variable.rs` | `tcl-lsp-core` | `crate::refactor::extract_variable` | `original_assignment_candidate` | private | `rust/tcl-lsp-core/src/refactor/extract_variable.rs` | `crate::refactor::extract_variable` | `extract_variable` | `static` |
| current source refactor and CLI reduction metadata | `rust/tcl-lsp-core/src/refactor/source_rewrite.rs` | `tcl-lsp-core` | `crate::refactor::source_rewrite` | `select` | pub(super) | `rust/tcl-lsp-core/src/refactor/switch_to_dict.rs` | `crate::refactor::switch_to_dict` | `switch_to_dict` | `static` |
| original selected Core dispatch and namespace hazards | `rust/tcl-lsp-core/src/namespace_rename.rs` | `tcl-lsp-core` | `crate::namespace_rename` | `namespace_rename_hazard` | private | `rust/tcl-lsp-core/src/namespace_rename.rs` | `crate::namespace_rename` | `lexical_namespace_rename_edits` | `static` |
| current structural diagram source metadata | `rust/tcl-diagram/src/data.rs` | `tcl-diagram` | `crate::data` | `registry_command` | private | `rust/tcl-diagram/src/data.rs` | `crate::data` | `walk_call` | `static` |
| current structural diagram source metadata | `rust/tcl-diagram/src/data.rs` | `tcl-diagram` | `crate::data` | `command_completion` | private | `rust/tcl-diagram/src/data.rs` | `crate::data` | `walk_call` | `static` |
| original substitution purity metadata | `rust/tcl-compiler/src/registry_invocation.rs` | `tcl-compiler` | `crate::registry_invocation` | `original_substitution_purity_with_metadata_context` | pub(crate) | `rust/tcl-compiler/src/optimiser/elimination/source_purity.rs` | `crate::optimiser::elimination::source_purity` | `substitutions_are_pure` | `static` |
| original substitution purity metadata | `rust/tcl-compiler/src/optimiser/elimination/source_purity.rs` | `tcl-compiler` | `crate::optimiser::elimination::source_purity` | `substitutions_are_pure` | pub(super) | `rust/tcl-compiler/src/optimiser/elimination.rs` | `crate::optimiser::elimination` | `word_has_observable_side_effect` | `static` |
| Original Logical source effect and frame carriers | `rust/tcl-compiler/src/command_binding.rs` | `tcl-compiler` | `crate::command_binding` | `SourceInvocationBinding::has_original_source_layout_owner` | pub(crate) | `rust/tcl-compiler/src/command_binding.rs` | `crate::command_binding` | `SourceInvocationBinding::original_logical_source_words` | `self` |
| Original Logical source effect and frame carriers | `rust/tcl-compiler/src/command_binding.rs` | `tcl-compiler` | `crate::command_binding` | `SourceInvocationBinding::retain_original_logical_source_advice` | pub(crate) | `rust/tcl-compiler/src/realm.rs` | `crate::realm` | `CommandBindingRealm::stamp_original_tokens` | `parameter-field:tokens:source_binding` |
| Original Logical source effect and frame carriers | `rust/tcl-compiler/src/command_binding.rs` | `tcl-compiler` | `crate::command_binding` | `SourceInvocationBinding::original_logical_source_words` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/source_structure.rs` | `crate::registry_invocation::source_structure` | `original_logical_registry_words_for_module` | `parameter-field:tokens:source_binding` |
| Original Logical source effect and frame carriers | `rust/tcl-compiler/src/registry_invocation/source_structure.rs` | `tcl-compiler` | `crate::registry_invocation::source_structure` | `original_logical_registry_words_for_module` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/source_structure.rs` | `crate::registry_invocation::source_structure` | `original_immediate_body_commands_for_module` | `static` |
| Original Logical source effect and frame carriers | `rust/tcl-compiler/src/registry_invocation/logical_frame.rs` | `tcl-compiler` | `crate::registry_invocation::logical_frame` | `original_logical_frame_invocation_for_module` | pub(crate) | `rust/tcl-compiler/src/cfg_builder/upvar_info.rs` | `crate::cfg_builder::upvar_info` | `record_original_child_frame_effects` | `static` |
| Original Logical source frame projection | `rust/tcl-compiler/src/command_binding/original_body_namespace.rs` | `tcl-compiler` | `crate::command_binding::original_body_namespace` | `original_logical_frame_body_namespace_for_module` | pub(crate) | `rust/tcl-compiler/src/lowering/mod.rs` | `crate::lowering` | `Lowerer::original_logical_body_for_module` | `static` |
| Original procedure MAY dependencies | `rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs` | `tcl-compiler` | `crate::registry_invocation::original_procedure_call` | `OriginalProcedureMayDependencies` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs` | `crate::registry_invocation::original_procedure_call` | `original_procedure_may_dependencies_for_word_for_module` | `type` |
| Original procedure MAY dependencies | `rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs` | `tcl-compiler` | `crate::registry_invocation::original_procedure_call` | `original_procedure_may_dependencies_for_word_for_module` | pub(crate) | `rust/tcl-compiler/src/taint_interproc.rs` | `crate::taint_interproc` | `command_subst_callees` | `static` |
| Original var-escape substitution inventory | `rust/tcl-compiler/src/var_escape/helpers.rs` | `tcl-compiler` | `crate::var_escape::helpers` | `OriginalSubstitutionInvocation` | pub(crate) | `rust/tcl-compiler/src/var_escape/helpers.rs` | `crate::var_escape::helpers` | `original_substitution_invocations` | `type` |
| Original var-escape substitution inventory | `rust/tcl-compiler/src/var_escape/helpers.rs` | `tcl-compiler` | `crate::var_escape::helpers` | `original_substitution_invocations` | pub(crate) | `rust/tcl-compiler/src/var_escape/walker.rs` | `crate::var_escape::walker` | `apply_original_substitution_scan` | `static` |
| Original var-escape substitution inventory | `rust/tcl-compiler/src/var_escape/helpers.rs` | `tcl-compiler` | `crate::var_escape::helpers` | `original_substitution_invocations` | pub(crate) | `rust/tcl-compiler/src/var_escape/cfg_propagation/walker.rs` | `crate::var_escape::cfg_propagation::walker` | `apply_original_substitution_scan` | `static` |
| Original Hosted control-flow source grammar | `rust/tcl-compiler/src/registry_invocation/hosted_structure.rs` | `tcl-compiler` | `crate::registry_invocation::hosted_structure` | `SourceStructuredInvocation` | pub(crate) | `rust/tcl-compiler/src/lowering_hooks.rs` | `crate::lowering_hooks` | `try_lower_logical_hook_with_binding` | `type` |
| Original Hosted control-flow source grammar | `rust/tcl-compiler/src/registry_invocation/hosted_structure.rs` | `tcl-compiler` | `crate::registry_invocation::hosted_structure` | `original_hosted_structured_invocation` | pub(crate) | `rust/tcl-compiler/src/lowering/mod.rs` | `crate::lowering` | `Lowerer::logical_structured_invocation` | `static` |
| Original Hosted control-flow source grammar | `rust/tcl-compiler/src/registry_invocation/source_taint.rs` | `tcl-compiler` | `crate::registry_invocation::source_taint` | `original_hosted_source_registry_metadata` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/hosted_structure.rs` | `crate::registry_invocation::hosted_structure` | `original_hosted_structured_invocation` | `static` |
| Original Native declaration child source grammar | `rust/tcl-compiler/src/command_binding/source_transition_advice/native_declared_body.rs` | `tcl-compiler` | `crate::command_binding::source_transition_advice::native_declared_body` | `OriginalNativeSourceBodyChildren` | pub(crate) | `rust/tcl-compiler/src/command_binding/generated_procedures.rs` | `crate::command_binding::generated_procedures` | `OriginalNativeProcedureBody::source_children` | `type` |
| Original Native declaration child source grammar | `rust/tcl-compiler/src/registry_invocation/hosted_structure.rs` | `tcl-compiler` | `crate::registry_invocation::hosted_structure` | `original_native_source_structured_invocation` | pub(crate) | `rust/tcl-compiler/src/lowering/mod.rs` | `crate::lowering` | `Lowerer::logical_structured_invocation` | `static` |
| MCP path source categories | `rust/tcl-mcp/src/irule_test.rs` | `tcl-mcp` | `crate::irule_test` | `PathSourceKind` | private | `rust/tcl-mcp/src/irule_test.rs` | `crate::irule_test` | `walk_flow` | `type` |
| MCP path source categories | `rust/tcl-mcp/src/irule_test.rs` | `tcl-mcp` | `crate::irule_test` | `PathSource` | private | `rust/tcl-mcp/src/irule_test.rs` | `crate::irule_test` | `build_path` | `type` |
| Exact native registration descriptor selection | `rust/tcl-registry/src/registry.rs` | `tcl-registry` | `crate::registry` | `CommandRegistry::native_registration_exact_descriptor` | pub(crate) | `rust/tcl-registry/src/registry.rs` | `crate::registry` | `CommandRegistry::native_compilation_for_registration_uncached` | `self` |
| Original Logical procedure MAY dependencies | `rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs` | `tcl-compiler` | `crate::registry_invocation::original_procedure_call` | `original_logical_procedure_may_dependencies_for_module` | pub(crate) | `rust/tcl-compiler/src/cfg_builder/summary_candidates.rs` | `crate::cfg_builder::summary_candidates` | `OriginalProcedureSummaryOwners::may_calls` | `static` |
| Original source name-role inventory | `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs` | `tcl-compiler` | `crate::registry_invocation::source_name_roles` | `SourceNameRoleInventory` | pub(crate) | `rust/tcl-compiler/src/var_escape/handlers.rs` | `crate::var_escape::handlers` | `handle_source_introspection` | `type` |
| Original source name-role inventory | `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs` | `tcl-compiler` | `crate::registry_invocation::source_name_roles` | `original_source_name_roles` | pub(crate) | `rust/tcl-compiler/src/var_escape/known_names.rs` | `crate::var_escape::known_names` | `visit` | `static` |
| Conservative local-slot metadata narrowing | `rust/tcl-compiler/src/var_escape/slot_resolution.rs` | `tcl-compiler` | `crate::var_escape::slot_resolution` | `typed_local_slot_name` | private | `rust/tcl-compiler/src/var_escape/slot_resolution.rs` | `crate::var_escape::slot_resolution` | `assign_local_slots` | `static` |
| Original handler slot eligibility | `rust/tcl-compiler/src/var_escape/slot_resolution.rs` | `tcl-compiler` | `crate::var_escape::slot_resolution` | `call_disables_slots` | private | `rust/tcl-compiler/src/var_escape/slot_resolution.rs` | `crate::var_escape::slot_resolution` | `stmt_disables_slots` | `static` |
| Original Authored body-name candidates | `rust/tcl-compiler/src/registry_invocation/source_body_names.rs` | `tcl-compiler` | `crate::registry_invocation::source_body_names` | `original_authored_body_name_candidates` | pub(crate) | `rust/tcl-compiler/src/var_escape/known_names.rs` | `crate::var_escape::known_names` | `visit` | `static` |
| Original Authored declaration and child source bodies | `rust/tcl-compiler/src/registry_invocation/authored_source_bodies.rs` | `tcl-compiler` | `crate::registry_invocation::authored_source_bodies` | `original_authored_procedure_body` | pub(crate) | `rust/tcl-compiler/src/lowering/authored_source_bodies.rs` | `crate::lowering::authored_source_bodies` | `Lowerer::try_lower_authored_procedure_declaration` | `static` |
| Original Authored declaration and child source bodies | `rust/tcl-compiler/src/registry_invocation/authored_body_scope.rs` | `tcl-compiler` | `crate::registry_invocation::authored_body_scope` | `OriginalAuthoredSourceBodyScope` | pub(crate) | `rust/tcl-compiler/src/command_binding.rs` | `crate::command_binding` | `SourceInvocationBinding::retain_original_authored_source_body_scope` | `type` |
| Original Authored control-flow source grammar | `rust/tcl-compiler/src/registry_invocation/authored_controls.rs` | `tcl-compiler` | `crate::registry_invocation::authored_controls` | `original_authored_control_invocation` | pub(crate) | `rust/tcl-compiler/src/lowering/mod.rs` | `crate::lowering` | `Lowerer::logical_structured_invocation` | `static` |
| Original Authored control-flow source grammar | `rust/tcl-compiler/src/registry_invocation/authored_controls.rs` | `tcl-compiler` | `crate::registry_invocation::authored_controls` | `AuthoredSourceInvocation` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/hosted_structure.rs` | `crate::registry_invocation::hosted_structure` | `SourceStructuredInvocation` | `type` |
| Original var-escape substitution inventory | `rust/tcl-compiler/src/var_escape/helpers.rs` | `tcl-compiler` | `crate::var_escape::helpers` | `OriginalSubstitutionInventory` | pub(crate) | `rust/tcl-compiler/src/var_escape/helpers.rs` | `crate::var_escape::helpers` | `original_substitution_invocations` | `type` |
| Original var-escape substitution inventory | `rust/tcl-compiler/src/word_subst.rs` | `tcl-compiler` | `crate::word_subst` | `OriginalAuthoredSourceSubstitutionWords` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs` | `crate::registry_invocation::source_name_roles` | `original_authored_substitution_name_roles` | `type` |
| Original var-escape substitution inventory | `rust/tcl-compiler/src/word_subst.rs` | `tcl-compiler` | `crate::word_subst` | `checked_authored_source_substitution_words` | pub(crate) | `rust/tcl-compiler/src/var_escape/helpers.rs` | `crate::var_escape::helpers` | `original_substitution_invocations` | `static` |
| Original var-escape substitution inventory | `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs` | `tcl-compiler` | `crate::registry_invocation::source_name_roles` | `original_authored_substitution_name_roles` | pub(crate) | `rust/tcl-compiler/src/var_escape/helpers.rs` | `crate::var_escape::helpers` | `original_substitution_invocations` | `static` |
| Original analysis SSA definitions | `rust/tcl-compiler/src/ssa.rs` | `tcl-compiler` | `crate::ssa` | `current_analysis_function` | private | `rust/tcl-compiler/src/ssa.rs` | `crate::ssa` | `original_analysis_definition_at_unit` | `static` |
| Configured single-command lexical projection | `rust/tcl-compiler/src/word_subst.rs` | `tcl-compiler` | `crate::word_subst` | `SingleCommandSourcePart` | pub(crate) | `rust/tcl-compiler/src/word_subst.rs` | `crate::word_subst` | `single_command_source_parts` | `type` |
| Configured single-command lexical projection | `rust/tcl-compiler/src/word_subst.rs` | `tcl-compiler` | `crate::word_subst` | `SingleCommandSourcePurpose` | pub(crate) | `rust/tcl-compiler/src/word_subst.rs` | `crate::word_subst` | `single_command_source_parts` | `type` |
| Configured single-command lexical projection | `rust/tcl-compiler/src/word_subst.rs` | `tcl-compiler` | `crate::word_subst` | `single_command_source_parts` | pub(crate) | `rust/tcl-compiler/src/codegen/cmd_subst.rs` | `crate::codegen::cmd_subst` | `parse_script_parts_with_config` | `static` |
| Original literal variable-name operands | `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs` | `tcl-compiler` | `crate::registry_invocation::source_name_roles` | `OriginalVariableNameOperand` | pub(crate) | `rust/tcl-compiler/src/var_escape/handlers.rs` | `crate::var_escape::handlers` | `transfer_introspection` | `type` |
| Original literal variable-name operands | `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs` | `tcl-compiler` | `crate::registry_invocation::source_name_roles` | `SourceNameRoleInventory::original_variable_name_operand` | pub(crate) | `rust/tcl-compiler/src/var_escape/handlers.rs` | `crate::var_escape::handlers` | `handle_source_introspection` | `parameter:invocation` |
| Original literal variable-name operands | `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs` | `tcl-compiler` | `crate::registry_invocation::source_name_roles` | `ResolvedStatementInvocation::original_variable_name_operand` | pub(crate) | `rust/tcl-compiler/src/var_escape/handlers.rs` | `crate::var_escape::handlers` | `handle_introspection` | `parameter:invocation` |
| Original source analysis attachments | `rust/tcl-compiler/src/command_binding.rs` | `tcl-compiler` | `crate::command_binding` | `OriginalSourceAnalysisOwner` | pub(crate) | `rust/tcl-compiler/src/command_binding.rs` | `crate::command_binding` | `SourceCommandBindings::original_analysis_owner` | `type` |
| Original source analysis attachments | `rust/tcl-compiler/src/command_binding.rs` | `tcl-compiler` | `crate::command_binding` | `SourceCommandBindings::original_analysis_owner` | pub(crate) | `rust/tcl-compiler/src/command_binding/source_transition_advice/native_declared_body.rs` | `crate::command_binding::source_transition_advice::native_declared_body` | `OriginalNativeSourceDeclaration::children` | `parameter:bindings` |
| Original control receiver name data | `rust/tcl-compiler/src/lowering/structured.rs` | `tcl-compiler` | `crate::lowering::structured` | `Lowerer::original_control_receiver_value` | private | `rust/tcl-compiler/src/lowering/structured.rs` | `crate::lowering::structured` | `Lowerer::lower_catch` | `self` |
| CFG structural variable-name data | `rust/tcl-compiler/src/ir_helpers.rs` | `tcl-compiler` | `crate::ir_helpers` | `structural_variable_write_projection` | pub(crate) | `rust/tcl-compiler/src/cfg_builder/mod.rs` | `crate::cfg_builder` | `structural_write_roots` | `static` |
| CFG structural variable-name data | `rust/tcl-compiler/src/cfg_builder/mod.rs` | `tcl-compiler` | `crate::cfg_builder` | `structural_write_roots` | private | `rust/tcl-compiler/src/cfg_builder/mod.rs` | `crate::cfg_builder` | `flow_facts_stmt_with_classes` | `static` |
| Original literal variable-name operands | `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs` | `tcl-compiler` | `crate::registry_invocation::source_name_roles` | `original_assignment_name_operand` | pub(crate) | `rust/tcl-compiler/src/var_escape/walker.rs` | `crate::var_escape::walker` | `walk_assign_or_incr` | `static` |
| Original dictionary destination data | `rust/tcl-compiler/src/lowering/structured.rs` | `tcl-compiler` | `crate::lowering::structured` | `Lowerer::original_dict_write_name` | private | `rust/tcl-compiler/src/lowering/structured.rs` | `crate::lowering::structured` | `Lowerer::lower_dict` | `self` |
| Original declaration source syntax memo | `rust/tcl-compiler/src/lowering/source_body_syntax.rs` | `tcl-compiler` | `crate::lowering::source_body_syntax` | `SourceBodySyntaxOwner` | pub(super) | `rust/tcl-compiler/src/lowering/source_body_syntax.rs` | `crate::lowering::source_body_syntax` | `Lowerer::source_syntax_request` | `type` |
| Original declaration source syntax memo | `rust/tcl-compiler/src/lowering/source_body_syntax.rs` | `tcl-compiler` | `crate::lowering::source_body_syntax` | `SourcePurpose` | private | `rust/tcl-compiler/src/lowering/source_body_syntax.rs` | `crate::lowering::source_body_syntax` | `SourceBodySyntaxOwner::geometry` | `type` |
| Logical completed class/member source declarations | `rust/tcl-compiler/src/analyser/types/logical_members.rs` | `tcl-compiler` | `crate::analyser::types::logical_members` | `LogicalSourceClassDeclaration` | pub | `rust/tcl-compiler/src/analyser/types/logical_members.rs` | `crate::analyser::types::logical_members` | `AnalysisResult::logical_source_class_declaration` | `type` |
| Logical completed class/member source declarations | `rust/tcl-compiler/src/analyser/types/logical_members.rs` | `tcl-compiler` | `crate::analyser::types::logical_members` | `LogicalSourceMemberDeclaration` | pub | `rust/tcl-compiler/src/analyser/types/logical_members.rs` | `crate::analyser::types::logical_members` | `LogicalSourceClassDeclaration::member` | `type` |
| Logical completed class/member source declarations | `rust/tcl-compiler/src/analyser/types/logical_members.rs` | `tcl-compiler` | `crate::analyser::types::logical_members` | `LogicalSourceClassDeclaration::capture` | pub(crate) | `rust/tcl-compiler/src/analyser/oo.rs` | `crate::analyser::oo` | `Analyser::retain_original_provider_class` | `static` |
| Original source analysis attachments | `rust/tcl-compiler/src/command_binding/retained_source.rs` | `tcl-compiler` | `crate::command_binding::retained_source` | `RetainedSourceModuleBindings::owns_original_analysis_attachment` | pub(crate) | `rust/tcl-compiler/src/command_binding/retained_source.rs` | `crate::command_binding::retained_source` | `RetainedSourceModuleBindings::owns_original_tokens` | `self` |
| Current editor source inventory | `rust/tcl-lsp-core/src/original_context.rs` | `tcl-lsp-core` | `crate::original_context` | `CurrentSourceContext::capture` | pub(crate) | `rust/tcl-lsp-core/src/document_symbols.rs` | `crate::document_symbols` | `document_symbols_from_analysis` | `static` |
| Original caller-frame scalar read geometry | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `FrameWalk::components` | pub(crate) | `rust/tcl-lsp-core/src/caller_frame.rs` | `crate::caller_frame` | `substituted_read_spans` | `producer:crate::refactor::FrameWalk::new` |
| Original variable token reference forms | `rust/tcl-compiler/src/var_refs.rs` | `tcl-compiler` | `crate::var_refs` | `original_variable_reference_form` | pub(crate) | `rust/tcl-compiler/src/analyser/scope.rs` | `crate::analyser::scope` | `Analyser::record_arg_var_reads` | `static` |
| Original variable token reference forms | `rust/tcl-compiler/src/var_refs.rs` | `tcl-compiler` | `crate::var_refs` | `original_variable_reference_form` | pub(crate) | `rust/tcl-compiler/src/analyser/scope.rs` | `crate::analyser::scope` | `collect_script_command_reads` | `static` |
| Original variable token reference forms | `rust/tcl-compiler/src/var_refs.rs` | `tcl-compiler` | `crate::var_refs` | `original_variable_reference_form` | pub(crate) | `rust/tcl-compiler/src/analyser/scope.rs` | `crate::analyser::scope` | `collect_expr_reads` | `static` |
| Current editor source inventory | `rust/tcl-lsp-core/src/original_context.rs` | `tcl-lsp-core` | `crate::original_context` | `CurrentSourceContext::capture` | pub(crate) | `rust/tcl-lsp-core/src/implementation.rs` | `crate::implementation` | `implementation` | `static` |
| Original variable completion extents | `rust/tcl-lsp-core/src/source_structure.rs` | `tcl-lsp-core` | `crate::source_structure` | `source_variable_reference_end` | pub(crate) | `rust/tcl-lsp-core/src/completion/original_variables.rs` | `crate::completion::original_variables` | `items` | `static` |
| SCCP phased write and destroy inventory | `rust/tcl-compiler/src/sccp/original_value_facts.rs` | `tcl-compiler` | `crate::sccp::original_value_facts` | `scan_defined_and_unset` | private | `rust/tcl-compiler/src/sccp/original_value_facts.rs` | `crate::sccp::original_value_facts` | `existence_constant_branches_with_metadata_context` | `static` |
| SCCP phased write and destroy inventory | `rust/tcl-compiler/src/sccp/original_value_facts.rs` | `tcl-compiler` | `crate::sccp::original_value_facts` | `retain_source_call_name_coverage` | private | `rust/tcl-compiler/src/sccp/original_value_facts.rs` | `crate::sccp::original_value_facts` | `scan_defined_and_unset` | `static` |
| Shimmer scalar sharing destinations | `rust/tcl-compiler/src/shimmer/sharing.rs` | `tcl-compiler` | `crate::shimmer::sharing` | `copy_destination_name` | private | `rust/tcl-compiler/src/shimmer/sharing.rs` | `crate::shimmer::sharing` | `copy_pair` | `static` |
| Shimmer scalar sharing destinations | `rust/tcl-compiler/src/shimmer/sharing.rs` | `tcl-compiler` | `crate::shimmer::sharing` | `mutation_target` | private | `rust/tcl-compiler/src/shimmer/sharing.rs` | `crate::shimmer::sharing` | `mutation_warning` | `static` |
| Original analysis SSA definitions | `rust/tcl-compiler/src/ssa.rs` | `tcl-compiler` | `crate::ssa` | `current_authored_analysis_function` | private | `rust/tcl-compiler/src/ssa.rs` | `crate::ssa` | `original_authored_analysis_definition_at_unit` | `static` |
| Original analysis SSA definitions | `rust/tcl-compiler/src/ssa.rs` | `tcl-compiler` | `crate::ssa` | `original_authored_definition_operands` | private | `rust/tcl-compiler/src/ssa.rs` | `crate::ssa` | `original_authored_analysis_definition_at_unit` | `static` |
| Original analysis SSA definitions | `rust/tcl-compiler/src/ssa.rs` | `tcl-compiler` | `crate::ssa` | `original_authored_analysis_definition_at_unit` | private | `rust/tcl-compiler/src/ssa.rs` | `crate::ssa` | `original_analysis_definition_at_unit` | `static` |
| Original analysis SSA definitions | `rust/tcl-compiler/src/registry_invocation/authored_body_scope.rs` | `tcl-compiler` | `crate::registry_invocation::authored_body_scope` | `OriginalAuthoredSourceBodyScope::belongs_to_original_scope` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/authored_source_bodies.rs` | `crate::registry_invocation::authored_source_bodies` | `OriginalAuthoredProcedureBody::owns_original_source_scope` | `parameter:scope` |
| Original source analysis attachments | `rust/tcl-compiler/src/command_binding.rs` | `tcl-compiler` | `crate::command_binding` | `SourceInvocationBinding::original_analysis_attachment` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/authored_body_scope.rs` | `crate::registry_invocation::authored_body_scope` | `OriginalAuthoredSourceBodyScope::original_analysis_attachment` | `field:self.header.source_binding` |
| Original source analysis attachments | `rust/tcl-compiler/src/registry_invocation/authored_body_scope.rs` | `tcl-compiler` | `crate::registry_invocation::authored_body_scope` | `OriginalAuthoredSourceBodyScope::original_analysis_attachment` | pub(crate) | `rust/tcl-compiler/src/command_binding.rs` | `crate::command_binding` | `SourceInvocationBinding::retain_original_authored_source_body_scope` | `parameter:scope` |
| Debugger current-frame variable reporting | `rust/tcl-vm/src/interp.rs` | `tcl-vm` | `crate::interp` | `Vm::debug_snapshot` | pub(crate) | `rust/tcl-vm/src/interp.rs` | `crate::interp` | `Vm::fire_debug_hook` | `self` |
| Current Logical member-provider correspondence | `rust/tcl-lsp-core/src/call_hierarchy.rs` | `tcl-lsp-core` | `crate::call_hierarchy` | `LogicalCallTarget::matches_indexed_member_provider` | pub(crate) | `rust/tcl-lsp-core/src/workspace_index.rs` | `crate::workspace_index` | `WorkspaceIndex::logical_member_provider_matches_target` | `parameter:target` |
| Original current variable reference extents | `rust/tcl-lsp-core/src/source_structure.rs` | `tcl-lsp-core` | `crate::source_structure` | `OriginalVariableReferences` | pub(crate) | `rust/tcl-lsp-core/src/minify.rs` | `crate::minify` | `variable_reference_name_spans` | `type` |
| Original current variable reference extents | `rust/tcl-lsp-core/src/source_structure.rs` | `tcl-lsp-core` | `crate::source_structure` | `OriginalVariableReferences::capture` | pub(crate) | `rust/tcl-lsp-core/src/minify.rs` | `crate::minify` | `compact_names` | `static` |
| Original current variable reference extents | `rust/tcl-lsp-core/src/source_structure.rs` | `tcl-lsp-core` | `crate::source_structure` | `OriginalVariableReferences::reference_at` | pub(crate) | `rust/tcl-lsp-core/src/minify.rs` | `crate::minify` | `variable_reference_name_spans` | `parameter:geometry` |
| Original Jim interpreter string crossings | `rust/tcl-vm/src/interp/native_jim_interpreter_crossing.rs` | `tcl-vm` | `crate::interp::native_jim_interpreter_crossing` | `Vm::copy_original_jim_value_to` | pub(super) | `rust/tcl-vm/src/interp/native_jim_interpreter_crossing.rs` | `crate::interp::native_jim_interpreter_crossing` | `Vm::eval_original_jim_child_object` | `self` |
| Original Jim interpreter string crossings | `rust/tcl-vm/src/interp/native_jim_interpreter_crossing.rs` | `tcl-vm` | `crate::interp::native_jim_interpreter_crossing` | `Vm::capture_original_jim_parent_alias_prefix` | pub(super) | `rust/tcl-vm/src/interp.rs` | `crate::interp` | `Vm::interp_alias_at_in_original_name` | `self` |
| Original compiler variable operands | `rust/tcl-compiler/src/codegen/values.rs` | `tcl-compiler` | `crate::codegen::values` | `CodegenCtx::original_variable_operand` | pub(super) | `rust/tcl-compiler/src/codegen/emitter/bytecoded.rs` | `crate::codegen::emitter::bytecoded` | `unset_cmd` | `parameter:ctx` |
| Original fixed scalar-formal alpha equivalence | `rust/tcl-compiler/src/analyser/types.rs` | `tcl-compiler` | `crate::analyser::types` | `ScalarAlphaFormalSelection` | private | `rust/tcl-compiler/src/analyser/types.rs` | `crate::analyser::types` | `AnalysisResult::original_scalar_alpha_for_formals` | `signature-type` |
| Original fixed scalar-formal alpha equivalence | `rust/tcl-compiler/src/analyser/types.rs` | `tcl-compiler` | `crate::analyser::types` | `AnalysisResult::original_scalar_alpha_for_formals` | private | `rust/tcl-compiler/src/analyser/types.rs` | `crate::analyser::types` | `AnalysisResult::original_fixed_scalar_body_alpha_rename` | `self` |
| Original positioned method-call syntax | `rust/tcl-lsp-core/src/source_structure.rs` | `tcl-lsp-core` | `crate::source_structure` | `original_method_site_at` | pub(crate) | `rust/tcl-lsp-core/src/definition.rs` | `crate::definition` | `original_instance_method_at_cursor` | `static` |
| original selected Core dispatch and namespace hazards | `rust/tcl-lsp-core/src/references.rs` | `tcl-lsp-core` | `crate::references` | `RetainedDispatchContext` | private | `rust/tcl-lsp-core/src/references.rs` | `crate::references` | `retained_dispatch_context` | `type` |
| original selected Core dispatch and namespace hazards | `rust/tcl-lsp-core/src/references.rs` | `tcl-lsp-core` | `crate::references` | `retained_dispatch_context` | private | `rust/tcl-lsp-core/src/references.rs` | `crate::references` | `dispatch_source_context` | `static` |
| original selected Core dispatch and namespace hazards | `rust/tcl-lsp-core/src/references.rs` | `tcl-lsp-core` | `crate::references` | `scan_my_method_sites` | pub(crate) | `rust/tcl-lsp-core/src/references.rs` | `crate::references` | `property_references_for_class` | `static` |
| reached original error-context ownership | `rust/tcl-vm/src/interp/native_error_stack.rs` | `tcl-vm` | `crate::interp::native_error_stack` | `NativeErrorStack::begin_inner` | pub(super) | `rust/tcl-vm/src/interp/native_error_stack.rs` | `crate::interp::native_error_stack` | `NativeErrorStack::begin_instruction` | `self` |
| reached original error-context ownership | `rust/tcl-vm/src/interp/native_error_stack.rs` | `tcl-vm` | `crate::interp::native_error_stack` | `NativeErrorStack::begin_instruction` | pub(super) | `rust/tcl-vm/src/interp.rs` | `crate::interp` | `Vm::capture_original_return_instruction_context` | `field:self.native_errors.error_stack` |
| reached original error-context ownership | `rust/tcl-vm/src/interp/native_error_stack.rs` | `tcl-vm` | `crate::interp::native_error_stack` | `NativeErrorStack::restart_inner` | pub(super) | `rust/tcl-vm/src/interp.rs` | `crate::interp` | `Vm::error_stack_restart_with_inner` | `field:self.native_errors.error_stack` |
| reached original error-context ownership | `rust/tcl-vm/src/value.rs` | `tcl-vm` | `crate::value` | `Value::capture_native_error_context` | pub(crate) | `rust/tcl-vm/src/interp/native_error_stack.rs` | `crate::interp::native_error_stack` | `NativeErrorStack::begin_inner` | `parameter:context` |
| original Logical procedure completion and no-error source calls | `rust/tcl-compiler/src/command_binding/retained_source.rs` | `tcl-compiler` | `crate::command_binding::retained_source` | `RetainedSourceModuleBindings::matches_original_procedure_summary` | pub(crate) | `rust/tcl-compiler/src/inlining/frame.rs` | `crate::inlining::frame` | `SourceContext::matches_procedure` | `field:self.retained` |
| original Logical procedure completion and no-error source calls | `rust/tcl-compiler/src/inlining/frame.rs` | `tcl-compiler` | `crate::inlining::frame` | `SourceContext::matches_procedure` | pub(super) | `rust/tcl-compiler/src/inlining/frame.rs` | `crate::inlining::frame` | `reads_only_bound_names` | `parameter:source` |
| original inlining source and alpha-rename geometry | `rust/tcl-compiler/src/inlining/frame.rs` | `tcl-compiler` | `crate::inlining::frame` | `SourceContext::invocation` | pub(super) | `rust/tcl-compiler/src/inlining/frame.rs` | `crate::inlining::frame` | `Frame::source_tokens` | `field:self.source` |
| original inlining source and alpha-rename geometry | `rust/tcl-compiler/src/inlining/frame.rs` | `tcl-compiler` | `crate::inlining::frame` | `reads_only_bound_names` | pub(super) | `rust/tcl-compiler/src/inlining/mod.rs` | `crate::inlining` | `build_inlinable_map` | `static` |
| original inlining source and alpha-rename geometry | `rust/tcl-compiler/src/inlining/mod.rs` | `tcl-compiler` | `crate::inlining` | `build_inlinable_map` | private | `rust/tcl-compiler/src/inlining/mod.rs` | `crate::inlining` | `inline_procedures` | `static` |
| original inlining source and alpha-rename geometry | `rust/tcl-compiler/src/inlining/heads.rs` | `tcl-compiler` | `crate::inlining::heads` | `names_a_command` | pub(super) | `rust/tcl-compiler/src/inlining/mod.rs` | `crate::inlining` | `splice_call_site` | `static` |
| original inlining source and alpha-rename geometry | `rust/tcl-compiler/src/inlining/heads.rs` | `tcl-compiler` | `crate::inlining::heads` | `root` | pub(super) | `rust/tcl-compiler/src/inlining/mod.rs` | `crate::inlining` | `splice_call_site` | `static` |
| original inlining source and alpha-rename geometry | `rust/tcl-compiler/src/inlining/heads.rs` | `tcl-compiler` | `crate::inlining::heads` | `substitutes` | private | `rust/tcl-compiler/src/inlining/heads.rs` | `crate::inlining::heads` | `call_substitutes` | `static` |
| original inlining source and alpha-rename geometry | `rust/tcl-compiler/src/inlining/rename.rs` | `tcl-compiler` | `crate::inlining::rename` | `rewrite_value_string` | private | `rust/tcl-compiler/src/inlining/rename.rs` | `crate::inlining::rename` | `rewrite_array_index_tail` | `static` |
| Typed instance-variable write candidates | `rust/tcl-compiler/src/ir_helpers.rs` | `tcl-compiler` | `crate::ir_helpers` | `structural_variable_write_projection` | pub(crate) | `rust/tcl-compiler/src/taint/instance_classes.rs` | `crate::taint::instance_classes` | `invalidate_structural_instance_names` | `static` |
| original branch expression source grammar | `rust/tcl-compiler/src/optimiser/branch_folding.rs` | `tcl-compiler` | `crate::optimiser::branch_folding` | `branch_original_expression` | private | `rust/tcl-compiler/src/optimiser/branch_folding.rs` | `crate::optimiser::branch_folding` | `branch_execution_equivalent` | `static` |
| original branch expression source grammar | `rust/tcl-compiler/src/optimiser/branch_folding.rs` | `tcl-compiler` | `crate::optimiser::branch_folding` | `branch_execution_equivalent` | private | `rust/tcl-compiler/src/optimiser/branch_folding.rs` | `crate::optimiser::branch_folding` | `propagate_into_branches` | `static` |
| nested codegen lexical source operands | `rust/tcl-compiler/src/codegen/statements.rs` | `tcl-compiler` | `crate::codegen::statements` | `CodegenCtx::nested_command_tokens` | pub(super) | `rust/tcl-compiler/src/codegen/statements.rs` | `crate::codegen::statements` | `CodegenCtx::emit_inline_cmd_subst_from_word` | `self` |
| Selected codegen wrapping traits | `rust/tcl-compiler/src/codegen/cmd_subst.rs` | `tcl-compiler` | `crate::codegen::cmd_subst` | `CodegenCtx::selected_invocation_traits` | pub(super) | `rust/tcl-compiler/src/codegen/control_flow.rs` | `crate::codegen::control_flow` | `CodegenCtx::emit_catch_body` | `self` |
| Original Logical class-body syntax | `rust/tcl-compiler/src/ir/definition_script.rs` | `tcl-compiler` | `crate::ir::definition_script` | `Module::original_class_definition_body_source` | pub(crate) | `rust/tcl-compiler/src/analyser/types/logical_members.rs` | `crate::analyser::types::logical_members` | `AnalysisResult::retain_logical_class_body_sources` | `parameter:module` |
| Original Logical class-body syntax | `rust/tcl-compiler/src/analyser/types/logical_members.rs` | `tcl-compiler` | `crate::analyser::types::logical_members` | `AnalysisResult::retain_logical_class_body_sources` | pub(crate) | `rust/tcl-compiler/src/analyser/diagnostics.rs` | `crate::analyser::diagnostics` | `Analyser::produce_object_handle_facts` | `field:self.result` |
| Original Authored declaration and child source bodies | `rust/tcl-compiler/src/command_binding.rs` | `tcl-compiler` | `crate::command_binding` | `SourceInvocationBinding::has_original_authored_source_body_name_role_owner` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/authored_controls.rs` | `crate::registry_invocation::authored_controls` | `original_authored_control_invocation` | `parameter-field:tokens:source_binding` |
| Original Authored declaration and child source bodies | `rust/tcl-compiler/src/command_binding.rs` | `tcl-compiler` | `crate::command_binding` | `SourceInvocationBinding::original_authored_source_body_scope_for_name_roles` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/authored_controls.rs` | `crate::registry_invocation::authored_controls` | `original_authored_control_invocation` | `parameter-field:tokens:source_binding` |
| Original Logical procedure MAY dependencies | `rust/tcl-compiler/src/cfg_builder/summary_candidates.rs` | `tcl-compiler` | `crate::cfg_builder::summary_candidates` | `OriginalProcedureSummaryOwners::may_calls` | pub(super) | `rust/tcl-compiler/src/cfg_builder/global_write_info.rs` | `crate::cfg_builder::global_write_info` | `direct_call_targets` | `parameter:owners` |
| Original Logical procedure MAY dependencies | `rust/tcl-compiler/src/command_binding/retained_source.rs` | `tcl-compiler` | `crate::command_binding::retained_source` | `RetainedSourceModuleBindings::matches_original_procedure_summary` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs` | `crate::registry_invocation::original_procedure_call` | `original_source_procedure_ids_for_word_for_module` | `parameter-field:module:retained_source_bindings` |
| Original Native declaration child source grammar | `rust/tcl-compiler/src/command_binding/generated_procedures.rs` | `tcl-compiler` | `crate::command_binding::generated_procedures` | `OriginalNativeProcedureBody::source_children` | pub(crate) | `rust/tcl-compiler/src/lowering/installed_bodies.rs` | `crate::lowering::installed_bodies` | `Lowerer::retain_original_native_body` | `parameter:card` |
| Original Native declaration child source grammar | `rust/tcl-compiler/src/command_binding/source_transition_advice/native_declared_body.rs` | `tcl-compiler` | `crate::command_binding::source_transition_advice::native_declared_body` | `SourceInvocationBinding::has_original_native_source_command` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs` | `crate::registry_invocation::original_procedure_call` | `original_procedure_may_dependencies_for_word_for_module` | `parameter-field:tokens:source_binding` |
| Original Native declaration child source grammar | `rust/tcl-compiler/src/command_binding/source_transition_advice/native_declared_body.rs` | `tcl-compiler` | `crate::command_binding::source_transition_advice::native_declared_body` | `SourceInvocationBinding::original_native_source_may_owner` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs` | `crate::registry_invocation::original_procedure_call` | `original_procedure_may_dependencies_for_word_for_module` | `parameter-field:tokens:source_binding` |
| Original Native declaration child source grammar | `rust/tcl-compiler/src/command_binding/source_transition_advice/native_declared_body.rs` | `tcl-compiler` | `crate::command_binding::source_transition_advice::native_declared_body` | `SourceInvocationBinding::original_native_source_words` | pub(crate) | `rust/tcl-compiler/src/word_subst.rs` | `crate::word_subst` | `collect_original_expression_calls` | `parameter-field:tokens:source_binding` |
| Original Native declaration child source grammar | `rust/tcl-compiler/src/command_binding/source_transition_advice/body_effects.rs` | `tcl-compiler` | `crate::command_binding::source_transition_advice::body_effects` | `AdviceInvocationContext::retain_native_expression_syntax_inventory` | pub(super) | `rust/tcl-compiler/src/command_binding/source_transition_advice.rs` | `crate::command_binding::source_transition_advice` | `AdviceInvocationContext::retain_source_inventory` | `self` |
| Original literal variable-name operands | `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs` | `tcl-compiler` | `crate::registry_invocation::source_name_roles` | `OriginalVariableNameOperand::as_str` | pub(crate) | `rust/tcl-compiler/src/var_escape/state.rs` | `crate::var_escape::state` | `EscapeState::invalidate_original_literal` | `parameter:name` |
| conditional declared overwrite advice | `rust/tcl-compiler/src/analyser/diagnostics.rs` | `tcl-compiler` | `crate::analyser::diagnostics` | `Analyser::emit_cfg_ssa_diagnostics_for_function_full` | pub(crate) | `rust/tcl-compiler/src/analyser/diagnostics.rs` | `crate::analyser::diagnostics` | `Analyser::emit_cfg_ssa_diagnostics_for_function` | `self` |
| diagnostic source and overwrite advice | `rust/tcl-compiler/src/analyser/diagnostics.rs` | `tcl-compiler` | `crate::analyser::diagnostics` | `Analyser::emit_cfg_ssa_diagnostics_for_function_full` | pub(crate) | `rust/tcl-compiler/src/analyser/diagnostics.rs` | `crate::analyser::diagnostics` | `Analyser::emit_cfg_ssa_diagnostics_for_function` | `self` |
| Source-owned analysis cache correspondence | `rust/tcl-compiler/src/command_binding/retained_source.rs` | `tcl-compiler` | `crate::command_binding::retained_source` | `RetainedSourceModuleBindings::original_logical_source_slot_for_body` | pub(crate) | `rust/tcl-compiler/src/body_analysis_capsule.rs` | `crate::body_analysis_capsule` | `BodyLatticeInputs::retain` | `parameter-field:request:module.retained_source_bindings` |
| Runtime command registration and identity | `runtime/rust/src/interp.rs` | `tcl-runtime` | `crate::interp` | `Interp::register_stock_builtin` | `pub(crate)` | `runtime/rust/src/builtins.rs` | `crate::builtins` | `install_for_bootstrap` | `parameter:interp` |
| Runtime command registration and identity | `runtime/rust/src/interp.rs` | `tcl-runtime` | `crate::interp` | `Interp::attested_command_identity` | `private` | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::attested_identities` | `self` |
| Runtime command registration and identity | `runtime/rust/src/interp.rs` | `tcl-runtime` | `crate::interp` | `Interp::bind_builtin` | `private` | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::register_builtin` | `self` |
| Runtime command registration and identity | `runtime/rust/src/interp.rs` | `tcl-runtime` | `crate::interp` | `Interp::synchronise_command_environment_guards` | `private` | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::prepare_command_guard` | `self` |
| Runtime command registration and identity | `runtime/rust/src/interp/stock_ensembles.rs` | `tcl-runtime` | `crate::interp::stock_ensembles` | `Interp::stock_ensemble_registry_identity` | `pub(super)` | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::attested_command_identity` | `self` |
| Runtime command registration and identity | `runtime/rust/src/interp/stock_ensembles.rs` | `tcl-runtime` | `crate::interp::stock_ensembles` | `Interp::stock_ensemble_implementation_is_current` | `pub(super)` | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::attested_command_identity` | `self` |
| captured Runtime byte storage | `runtime/rust/src/vars.rs` | `tcl-runtime` | `crate::vars` | `CapturedByteStore` | pub(crate) | `runtime/rust/src/vars/entered_container.rs` | `crate::vars::entered_container` | `capture_byte_store_entered` | `signature-type` |
| captured Runtime byte storage | `runtime/rust/src/vars/entered_container.rs` | `tcl-runtime` | `crate::vars::entered_container` | `capture_byte_store_entered` | pub(crate) | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::store_captured_byte_variable` | `static` |
| captured Runtime byte storage | `runtime/rust/src/frame/container.rs` | `tcl-runtime` | `crate::frame::container` | `CapturedContainerModel` | `pub(crate)` | `runtime/rust/src/frame/container.rs` | `crate::frame::container` | `VarTable::capture_container_model` | `signature-type` |
| captured Runtime byte storage | `runtime/rust/src/frame/container.rs` | `tcl-runtime` | `crate::frame::container` | `VarTable::publish_dictionary_root` | `pub(super)` | `runtime/rust/src/frame/container.rs` | `crate::frame::container` | `VarTable::store_dictionary_root` | `self` |
| captured Runtime byte storage | `runtime/rust/src/frame.rs` | `tcl-runtime` | `crate::frame` | `VarTable::publish_scalar` | `pub(crate)` | `runtime/rust/src/frame.rs` | `crate::frame` | `VarTable::store_scalar` | `self` |
| captured Runtime byte storage | `runtime/rust/src/frame.rs` | `tcl-runtime` | `crate::frame` | `VarTable::publish_elem` | `pub(crate)` | `runtime/rust/src/frame.rs` | `crate::frame` | `VarTable::store_elem` | `self` |
| captured Runtime byte storage | `runtime/rust/src/interp.rs` | `tcl-runtime` | `crate::interp` | `Interp::store_captured_byte_variable` | `pub(crate)` | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::var_set_at` | `self` |
| captured Runtime byte storage | `runtime/rust/src/interp.rs` | `tcl-runtime` | `crate::interp` | `Interp::store_native_compiled_temporary_checked` | `pub(crate)` | `runtime/rust/src/interp/native_body_artifact/native_dictionary_scope.rs` | `crate::interp::native_body_artifact::native_dictionary_scope` | `Interp::dictionary_store_temporary` | `self` |
| captured Runtime variable removal | `runtime/rust/src/frame/retired_variable.rs` | `tcl-runtime` | `crate::frame::retired_variable` | `RetiredVariableParts` | `pub(crate)` | `runtime/rust/src/frame.rs` | `crate::frame` | `VarTable::publish_remove_after_default` | `signature-type` |
| captured Runtime variable removal | `runtime/rust/src/frame.rs` | `tcl-runtime` | `crate::frame` | `VarTable::publish_remove_after_default` | `pub(crate)` | `runtime/rust/src/frame.rs` | `crate::frame` | `VarTable::remove` | `self` |
| captured Runtime variable removal | `runtime/rust/src/frame.rs` | `tcl-runtime` | `crate::frame` | `VarTable::publish_remove_elem` | `pub(crate)` | `runtime/rust/src/frame.rs` | `crate::frame` | `VarTable::remove_elem` | `self` |
| captured Runtime variable removal | `runtime/rust/src/frame/rmw.rs` | `tcl-runtime` | `crate::frame::rmw` | `VariableReceiver::publish_unset` | `pub(crate)` | `runtime/rust/src/frame/rmw.rs` | `crate::frame::rmw` | `VariableReceiver::unset_checked` | `self` |
| captured Runtime variable removal | `runtime/rust/src/interp/native_variable_teardown.rs` | `tcl-runtime` | `crate::interp::native_variable_teardown` | `Interp::remove_captured_byte_variable` | `pub(super)` | `runtime/rust/src/interp/native_variable_teardown.rs` | `crate::interp::native_variable_teardown` | `Interp::unset_byte_variable_at` | `self` |
| captured Runtime variable removal | `runtime/rust/src/interp/native_variable_teardown.rs` | `tcl-runtime` | `crate::interp::native_variable_teardown` | `Interp::retire_namespace_variables` | `pub(super)` | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::delete_namespace_token` | `self` |
| native original Jim key producer handoff | `runtime/rust/src/interp/native_command_names.rs` | `tcl-runtime` | `crate::interp::native_command_names` | `OriginalByteCommandRegistration` | `pub(super)` | `runtime/rust/src/interp/native_command_names.rs` | `crate::interp::native_command_names` | `Interp::remember_original_byte_command_registration` | `type` |
| native original Jim key producer handoff | `runtime/rust/src/interp/native_command_names.rs` | `tcl-runtime` | `crate::interp::native_command_names` | `Interp::prune_original_byte_command_registrations` | `pub(super)` | `runtime/rust/src/interp/native_command_names.rs` | `crate::interp::native_command_names` | `Interp::remember_original_byte_command_registration` | `self` |
| native original Jim key producer handoff | `runtime/rust/src/interp/native_command_names.rs` | `tcl-runtime` | `crate::interp::native_command_names` | `Interp::remember_original_byte_command_registration` | `pub(super)` | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::bind_builtin_publication` | `self` |
| native original Jim key producer handoff | `runtime/rust/src/interp/native_command_names.rs` | `tcl-runtime` | `crate::interp::native_command_names` | `Interp::publish_original_jim_registration_keys` | `pub(super)` | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::install_pin` | `self` |
| dictionary scope binding | `runtime/rust/src/interp/native_dictionary.rs` | `tcl-runtime` | `crate::interp::native_dictionary` | `DictionaryScopeRead` | `pub(crate)` | `runtime/rust/src/interp/native_dictionary.rs` | `crate::interp::native_dictionary` | `Interp::dictionary_scope_variable_read` | `signature-type` |
| original C9 class-variable links | `runtime/rust/src/cmd_oo.rs` | `tcl-runtime` | `crate::cmd_oo` | `classvariable_original` | `private` | `runtime/rust/src/cmd_oo.rs` | `crate::cmd_oo` | `classvariable_cmd` | `static` |
| original C9 class-variable links | `runtime/rust/src/interp/native_variable_names.rs` | `tcl-runtime` | `crate::interp::native_variable_names` | `Interp::prepare_original_c_namespace_link_target` | `pub(crate)` | `runtime/rust/src/interp/native_variable_names.rs` | `crate::interp::native_variable_names` | `Interp::link_original_c_namespace_objects` | `self` |
| active VM artifact identity | `rust/tcl-vm/src/interp.rs` | `tcl-vm` | `crate::interp` | `Vm::function_identity_claims_match` | `pub(crate)` | `rust/tcl-vm/src/interp.rs` | `crate::interp` | `Vm::function_live_command_bindings_match` | `self` |
| active VM artifact identity | `rust/tcl-vm/src/interp.rs` | `tcl-vm` | `crate::interp` | `Vm::function_live_command_bindings_match` | `pub(crate)` | `rust/tcl-vm/src/exec.rs` | `crate::exec` | `Vm::active_native_bindings_match` | `self` |
| active VM artifact identity | `rust/tcl-vm/src/exec.rs` | `tcl-vm` | `crate::exec` | `Vm::run_compiled_unit` | `pub(crate)` | `rust/tcl-vm/src/exec.rs` | `crate::exec` | `Vm::run_admitted_function` | `self` |
| active VM artifact identity | `rust/tcl-vm/src/exec.rs` | `tcl-vm` | `crate::exec` | `Frame` | `pub(crate)` | `rust/tcl-vm/src/exec.rs` | `crate::exec` | `Vm::active_native_bindings_match` | `signature-type` |
| checked original numeric stages | `rust/tcl-vm/src/interp/native_numeric_ops.rs` | `tcl-vm` | `crate::interp::native_numeric_ops` | `OriginalNumericOps` | `pub(crate)` | `rust/tcl-vm/src/cmd_package/original.rs` | `crate::cmd_package::original` | `retain_at_owner` | `signature-type` |
| checked original numeric stages | `rust/tcl-vm/src/interp.rs` | `tcl-vm` | `crate::interp` | `Vm::numeric_policy_context` | `pub(crate)` | `rust/tcl-vm/src/interp.rs` | `crate::interp` | `Vm::authored_numeric_context` | `self` |
| checked original numeric stages | `rust/tcl-vm/src/interp.rs` | `tcl-vm` | `crate::interp` | `Vm::authored_numeric_context` | `pub(crate)` | `rust/tcl-vm/src/interp.rs` | `crate::interp` | `Vm::prepare_increment_amount` | `self` |
| native string materialisation | `rust/tcl-vm/src/interp/native_numeric_ops.rs` | `tcl-vm` | `crate::interp::native_numeric_ops` | `OriginalNumericOps::string` | `pub(crate)` | `rust/tcl-vm/src/interp/native_numeric_ops.rs` | `crate::interp::native_numeric_ops` | `OriginalNumericOps::getter` | `self` |
| native string materialisation | `rust/tcl-vm/src/interp/native_numeric_ops.rs` | `tcl-vm` | `crate::interp::native_numeric_ops` | `OriginalNumericOps::string_value` | `pub(crate)` | `rust/tcl-vm/src/interp/native_numeric_ops.rs` | `crate::interp::native_numeric_ops` | `OriginalNumericOps::cache_value_number` | `self` |
| selected VM execution frame and saved target | `rust/tcl-vm/src/interp/native_operation_currency.rs` | `tcl-vm` | `crate::interp::native_operation_currency` | `SavedExecutionTarget` | `pub(crate)` | `rust/tcl-vm/src/interp/selected_frame.rs` | `crate::interp::selected_frame` | `Vm::select_execution_frame_using` | `signature-type` |
| selected VM execution frame and saved target | `rust/tcl-vm/src/interp/selected_frame.rs` | `tcl-vm` | `crate::interp::selected_frame` | `SelectedFrameRestore` | `pub(crate)` | `rust/tcl-vm/src/interp/selected_frame.rs` | `crate::interp::selected_frame` | `Vm::restore_execution_frame` | `signature-type` |
| selected VM execution frame and saved target | `rust/tcl-vm/src/interp/selected_frame.rs` | `tcl-vm` | `crate::interp::selected_frame` | `Vm::select_execution_frame` | `pub(crate)` | `rust/tcl-vm/src/interp.rs` | `crate::interp` | `Vm::eval_value_at_level` | `self` |
| selected VM execution frame and saved target | `rust/tcl-vm/src/interp/selected_frame.rs` | `tcl-vm` | `crate::interp::selected_frame` | `Vm::select_execution_frame_checked` | `pub(crate)` | `rust/tcl-vm/src/interp.rs` | `crate::interp` | `Vm::eval_value_at_level_checked` | `self` |
| selected VM execution frame and saved target | `rust/tcl-vm/src/interp/selected_frame.rs` | `tcl-vm` | `crate::interp::selected_frame` | `Vm::restore_execution_frame` | `pub(crate)` | `rust/tcl-vm/src/interp.rs` | `crate::interp` | `Vm::eval_value_in_selected_frame` | `self` |
| selected VM execution frame and saved target | `rust/tcl-vm/src/interp.rs` | `tcl-vm` | `crate::interp` | `Vm::eval_value_at_level_checked` | `pub(crate)` | `rust/tcl-vm/src/cmd_package/original.rs` | `crate::cmd_package::original` | `OriginalPackage::eval_script` | `field:self.vm` |
| intrinsic original stock class creation | `rust/tcl-compiler/src/command_binding/original_class_factory_transfer.rs` | `tcl-compiler` | `crate::command_binding::original_class_factory_transfer` | `OriginalClassFactoryTransfer::capture` | pub(super) | `rust/tcl-compiler/src/command_binding.rs` | `crate::command_binding` | `SourceCommandBindings::prepare_native_definition` | `static` |
| intrinsic original stock class creation | `rust/tcl-compiler/src/command_binding/original_class_factory_transfer.rs` | `tcl-compiler` | `crate::command_binding::original_class_factory_transfer` | `OriginalClassFactoryTransfer::completed` | pub(super) | `rust/tcl-compiler/src/command_binding/original_class_factory_transfer.rs` | `crate::command_binding::original_class_factory_transfer` | `OriginalClassFactoryTransfer::retain_completed_definition` | `self` |
| original stock TclOO factory/bootstrap roles | `rust/tcl-compiler/src/command_binding/registered_class_factory.rs` | `tcl-compiler` | `crate::command_binding::registered_class_factory` | `OriginalClassFactoryState` | pub(super) | `rust/tcl-compiler/src/command_binding/deferred_method.rs` | `crate::command_binding::deferred_method` | `ModuleCommandBindings::record_class_definition` | `signature-type` |
| Original formal name source geometry | `rust/tcl-compiler/src/interprocedural/transfer.rs` | `tcl-compiler` | `crate::interprocedural::transfer` | `ModuleProcedures::original_source_module` | pub(crate) | `rust/tcl-compiler/src/analyser/diagnostics.rs` | `crate::analyser::diagnostics` | `Analyser::emit_cfg_ssa_diagnostics_for_function_with_cells` | `parameter:module` |
| selected possible option source metadata | `rust/tcl-compiler/src/registry_invocation.rs` | `tcl-compiler` | `crate::registry_invocation` | `conditional_option_arguments_with_metadata_context` | pub(crate) | `rust/tcl-compiler/src/taint.rs` | `crate::taint` | `option_injection_arguments` | `static` |
| original declaration read metadata | `rust/tcl-compiler/src/script_binds.rs` | `tcl-compiler` | `crate::script_binds` | `authored_procedure_read_advice` | pub(crate) | `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs` | `crate::analyser::diagnostics::dataflow` | `Analyser::emit_read_before_set_diagnostics` | `static` |
| original diagnostic protocol peers | `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs` | `tcl-compiler` | `crate::analyser::diagnostics::dataflow` | `Analyser::original_dispatch_peers` | private | `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs` | `crate::analyser::diagnostics::dataflow` | `Analyser::emit_unused_param_diagnostics` | `self` |
| original diagnostic protocol peers | `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs` | `tcl-compiler` | `crate::analyser::diagnostics::dataflow` | `Analyser::dispatch_protocol_signatures` | private | `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs` | `crate::analyser::diagnostics::dataflow` | `Analyser::emit_unused_param_diagnostics` | `self` |
| original diagnostic protocol peers | `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs` | `tcl-compiler` | `crate::analyser::diagnostics::dataflow` | `original_dispatch_site` | private | `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs` | `crate::analyser::diagnostics::dataflow` | `Analyser::dispatch_protocol_signatures` | `static` |
| original source index diagnostics | `rust/tcl-compiler/src/analyser/bounds_checks/source_index.rs` | `tcl-compiler` | `crate::analyser::bounds_checks::source_index` | `original_index_diagnostics` | pub(in crate::analyser) | `rust/tcl-compiler/src/analyser/diagnostics/proven.rs` | `crate::analyser::diagnostics::proven` | `Analyser::rerun_index_checks` | `static` |
| conditional literal body name ownership | `rust/tcl-compiler/src/cfg_builder/global_write_info.rs` | `tcl-compiler` | `crate::cfg_builder::global_write_info` | `own_frame_global_writes_with_metadata_context` | pub(crate) | `rust/tcl-compiler/src/deferred_writes/supplied.rs` | `crate::deferred_writes::supplied` | `SourceScan::lambda` | `static` |
| conditional literal body name ownership | `rust/tcl-compiler/src/cfg_builder/global_write_info.rs` | `tcl-compiler` | `crate::cfg_builder::global_write_info` | `own_frame_global_writes_standalone` | pub(crate) | `rust/tcl-compiler/src/deferred_writes.rs` | `crate::deferred_writes` | `Scan::lambda` | `static` |
| Logical object assignment and factory candidates | `rust/tcl-compiler/src/registry_invocation.rs` | `tcl-compiler` | `crate::registry_invocation` | `original_advisory_value_assignments_with_metadata_context` | pub(crate) | `rust/tcl-compiler/src/object_types.rs` | `crate::object_types` | `source_assignment_candidates` | `static` |
| Logical object assignment and factory candidates | `rust/tcl-compiler/src/registry_invocation.rs` | `tcl-compiler` | `crate::registry_invocation` | `advisory_object_factories_with_metadata_context` | pub(crate) | `rust/tcl-compiler/src/object_types.rs` | `crate::object_types` | `source_object_factory_candidates` | `static` |
| Whole source variable-reference shapes | `rust/tcl-compiler/src/value_transfer.rs` | `tcl-compiler` | `crate::value_transfer` | `LatticeDriver::source_reference_lexer_config` | pub(crate) | `rust/tcl-compiler/src/sccp.rs` | `crate::sccp` | `fold_assign_value` | `parameter:driver` |
| Whole source variable-reference shapes | `rust/tcl-compiler/src/value_transfer.rs` | `tcl-compiler` | `crate::value_transfer` | `LatticeDriver::source_reference_lexer_config` | pub(crate) | `rust/tcl-compiler/src/value_transfer.rs` | `crate::value_transfer` | `LatticeDriver::evaluate_loop_header` | `self` |
| complete retained Module source correspondence | `rust/tcl-compiler/src/optimiser/propagation.rs` | `tcl-compiler` | `crate::optimiser::propagation` | `retained_substitution_calls` | private | `rust/tcl-compiler/src/optimiser/propagation.rs` | `crate::optimiser::propagation` | `visit_oo_frame_folds` | `static` |
| complete retained Module source correspondence | `rust/tcl-compiler/src/optimiser/method_barrier.rs` | `tcl-compiler` | `crate::optimiser::method_barrier` | `proc_dispatch_facts` | private | `rust/tcl-compiler/src/optimiser/method_barrier.rs` | `crate::optimiser::method_barrier` | `compute` | `static` |
| conditional literal body name ownership | `rust/tcl-compiler/src/deferred_writes/supplied.rs` | `tcl-compiler` | `crate::deferred_writes::supplied` | `selected_values` | private | `rust/tcl-compiler/src/deferred_writes/supplied.rs` | `crate::deferred_writes::supplied` | `SourceScan::original_registration` | `static` |
| original summary source dependencies | `rust/tcl-lsp-db/src/lib.rs` | `tcl-lsp-db` | `crate` | `summary_deps_key` | private | `rust/tcl-lsp-db/src/lib.rs` | `crate` | `solve_checks_for_unit` | `static` |
| conditional dictionary source phase | `rust/tcl-compiler/src/registry_invocation/source_scoped_body.rs` | `tcl-compiler` | `crate::registry_invocation::source_scoped_body` | `original_written_script_body_is_empty` | pub(crate) | `rust/tcl-compiler/src/command_binding/logical_operation.rs` | `crate::command_binding::logical_operation` | `original_empty_dictionary_body` | `static` |
| retained expression assistance and unavailable callback source joins | `rust/tcl-compiler/src/analyser/diagnostics/validity.rs` | `tcl-compiler` | `crate::analyser::diagnostics::validity` | `gated_operator_assistance_context` | private | `rust/tcl-compiler/src/analyser/diagnostics/validity.rs` | `crate::analyser::diagnostics::validity` | `Analyser::emit_w003_dialect_invalid_expr_operator` | `static` |
| Logical object assignment and factory candidates | `rust/tcl-compiler/src/object_types.rs` | `tcl-compiler` | `crate::object_types` | `object_handle_facts_for_analysis` | pub(crate) | `rust/tcl-compiler/src/analyser/diagnostics.rs` | `crate::analyser::diagnostics` | `Analyser::produce_object_handle_facts` | `static` |
| Logical object assignment and factory candidates | `rust/tcl-compiler/src/object_types.rs` | `tcl-compiler` | `crate::object_types` | `original_logical_constructor_class` | private | `rust/tcl-compiler/src/object_types.rs` | `crate::object_types` | `constructor_class_candidate` | `static` |
| Original lowered expression name footprints | `rust/tcl-compiler/src/registry_invocation/original_word_data.rs` | `tcl-compiler` | `crate::registry_invocation::original_word_data` | `original_lowered_expression_syntax` | pub(crate) | `rust/tcl-compiler/src/executable_ir.rs` | `crate::executable_ir` | `expr_footprint` | `static` |
| Original lowered expression name footprints | `rust/tcl-compiler/src/command_binding/original_name_value.rs` | `tcl-compiler` | `crate::command_binding::original_name_value` | `SourceInvocationBinding::original_variable_reference_policy` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/original_word_data.rs` | `crate::registry_invocation::original_word_data` | `original_variable_reference_syntax` | `parameter-field:tokens:source_binding` |
| Original source value components | `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs` | `tcl-compiler` | `crate::registry_invocation::source_name_roles` | `original_name_role_values` | pub(crate) | `rust/tcl-compiler/src/lowering/structured.rs` | `crate::lowering::structured` | `Lowerer::lower_foreach` | `static` |
| Original source value components | `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs` | `tcl-compiler` | `crate::registry_invocation::source_name_roles` | `original_name_role_values` | pub(crate) | `rust/tcl-compiler/src/lowering/structured.rs` | `crate::lowering::structured` | `Lowerer::lower_foreach_line` | `static` |
| Original source value components | `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs` | `tcl-compiler` | `crate::registry_invocation::source_name_roles` | `original_name_role_values` | pub(crate) | `rust/tcl-compiler/src/lowering/structured.rs` | `crate::lowering::structured` | `Lowerer::lower_dict` | `static` |
| Original source value components | `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs` | `tcl-compiler` | `crate::registry_invocation::source_name_roles` | `original_name_role_values` | pub(crate) | `rust/tcl-compiler/src/lowering/mod.rs` | `crate::lowering` | `Lowerer::lower_array_for` | `static` |
| Original source value components | `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs` | `tcl-compiler` | `crate::registry_invocation::source_name_roles` | `original_name_role_values` | pub(crate) | `rust/tcl-compiler/src/executable_ir.rs` | `crate::executable_ir` | `original_list_iterator_names` | `static` |
| Logical original alias prefix provenance | `rust/tcl-compiler/src/command_binding.rs` | `tcl-compiler` | `crate::command_binding` | `SourceCommandTarget::original_logical_prepended_name_inputs` | pub(crate) | `rust/tcl-compiler/src/registry_invocation.rs` | `crate::registry_invocation` | `original_logical_prefix_for_tokens` | `parameter:target` |
| retained expression assistance and unavailable callback source joins | `rust/tcl-compiler/src/analyser/commands.rs` | `tcl-compiler` | `crate::analyser::commands` | `Analyser::expr_function_calls` | pub(in crate::analyser) | `rust/tcl-compiler/src/analyser/diagnostics/validity.rs` | `crate::analyser::diagnostics::validity` | `Analyser::emit_expr_function_dialect_diagnostics` | `self` |
| retained expression assistance and unavailable callback source joins | `rust/tcl-compiler/src/analyser/diagnostics/usage.rs` | `tcl-compiler` | `crate::analyser::diagnostics::usage` | `Analyser::original_expression_parser_context` | pub(in crate::analyser) | `rust/tcl-compiler/src/analyser/diagnostics/usage.rs` | `crate::analyser::diagnostics::usage` | `Analyser::emit_w110_string_eq_ne` | `self` |
| Original positioned method-call syntax | `rust/tcl-lsp-core/src/definition.rs` | `tcl-lsp-core` | `crate::definition` | `original_instance_method_at_cursor` | pub(crate) | `rust/tcl-lsp-core/src/hover.rs` | `crate::hover` | `ensemble_subcommand_hover` | `static` |
| original substitution purity metadata | `rust/tcl-compiler/src/command_binding/materialized_footprint.rs` | `tcl-compiler` | `crate::command_binding::materialized_footprint` | `SourceInvocationBinding::original_logical_procedure_call_targets` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/original_procedure_call.rs` | `crate::registry_invocation::original_procedure_call` | `original_logical_procedure_calls_for_module` | `parameter-field:tokens:source_binding` |
| Original source value components | `rust/tcl-compiler/src/registry_invocation/original_word_data.rs` | `tcl-compiler` | `crate::registry_invocation::original_word_data` | `original_literal_argument_data_for_module` | pub(crate) | `rust/tcl-compiler/src/codegen/wasm/backend.rs` | `crate::codegen::wasm::backend` | `original_assignment_name` | `static` |
| Original source value components | `rust/tcl-compiler/src/registry_invocation/original_word_data.rs` | `tcl-compiler` | `crate::registry_invocation::original_word_data` | `original_literal_argument_data` | pub(crate) | `rust/tcl-compiler/src/executable_ir.rs` | `crate::executable_ir` | `lowered_source_name_reference` | `static` |
| temporal receiver navigation | `rust/tcl-compiler/src/registry_invocation/source_receiver_names.rs` | `tcl-compiler` | `crate::registry_invocation::source_receiver_names` | `original_source_receiver_name_operand` | pub(crate) | `rust/tcl-compiler/src/analyser/handlers.rs` | `crate::analyser::handlers` | `Analyser::oo_objdefine_receiver` | `static` |
| temporal receiver navigation | `rust/tcl-compiler/src/registry_invocation/source_receiver_names.rs` | `tcl-compiler` | `crate::registry_invocation::source_receiver_names` | `SourceReceiverNameOperand` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/source_receiver_names.rs` | `crate::registry_invocation::source_receiver_names` | `original_source_receiver_name_operand` | `signature-type` |
| temporal receiver navigation | `rust/tcl-compiler/src/command_binding/original_receiver_body_context.rs` | `tcl-compiler` | `crate::command_binding::original_receiver_body_context` | `OriginalReceiverBodyContext` | pub(crate) | `rust/tcl-compiler/src/command_binding.rs` | `crate::command_binding` | `entered_receiver_body_context` | `signature-type` |
| Logical object assignment and factory candidates | `rust/tcl-compiler/src/object_types/logical_source_candidates.rs` | `tcl-compiler` | `crate::object_types::logical_source_candidates` | `logical_source_unit_input` | pub(super) | `rust/tcl-compiler/src/object_types.rs` | `crate::object_types` | `logical_source_object_handle_classes` | `static` |
| invocation completion routing | `rust/tcl-compiler/src/registry_invocation.rs` | `tcl-compiler` | `crate::registry_invocation` | `original_native_source_conditional_completion` | pub(crate) | `rust/tcl-compiler/src/cfg_builder/mod.rs` | `crate::cfg_builder` | `CfgCommandClasses::conditional_source_completion_route` | `static` |
| invocation completion routing | `rust/tcl-compiler/src/cfg_builder/mod.rs` | `tcl-compiler` | `crate::cfg_builder` | `CfgCommandClasses::conditional_source_completion_route` | private | `rust/tcl-compiler/src/cfg_builder/mod.rs` | `crate::cfg_builder` | `CfgBuilder::push_plain_statement` | `field:self.command_classes` |
| selected invalid namespace creation address | `runtime/rust/src/interp/namespace_address.rs` | `tcl-runtime` | `crate::interp::namespace_address` | `Interp::namespace_creation_conflict_error` | pub(crate) | `runtime/rust/src/interp/namespace_address.rs` | `crate::interp::namespace_address` | `Interp::publish_namespace_address` | `self` |
| Original source value components | `rust/tcl-compiler/src/registry_invocation/original_word_data.rs` | `tcl-compiler` | `crate::registry_invocation::original_word_data` | `OriginalSourceArgument` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/original_word_data.rs` | `crate::registry_invocation::original_word_data` | `original_source_argument_for_module` | `signature-type` |
| Original source value components | `rust/tcl-compiler/src/registry_invocation/original_word_data.rs` | `tcl-compiler` | `crate::registry_invocation::original_word_data` | `original_source_argument_for_module` | pub(crate) | `rust/tcl-compiler/src/native_lowering/lower.rs` | `crate::native_lowering::lower` | `Lowerer::original_lowered_argument` | `static` |
| lambda invocation selection | `rust/tcl-vm/src/interp.rs` | `tcl-vm` | `crate::interp` | `Vm::namespace_operation_error` | pub(crate) | `rust/tcl-vm/src/command.rs` | `crate::command` | `lambda_namespace` | `parameter:vm` |
| lambda invocation selection | `rust/tcl-vm/src/command.rs` | `tcl-vm` | `crate::command` | `formal_parameters` | pub(crate) | `rust/tcl-vm/src/command.rs` | `crate::command` | `ProcDef::formal_parameters` | `static` |
| lambda invocation selection | `rust/tcl-vm/src/command.rs` | `tcl-vm` | `crate::command` | `formal_parameters` | pub(crate) | `rust/tcl-vm/src/exec.rs` | `crate::exec` | `Vm::preflight_original_lambda_activation` | `static` |
| lambda invocation selection | `rust/tcl-vm/src/exec.rs` | `tcl-vm` | `crate::exec` | `Vm::preflight_original_lambda_activation` | pub(crate) | `rust/tcl-vm/src/command.rs` | `crate::command` | `build_lambda_proc` | `parameter:vm` |
| scalar variable source-reference proposals | `rust/tcl-syntax/src/naming.rs` | `tcl-syntax` | `crate::naming` | `whole_scalar_variable_reference_name` | pub | `rust/tcl-syntax/src/naming/variable_spelling.rs` | `crate::naming::variable_spelling` | `scalar_source_reference` | `static` |
| Original Logical array-index candidates | `rust/tcl-syntax/src/word_rules.rs` | `tcl-syntax` | `crate::word_rules` | `standalone_static_word_fragment_value` | pub | `rust/tcl-syntax/src/naming/variable_spelling.rs` | `crate::naming::variable_spelling` | `array_source_reference` | `static` |
| Original rendered result properties | `rust/tcl-compiler/src/rendered_properties.rs` | `tcl-compiler` | `crate::rendered_properties` | `propagate_rendered_props_with_config` | pub(crate) | `rust/tcl-compiler/src/compilation_unit.rs` | `crate::compilation_unit` | `FunctionUnit::build_full` | `static` |
| Original rendered result properties | `rust/tcl-compiler/src/registry_invocation.rs` | `tcl-compiler` | `crate::registry_invocation` | `NormalRepresentationInvocation::rendered_value_traits` | pub(crate) | `rust/tcl-compiler/src/rendered_properties.rs` | `crate::rendered_properties` | `rendered_command_properties` | `parameter:normal` |
| Original source value components | `rust/tcl-compiler/src/registry_invocation/original_word_data.rs` | `tcl-compiler` | `crate::registry_invocation::original_word_data` | `OriginalSourceArgument` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/original_word_data.rs` | `crate::registry_invocation::original_word_data` | `original_source_argument_for_script` | `signature-type` |
| Original source value components | `rust/tcl-compiler/src/registry_invocation/original_word_data.rs` | `tcl-compiler` | `crate::registry_invocation::original_word_data` | `original_source_argument_for_script` | pub(crate) | `rust/tcl-compiler/src/static_loops/original_simulation.rs` | `crate::static_loops::original_simulation` | `exec_statement` | `static` |
| Original source value components | `rust/tcl-compiler/src/registry_invocation/original_word_data.rs` | `tcl-compiler` | `crate::registry_invocation::original_word_data` | `OriginalSourceArgument::scalar_reference_name` | pub(crate) | `rust/tcl-compiler/src/static_loops/original_simulation.rs` | `crate::static_loops::original_simulation` | `exec_statement` | `producer:crate::registry_invocation::original_word_data::original_source_argument_for_script` |
| native deferred Jim namespace and invocation context | `runtime/rust/src/namespace.rs` | `tcl-runtime` | `crate::namespace` | `JimNamespaceName` | private | `runtime/rust/src/namespace.rs` | `crate::namespace` | `Namespaces::retain_jim_namespace_with_name` | `signature-type` |
| native deferred Jim namespace and invocation context | `runtime/rust/src/namespace.rs` | `tcl-runtime` | `crate::namespace` | `Namespaces::retain_unmaterialized_jim_namespace` | pub(crate) | `runtime/rust/src/cmd_proc/native_jim_apply.rs` | `crate::cmd_proc::native_jim_apply` | `apply` | `static` |
| native deferred Jim namespace and invocation context | `runtime/rust/src/namespace.rs` | `tcl-runtime` | `crate::namespace` | `Namespaces::record_jim_namespace_materialized` | pub(crate) | `runtime/rust/src/interp/native_jim_namespace.rs` | `crate::interp::native_jim_namespace` | `Interp::materialize_jim_namespace` | `static` |
| native deferred Jim namespace and invocation context | `runtime/rust/src/namespace.rs` | `tcl-runtime` | `crate::namespace` | `Namespaces::remember_jim_namespace_owner` | pub(crate) | `runtime/rust/src/interp/native_compilation.rs` | `crate::interp::native_compilation` | `Interp::enter_native_procedure_frame` | `static` |
| native deferred Jim namespace and invocation context | `runtime/rust/src/interp/native_jim_namespace.rs` | `tcl-runtime` | `crate::interp::native_jim_namespace` | `Interp::materialize_jim_namespace` | pub(crate) | `runtime/rust/src/interp/native_jim_lookup.rs` | `crate::interp::native_jim_lookup` | `Interp::resolve_original_jim_command` | `self` |
| native deferred Jim namespace and invocation context | `runtime/rust/src/interp/native_jim_lookup.rs` | `tcl-runtime` | `crate::interp::native_jim_lookup` | `OriginalJimLookupNamespace` | pub(super) | `runtime/rust/src/interp/native_jim_lookup.rs` | `crate::interp::native_jim_lookup` | `Interp::resolve_original_jim_command` | `type` |
| native deferred Jim namespace and invocation context | `runtime/rust/src/interp/native_jim_lookup.rs` | `tcl-runtime` | `crate::interp::native_jim_lookup` | `OriginalJimLookupNamespace::capture` | pub(super) | `runtime/rust/src/interp/native_jim_lookup.rs` | `crate::interp::native_jim_lookup` | `Interp::resolve_original_jim_command` | `static` |
| native deferred Jim namespace and invocation context | `runtime/rust/src/interp/native_jim_lookup.rs` | `tcl-runtime` | `crate::interp::native_jim_lookup` | `OriginalJimLookupNamespace::check` | pub(super) | `runtime/rust/src/interp/native_jim_lookup.rs` | `crate::interp::native_jim_lookup` | `Interp::resolve_original_jim_command` | `static` |
| native deferred Jim namespace and invocation context | `runtime/rust/src/interp.rs` | `tcl-runtime` | `crate::interp` | `CallMeta::original_frame_arguments` | pub(crate) | `runtime/rust/src/interp/native_compilation.rs` | `crate::interp::native_compilation` | `Interp::enter_native_procedure_frame` | `static` |
| native deferred Jim namespace and invocation context | `runtime/rust/src/cmd_proc/native_jim_apply.rs` | `tcl-runtime` | `crate::cmd_proc::native_jim_apply` | `apply` | pub(super) | `runtime/rust/src/cmd_proc.rs` | `crate::cmd_proc` | `apply_cmd` | `static` |
| Original failed-operand update syntax | `rust/tcl-compiler/src/registry_invocation/source_operand_evaluation.rs` | `tcl-compiler` | `crate::registry_invocation::source_operand_evaluation` | `OriginalOperandEvaluationUpdate` | pub(crate) | `rust/tcl-compiler/src/lowering_hooks.rs` | `crate::lowering_hooks` | `try_lower_operand_evaluation_update_with_binding` | `signature-type` |
| Original failed-operand update syntax | `rust/tcl-compiler/src/registry_invocation/source_operand_evaluation.rs` | `tcl-compiler` | `crate::registry_invocation::source_operand_evaluation` | `original_operand_evaluation_update` | pub(crate) | `rust/tcl-compiler/src/lowering/mod.rs` | `crate::lowering` | `Lowerer::try_lower_original_operand_evaluation_update` | `static` |
| Original failed-operand update syntax | `rust/tcl-compiler/src/lowering_hooks.rs` | `tcl-compiler` | `crate::lowering_hooks` | `try_lower_operand_evaluation_update_with_binding` | pub(crate) | `rust/tcl-compiler/src/lowering/mod.rs` | `crate::lowering` | `Lowerer::try_lower_original_operand_evaluation_update` | `static` |
| Original refactor command geometry | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `FrameWalk::command_span_offsets` | pub(crate) | `rust/tcl-lsp-core/src/refactor/extract_proc.rs` | `crate::refactor::extract_proc` | `classify_command` | `parameter:walk` |
| native name input and reporting | `rust/tcl-syntax/src/naming/native.rs` | `tcl-syntax` | `crate::naming::native` | `NativeNameProtocol::jim_namespace_absolute_canonical_input` | pub | `rust/tcl-syntax/src/naming/native.rs` | `crate::naming::native` | `NativeNameProtocol::jim_namespace_canonical_input` | `self` |
| native name input and reporting | `rust/tcl-syntax/src/naming/native.rs` | `tcl-syntax` | `crate::naming::native` | `NativeNameProtocol::jim_namespace_command_query_candidates_from_canonical` | pub | `rust/tcl-syntax/src/naming/native.rs` | `crate::naming::native` | `NativeNameProtocol::jim_namespace_command_query_candidates` | `self` |
| native deferred Jim namespace and invocation context | `runtime/rust/src/interp/native_jim_namespace.rs` | `tcl-runtime` | `crate::interp::native_jim_namespace` | `Interp::jim_canonical_namespace_object` | pub(crate) | `runtime/rust/src/interp/native_jim_namespace.rs` | `crate::interp::native_jim_namespace` | `Interp::ns_eval_objects_with_arguments` | `self` |
| Original source handle inventories | `rust/tcl-lsp-core/src/executable_regions.rs` | `tcl-lsp-core` | `crate::executable_regions` | `visit_analysis_executable_commands` | pub(crate) | `rust/tcl-lsp-core/src/semantic_tokens.rs` | `crate::semantic_tokens` | `selected_member_handle_regions` | `static` |
| command / word segmentation | `rust/tcl-lexer/src/native_word.rs` | `tcl-lexer` | `crate::native_word` | `complete_group_span` | pub(crate) | `rust/tcl-lexer/src/parse_cut.rs` | `crate::parse_cut` | `written_span` | `static` |
| Original expression child source catalogue | `rust/tcl-compiler/src/command_binding/source_transition_advice/body_effects.rs` | `tcl-compiler` | `crate::command_binding::source_transition_advice::body_effects` | `OriginalSourceExpressionCommand` | pub(crate) | `rust/tcl-compiler/src/command_binding/source_transition_advice.rs` | `crate::command_binding::source_transition_advice` | `OriginalSourceCatalogueBody` | `type` |
| Original expression child source catalogue | `rust/tcl-compiler/src/command_binding/source_transition_advice/body_effects.rs` | `tcl-compiler` | `crate::command_binding::source_transition_advice::body_effects` | `AdviceInvocationContext::retain_original_expression_catalogue` | pub(super) | `rust/tcl-compiler/src/command_binding/source_transition_advice.rs` | `crate::command_binding::source_transition_advice` | `AdviceInvocationContext::retain_source_inventory` | `self` |
| Native source warning candidates | `rust/tcl-compiler/src/registry_invocation/source_taint.rs` | `tcl-compiler` | `crate::registry_invocation::source_taint` | `NativeSourceTaintInvocation` | pub(crate) | `rust/tcl-compiler/src/taint.rs` | `crate::taint` | `SourceWarningMetadata` | `type` |
| Native source warning candidates | `rust/tcl-compiler/src/registry_invocation/source_taint.rs` | `tcl-compiler` | `crate::registry_invocation::source_taint` | `native_source_taint_invocation` | pub(crate) | `rust/tcl-compiler/src/taint.rs` | `crate::taint` | `emit_retained_invocation_warnings` | `static` |
| Original conditional case source geometry | `rust/tcl-compiler/src/static_loops/original_simulation.rs` | `tcl-compiler` | `crate::static_loops::original_simulation` | `StaticLoopSource` | pub(crate) | `rust/tcl-compiler/src/static_loops/original_simulation.rs` | `crate::static_loops::original_simulation` | `analyse_for_statement_with_source_dependencies` | `signature-type` |
| Original conditional case source geometry | `rust/tcl-compiler/src/static_loops/original_simulation.rs` | `tcl-compiler` | `crate::static_loops::original_simulation` | `analyse_for_statement_with_source_dependencies` | pub(crate) | `rust/tcl-compiler/src/sccp/original_value_facts.rs` | `crate::sccp::original_value_facts` | `semantic_loop_summary_decision` | `static` |
| Original conditional case source geometry | `rust/tcl-compiler/src/static_loops/original_simulation.rs` | `tcl-compiler` | `crate::static_loops::original_simulation` | `summarise_for_statement_with_source_dependencies` | pub(crate) | `rust/tcl-compiler/src/sccp/original_value_facts.rs` | `crate::sccp::original_value_facts` | `loop_summary_decision` | `static` |
| Original refactor command geometry | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `FrameWalk::native_words` | pub(crate) | `rust/tcl-lsp-core/src/refactor/inline_variable.rs` | `crate::refactor::inline_variable` | `reference_context` | `parameter:walk` |
| Original refactor command geometry | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `FrameWalk::variable_references` | pub(crate) | `rust/tcl-lsp-core/src/refactor/inline_variable.rs` | `crate::refactor::inline_variable` | `reference_context` | `parameter:walk` |
| Original Jim selected command and tailcall transport | `runtime/rust/src/interp/native_jim_tailcall.rs` | `tcl-runtime` | `crate::interp::native_jim_tailcall` | `JimCommandReference` | pub(super) | `runtime/rust/src/interp/native_jim_lookup.rs` | `crate::interp::native_jim_lookup` | `Interp::retain_active_jim_command` | `signature-type` |
| Original Jim selected command and tailcall transport | `runtime/rust/src/interp/native_jim_tailcall.rs` | `tcl-runtime` | `crate::interp::native_jim_tailcall` | `JimCommandReferenceData` | pub(super) | `runtime/rust/src/interp/native_jim_tailcall.rs` | `crate::interp::native_jim_tailcall` | `Interp::capture_jim_command_reference` | `type` |
| Original Jim selected command and tailcall transport | `runtime/rust/src/interp/native_jim_tailcall.rs` | `tcl-runtime` | `crate::interp::native_jim_tailcall` | `JimSelectedTailcall` | pub(crate) | `runtime/rust/src/frame.rs` | `crate::frame` | `PendingTailcall` | `type` |
| Original Jim selected command and tailcall transport | `runtime/rust/src/frame.rs` | `tcl-runtime` | `crate::frame` | `PendingTailcall` | pub(crate) | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::dispatch_tailcall` | `signature-type` |
| Original Jim selected command and tailcall transport | `runtime/rust/src/interp/native_jim_tailcall.rs` | `tcl-runtime` | `crate::interp::native_jim_tailcall` | `JimCommandReference::check` | pub(super) | `runtime/rust/src/interp/native_jim_lookup.rs` | `crate::interp::native_jim_lookup` | `Interp::retain_active_jim_command` | `static` |
| Original Jim selected command and tailcall transport | `runtime/rust/src/interp/native_jim_tailcall.rs` | `tcl-runtime` | `crate::interp::native_jim_tailcall` | `JimCommandReference::token` | pub(super) | `runtime/rust/src/interp/native_jim_lookup.rs` | `crate::interp::native_jim_lookup` | `Interp::retain_active_jim_command` | `static` |
| Original Jim selected command and tailcall transport | `runtime/rust/src/interp/native_jim_tailcall.rs` | `tcl-runtime` | `crate::interp::native_jim_tailcall` | `JimCommandReference::command` | pub(super) | `runtime/rust/src/interp/native_jim_lookup.rs` | `crate::interp::native_jim_lookup` | `Interp::retain_active_jim_command` | `static` |
| Original Jim selected command and tailcall transport | `runtime/rust/src/interp/native_jim_tailcall.rs` | `tcl-runtime` | `crate::interp::native_jim_tailcall` | `JimCommandReference::same_reference` | pub(super) | `runtime/rust/src/interp/native_jim_lookup.rs` | `crate::interp::native_jim_lookup` | `JimActiveCommandScope::drop` | `static` |
| Original Jim selected command and tailcall transport | `runtime/rust/src/interp/native_jim_tailcall.rs` | `tcl-runtime` | `crate::interp::native_jim_tailcall` | `Interp::capture_jim_command_reference` | pub(super) | `runtime/rust/src/interp/native_jim_lookup.rs` | `crate::interp::native_jim_lookup` | `Interp::retain_active_jim_command` | `self` |
| Original Jim selected command and tailcall transport | `runtime/rust/src/interp/native_jim_tailcall.rs` | `tcl-runtime` | `crate::interp::native_jim_tailcall` | `Interp::retained_jim_command_reference` | pub(super) | `runtime/rust/src/interp/native_jim_lookup.rs` | `crate::interp::native_jim_lookup` | `Interp::resolve_original_jim_command` | `self` |
| Original Jim selected command and tailcall transport | `runtime/rust/src/interp/native_jim_lookup.rs` | `tcl-runtime` | `crate::interp::native_jim_lookup` | `Interp::retain_active_jim_command` | pub(super) | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::invoke_bound_with_jim_reference` | `self` |
| Original Jim selected command and tailcall transport | `runtime/rust/src/interp/native_jim_tailcall.rs` | `tcl-runtime` | `crate::interp::native_jim_tailcall` | `Interp::schedule_jim_tailcall` | pub(super) | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::schedule_tailcall` | `self` |
| Original Jim selected command and tailcall transport | `runtime/rust/src/interp/native_jim_tailcall.rs` | `tcl-runtime` | `crate::interp::native_jim_tailcall` | `Interp::invoke_selected_jim_tailcall` | pub(super) | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::dispatch_tailcall` | `self` |
| Original Jim selected command and tailcall transport | `runtime/rust/src/interp.rs` | `tcl-runtime` | `crate::interp` | `Interp::invoke_bound_with_jim_reference` | private | `runtime/rust/src/interp/native_jim_tailcall.rs` | `crate::interp::native_jim_tailcall` | `Interp::invoke_selected_jim_tailcall` | `self` |
| Original Jim command reset and unknown header | `runtime/rust/src/native_source.rs` | `tcl-runtime` | `crate::native_source` | `NativeJimObjectContext::unknown_object` | pub(crate) | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::prepare_missing_command` | `static` |
| Original Jim command reset and unknown header | `runtime/rust/src/interp.rs` | `tcl-runtime` | `crate::interp` | `Interp::dispatch_prebound_inner` | private | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::dispatch_selection_with_entry` | `self` |
| Original Jim command reset and unknown header | `runtime/rust/src/interp.rs` | `tcl-runtime` | `crate::interp` | `Interp::dispatch_missing_command` | private | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::dispatch_inner_at` | `self` |
| Original Jim command reset and unknown header | `runtime/rust/src/interp.rs` | `tcl-runtime` | `crate::interp` | `Interp::prepare_missing_command` | private | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::dispatch_missing_command` | `self` |
| Original refactor command geometry | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `FrameWalk::command_span_offsets` | pub(crate) | `rust/tcl-lsp-core/src/refactor/if_to_switch.rs` | `crate::refactor::if_to_switch` | `if_to_switch` | `producer:crate::refactor::FrameWalk::new` |
| Original refactor command geometry | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `FrameWalk::command_span_offsets` | pub(crate) | `rust/tcl-lsp-core/src/refactor/switch_to_dict.rs` | `crate::refactor::switch_to_dict` | `switch_to_dict` | `producer:crate::refactor::FrameWalk::new` |
| Original refactor command geometry | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `FrameWalk::command_span_offsets` | pub(crate) | `rust/tcl-lsp-core/src/refactor/datagroup.rs` | `crate::refactor::datagroup` | `extract_to_datagroup_with_analysis` | `producer:crate::refactor::FrameWalk::new` |
| Original completion argument separators | `rust/tcl-lsp-core/src/original_oo/instance_completion.rs` | `tcl-lsp-core` | `crate::original_oo::instance_completion` | `completion_gap` | pub(crate) | `rust/tcl-lsp-core/src/original_oo.rs` | `crate::original_oo` | `class_completion_context` | `static` |
| Original completion argument separators | `rust/tcl-lsp-core/src/original_oo/instance_completion.rs` | `tcl-lsp-core` | `crate::original_oo::instance_completion` | `completion_gap` | pub(crate) | `rust/tcl-lsp-core/src/original_oo.rs` | `crate::original_oo` | `method_selector_replacement` | `static` |
| Original NativeLowering descriptor queries | `rust/tcl-compiler/src/native_lowering/original_invocations.rs` | `tcl-compiler` | `crate::native_lowering::original_invocations` | `OriginalInvocations::for_module` | pub(super) | `rust/tcl-compiler/src/native_lowering/lower.rs` | `crate::native_lowering::lower` | `Lowerer::new` | `static` |
| Original NativeLowering descriptor queries | `rust/tcl-compiler/src/native_lowering/original_invocations.rs` | `tcl-compiler` | `crate::native_lowering::original_invocations` | `OriginalInvocations::command` | pub(super) | `rust/tcl-compiler/src/native_lowering/lower.rs` | `crate::native_lowering::lower` | `Lowerer::invocation_native_lowering` | `field:self.original_invocations` |
| Original NativeLowering descriptor queries | `rust/tcl-compiler/src/native_lowering/original_invocations.rs` | `tcl-compiler` | `crate::native_lowering::original_invocations` | `OriginalInvocations::substitution` | pub(super) | `rust/tcl-compiler/src/native_lowering/lower.rs` | `crate::native_lowering::lower` | `Lowerer::substitution_is_native_expression` | `field:self.original_invocations` |
| Original method caller-frame source correspondence | `rust/tcl-compiler/src/command_binding/retained_source.rs` | `tcl-compiler` | `crate::command_binding::retained_source` | `RetainedSourceModuleBindings::matches_original_method_summary` | pub(crate) | `rust/tcl-compiler/src/cfg_builder/upvar_info.rs` | `crate::cfg_builder::upvar_info` | `method_reaches_caller_frame` | `parameter-field:module:retained_source_bindings` |
| Original method caller-frame source correspondence | `rust/tcl-compiler/src/cfg_builder/upvar_info.rs` | `tcl-compiler` | `crate::cfg_builder::upvar_info` | `method_reaches_caller_frame` | pub(crate) | `rust/tcl-compiler/src/optimiser/method_barrier.rs` | `crate::optimiser::method_barrier` | `compute` | `static` |
| Native source warning candidates | `rust/tcl-compiler/src/taint.rs` | `tcl-compiler` | `crate::taint` | `option_injection_arguments` | private | `rust/tcl-compiler/src/taint.rs` | `crate::taint` | `emit_option_injection` | `static` |
| Original Logical callback source candidates | `rust/tcl-compiler/src/command_binding/source_transition_advice/logical_body.rs` | `tcl-compiler` | `crate::command_binding::source_transition_advice::logical_body` | `LogicalNamespaceScope::body_scope` | pub(super) | `rust/tcl-compiler/src/command_binding/source_transition_advice/logical_body.rs` | `crate::command_binding::source_transition_advice::logical_body` | `LogicalBodyWalk::prepare_namespace` | `producer:crate::command_binding::source_transition_advice::AdviceInvocationContext::logical_namespace_scope` |
| Original Logical callback source candidates | `rust/tcl-compiler/src/command_binding/source_transition_advice/logical_body.rs` | `tcl-compiler` | `crate::command_binding::source_transition_advice::logical_body` | `LogicalNamespaceScope::body_scope` | pub(super) | `rust/tcl-compiler/src/command_binding/source_transition_advice/body_effects.rs` | `crate::command_binding::source_transition_advice::body_effects` | `BodyWalk::prepare_region_branch` | `producer:crate::command_binding::source_transition_advice::AdviceInvocationContext::logical_namespace_scope` |
| Original Jim retiring resident-result observation | `runtime/rust/src/obj.rs` | `tcl-runtime` | `crate::obj` | `RetiringResultHeader` | pub(crate) | `runtime/rust/src/interp/native_jim_result.rs` | `crate::interp::native_jim_result` | `JimResultRetirement::enter` | `type` |
| Original Jim retiring resident-result observation | `runtime/rust/src/obj.rs` | `tcl-runtime` | `crate::obj` | `RetiringResultHeader::capture` | pub(crate) | `runtime/rust/src/interp/native_jim_result.rs` | `crate::interp::native_jim_result` | `JimResultRetirement::enter` | `static` |
| Original Jim retiring resident-result observation | `runtime/rust/src/obj.rs` | `tcl-runtime` | `crate::obj` | `RetiringResultHeader::resident_bytes` | pub(crate) | `runtime/rust/src/interp/native_jim_result.rs` | `crate::interp::native_jim_result` | `Interp::retiring_jim_result_bytes` | `static` |
| Original Jim retiring resident-result observation | `runtime/rust/src/interp/native_jim_result.rs` | `tcl-runtime` | `crate::interp::native_jim_result` | `JimResultRetirement` | pub(super) | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::set_obj_result` | `type` |
| Original Jim retiring resident-result observation | `runtime/rust/src/interp/native_jim_result.rs` | `tcl-runtime` | `crate::interp::native_jim_result` | `JimResultRetirement::enter` | pub(super) | `runtime/rust/src/interp.rs` | `crate::interp` | `Interp::set_obj_result` | `static` |
| Original Jim retiring resident-result observation | `runtime/rust/src/interp/native_jim_result.rs` | `tcl-runtime` | `crate::interp::native_jim_result` | `RetiringJimResult` | private | `runtime/rust/src/interp/native_jim_result.rs` | `crate::interp::native_jim_result` | `JimResultRetirement::enter` | `type` |
| Original Jim retiring resident-result observation | `runtime/rust/src/interp/native_jim_result.rs` | `tcl-runtime` | `crate::interp::native_jim_result` | `Interp::retiring_jim_result_bytes` | pub(crate) | `runtime/rust/src/value_ops.rs` | `crate::value_ops` | `Interp::native_object_string_bytes_with_stage` | `self` |
| Original Jim retiring resident-result observation | `runtime/rust/src/value_ops.rs` | `tcl-runtime` | `crate::value_ops` | `Interp::native_string_primary_admission` | pub(crate) | `runtime/rust/src/value_ops.rs` | `crate::value_ops` | `Interp::native_object_string_bytes_with_stage` | `self` |
| Original Jim retiring resident-result observation | `runtime/rust/src/value_ops.rs` | `tcl-runtime` | `crate::value_ops` | `Interp::native_string_primary_admission` | pub(crate) | `runtime/rust/src/interp/native_jim_result.rs` | `crate::interp::native_jim_result` | `JimResultRetirement::enter` | `static` |
| Original refactor command geometry | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `configured_command_span_offsets` | pub(crate) | `rust/tcl-lsp-core/src/refactor/mod.rs` | `crate::refactor` | `find_command_at_inner` | `static` |
| Original refactor command geometry | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `configured_command_span_offsets` | pub(crate) | `rust/tcl-lsp-core/src/refactor/datagroup.rs` | `crate::refactor::datagroup` | `extract_to_datagroup_from_if` | `static` |
| Original refactor command geometry | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `configured_command_span_offsets` | pub(crate) | `rust/tcl-lsp-core/src/refactor/datagroup.rs` | `crate::refactor::datagroup` | `extract_to_datagroup_from_switch` | `static` |
| Original completion argument separators | `rust/tcl-lsp-core/src/original_oo/instance_completion.rs` | `tcl-lsp-core` | `crate::original_oo::instance_completion` | `completion_gap` | pub(crate) | `rust/tcl-lsp-core/src/completion/original_context.rs` | `crate::completion::original_context` | `select` | `static` |
| Original autoload index filename data | `rust/tcl-lsp-core/src/package_resolver/original_names/index_loader.rs` | `tcl-lsp-core` | `crate::package_resolver::original_names::index_loader` | `IndexLoader::new` | pub(super) | `rust/tcl-lsp-core/src/package_resolver/original_names.rs` | `crate::package_resolver::original_names` | `original_index_entries` | `static` |
| Original autoload index filename data | `rust/tcl-lsp-core/src/package_resolver/original_names/index_loader.rs` | `tcl-lsp-core` | `crate::package_resolver::original_names::index_loader` | `IndexLoader::filename` | pub(in crate::package_resolver) | `rust/tcl-lsp-core/src/package_resolver/original_names.rs` | `crate::package_resolver::original_names` | `original_index_entries` | `producer:crate::package_resolver::original_names::index_loader::IndexLoader::new` |
| Original authored corpus scalar evidence | `rust/tcl-spec-studio/src/corpus.rs` | `tcl-spec-studio` | `crate::corpus` | `Stmt::scalar_read` | private | `rust/tcl-spec-studio/src/corpus.rs` | `crate::corpus` | `switched_variable` | `parameter:stmt` |
| complete retained Module source correspondence | `rust/tcl-compiler/src/registry_invocation/source_structure.rs` | `tcl-compiler` | `crate::registry_invocation::source_structure` | `original_registry_words_for_module` | pub(crate) | `rust/tcl-compiler/src/analyser/class_lattice.rs` | `crate::analyser::class_lattice` | `collect_assign_kinds` | `static` |
| complete retained Module source correspondence | `rust/tcl-compiler/src/registry_invocation/source_structure.rs` | `tcl-compiler` | `crate::registry_invocation::source_structure` | `original_registry_words_for_module` | pub(crate) | `rust/tcl-compiler/src/analyser/diagnostics/var_command.rs` | `crate::analyser::diagnostics::var_command` | `original_source_return_operand` | `static` |
| Current editor source inventory | `rust/tcl-lsp-core/src/original_context.rs` | `tcl-lsp-core` | `crate::original_context` | `CurrentSourceContext::capture` | pub(crate) | `rust/tcl-lsp-core/src/rename.rs` | `crate::rename` | `prepare_rename_in_program` | `static` |
| Current editor source inventory | `rust/tcl-lsp-core/src/original_context.rs` | `tcl-lsp-core` | `crate::original_context` | `CurrentSourceContext::capture` | pub(crate) | `rust/tcl-lsp-core/src/rename.rs` | `crate::rename` | `rename_in_program` | `static` |
| Current editor source inventory | `rust/tcl-lsp-core/src/original_context.rs` | `tcl-lsp-core` | `crate::original_context` | `CurrentSourceContext::capture` | pub(crate) | `rust/tcl-lsp-core/src/rename.rs` | `crate::rename` | `method_target_with_access_in_workspace` | `static` |
| Current editor source inventory | `rust/tcl-lsp-core/src/original_context.rs` | `tcl-lsp-core` | `crate::original_context` | `CurrentSourceContext::capture` | pub(crate) | `rust/tcl-lsp-core/src/receiver_identity.rs` | `crate::receiver_identity` | `method_at_cursor` | `static` |
| Current editor source inventory | `rust/tcl-lsp-core/src/original_context.rs` | `tcl-lsp-core` | `crate::original_context` | `CurrentSourceContext::capture` | pub(crate) | `rust/tcl-lsp-core/src/references.rs` | `crate::references` | `references_in_program` | `static` |
| Current editor source inventory | `rust/tcl-lsp-core/src/original_context.rs` | `tcl-lsp-core` | `crate::original_context` | `CurrentSourceContext::capture` | pub(crate) | `rust/tcl-lsp-core/src/references.rs` | `crate::references` | `document_highlights_in_program` | `static` |
| Explicit Standalone source authoring ingress | `rust/tcl-compiler/src/lowering/mod.rs` | `tcl-compiler` | `crate::lowering` | `Lowerer::for_standalone_source` | pub(crate) | `rust/tcl-compiler/src/compilation_unit.rs` | `crate::compilation_unit` | `lower_and_build_cfg` | `static` |
| Original Jim existence queries | `runtime/rust/src/cmd_jim_exists.rs` | `tcl-runtime` | `crate::cmd_jim_exists` | `install` | pub(crate) | `runtime/rust/src/builtins.rs` | `crate::builtins` | `install_for_bootstrap` | `static` |
| Original Jim existence queries | `runtime/rust/src/cmd_jim_exists.rs` | `tcl-runtime` | `crate::cmd_jim_exists` | `exists_original` | private | `runtime/rust/src/cmd_jim_exists.rs` | `crate::cmd_jim_exists` | `exists` | `static` |
| Original Jim existence queries | `runtime/rust/src/interp/native_jim_lookup.rs` | `tcl-runtime` | `crate::interp::native_jim_lookup` | `Interp::exists_original_jim_variable` | pub(crate) | `runtime/rust/src/cmd_jim_exists.rs` | `crate::cmd_jim_exists` | `exists_original` | `parameter:interpreter` |
| Original Jim existence queries | `runtime/rust/src/interp/native_jim_lookup.rs` | `tcl-runtime` | `crate::interp::native_jim_lookup` | `Interp::read_original_named_variable_reported` | pub(super) | `runtime/rust/src/interp/native_jim_lookup.rs` | `crate::interp::native_jim_lookup` | `Interp::exists_original_jim_variable` | `self` |
| Original Jim existence queries | `runtime/rust/src/interp/native_jim_lookup.rs` | `tcl-runtime` | `crate::interp::native_jim_lookup` | `OriginalJimVariableReport` | pub(super) | `runtime/rust/src/interp/native_jim_lookup.rs` | `crate::interp::native_jim_lookup` | `Interp::read_original_named_variable_reported` | `signature-type` |
| Original Jim existence queries | `runtime/rust/src/interp/native_jim_lookup.rs` | `tcl-runtime` | `crate::interp::native_jim_lookup` | `OriginalJimVariableReport` | pub(super) | `runtime/rust/src/interp/native_jim_links.rs` | `crate::interp::native_jim_links` | `Interp::read_original_jim_link` | `signature-type` |
| Original autoload index filename data | `rust/tcl-lsp-core/src/package_resolver/original_names/index_loader.rs` | `tcl-lsp-core` | `crate::package_resolver::original_names::index_loader` | `IndexLoader::standalone` | pub(in crate::package_resolver) | `rust/tcl-lsp-core/src/package_resolver.rs` | `crate::package_resolver` | `parse_tcl_index` | `static` |
| Original autoload index filename data | `rust/tcl-lsp-core/src/package_resolver/original_names/index_loader.rs` | `tcl-lsp-core` | `crate::package_resolver::original_names::index_loader` | `IndexLoader::standalone` | pub(in crate::package_resolver) | `rust/tcl-lsp-core/src/package_resolver.rs` | `crate::package_resolver` | `collect_source_targets` | `static` |
| Original autoload index filename data | `rust/tcl-lsp-core/src/package_resolver/original_names/index_loader.rs` | `tcl-lsp-core` | `crate::package_resolver::original_names::index_loader` | `IndexLoader::filename` | pub(in crate::package_resolver) | `rust/tcl-lsp-core/src/package_resolver.rs` | `crate::package_resolver` | `parse_tcl_index` | `producer:crate::package_resolver::original_names::index_loader::IndexLoader::standalone` |
| Original autoload index filename data | `rust/tcl-lsp-core/src/package_resolver/original_names/index_loader.rs` | `tcl-lsp-core` | `crate::package_resolver::original_names::index_loader` | `IndexLoader::collect_word` | pub(in crate::package_resolver) | `rust/tcl-lsp-core/src/package_resolver.rs` | `crate::package_resolver` | `collect_source_targets` | `producer:crate::package_resolver::original_names::index_loader::IndexLoader::standalone` |
| Original declaration body content | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `FrameWalk::native_words` | pub(crate) | `rust/tcl-lsp-core/src/declaration.rs` | `crate::declaration` | `original_declaration_body_content` | `producer:crate::refactor::FrameWalk::new` |
| Original declaration body content | `rust/tcl-lsp-core/src/declaration.rs` | `tcl-lsp-core` | `crate::declaration` | `original_declaration_body_content` | private | `rust/tcl-lsp-core/src/declaration.rs` | `crate::declaration` | `original_declaration_region_commands` | `static` |
| Original variable reference tail proposals | `rust/tcl-syntax/src/naming.rs` | `tcl-syntax` | `crate::naming` | `variable_reference_root_bytes` | pub | `rust/tcl-syntax/src/naming/variable_spelling.rs` | `crate::naming::variable_spelling` | `variable_source_reference_with_tail` | `static` |
| Original variable reference tail proposals | `rust/tcl-syntax/src/naming.rs` | `tcl-syntax` | `crate::naming` | `written_command_tail` | pub | `rust/tcl-syntax/src/naming/variable_spelling.rs` | `crate::naming::variable_spelling` | `variable_source_reference_with_tail` | `static` |
| Original Logical method procedure candidates | `rust/tcl-lsp-core/src/call_hierarchy.rs` | `tcl-lsp-core` | `crate::call_hierarchy` | `logical_method_procedure_at` | private | `rust/tcl-lsp-core/src/call_hierarchy.rs` | `crate::call_hierarchy` | `method_outgoing_calls` | `static` |
| Original Logical method procedure candidates | `rust/tcl-lsp-core/src/call_hierarchy.rs` | `tcl-lsp-core` | `crate::call_hierarchy` | `logical_method_procedure_at` | private | `rust/tcl-lsp-core/src/call_hierarchy.rs` | `crate::call_hierarchy` | `unresolved_method_outgoing_calls` | `static` |
| Original refactor command geometry | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `configured_command_words` | pub(crate) | `rust/tcl-lsp-core/src/refactor/mod.rs` | `crate::refactor` | `configured_command_span_offsets` | `static` |
| Original refactor command geometry | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `configured_command_words` | pub(crate) | `rust/tcl-lsp-core/src/refactor/switch_to_dict.rs` | `crate::refactor::switch_to_dict` | `parse_branch_assignment` | `static` |
| Original refactor command geometry | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `configured_command_words` | pub(crate) | `rust/tcl-lsp-core/src/refactor/datagroup.rs` | `crate::refactor::datagroup` | `parse_set_or_return` | `static` |
| Source-owned analysis cache correspondence | `rust/tcl-compiler/src/command_binding/analysis_body_correspondence.rs` | `tcl-compiler` | `crate::command_binding::analysis_body_correspondence` | `ModuleCommandBindings::prepare_authored_source_body_correspondence` | pub(crate) | `rust/tcl-compiler/src/body_analysis_capsule.rs` | `crate::body_analysis_capsule` | `analytical_closed_projection` | `producer:crate::command_binding::ModuleCommandBindings::analyse` |
| Source-owned analysis cache correspondence | `rust/tcl-compiler/src/command_binding/analysis_body_correspondence.rs` | `tcl-compiler` | `crate::command_binding::analysis_body_correspondence` | `ModuleCommandBindings::prepare_authored_source_body_correspondence` | pub(crate) | `rust/tcl-compiler/src/body_analysis_capsule.rs` | `crate::body_analysis_capsule` | `BodyLatticeInputs::retain_authored_source` | `parameter-field:request:command_bindings` |
| Original authored corpus scalar evidence | `rust/tcl-spec-studio/src/infer.rs` | `tcl-spec-studio` | `crate::infer` | `original_body_data` | private | `rust/tcl-spec-studio/src/infer.rs` | `crate::infer` | `import_package` | `static` |
| Original authored corpus scalar evidence | `rust/tcl-spec-studio/src/corpus.rs` | `tcl-spec-studio` | `crate::corpus` | `scan_in` | pub(crate) | `rust/tcl-spec-studio/src/infer.rs` | `crate::infer` | `draft_for_proc` | `static` |
| Original Jim namespace scripted helpers | `runtime/rust/src/interp/stock_ensembles.rs` | `tcl-runtime` | `crate::interp::stock_ensembles` | `Interp::install_stock_jim_namespace_helpers` | pub(crate) | `runtime/rust/src/cmd_namespace.rs` | `crate::cmd_namespace` | `install` | `parameter:interp` |
| Original Jim namespace scripted helpers | `runtime/rust/src/interp/stock_ensembles.rs` | `tcl-runtime` | `crate::interp::stock_ensembles` | `Interp::install_stock_jim_namespace_helper` | private | `runtime/rust/src/interp/stock_ensembles.rs` | `crate::interp::stock_ensembles` | `Interp::install_stock_jim_namespace_helpers` | `self` |
| Original Jim namespace scripted helpers | `runtime/rust/src/interp/stock_ensembles.rs` | `tcl-runtime` | `crate::interp::stock_ensembles` | `Interp::dispatch_stock_jim_namespace_helper` | pub(crate) | `runtime/rust/src/cmd_namespace.rs` | `crate::cmd_namespace` | `ns_which` | `parameter:interp` |
| Original Jim namespace scripted helpers | `runtime/rust/src/interp/stock_ensembles.rs` | `tcl-runtime` | `crate::interp::stock_ensembles` | `Interp::dispatch_stock_jim_namespace_helper` | pub(crate) | `runtime/rust/src/cmd_namespace.rs` | `crate::cmd_namespace` | `ns_code` | `parameter:interp` |
| native command-source error logging | `runtime/rust/src/interp/native_object_vector.rs` | `tcl-runtime` | `crate::interp::native_object_vector` | `Interp::eval_original_object_vector` | pub | `runtime/rust/src/cmd_regex.rs` | `crate::cmd_regex` | `invoke_jim_match_command` | `parameter:interp` |
| native command-source error logging | `runtime/rust/src/interp/native_arguments.rs` | `tcl-runtime` | `crate::interp::native_arguments` | `OriginalArguments::enter` | pub(crate) | `runtime/rust/src/interp/native_object_vector.rs` | `crate::interp::native_object_vector` | `Interp::eval_original_object_vector` | `static` |
| Expanded list scalar source words | `rust/tcl-compiler/src/codegen/values.rs` | `tcl-compiler` | `crate::codegen::values` | `CodegenCtx::emit_variable_reference` | pub(crate) | `rust/tcl-compiler/src/codegen/cmd_subst.rs` | `crate::codegen::cmd_subst` | `CodegenCtx::try_list_expand_concat` | `self` |
| Original expression child source catalogue | `rust/tcl-compiler/src/command_binding/source_transition_advice.rs` | `tcl-compiler` | `crate::command_binding::source_transition_advice` | `OriginalSourceCatalogueBody` | pub(crate) | `rust/tcl-compiler/src/command_binding/source_transition_advice/body_effects.rs` | `crate::command_binding::source_transition_advice::body_effects` | `AdviceInvocationContext::retain_original_body_command_catalogue` | `signature-type` |
| Original refactor command geometry | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `configured_command_words` | pub(crate) | `rust/tcl-lsp-core/src/refactor/mod.rs` | `crate::refactor` | `body_words` | `static` |
| Original refactor command geometry | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `body_words` | private | `rust/tcl-lsp-core/src/refactor/mod.rs` | `crate::refactor` | `walk_commands_inner` | `static` |
| Original refactor command geometry | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `body_words` | private | `rust/tcl-lsp-core/src/refactor/mod.rs` | `crate::refactor` | `find_command_at_inner` | `static` |
| Decoded refactor script indentation | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `reindent_body` | pub | `rust/tcl-lsp-core/src/refactor/if_to_switch.rs` | `crate::refactor::if_to_switch` | `render_switch` | `static` |
| Decoded refactor script indentation | `rust/tcl-lsp-core/src/refactor/mod.rs` | `tcl-lsp-core` | `crate::refactor` | `reindent_body` | pub | `rust/tcl-lsp-core/src/refactor/datagroup.rs` | `crate::refactor::datagroup` | `membership_extraction` | `static` |
| Original method hierarchy presentation | `rust/tcl-lsp-core/src/original_declaration.rs` | `tcl-lsp-core` | `crate::original_declaration` | `OriginalDeclarationIdentity::reporting_key` | pub(crate) | `rust/tcl-lsp-core/src/original_call_hierarchy.rs` | `crate::original_call_hierarchy` | `item` | `parameter:identity` |
| Original caller-template literal name data | `rust/tcl-compiler/src/registry_invocation.rs` | `tcl-compiler` | `crate::registry_invocation` | `InvocationBodyAssistance::caller_template_literal_data` | pub(crate) | `rust/tcl-compiler/src/analyser/param_traits.rs` | `crate::analyser::param_traits` | `written_template_literal` | `parameter:view` |
| Original caller-template literal name data | `rust/tcl-compiler/src/registry_invocation.rs` | `tcl-compiler` | `crate::registry_invocation` | `InvocationBodyAssistance::caller_template_literal_data` | pub(crate) | `rust/tcl-compiler/src/analyser/param_traits.rs` | `crate::analyser::param_traits` | `record_selected_aliases` | `parameter:view` |
| Original caller-template literal name data | `rust/tcl-compiler/src/registry_invocation.rs` | `tcl-compiler` | `crate::registry_invocation` | `InvocationBodyAssistance::caller_template_literal_data` | pub(crate) | `rust/tcl-compiler/src/analyser/param_traits.rs` | `crate::analyser::param_traits` | `apply_definite_writes` | `parameter:view` |
| Explicit Standalone source authoring ingress | `rust/tcl-compiler/src/lowering/mod.rs` | `tcl-compiler` | `crate::lowering` | `Lowerer::for_standalone_source` | pub(crate) | `rust/tcl-compiler/src/lowering/mod.rs` | `crate::lowering` | `lower_proc_body_isolated` | `static` |
| Original Jim namespace scripted helpers | `runtime/rust/src/interp/stock_ensembles.rs` | `tcl-runtime` | `crate::interp::stock_ensembles` | `Interp::dispatch_stock_jim_namespace_helper` | pub(crate) | `runtime/rust/src/cmd_namespace.rs` | `crate::cmd_namespace` | `ns_origin` | `parameter:interp` |
| Original procedure report labels | `rust/tcl-compiler/src/ir.rs` | `tcl-compiler` | `crate::ir` | `Module::original_procedure_report_label` | pub | `rust/tcl-compiler/src/interprocedural.rs` | `crate::interprocedural` | `materialise_summaries` | `parameter:ir_module` |
| Original Logical body-word geometry | `rust/tcl-compiler/src/command_binding/source_transition_advice/logical_word_geometry.rs` | `tcl-compiler` | `crate::command_binding::source_transition_advice::logical_word_geometry` | `OriginalLogicalBodyWords` | pub(super) | `rust/tcl-compiler/src/command_binding/source_transition_advice.rs` | `crate::command_binding::source_transition_advice` | `OriginalSourceTransitionAdviceTape` | `type` |
| Original Logical body-word geometry | `rust/tcl-compiler/src/command_binding/source_transition_advice/logical_word_geometry.rs` | `tcl-compiler` | `crate::command_binding::source_transition_advice::logical_word_geometry` | `OriginalLogicalBodyWords::capture` | private | `rust/tcl-compiler/src/command_binding/source_transition_advice/logical_word_geometry.rs` | `crate::command_binding::source_transition_advice::logical_word_geometry` | `OriginalSourceTransitionAdviceTape::retain_logical_body_words` | `static` |
| Original Logical body-word geometry | `rust/tcl-compiler/src/command_binding/source_transition_advice/logical_word_geometry.rs` | `tcl-compiler` | `crate::command_binding::source_transition_advice::logical_word_geometry` | `OriginalSourceTransitionAdviceTape::logical_body_original_word_at_span` | pub(super) | `rust/tcl-compiler/src/command_binding/source_transition_advice.rs` | `crate::command_binding::source_transition_advice` | `OriginalSourceTransitionAdviceTape::logical_original_word_at_span` | `self` |
| Conditional original source semantic metadata | `rust/tcl-compiler/src/semantic_analysis/source_metadata.rs` | `tcl-compiler` | `crate::semantic_analysis::source_metadata` | `ConditionalSourceMetadata::build` | pub(super) | `rust/tcl-compiler/src/semantic_analysis.rs` | `crate::semantic_analysis` | `SemanticAnalysisBundle::build_with_metadata_context` | `static` |
| Conditional original source semantic metadata | `rust/tcl-compiler/src/semantic_analysis/source_metadata.rs` | `tcl-compiler` | `crate::semantic_analysis::source_metadata` | `ConditionalSourceMetadata::build` | pub(super) | `rust/tcl-compiler/src/semantic_analysis.rs` | `crate::semantic_analysis` | `SemanticAnalysisBundle::build_for_interactive_analysis_with_metadata_context` | `static` |
| Conditional original source semantic metadata | `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs` | `tcl-compiler` | `crate::registry_invocation::source_name_roles` | `original_source_metadata_name_roles` | pub(crate) | `rust/tcl-compiler/src/semantic_analysis/source_metadata.rs` | `crate::semantic_analysis::source_metadata` | `ConditionalSourceInvocationMetadata::capture` | `static` |
| Conditional original source semantic metadata | `rust/tcl-compiler/src/command_binding.rs` | `tcl-compiler` | `crate::command_binding` | `SourceInvocationBinding::has_nonpoint_original_source_name_role_owner` | pub(crate) | `rust/tcl-compiler/src/registry_invocation/source_name_roles.rs` | `crate::registry_invocation::source_name_roles` | `original_source_metadata_name_roles` | `parameter-field:tokens:source_binding` |
| authored naming policy and signature publication | `rust/tcl-compiler/src/signature_scan/original_name.rs` | `tcl-compiler` | `crate::signature_scan::original_name` | `SourceNamespacePattern::matched_imported_command_tail` | pub | `rust/tcl-compiler/src/signature_scan/original_name.rs` | `crate::signature_scan::original_name::original_import_publication_tests` | `actual_import_publications_keep_selected_search_and_original_tail` | `static` |
<!-- end-owner-resolution-source-manifest -->


### Compiler evaluated substitutions

The compiler's crate-internal `ir_helpers::EvaluatedCommandSubstitutions`
combines the lexical owners above with registry expression roles. Its
`all_commands` view is one evaluation-ordered stream across ordinary and
in-frame expression commands: nested invocations precede their enclosing
command and later sibling words. Binding-transition and scalar-barrier
consumers share this stream; call-graph consumers retain the separate ordinary
inventory. The compiler library's
`complete_substitution_inventory_preserves_expression_sibling_order` regression
checks both ordering and classification, and `compiler_analysis_residual` checks
the resulting scalar proof against the Tcl rename/eval witnesses.
`command_binding::ModuleCommandBindings::resolved_embedded_head` supplies the
shared registry target and prepended arguments for this expression descent in
both CFG projection and binding replay, including aliases to expression commands.
The inventory walker accepts an invocation observer alongside this resolver;
discovery and replay advance together so an earlier substitution can introduce
the alias whose expression words a later sibling evaluates.

The following owners are internal implementation seams. Their names do not
claim a public API; their source paths and semantic limits remain explicit.

| Internal owner | Source paths | Internal entry points | Semantic axis |
| --- | --- | --- | --- |
| entered expression operands | `rust/tcl-compiler/src/word_subst.rs` | `entered_expression_evaluations`, `EnteredExpressionEvaluation` | original source instance and unchanged operand bytes, retained child handler, reached dispatch and logical expression grammar/number policy; entered child diagnostic/read evidence survives an absent parent dispatch; missing, replaced, unentered or misaligned child evidence declines, with no parent completion or erasure authority |
| native expression source objects | `rust/tcl-vm/src/value.rs`; `rust/tcl-vm/src/interp.rs` | `prepare_expression_value`, `expression_source`, `cached_expression`, `cache_expression`, `cache_integer_representation`, `cache_double_representation`, `existing_string_representation` | original shared value, lazy source string generation, parse-time representation replacement including errors, reached operand coercion preserving original bytes; full source profile and native invocation grammar equality before AST reuse; compile admission only inspects existing bytes; internal implementation: `prepare_expression_value`, `expression_source`, `cached_expression`, `cache_expression`, `cache_integer_representation`, `cache_double_representation`, `existing_string_representation` |
| VM captured variable updates | `rust/tcl-vm/src/interp.rs`; `rust/tcl-vm/src/vars.rs` | `lappend_list_update`, `with_variable_operation`, `VarArena::retain_operation`, `VarArena::release_operation` | list-update opcodes capture the reached cell and parent before read callbacks; operation references retain native binding shells; existing empty appends validate without stores and deleted parent elements cannot redirect writes to a replacement; internal implementation: `lappend_list_update`, `with_variable_operation`, `VarArena::retain_operation`, `VarArena::release_operation` |


### `tcl-dialect` + `tcl-test-support` — C Tcl reference toolchains

- The `tcl-dialect/data/reference-toolchains.tsv` manifest is the
  language-neutral owner for the five pinned C Tcl patchlevels and their
  upstream Tcl/Tk source tags. `tcl-dialect` generates `TclVersion`'s release
  facts from it at build time; `tcl-test-support` reuses those APIs for oracle
  provenance, while the POSIX shell adapter under `scripts/dev` supplies the
  same rows to ensure-test-deps, the source-fetch skill, and remote-session
  bootstrap. `tcl-pkg` reads the same pins for the source-build layers of a
  generated Dockerfile, so a pin bump reaches user-facing output and not only
  test oracles and dev-host bootstrap.
- Complete reference-engine tests use `required_tclshs` and
  `require_jimsh`, never optional discovery. `TCL_LSP_REQUIRE_ALL_TCL_ORACLES`
  makes `available_tclshs` consumers fail on missing or invalid C interpreters.
  `TCL_LSP_REQUIRE_JIM_ORACLE` requires the Jim interpreter. The dedicated
  `run-resolution-oracles.sh` sets both; missing binaries are failures. It
  schedules Registry's exact 8.6/9.0 vocabulary differential, live five-release
  package comparison/lookup and Jim unsupported-surface controls, the hermetic
  package-version corpus, and Runtime's source-built 9.0.4 array-trace suite.
  Runtime selects its actual profile and host before core registration; strict
  source-built startup cannot skip a missing interpreter or numeric tower.
  Vocabulary observations do not attest installed compiler or handler identity.
- Jim build provenance comes from `rust/tcl-test-support/jim-reference.txt`,
  consumed by discovery and `scripts/dev/ensure-jim-oracle.sh`. A requested
  current-Jim test run requires the revision-bearing reported patchlevel; optional
  Jim discovery records whatever binary it validates. `JimCapability` runs the
  actual required operation, because extensions are configurable and Jim's
  `interp` is not C Tcl's child-interpreter API. Capability probes do not grant
  production semantics; per-dialect descriptors still own those contracts.
- Default/PATH oracle resolution requires the exact pinned patchlevel and
  records the interpreter's reported value as provenance. An explicitly
  paired source-tree interpreter may name another patchlevel on the same
  release line, but its binary and `generic/tcl.h` must agree exactly.
- The output witnesses — the programs the value-transfer tests run before and
  after `tcl opt`, in `tcl-compiler`'s `value_transfer_witnesses.rs`,
  `tcl-registry`'s `differential_fold.rs` and `tcl-cli`'s
  `value_transfers_cli.rs` — find each release's interpreter through one
  helper, `witness_tclsh`, over `locate_tclsh`. A release it does not find
  fails the test when `TCL_LSP_REQUIRE_TCLSH` names it (a comma-separated
  list, `9.0` or `8.6,9.0`, or any value naming none for every release), and
  is otherwise reported as skipped on standard error, past the test harness's
  capture, so a run that compared under fewer releases says so. CI's test job
  requires the 9.0 interpreter it installs.
- `ParsedVersion::parse` owns package-version grammar. The versionless public
  helpers (`validate_version`, `validate_requirement`, `compare_versions`,
  `version_satisfies`, and provider selection) use the strict Tcl 8 grammar
  and are for static, release-agnostic consumers. Runtime package commands
  must pass their pinned `TclVersion` through the corresponding `*_for` API:
  Tcl 9 accepts a `+` suffix and compares the numeric version prefix while Tcl
  8 rejects it. Exact
  package requests use the explicit exact-match/selection helpers rather than
  reconstructing a `version-version` range, because a Tcl 9 suffix may itself
  contain a dash. Checked validation remains the command boundary; comparison
  and provider selection apply the same release policy at lookup time, even
  after an interpreter profile is re-pinned.
- The check-tcl-reference-toolchains Make target runs the hermetic
  stale-interpreter regression (including `/bin/sh` adapter execution) and the
  Rust all-axis release-fact test.

### `tcl-platform` — predefined platform surface

- `bootstrap::entries` is the one schema for the predefined
  `tcl_platform` array in both interpreters. Constants live in the schema;
  machine, user, operating-system version, engine identity, and backend facts
  enter through `bootstrap::Values`. `bootstrap::snapshot` captures those
  values together with the selected host's environment and Tcl library path,
  so neither engine may read the process host behind an embedder's selected
  `Host`.
- `bootstrap::HOST_ARRAYS` and `bootstrap::HOST_PATH_GLOBALS` define the whole
  stale surface an engine clears before installing a replacement snapshot.
  `Interp::with_host` avoids a native-host bootstrap entirely; both engines'
  `set_host` paths replace this surface, and normal children inherit their
  parent's host before their first bootstrap.
- `bootstrap::safe_scrub_keys` is derived from those same entries. It follows
  Tcl 9's `Tcl_MakeSafe` distinction: identity-bearing `os`, `osVersion`,
  `machine`, and `user` are removed; portable facts, including `threaded`,
  remain. The project-specific runtime/WASM/WASI/eBPF facts are also removed.
- `bootstrap::SHARED_LIBRARY_EXTENSION` is the one suffix exposed by both
  engines' `info sharedlibextension` implementations. It belongs beside the
  canonical Unix `tcl_platform(platform)` fact rather than in either engine's
  command adapter; real `init.tcl` package-index discovery reads it while
  rejecting Windows-only packages.

  Consumers: `tcl-vm::Vm::rebootstrap_host_globals` and
  `tcl_runtime::Interp::rebootstrap_host_globals` install the snapshot;
  each engine's `make_safe` consumes the derived scrub iterator. A fresh
  tree-walk `Interp`, its normal children, and bytecode-VM children all install
  the surface before any `init.tcl` work.

### Original-object return preparation

`InvocationDialect::return_options_protocol()` selects actual C Tcl or Jim
behavior independently of source lexer grammar. An authored logical F5 return
provider is a separate capability and grants no native engine evidence.

Adapters implement `ReturnOptionsOps` over original object references. String
and list operations use the selected native object protocol. Integer probes
use `native_scalar_probe`: they apply reached cache changes without formatting
or publishing a guest error. C completion codes and levels use the primitive
Int getter; Jim uses Long. A failed probe preserves host refusal separately
from the native conversion outcome.

`prepare_return` accepts head-stripped original argv and an explicit purpose.
Modern C merges options before converting the selected final code and level;
Tcl 8.4 and Jim convert each reached pair sequentially. Original custom keys
retain their complete bytes and object identity. C's return code normalization
and Jim's return-code cache are independently selected. A Jim return-code object
without resident bytes has no string updater; checked spelling access refuses
that unsupported storage operation.

`prepare_option_pairs` validates an original dictionary/list object without
reconstructing its members. `prepare_prefix_error_options` has a separate
purpose: C checks list cardinality without reading member keys, while Jim
checks the original byte cardinality. Prefix applies return conversion only
on the actual miss. C appends its forced error code after caller pairs; Jim
places its forced arguments before them. Bytecode return instructions use
internal dictionary ingestion rather than borrowing user argv grammar.

The Runtime retains original carried keys and values across return, saved
interpreter state, coroutine and child boundaries. Completion-option synthesis
preserves those objects while replacing normalized code and level. Live error
information that the interpreter extends is materialized from its error-state
owner. The Runtime refuses a negative wrapped native return level because its
boundary counter stores nonnegative levels; the VM retains the signed level.

Namespace child queries use `children_tokens_checked` before original result
production or reporting. Supply an independently selected name policy and an
actual parent token. `namespace_children_lookup` selects the physical query:
Tcl 8.4 scans, Tcl 8.5 probes the parent's child table with its original raw
suffix, and later C releases resolve an exact full name and check its parent.
Wildcard scans match opaque native CString bytes. Implement
`find_namespace_child_bytes_checked` against the actual child table; splitting a
reported full name cannot recover that member key or authenticate a token.
Missing policy or physical lookup capability is a typed refusal.

### `tcl-runtime-api` — completion metadata

- `completion_options::plan` owns the standard Tcl return-options overlay.
  `plan_with_origin` also accepts `CompletionOptionOrigin`: actual carried
  dictionaries preserve insertion order, while `ErrorMetadata` transports
  private primitive fields without installing a native return dictionary.
  Use `Completion::new_error_metadata` for primitive failures and
  `Completion::new` for actual carried options; propagate an existing
  completion without rebuilding it.
  A default private `errorCode` is not an explicit carried option. Return
  preparation retains its original extra pairs; the snapshot planner adds
  live private fields after the control keys. Primitive error transport and
  merged return dictionaries therefore keep distinct provenance and order.
  Engines supply their concrete values and live error metadata; the owner
  preserves carried custom and explicit return options, replaces `-code` and
  `-level` with their settled values, and release-gates only the synthesis of
  TIP 348 `-errorstack`. A carried option named `-errorstack` remains an
  ordinary custom pair on Tcl 8.4/8.5 and must not be deleted or validated as
  TIP 348 metadata there.
- `completion_options::retained_array_read_options` owns the carried metadata
  for a candidate read that `array get` skips. Both runtime adapters materialise
  the typed `ArrayReadMiss` into their value model and pass the pairs through
  `plan`; neither assembles a private return-options dictionary.
- `completion_options::ControlOptionPolicy` owns control-command boundaries as
  two independent axes: whether a nested body activation begins with inherited
  or fresh options, and whether the successful owner forwards that body's
  options or settles an ordinary option set. Both runtime engines consume this
  vocabulary; bytecode carries the activation scope as instruction metadata
  instead of recognising command names in the executor. Alias trampolines use
  the same fresh/forwarded policy, including when they cross interpreters, so
  target-produced options cross the wrapper but caller options do not leak in.
- `error_stack::ErrorStack` owns TIP 348's flat tag/value shape, lazy reset,
  explicit-stack adoption, and reached command-log frame entries. `CALL` is
  recorded while the procedure is active, including errors caught inside it;
  a procedure exit must not append a duplicate entry. A redirected evaluation
  retains the original execution frame's invocation role and native level
  delta. An empty-invocation special frame logs no entry even when its
  variable frame is redirected; the selected target's invocation cannot
  supply that role. The native VM
  and portable runtime render their own value types but do not reproduce that
  lifecycle. `TclVersion::has_error_stack` is the release fact: Tcl 8.6 and
  later expose it; older and vendor profiles inherit their selected runtime.

### `tcl-runtime-api` + Runtime + VM — checked completion transport

- `Commands::dispatch` returns
  `Result<Completion<Value>, NativeExecutionError>`. Every Guest code remains
  an `Ok` transport; first Host refusal is `Err`, independent of command-id
  validity. Runtime and VM are the actual implementations. The VM list-sort
  callback propagates the typed first cause through its existing internal
  unwind owner instead of rendering a Guest error.
- `Commands::dispatch_id` preserves the same checked transport for an already
  resolved command id; stale or fabricated ids remain Guest errors.
- Runtime `state_traits::capture_completion_checked` is the crate-private
  original object-capture owner. It requires the original entered currency and
  transfers one real independent owning reference to each result/options handle
  on success. Refusal releases partial ownership and returns the retained typed
  cause; no NULL Guest placeholders are returned. Options getters may reach
  effects or invalidate the original operation, including change and restoration.
- Runtime's crate-private `completion::capture_bytes`
  (`runtime/rust/src/completion.rs`) uses that owner and the actual selected String
  getter, preserves result/options bytes, retires the two transport references,
  then checks currency before exposing a `ScriptCompletion`. Public
  `Interp::{eval_completion,eval_sourced_completion}` establish the before-body
  scope and preserve first Host through outer error publication. Object-vector
  codegen, original child completion and EngineABI capture consume the same
  checked owner under their actual original scope and owning-reference contracts.
  ABI outputs remain untouched on refused capture.
- Runtime `NativeOperationScope` is a crate-private currency owner, not a
  native-operation permission. `enter_engine_evaluation` resets an old episode
  only at an actual independent engine entry with an empty operation stack.
  Nested work inherits the first cause. Public Interp pending-Host semantics
  and the VM's separate outer-boundary cause retirement remain explicit.
- `tcl-engine-api` retains an actual `NativeExecutionError` through
  `EngineError::ExecutionFailure`. The engine returns the original cause without
  rebuilding its interpreter, profile, frame or namespace fields from text.
  `ExecutionRefusal(String)` remains the explicit diagnostic carrier for
  adapters that expose only text. Neither carrier grants execution authority.
  The C door keeps the whole first host failure outside guest completion.
  A WASM host callback retains its Rust-origin typed cause for the same actual
  interpreter only when no earlier runtime refusal is pending; settlement
  returns it and the next genuine independent entry clears that sidecar.
  Runtime-origin WASM failures retain their existing text-only export.

### `tcl-runtime-api` + `tcl-cmd-core` — trace-aware array enumeration

- `ArrayTarget` is the variable cell located before an array-operation trace.
  `array::dispatch_at` snapshots candidate keys from that cell, then asks
  `VarStore::array_read_elem_at` to perform each live, trace-aware read. This is
  the one boundary at which a runtime may combine its stable cell identity,
  frame/namespace lookup, alias movement, and trace engine.
- The runtime fires the containing-array and element read traces in Tcl order,
  resolving the element group only after the containing-array callbacks have
  completed. It then reports `Value`, `Missing(ArrayReadMiss)`,
  `TraceError(ArrayReadFailure)`, or `ArrayInvalidated`. A callback error wins
  when that callback also invalidated the base; otherwise the shared command
  owner skips a missing candidate and chronologically merges miss metadata.
- `ArrayElementRead` returns its `Value` variant transiently owned across the
  runtime boundary.
  Pointer-backed runtimes call `ValueOps::pin_value` before releasing the
  selected cell; the shared owner calls `unpin_value` after `new_list` has
  retained every element, and on every earlier hard-error exit. Owning value
  models implement those hooks as no-ops.
- The cell/ordering rules apply across the runtime releases. The callback name
  recovered through a scalar-looking alias to an array element does not:
  engines continue to gate that spelling through their selected Tcl release.

### `tcl-syntax` — the parse grammars and value seam

Native compound producers select the physical string recipe where the actual
interpreter is available. Runtime consumers use `Interp::new_list_object`;
VM consumers use `Value::native_list_constructor`. Retained List transport uses
the issuing engine's private whole-backing receipt and shares its actual canonical
flag; independently created List data uses a separate constructor. A zero
member C List constructor yields NULL-type canonical empty storage; a cached
empty List remains a distinct representation. Unknown producers retain no
recipe and checked materialisation refuses unavailable authority.

List and Dictionary string updaters use `NativeListResultSerialization` with
the retained `NativeStringProtocol`, recursively materialising original member
objects. Existing resident bytes remain exact. Checked dictionary construction
materialises original keys before hash insertion, retains the first key object
and last value for duplicates, and preserves native bucket history. Native
string allocation identity comes from constructors and reached updater
receipts, independently of byte length.

A fresh C List backing has an unset canonical flag. Missing resident bytes
still permit pure List evaluation. C 8.5 and 8.6 string updaters set the shared
backing flag; C 9 sets it only for an unshared backing. Copying shared backing
for mutation retains the flag in C 8.6 and clears it in C 8.5 and C 9. Backing
leases keep original members alive through shimmering without adding child
object references. Renderer bookkeeping does not counterfeit native sharing.
Use the shared copy/updater recipes when adding mutation or string access;
fixture checks cover the original object, resident bytes and backing flag.

Selected Double formatting retains the engine independently of digit precision.
C 8.4 uses lower-case nonfinite spellings, later C uses `NaN` and `Inf`, and
Jim drops the NaN sign. Checked access refuses a pure C 8.4 minimum integer
without an authenticated formatter build recipe; already resident bytes remain
authoritative. Native numeric parsing does not grant string-updater authority.

- `naming::NativeNameProtocol` selects audited purpose-specific byte extents.
  Command lookup and script publication, public C registration, namespaces,
  scalar roots, combined/separate elements, trace registration, package keys,
  formals and hidden tokens use their named operations. `NativeNameProjection`
  retains original bytes, selected input, purpose and actual namespace context.
  `NativeCommandSlotProjection` retains the purpose-selected root/current
  token route and exact qualifier keys alongside `ByteCommandSlot` geometry.
  Physical command consumers walk those keys from the selected actual token;
  equal constructed paths cannot substitute a retained current token for a
  rooted request. C8.4/8.5 procedure publication retains its final rooted C API
  reparse, while alias and C API publication use their distinct unqualified
  global rule. Use the purpose-specific `*_projection` methods for physical
  holder lookup/publication and `*_slot` methods for pure geometry only.
  Consumers retain responsibility for fallback order, qualifier creation,
  incarnation checks and lifecycle; the projection supplies none of those.
  Jim command keys are flat and keep publication spelling separate from the
  root-marker-stripped comparison key. Native name recipes grant no cell,
  namespace incarnation, command token or compiler permission.
- `naming::NamePolicyProtocol` retains provider authority. An actual audited
  engine/build selects a native recipe; `authored_tcl` explicitly selects a
  logical simulation. `InvocationDialect::authored_name_policy` selects an
  `AuthoredSimulation` recipe for pure source assistance; a selected C release
  and pinned Jim retain distinct policies, while unversioned Tcl explicitly
  uses the C8.6 analysis abstraction. Vendor compatibility, source grammar and
  numeric engine availability do not select native name or string recipes.
- `naming::NativeCompiledVariableProtocol` selects compiler-local lookup
  independently of native name lookup, numeric support and source grammar.
  C compares full name lengths before a bounded CString comparison; counted
  suffixes after raw NUL can select an earlier slot without changing either
  primary key. Dynamic scans of already compiled frame locals use CString
  equality in C8.4/8.5 and counted equality in C8.6+. Dynamic hash-table keys
  retain their own purpose. The source substitution selector scans every
  counted byte in order: the first qualifier selects dynamic lookup, while an
  earlier array opener in a single-component token ending in `)` permits only
  an existing local. Synthetic slots never satisfy a source-local query.
  Jim retains counted exact name comparison and does not attest C indexed
  procedure compilation. Registry issuers validate an actual audited engine;
  an explicit F5 Tcl8.4 logical provider retains authored authority separately.
  `NativeCompilationEntry.compiled_variable_protocol` carries that selected
  issuer through live compilation. A supplied entry with no policy remains
  unknown. `SourceAnalysisOptions::compiled_variable_protocol` preserves this
  withdrawal; physical engine metadata cannot replace it. VM logical-provider
  installation requires a selected actual host and a valid F5 context, affects
  cache policy, and passes to child interpreters. Active native host calls use
  their own compiler policy. `LocalVarTable::set_native_protocol` configures all
  ordinary intern and query methods, so Unicode and native-byte emitters share
  the same comparison while retained slot names remain exact.
  `NativeCompiledVariableRecipe::scalar_name` returns a
  `NativeCompiledScalarName` declaration projection, not an allocated slot.
  C8.4 rejects an array name before declaring its base; C8.5+ can declare the
  base before declining scalar compilation. Consume `declaration` and `scalar`
  separately. `LocalVarTable::intern_anonymous` reserves a distinct unnamed
  slot: it has no dynamic key or source-local match. Keep those indices in the
  native auxiliary layout and install genuine frame cells before execution.
  Runtime logical eval-object, source-word and expression-parser capabilities
  retain the independently supplied host profile in their policy identity.
  An accepted capability change invalidates retained `Interpreter` guards;
  identical installation and rejected capabilities preserve them. This policy
  invalidation does not mutate native C compiler or resolver epochs. Original
  expression caches retain their selected parser policy and decline reuse
  under a different host policy without rebinding the original header.
  `NativeCompiledVariableEnvironment` distinguishes declarations, absent local
  environments and read-only borrowing of actual frame slots. Only C8.6+ can
  borrow an existing frame local cache; C8.4/8.5 and Jim cannot. Borrowing never
  creates a slot. `command_lookup` selects an already resolved literal scalar
  or array base; `substitution_lookup` selects the original variable-token
  geometry. An existing-only substitution compares its entire original name,
  including array-looking syntax. `from_native_slot_names` preserves every
  physical index and unnamed temporary marker, while `native_slot_names`
  exports that layout without inventing source names for temporaries.
- `key_suffixes` builds possible summary labels from the existing constructed
  `key_segments` projection. It retains the complete input, lone colon segments,
  trailing empty components and unique nonempty suffixes. These labels select
  neither a written lookup path nor a Native command or namespace incarnation.
  CFG frame and parameter maps retain all procedures sharing a key through the
  same original Module and procedure-header/body inventory. One unavailable,
  changed or redefined member leaves that key unavailable. Parameters require
  identical ordered names and original raw declarations, including defaults and
  variadic position. Frame composition additionally requires identical summaries;
  unknown or conflicting targets become opaque without definite alias maps.
  The protective `ProcIndex` retains every possible parameter-summary candidate
  and unions its possible literal by-name arguments and omitted defaults. A
  missing label protects literal passed names instead of establishing an
  effect-free target. It only suppresses store advice: original binding/CFG
  opaque effects, positioned source reads and store-removal proofs remain their
  independent owners. Summary labels cannot supply missing source or Native
  dispatch, frame, contents, observer or completion authority.
- `key_holder_and_tail` and `key_segments` operate on analytical constructed
  text keys. They cannot reconstruct every native path from a display: `[a:, b]`
  and `[a, :b]` both render as `::a:::b`. Preserve original `ByteNamespacePath`
  boundaries and actual interpreter/namespace incarnation independently. Written
  source operands still use the selected written-name grammar; rendering a native
  slot supplies neither those operands nor a globally callable spelling receipt.
- Runtime command-generation queries share the actual current node and
  resident-table traversal. The callable query retains its independent Jim
  previous-worker observation and existing clone order; the FQN-only query
  reports current resident placement without acquiring callable or Native
  reference holds. Hidden-command exposure settles its existing mandatory
  cleanup under the same entered operation before joining that generation's
  actual placement. Trace/import labels follow the surviving original token;
  a callback-created replacement or retired token supplies no publication name.
  Requested spelling and namespace display never reconstruct a missing slot.
  The same restore kernel returns an unpublished original key/binding bundle to
  checked callers for mandatory cleanup outside the table borrow. Neutral bool
  callers retain their existing drop order. Publication refusal is retained
  before original incoming owners or the held ensemble token retire; same-scope
  checks carry that complete first cause through the existing cleanup workers.
- Compiler namespace storage classification is shared by side-effect readers
  and generic static-key advice through `namespace_storage_scope_for_place`.
  Actual retained cell ownership and its selected storage domain precede the
  checked simple-key UTF-8 projection; namespace display and F5 profile metadata
  cannot classify a Native cell as worker-owned. Explicit authored source models
  retain their descriptive classification without acquiring physical TMM facts.
  `namespace_variable_simple_name` and the root-only query share the retained
  namespace-variable root, including real variable links. IRULE4002 consumes
  `def_places_at` at the retained CFG operation and its operand-selection phase,
  after original Module/event/source/config guards and executable-block checks.
  Configured patterns match selected simple keys and literal element metadata;
  source labels and document spans remain presentation. Missing original cells,
  unavailable multi-output source correspondence, unknown lifetimes or unchecked
  UTF-8 decline advice. Static classification grants no executing worker, value
  equality, cross-worker publication, Native getter or compiler-cache authority.
- `written_separator_extent` derives the final complete separator run from
  `qualifier_component_extents`; `written_command_tail` shares that boundary.
  Core neutral qualifiers/tail and Registry counted qualifier/fold adapters use
  these pure counted spans. Native C qualifier reporting selects its original
  CString extent before the same query; Jim keeps its separate selected
  last-colon rule and original-object result roles. The query grants no dialect,
  namespace, lookup, publication or writable-source authority. Nested ASCII bare
  variables in the array-index scanner use the dependency-free core name
  scanner; selected braced and array-index styles remain independent. VM math
  dispatch selects a direct global function through the Registry descriptor
  owner after retaining the actual stock implementation identity.
- `reported_qname_namespace` owns the literal namespace-prefix projection of
  Unicode analytical QNames. `AfterLeadingRoot` searches beyond one leading root
  marker; `RootRelativePrefix` roots a relative prefix; `LiteralPrefix` preserves
  that prefix. All purposes default to `::` when no prefix is selected. Overlapping
  colon pairs, Unicode and NUL remain reporting text. This query supplies no
  written-name grammar, constructed-key split or Native namespace identity;
  actual source and execution consumers retain their independent typed contexts.
- Script procedure declarations validate their selected holder before formal
  parsing. C Tcl 8.4/8.5 then render and reparse the publication name through
  the C registration recipe; C 8.6+ retain the holder token. The C API keeps an
  unqualified registration global and qualifies a relative qualified name
  against the actual current namespace. These entry points share constructed
  byte slots but preserve their distinct selection rules.
- Jim canonical namespace construction retains the original root-relative
  operand, constructs a fresh absolute CString suffix, or duplicates the actual
  nonroot namespace object and appends the original operand. Equal projected
  bytes cannot identify which object is returned. Runtime namespace activations
  retain that actual object separately from C parent/child edges. Jim command
  storage is flat; procedure context derives from the counted publication key.
  `jim_info_command_names` selects the native flat scan and namespace helper
  projection: relative procedures are local, relative commands also include
  matching global keys, and absolute results retain the helper's root marker.
  Original opaque keys remain counted throughout enumeration.
- Namespace-variable queries use their own purpose. C uses CString input and
  reports the selected namespace-table cell, including undefined or linked
  cells, without following its target. C 8.4–8.6 admit the global alternate
  candidate; C 9 uses the current namespace only. Jim's script helper returns
  a canonical qualified name independently of whether a variable exists.
- Namespace qualifier and tail reports use their selected text operation.
  Jim procedure activation instead reads its counted selected command key;
  a raw NUL before a later separator can therefore affect qualifier reporting
  and procedure context differently. Namespace lookup failures retain the
  actual current namespace reporting bytes and the operation's error code.
- `native_string::materialize_native_string` preserves every resident string
  byte. Only independently proved pure C ByteArray input receives the native
  per-byte U+00XX encoding, including modified NUL. The adapter retains the
  original object/backing, installs a string once and keeps its physical storage
  marker; empty bytes alone do not attest the canonical empty singleton.
- `backslash::native_source_string_bytes_in` shares lexical fragments with
  Unicode presentation and supplies the native original-unit decoder and
  encoder. Source token scanning and bounded token decoding remain explicit.
  Literal raw bytes, escaped native units and ByteArray script materialisation
  are different inputs. The native source fixtures contain original controls
  and results for C Tcl 8.4–9.1 and the audited current Jim implementation.
- `naming::checked_command_slot_utf8` and `checked_namespace_path_utf8` convert
  constructed components directly. Source aliases additionally require the
  native spelling helpers' exact round-trip. Opaque byte rows remain in complete
  native snapshots and token indexes; an unavailable Unicode alias does not
  remove a binding or prove its absence. Jim namespace source aliases require
  the independently retained actual namespace object.

- `list` — the Tcl list codec. `find_element` is the single grammar
  primitive every splitter layers over (`split_list`,
  `split_list_raw`, and the lenient pair), and the **dict** string-rep
  scan in the WASM runtime walks it too — `SetDictFromAny` uses the
  same `FindElement` grammar and differs only in the noun it prints,
  which it composes from `junk_fragment`. On the join side,
  `append_list_element` is the byte entry point
  (`TclScanElement` + `TclConvertElement`); `list_element` and
  `join_list` are its `&str` facades, and the WASM runtime's list and
  dict string-rep generation binds it directly rather than carrying a
  second implementation.
- `number` — the `TclParseNumber` port (9.0-first: `0d` radix prefix,
  `_` digit separators, bare leading `0` is decimal), with
  `parse`/`parse_whole`/`parse_whole_with` (`ParseFlags` mirrors
  `TCL_PARSE_INTEGER_ONLY` etc.), `is_expr_number` (which delegates its
  boundary question below), and `format_double` (`Tcl_PrintDouble`).
- `glob` — `string match` globbing (`string_match`,
  `string_case_match`).
- `switch_body` — the one `switch` pattern/body-pair tokeniser (brace
  levels, comment rule, `-`-fallthrough), shared by the analyser,
  formatter, minifier, and semantic tokens.
- `naming` — `::`-qualified-name parsing and command resolution:
  `qualifier_segments` / `qualifier_segments_owned` (a colon **run** is
  one separator, mirroring `TclGetNamespaceForQualName`),
  `ends_with_separator`, `is_qualified`, `normalise_qualified_name`,
  `qualify`, `command_resolution_candidates` / `resolve_command_with`
  (the `Tcl_FindCommand` order, conformance-pinned), and the variable
  helpers (`normalise_var_name`, `split_array_name`,
  `split_element_ref` / `split_element_ref_bytes`, …).
  `native_autoload_command_candidates` owns C `init.tcl`'s ordered
  resident-byte `auto_index` keys under the actual native string protocol.
  A matched command separator invokes the shared native Unicode decoder and
  encoder, reproducing `regsub` rebuilding; no match preserves original
  counted bytes. The constructed namespace remains unchanged. Jim declines
  the C recipe. `autoload_command_candidates` supplies analytical Unicode
  keys; the LSP `auto_qualify` facade delegates to it. Neither query proves
  original-object getter authority, library availability or a command binding.
  `split_element_ref` is `TclObjLookupVarEx`'s rule over an already
  **resolved** name (no `$` / `${...}` sigil stripping): array element
  iff longer than one byte, ends in `)`, and contains a `(`. The `(`
  may sit at offset 0, so a zero-length array name is legal — `set (x) 5`
  writes element `x` of the array named `""`. Both runtimes' element
  splitters bind it.

  Consumers: `tcl-vm` (`interp::elem_ref`, `command::looks_like_element`,
  `command::parent_namespace_of`, `exec`'s `ARRAY_MAKE_STK` guard),
  `tcl-compiler` (`codegen::values::split_array_ref` / `is_array_ref`, the
  facade the whole codegen layer calls, and
  `analyser::diagnostics::var_command`'s array-element harvest), and — inside
  the owner's own file — `split_array_name_for_style` and
  `split_array_name_braced_for_style`. Consumers call this owner rather
  than re-derive the element split. The owner manifest validates declared
  ownership; it does not establish that a consumer calls the shared function.


  Deliberately **not** a consumer: `tcl-compiler`'s
  `sccp::array_element_base` is a narrower fold-safety predicate (documented
  at the site) that excludes the zero-length array name this owner admits.
- `boolean` — `Tcl_GetBoolean` word recognition (unique prefixes of
  `true`/`yes`/`on`/`false`/`no`/`off`). Its prefix rule is *not* the
  option-table matcher below — boolean words have a fixed six-word
  vocabulary with cross-set ambiguity (`o`), so it stays here.
- `expr` — the expression AST, parser, evaluator seam, and walk
  (`ExprOps`, `mathfunc`). The math-function table is one dispatch over
  `NumValue<B>` for both engines and the const-folder, with the two release
  axes (`MathFuncSince`, `IntWidth`) and the typed refusals (`MathFuncError`)
  owned here rather than re-derived per engine.
  `quoted_string_body` is the one rule for which `ExprNode::String` operands
  substitute: the variant keeps both spellings with their delimiters, and
  only `"…"` substitutes. The walk hands `ExprOps::string` that answer, so
  the VM substitutes only a quoted operand and the const-folder declines one
  it cannot substitute. A braced operand is not quite raw: its
  backslash-newlines fold even inside braces, so the engines pass it through
  `backslash::collapse_brace_continuations`, and the const-folder, which
  does not know whether the dialect folds (Jim keeps the bytes), declines one
  that carries a backslash-newline. `fixed_string_operand` (and
  `fixed_string_body` for the walk's stripped body) answers "is this operand
  its own value in every dialect": the const-folder, native lowering and the
  F5 XC translator read a constant only through it. Codegen, taint and the
  `uri_split` hint read the quoted/braced split through
  `quoted_string_body`; the hint rejects a quoted operand that substitutes
  (`$`, `[`) but reads its backslash escapes as written, the form its regex
  classifier expects. The compiler's substitution walks read the same rule
  too.
- `rand` — the Park-Miller `rand()`/`srand()` generator both engines call
  (step, seed nudge, and C's reciprocal-multiply scaling). Only seed storage
  and the first-seed policy are per engine.
- `backslash` — the byte-slice convenience over the lexer's decoder
  (see next); deliberately no second decode implementation.
- `word_rules` — what a *word* means as a value: `WordValueRules` carries
  the brace-continuation and list-parse axes together (a word-shaped list
  asks both at once), and `whole_braced_word` answers "is this word a braced
  literal". That last one is release-invariant and so takes no rules, but it
  is shared for the same reason the axes are: the VM's `subst_word` strips
  such a word's braces and suppresses all substitution, and codegen has to
  decide the identical question about the identical word before it emits.
  Matching outer braces alone is insufficient: `{}${z}` contains a
  substitution outside the braced portion and must retain its complete word.

### `tcl-dialect` — foundational expression grammar

- `tcl_dialect::scan_expr_number` owns C-style candidate numeral boundaries.
  `scan_jim_expression_number` owns Jim's integer-first native token frontier,
  including fallback to decimal zero for an invalid radix prefix and retry of
  double parsing only at the native retry characters. The expression lexer and
  fresh Jim term constructor share that frontier and its integer/double kind.
  A constructor receipt requires the exact counted token extent; a valid numeric
  prefix does not validate a following suffix. These owners live below both the
  lexer and syntax parser.
- `tcl_dialect::scan_nan_payload` — the 8.5+ `TclParseNumber` NaN payload
  state machine: ASCII whitespace is allowed around/between one through thirteen
  hexadecimal digits; a fourteenth digit invalidates the parenthesised form.
  The scanner and value parser share it.

- `tcl_dialect::DialectProfile::find` — catalogue lookup from a canonical
  profile name or registered alias to an interned profile, returning `None`
  for an unknown name. User-written dialect names resolve through
  `tcl_registry::model::ingress::resolve_environment`; the validator form is
  `resolve_known_environment`. Its `analyser_profile` and `unit_profile`
  projections retain their distinct unknown-input and `tk` promotion policies.
  Consumers choose the projection for their purpose rather than parsing or
  validating environment names independently.

- `tcl_dialect::DialectProfile`'s `file_extensions` and `filenames` — the two axes
  of **file** recognition, and the single source every editor's registration
  list and every extension→dialect route projects. They answer different
  questions and neither substitutes for the other: `file_extensions` claims a
  trailing suffix (`xdc` → `xilinx-eda-tcl`), while `filenames` claims a whole
  basename (`bigip.conf` → `f5-bigip`) because the files it names have no
  suffix worth claiming — a bare `.conf` belongs to every unrelated config file
  on the machine. `tcl_registry::dialects::dialect_from_extension` consults the
  basename axis **first** (a name claim is the more specific one), and
  `cargo xtask gen-editor-extensions` projects both into every editor. Both are
  one-owner-per-key across the catalogue, invariant-tested in `profile.rs`.
  Consumers project both lists rather than restating file-recognition rules.

- Command availability for a *document* is asked at one **point** — the
  resolved environment's authoring query
  (`tcl_registry::model::ingress::DocumentEnvironment::document_context`,
  then `ResolvedContext::authoring_query`, as
  `static_document_context_for` returns it). It is what keeps the additive
  `tk` ingress working: the `tk` environment's point names `Tk` among its
  packages even though the analyser-facing fallback's does not — the union is
  a fact of the resolved environment, never recomputed at a string boundary.

### `tcl-lexer` — source-text decoding

- `backslash_subst` (re-exported as `tcl_syntax::backslash::decode`) —
  the one byte-exact `TclParseBackslash` port, shared by the
  LSP/compiler token pipeline and both runtimes.
- `ranges::braced_var_name_end` — the one release-aware `${...}` close
  rule (`Tcl_ParseVarName`), selected by `BracedVarStyle`: 9.x counts
  nested `{...}` and consumes `\X` as an inert pair, the 8.x family ends
  the name at the first literal `}`. The lexer's own `parse_var` /
  array-index scans and **both** `subst` engines resolve the form here;
  an engine that hard-codes one release's rule answers
  `subst {${a{b}c}}` wrongly on the other.

  Returns `BracedVarEnd` — `Closed(offset)` or `Unterminated` — **not** an
  `Option`. "No closer" is an error C names
  (`MISSING_CLOSE_BRACE_FOR_VAR`, owned here too), not a benign miss, and
  evaluating engines must raise this error; only a tokeniser may recover
  (the lexer runs the name to end-of-input so it can keep tokenising
  half-typed source). The 9.x rule also *widens* what is unterminated —
  `${a\}` and `${a{b}` close under 8.x but not under 9.x.

  Consumers include the `subst` and tokeniser surfaces, compiled-word
  decoders (`segmenter`, `values` and `helpers`), `expr_lexer::variable`, and
  free-text scanners in Syntax, optimisation, dataflow, taint and analysis.
  An expression body parsed from an ordinary Tcl word must retain the same
  variable-close grammar as the corresponding conditional expression.


  A caller **passes the resolved `BracedVarStyle` down**; it does not
  re-derive the closer. `tcl_syntax::naming` exposes `split_braced_var_ref`
  plus `*_for_style` entry points for every reader that unwraps `${...}`
  (`normalise_var_name`, `var_reference`, `element_var_name`,
  `split_array_name` and their `_braced` variants) — the no-argument
  spellings take `BracedVarStyle::default()`, which is the rule a document
  with no explicit dialect is lexed under, and a caller that has resolved
  the document's dialect must use the `_for_style` form. In the compiler the
  style comes from the layer's existing dialect view:
  `optimiser::PassContext::braced_var`, `analyser::Analyser::braced_var`,
  the taint `TaintCtx` / `TaintScan` / `SinkCall`, `ScanCtx`'s `LexerConfig`,
  the `Lowerer`'s `config`. In `tcl-lsp-core` it comes from the resolved
  `DialectProfile` the rename entry points already carry.

  `dynamic_variable_word_can_spell` requires the resolved style. Where
  `naming` exposes a defaulted convenience, it is for an entry genuinely
  lacking a resolved dialect. A consumer holding a resolved dialect uses
  the style-taking form.


  The selected rule matters to rename safety. The literal
  characters around a substitution bound which cells a computed name can
  spell, so the two rules move a rename decision opposite ways: 9.x reads
  `${a{b}c}` as one wildcard that can spell anything (refuse the rename),
  while 8.x ends the name at the first `}` and leaves the literal `c}`
  (provably out of reach, allow it). Reading an 8.x document with the 9.x
  default refuses a rename that is provably safe; reading a 9.x document with
  the 8.x rule lets an unsafe one through.

  Two classes of site are deliberately **not** threaded, and both are
  documented in place so they are not "fixed" back into plumbing that cannot
  change an answer:

  - a scan whose own gate rejects every name the two rules can disagree
    about. The rules differ only on names containing `{`, `}` or ``, and
    `analyser::param_traits::extract_var_name` accepts only
    `[A-Za-z_][A-Za-z0-9_:]*` while `subst_nocommands`'
    `is_complex_var_name` accepts only alphanumerics and `_`. Threading a
    style into either cannot change the accepted name set, so both document
    the restriction rather than taking an unused parameter;
  - an entry point with no document profile in scope at all
    (`auto_path_eval`), which passes
    `BracedVarStyle::default()` explicitly rather than silently.

  A scanner with a narrower accepted name set must decline divergent
  grammar shapes explicitly. That restriction does not grant it authority
  to decode unrestricted variable references independently.


- `parse_cut::first_parse_cut` — the one answer to *where a script stops
  parsing*: which command C rejects, at what offset, with which message.
  It walks `group_commands` and `word_parts` in C's own order — commands,
  then words, then components, descending into `[…]` bodies and
  `$arr(index)` — rather than filtering the lexer's flat warning stream,
  so nested delimiter failures preserve native parse-error ordering. The command index is
  the part a message alone cannot carry, and it is what a bytecode
  front-end needs to compile the prefix that runs before the raise
.

- `word_parts::decompose` — the one splitter of a Tcl word (or a `subst`
  template) into its substitution components: C's `ParseTokens` breakdown
  into text runs with their backslash escapes folded in, `$name` /
  `${name}` / `$arr(index)` references, and `[script]` substitutions, with
  the `{*}` expansion flag and the parse errors C names carried alongside.

  It exists because that walk had **four** implementations that drifted
  (bucket R10): `runtime/rust/src/parse.rs`'s `scan_parts`, that crate's
  `subst.rs` mirror, `tcl-vm`'s `subst.rs`, and the compiler's
  `segmenter.rs` / `ir.rs` `WordExpr` builder. Only one raised C's `missing
  close-bracket`; only one found a `]` without being fooled by a brace,
  quote or comment in the substituted script; only one decoded a literal
  run's escapes. All four are consumers: `runtime/rust`, `tcl-vm`, and the compiler, whose
  `WordExpr` builder lives in `word_expr.rs` over `decompose_spanned` rather
  than in `ir.rs`'s fragment walk. The segmenter keeps what it owns, command
  and word boundaries; only the within-word breakdown is the owner's.
  `differential_word_expr` asserts that production against a frozen reference
  walk across crafted edge cases, the sample corpus and tcllib, so any
  behaviour difference is enumerated rather than assumed.

  The module sits in `tcl-lexer` because its dependencies already do —
  `braced_var_name_end`, `scan_array_index`, `command_substitution_end`,
  `close_quote_offset`, `backslash_subst_in` — and because the crate is
  below `tcl-syntax`, both runtimes and the compiler, so every consumer
  reaches it without inverting an edge.

  Three properties are contractual:

  - **Borrow-based.** `WordBody::Literal` and every `Variable` / `Command`
    part are sub-slices of the caller's source; only a text run that
    actually had an escape to decode owns its bytes. That keeps the literal
    fast path zero-copy, which is what lets the runtime's `parse_cache`
    hold parsed commands against a stable script slab
    (memory-management.md MM-B.6) with the stale-slab hazard a *compile*
    error rather than a runtime one. Consumers preserve this borrowed
    source ownership instead of replacing it with independent owned bytes.
  - **Errors are parts, not a `Result`.** A malformed construct becomes a
    `WordPart::ParseError` carrying C's exact message (`missing "`,
    `missing close-bracket`, `missing close-brace`, `missing close-brace
    for variable name`, `missing )`, `invalid character in array index`),
    and the parts scanned before it are still returned. That is C's order,
    not leniency: `subst` substitutes incrementally, so `subst {[side][b}`
    runs `side` and keeps its side effects before reporting the missing
    bracket (8.6.16 and 9.0.4 agree). A consumer wanting C's *script*
    order — `Tcl_ParseCommand` parses every word before evaluating any, so
    nothing runs — scans all its words and raises the first `ParseError`
    before resolving anything.
  - **An unterminated `[` reports what is inside it.** C recurses into
    `Tcl_ParseCommand` at the bracket rather than hunting for the matching
    `]`, so `subst $t` with `t` = `[set y ${a{b]` is `missing close-brace
    for variable name`, not `missing close-bracket`. The missing bracket is
    the fallback, not the default.

  The release axis is the whole `LexerConfig`: the `${...}` close rule,
  array-index source mask and escape grammar. The one non-release axis is `SubstFlags::bare_var_refs`, false
  for exactly one consumer — `tcl-vm`'s compiled-word `PUSH` operands,
  where the compiler has already inlined or normalised every real
  reference, so a surviving bare `$` is data. Modelling that as a flag on
  the shared scan is what keeps the VM on this owner rather than justifying
  a private copy.

- `structural_index::command_boundaries` — byte-scanned reparse split
  points, sharing one scanner family with `script_is_complete` and
  `reparse_window`. `script_is_complete` answers completeness on raw document
  text; `command_boundaries` supplies command terminator offsets;
  `reparse_window` snaps an edit range outward to those boundaries. Every
  boundary must satisfy `script_is_complete(&source[..b])`.

  This scanner is distinct from `script::group_commands`: it retains empty
  and comment-only terminator offsets used by incremental reparse windows.
  For example, `a\n\n\nb\n` yields boundaries `[2,3,4,6]` but only two
  grouped commands. It makes an allocation-free byte pass and remains total
  on malformed input.

  The scanner takes no `LexerConfig`. Its completeness boundary therefore
  cannot replace selected-dialect command grouping. F5 brace-line continuation
  and C 8.x first-close braced variables can change actual command boundaries.
  Completeness recovery uses `ranges::command_substitution_end` and, on a
  terminal quoted scan, `ranges::close_quote_offset`, retaining later cuts
  even when a nested command is malformed. Coverage agreement is required
  only for complete scripts. The differential boundary and grouping tests
  check the scanner, grouper and compiler segmenter under their own contracts.

### `tcl-cmd-core` — portable command logic

- `native_list_storage` owns release-selected range disposition and C9
  replacement layout over issued allocation extents and original member stores.
  Actual engines own headers, slots, windows and memory-only inspection views;
  the pure recipes grant none of those owners or compiler admission.

- `channel` owns output-facing channel configuration and encoding. The host
  supplies one typed `SystemEncoding`; `ChannelConfig` derives Tcl 8/Tcl 9
  profile defaults, retains each channel's input/output translation and common
  encoding, and `StandardChannelConfigs` owns the predefined handle defaults.
  The command registry resolves the dialect-filtered `fconfigure` option into
  a typed operation; `config_list`, `config_value`, and `set_config_value` own
  its runtime-neutral state policy. `OpenAccess` and
  `resolve_open_access_mode` normalise both simple and Tcl-list access modes so
  handle permissions, creation flags, direction, and binary configuration
  cannot diverge between adapters. `encode_output` and `encode_output_bytes`
  return raw bytes together with a possible conversion error, preserving Tcl's
  successfully converted prefix before `POSIX EILSEQ`. Runtime channel tables
  own handles, sharing topology, and mutable instances of this state, but do
  not reimplement access modes, encoders, binary mode, locale interpretation,
  translation, option availability, or conversion-profile policy.
- `namespace` — the pure `::` byte-ops `tail` / `qualifiers`
  (`last_sep_run`: colon runs are one separator) plus the
  `Namespaces`-generic cores. Runtime name resolution routes through
  these — the VM's `interp.rs` canonicalisers (`canonical_cmd_key`,
  namespace declare/find/parent/import/forget) and `command.rs`
  (rename re-homing, `proc` namespace derivation) are built on them.
  The generic cores cover `current` / `exists` / `parent` / `children`
  (including byte-valued twins and Tcl string-hash enumeration order), the
  positional `which_request`, import-source validation, `which_command`, and
  `which_variable` (the
  `Tcl_FindNamespaceVar` probe — namespace variable tables only, never
  a call frame; its *alternate* global-rooted candidate is the one
  release axis, dropped by 9.0's `flags |= TCL_NAMESPACE_ONLY`) and
  `origin` (`NamespaceOriginCmd`). The two accessors they need are on
  the `Namespaces` role trait: `namespace_var_exists` and
  `command_origin`. `command_origin` is the *whole* import walk, not a
  single hop, because C's `TclGetOriginalCommand` is, and because a
  runtime whose import links are name-keyed needs its own
  disambiguation (the VM's hidden/visible token domains).
- `ensemble` — `tclEnsemble.c`'s tables and rules: the
  `namespace ensemble` subcommand table, the **two** option tables
  (`create` carries `-command` and no `-namespace`; `configure`
  carries `-namespace`, read-only, and no `-command`), the
  exact-then-unique-prefix subcommand scan, and the dispatch miss
  messages, plus the non-empty implementation-prefix invariant for `-map`.
  `EnsembleToken` is the shared stable command-token lifecycle: its live
  configuration and name survive imports, reconfiguration, and rename, while
  true deletion irreversibly retires that token. `InvocationLayout` and
  `invocation_layout` own the parameter/subcommand/argument positions and must
  be recomputed from the live token after an `-unknown` callback. The exact
  `UNKNOWN_DELETED` message and error code live beside that lifecycle rather
  than in either runtime.
  The scan is `prefix::scan`'s rule with one documented
  divergence: C's ensemble path is a `strncmp` over the word's length,
  so an **empty** subcommand prefixes every entry and resolves against
  a one-entry table, where `Tcl_GetIndexFromObj` forces the error
  path. `subcommand_choices` is the ensemble enumeration, which keeps
  a comma before `or` even for two entries (`bar, or baz`) — the
  wording `prefix::choice_list` must not be used for.
  Consumers of the scan and of `unknown_subcommand_message`: both
  engines' script ensembles and their built-in `info` and `file`
  ensembles (the runtime's `file` keeps the registry's release-gated
  name set and borrows only the sentence), plus their `string` and
  `tcl::prefix` ensembles — `tcl::prefix`'s enumeration comes from the
  ensemble owner, not `prefix::choice_list_bytes` — and their `dict` and
  `array` ensembles, where resolving the word first is also what makes
  `array e a` fire the variable's `array` trace under the canonical
  name — and their `binary`, `binary encode`/`decode` (the one
  `prefixes = false` pair, whose miss is `unknown subcommand`),
  `encoding`, `chan`, `zlib` and `namespace` ensembles.
  TclOO's own tables resolve through `prefix` too — `info object isa`'s
  `category`, `configure`'s `property` (C reaches it through
  `tcl::prefix match -message property`, so it abbreviates),
  `definitionnamespace`'s `kind`, and `method`'s `export flag` — while
  every *method* list is joined by `tcloo_choice_list_bytes`, which
  drops the Oxford comma the option tables keep. Resolving the word is
  only half of each of these: `info object isa` then checks an **exact**
  count per resolved category (`InfoObjectIsACmd` checks the argument
  count twice — once before the category resolves, once after), and the
  `$child` shorthand words every arity message from `invoked_word` while
  the *option* identity comes from the table.

  **The table a scan runs over is release-gated.** A `TclMakeEnsemble`
  subcommand set is a release fact, both engines are release-selectable,
  and a name from the wrong release changes prefix verdicts for words
  that have nothing to do with it (`dict g` is `get` on 8.6, ambiguous
  on 9.0 once `getdef`/`getwithdefault` exist). Each engine's
  `environment::release_subcommands` filters its table through the
  selected release's registry surface — removal only, so the engine
  still owns which names it dispatches and their order — and it is
  memoised per `(command, release)` because it sits on the `dict` /
  `string` / `info` dispatch path. A resolution miss must then *report*
  rather than fall through with the raw word: dispatch arms match
  canonical names, so an exact 9-only spelling would otherwise still
  run under an 8.6 pin.
- `index` — Tcl index parsing (`Tcl_GetIntForIndex`: `end`, `end-2`,
  `1+1`) and nested-index drilling.
- `prefix` — the `Tcl_GetIndexFromObjStruct` port, with
  `prefix::OptionTable` as the one API: a const-constructible value
  generic over `AsRef<[u8]>` entries carrying a command's names in C
  table order, its error noun, and its abbreviation mode
  (`abbreviating` = C flags `0`; `exact_only` = `TCL_EXACT`).
  `resolve` applies C's rule (exact-match wins; unique non-empty
  prefix; ambiguous-vs-bad distinguished exactly as C words it,
  including the empty-key rule) and `index_of`/`index_of_str` attach
  the canonical miss message. Executable static-table consumers use
  `index_of_original`: the neutral `ValueOps::original_option_index` door
  retains the SAME original object, declaration-table identity, exact flag,
  noun and selected native failure metadata. Both ports inspect a compatible
  Index before invoking its string getter. Jim performs portable matching
  without installing a C Index; an unavailable actual issuer refuses.
  Sort/search pass each original option directly to this door. Switch and
  regex read the selected original first byte before resolving an option;
  regsub retains that option plan through callback execution without rescanning.
  C switch adapters call `switch::parse_options` with the actual release and
  `switch::usage` for both argument forms. C8.4 consumes all leading options
  and lets the last matching mode win; C8.5+ reserves the subject/body words
  and rejects a second mode. Jim uses its separate two-byte scanner.
  The composing escape hatches stay
  public for sites that build their own sentence: `scan` (the
  noun-free rule), `choice_list_bytes` / `choice_list` (the `a`,
  `a or b`, `a, b, or c` enumeration with C's empty-entry quirks),
  and `bad_key_message` (byte nouns — the runtime's `tcl::prefix
  match` `-message`). Consumers: `switch`/`lsort`/`lsearch`/`regexp`/
  `regsub`/`trace`/`string is` option words (this crate), the VM's
  `tcl::prefix match` and `string is`, the WASM runtime's `string`
  ensemble, `tcl::prefix match`, OO option tables, and both engines'
  `interp debug` option word (noun `debug option`),
  `interp limit` type word (noun `limit type`), and the `interp`
  ensemble, child-as-command, `interp create` and `interp invokehidden`
  option words. Where an engine advertises only the subcommands it
  dispatches, the miss message is composed with `scan` +
  `bad_key_message` over the shorter table — the way C reports
  `interp`'s own misses against `optionsNoSlaves[]`. `package`'s
  subcommand word and `package prefer`'s `preference` word resolve here
  too (they are `Tcl_GetIndexFromObj` tables), as do `update`'s option, `try`'s
  `handler type`, and `seek`'s `origin`. `after` shares only the
  `scan` — C looks its subcommands up with a NULL interp and composes
  its own `bad argument …, or an integer` sentence, so no
  `OptionTable` message may surface there. New command
  modules MUST resolve through `OptionTable` (or `scan` +
  `bad_key_message` where a byte noun or interleaved control flow
  demands composition) — never a hand-rolled scan.
- `trace` — argument decoding shared by both ports, which retain their
  trace tables and firing sites. `resolve_option_original` selects the static
  C8/C9 roster using the actual engine recipe; Jim's compatibility dispatcher
  uses its portable roster without C cache publication. The original type
  and operation-list members pass through `resolve_type_original` and
  `parse_ops_original`; these retain native Index getter order and diagnostics.
  The byte compatibility helpers resolve already materialised table inputs.
  `parse_ops` validates an op list and
  `parse_legacy_variable_ops` the 8.x `rwua` letter string; **both return
  the set in `TraceKind::info_order`**, the order C's `TRACE_INFO` arms
  render (`array read write unset`, `rename delete`), which is *not*
  `TraceKind::ops`' `opStrings[]` table order used by the bad-operation
  error. Storing that canonical order is what makes `trace info`
  byte-identical without per-runtime render tables.
  `legacy_ops_letters` renders a stored set back to `rwua` for
  `trace vinfo`, and `callback_op_word` supplies the single letter an
  old-style (`trace variable`-installed) trace's callback receives.
  Firing order, storage, and re-entrancy stay per-runtime — see
  [variable-trace-dispatch-and-introspection.md](variable-trace-dispatch-and-introspection.md).
- `sort::parse_wide` / `sort::parse_real` — the `-integer` / `-real`
  key parsers (`parse_wide` is the whole-string integer-only shape of
  `tcl_syntax::number`, `i128`-wide; `binary`'s wide parse narrows it
  by wrapping, matching C's `binary format`).
- `error::CmdError` — the canonical error-message catalogue
  (`wrong_args`, `bad_choice`, …). The runtimes' arity helpers are
  thin adapters: `runtime/rust`'s single `Interp::wrong_args` method
  and the VM's `interp::err_wrong_args`.

### `tcl-registry` — immutable semantic snapshots

`CommandRegistry::snapshot` retains the complete authored registry and its
actual generation. Its cache-free `RegistrySemanticKey` compares command and
overlay spec order, installed surface layers, the full profile key, ambient
and own package floors, special-variable rows, overlay identity, pack origins,
reference texts and document grammar. A fingerprint selects a hash bucket;
full structural equality decides reuse. The snapshot's derived reader caches
are excluded from that identity.

Every semantic mutation uses the shared invalidator to retire derived
semantics, snapshots and native-registration projections and issue the next
registry generation. Held snapshots keep their original registry. Pack
reference-text and special-variable changes therefore cannot share a stale
semantic snapshot. `registry_snapshot_tests::snapshot_retains_pack_reference_overlay_and_special_variable_axes`
is a software cache control, with no Native command-table, body, frame or
executed-handler observation.

### `tcl-registry` — constant evaluation and value transfers

- `value_transfer` — what one invocation computes on the value axis, as a
  registry-owned `CommandSemantics` specialisation reached through the
  resolved invocation (`InvocationSemantics::value`), with three declaration
  states at command, subcommand, and form scope and a derivation from a
  descriptor that states the same operation. The compiler's
  `value_transfer` module is the engine: it proves operands from the SCCP
  lattice and applies validated answers, and it recognises no command by
  name — `cargo xtask value-transfers` is the gate. Every direct route is
  implemented in the registry (`NativeEvalId::owner`). The generic loop
  plan's per-binder transfer stays in the compiler; its current ownership
  is recorded in the
  [command-knowledge inventory](../compiler/value-transfers-migration.md#compiler-owned-handlers).
  Design:
  [value-transfers.md](../compiler/value-transfers.md),
  [value-evaluation.md](../compiler/value-evaluation.md).

### `tcl-compiler` — nested command-substitution words

- `word_subst::nested_command_words` recovers exactly one complete command
  from a recorded substitution extent using the canonical configured segmenter.
  `lifted_calls`, native lowering and the WASM leaf-invoke planner consume this
  syntax geometry. Execution and read evidence require their independent owners.
- `LiftedCall` carries that structure to consumers in `arg_words`, so a
  braced literal is told from a word that substitutes by the
  segmenter's `WordExpr`, not by a `{`/`[` test over argument text —
  the distinction `Statement::Call::args` cannot make.

`word_subst::entered_expression_evaluations` is the crate-private query for a
reached native expression evaluator inside a written operand. Call it with the
original `CommandTokens`, fallback lexical configuration and the compilation
unit's registry. The child must retain its actual handler and a reached source
binding; its single original braced expression must match the exact source
instance and byte extent. The receipt contains the parsed expression, absolute
content base, child invocation span and retained logical grammar/number axes.
Missing evidence, replacement, an earlier failing operand, inert brace quoting
or changed source bytes returns no receipt. This is unavailable evaluation
evidence, not proof that the parent completes or that its operands can be erased.

`interval_bounds::find_divide_by_zero_with_entered_operands` and SSA's nested
expression-read projection consume this query. For example, a reached
`return [expr {1 / 0}]` retains the child divide-by-zero finding even though
`return` cannot dispatch. A child with different number or lexer rules cannot
consume the surrounding function's interval or guard facts. Other selected
expression-argument roles use the same retained handler's original written
argument mapping. The tests
`entered_expression_operand_survives_absent_parent_dispatch`,
`entered_expression_operands_decline_unreached_and_replaced_evaluators`,
`entered_expression_operands_retain_actual_grammar_and_original_bytes` and
`entered_operand_divisions_survive_parent_failure_and_respect_reachability`
cover reached errors, unavailable evaluators, unchanged geometry and mixed
syntax policies. Source effects remain owned by the original child invocation.

### Source activation and compilation profiles

`InterpState::source_profile` selects the source grammar and compiler target of
the current activation. An ordinary activation uses the configured user
dialect. An explicit native host activation uses the selected host profile;
restoring the activation restores the user profile. The interpreter's
configured dialect, command visibility and physical engine remain independent.

The VM's lexer configuration, escape and braced-variable rules, script command
planning, procedure and script compilation, `NativeCompilationEntry.profile`
and executable-module profile validation consume this selection. A compile
service must use the profile passed with its target and preserve the entry's
independent engine and invocation evidence. Reconstructing a compiler profile
from the physical engine or command catalogue loses the source activation.
Cached modules compare their complete retained compilation entry before reuse.

An iRules event uses the configured Tcl 8.4 source grammar, including its
expansion and variable delimiter rules. The simulator's explicit C9 framework
host activation uses C9 source rules through the same compiler service.
Neither command availability nor ordinary event execution selects that host
activation. `explicit_host_source_keeps_parser_compiler_and_cache_profile_together`
and `host_script_and_its_nested_substitution_use_the_explicit_source_activation`
exercise profile agreement, nested source evaluation and restoration.

### `tcl-compiler` — body assistance and definite grammar

The private facade in `registry_invocation.rs` keeps body navigation separate
from an actual entered activation. Its roles use **original post-head operand
indices**: a captured alias prefix contributes values and grammar, but cannot
acquire an editable source index. Every actual candidate participates in the
possible union; opaque and absent alternatives remain explicit residuals.
Applicable catalogue and document declarations contribute navigation only.
The nonrecursive builder returns a boxed private facade, so recursive body
walkers retain one pointer instead of the candidate vectors and invocation
carrier. This is analysis storage only; boxing grants no additional proof and
does not change the shared depth or stack budgets. One private candidate
preparation retains the original effective argv and selected facts for this
invocation. Possible roles and unanimous roles/traits/scope project that same
collection. When the May query uses a catalogue dialect fallback but the exact
actual dialect is absent, the two different query axes retain separate facts;
May fallback never becomes an actual engine receipt. Applicable catalogue and
declaration navigation are prepared separately once. A definite invocation
still requires the owner's proved singleton execution target.

| Consumer purpose | Projection | Required input | Permitted conclusion |
|---|---|---|---|
| Original retained invocation | `invocation_body_assistance` | Original `CommandTokens`, selected context and current document surface | Possible body/expr roles and candidate grammars; separate unanimous accepted traits, scope and invocation |
| Positioned segmented body | `segmented_body_assistance` | Exact retained source inventory, source bytes, affine base and actual lexer configuration | The same projections, only after the original source slice matches and tokens receive that inventory |
| Offset-free body syntax | `unpositioned_body_assistance` | Retained all-world lookup consensus and the selected syntax surface | Possible navigation; all definite roles, traits, scope and state-bearing invocation fields are withdrawn |
| Entered body reconstruction | `executed_script_at` and `executed_script_entry_namespace_at` | Actual invocation allocation site, effective operand and exact retained body source | Unanimous recorded source and entered namespace; neither proves unconditional entry or a successful body |
| Conditional declaration entry | `conditional_body_entry_at` and facade `parameter_invocation` | Exact retained declaration allocation, original body source, native formal recipe and own namespace; converged accepted invocation layouts | Symbolic local-copy and alias traits inside that declaration, conditional on entry; no caller depth, actual argument value, store or completion |

For an original statement, pass its retained tokens directly. For a segmented
body, first select its `ExecutedScriptSource` inventory when the origin differs
from the document root, then pass the body's actual base and lexer configuration.
Do not stamp local body offsets against the root inventory, analyse a detached
body as a new source entry, or query the final import list to recover a role.
Without a positioned source carrier, request offset-free syntax assistance.
The positioned adapter attaches the original binding before selecting a query
context. Its retained actual execution point takes precedence over the
catalogue's optional core authoring profile: an unprofiled catalogue does not
erase an actual entry's typed Tcl release. Explicit package placements keep
their separate authoring availability context; the retained invocation dialect
and realm still select native facts. A missing actual point and missing
applicable authoring context decline the query. Offset-free assistance still
withdraws every definite field, even when that point is known.

Use `possible_roles` to find script and lambda literals for navigation, and
candidate definition/case grammars to find their nested syntax. A member keyword
is interpreted only inside an already selected definition grammar; it supplies
no global Tcl command identity. Use `definite_roles`, `definite_traits` and
`definite_scope` for a grammar obligation or scoped definition. They require
accepted cardinality, complete roles and agreement of every actual candidate,
with no opaque or absent alternative. Alias/value-copy summaries use the
separate definite invocation and preserve read-before-write order. These
summaries still grant no physical successful store or future callback immunity.

A declaration-only body uses its retained `DeferredSourceBody` recipe through
`SourceConditionalBodyEntry`. The query checks the original source occurrence,
formal names and own namespace; conflicting allocations or layouts withdraw it.
`namespace_context` retains the procedure body's exact key; receiver previews
return no entered context. A receiver declaration's
`declaration_namespace_context` supplies provenance only. `RootScript` requires
a global frame layout and the actual retained root identity. Original body
restoration filters preparation and math records by that exact namespace key.
None of these declaration queries adds an entered-frame observation. ParamTraits may follow a local
symbolic copy such as `set local $body; eval $local` in that conditional own
frame. An accepted `upvar 1 $formal alias` layout can also identify the
formal's symbolic caller-variable-name role without selecting a caller cell.
Actual caller cells, literal targets, read/store effects and activation values
require a recorded procedure entry for the exact source. A detached bare body
cannot supply that entry, even when a similarly named declaration exists
elsewhere.
Document stub roles attach through the retained lookup worlds independently of
native execution; opaque or replaced slots cannot recover those roles. With no
native or authoring context, an applicable stub still supplies declaration-only
navigation. Unknown replacement alternatives withdraw parameter evaluation
traits while remaining visible to syntax navigation.

Option-value roles come from the selected `InvocationOptions` owner, including
its authored leading operands, available option table and exact value widths.
The same projection feeds owned invocation facts and structured role queries.
For example, a tcltest `-body` value is a script while a `-result` value containing
`-body` remains data. This does not establish provider loading or body entry.

For example, `interp alias {} declare {} proc p {x}; declare {}` maps the written
argument zero to Body. Neither captured `p` nor `{x}` gets a written Name or
ParamList position. Replacing `if` with an ordinary procedure withdraws builtin
body roles. Conditional replacement may retain possible Body navigation, but
withdraws definite grammar, scope and state-bearing summaries.

| Consumer | Permitted use |
|---|---|
| `analyser/commands.rs` generic body dispatch | Original operand navigation; unanimous scoped sibling definitions and event context; no final-import body query |
| `analyser/utils.rs` comment walker | Possible body/lambda, definition-member and case-list syntax from the retained original invocation |
| `analyser/param_traits.rs` shallow/deep scans | May Body/Expr/evaluation/loop-input traits; accepted original-operand alias and value-copy summaries |
| `analyser/param_traits.rs` caller-target queries | Accepted frame-alias grammar and definite mapped writes; offset-free navigation cannot create caller targets |

Entered-frame and source-role reconstruction in lowering requires the retained
body inventory. Body navigation supplies neither those receipts nor physical
state effects for handler-specific declaration and per-item consumers.
Nested-expression contents reads and source edits likewise require their exact
retained expression source and read-site owners.

### `tcl-compiler` — text similarity

- `text` — `edit_distance` (optimal string alignment over chars),
  `suggest_similar` and the ranking cores `rank_suggestions`
  (ascending `(score, name)`, capped) and
  `rank_containment_suggestions` (exact > prefix > substring).
  Consumers: every did-you-mean suffix (W001/W123/W210/W212/W215,
  E001), completion's fuzzy fallback, and the package-suggestion
  ranking in code actions. These compiler-local consumers use
  `tcl-compiler::text`; the module has no compiler-independent consumer.

### `tcl-compiler` — diagnostic suppression directives

- `analyser::utils` owns both halves of the `# noqa` / `# tcl-lsp: disable=…`
  contract: the pre-scans that build the analyser's `suppressed_lines` map
  (`parse_file_suppression` for the top-of-file directive, recorded against the
  `FILE_SUPPRESS_KEY` sentinel line; `parse_noqa_line_suppressions_for_dialect`
  and `apply_preceding_noqa` for an inline directive, recorded against every
  line the *following* command occupies), and `line_suppressed`, the one
  predicate that reads it.
- `parse_noqa_marker` is the one grammar for an inline directive, shared by
  both pre-scans: a `noqa:` marker at a word boundary carries the codes it
  names, a comment whose whole body is `noqa` carries the `"*"` wildcard, and
  a comment that merely mentions the word carries nothing. A consumer that
  matched the bare substring instead would silence every finding on the
  command below `# do not use noqa here`.
- The analyser records the map but does not filter with it — only the surface
  that renders a finding knows which line it lands on. One consumer applies
  the map now: the policy step, through `Directives::reason_for`
  (`rust/tcl-lsp-core/src/diagnostic_policy.rs`), whose bucket rule restates
  `line_suppressed`'s and is pinned equal to it by
  `directives_agree_with_line_suppressed`. That is what makes a directive
  mean the same thing in the editor and on the command line, as
  [`docs/kcs/kcs-howto-suppress-diagnostics.md`](../../kcs/kcs-howto-suppress-diagnostics.md)
  promises; see [diagnostic-policy.md](../compiler/diagnostic-policy.md).
  No producer filters with it, the W305 producer
  (`rust/tcl-compiler/src/analyser/source_integrity.rs`) included. The one
  policy step left inside the analyser is its fold of the top-of-file
  directive into its production skip (`apply_disabled_diagnostics`,
  `rust/tcl-compiler/src/analyser/diagnostics.rs`), which the report declares
  as a gap rather than reading as clean.

### `tcl-core-types` + `tcl-runtime-api` — command-table identity

- `CommandSlot` is the dependency-low, generic identity of one Tcl
  command-table entry. A namespace handle/path and the simple tail remain
  separate because Tcl permits distinct slots whose fully-qualified display
  strings collide (`a: -> b` and `a -> :b` both render as `::a:::b`). Runtime
  tables use `CommandSlot<NameBytes, NsId>`; static compiler and workspace tables
  use the `StaticCommandSlot` alias over `StaticNamespacePath`.
- `display_namespace` and `display_command` are output projections only.
  `encode_command_slot` is the sole injective compatibility encoding for a
  consumer that still requires a string map key. No consumer may parse either
  projection back into semantic identity. Native namespace tables retain
  explicit parent/child `NsId` edges.
- `OoId` is the shared interpreter-local identity for a TclOO object or class.
  The native runtime carries it through class/object/provider and active-call
  relationships; mutable command slots and display names remain projections
  attached to that stable token. Every consumer uses this owner rather than
  declaring a private object-identity type.

Runtime command retirement transfers the existing procedure binding reference
with `NativeProcedureBinding::take_for_retirement` into an extracted command
slot. The shared retirement drain releases that reference, then the actual key
and binding owners, outside namespace and frame borrows. Metadata views add no
procedure role; active frames keep their independently acquired role. Jim
slots preserve their distinct key-before-value deletion and value-only
replacement, while C slots release their selected procedure binding role.

Command replacement, procedure installation and copied namespace publication
retain the original operation through registration-name and argument cleanup.
The same namespace token and lifecycle are checked after reached retirements
before compiler-header handoff, token activation or another store. Copied
variable stores propagate their full typed failure as well as checking the
original operation. Insertion returns its actual minted generation before retirement. Fixed-slot
publication continuations check that token and original Proc binding after
registration cleanup and each header getter; they cannot adopt a replacement
or a renamed slot. This software refusal boundary does not assert Native
command-pointer rename chronology. A callback-recreated idle ObjCmd uses its existing unique
raw-token disposal purpose; other recreated native payloads without an admitted
disposal owner refuse before incoming adoption and preserve the reached slot.
Those unsupported payloads require their own concrete disposal contract.

### `tcl-syntax` — event-handler boundaries

- `event_handlers` is the dependency-low extractor for live
  `when EVENT { … }` handlers in one supplied script region;
  `event_handlers_with_head_predicate` lets a higher layer supply the resolved
  command identity without making `tcl-syntax` depend on compiler facts.
  `script_commands` exposes the same lexer-owned word boundaries without
  guessing that arbitrary braced values are executable.
  `tcl_registry::events::top_level_when_handlers_with_registry_and_head_resolver`
  resolves each top-level head at its absolute document offset before accepting
  an event handler, using the resolved profile's lexer grammar. Nested script
  surfaces remain handler data: F5 iRules permits `when` only at the top level.
  Rooted colon runs, event case, comments, quoting, and iRules' `}{` separator
  therefore have one lexer/naming contract. `tcl-irules::when_blocks` is the
  iRules-configured wrapper. Registry, CLI, explorer, LSP, and MCP consumers
  use these APIs and spans rather than scanning text.
- `IrulesDeclarationArguments` carries the decoded values beside each
  argument's lexer token and single-word fact. The registry uses it to accept
  an iRules `when` or `proc` declaration only when its body is one braced
  source word; bare, quoted, and compound bodies cannot create lowering,
  symbol, diagnostic, or executable-inventory regions. The shape query keeps
  an otherwise valid unknown event available to IRULE1002, while the
  known-event query excludes it from executable roots.
- The registry wrapper also owns iRules priority state. `priority N` changes
  the inherited priority of subsequent event declarations, an inline
  `when EVENT priority N` overrides only that handler, and an omitted priority
  inherits 500 until changed. Valid priorities are `0..=1000`; lower values
  run first. Repeated handlers for one event remain distinct, and equal-priority
  handlers preserve source insertion order. Cross-file ties preserve the
  virtual server's iRule attachment order at the host boundary.
- `IrulesExecutionContext` and `IrulesCommandPlacement` own the other half of
  that boundary: the iRules top level is declaration-only (`when`, `proc`,
  `timing`, and `priority`), while executable commands belong in event or
  procedure bodies. The analyser supplies lexical context and consumes the
  registry decision for IRULE5005, IRULE5006, and IRULE5007; it does not keep
  a second command-name allow-list.
- `tcl_irules::irules_executable_commands` is the inventory view of that same
  contract. It follows registry-declared bodies, clause lists, and command
  substitutions inside valid top-level event and procedure declarations while
  treating comments, ordinary Tcl data, invalid top-level execution, and
  nested declarations as inert.

### Native name consumers

Materialise the original operand through the selected native string provider
before applying a naming purpose. A pure ByteArray payload is not its native
string, and a lexical escape is not a resident string. Retain the original
value for aliases, usage headers, traces and script arguments. Select a slot
once, then carry its namespace token, exact simple bytes and generation;
rendering that slot never authorises reparsing it as an address.

Use checked namespace and command lookup doors. Missing names, retired worlds
and unavailable selected implementations are distinct outcomes. An exact
pending publication refuses dispatch for that selected slot; a pending deletion
masks only the old slot and preserves ordinary lookup fallback. Unrelated
committed workers remain available. Native publication receipts and services
retain weak world ownership, exact slot identities and revisions across
reentrant callbacks. Native retirement runs while the retiring generation is
bound; callback replacements survive generation-checked removal.

Runtime original command keys use the selected
`NativeNameProtocol::jim_command_key_construction` purpose.
`Interp::original_jim_command_key` preserves the actual namespace object and
original procedure or rename operand; `generated_jim_command_key` constructs an
object only for a genuine provided-byte registration API. Jim registration and
qualification temporaries remain distinct from the command table's independent
key reference. Fresh slots acquire that reference, occupied replacements keep
the original key, and local previous-node pushes preserve it. Jim core alias
capture keeps its original List and members lazy; child registration creates a
fresh CString key. C alias records independently own their supplied original
name, and their fresh query List is distinct from Jim's stored header.

Use the original-key publication entries with the actual prepared owner and
`NativeCommandCreation` slot guard. A same-spelled replacement cannot restore a
retired weak key owner. Extract real key/value retirement under the namespace
borrow and call `Interp::retire_pending_native_ensemble_roles` after releasing it
at each genuine worker boundary, before later lookup or result publication. The
shared drain releases a Jim key before its value and continues mandatory cleanup
under a retained first Host cause. Qualification temporaries retire separately
before caller result or diagnostic work. Allocation leases do not add a native
reference, and generated bytes, table keys and retirement tokens establish no
additional name, frame, command or source authority.

Namespace script assembly uses the selected object concatenation owner and
original byte source. Native eval and inscope preserve completion codes and
source locations. C inscope resolves an existing namespace; Jim's helper
enters its canonical namespace object. Byte compilation caches retain
SourceImage, constructed namespace path and the actual entry receipt.

Tests for a new naming purpose require original positive and negative controls:
raw NUL versus modified NUL, opaque bytes versus their encoded scalar, resident
String versus pure ByteArray, original source versus constructed object, and
actual retained token versus a same-spelled recreation. Record publication,
lookup and reporting independently. Observer operations must not mutate error
or cache state before capturing the native result being compared.

### `tcl-core-types` — shared vocabulary

- `NameBytes` owns immutable exact byte names with byte equality/order/hash and
  a checked Unicode view. It performs no qualification, truncation or encoding.
  `ByteNamespacePath` stores already constructed components, preserving literal
  colons. `ByteCommandSlot` pairs such a path with an exact simple name; native
  snapshot consumers use `NativeByteCommandSlot` for the same vocabulary.

- The crate for cross-runtime plain types (diagnostic codes today).
  Anything two crates must *name* identically without depending on
  each other's machinery lands here.

### `tcl-sslictcl` — the TLS declaration vocabulary

- `load_with_diagnostics` is the one reader of a `.sslictcl` document.
  It walks the canonical syntax tree and constructs no interpreter, so the
  document is never evaluated — not even a `check`'s `predicate`, which it
  retains verbatim. `DECLARATIONS` is the machine-readable statement of the
  vocabulary the loader implements; `docs/design/f5/sslictcl-vocabulary.md` is
  its prose, and a unit test holds the two together.
- `evaluate_policy` owns **finding identity**: a policy finding is
  `(check id, endpoint)`, which is why the `grade` id is reserved and why
  declaring `check grade { … }` is `SSLIC1009`. Every consumer that
  deduplicates, suppresses, or compares findings across runs keys on that
  pair rather than on a message.
- `embedded_dataset` is the single reader of the embedded trust-store and TLS
  source bundle. Nothing else fetches it, and nothing reaches upstream at
  build, report, or editing time — see
  [`sslictcl-source-data.md`](sslictcl-source-data.md).
- The loader reuses the shared owners rather than re-deriving them: the
  command/word segmentation owner (`tcl_compiler::segmenter` over the
  canonical CST) reads the document, and `tcl_syntax::list` splits every
  braced `LIST` value, so a `forbid-ciphers {[A-Z]*RC4}` glob means exactly
  what a Tcl list means. There is no SslicTcl exception in the "Known
  deliberate exceptions" section because there is no divergence to record.
- The editor projection is a separate owner because two binaries consume it:
  `tcl-lsp-server` publishes the loader's diagnostics and `tcl-cli`'s
  `diag` / `lint` verbs report the same set, and the rule that the loader
  **supersedes** the analyser's unknown-command verdict in a never-evaluated
  document must be stated once for both. Its second half is the *outline*:
  a declaration document has no procs, classes or namespaces for the
  analyser's scope walk, so its blocks are its structure.
- Which **vocabulary** a position admits is a different question with a
  different owner, and deliberately not stated here: it is the
  definition-body grammar in force, which `tcl_lsp_core::oo_body`'s
  `definition_grammar_at` answers for every definition body of every class
  system, rooted in `CommandRegistry::document_grammar` for a dialect whose
  file is itself a declaration body. Completion and the token walk read it,
  and this owner must not grow a second answer to it.

### `tcl-lsp-core` — diagnostic policy

- `diagnostic_policy::apply` is the one function that decides whether and
  how a finding shows: the five configuration scopes, the default-off seed,
  severity overrides, the optimiser and shimmer switches, overlap
  precedence and encoding abstention, in one fixed order, for every
  producer's finding on every surface. `PolicyBuilder` resolves the
  configuration layers, lowest first, into the `Policy` `apply` reads, so
  only it — never a merged, already-flattened settings value — can name the
  layer that decided a code. `Report` keeps every finding paired with its
  outcome, including the ones it hides, each with a `Reason`, so "why is
  this not firing" has one answer every surface can read.
- `diagnostic_report::document_report` and `standalone_findings` are where
  a surface without the language server's salsa database gets the same
  producer set the editor does before handing it to `apply`:
  `document_report` adds the producers this crate owns (the source-style
  pass, the byte-integrity pass, the SslicTcl projection) to a surface's own
  findings, and `standalone_findings` runs the analyser and the compiler
  checks over one compilation unit for a surface with no database at all.
  `optimise_under_policy` is the rewrite loop every rewrite surface
  (`tcl opt`, MCP `optimize`, `tcl-lsp.optimiseDocument`) shares: it admits
  a pass's rewrites into the optimiser's own multipass loop only where the
  policy shows them.
- `config_ini::settings_from_ini` and `merge_settings` are the one INI parse
  and the one three-layer merge: every surface that reads a `config.ini` or
  a `.tcl-lsp.ini` — the server, the CLI, the MCP tools — resolves the same
  layers `PolicyBuilder` builds the policy from.
- Consumers: `rust/tcl-lsp-server/src/lib.rs`,
  `rust/tcl-cli/src/commands/diag.rs`, `transform.rs`, `policy.rs`,
  `rust/tcl-mcp/src/tools.rs`. A producer emits a typed finding and reads no
  policy; a surface renders a `Report` and decides nothing itself. Design:
  [diagnostic-policy.md](../compiler/diagnostic-policy.md).

## Decision rules / contracts

1. Consumers must not re-derive these utilities. A new `split("::")`,
   integer scanner, option-prefix loop, or `wrong # args:` /
   `bad option` format string in a consumer crate is a review defect —
   call the owner (or extend it) instead.
2. Byte/`&str` duality is handled by the owner: the canonical
   implementation is byte-based where both runtimes need it
   (`tcl-cmd-core::namespace`, `::prefix`; `tcl_syntax::naming::
   qualifier_segments`), with `&str` conveniences layered on top —
   never a parallel string-side re-implementation.
3. Namespace-name splitting must be separator-**run** aware everywhere
   (a run of 2+ colons is one separator; a lone `:` is an ordinary name
   character). Command names keep a trailing run as the `{}`-named
   entity (`proc quux::: …` defines `::quux::`); namespace names drop
   it (`namespace eval c::: {}` creates `::c`).
4. Message texts come from the owner so they stay byte-identical to C
   Tcl across runtimes (`tcl-cmd-core::prefix` for bad/ambiguous
   option, `CmdError` for arity). Adapters may *prefix* (see the OO
   exception below) but never re-spell the core text.
5. Grammar direction is 9.0-first by design: the shared number grammar
   accepts `0d5` and `1_000` even though 8.6 rejects them (dialect
   gating is a lexer/analyser concern, not a per-consumer parser fork).
6. **A per-argument fact is one authored row, projected once at load.**
   `CommandSpec`'s six parallel per-argument tables (`arg_roles`,
   `arg_types`, `arg_values`, `closed_value_args`, `arg_presentation`,
   `command_prefixes`) are index-keyed slices, and the loader's
   `ArgRows::seal` is the **one** place an authored `arg` row becomes that
   parallel form — a column added to the row and not to the projection
   vanishes silently, which is the defect the record shape exists to
   prevent.

   There is no per-argument **lifecycle** machinery, and the retired-api gate
   (`make xtask-retired-api-gate`) bans every spelling of it — `arg_rows`,
   `VersionedArgRow`, `ProjectedArgs`, `ArgTables`, `arg_tables_at`,
   `project_arg_rows`, `arg_indices_for_role_at`, `command_prefixes_at`.
   Declared-and-unpopulated surface costs an `is_empty()` probe on every call
   and has no consumer but its own tests. A per-argument version gate comes
   back **with** its consumer (principle P-C), and `ArgRows::seal` above is
   where it would attach.

   The version-gated facts that remain — a *value*'s own `Lifecycle` and
   `versioned_arg_values` — keep the request-time discipline the rest of
   this point states: the floor is a per-document fact settled by the
   document's `package require` lines, so it is an **argument** to
   `available_arg_values_at`, never registry state. Registry handles are
   cached per (profile, pack overlay) and shared across documents, so one
   that remembered a floor would answer the wrong document. Consumers
   reading during the walk keep the permissive no-floor answer: their
   verdicts are formed before the floor is knowable, which is why the arity
   axis (invariant 7) buffers and decides post-walk.
7. A version floor is a lower bound and composes by taking the greatest.
   Three things can state one — a `package require` in the document, a
   `SpecTcl` pack's `ambient_package` row, and the profile's
   `LibraryPin` — and `version_gate::FloorSource` is the single place
   that ranks them. The *version* is the max; the ordering of the
   `FloorSource` variants is the **reporting** tie-break at equal
   versions, closest-to-the-author first, and is not a claim that one
   source is more authoritative than another.

## Known deliberate exceptions

- `string match` / `string map` / `string compare` / `string equal`
  option words: C hand-rolls a `length > 1` prefix test (`strncmp`,
  `StringCmpOpts`) instead of `Tcl_GetIndexFromObj`, which differs from
  the table rule on the empty word and a lone `-` (`string match - a b`
  and `string compare "" a b` are both `bad option`, where the table
  rule would call them ambiguous) — kept hand-rolled, with the probe
  cited at the site (`tcl-cmd-core::string`).
- `try`'s completion codes (`TclGetCompletionCodeFromObj`) are
  `TCL_EXACT` with a custom `…, or an integer` trailer, and `after`'s
  subcommand scan is a NULL-interp lookup whose miss falls through to
  an integer parse: both compose their own sentence, so only the
  matcher — never an `OptionTable` message — is shared.
- TclOO method dispatch, `oo::define`'s slot ops, and its body-command
  lookup are hash probes, not tables: only the **join**
  (`prefix::tcloo_choice_list_bytes`) is shared.
- `glob` is **not** an exception — both engines resolve its option words
  through `OptionTable`, and the record belongs here because the VM's
  rejection of an unknown option is a ruled-on behaviour, not an incidental
  one: `glob -x a` must error rather than run, and `-types d` must not leak
  its value into the pattern list. The table advertises `-tails` and
  `-types`, so the VM honours them — no engine may advertise an option it
  ignores. Both engines' text is
  tclsh 8.6.16/9.0.4-exact (`bad option "-x": must be -directory, -join,
  -nocomplain, -path, -tails, -types, or --`), pinned in
  `glob_option_words_resolve_like_tcl_get_index_from_obj` on each side.
- `tcl_registry::abbrev::KeywordTable` is a fourth prefix matcher (it
  supports `min_abbrev` and carries the ambiguous candidates), reached
  through `CommandSpec::resolve_subcommand_word` by the WASM runtime's
  `file` dispatch. It answers a registry-authored keyword set rather than an
  `OptionTable`, so the "new command modules MUST resolve through
  `OptionTable`" rule above does not reach it.

Each of these is a *documented* divergence — keep the comment at the
site pointing back here, and do not "fix" them onto the canonical
helper without reading the rationale:

- `rust/tcl-registry/src/const_fold.rs::split_list` — a conservative
  *fold-safety* splitter that bails on any backslash or bare
  `{`/`}`/`"`, so the optimiser only folds provably-simple lists. Using
  the canonical `tcl_syntax::list::split_list` would fold **more**
  (changing optimiser output); the policy is local on purpose.
- Compiler variable operands use `tcl_lexer::word_parts::whole_var_ref`
  with the actual ingress `LexerConfig`. It recognises only an entire original
  reference; malformed syntax retains the native scanner error and compound
  words decline. `scan_var_ref` shares bare-name scanning and Jim parenthesis
  boundaries with the main lexer. C empty-name array references, Jim high bytes
  and nested keys, and C8/C9 braced closing stay on their individual axes.
  Jim expression sugar supplies no variable reading. `Module::native_lexer_config`
  centralises actual invocation axes over assistance metadata while retaining
  parser coordinates and mode knobs. Bytecode uses
  `CodegenCtx::emit_variable_reference` and a distinct finished-key template
  operand; native expression and word lowering share `variable_reference_place`,
  and direct WASM procedure admission excludes element reads. Original word
  projections additionally validate their exact source extent and provenance.
  Both `$a(k)` and `${a(k)}` denote the same symbolic element; `$a($i)`
  evaluates its key, whereas `${a($i)}` selects the literal key `$i`.
  Syntax supplies no physical cell, successful-read or execution proof. Such
  proof remains with the native-family/frame lookup and captured source owner.
  Native C8.4–9.1/Jim comparisons cover whole/quoted/compound/procedure/nested
  command/expression operands, literal vs evaluated keys, empty names and
  dialect-dependent high-byte/parenthesis scanning. Statically spelled name
  words retain the separate `cell_place` projection over shared name splitting.
- `runtime/rust/src/cmd_oo.rs::wrong_args` — wraps the shared
  `Interp::wrong_args` but prepends the active `oo::define`
  ensemble-rewrite prefix, so single-command definition forms report
  the whole original command (`oo::define Foo method …`) as C's
  `Tcl_WrongNumArgs` rewrite path does.
- `tcl-cmd-core::ensemble::subcommand_choices` — the **ensemble**
  subcommand enumeration, which C renders with a comma before `or`
  even for two items (`x1, or x2`), unlike `Tcl_GetIndexFromObj`; it
  must not be collapsed onto `tcl-cmd-core::prefix::choice_list`, and
  neither runtime may keep a private copy of the quirk.
- The LSP-side matchers (semantic-tokens / minify candidate ranking)
  and `tcl_syntax::boolean` keep their own prefix rules — different
  contracts (ranking, fixed vocabulary with cross-set ambiguity), not
  option-table lookup.

## Failure modes

- Colon-run names resolve differently between the compiler, the VM,
  and the WASM runtime (`foo:::bar` dispatches in one and errors in
  another).
- `lsort -integer` and `binary format` disagree on which strings are
  integers.
- `bad option` / `ambiguous option` texts drift from tclsh by a comma
  or an empty-table wording (`no valid options` vs a pluralised noun).
- An abbreviation resolves in one runtime and is ambiguous in the
  other.
- The optimiser folds a list the runtime would split differently.

## Test anchors

- `rust/tcl-syntax/src/naming.rs` — `qualifier_segments_cases`,
  doctests; `rust/tcl-syntax/tests/command_resolution_conformance.rs`
  (tclsh-pinned; `tcl-compiler` and `tcl-vm` each carry a same-named suite
  for their own layer).
- `rust/tcl-cmd-core/src/namespace.rs` — `qualifiers_and_tail_match_c`,
  and the `tcl_string_hash_order_*` suite pinning the retained
  `TCL_STRING_KEYS` table both namespace child and namespace command
  teardown enumerate.
- `rust/tcl-cmd-core/src/prefix.rs` — C-parity unit tests (empty-key,
  empty-entry, exact-mode wording).
- `rust/tcl-cmd-core/src/ensemble.rs` — the two option tables, the
  ensemble subcommand scan's empty-word divergence, and the
  comma-before-`or` enumeration.
- `rust/tcl-vm/tests/namespace_surface_e2e.rs` and the
  `cmd_namespace.rs` test module in `runtime/rust` — the `namespace`
  command surface (`which -variable`, `origin`, `export`/`import`
  leading options, teardown, `ensemble`) pinned against tclsh 8.6.16
  and 9.0.4 on both engines.
- `rust/tcl-cmd-core/src/sort.rs` —
  `parse_wide_shares_the_canonical_integer_grammar`.
- `rust/tcl-vm/tests/namespace_colon_runs_e2e.rs` — colon-run
  resolution/creation pinned against tclsh8.6.
- `rust/tcl-vm/tests/cmd_info_prefix_e2e.rs` — `tcl::prefix` message
  texts pinned against tclsh.
- `rust/tcl-vm/tests/builtins_e2e.rs` and the `builtins.rs` /
  `cmd_alias.rs` test modules in `runtime/rust` —
  `interp_debug_option_uses_c_noun_and_abbreviates`,
  `interp_limit_type_word_resolves_like_tcl_get_index_from_obj`,
  `interp_subcommand_words_resolve_like_tcl_get_index_from_obj`, and
  `interp_create_and_invokehidden_options_resolve_like_tcl_get_index_from_obj`,
  the `interp` family's option/type nouns and abbreviation verdicts
  pinned against tclsh 8.6.16 and 9.0.4 on both engines
  (`runtime/rust/tests/rename_interp_semantics.rs` pins the runtime's
  deliberately shortened enumeration).
- `rust/tcl-vm/tests/builtins_e2e.rs` and the `cmd_package.rs` test
  module in `runtime/rust` —
  `package_option_words_resolve_like_tcl_get_index_from_obj`, with the
  release axis of `package`'s table (`prefer` from 8.5, `files` from
  9.0) pinned in
  `rust/tcl-vm/tests/cross_version_command_surface_e2e.rs`.
- `rust/tcl-vm/tests/cmd_event_e2e.rs` /
  `cmd_control_e2e.rs` / `builtins_e2e.rs` and the `cmd_event.rs`,
  `cmd_error.rs`, `cmd_chan.rs` test modules in `runtime/rust` —
  `update_and_after_words_resolve_like_tcl_get_index_from_obj`,
  `try_handler_type_resolves_like_tcl_get_index_from_obj`, and
  `seek_origin_resolves_like_tcl_get_index_from_obj`.
- `rust/tcl-vm/tests/cmd_info_prefix_e2e.rs` —
  `info_and_file_ensemble_misses_carry_the_full_option_list`; the
  `cmd_info.rs` / `cmd_fs.rs` test modules in `runtime/rust` carry the
  matching `*_ensemble_miss_carries_the_full_option_list` rows.
- `rust/tcl-vm/tests/cmd_info_prefix_e2e.rs`
  (`prefix_ensemble_and_match_options_resolve_like_tclsh`),
  `cmd_string_e2e.rs` (`string_subcommand_dispatch`, tightened from
  `starts_with` to the full tclsh 9.0.4 text), and the `cmd_string.rs`
  test module in `runtime/rust`
  (`string_and_prefix_ensembles_resolve_like_tclsh`, which also pins
  `string is`'s `class` and `option` nouns).
- `rust/tcl-vm/tests/cmd_collections_e2e.rs`
  (`array_bad_subcommand`, `dict_bad_subcommand` — both tightened from
  `starts_with` to full text) and the `cmd_array.rs` / `cmd_dict.rs`
  test modules in `runtime/rust`, plus
  `prefix.rs::dict_filter_type_noun_and_abbreviations` for the owner's
  own `filterType` row.
- `rust/tcl-vm/tests/builtins_e2e.rs`
  (`ensemble_subcommand_words_resolve_like_tclsh`) and the
  `cmd_binary.rs`, `cmd_misc.rs`, `cmd_zlib.rs`, `cmd_namespace.rs`
  test modules in `runtime/rust` — the `binary`/`encoding`/`zlib`/
  `namespace` surfaces, including `zlib gzip`'s and `gunzip`'s option
  tables (C's order is `-header, -level`, not alphabetical).
- `rust/tcl-vm/tests/builtins_e2e.rs` and the `cmd_fs.rs` test module in
  `runtime/rust` — `glob_option_words_resolve_like_tcl_get_index_from_obj`
  on each side, covering rejection of an unknown option
  as well as the shared abbreviation verdicts.
- `rust/tcl-vm/tests/cmd_oo_e2e.rs`
  (`info_object_isa_category_resolves_like_tcl_get_index_from_obj`,
  `tip558_configurable_property_abbreviates`) and the `cmd_oo.rs` test
  module in `runtime/rust`
  (`oo_option_tables_resolve_like_tcl_get_index_from_obj`,
  `configure_property_word_abbreviates`) — the four TclOO nouns plus a
  four-entry `unknown method` list, which is the shortest one where the
  two join styles differ.
- `rust/tcl-compiler/src/interprocedural.rs` —
  `namespace_parts_from_proc_extracts_segments` (colon-run rows).
- `rust/tcl-syntax/src/list.rs` — `list_element_matches_tcl9` (the shared
  join parity table) and
  `append_list_element_is_byte_exact_and_agrees_with_the_str_api`.
- `runtime/rust/src/dict.rs` —
  `backslash_newline_absorbs_the_following_space_run` and
  `delimiter_errors_keep_their_dict_wording_and_fragment` (the dict scan
  riding the shared list codec).
- `rust/tcl-vm/tests/cmd_collections_e2e.rs` —
  `duplicate_dict_keys_canonicalise_last_value_wins` (every VM dict path
  through `ValueOps::dict_pairs`, including the compile-time
  `dict create` fold) and
  `dict_parse_errors_use_the_dict_noun_and_error_code`;
  `rust/tcl-vm/tests/cross_version_command_surface_e2e.rs` — the
  *foldable* `dict create` availability vector.
- `rust/tcl-vm/tests/dict_canonicalisation_parity.rs` — the cross-crate
  gate for `canonical_dict_slots`. The canonicalisation rule is
  "first-occurrence key position, last value wins". The suite feeds
  duplicate-key and odd-shape inputs through every binding: the `ValueOps`
  seam, the registry `dict`
  const-folds via `run_const_fold`, and the codegen's
  `fold_dict_create_cmd` — asserting byte identity against each other and
  against real tclsh8.6/9.0. The WASM runtime's native dict rep
  (`runtime/rust/src/dict.rs`) canonicalises *incrementally* across
  mutation instead, so it binds the rule by agreement rather than by
  construction; `duplicate_keys_canonicalise_like_the_shared_owner` there
  is its leg of the same gate.
- `rust/tcl-lexer/src/ranges.rs` —
  `braced_var_name_end_follows_the_release_rule`;
  `rust/tcl-vm/tests/cross_version_vars_e2e.rs` —
  `subst_braced_var_close_rule_follows_the_emulated_release`,
  `unterminated_braced_var_raises_on_both_releases` and their
  tclsh-pinned sibling; `runtime/rust/src/subst.rs` —
  `braced_var_close_rule_follows_the_emulated_release`;
  `runtime/rust/src/builtins.rs` —
  `unterminated_braced_var_raises_missing_close_brace`.
- `rust/tcl-vm/tests/language_e2e.rs` —
  `zero_length_array_name_is_an_array_element` and
  `link_commands_reject_element_looking_names` (`split_element_ref`).
- `rust/tcl-spectcl/src/loader.rs` — `ArgRows::seal`, the single projection
  point rule 6 names; `rust/tcl-registry/src/spec.rs` —
  `available_arg_values_filters_the_declared_table`,
  `primary_synopsis_skips_a_form_the_floor_predates` and
  `sub_subcommand_resolution_honours_the_package_floor` for the request-time
  floor; `rust/tcl-registry/src/arity.rs` —
  `adjacent_windows_do_not_overlap_but_straddling_ones_do`;
  `rust/tcl-registry/tests/registry_sweep.rs` —
  `arity_window_gate_rejects_each_malformed_shape` (the shipped-spec
  side of the same invariant the pack loader only notices).
- `rust/tcl-compiler/src/analyser/diagnostics/version_gate.rs` —
  `the_highest_of_the_three_floors_wins` and
  `equal_floors_are_reported_in_precedence_order` (rule 7's two halves:
  the max, then the reporting tie-break), with
  `no_pack_overlay_leaves_every_floor_where_it_was` as the FN guard for
  a session that loads no packs.
- `rust/tcl-compiler/tests/differential_group.rs` — the command / word
  segmentation gate: `group_commands` against the shipping segmenter,
  command for command and word for word, over `samples/` and the Tcl
  9.0.4 library at three dialects and three nesting levels
  (`owner_matches_segmenter_over_corpora`), with tcllib behind
  `--ignored`. `rust/tcl-lexer/tests/differential_boundaries.rs` — the
  other half: `command_boundaries` against that owner
  (`scanner_agrees_with_owner_over_corpora`), plus
  `dialect_divergences_are_pinned` for the two axes the byte scanner is
  deliberately blind to. `make xtask-segmentation-drift` is the
  banned-spelling gate that keeps a *new* consumer from re-deriving
  either boundary privately — including by collecting C's parse-error
  messages into a private list instead of asking
  `tcl_lexer::first_parse_cut`.
- `runtime/rust/tests/parse_cut_agreement.rs` — the parse-error cut gate.
  The cut is applied twice on purpose: `first_parse_cut` answers it from
  source, for a compile front-end that needs the index of the command the
  clean prefix ends at, and `runtime/rust` answers it per command from the
  borrowed tree it has already built, for an evaluator that must not
  substitute a word of a command that does not parse. Runtime evaluation
  uses the borrowed parse tree without re-lexing each command; compiler ingress
  uses the source cut's optional result. Both applications use the same
  `WordSpan::welded_after_close_quote` contract for close-quote boundaries.


## Discoverability

- [design docs index](../README.md)
- [project-layout.md](project-layout.md) — the crate boundaries these
  ownership rules sit inside.
- [family-b-routing.md](../runtime/family-b-routing.md) — the runtime seam this
  crate layering serves.

The substitution observer receives the expression owner’s conditional-path
fact. Binding consumers join skipped and executed states rather than
interpreting lexical discovery as definite execution. Elimination reads the
SSA binding lineage to keep executable stores observed through fresh scalar-
analysis versions live; markers themselves remain without executable uses.

Value provenance uses the canonical lifted-call inventory to recognise the
first substitution’s variable head read before invocation. Only clobbers
belonging to that same host are undone for that operand; earlier invocations
and following statements retain invalidation. Type inference widens
registry-clobbered versions independently of binding and taint lineage. An
unknown prior container contributes unknown elements to later updates.

The straight-line factory precision gate accounts for empty class declarations
through their original structured invocation and selected Registry descriptor.
The signature roster checks inclusion only. A factory invocation must retain
its own complete `CommandTokens` and `SourceInvocationBinding`;
`original_default_manufacture_shape` validates the current class, original
vector, full source policy, actual factory recipe, reached source point and
independently absent lifecycle bodies. Its private
`is_argument_free_anonymous_shape` projection reads the selected manufacturer's
captured layout and complete argument count. It supplies source shape only,
with no completed allocation, Normal result, entered frame or Native compiler
authority. Type inference's result and Normal transfer guards remain separate.

Direct calls pass their actual tokens. Nested calls use
`checked_original_lifted_calls_with_metadata_context` with the supplied complete
source input, or `checked_lifted_calls` for positively tagged standalone
metadata. Each child retains its own original binding. Missing or changed
owners and unavailable inventories withdraw precision; command text, reported
class names and a constant TclOO grammar cannot supply a replacement. An
opaque invocation, binding transition, nonempty class body or control edge
also withdraws the whole straight-line gate.

`manufacturer_method_for_descriptor` reads a selected command's own
manufacturer rows before its definition-body defaults. For example,
`oo::abstract create` uses the metaclass command's public factory row, while
classes it produces inherit the private `tcloo-abstract` defaults. Explicit
original object exports affect the source visibility query; they do not issue
an ordinary Native default factory recipe. Positively retained Logical
conditional advice cannot borrow the reached-only manufacture shape. The
source-layout, owner-withdrawal and homogeneous/mixed collection controls
check these distinct contracts; the public abstract-factory visibility evidence
is recorded in
[tcloo-original-abstract-manufacturer-public-visibility.md](../analysis/name-resolution-proofs/tcloo-original-abstract-manufacturer-public-visibility.md).

The internal SSA entry-binding adapter receives the compilation unit’s formal
parameters before allocating scalar value versions. The seeded-parameter
residual tests and first-store collection assertions check this route.

`NativeResultContract::ListRange` retains the exact list/first/last positions.
`NativeResultSelection::ordinary_list_range_representation` requires an
independently proved ordinary input and its captured actual list length, then
uses the selected native index policy and clamped nonempty range. Empty,
unknown and C 9 abstract-list inputs provide no representation proof. Source
captures the frozen ordinary receipt before native operand coercion changes its
epoch, then publishes only on normal completion. This shape proof supplies no
freshness, object identity or executable constant.

`InvocationFacts::successful_variable_output_commitments` refines an authored
conditional native output contract using the actual accepted argv. Its bounded
literal scan and regexp protocols return absolute output positions marked
`Written`, `Unchanged` or `MayWrite`. Consumers still resolve each physical
destination in native order: observers, unknown addresses and an earlier store
error retain their own effects. A proved match supplies neither stored bytes nor
an opcode. The regexp subset accepts ASCII literal text and mandatory capture
groups, with optional `-nocase` and `--`. Successful matching writes every supplied
output, including empty excess outputs; failed matching leaves them unchanged.
Alternation, quantifiers, character classes, escapes and non-ASCII input remain
unsupported by this bounded protocol. Unsupported matching syntax keeps the
original conditional output.
The source walker publishes a strong write only after it proves each ordered
physical store succeeds. A known later store failure retains earlier writes on
the error successor; an unresolved or observed store returns to the original
conditional protocol. Normal-only commitments never enter a joined error world.
Closed conditional writes retain a proved defined/missing presence union for
potential-missing diagnostics without proving that any output was written.
Opaque target or observer alternatives remain unknown.

`RepresentationEffect::ordinary_container_index_arguments` identifies native
index operands independently of the original list's conversion footprint. C's
single empty grouped index can return the input unchanged; Jim's native index
protocol differs. An independently proved numeric index cannot simultaneously
share a current ordinary List/Dict representation. That exclusion applies only
to ordinary-container sharing: index conversion still retires numeric receipts
and advances the representation epoch. Unknown nonnumeric index objects retain
their independent possible coercions, and C 9 abstract lists supply no ordinary
container proof. SpecTcl and Studio preserve both indexed-list and range layouts.

`InvocationFacts::ordinary_list_length_completion` bounds the authored native
list-length protocol to OK/Error only after selected `IntrinsicId::ListLength`,
its normal Leaf handler, accepted frozen argv and independent original ordinary
List/Dict evidence. Representation-effect metadata alone grants no completion
contract. Source analysis reuses the current final-operand coercion
receipt and requires agreement for every retained ordinary representation. The
query runs before coercion; command observers, unknown representations, C 9
abstract lists and replaced handlers retain their independent completion
uncertainty. This prevents an invented return alternative from rejoining the
pre-conversion representation at a procedure boundary. Error remains possible;
the query proves neither a normal result nor a completed store or opcode.

`NativeResultSelection::ordinary_range_literal_result(arguments, input, length)` is a selected range-byte producer. Its caller must independently capture the actual original current ordinary List/Dict input before native operand coercion, retain exact operand bytes and parsed length, and publish only after the native normal successor. The query shares the shape recipe's selected engine, index grammar and nonempty clamp. It validates the parsed length and uses `InvocationDialect::list_result_serialization` for canonical result bytes, including quoting, leading hash and non-ASCII elements. C 8.4's leading-hash serialization remains distinct from newer C and Jim. Full-range spacing is canonicalized too. Unknown input/indices, empty selection, inconsistent length or a byte result that cannot be projected to Unicode decline. These bytes establish neither object freshness/identity nor representation, effects, completion or erasure permission. Source capture stores optional bytes beside its separate shape receipt on the heap.

Use `InvocationFacts::argument_type_hint(index)` for selected positional representation advice. The index is an absolute frozen argv position after the head; an ensemble selector and actual option prefix remain separate positions. The facts retain the authored hint table and the boundary selected by the same available-option grammar as operand coercions. For example, `string map -nocase mapping subject` exposes mapping advice at index 2 and subject advice at index 3, preserving the mapping's `transparent_from` list. Unknown option boundaries, expansion cardinality, unresolved members and rejected arity return no hint. `NormalRepresentationInvocation::argument_type_hint(index)` additionally requires its existing actual normal-handler proof and reads the retained facts; it takes no registry argument. These hints describe possible conversion cost and establish neither the input's current representation nor a successful conversion, result, allocation or compiler route.

For an independently proved native scalar math implementation, `InvocationFacts::normal_scalar_math_protocol(arguments)` projects its authored operand conversion and normal result category. Supported recipes are `NativeResultContract::ScalarMath(Sqrt)` for C Tcl and Jim, and `NativeResultContract::ScalarMath(Double)` for C Tcl. Both require exactly one operand, native double access, and supply a normal Double category with OK/Error completion after operand evaluation. Jim numeric-unary `double` does not borrow the C scalar conversion recipe. C Tcl reads existing integer/double representations without changing their numeric category; Jim 0.84 may replace an integer with its coerced-double cache, so the prior category weakens to Numeric. Both paths still account for actual shared-object conversion separately. C 8.4 and Jim fixed tables use `CommandRegistry::fixed_scalar_math_protocol` with the retained registration's native registry identity and actual arity; a fresh function name or the C 8.5 wrapper catalogue floor cannot replace that identity. Mutable command-table dispatch instead requires the actual selected callable implementation. `protocol.input_requirement()` separately requires a current native numeric object or independently closed string access. `InvocationFacts::refine_scalar_math_input_effects` adds the host-callback residual when that input proof is missing, while retaining the native Double/OKError result contract. A custom string updater can change a procedure-local variable during normal sqrt; ordinary outer List/Dict shape cannot close nested element updaters. Result category establishes no number, object freshness, compiler route, erasure, operand-effect closure or observer immunity. Publish it only at the actual normal handler boundary before command-leave observers. C Tcl's expression compiler explicitly excludes function calls from constant-expression folding, including calls with literal operands.

Scalar numeric adapters select `InvocationDialect::scalar_numeric_input_policy()`, which retains an actual C native family/version even when no explicit core point is stored, or delegates an exact point to `tcl_syntax::number::NativeScalarNumericInputPolicy::for_point`. A vendor compatibility version cannot supply that native-family proof. The C LengthDelimited byte helper applies to guest-object inputs; Jim 0.84 borrows the prefix before the first NUL before checked Unicode projection. Direct native C getters also depend on the original storage class: raw string getters accept a NUL prefix on C 8.5 and later, while equivalent bytearrays materialize modified UTF-8 and reject it. Complete scalar getter parity therefore also requires the selected original object storage and string-generation protocol. This protocol is limited to scalar numeric/boolean getters and does not truncate expression source, equality, membership or names. It parses nothing and preserves the original object bytes, cache identity and error-presentation operand. Unknown engine points abstain through the typed host capability path. Runtime and VM adapters commit successful native conversions on the original object, rather than manufacturing a replacement from the prefix. Integer failure presentation is a separate selected protocol: `NativeIntegerErrorPresentation::Jim084Quoted` clips the original error operand at its first NUL, while the current Tcl 8.6 presenter retains materialized guest operand bytes. Raw C host-string diagnostics also clip at NUL and require the storage/getter-specific projection; neither presenter supplies numeric acceptance.

The pure getter protocol selects grammar, conversion, cache and error records.
Its records do not themselves mutate VM or Runtime objects. The
`InvocationDialect::native_scalar_getter_protocol()` selects only actual native
engine authority. An explicit core point must agree with any retained native
family and C release; contradictory axes decline. Without a core point, only
the authenticated C family plus its actual C release suffices. A vendor's
compatibility release, caller numeral/lexer overrides, or a missing engine
cannot select a primitive getter grammar. Unknown or unaudited engine/build
axes use the host capability refusal before publishing a guest completion.

Reached expression operand conversions use a separate Registry recipe in
`native_numeric_conversion.rs`. The retained C8.5+ prepared numeric `<` stage
initially yields Numeric; `with_original_cache_class` refines only independently
proved current original String/List/ByteArray/Integer to Int, and Double to
Double, after an actual normal integer-only numeric conversion. Dict and unknown
cache classes keep Numeric because normal comparison can use string fallback.
Runtime index conversion
has its own selected layout/compilation recipe: immediate bytecode indices and
conversions of grouped children do not publish on the original object. These
recipes retain original spelling and never grant canonical numeric output.

The Syntax protocol separates getter kind, input materialization and
cache transition. A conversion record retains the parsed cache value and the
success/error outcome independently: a failed getter can already have cached a
bignum or NaN, and a successful wrapped Wide return is not the cached magnitude.
Adapters first select `cached_conversion` on the actual existing cache. If it
returns no fast path, they obtain the original native materialized string and
call `fresh_conversion`; `None` retains a missing native dependency and becomes
a host capability abstention, never a guest Invalid parse. Before committing a
returned conversion they fulfil `requires_string_materialization` on the
original object. `into_parts` retains that preparation flag, the full cache
change and the independent outcome. They then apply the cache even on failure
and finally publish the getter return or actual guest error. Original bytes
and identity remain retained; no temporary prefix object substitutes for them.

Primitive error presentation must retain the failure's actual origin. A fresh
number parse and a cached Double rejected by Wide extraction can report different
codes and byte extents on the same release. Its error-code update distinguishes
`Unchanged` from `Set(bytes)`: C 8.5 cached Double-to-Wide and NaN Double failures
preserve an existing interpreter code, whereas other measured failures explicitly
replace it. The concrete error/completion owner applies that update to the actual
interpreter state; `Unchanged` does not mean an assigned `NONE` code. Primitive
message bytes also remain separate from propagated interpreter completion: C 8.5
can retain an embedded NUL and trailing bytes in the primitive cached-Wide error,
then truncate the exposed message during `Tcl_Eval` error propagation. Neither
stage borrows primitive Boolean diagnostics for unary expression truth or
mathematical Boolean conversion. Concrete adapters apply the typed primitive
error record to their actual completion and interpreter state.
Its quoted-value renderer also requires the actual native string-unit policy:
the measured C 8.4 byte cut and C 8.6 native previous-boundary cut can retain
the first byte of a four-byte sequence, while C 8.5 and C 9 cut before that
sequence. C 9's forced Unicode append can
map a naked leading `0x80` through its pinned CP1252 table; a Rust UTF-8 boundary
check or an assumed Latin-1 mapping cannot replace that native operation.
`native_tcl_utf::NativeTclUtf` owns that selected decode/encode/boundary policy,
including C 8.6's previous-surrogate decoder state. ByteArray materialization
and primitive diagnostics consume the same native unit owner. The primitive
`failure_presentation` record exposes `message_bytes`, `error_code_update` and
the distinct `eval_message_bytes` projection; unsupported failure/stage pairs
abstain. Prior error state, octal failure origin and primitive versus propagated
bytes remain independent inputs.

Jim's fresh Wide signed MIN/MAX depends on native retained `errno == ERANGE`.
Only independently captured native range state may select
`fresh_conversion_with_range_error`; Rust process errno or the last guest
error supplies no such authority. Unknown state makes these fresh boundaries
abstain. Existing native Int and coerced-integer caches bypass that native
string-parse dependency.

| Operation | Actual protocol obligation | Object and completion ownership |
| --- | --- | --- |
| String materialization | Retain RawString versus ByteArray and any existing native cache. C bytearrays generate modified UTF-8, including `C0 80` for a zero byte; the equivalent raw string retains its zero byte. | Exact bytes precede checked Unicode access; `new_bytes` remains a raw String constructor, while actual binary producers retain their ByteArray recipe. |
| Wide getter | C 8.4–8.6 accept unsigned-64 magnitudes with a signed return; C 9.0–9.1 require signed-64 range. Jim's selected unsigned conversion has its own saturation/sign behavior. | Returned `i64` and cached integer/bignum remain distinct. Preserve cache changes even on an overflow error. Arithmetic numeric interpretation has no Wide range limit donated by this getter. |
| Double getter | Fresh C 8.4 strings follow `strtod`, including hex floats and range errors; C 8.5+ follows the actual release's native number parser. Jim first tries decimal-wide coercion, then its `strtod` grammar. | Retain Integer, Bignum, Double or Jim CoercedInteger separately. C 8.4 converts an existing WideInt to Double; later C preserves it. Cached NaN acceptance also differs by engine/release. |
| Boolean getter | Cached numeric objects and fresh boolean words are different stages. Jim's fresh table uses exact case-sensitive full words; its cached Int bypasses that table. C word matching, numeric fallback and byte extent vary by release. | C word-Boolean caches do not grant Integer representation. C 8.4's word stage scans original bytes before NUL comparisons; its numeric fallback validates full length. Later C word matching is length-delimited while its numeric fallback can consume a raw NUL prefix. |
| Error publication | Selected getter kind, original storage/string generation, fresh-versus-cached failure origin and full original bytes determine primitive presentation. Retain `Unchanged` versus `Set(bytes)` error-code updates and the independently propagated interpreter completion. | The interpreter owner applies the update to actual state; it never substitutes `NONE` for `Unchanged`. UTF-8 projection failure is not a native guest parse error. Host refusals bypass Tcl capture/finally; native parse/range failures remain guest completions, with their actual cache effects. |

| Consumer | Required operation |
| --- | --- |
| Registry `InvocationDialect` | Project the actual engine with conflict negatives before delegating to the Syntax getter protocol; keep scalar byte-input projection separate from original storage, materialization and cache ownership. |
| VM `Value` and `ValueOps` | `native_scalar_probe` and `native_scalar_getter` consume the selected getter/cache record on the original object. Accepted C 8.5/8.6 Wide returns require an exact physical bignum cache; keep pooled expression Boolean results separate from native word-Boolean caches. |
| Runtime `typed_value`, `obj` and `ValueOps` | Retain actual RawString/ByteArray materialization and existing integer, bignum, Double and Jim coerced-integer ownership. Explicit Double conversion uses its own recipe; it does not replace arithmetic's integer-first preparation. |
| VM and Runtime expression scalar operands | Reuse only the selected materialization/operand stage that the native expression door actually reaches. Boolean conditions, mathematical numeric interpretation and explicit Boolean getters retain separate acceptance and completion protocols. Expression source, equality and membership remain length-delimited. |
| Portable Core commands | Continue through the concrete selected `ValueOps` adapters. No command-local prefix parsing, cache inference or diagnostic rendering. |
| Compiler/source facts | Native normal result/category and object-effect receipts remain independent. Primitive getter parsing supplies no constant, stock-object identity, fresh result, completion envelope, opcode or erasure permission. |

Scalar byte-input helpers do not supply the original storage class, native
cache, primitive error-state update or propagated completion. Adapters must
retain those inputs independently when consuming a getter conversion record.

Reached invalid or incompatible expression operands retain their selected
`NativeExpressionOperandStage`. For actual C implementations,
`InvocationDialect::expression_invalid_type_error_code` supplies independent
`direct_update` and `eval_update` records. C 8.4 direct expression APIs leave
the prior error code unchanged, while interpreter Eval propagation assigns
`NONE`. Later C retains the selected structured type code at both boundaries.
Jim uses its separate expression presenter; the C direct-API record abstains.
`expression_operand_error_code_policy().select_invalid_type_code` supplies
expression type-code metadata only. These projections change neither messages
nor acceptance and cannot select primitive getter, arithmetic, range, domain,
NaN or parser presentation.


`InvocationFacts::deferred_script_argument_indices` returns explicit optional
coverage of selected single-script/prefix positions. Unknown selector, expanded
cardinality, unsupported value-sensitive timing or deferred concatenation
returns None. Known original positions are positive navigation metadata;
captured alias values have no original written argument. Even a known empty
position set cannot establish no callback effects, edit completeness, erasure
or executable future entry.


Native list-method behavior specimens and their interpreter usage are documented in [`native_list_methods/README.md`](../../../rust/tcl-syntax/tests/data/native_list_methods/README.md).

For C Tcl 9 list length, consume `InvocationFacts::list_length_object_protocol` on the retained actual arguments before applying leaf world preservation. `AbstractLength` requires an independently current ordinary container or an audited native result-provider receipt. A registered native `lengthProc` can change the interpreter world even though it returns only a size: custom objects can modify variables in both Tcl 9.0 and 9.1. The stock arithmetic-series method only reads its stored length. `NativeResultContract::ArithmeticSequence` therefore describes the reached native sequence result's read-only length provider, independently of the producer's operand effects. Replaced sequence creators, unknown object types and semantic List hints do not establish that provider. The separate `SuccessfulHandlerSpec::ArithmeticSequenceArguments` closes normal producer effects only when the shared native `lseq::decode` accepts every frozen numeric/keyword operand. Unknown operands retain opaque producer effects and arbitrary completion residuals; native sequence operands are number-converted and are never evaluated as Tcl expression scripts; their normal successor alone may carry the native result provider. Tcl 9.1 can retain that purpose-only result under an Unknown compiler route without granting compiler entry or an opcode. Representation metadata alone continues to grant no completion or callback closure. The source kernel records effect closure at the original invocation site before handler coercion. Strict and normal invocation projections share this private receipt: an absent or differing receipt adds host callback effects and withdraws purity, while native compiler selection remains independent. Unknown object methods also withdraw the reached source world. A stored audited provider follows the exact physical cell and current representation lifetime; writes, shared coercions and opaque callbacks invalidate it. Its strict representation remains Unknown and it never enters the ordinary List/Dict alternative domain. Length closure alone cannot be reused for index, slice or iterator methods.

`InvocationFacts::native_list_method_requirements(frozen_arguments, actual_compiler_selection, proved_native_numeric_indices)` supplies the runtime callback footprint for list length, indexing, slicing and paired-list iteration. Each `NativeListMethodRequirement` retains its absolute frozen operand and method: Duplicate, Length, Index, Slice, Elements or StringAccess. The conservative ordered footprint may include alternative branches; consumers intersect independently current object-method proofs and never replay it as an execution transcript. A grouped index or nested element residual prevents closure. Current native numeric index-object proof can remove only that original grouped-index object residual; it supplies no index contents or successful conversion. Generic foreach parses and duplicates variable-list objects at runtime; selected Inline foreach consumes variable-list syntax during compilation, so its runtime footprint includes value-list objects only. Unknown compilation retains its preparation residual.

Stock conversion effects apply on all native engines. A custom top-level scalar can run string-update or intrep-free hooks during a list conversion, even before a zero-trip `foreach` exits. `Conversion` therefore retains operand obligations on C8 and Jim rather than treating the absence of Tcl9 abstract methods as world closure. Current original stock-literal class or native numeric receipts can close their exact object access; ordinary List/Dict Length has separately measured safe member behavior. `StringAccess` obligations for index words and Generic runtime variable names require their own stock-class proof: an ordinary outer List/Dict does not prove nested members' string hooks are closed. Completion, intrep publication, compiler preparation and callback effects remain separate axes.

Implicit scalar math has its own numeric cache protocol. Expression `sqrt` preserves Integer input categories on the supported C releases, including Tcl 8.4. Direct Tcl 8.4 Double getters can change an Integer cache, so their getter metadata cannot replace the selected expression protocol. Jim's input-category weakening remains separate from the normal Double result recipe.

`command_binding/literal_object_pool.rs` owns a separate private ordinary literal-pool provenance. Only a fresh authored Tcl source entry creates it; original unchanged Source words must match their frozen operand bytes and owning origin. Joins intersect this authority, and unknown host/object effects or runtime-entry installation permanently withdraw it. Constructor epoch recovery cannot restore withdrawn pool provenance. This closes effects only: it supplies no List representation, list validity, normal completion, result or compiler admission. Actual command-table snapshots and known literal text do not establish the current intrep of globally reused compiled literals.

The arithmetic-sequence provider exposes explicit `world_is_closed_for(method)` capabilities for the audited stock methods. Index and Elements can allocate or populate object caches; Slice can modify the receiver and invalidate its string representation. Closing interpreter-world effects therefore cannot preserve a representation epoch, producer purity, completion, result freshness or an opcode. Add a new method protocol in this registry query and audit its selected native compiler/handler branches before extending the source sidecar; do not recognize a written command name or borrow a Length capability in a consumer.

Runtime's actual C Tcl 9 `lseq` handler retains a genuine arithmetic-series primary through `native_arithseries`. Duplicated headers share its backing; Length and fresh unreferenced Index values do not populate the element cache or string representation. Generic foreach captures an actual duplicated abstract header and Length, then requests each Index immediately before assignment. Native comparisons cover 196 original storage windows and 36 iteration windows on C9.0/C9.1, including a 100,000,001-element sequence followed by an immediate break. VM and non-native Runtime generation still materialize concrete Lists. The unchanged 100 million element materialization cap applies to actual GetElements/String access in Runtime and eager `lseq::generate` elsewhere. Both retain `NativeValueAccessRefusal::Materialization(NativeMaterializationLimitError::new(requested, limit))` through `CmdError` before guest completion or option publication; catch and finally cannot consume it. The quota cannot prove native guest failure, alter source completion, or withdraw the native result-provider contract. Unauthenticated or foreign descriptors cannot issue native Length/Index permission. Audited signed-distance and floating-to-wide conversion frontiers remain checked refusals where native arithmetic does not provide a supported Rust result. The numeric decoder accepts no expression scripts.

Native normal leaf preservation is a handler contract, independently of compiler-hook selection. C String workers become independent command slots in 8.5; Binary format does so in 8.6. `EnsembleLeaf` requires the shared source kernel to prove both original mapping and worker identity on those releases. Old monolithic C implementations and Jim have no such helper dependency. F5 string and binary value leaves instead author an independent `NormalValueLeafProvider` contract for the documented TMM public handlers ([string](https://clouddocs.f5.com/api/irules/string.html), [binary](https://clouddocs.f5.com/api/irules/binary.html)). It requires the original converged public handler and accepted frozen argv; C private worker names have no role in this vendor protocol. This grants normal representation analysis only, with no native compiler, body, completion or error-edge licence. An unknown family or replaced public handler still declines. Missing actual native protocol remains unknown. SpecTcl packs explicitly reject this native descriptor, including vendor provider assertions, because they do not carry interpreter identity and observation proofs.

Closed C 8.6–9.1 Try compiler clauses reuse the runtime clause owner. Per-child `ExceptionRange` metadata supplies protected compilation environments; a bare Try body inherits its environment, while handlers require the native procedure-local compiler table. Runtime completion routing remains a separate shared body-flow projection. Unknown clause syntax retains residual execution rather than selecting a native opcode. Inline body coordinates use `ExecutedScriptSource::literal_word_base`, with lexer-owned token extent and exact evaluated bytes, so compiler guards cannot disappear through manual delimiter arithmetic.

Selected result intrep facts apply the registry return-type hook to retained
`InvocationArguments` at resolution. The common option scanner proves a
literal option prefix or positional boundary without replacing unavailable
slots with empty strings. This preserves unknown intrep for `regexp -inline`
and `lsearch -all`, and keeps a normal result-type projection separate from
native opcode or failure-timing admission.

`CommandForm.successful_handler` carries the same native normal-handler
contract at the selected invocation form. Niladic `pid` reads the current
process identity without channel lookup; `pid channel` retains the broader
handler envelope. The F5 `TCP::payload` getter returns collected bytes
([vendor contract](https://clouddocs.f5.com/api/irules/TCP__payload.html));
its `replace` member retains mutation effects and receives no getter proof.
Form selection never licenses native compilation, callback suppression or
completion timing. SpecTcl `refine` blocks reject asserted native handler
contracts explicitly, just as command/member blocks do; Studio's existing
`successful_handler` native-descriptor gap also applies to this refinement
axis until live implementation/observation proof can be authored.

`compile_native_coroutine` owns the original C8.6–9.1 Tailcall, Yield and
YieldTo compiler work. It consumes an independently selected compiler and the
complete unchanged parser vector. C8.6/C9.0 tailcalls evaluate the original
head as a placeholder, then replace it with the executing procedure namespace
at execution. C9.1 produces the namespace before argv and registers only a
simple compile-known first target with the command-literal role. Original
expansion-member coordinates, List segmentation and empty-tailcall cancellation
remain part of this recipe. A namespace-only YieldTo List retains its native
wrong-argument failure before suspension. Yield accepts zero or one operand;
earlier release or unsupported arity/frame/expansion declines remain generic.

The VM creation driver retains an original namespace/argv List and an admitted
internal activation, without reconstructing a script or minting a compiler
hook. Relay lookup consumes the original namespace object after restoring the
resumer's variable frame and keeps the original List alive through the target.
The Runtime's private serialised handoff transfers an owning reference to the
same creation argv, yielded value, resume value and final result; only one
interpreter thread accesses those objects at a time. Native wasm suspension is
a typed unavailable capability. The VM probe/injection adapters resolve their original coroutine-name object
to its live command token and dispatch the unchanged command prefix objects.
They preserve lookup refusal separately from a genuinely missing command. The
Runtime probe/injection adapters retain the actual prefix in a native List
owner and transfer that owner through the same serialised rendezvous as creation
and resume values. Probe results and appended resume operands remain original
objects. `NativeCoroutineInjectionProtocol` separately selects measured C9.1 callback
chronology: newest queued callback runs first, every callback replaces the prior
completion even after an error, and the appended kind follows retained yield
versus yieldto resume arity. Both ports use its fold while keeping actual object
invocation and cleanup in their interpreter owner. This pure selection supplies
no List header, compiler, frame, observer or Normal grant. Compiler
preparation, live header ownership and suspension capability never imply one
another. SpecTcl inline hooks retain their documented native-descriptor
authoring gap.

`DictionaryConstructor` is a separate normal-handler contract: dictionary
creation hashes/coerces keys and retains value objects. Known keys therefore
close its normal Tcl-world footprint even when value bytes remain unknown;
unknown keys retain possible coercion observers. The original C private
worker/mapping and release are proved through the same shared native-worker
query, including selected form descriptors. No other dictionary operation
inherits this constructor contract. Error effects and compiler selection
remain separate.

No-value variable append forms have independent normal contracts. Across all
five C cores and current Jim, `append var` reads existing bytes and errors
when missing. `lappend var` reads an existing value without storing it again,
and initialises a still-undefined variable to empty. The typed
`InitialiseEmptyVariable` contract omits that store when shared physical
contents are proved defined and no read observer can change them. Missing
proved contents receive the actual source store origin; uncertain observers
retain conditional effects. C validates the existing list, while Jim retains
its bytes without validation. Selected form value overrides withdraw inherited
mutation, result-intrep and conversion claims; their explicit tri-state DSL
limitation is recorded in Studio's `form_value_effects` gap.

`NormalRepresentationInvocation::value_assignment` retains a selected setter's
original value word for representation tracking even when native compilation is
unknown. It cannot create a store: the consumer still requires the physical SSA
definition. `NativeResultContract::DictionaryArguments` similarly retains exact
key/value slots; constructor bytes use the shared dictionary canonicalisation
owner, and an unknown value never becomes empty text. Its native dictionary
representation is independent of list coercion and numeric purity.

Connection-event entry preserves exact local bindings but admits unknown
incoming contents from the host and preceding handlers. This permits existing
descriptor-driven May body traversal without inventing execution after a proved
failed read. Explicit stores and unsets override that incoming contents residual.

### Native invocation realm

`tcl_dialect::model::InvocationRealm` and `SurfaceQuery::with_realm` own the invocation realm. Registry `irules_policy::runtime_surface_admits` and `rule_loader_refuses` separate the measured interpreter roster from authored-source refusal. `ResolvedContext::resolve_spec_in_realm` and `semantic::resolve_structured_invocation_in_realm` retain contextual/package filtering. Source entry, execution contexts and immutable lookup snapshots carry the realm; token adapters and diagnostics query that same retained point. Definition packs and editor metadata cannot create a runtime entry.

### Positioned command reference projection

`command_binding/command_reference.rs` owns `SourceCommandReference` and
`SourceCommandReferenceBinding`, privately minted `SourceCommandDefinition` and
`SourceCommandDefinitionKind`. `SourceInvocationBinding::command_reference`
selects the actual called slot from its post-argument interpreter snapshot;
`const_dispatch` consumes this navigation-only projection with reaching value
contributors. Direct alias identity and imported origin identity remain
separate. The carrier cannot license an executable target, opcode, normal
completion or current object class. Container-dispatch consumers must also
retain physical read and exact source-entry producer provenance; they must not
reconstruct targets from a lexical definition inventory.
`SignatureCommandInvocation::{written,retain_reference,clear_positioned_reference}` centralise recording and
atomic navigation projection. Signature and workspace records retain the typed
current definition and complete called reference, distinct from command tokens
and slots. A known non-definition cannot fall back to a same-named class/procedure.
`SignatureCommandLookup` retains invocation, consumed-name and deferred-reference
purposes. The LSP `invocation_reference_at` selector includes those positioned
references; `invocation_head_at` requires `is_execution_site()`. Editable name
arguments therefore retain navigation without donating execution-head authority.
`AnalysisResult::{proc_for_definition,class_for_definition}` validate declaration
category, complete authored source and exact original declaration tokens; foreign
or overwritten records cannot fall back to same-named assistance.
`AnalysisResult::body_lexer_config` retains the actual selected grammar for that
token lookup. `retain_class_declaration` preserves displaced class declarations,
and fragment relocation handles current and displaced allocation records on the same path.

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

The selected stock Length cache owner is `InvocationFacts::stock_list_length_protocol` plus `command_binding/container_coercion`: actual handler/argv/engine selects the protocol; an unchanged successful original physical read and sealed input class supply its precondition; only normal completion publishes List, preserved cache summary or Unknown. Tcl 9 numeric preservation and StockUnknown refusal are independent of bytes and semantic types. `normal_empty_list_root_provider` supplies Generic zero-argument constructor root effects only; the source representation owner intersects that capability with actual List alternatives without supplying a List cache or StringAccess. Consumers use the shared SSA/physical read queries, never the provider name as an emptiness/value/freshness proof. Ownership and query usage are documented in the implementer guide's “Selected stock Length cache and empty constructor effects” section.


Original numeric input effects are owned by `RepresentationEffect::CoerceNumericValues` / `InvocationFacts::numeric_object_conversion_arguments` and `native_rmw::NativeIncrementObjectProtocol`. Exact accepted argv and authentic native axes select obligations; source callback-effect inventory closes only independently retained original stock/current numeric classes. Increment preserves its authored BeforeRead/AfterRead amount order and checks the protected old cell after completed read callbacks without a second read. Missing proof withdraws effect/purity and source worlds independently of selected implementation and normal numeric result/category. The shared SSA, shimmer, folding and native consumers must not manufacture input closure from semantic types, contents, normal success or primitive getter policy. See the implementer guide's “Original numeric conversion hooks and reached Increment inputs” for input ownership and callback/dependency withdrawal requirements.


### Native glob inputs and consumers

`NativeGlobProtocol::for_native_point` selects a supported canonical C or Jim
build. `from_name_policy` preserves the selected provider and its issuer;
`authored_tcl` selects an explicit logical simulation. None authenticates an
object class, closes callbacks or establishes live command visibility.

For named operations, materialise each original operand through its retained
native string provider before calling `match_name_pattern`. `ExportFilter`,
`ImportSearch` and `InfoCommandsSearch` retain their distinct native scan and
exact-lookup rules. Jim export imposes no filter. Jim info patterns use complete
object bytes; Jim import uses its canonical CString extent. C scans use native
CString units, including invalid byte units and C8.6 supplementary fusion.
A trivial C8.5+ `ArrayNamesSearch` pattern compares the original complete object
key even though the triviality predicate inspected only its CString prefix.
`InfoVariablesScan` applies only after the caller selects a scan. Namespace,
compiled-local and dynamic-local lookup selection remains the variable-table
owner's responsibility; a combined row inventory cannot infer those origins.

Object matching requires `NativeGlobObject` describing the actual original
NULL type, String type, cached native units, pure byte array, byte array with
its independently retained resident string, or materialised other type. Cached
units, byte-array backing and resident string bytes remain separate. Counted Unicode
matching consumes the actual units; a CString path requiring absent resident
cache bytes refuses until the concrete object owner materialises them. Unknown
object storage refuses. These APIs never substitute Rust UTF-8 equality or
Unicode casing for a native matcher. The Unicode compatibility functions in
`glob` are suitable for analysis/display text, not native byte operations.


### Original byte scripts and compilation targets

Retain a runtime script with `SourceImage::native(original_bytes)`. A document
uses `SourceImage::document(text)`; `from_bytes(bytes, channel)` requires an
explicit channel when the source already has a channel policy. Equality and
hashing retain both bytes and channel. Native value scripts do not translate
escaped CR into source-channel LF continuations. The image carries no object
class, native compiler identity or logical dialect proof.

`source_backslash_fragment_in` is the lexical owner of original continuation
extents. `native_arena_text` forms executable text from those original spans,
using the independently issued source-string protocol. `source_literal_bytes`
and `source_braced_word_bytes` select Document translation and braced folding
without replacing the original image. Document CR/CRLF becomes value LF;
NativeValue retains raw CR/CRLF and recognises only backslash-LF continuation.
C Tcl's default `source` file channel translates CRLF; current Jim `source`
preserves it. A source issuer retains the actual channel, rather than inferring
it from byte contents or the physical compiler point. Escaped NUL uses the
selected native string encoding; a counted raw NUL remains an original byte.

Use `Lexer::with_source_image` and `group_commands_bytes` for segmentation.
`SourceMap::bytes` and `token_bytes` preserve original source extents and share
the delimiter rules used by Unicode consumers. `first_parse_cut_image_checked`
retains the same channel while descending into command substitutions and
array-index scripts; lexical unavailability is distinct from `Ok(None)`. An opaque byte never makes a script empty or suppresses a nested parse
error. Unicode assistance may call `try_text` or `try_source`; decoding failure
withdraws that view without changing the original source.

A `ScriptCompileTargetBytes` retains the source image and an already constructed
`ByteNamespacePath`. A `ProcedureCompileTargetBytes` also carries actual bound
`NameBytes` formal keys; defaults remain original runtime values. The compiler
retains every formal declaration slot through `LocalVarTable::from_native_names`.
Compiler-local selection uses the independently issued
`NativeCompiledVariableProtocol` and `find_native` / `intern_native`; it does not
change retained `NameBytes` when the native comparison selects another slot.
Local-slot interning and replay
metadata preserve bytes and segment boundaries independently of display text.

Code generation projects original variable words with `native_variable_word`
before selecting an address. `CodegenCtx::command_variable_slot` applies the
selected compiler's declaration or borrowed-layout rules to the original
scalar name or array base. Source substitutions use the separate
`substitution_lookup` purpose. A resolved value or a decoded backslash word
does not establish the original token geometry needed for a compiled slot.
Allocate targets before emitting their value arguments. Indexed instructions
and iterator auxiliary targets retain physical slot indices; they do not
repeat lookup by a displayed name. `CompiledVariableTarget::Name` carries
counted names for authored dynamic backend operations. Native compiled
foreach targets use `CompiledVariableTarget::Slot` and the shared cell-write
operation, including its traces and completion handling.

Original byte command emission uses one heap work list for words, array
indices and nested scripts. Every nested command retains its complete lexical
extent and exact input channel, including closing delimiters. Emitter source
state is restored when that command finishes. Generic command boundaries and
registered compiler prerequisites carry that same original source for error
reporting and replay; representative token spans do not widen it again.
`SourceCompilationScope::EnteredSource` prepares only the script or procedure body
selected by a runtime target. Typed `BytecodeCompileService` script and
procedure APIs use this policy for both byte and Unicode source, optimised and
plain dispatch, with or without a live compilation entry. Nested procedure
words remain in the original invocation; their bodies are compiled upon actual
activation. The retained source-analysis entry carries this scope. Deferred
declaration analysis, procedure-body lowering and method inventory are excluded
before CFG preparation; nested declarations still retain their original words. Generic script-bearing invocations keep their complete original arguments and do not analyse or lower runtime child bodies during parent compilation. Actual compiler-selected structured bodies keep their native traversal and preflight checks. A runtime entry for a namespace, evaluation or procedure body prepares that source separately.
Unentered procedure assembly, provenance and body source proofs are absent
from these artifacts. `SourceCompilationScope::WholeModule` is the
explicit AOT policy used by the public whole-module codegen APIs and the
service's `compile`, `compile_for_profile` and traced whole-module entries.
It includes the owned procedure inventory. `ModuleEmissionScope` names the
same shared scope at the emitter. Neither policy changes native
admission, source bytes, the selected namespace or argument evaluation.

A runtime compiler must implement the byte CompileService entry explicitly.
Its default reports unsupported ingress rather than decoding, rewriting source,
reconstructing names from display, or silently selecting another namespace.
`script_command_plan_bytes_with_entry` uses the retained entry's grammar and
reports the complete original byte prefix plus the exact malformed command
bytes. Physical compiler selection and logical invocation policy remain
independent requirements of executable lowering.

The command-plan APIs return `Result`: an authentic malformed command is a
successful plan with a fatal tail after its complete prefix, while unavailable
lexical ownership is a host capability refusal before effects.
`native_script_words_in(image, region, config)` supplies complete command words
for an already selected original script region. Word/command spans and fatal
cut offsets address the full retained image; the command ordinal is local to
the region. Inner BOM bytes are content. Nested bracket compilation retains
this same image/channel and body span rather than copying a new native image.

`NativeScriptWordsPlan::source()` retains the whole original source image and
input channel even when there are no complete commands. Empty plans and fatal
tails do not recover their source from the first word or reconstruct bytes.
An unprojected expanded head selects generic native dispatch only when an
authentic C8.5+ parser projection retains that original `Expanded` word. Literal
head expansion uses its independent projected compiler lookup; an unavailable
recipe cannot turn it into a generic dispatch. Unit cache prerequisites validate
the exact immutable compilation-entry world with ChunkEntry guards. Each
original Named invocation separately retains its BeforeArguments
compiler-selection guard; later child worlds remain instruction-scoped.
An authenticated Generic selection without a preparatory recipe uses generic
dispatch. Inline and preparatory selections require their supported original
emitter; an unavailable emitter cannot donate Generic selection.

`NativeCompilerWords::capture` retains the full original word vector, token
shapes and static byte values under the independent source-word protocol.
`literal(index)` distinguishes a static empty value from substitution; opaque
static bytes do not become dynamic or replacement Unicode. All indices,
including `NativeCompilationSpec::select_native_words`'s `operand_from`, address
the full original vector including its head. Shape/count compiler grammars use
this view directly; value-dependent grammars require an unchanged checked value
view or retain `Unknown`. Static expansion requiring native parser flattening
also retains `Unknown` until that original-token projection is available.
An authored compiler descriptor does not attest a runtime registration.

`list_recipe(operand_from, physical_version)` distinguishes the registered empty
string, runtime element assembly and C8.6+ private constant List construction.
Members retain exact static value bytes. Expansion without the original parser's
flattened layout is unavailable. The compilation descriptor separately requires
a procedure compiler for C8.4/C8.5 List selection; hook registration remains
present in a script frame even when that compiler declines the command.

`increment_immediate(original_index, physical_version)` consumes the original
amount word and the shared fresh scalar getter. C8.4/C8.5 select GetInt, including
its native narrowing; C8.6+ select GetWide. C8.6 additionally requires an integer
cache. Earlier compilers require SIMPLE_WORD, while C9.1 accepts any known word.
The immediate range is -127 through 127. Missing/substituted amounts return
`None`, requiring evaluation of the original word; this does not assert that
native integer conversion fails. `compiler_integer_prefix84` is only C8.4's
prefix predicate, followed by the independent full GetInt conversion.

The compiler's native byte plan resolves original heads through the immutable
entry's exact byte lookup, retaining command token, generation, selected compiler
and original validation boundary. Ordinary set/increment/list opcode recipes
have no additional private path. Uncaptured ensemble configurations, unknown
registrations and unsupported recipes retain the provider obligation. Captured
original configurations retain every selected nested compiler prerequisite
before operands. Nested compiler hooks must
also close before the whole script can be admitted. A registered plan requires
an emitter for that exact operation; it grants no normal-handler effect, purity,
value, current cell or runtime object-class facts. Command and namespace
mutations apply the selected native compiler and resolver invalidation rules.
Retained prerequisites validate at their declared boundary before argument
evaluation; command-reference changes alone do not imply native compiler-cache
invalidation.

`closed_original_byte_compilation(module, entry, registry, context)` checks every
original command and bracket child in the same immutable source images. Generic
commands need a closed lookup or an absent compiler hook; registered commands
need exact registration, supported descriptor selection and the original
variable/List operand recipe used by the emitter. Missing issuers, unknown
hooks, unsupported operations, unavailable geometry and unclosed child scripts
retain the provider obligation. The generic-only certificate remains stricter
and never licenses a registered operation.


`NativeWord::from_group(image, config, tokens, group)` retains the original
shared lexer group, its exact token fragments and full written extent, including
expansion markers and final delimiters. Its normalized token indices address
`NativeWord::tokens`; source spans and decomposed part extents address the
retained image. `parts()` delegates the common substitution scanner; names,
command bodies and Jim expression sugar borrow original byte slices. Literal
escape runs use the byte decoder. Braced content remains uncollapsed lexical
text; the selected braced-value owner performs continuation collapse. Jim
concatenating quoted words may have multiple content ranges, so `content_span`
reports `NonContiguousContent` while `parts` retains their fragments. Missing
closers, invalid group geometry and unavailable token streams cannot become an
empty word. `try_text` is only an unchanged checked advisory view. A lexical
word does not grant native compiler admission or an object representation.

Executable callers use `ExecutablePartArena` and
`NativeWord::executable_parts()`. The arena owns the sole immutable `SourceImage`;
its component spans, variable names and command/expression bodies address that
image. `ExecutableText::Original` selects the raw span, while `Decoded` retains
only the shared escape decoder's output. `root()` and `list(PartListId)` retain
ordered component lists; variable indices refer to child IDs from the same
arena. Construction queues raw indices through the existing scanners without a
recursive component tree. Consumers traverse those IDs explicitly, preserving
variable-name/index/read and concatenation order. `source_span(&component)`
retains the shared native token extent separately from the full raw component
span, including closer and empty-name conventions.

`ExecutablePartArena::decompose` takes original image coordinates, substitution
flags and lexical configuration. `decompose_template` additionally takes the
independent native template variable grammar; Jim template acceptance cannot be
borrowed for a written word. Geometry failures return
`ExecutablePartsUnavailable`; authentic syntax errors remain `ParseError`
components after earlier components. Neither result invents equivalent Unicode,
a literal fallback, a selected handler or compiler permission. Native grouped
word construction validates syntax across every arena list before publishing
immutable words. Arena source data does not change with command or variable
mutations; any current execution receipt remains a separate dependency.

`ExecutablePartArena::from_inputs` accepts original literal/substitution regions
selected by a lexical owner. It preserves one image and uses the same scanners;
it does not manufacture source or infer execution authority. The Syntax
`native_variable_word(word, physical_version, source_protocol)` projection
models the original C variable compiler's token layout. A SIMPLE_WORD/TEXT gives
a counted scalar/root and optional literal index, including an empty index.
An original first TEXT containing `(` and last TEXT ending `)` gives an index
arena from the same image. Braced index data keeps literal `$` and `[`; only
its original continuation tokens substitute. Other words remain DynamicWord,
even when backslash decoding could produce a known name. C8.4 braced operands
remain dynamic under its native compiler rule. Geometry/decoder failures are
explicit unavailable results. LVT eligibility, local comparison and physical
cell lookup use their separate native-purpose owners; none follows from this
lexical operand plan alone.

Typed and command-substitution increment emitters share original compiler-word
projection from retained invocation images and source extents. They use the
physical compiler point for token/amount rules and the independent source-word
issuer for values. Original targets and non-immediate amounts emit through the
same arena owner; array index evaluation precedes amount evaluation. A supplied
entry with an unknown physical point cannot borrow an authoring profile.
Unlocated evaluated IR amounts remain values rather than new variable reads.
The no-entry authoring compatibility branch remains distinct from native proof.

Info/array existence emitters use the same original scalar-operand projection
and `command_variable_slot`; they do not allocate a local through a rendered
name merely because a procedure frame exists. A borrowed layout can supply an
existing eligible slot and retains its layout receipt. Missing, computed and
array-element operands cannot manufacture a scalar slot. Unsupported source
geometry keeps the stack/dispatch path or the native provider obligation.

Bounded compatibility callers may use `decompose_spanned_checked`, which reports
`DecompositionUnavailable::IndexNesting` when the retained advisory tree would
conceal substitutions. `NativeWord::parts()` reports
`SubstitutionDepthUnavailable` for that compatibility projection; its full arena
remains available. Unchecked advisory `decompose_spanned` cannot establish
constant values, complete reads/effects or executable authority past its budget.
The fixed 2,000-index byte fixture is parsed and evaluated by all five C releases
and current Jim with result bytes `78ff`; the arena construction/drop control
uses a 64 KiB thread without raising the shared compiler execution budgets.

The VM's byte compilation cache keys exact `SourceImage` and constructed
`ByteNamespacePath` together; document and native-value channels do not share
an entry. The cached actual compiler entry separately retains interpreter,
namespace generation, physical compiler and logical invocation policy. Changes
withdraw reuse through the existing entry and command-assumption checks.
`compile_plain_script_bytes_with_entry` keeps actual compiler prerequisites even
when command specializations are disabled. An embedder uses
`Vm::try_compile_function_bytes` to retain an original image in a reusable
function handle. Byte-source provider refusals retain exact source/path fields
outside guest Tcl completion; they do not select another namespace or rewrite
source into Unicode.

Compiled binding and replay transport uses `CompiledNamespaceContext`.
`Native` retains an actual `NativeNamespaceContext`; `ConstructedPath` retains
exact `ByteNamespacePath` components without asserting an actual incarnation.
`CommandBindingIdentity::namespace_context` and
`ProcedureBindingIdentity::namespace_context` take precedence over their text
context when present. An absent context preserves the explicit compatibility
text route; it does not turn that text into a native identity.

`Instruction::source_namespace_context` and
`NativeOperationSelectionSite::replay_namespace_context` return their retained
context, or exact constructed-path geometry when the optional field is absent.
The VM validates a native context against the current interpreter, actual token,
retained path and live or retained namespace owner. A constructed path resolves
through the selected component-based namespace owner. Neither branch reparses a
rendered name. Invalid native identity is unavailable rather than a request to
find a same-spelled replacement. Replay enters the validated namespace without
creating a Tcl call frame. Cross-namespace replay at global level refuses because
replacing that immutable global entry would change `uplevel #0` and coroutine
flow semantics. Assembly validation rejects a source-command context whose
component geometry conflicts with the retained source-command path.

Native TclOO method operands retain their counted bytes as primary keys. Object
commands use the selected object-declaration slot and actual namespace token.
Method enumeration and unknown-method diagnostics have separate reporting
recipes; a displayed method name is not a lookup key. The VM stores exact
forward and reverse byte indexes around opaque method storage identifiers.
Hidden command tokens use an independent exact byte index and retain their
original invocation Values through trace and usage presentation. Shared
namespace failure reporting receives the actual operation, release and current
namespace name; dictionary missing-key reporting receives the reached get or
intermediate-unset operation and does not alter counted dictionary equality.

VM method rename checks the original source table entry before reaching the
destination Name getter. Its weak record observation carries the actual target
and table facet and adds no entered Method or clientData owner. The same record
must still occupy the source slot after conversion and the destination slot after
splice and cache retirement. A recreated slot cannot continue the earlier
selection; same-record payload changes remain visible. Checked Name and record
currentness retain the original typed first cause. These VM allocation and table
checks are separate from C record-name, table-key and physical reference roles.

The VM's `cmd_oo::original_name_operands` owner retains the entered
`OriginalNameConsumerScope` through checked Name getters and owned cleanup.
Object selection captures the actual object name-owner allocation, creation ID
and namespace token; class selection also captures the actual facet birth.
These weak observations add no native references or callable owners. A changed
object or class facet cannot supply the continuation of an earlier selection.
Private-variable lookup additionally checks the declaring provider selected by
the genuine entered method-chain step. Ordinary method and declaration changes
remain permitted when those original owners remain current. Definition workers
enter their real frame with the same operation currency, then restore the caller
frame before final settlement. Target checks guard subsequent Name, metadata and
publication stages; executing an ordinary constructor, method or definition body
does not require the object to survive its own legitimate lifecycle effects.
Checked String/header currency supplies no object lookup, class, cache, alias or
variable-storage purpose, and these software guards do not certify native class
pointer lifetime or physical method-retirement chronology.

### TclOO variable declaration objects

Select the native declaration operation with `native_oo_variable_slot` using
materialised original operand bytes. C8.6 exposes `-append`, `-clear`, and
`-set`; C9 also exposes `-appendifnew`, `-prepend`, and `-remove`. Method names
are exact, and bare names select append. Arity presentation uses the shared
native argument-usage owner with the original invocation values.

Retain each declaration's original value and full counted `NameBytes` key.
`apply_native_oo_slot_records` retains original slot members through native
Resolve/Get/Set calls; append and prepend preserve their duplicates.
`apply_native_oo_variable_slot` selects declaration records using counted ObjHash
membership and declaration order. Validate the final list with
`validate_native_oo_variable`: validation examines the CString prefix for
namespace separators and the native `*(*)` pattern. An invalid removal operand
that does not remain in the list is not a declaration-validation failure.
Introspection returns the retained original declaration objects. Method links
select the object's retained namespace token and counted storage key directly;
no displayed namespace spelling or combined variable parser supplies that
receiver. Object-variable enumeration reads the physical object namespace
rather than the list of declared names.

Property membership uses native original-object equality, independently of
variable-declaration ObjHash membership. Neither property comparison nor
CString diagnostics may replace a declaration's counted key.

Constant declaration failures use `MakeConstant` at `ValueWrite`, with
`AlreadyExists`, `ArrayElement`, or `IsArray` selected by the actual receiver.
C9 reports the original combined CString operand and issues `TCL LOOKUP CONST`.
Early `NameLookup` failures instead retain independently split CString parts
and issue `TCL LOOKUP VARNAME` or `TCL LOOKUP ELEMENT` with the selected key.
`native_constant_failure_verb` selects `const` for binding failures and
`make constant` for late receiver failures. The failure owner does not infer a
cell from its name, create an element, or change the retained key. Older C
engines and unaudited sites refuse this purpose.

`PossibleBodyRegion::conditional_sources` retains unchanged original body
slices separately from entered `ExecutionPhase::script` values. Each optional
receipt has the phase's effective operand position, original source instance
and base, the wrapper's variable frame, and its retained lexical policy. Case
list elements use `ExecutedScriptSource::list_element`; escaped or substituted
bodies without an unchanged map remain unavailable. `None` means unavailable
advice, not an empty script.

`conditional_body_topology_advice` queries separately owned original layout
observations when runtime dispatch remains opaque. An actual compilation
observation uses `SourceInvocationBinding::original_compilation_lookup_advice`
and requires its immutable compiler snapshot. An accepted declaration can instead
supply `SourceCommandBindings::declaration_operand_layout_advice`: it retains the
own-frame table before evaluating that command's operands. Both paths validate
exact source instance, byte geometry, unchanged word shapes, a static head, the
common command-lookup kernel, and retained logical grammar. This query selects named
slots through that kernel without unresolved-command fallback: the `unknown`
handler's own layout cannot describe a missing written command. Alias traversal
keeps the same restriction. Actual dispatch continues to select the fallback
handler independently. The selected topology retains that grammar for
conditional source decoding even when runtime dispatch loses its context. An accepted
declaration's local-frame classification can supply the separate lexical body-layout
condition when runtime alias policy is unknown; it does not install aliases or supply
an executed frame. Prior replacements,
non-native targets, missing snapshots and conflicting candidate layouts decline.
The declaration path does not create a compiler observation. Its private Arc
observations also travel on `SourceInvocationBinding`, so diagnostic consumers
can query the same original layout after lowering releases the source inventory.
Conflicting observations withdraw the receipt; it does not participate in runtime
dispatch equivalence. Argument effects and unknown runtime alternatives remain. Both return lexical candidates, never an
executable binding, and publish lexical sources with empty execution scripts and
`OpaqueResidual`.

Source-backed callee argument advice uses the separate private
`declaration_call_layout_advice` receipt. It retains an original procedure
allocation and native formal-binding layout without granting handler metadata,
dispatch, successful completion or effects. Registry body-layout advice still
declines replaced non-native targets.

Logical procedure argv roles use `original_source_procedure_argument_roles` with
its original declaration receipt or independently retained source point. The
allocation-stripped declaration targets must exactly match the retained original
procedure correspondence targets, including every captured alias argument.
Internal source allocations identify that declaration only. An unentered source
keeps its raw unknown lookup, empty runtime targets and absent handler/frame
facts. Native argv keeps its actual entry, same compilation world and closed
runtime target checks. Changed words, input, rows, receipt, alias prefix or unknown
callback paths decline this lexical role projection.

Reached expression evaluation captures canonical variable read events before
the retained source inventory joins other invocations. Each synchronous capture
validates the expression invocation, original source and actual variable frame;
nested evaluations restore the enclosing capture. Abrupt or incompatible
captures supply no normal operand proof. Operand evaluation retains effects and
abrupt results, while the whole expression's normal contents and object
representation require the independent selected result producer. An operand's
last value cannot become the expression result merely because that producer is
unavailable.

`conditional_index_access_advice` selects only unanimous original native
single-index layouts for list reads, list writes or string reads. It preserves
the written operand sites and logical index-boundary policy. Normal bounds
findings require independent positioned source reads and represented SSA
versions. A conditional finding may instead query
`SourceInvocationBinding::declaration_read_occurrences`: each occurrence retains
its exact invocation, original source spelling and read site, direct operand
owner, accepted declaration frame and logical variable grammar. Formal inputs,
qualified names, scoped aliases, traces, conflicting owners and changed source
spelling decline. Unknown runtime alternatives remain open.

`DeclarationReadOccurrenceAdvice::diagnostic_version` references only an existing
unambiguous symbolic SSA value at the same original source boundary.
`declaration_variable_operand_advice` separately validates a statically decoded
scalar operand in that declaration's own frame. It retains literal leading sigils,
rejects formals, scope aliases, arrays, qualified names and changed words, and never
returns a physical address. Its diagnostic version references the same existing
symbolic boundary. `DeclarationIntervals` runs the shared interval fixpoint with
unanimous original native increment layouts; only conditional bounds candidates
consume that result. Conditional length preservation also requires the original
list-write layout and retained release's existing-element policy. Modern append
slots without sufficient independent evidence remain unavailable. It creates
no SSA definition, use or physical binding. Bounds diagnostics still require
independent container lengths and guard intervals, and describe both the original
handler and declaration-value conditions. Read-before-set diagnostics consume
original occurrences independently of absent def-use chains, while retaining
unknown-writer, local binding, alias and existence-guard exclusions. The
missing-read projection consumes `SourceInvocationBinding::declaration_flow_report`.
Its flat original-source graph uses retained command observations, logical lexer
configuration and Registry completion/body descriptors. It evaluates operand reads
before dispatch, preserves switch no-match successors and zero-trip loop paths,
and routes return, error, break and continue independently. `CaseListSpec` authors
`exhaustive_keyword_patterns` separately from event keywords; the shared case option
scanner preserves a dynamic subject without inventing its value. Physical-read candidates must match retained source spelling, evaluation owner,
declaration activation and local scope. `potential_declared_reads` separately retains
exact original local-reference occurrences in the accepted declaration frame,
including array roots after their original index operands. It does not require a
successful physical read and does not close unknown runtime writers.
`allows_declared_absence_warning` checks the exact name and occurrence against the
authored dispatch prefix. An unknown handler can initialise arbitrary locals; its
successors retain their reference inventory but cannot supply an absence warning.
A selected conditional-output descriptor can suppress an absence warning for its
exact destination when output may be written. It supplies no physical store or
successful matcher result. Definitions
intersect at joins; aliases and unknown writers accumulate.
Unsupported lazy expression, materialised body and lifecycle layouts retain an
unavailable alternative. The graph creates no SSA definitions or uses.

`DeclarationFlowReport::invocation_may_be_reached` accepts an exact source allocation
site. `Some(true)` describes a conditional path to original argument entry with
available predecessors. The command's subsequent unknown dispatch does not withdraw
its own entry; it withdraws successor queries. `Some(false)` means no such path in
the available graph, and `None` preserves unavailable predecessors, unrepresented
sites or a different source instance. It proves neither actual dispatch nor runtime
dead code. Potential-read messages state a possible read in the declared local frame.
Physical and SSA diagnostics remain independent; neither consumer obtains successful
access, physical absence, executable body entry, removal permission or opcode admission.

`declared_argument_entry` separately answers whether an accepted original invocation
has a path in the declared-frame diagnostic interpretation. It permits only candidate
warnings and no-edit suggestions; it does not replace `invocation_may_be_reached` for
successful entry, physical reads or executable rewrites.

Declared loop diagnostics distinguish the first test from repeating tests. Original
literal assignments and integer updates feed the shared dialect-aware scalar evaluator;
unknown writes invalidate those values. A known false first condition keeps zero
iterations. For a pure unknown while/for condition, after-loop warning suppression
states the condition that the loop iterates. For list-pair loops, the shared authored
input layout and literal list parser distinguish nonempty, empty and unknown inputs;
unknown list values keep their zero-iteration diagnostic path. A separately
conditioned accumulator interpretation uses the selected `AppendsListElements`
store descriptor and assumes enclosing unknown list loops iterate. It can suppress
an after-loop accumulator warning only when every relevant body exit defines that
name; ordinary setters do not obtain this assumption. Empty literal lists, conditional
stores and first-iteration reads still report. Definitions intersect across break,
continue and branch exits, and a first-iteration read cannot borrow a later body
definition. `declared_read_is_defined` refers to one original read
occurrence on those diagnostic paths. Neither query closes runtime pooled-literal,
object callback or zero-trip alternatives, changes SSA, or admits a loop opcode.

`authored_use_spans` and `has_named_use` retain original value-reference and named-read
operands on conditional graph paths. W220 can suppress a warning for an original use
that has no represented SSA use; this does not mark a physical version live.
`has_quoted_use` separately projects dollar spellings in original braced data
operands and Registry-selected case patterns. It uses exact original image spans
and the shared executable decomposition arena. Script bodies, expressions, formal
lists and variable declarations retain their separate owners. These inert mentions
suppress unused-store warnings only: they create no executed read, SSA use,
physical liveness or erasure permission. Unavailable quoted geometry preserves an
unknown use alternative, and terminal graph paths cannot donate later mentions.
`CaseBodyOperands::patterns` and `bodies` share the same selected clause parser and
argument/list-element address spaces; pattern origins do not identify a matched arm.
`has_declared_alias` projects the selected variable-cell transition's exact local name,
including qualified namespace tails, rather than the written external operand.
Alias advice suppresses local-absence warnings, while an independently known unset
still reports. Every consumer verifies the report's original body or invocation
allocation. Terminal paths do not donate later uses, and dynamic or missing alias
destinations preserve an unknown path.

`script_image_binds_name` uses the original image's channel for the enclosing
script. Nested unchanged literal body words are evaluated by the shared word
owner and then scanned as native-value bytes. Incomplete, opaque or unavailable
images supply no suppression. `Ownership::DecodedBindings` and `ScopeAliases` use
original structured variable operands: literal leading `$` bytes remain part of the
decoded name, and dynamic names provide no ownership. They do not interpret a decoded
name as reference syntax. `authored_procedure_read_advice` separately checks
original declaration, formal list and body spans to delimit individual diagnostic
reads. Declaration ownership does not override an unknown writer, unresolved
operand grammar or caller frame. It does not say that any runtime cell was absent
or that a declaration succeeded.

`overwritten_local_store_advice` can keep a version-specific W220 finding when
later dynamic reads refer to its replacement. Both original setters must retain
unobserved selected stock dispatch, closed object effects, literal original
values and the same local physical cell. The second setter must observe the
first setter's exact contents origin and a current stock old object; original
intervening source must contain only separators/comments. Execution traces,
substituted values, intervening reads, unknown epochs and replaced handlers
withdraw the advice. CFG consumers additionally check statement adjacency.
See `exact_overwrite_interval_rejects_reads_callbacks_and_substituted_values`
and `declaration_read_advice_requires_original_unchanged_body_and_formals` for
positive and withdrawal usage.

`conditional_overwritten_local_store_advice` retains the same source, selected
setter, cell, origin, stock old-object and adjacency requirements while exposing
an explicit condition: object conversion and release hooks must not observe the
intermediate value. It cannot restore closed runtime effects. W220 consumers may
say that the assignment **may be overwritten before it is read**, including when
conservative callback reads keep its SSA version live. Alias, global, synthetic
store and source-ownership exclusions still apply. This advice supplies no store
removal or executable optimization licence.

`conditional_unread_local_store_advice` requires an original unobserved native
setter with a literal value and an exact local scalar place. Its declaration
graph must represent that store on an available conditional path and contain
no read of the name, scope alias, unknown command/script path or unavailable
read layout. Named variable reads and read-before-write operands count even
without a `$` substitution. `subst` retains its unmodelled read obligation.
The dataflow consumer additionally keeps hidden-read, scope, synthetic-store,
executable-block and assignment filters, and requires an already-dead SSA
candidate. A warning says the value **may be unused in the declared local
frame**; it changes neither the dynamic-read barrier nor removal permission.
`conditional_unread_store_requires_the_complete_original_layout` pairs a
literal dictionary-loop store with direct/named reads, substitution, unknown
commands and aliases that withdraw the advice.

Diagnostics may inspect a receipt's `source()`, `frame()` and `config()` to
recognize bindings inside that conditional source. For example, the `j` in a
literal switch arm `for {set j 0} {$j < 3} {incr j} {puts $j}` belongs to that
body's own frame. This permits suppression of a lexical read incorrectly
attributed to the wrapper. It grants no reached body, completed write, current
value, successor definition, native opcode or command identity inside the body.
The original invocation and `OpaqueResidual` remain intact; executable and
optimizer consumers continue to use entered scripts and their independent
receipts. `conditional_case_sources_do_not_claim_entered_bodies` checks original
list-element geometry and the separation from entered statements;
`conditional_case_sources_decline_replaced_handlers` checks handler replacement.


### Interpreter results and completion ownership

An authentic C interpreter owns one original result object. Result publication
moves that reference into the interpreter; evaluation-frame mirrors and returned
completions retain lifetime leases to the same header. `Value::clone` acquires an
actual native reference. `NativeObjectLifetimeLease::into_value` transfers a
lease into a completion, and `Value::into_native_reference` retains it when an
operand stack, variable, List, Dictionary, or saved `finally` outcome takes
ownership. These operations grant no parser, cache, or command authority.

Result reset reuses an unshared C header and replaces its primary representation
with the canonical empty String. A shared result gets a new empty header. `POP`
discards its operand; `DONE` publishes the returned operand. Native C8.5 and
later trace chains preserve the result, private return code and level, carried
return options, private error objects and logging flag. Releases with TIP 348
also preserve the stack and its reset state. C8.4 has no whole-chain save: each
script callback transfers the result into saved storage and installs a fresh
empty result, then resets and restores the result after evaluation. Its command
rename/delete script callbacks also preserve the private return code; variable
callbacks do not. Direct native observers perform no script evaluation or
result reset. Jim uses its original interpreter context result role. Unknown
physical result issuers return a typed host refusal.

`ErrorStack` snapshots share their entry backing. Appending to shared backing
copies its children at the mutation boundary; beginning a new error episode
replaces the entry storage. Runtime carried return-option snapshots likewise
share their original pair storage. These carriers preserve semantic state and
original children. The VM stack getter constructs a List header from those
entries, and Runtime carried option pairs do not retain an original native
Dictionary header; neither carrier issues such header identity.

### Original List backing and native ownership

`NativeListItems` owns the VM's original member store and selected window.
Runtime `NativeListStorage` owns the equivalent pointer inventory. Each genuine
List header owns one store reference; every occupied slot owns one reference to
its original child. Header duplication shares that inventory without retaining
children again. A span selects a window over the same store; shared stores keep
members outside that window alive. An unshared trim releases unreachable member
references at the reached primitive.

The visible member count, store's used slots, first occupied slot, selected
window and allocated extent are separate. C9 conversion records the original
`max_list_length_bytes` allocation request while parsing the original object;
construction and growing storage record their selected allocation. A retained
window cannot manufacture an extent from member count or rendered text.
`native_list_storage::range_action` selects opcode outcomes, and
`range_storage_action` selects an already validated private range. Full-range
reuse precedes unshared prefix trimming, then the span threshold, then fresh
storage or in-place movement. The threshold uses native integer division of
allocated and used extents. `replace_layout` owns the C9 append, prepend,
allocation and slot-shift geometry of a physically selected header.

Memory snapshots hold allocation leases to the original children. They acquire
neither native header references nor native member references and do not affect
COW sharing. A generation identifies the original member vector; checked views
reject a changed generation and retired child caches. Dropping the last genuine
store header releases its member owners immediately, even if an inspection
snapshot still keeps their memory allocated. Inspectors, renderers, original
callback members and direct List invocation use lifetime views. A generated
argument vector uses `invocation_view`; that container grants no ordinary List
store authority. `Value::invocation_list_view` retains original argument memory;
`capture_invocation_list` acquires real member references when the actual native
consumer captures those arguments.

`NativeListBacking` crosses the engine boundary with an engine-private storage
receipt, actual C release, interpreter scope, ordinary storage shape, selected
window and the same live canonical flag. These are separate from resident
string allocation and transport representation authority. The recovering engine
validates its private receipt, current header attachment and live vector before
applying a retained backing. Public members and canonical booleans describe
fresh construction and cannot recreate original storage authority. Foreign
scopes, forged receipts, retired or changed attachments, and diagnostic argv
views return a typed host refusal.

`Value::native_list_range` and Runtime `list::native_list_range` consume original
`NativeCompiledListRange` coordinates. Marked `LIST_RANGE_IMM` validates its
original getter before omitting allocation for a following plain `POP`; guards
and source boundaries prevent that omission. C9 canonical-empty length retains
the original primary. Native append and complete replacement mutate the actual
store, splitting genuine shared storage before member writes; temporary memory
views do not force a split. Nested LSET requires its independent per-node
transaction and is not licensed by a flat replacement. Runtime abstract-series
slicing requires a concrete slice provider; ordinary member conversion cannot
stand in for that hook. Storage recipes grant no CPP selection, source-entry
identity, argument order, variable receiver or compiled admission.

The C9 shim duplicates an actual original header through
`OriginalObject::duplicate_native_header`. Its duplicate retains the member
store and flag until reached COW. Refresh preserves that store only while the
original attachment remains current. Native updater flag changes use
`NativeStringProtocol`; lifetime pins never make native headers or children
appear shared. The shim's C9 ABI accepts only its actual C9 issuer. Use
`list_backing` and `list_backing_matches` for original storage, and explicit
fresh `OriginalObjectResult::List` for newly constructed members.

The `tcl-cmd-core` filters
`range_storage_geometry_matches_288_actual_c9_owner_windows` and
`native_c9_span_uses_allocated_and_used_extents_in_native_order` cover the
captured allocated/used/span geometry. The `tcl-vm` filters
`original_list_ranges_preserve_all_five_native_shared_header_windows`,
`native_range_rejects_jim_and_foreign_cached_protocols`,
`range_headers_retain_one_store_and_original_member_owners`,
`replacing_a_shared_span_copies_only_selected_original_members`,
`lifetime_views_do_not_own_native_headers_or_member_references`, and
`repeated_member_slots_each_own_one_native_reference` cover actual header,
member, COW and memory-view roles. Runtime's `tcl-runtime` filter
`runtime_list_ranges_preserve_all_five_native_shared_header_windows` covers its
original shared headers. These controls complement private receipt, foreign
scope, live-flag and retired-attachment tests. `xtask-owner-resolution` verifies
that both adapters use the shared range/replacement owners and that marked
range execution preserves native coordinates and checked getters.

### Original namespace-name objects

`NativeNamespaceNameRecipe` owns C release-specific cache validation, string
updater availability and result producer classes. Registry authenticates the
actual engine separately from an explicitly authored F5 result recipe. Logical
simulation cannot install an actual host namespace-name primary.

`NativeNamespaceNameToken` records one actual interpreter-owned namespace
incarnation. Engines validate its closed state identity against their own
ledger. Live, dying and dead states are independent of public name-table
membership; physical parent detachment is a separate operation. A global
namespace reset restores the same dying root incarnation to live, while dead
incarnations cannot revive. Namespace caches retain these descriptors without
owning interpreter activation or command implementations.

Namespace getters inspect the original primary before applying written-name
lookup. A valid descriptor returns its actual token without reparsing the
reported full name. Invalid later-C descriptors retire before a reached string
access; a missing required resident string produces a typed refusal. C8.4
retains its independently selected updater and unresolved descriptor. Actual
parent, child, path and code-context producers receive physical tokens through
`NamespaceObjectBackend`; child filtering returns tokens through
`children_tokens_checked`. The VM and Runtime implement the same token-directed
producer contract over their genuine object headers and namespace ledgers.
Runtime namespace eval, inscope, parent, children, path and upvar retain original
arguments through this getter; namespace-code results retain their original
context member. An ordinary reporting factory remains available for consumers
whose native producer constructs text.

`NamespaceDeleteBackend` uses written-name lookup independently of namespace
object cache conversion. `delete_original` validates every original argument
before retirement and repeats lookup during retirement, when an earlier callback
may have deleted another target. Namespace diagnostics use the shared selected
operation, native CString extent, error-code update and String producer recipe.
Snapshot data cannot substitute for an original object or lifecycle token.

Callback namespace transport retains an engine-private receipt, original
interpreter scope, descriptor release and resident storage independently.
Only the issuing engine can recover that receipt. The C9.0 shim refuses foreign
release descriptors and keeps genuine duplicate headers on the same resolved
cache descriptor. Snapshot classes and full-name bytes provide no live lookup
or reconstruction permission. Tests distinguish a cached unaddressable
namespace from a fresh identical string, actual duplication from reconstruction,
reference-context changes, deferred deletion, final death and global reset.

Original child compiler context and runtime entry are separate inventories.
A retained `SourceCompiledChild` supplies its exact source image/channel, selected
compilation frame and typed namespace key only while its original parent's
compiler admission, table and namespace prerequisites agree. A switch arm need
not execute for its original compiler visits to remain available. Missing body
namespace or source admission requires a native source provider; it cannot
license an empty executable body or a fabricated nesting-depth error.

Native numeric thread state is a separate host capability. `tcl-platform::Host::numeric_environment`
supplies actual before/after errno facts and unsigned/double C conversion results;
absence establishes no baseline. `tcl-host-c-abi` owns the thread-local errno
pointer and C calls on explicitly supported Linux, Apple and FreeBSD targets.
`NativeHost` delegates that safe capability. The genuine Jim object context
retains the selected Host, while `tcl-cmd-core::native_numeric` executes the
stages selected by the Syntax getter's prefix/end-pointer recipe. Fresh Wide
and Double conversion reaches this environment; a cached primitive fast path
does not. Jim's unsigned stage preserves prior errno, and its later strtod
stage explicitly resets errno before parsing. Interpreter-local state, source
profiles and equal numeric spellings cannot supply those facts. C8.4 fresh
conversion uses the separately selected reset and C call after cache lookup.
`NumericErrorState::domain_error` distinguishes actual EDOM from ERANGE;
`NativeFloatError::classify` selects EDOM/NaN before range/infinity and preserves
unclassified raw errno. Finite values and known NaN classification do not reset
or query unrelated state; infinity refuses when the required host facts are
absent. C8.4 minimum
integer output requires its separate independently authenticated native
formatter provider; this numeric environment grants no formatter permission.

`CheckedNumericEnvironment` is the shared callback-neutral forwarding owner.
It checks before and after each supplied numeric operation and compares ABI
observations in the supplied cell. Runtime and VM original adapters retain their
actual operation receipts and full typed first causes independently of that
neutral availability result. VM scalar, Number and original Boolean conversion
children share one optional-checker Value kernel; pure Value entry points select
no checker. The [scalar getter contract](native-scalar-getters.md) describes
checked cache publication and precision policies, including software proof
limits for the VM's original headers.

`Host::native_integer_formatter` supplies that separate capability only after
explicit installation. `LoadedNativeIntegerFormatter` verifies a supplied
library SHA-256, actual Tcl 8.4 version and symbol image/file identity. It calls
the genuine Long/Wide constructors and string updater with native interpreter
result ownership. `NativeHost` supplies no formatter by default. Checked VM and
Runtime string access can propagate the explicit provider through original
compound children without adding child references, changing numeric caches or
replacing already resident bytes. The loader is Linux-only; unsupported targets
refuse. Loaded Tcl callbacks/images retain process lifetime, independently of
the provider's native interpreter retirement; this is not a Tcl_Finalize claim.
The positive test-support producer separately verifies the pinned source header
and PIC archive, records the whole-archive link operation and hashes its output
before admission. Paired tests compare 812 Long/Wide windows against both the
original archive and loaded image. Original-object increment controls use the
explicitly installed updater for minimum-integer output; an absent provider
retains its separate typed refusal.

### Native readonly naming facade drift

The `xtask-owner-resolution` gate inventories the typed Core source declaration, indexed-location, invocation, method, namespace, variable, rename, math, procedure, workspace-completion and inlay facades. It rejects local namespace delimiter splitting, local zero-terminated byte-name cuts and lossy UTF-8 resident-name reconstruction in production items. Rust comments, literal fixture text and `#[cfg(test)]` items are excluded; production after a test item remains checked. The inventory covers these source facades, while syntax issuers and explicitly selected lexical adapters retain their own purpose owners. Source/configuration/Registry and independent URI currency remain carrier obligations. A readable resident label cannot supply counted equality, original input, lookup or edit permission.

`NativeTclooMethodInfoProtocol` in `tcl-registry::native_tcloo_info`
selects C8.6/C9 method-info option tables after the runtime has resolved the
actual target. `parse_original` delegates each sequential option and scope
lookup to the retained original object's `ValueOps` Index owner. The last
scope wins and an explicit scope disables recursive enumeration even when
`-all` follows it. `NativeTclooMethodInfoSelection::matches_local` folds only
independently supplied native visibility fields; it supplies no method
inventory, object/name header, target, body, dispatch, compiler preparation or
normal-completion grant. VM and WASM Runtime consume the same owner.

The method `SubSubCommand` rows share `METHOD_INFO_OPTIONS`, including the
C9-only `-scope` surface and its value catalogue. These are existing
`OptionSpec`/`OptionValue` data fields: the SpecTcl option-block loader and
Studio draft/Rust/SpecTcl renderers already carry them. The runtime selection
recipe is native protocol code, rather than a new authorable DSL field.
The native source and finite original-vector controls are linked by
`naming.tcloo.method-info-option-source`,
`naming.tcloo.method-info-option-selection`, and
`naming.tcloo.method-info-scope-ordering`. Positive membership comparisons do
not replace the independently required original method-name result owners.

Original lambda diagnostics are owned by `tcl-registry::native_lambda::NativeLambdaDiagnosticProtocol`, selected from the independently identified invocation point. C8.5 conversion errors append the full counted original object; C8.6 and later and Jim format its CString view. C conversion and formal-parse error frames have separate extents and error-code rules. Both VM and Runtime retain operational getter/capability failures independently of guest list-syntax failure and use the original object getter at the reached formatter. This recipe grants no list-conversion success, lambda cache, body preparation, namespace, activation or result storage class.

Native variable enumeration uses `NativeNameGlobPurpose::{InfoVariablesScan,InfoVariablesSearch}` through `tcl-cmd-core::info`. The actual frame issuer distinguishes compiled declared-slot names from dynamic hash keys; flattening their display names cannot choose a matcher. Namespace/dynamic searches preserve original counted keys for trivial patterns on C8.5 and later and use selected native CString units for glob scans. Jim retains its independent full input scan. Pattern/namespace geometry, cell liveness, declaration flags and table membership remain separate from any read/value or normal-completion grant.

Native formal-format diagnostics retain `ByteFormalParameterError::error_code_for_definition` separately from message rendering: C8.6+ installs `TCL OPERATION PROC FORMALARGUMENTFORMAT`; the selected older-C/Jim paths keep their default code. Runtime namespace variable lookup uses the shared root qualification once, so a separator after raw zero cannot reclassify a full counted simple key. This does not shorten the cell key or change array/cache/alias ownership.

The VM's `OriginalProcedureCallReq` transports the actual prepared lambda declaration and unchanged caller argv through the shared activation stack. Its early argument/body selection and `ensure_proc_ready_in` admission remain the native procedure owners; no generated source, List header, extra lookup or command trace is created for the handoff. Coroutine dispatch retains the actual requested command instead of recognising an apply label. Temporary registration cleanup follows the actual active command lifecycle handle through success, error, relocation, operational abort and suspended activation retirement; deletion detaches that handle, so a same-name replacement is not removed. This scheduling transport is independent of native Tcl's commandless lambda cache/header model; it does not certify that physical cache, object methods, parameter/value effects or normal completion.

`NativeNameProtocol::coroutine_publication_slot` owns the C8.6/C9 coroutine declaration name purpose separately from procedure and C API publication. It selects the original getter's CString bytes in the actual current holder, including an empty simple command name, and supplies no namespace creation. VM and Runtime `native_coroutine_publication_slot` traverse exact existing child tokens from the actual current holder or global token; missing qualifiers retain the native formatted diagnostic and `TCL LOOKUP NAMESPACE`. Selected slots feed each port's actual replacement publisher without rendering and reparsing a name. C8.4/C8.5/Jim refuse this purpose. Runtime builtin activation separately retains the actual selected registration generation, including dynamic coroutine registrations. Resume uses that token's current placement; it never rebuilds the worker key from the original argv spelling. Direct compiled helpers retain no registration token, and stock identity does not manufacture one. The same activation guard and coroutine context swap own this metadata lifetime. Publication and original coroutine invocation transport do not prove command/namespace cache, refcount, tracing, preparation, complete suspension lifecycle or probe/injection ordering equivalence.

Direct counted source and original file loading have separate newline channels in the finite [CRLF public record](../analysis/name-resolution-proofs/source-original-crlf-evaluation-channels.md). All six original direct controls preserve CRLF; C file loading translates it to LF and Jim retains it. The fixed retained source path projection keeps the original literal CRLF and escaped dollar; this does not issue runtime current-value, native object or edit-equivalence authority.

The Runtime `interp/native_operation_currency.rs` owner snapshots the entered interpreter allocation, actual host, full `RuntimeContext`, selected invocation dialect and independent non-wrapping execution-context epoch. Operation and scalar receipts share that epoch. Host, provider and execution-context ingress changes retire it before mutation; legal Guest command-world effects retain their separate compiler-policy invalidations without retiring an entered operation. The owner checks currency before and after reached Host calls without holding an engine borrow. A current receipt supplies no command, getter, expression or object authority; those remain with the actual purpose and producer owners. A change followed by restoration still invalidates the receipt, and epoch exhaustion remains terminal.

`NativeOperationScope` retains that original receipt on the actual interpreter
for synchronous nested work. `NativeOperationCurrency::issue` inherits an
entered receipt and validates it; it cannot refresh a changed or restored
world. `Interp::check_entered_native_operation` checks the original operation
around source, command, variable observer and String getter stages without
holding a stack borrow across callbacks. Ordinary callers without an entered
scope retain their explicit purposes and the first Host channel remains
terminal. Scope and receipt currency grant no native semantic capability.

`state_traits::capture_completion_checked` owns checked Guest completion
settlement for an explicit interpreter and original currency. It delegates
result/options production to the actual completion owner, preserves reached
getter effects and releases captured owned handles on a later Host refusal.
The full expression and fixed-math ABI exports share this settlement. Mandatory
cleanup and scope unwinding remain independent of publication permission.


The VM `interp/native_operation_currency.rs` owner retains the original interpreter identity, actual Host, full `RuntimeContext`, installed profile and engine, and one checked execution-context generation on that interpreter. `NativeOperationScope::enter` inherits an existing entered receipt; `currency()` and `check(&mut Vm)` preserve its original generation and first typed Host cause. Actual Host, compiler, pack, profile, logical-provider and observed name/frame input changes advance the same generation before mutation; checked-add exhaustion permanently refuses entry. Parking a parent or child carries that interpreter's owner without changing another interpreter's generation. Ordinary Guest table effects and compiler cache invalidation remain independent.

The VM Boolean adapter retains this scope while its checked physical helpers delegate to the same CmdCore result and operand workers as Runtime. Each adapter still supplies its actual selected dialect, getter, source, normalisation and liveness obligations. Check the original currency after reached callbacks and before later cache or result publication, and complete mandatory cleanup on refusal. A scope supplies no command, name, cell, object or Native execution purpose. Speculative Interpreter guards and wrapping compilation generations cannot replace it.

`InvocationDialect::native_numeric_instruction_result_protocol` selects an actually reached C `TRY_CVT_TO_NUMERIC` instruction. `NativeExpressionResultProducer::NumericInstruction` always performs numeric conversion without a public API copy. It remains distinct from `Boolean(InlineExpression)` and `Boolean(PublicExpressionApi)`, and supplies no additional Boolean ABI tag.

The original VM String stage uses the Value owner with an optional caller
checker. `OriginalNumericOps::string` selects the captured Host formatter and
actual string protocol. It does not create a formatter from a profile label.
`current_for_value` joins that same operation to the exact original header;
`with_environment_for_value` shares the captured Host and ABI observation cell
at operand-dependent numeric workers. Actual header withdrawal is retained as
a full typed first cause before subsequent workers or guest presentation.
Procedure-header and concat consumers use this facade; frame and name consumers
must retain their independent operand/frame/name admission before calling it.
The pure Value String adapters remain independent of VM execution currency.


The VM arithmetic algorithms accept `NumericContext` with an optional borrowed
`OriginalArithmeticStages` provider. `expr::with_original_numeric_context` joins
independent selected arithmetic/scalar/String axes and borrows the canonical
`OriginalNumericOps` for actual expression, opcode and unary consumers. The
provider delegates to the existing Value parser, scalar, String, cache and
C84 expression-store owners; it has no duplicate conversion worker. Pure
context adapters select no provider. Reached success and failure both check the
same original operation/header before later stages. `TclError` retains the full
typed execution cause, independently of Guest presentation. Resident String
bytes require the original scope/header/protocol/Jim-context joins but do not
query the Host formatter. See the [scalar getter contract](native-scalar-getters.md)
for constructor, cache-retirement and consumer obligations.

The original scalar, Number, getter-failure and C84 expression adapters forward
that same captured String provider into their existing Value kernels. It is
selected at the actual String branch and checked on success and failure before
a later conversion, diagnostic or cache store. Pure wrappers keep their None
provider and existing header checks; test-only checked probe siblings retain
their explicit software checker. This forwarding adds no conversion replay,
formatter implementation or native object reference.

Original VM increment consumers use the canonical numeric facade through the
existing COW preparation, Number/legacy getter and cache-store kernels. Captured
cell views are allocation leases, not native reference holders. The shared Jim
Wide expression worker also serves the original regexp index callers. Its
entered source preparation checks the same actual source header after String
and before parser/cache work; the original Jim context and selected compiler
policy remain separate admissions. `prepare_original_expression_value` routes
actual VM callers to that owner, while explicit authored and independent
`prepare_expression_value` inputs keep their None route. Source-cache publication
delegates to the existing checked Value backing-retirement owner.


The original Direct script-entry parser ordering comes from
`NativeCompilationContext::direct_script_parse_entry` on the actual retained
entry, before source activation or prefix publication. C command-at-a-time and
Jim whole-script entry keep separate execution purposes. The readonly Jim
syntax roster and explicit parse-cut descent APIs are described in
[lexing-segmentation.md](../compiler/lexing-segmentation.md#selected-direct-script-parsing).
Their source/configuration facts supply no NativeValue, Script object,
interpreter or execution authority.

Runtime builtin registration uses the actual command publication owner. Public
`register_builtin` publishes an embedding host's handler and claims no stock
identity, even when the name matches a registry command. Engine installers use
crate-private `register_stock_builtin` and retain its exact returned generation.
Zero means publication refused; it cannot identify a command or acquire an
attestation. `register_guarded_builtin` and `register_spec_builtin` explicitly
attach semantic identities to their own returned generation. A spelling lookup
after a callback cannot supply that receipt. Spec registration grants neither a
stock release floor nor a C compiler hook. Distribution helper admission is
recorded only for an actual stock registration selected by its installed family.
Stock root and nested ensemble installers consume `create_ensemble`'s actual
publication generation. The shared publisher checks original currency and the
same binding after its token/configuration cleanup; refusal supplies no
receipt and cannot classify a callback-created command at that spelling. Private stock helpers carry the
same original scope through registration preparation and cleanup, check their
actual fixed-slot publication before metadata, and stop later helper installation
on refusal. Scripted distribution roots retain the procedure publisher's weak
original Proc/declaration, namespace slot and generation through caller body
cleanup. Stock metadata consumes that same receipt; a later spelling lookup
cannot adopt a callback-created procedure. Existing generation and void
registration facades delegate to the same publication worker. Stock-family
replacement extracts the displaced record and releases its actual configured
object roles outside the stock map borrow. The original scope and pre-retirement
publication receipt are checked before later installers or attestation. Refresh
restores reached bookkeeping on refusal and completes its mandatory outside-borrow
retirement without granting a new semantic continuation.

Command visibility reads stock identity by generation, including renamed
primitive and ensemble tokens. Host handlers use their actual binding without a
stock release gate. Every intrinsic guard resolves the same visible command and
its exact attestation. A stock ensemble's semantic attestation additionally
requires its actual unchanged map, original worker generations at the map's
registered slots, and nested ensemble configurations. A displaced worker held
elsewhere supplies no map-slot proof. The backing catalogue reports physically
installed stock ensemble roots as handlers; that presentation is independent of
semantic attestation and includes their real private workers. A guest-created
ensemble supplies no stock registration merely by occupying the same name.

The Runtime namespace table owns a checked lookup-policy revision for path,
export, unknown-handler, imported-command removal and namespace lifecycle
mutations, including actual child birth and clearing nonempty exports. Selecting
an existing child or clearing empty exports is observation. Guard preparation and both guard-check doors synchronise it into
`CommandEnvironment`, `Namespace` and `UnknownHandling` together. Counter
exhaustion permanently poisons these domains. Ordinary command definition,
rename, import, hide and expose within existing namespaces keep their existing
token-generation rules and do not advance this revision. Add policy mutations
at their actual table owner; obtaining a mutable namespace borrow is not itself
a policy mutation. Native compiler and command
reference epochs retain their separate selected source semantics.


Public numeric command arguments use the same scalar owner through
`InvocationDialect::native_numeric_command_protocol`. Read count dispatch,
encoder Int/Wide purpose, debug Boolean support and seek-origin order are
selected from authentic native axes. The Registry descriptor supplies no
conversion, object or I/O success. `expr::with_original_numeric_command` keeps
one actual entered operation through Name/index/getter/error/backend/result
stages while retaining independent authored numeric policy. Its
`original_numeric_command_ops` adapter validates actual selected scalar and
String protocols before calling the canonical facade. Host errors remain typed;
only Guest failures can follow a selected command continuation. Open channels
retain their original checked path, and invalid seek targets reject before
cursor publication. The [scalar contract](native-scalar-getters.md) specifies
adapter usage and the distinct public versus software callback evidence.

Runtime active OO frames retain the reached method chain and original name owner
through invocation. Their shared retirement extracts the actual frame under a
short OO borrow and releases its owning roles after that borrow ends. The
original entered operation spans all method and eval temporaries; mandatory
filter, forward rewrite, Tcl frame, namespace and activation restoration
continues after refusal, before the final full first-cause verdict. These
software cleanup checks do not attest Native callback chronology or physical
Method/clientData reference counts.

The VM method-name cache reaches its String stage through the canonical
original String worker with the captured Host formatter and entered operation.
Its existing world/stamp and original-header checks bracket that stage and
primary retirement. The first typed String refusal is retained before later
checks or cleanup; ALL method-record upgrades follow successful settlement.
Allocation leases and idle weak records add no Native method ownership.

Checked VM List-copy, Dictionary conversion and prepared member updates require
the same node-aware String provider as ordinary original List conversion. Their
shared Value workers retain an explicit `None` route for independent pure APIs.
The array command supplies `OriginalNumericOps::{copy_list_with_currency,
prepare_dictionary_with_currency,set_dictionary_member_with_currency}` using its
original entered currency; selected String protocol, root and reached child
headers are checked before and after String, cleanup and cache publication.
Dictionary key conversion does not acquire a fresh formatter or a new operation
receipt. Full Host refusal precedes Guest dictionary parsing or command result
publication. Allocation-only root views preserve the original COW observation.

VM method invocation requires the originally retained Method record. An
ownerless descriptive context supplies no implementation lookup; the synthetic
object-eval reporting context remains separate and cannot enter a later chain
step. Same-name replacement preserves the actual record allocation and current
payload. The shared table and compiled-body replacement workers extract old
payloads under short Method borrows and release them after those borrows end.
Unnamed slot replacement creates its genuine new record while entered original
chains retain their earlier record. These software roles do not provide a
Native MethodType callback or physical reference-count equivalence.


Original VM Name consumers retain `OriginalNameConsumerScope` through the whole
selected worker, including cross-interpreter flow restoration and mandatory
retirement. Its borrowed `operation_scope` is the same entered owner used by
`OriginalNumericOps`; it issues no new physical, namespace, object or command
role. Full Name checks preserve the actual frame, namespace incarnation and
world before subsequent lookup, publication or Guest interpretation. Typed
`CmdError` and `TclError` outcomes are checked on both success and failure. A
legacy `ValueOps::native_string_bytes` call delegates the same node-aware
original String owner and leaves the full cause in the Vm; its `ValueError`
unwind must reach a real checked completion boundary. Explicit authored Name
helpers and pure String constructors keep their separate neutral purposes.

Procedure declarations call the mutable `capture_procedure_declaration_body`
owner before formal List and body String stages. It chooses actual Jim formal
List order independently from C body creation, and observes the original body
sharing count before the working definition reference. C shared bodies use the
same counted String copy; unshared C bodies and Jim bodies retain the same
original object. Scope and header withdrawal stop before later capture or
compiler publication. Preparation retains the original source, selected
namespace, real compiler entry and service; unavailable admission is terminal.
Jim static construction preserves the existing copied-member reference roles
and discards fresh unbound cells on failed installation. These software
callbacks do not measure an external extension object's reference/free ABI.

Alias loop queries use the selected physical algorithm independently from
source name policy. C refuses missing or deleted actual target owners before any word query. It reads each original target word in its genuine live target
interpreter and queries that target's global namespace; missing commands remain
legal late binding. The defining slot, generation and world remain original
through restoration. This query executes no command and issues no Native lookup
receipt. Jim creation has no C loop validation worker. Full failures pass through
the typed creation and rename boundaries after mandatory alias retirement. The
[alias contract](../analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md)
and [body capture contract](../analysis/name-resolution-proofs/procedure-counted-body-header-selection.md)
state the bounded physical observations; software embedding controls remain
separate from those provider answers.

VM property report and accessor stages retain the original OO target and class
facet, caller context, entered currency and reached header independently of
String and List conversion. Report derivation checks its actual roster and graph
observations until materialisation. The materialised configure reader owns its
original List header across getters; a configure write table owns its copied
List and frozen Strings. Ordinary getter and setter changes to the property or
method roster remain permitted. Later write pairs use the retained table's index
with the current report header, as required by the selected C9.1 worker.

The property stages use the canonical `OriginalNumericOps` String provider and
checked Value List, property-name and Dictionary kernels. Configure-all hashes
and adopts each getter result before reaching the next getter. Dictionary
construction shares its sequential key worker with the neutral API; supplied
checks and String providers add no eager key preflight, parser or formatter.
Cached accessor names require no property String until a reached Break or
Continue diagnostic. The actual my, accessor and setter-value call roles retire
before that presentation, outside Method and OO borrows. Private-variable
comparison checks the original declaring inventory independently of method
mutation epochs. Full typed first cause precedes later conversion, cache/result
publication and owned cleanup. These report and accessor checks do not provide a
property-member retirement recipe, whole-object lifecycle authority or Native
callback chronology; the literal configure controls use the independently
retained public property snapshot and table observations.


## Original VM dispatch and parked continuation ownership

`NativeOperationScope` enters the actual invocation before original name
getters, reset, command charging and enter callbacks. The same original owner
travels through `ExecLeaveCtx`, deferred `Call`/`Push*` handoffs, the actual
activation `Frame` and `TailcallReq`. Trace eligibility remains independent;
the scope supplies no name, command, frame, compiler, scalar or String recipe.
The checked original name getter returns full `CmdError` to existing completion
boundaries before Guest formatting or later callbacks.

Coroutine suspension changes only those carried execution owners from
`Entered(Scope)` to `Parked(Currency)`. Parked-only scope clones retire, while an
actual active parent invocation remains entered. Saved currency does not make
`current_entered` succeed for an unrelated call. Authentic resume, queued
injections, `coroprobe` and retained activation callbacks attach that same saved
currency after complete checks and before work. Cleanup has a separate
same-target attachment that permits mandatory owner retirement after currency
withdrawal; it grants no resumed execution.

Every successful or failed worker checks the original scope before Guest
presentation or continuation. Host refusal retains the first full typed cause
while releasing actual scripts, procedure frames, selected namespaces, step
scopes, Jim command leases, argument owners and temporary lambda publications.
The `RETURN_CODE_BRANCH` opcode reads its actual popped operand through the
canonical selected Return-code Int getter; `END_CATCH` remains separate. Pure
source and authored compatibility APIs keep their independent purposes.

The bounded software controls in
`exec::original_name_dispatch_tests` and
`interp::native_operation_currency::tests` exercise actual paused continuations,
an unrelated fresh invocation, active parent preservation, Host ABA, resume and
another retained callback route. These controls do not measure an external C
extension object or grant physical execution authority from saved metadata.


Stored variable-link roles belong to the actual published cell. `NativeScalarAliasEntry`, `NativeElementAliasEntry`, and `OriginalJimLinkTarget` distinguish that once-only role from retained Rust metadata. Releasing `Var::Link` explicitly withdraws its genuine counter/name owner into `RetiredVariableParts`; borrowed `Link` views cannot postpone the withdrawal and become unavailable for semantic reads afterward. Re-adopting the same issuer transfers its single stored role without decrementing it through the displaced metadata view. No allocation lease adds a Native reference.

For selected C aliases, `NativeVariableNameProtocol::alias_publication_order` selects the original old-target retirement before new assignment/reference acquisition. `PreparedAliasReplacement` captures the original local cell and old-link identity, then checks that cell plus the selected target, frame/table observations, and original operation around outside-borrow cleanup and before adoption. Same-target no-op precedes mutation. The original C8.4 target-reference recipe includes compiled cells; C8.5/C8.6/C9.0/C9.1 acquire target references only for hash-owned entries. Actual object upvar/namespace/classvariable adapters and the automatic TclOO compiled-slot resolver consume this prepared worker. Generic logical link publication separately retains adoption before old-role retirement through `PublishedVariableLink`, with ordinary errors and the first full execution refusal checked before later consumers.

These are software storage/continuation contracts with independently pinned original source recipes. The destructor/metadata controls do not establish Native callback chronology or transfer any earlier test outcome. Jim original-object link publication clears the original local's link-frame marker before its later local and target String stages, transferring the same stored target-name reference into the visible scalar cell. The selected Jim worker acquires its target-name temporary, then its distinct incoming cell reference; it retires the old visible scalar before assignment and publishes the link frame afterward. Missing local publication gives only the original or stripped fresh key its actual hash reference. A pure allocation ticket excludes the already extracted old reference from reentrant replacement or teardown, and semantic reads refuse that retiring pointer. Original weak cell, binding, table, frame and entered-operation observations bracket these stages and mandatory temporary cleanup. Ordinary cached Jim scalar assignment uses the same prepared cell and retirement ticket: its one genuine incoming cell reference precedes old visible-pointer retirement, original cell/table/frame checks precede assignment, and link recursion retains its existing target/frame worker. Existing static fallback assignment updates the same shared cell without a local-table entry. Checked Jim variable-cache installation consumes the original cell/frame receipt and the existing primary-incarnation worker; a callback-published primary survives, and refusal precedes later scalar acquisition. Neutral cache installation retains its explicit unchecked worker schedule. These software checks supply no external Native callback chronology. Checked original member/indexed materialisation retains its actual captured receiver boundaries; ordinary array, trace-shell and container preparation also has neutral ports whose callbacks are not covered by this link-publication contract.

The VM Jim indexed-variable continuation uses `OriginalNameConsumerScope::park`
to retain the exact existing Name state and original operation currency while
dropping its entered scope. The pending record retains the actual tuple root
and name as allocation-only leases. Before lookup or continuation, its opaque
parked context delegates `NativeOperationScope::resume_saved`; mandatory
same-target completion/key/context retirement delegates `cleanup_saved`,
including refusal paths. Saved metadata alone supplies no execution admission,
and a foreign target is refused before state publication.

All actual tuple preparation consumers use the checked original Jim tuple
worker at its existing String/parse/free/adopt stages. Dictionary lookup uses
`OriginalNumericOps::dictionary_with_currency` and the same checked Dictionary
projection kernel, then selects a weak original Dict backing/epoch cursor after
the actual key getter. Short member borrows issue allocation-only views before
canonical String checks; no mutable borrow crosses a getter and no new Native
hold or Dictionary COW is introduced. Cursor or String refusal retains the full
first cause before cleanup or Guest presentation. Existing independent Script
continuations keep their explicit unchanged route.


VM OO destruction captures the actual physical interpreter and its entered
currency before destructor, command or namespace effects. Entry refusal retains
the first full typed cause and still retires the original owned records. The
same physical receipt carries descendant cleanup; later optional destructor
calls require the original current operation. Mandatory cleanup can access only
the captured interpreter's existing OO storage, including while that state is
parked. It issues no live object, namespace lookup or resumed execution purpose.

The original object's captured namespace token reaches the existing namespace
teardown kernel with the originally selected pure traversal recipe. Command
cleanup requires the same original object token at its captured sidecar. Cached
names, property caches and members, method-cache observations and whole Object
and Class payloads leave their storage accesses before release. Every reached
release checks the original saved target and preserves the first cause. An
exhausted method or property metadata epoch refuses through the typed owner
while mandatory payload retirement continues. OO rename retains the actual
object and class facet before destination publication; cached-name retirement
must settle before later public name metadata and import/source reattachment.

Property storage retains its originating pure recipe per actual object/class
facet. Its selected cache-before-member retirement order is independently
inspected C9.1 source provenance, not measured Native callback chronology.
Whole VM Object/Class field release has ordinary Rust owning roles; these
controls do not establish Native MethodType, Class pointer or extension free
ABI equivalence. Property mutation and invalidation publishers outside this
lifecycle owner retain their separately defined conversion and storage ports.

VM static Index, Jim Enum and immediate-comparison lookup capture or inherit one
entered operation before the original header, String or cache stages. The shared
`NativeStaticLookup` worker uses `OriginalNumericOps` for reached node-aware
String access. Its cached hits keep their no-getter path. The original-option
`ValueOps` projection retains this same owner through failure String and
error-code construction; static table identity supplies no namespace or Name
purpose. Checked mirror and Jim Enum adoption delegate the existing
old-primary retirement kernel, check the actual root and donor before adoption,
and preserve the mirror's resident allocation and donor formatting state. The
compiled lower-case mirror consumes that same checked worker under its tick
entry. Explicit test compatibility adapters remain on the same neutral worker.

The selected VM Jim cat adapter checks an inherited entry before authored
routing. Its checked sibling of `concatenate_jim` reaches the same canonical
String provider and actual source/result header checker, retaining full first
cause before fresh-result retirement or later operands. Empty and unary roles,
original input order and existing result references stay with the cat worker.
A legacy `ValueError` only unwinds to a real completion boundary that observes
the retained full Host cause. These software controls grant no opaque extension
callback or physical reference/free chronology.


VM OO property mutation uses `native_properties::mutation` and the existing
`OriginalPropertyOperation` for the actual target, class-facet birth, current
frame and entered currency. The selected C9.1 setter keeps cache retirement,
all incoming header references, old member retirement, then sequential canonical
String/hash uniqueness in that order. Allocation-only input leases do not
supply the incoming reference stage. Append and Remove retain the actual
GetPropertyList caller List and original members; empty Append/Remove are
no-ops. Registration compares each original member before acquiring that
working List member reference, then delegates the same member installer.
These stages are inspected C9.1 source facts and independent software controls,
not external Native free-callback chronology.

Method/property metadata invalidation returns full `CmdError` through its
method, forward, constructor, destructor, superclass, mixin, export, property,
method-rename and method-delete consumers. The same pure metadata kernel checks
Foundation/object epoch exhaustion before extracting a retired cache; its
first typed cause is retained before payload cleanup. `MethodTable` and
`MethodSlot` transfer replaced payloads/slot roles out of the short OO access;
actual publishers retire them before later invalidation or result work.
Superclass and mixin selection additionally retain original target and Weak
selected class-facet identities through their sequential original getters.
The genuine bootstrap intrinsic installers and test setup keep their separate
fresh-record purpose. Whole physical cleanup uses `OriginalOoCleanup`; mutation
owners do not grant lifecycle or Native Class/MethodType authority. Frozen
configure reports and retained configure name tables preserve their existing
legal Guest roster mutations and original-table/current-header index roles.

Trace-shell removal consumes the original resolved `TraceHome` and observes the
actual table ledger, frame birth, root cell and selected member without another
name lookup or an owning variable reference. The physical withdrawal worker
transfers existing value, Link and hash-key roles into `RetiredVariableParts`.
Their retirement occurs after the cell and enclosing frame/namespace borrows
end; the same original currency and reached cell state are checked before
continuation. General container conversion is not part of this physical table
access. A stale trace registration can be removed without withdrawing a reborn
variable under its spelling.

Direct native-observer removal attaches its saved original target for mandatory
cleanup, including an entry that already carries a Host refusal. Removed
observer allocations retire outside trace and variable borrows before that
cleanup scope ends. Reached trace-guard invalidation remains mandatory pure
bookkeeping, and the event waiter checks the retained cause before result
publication. These software controls establish no external Native callback
chronology. C8.4 string keys and later C object keys retain their separate
physical source contracts. Trace creation, precision materialisation, detached
alias-generation cleanup and unset-callback dispatch have separate owners.


### Original encoding byte names and source stages

Entered encoding system and source adapters call the shared byte-name resolver. They retain the original String header and operation across getter, Host capability lookup, catalogue lookup, file open, encoding configuration, read, close, evaluation and cleanup. The resolver uses literal-zero C-string extent and exact bytes; an unavailable catalogue returns a typed refusal, while a supplied catalogue can establish an actual miss. Implemented codec spellings use the existing channel conversion workers. Empty system reset restores the independently captured Host default, which remains distinct from the mutable current encoding and is shared by an interpreter tree.

An explicit source encoding String is obtained before the path and file access. Its catalogue lookup follows successful Host file open and precedes the first read, so an open failure precedes a later encoding failure and a lookup failure prevents reading. Runtime source consumes the selected encoding operand. C8 source uses the current system default, C9 plain source selects UTF-8, and Jim retains its filename-only byte path. Identity source conversion preserves bytes. `Filesystem::open_source_bytes` supplies an owned `SourceFile`; the shared coordinator always closes that same handle under the original operation, including refusal paths. Typed stage refusals are retained before mandatory close; an unmapped Host I/O result cannot supply a fabricated errno diagnostic. The native Host retains the actual opened OS file, and the VFS retains its original immutable file backing. Whole-file `read_bytes` remains an independent neutral surface; an absent staged handle cannot substitute a premature read. Complete streaming, codec/profile/EOF/BOM behavior, OS close-error capability, and external callback or native-reference chronology require their own owners and evidence.


The selected TclOO declaration-map publisher delegates to
`tcl_cmd_core::native_oo_variable_mapping::install_original_variable_mapping`.
The embedding first supplies genuinely validated original descriptors and fixes
the actual Object/Class facet, activation, naming provider and operation
currency. The coordinator acquires every incoming stored reference before it
releases the old stored references. A matching retirement ticket leaves an
allocation-only view of the old map visible through that release, then removes
only that view before sequential original String/hash stages and first-unique
installation. The ticket permits mandatory physical cleanup of its own original
or parked owner; it grants no lookup, current operation or replacement authority.
Displaced roles and unused incoming roles retire outside OO storage access.
Complete typed failures are retained before cleanup and checked before later
hashing, adoption or publication. Ordinary recursive declaration and method
changes remain legal when the actual target and currency remain current.

Private mapping records retain both their original declared-name header and the
actual reached full-name header. The counted declaration key remains distinct
from the CString input at the private full-name constructor. Whole-record copy
retains both stored roles, and variable/property consumers use the stored full
name rather than reconstructing it from a destination creation identifier.
Allocation-only roster observations and selected method-link staging preserve
identity without acquiring either stored role. Actual whole-record copy remains
distinct from those read-only descriptors. Public mapping is selected for genuine audited C TclOO providers;
private mapping additionally requires its genuine C9 provider. Authored neutral
slot publication remains a separate worker. Inspected C source stages and
software role/formatter controls establish no external callback chronology,
Native Class/Method lifetime or whole-object cleanup equivalence.


Trace registration materialises its selected shell through
`EnteredVariablePlace::prepare_trace_registration`. The trace lane uses the
existing classifier and retained-link permission worker, and retains actual
frame, table, link, root and member receipts through search/key retirement.
Existing stored roles retire outside cell and enclosing frame/namespace access
before the same original currency and selected births permit registration.
The returned `TraceHome` is the original reached home; command/direct observer,
error bootstrap and addressed precision adapters consume it without resolving
the spelling again. Native observer entry keeps the original argument Scope
through its String getter and actual incoming observer cleanup. Precision
preserves its ignored Guest registration result and terminal operational cause.
C8.4 string-key and later object-key source schedules remain independent;
software callbacks establish no external Native chronology. Detached-generation
destruction and unset callback dispatch are separate current owners.


The OO procedure-method preparation continuation retains the original object,
declaring facet, entered chain record and actual caller frame before the shared
procedure owner requests body String or compiler services. These are weak
observations of independently owned state; they acquire no additional Method or
native payload reference. Both outcomes return through the same check before
compiled-cache publication or a new method frame. A reached complete Host cause
is retained before a refused prepared activation retires, and displaced cache
payloads retire before the next original-owner check.

The reached chain remains the implementation owner after deletion or replacement
of a same-named table entry. Ordinary record-flag, method and declaration-roster
mutation does not invalidate that chain. Original object or declaring-facet
withdrawal, replacement of the actual caller frame, or execution-context change
prevents further preparation continuation. Intentional entry into the new method
frame follows settlement of the original caller context. This continuation
check grants no Native Class pointer lifetime, native procedure callback
chronology, command publication or whole-frame cleanup equivalence.

The Compiler's original Inline Control source purpose is separate from reached
handler structure. `SourceInvocationBinding::original_inline_control_compilation`
requires the retained physical entry and compiler registration, unchanged whole
compiler words, full configuration and actual Inline Control outcome.
`original_inline_control_metadata_for_input` joins that original point to the
caller's exact retained input and command store. Nested producers supply their
explicit nested input before this join.
The shared original structure selector uses the same original lookup target and
registry schema; generic, rejected and unavailable recipes remain unavailable.
The retained compiler site may supply lexical body geometry, while compiled
child namespace selection keeps its existing independent inventory checks. These
projections create no handler, Normal completion, body activation, numeric
normalisation or Native execution authority.


Source filename facts distinguish backend bytes from diagnostic bytes and the
original source label. The actual current system encoding at the reached filesystem stage determines
filename conversion independently of explicit source decoding. It is read after
original option/path String stages, so legal same-owner setting changes remain
visible; execution/name policy and currency stay captured from entry. UTF-8 delegates to the
shared external-octet worker. ISO-8859-1 uses the same selected native-unit decoder:
C8 maps units through FF and substitutes question marks for unrepresentable
units, while C9 refuses unrepresentable units with EILSEQ. Identity preserves
raw bytes on its supported C8 path; C9 named identity needs actual catalogue
knowledge rather than a fabricated builtin. UTF-8 strict surrogate failures use
filesystem EILSEQ presentation rather than converter unavailability. Other codecs
require an independently supplied filesystem converter receiving shared selected
native units, not raw dialect bytes. A conversion callback
uses the same original operation and reached-header checks on both outcomes.

The selected Unix C8.4 filename worker applies C-string extent after conversion;
C8.5 and later refuse embedded external zero with EINVAL before file effects.
Jim applies raw C-string extent without C external encoding conversion. Error
presentation uses the original String's C-string extent, so a modified zero can
remain visible in a diagnostic even when it terminates the converted lookup.
These are separate source filename facts and do not confer filesystem callback,
path normalization, platform-wide codec or Native header/reference chronology.


Runtime selected C86+ declared-variable Set uses
`cmd_oo::variable_mapping` as an adapter to
`install_original_variable_mapping`. Genuine arguments, target/class facet,
selected policy and the original definition-frame birth span the actual List
and String stages. Every incoming reference precedes old-visible retirement;
stored original then private fullName roles retire outside OO access before
zero-length setup and sequential canonical first-unique key installation. C90+
private records retain their reached fullName header independently of their
counted declaration key. Genuine Object/Class copies retain both stored roles;
report and automatic metadata views acquire allocation-only observations.
Explicit private linking consumes the stored fullName header, while variable
reporting, automatic bindings and property projection preserve its stored
storage bytes and identity. Allocation retention never revives a retired role.
Matching retirement tickets grant only mandatory physical cleanup and preserve
callback replacements; ordinary same-target roster mutations remain permitted.
The neutral software worker has its separate adopt-before-retire schedule and
None fullName reconstruction. C84/Jim original TclOO mapping authority, Native
Class pointer lifetimes and external callback chronology are not supplied by
these adapters.

Logical source names retain complete valid UTF-8 bytes and reject literal NUL or invalid UTF-8 in both written names and independently retained namespace components. `authored_source_command_slot`, `authored_source_command_candidates` and `authored_source_namespace_path` share written colon geometry; they preserve constructed components without reparsing or Unicode normalization. These projections supply conditional source scope only. They supply no Native name recipe, current table entry, entered frame, successful handler or Normal result. `source_transition_advice::BodyWalk::script` uses the existing `NestedScript` lexer purpose for original bracket, expression and body regions while preserving each word's full parent image and configuration. Root source tapes retain `WholeInput` purpose; source applicability, target transitions, unknown effects and complete original vectors remain independent guards.

The VM OO cleanup owner captures the actual interpreter, original currency, pushed OO frame birth and description, call-frame activation, namespace activation birth and dispatch depth. Namespace names, tokens and birth metadata move together through the single stack owner, selected-frame hiding and coroutine flow transport. Weak birth observations and readonly saved currency own no Method, Namespace or Native reference role and grant no lookup, compiler, provider or entered-method purpose. Synthetic eval contexts retain a separate descriptive cleanup role.

Actual OO procedure activations carry only saved cleanup metadata in their real bytecode Frame. Optional epilogue presentation requires the same original call frame, namespace activation and pushed method description; mandatory retirement extracts only matching original owners even after a retained first cause. Procedure execution ownership leaves before its call frame, and the actual OO Method/name context leaves afterward. Frame locals and displaced OO contexts retire outside storage borrows. A callback-created activation at the same position is preserved and causes terminal refusal. The shared driver restores its entry interpreter's Jim evaluation metadata and captured activation depth, preserving a child left current, then transports any full first cause after owned cleanup.

These checks describe software continuation and physical cleanup roles. They establish no Native callback chronology, extra Native reference, or TclOO availability for another engine. Variable retirement inside `destroy_owned_variable`, definition-frame cleanup, command-table publishers and whole-object destruction retain their independent owners and checks; this frame transport does not establish those consumers' closure.

Original compiled switch captures the existing `BodyObjectOperation` before its first subject word callback. Exact String reads use that captured body's canonical byte stage, and unshared nocase retirement uses the existing checked primary worker before String mutation. Its Wide and Boolean primitives use the same scalar getter/probe/failure kernels with an optional original frame/header stage; neutral entry points supply `None`. A full retained execution refusal is checked before Guest classification, selected-body continuation, and after the actual subject POP. The POP need not preserve the released subject's header. These thin consumers add no Native reference, replacement recipe or independent parsing policy.

The VM definition-frame owner retains the actual pushed call/namespace activations, selected definition row and original usage-slot birth. Usage words, their pure birth and originating issued activation move together through the existing OO flow swap; observations add no Native/header reference. Both temporary definition-worker target rewrites restore only the same original physical row. An ordinary continuation requires its original frame, namespace, row and usage; mandatory exit removes only the selected row and activations, preserving a genuinely new activation or usage slot. A saved previous usage whose issuing row was withdrawn is released rather than republished. Existing actual usage-word ownership leaves storage access before release and the full first cause remains retained through cleanup.

Both failed object/class constructor callers capture the actual allocated record birth and physical cleanup owner before reached body callbacks. Failure teardown delegates the existing `teardown_original` worker in that physical interpreter; a child left current and its records remain independent. Definition cleanup supplies no object lookup, callable Method, Native object/Class hold or Native callback schedule. Variable arena destruction and command-slot publication retain their independent owners and stage checks; this definition owner establishes neither consumer's closure.

The retained scalar carrier distinguishes `Tcl84Long`, `Tcl84Wide` and modern `Integer`; wire tag 7 carries the Wide descriptor category. Syntax preserves its existing `Number(Int)` getter equations and transports the Wide descriptor origin separately. Runtime `TCL_WIDE84_TYPE` and VM `IntRep::Tcl84Wide` preserve the public `wideInt` descriptor through canonical duplication, scalar snapshots, numeric probes and String formatting. Existing Long84 and neutral integer constructors retain their own descriptor contracts.

Actual VM scalar import retains `NativeScalarIngress`, issued by the original entered interpreter, through recursive List/Dict/resident import, Host reply recovery and owned cleanup. Explicit Wide import checks that same operation and its queried `CheckedNumericEnvironment` ABI; a copied dialect or descriptor data alone cannot construct this selected primary. Runtime scalar import uses its existing actual scalar object issuer. Foreign origins and unqualified Core Integer at explicit C84 ingress refuse before adoption, retaining the original full execution cause. The existing outer Host boundary settles that cause only after transport cleanup; a surrounding entered operation continues to retain it. These software contracts grant no Native callback chronology, parser authority or reference role. Selected Index result constructors and ordinary neutral integer construction retain their independent purposes.


Selected C84 Index integer results borrow the command's already entered original operation through `NativeIndexOps::index_integer_result` and the shared `index::integer_result` owner. All six C-style `lsearch` result constructors pass that same receipt. Actual C84 adapters select the `Tcl84Long` primary with public `int` descriptor, no resident String, an Int32 input bound and the original operation's queried scalar ABI. This result purpose follows the pinned `Tcl_NewIntObj`/`Tcl_SetIntObj` descriptor equations independently of a retained `wideInt` operand. The native final single-result path uses SetInt; direct software fresh-result and List receiving roles describe their own API contracts.

Runtime queries its existing scalar issuer under the original argument receipt before fresh allocation; VM borrows the original Index scope through its existing numeric facade. Both retain the earliest full typed refusal before unused results or a reached result prefix retire and check the same original operation after mandatory cleanup. A fresh Runtime result has its actual unowned zero-reference state; a reached List or result receiver supplies its separate owning reference. Modern integer results, neutral constructors and Jim's independent result workers retain their existing categories and reference schedules. An unavailable C84 sort `-indices` recipe remains unavailable. These software result controls establish no Native callback chronology, EvalObjv reference counts, parser policy or whole Index/List closure.

Compiled Next retains the original `BodyObjectOperation` before operand callbacks through the existing List construction/concatenation/member workers and reached stack cleanup. C9.1 reads its actual head String before selecting the OO context; earlier compiled C routes select after operands without that getter. A late weak context/chain receipt supplies original stages to the same class-name String/cache worker. It adds no Method/native hold, and the existing NextTo worker reads mutable context payload after its class lookup. Typed refusal is retained before cleanup and Guest projection; neutral Name/List entry points keep their existing independent purpose.

Ordinary C NextTo keeps its own entry order after argv evaluation: C8.6/C9.0 select context before the absent-context head diagnostic, and C9.1 reads the head before selection. The same weak selected-context receipt guards the existing class String/cache stages, without reselecting an earlier missing context after its diagnostic getter. Authored entry uses the neutral worker.


### Original variable cleanup and transferred storage roles

VM variable cleanup uses `vars::VariableRetirement` as the single storage
coordinator for the existing binding, static, trace-pin, operation, link, key,
scalar and array-table releases. The neutral arena wrapper preserves its
ordinary release phases. `OriginalVariableCleanup` separately captures the real
interpreter, arena allocation, name world, call-frame birth and namespace
activation together with the entered operation. Its weak observations acquire
no native Var, key, value or name reference.

The selected driver extracts each actual owning release before a callback and
performs that release outside storage access. Subsequent pure retirement uses
only the original physical arena, including parked same-owner cleanup after the
first typed refusal. Ordinary getters and later publication stop on refusal;
mandatory trace, pin, binding and table retirement remains tied to its actual
owner. A callback's current child and genuine replacement arena or table are
preserved. Operation and taken-trace epilogues release the captured original
cell and active-trace position rather than the callback's current arena.

The selected C unset phases are independent of the neutral storage wrapper:
old search handles retire before undefined publication, the detached scalar
role retires after its unset trace, array elements release their stored scalar
role while its header remains visible, and the actual old C91 default roles
retire after the old members and before the table's key drain. The visible
scalar ticket transfers the existing stored role once; its temporary header
view is allocation-only. Genuine Jim flat-table teardown retains its own
selection and stored-role schedule. Neither this driver nor the neutral arena
wrapper creates a new Native name, trace, procedure-cache or array-operation
issuer. The dictionary-container mutation and other neutral storage writers
retain their independently selected workers; fixed command-slot publication
and whole interpreter teardown are separate contracts.

The six variable-cleanup controls are software ownership/currentness controls
with genuine core, observer and storage setup. Physical C/Jim source joins
bound the selected phases; they provide no matching native callback chronology
or reference-count assertion outcome. Current source-only bindings do not
attribute earlier owned-image results to these changed workers.

The abstract metaclass bootstrap shares a release-selected Registry descriptor
between the VM and Runtime. For C9, the actual initializer publishes a class
whose object maker and class superclass are the foundation class, whose own
constructor and destructor are absent, and whose class-side create,
createWithNamespace and new declarations are unexported. VM bootstrap allocates
the genuine class, object namespace and command; Runtime executes its existing
factory/definition workers from the same initializer data. Compilation capture
still requires the current actual command token, generation, object, class and
namespace. This allocation receives no ordinary/configurable factory role from
its reporting name or pure recipe; the existing factory descriptor remains
independent. C86 has no selected abstract initializer. Other stock metaclass
initializers, inherited method admission and complete bootstrap inventory are
independent contracts. No native callback chronology or execution outcome is
attached to this source contract.

Standalone configured signature scanning retains Symbolic procedure and namespace
metadata for a positively selected hosted source descriptor. It reads complete
original Document words under the actual full configuration through the shared
Unicode word owner. Symbolic records contain no SourceName or NameKey, Native
policy, command allocation or entered frame. A supplied binding inventory keeps
its separate Native, hosted or unavailable contract; it cannot fall back to the
standalone route. Conflicting Native release/point profiles remain unavailable.


Actual VM activation cleanup records its original physical execution target before body callbacks. Plain procedure Frames also retain only weak call-frame and namespace births through the same matching retirement worker as OO procedure exits; OO Method-purpose requirements remain separate. Saved targets carry no entered Scope across suspension. Unwind retains its original target when it consumes the final Frame, and an empty tailcall continuation derives a cleanup-only target from its already issued currency rather than the callback-selected current interpreter.

Mandatory abort retires activation-owned script, operand, result, step-scope and temporary-publication owners in that authentic physical target, then transports the first full typed cause through same-target cleanup and restoration. The actual selected call/namespace kernel preserves callback-reborn activations and any child left current. Readonly independent target observations issue no Native receipt. These software source controls establish no Native callback schedule, object-reference equivalence or broader activation lifecycle coverage.

The VM `SavedExecutionTarget` owns one physical mandatory-retirement kernel.
`retire_in_original` transports already reached metadata and the retained full
first cause; metadata can contain extracted owning retirement iterators or
tickets needed for the next original cleanup step. It grants no ordinary result
publication or new Native role. `in_original` uses the same kernel and retires a
refused returned owner at its reached attachment or restoration boundary.
`CleanupRestoration::RestoreCaller` retains the physical interpreter calling
convention; `PreserveCallbackSelection` preserves the latest actual callback
selection, including deferred finalization, and keeps direct-current mandatory
retirement free of an extra preservation hold. Neither mode issues a fresh
scope, revives withdrawn currency or replaces independent command, variable,
frame, namespace or Method guards.


### Original command deletion and physical cleanup

`NativeCommandCreationProtocol::c_deletion_order` selects implementation-before-import
retirement for C8.4/C8.5 and imports-before-implementation for C8.6/C9. Jim and
neutral storage retain their separate worker. `NativeCommandNameProtocol` owns
independent original deletion epoch stages: C8.4 outer entry, C8.5 outer and
recursive entry, C8.6 entry plus actual final unlink, and C9 recursive or final
unlink. Namespace lookup invalidation follows the outer trace pass after C8.4.
These source predicates supply no live command, namespace or cached lookup role.
The Core `native_command_retirement::retire_original_command` coordinator runs
both already-owned stages in the selected order and preserves the first full
error; each embedding still owns actual roles and mandatory cleanup.

The VM `interp::command_cleanup::OriginalCommandCleanup` captures the actual
interpreter, operation context and command world before callbacks. Its weak
physical observations add no Native, command, Proc or namespace reference.
The selected C dying-generation guard precedes delete traces. Recursive deletion
unlinks that exact generation without repeating implementation/import disposal;
the outer driver still retires the captured implementation. Same-generation
rename, hide and expose remain independently effectful. Every later table step
uses the shared SavedExecutionTarget mandatory-retirement kernel and the original
physical command world. Command and variable retirement select the explicit
PreserveCallbackSelection calling convention; OO cleanup selects RestoreCaller,
including direct-current work. The shared kernel transports reached mandatory
metadata and the retained first cause after actual restoration and finalization.
Independent world, arena, frame, namespace and Method checks retain their own
purposes and grant no new Native role. Trace firing-stack and gate epilogues update their actual original state.
A coroutine's actual published generation is transported with its existing parked
state; old retirement cannot remove a same-key coroutine of a later generation.

Raw unlink receives its actual selected retirement stage from this driver.
It preserves existing neutral and replacement invalidation workers and does not
use a global epoch marker to suppress a callback's ordinary hide/rename. The
original compiler hook, namespace observation and release predicate precede the
trace; the posttrace namespace effect follows the actual surviving generation
after a legal move, retaining the original observation if no visible holder
remains. Outer compiler deletion follows the trace even if recursive unlink
removed the old table entry. No namespace is reconstructed from a hidden spelling.

Thirteen controls bind pure order/epoch rules and genuine software table, trace,
implementation, child and coroutine producers. Original C/Jim full-function LF
joins describe the inspected source and provide no matching Native callback,
reference-count, cache or assertion outcome. The C86+ native namespace temporary
reference, imported-command client-data/reference disposal and Jim refcount-driven
physical schedule retain their independent owners. Fixed destination publication,
whole namespace traversal and complete command/interpreter lifecycle remain
separate contracts. No earlier owned-image result binds these changed workers.

The actual saved OO cleanup continuation transports the same captured execution
target from entry or readonly observation through saved metadata and reattachment;
it never recaptures the callback-selected interpreter on resume. Two genuine
software controls cover mandatory metadata after refusal and default caller
restoration, including scope expiry and same-target saved cleanup. The shared
physical kernel and its own ordinary-result disposal tests are independently
owned by the current SavedExecutionTarget contract.

The Syntax expression owner separates Native checked parsing from checked source
substitution topology. `checked_source_expression_substitutions` uses the same
lexer, Pratt parser, operator-availability and complete-consumption worker,
retaining the original grammar axes and complete bracket ranges. It returns no
Native error, preparation, result representation, folding or execution receipt.
Authenticated Logical immediate-body advice uses this source purpose only after
checking the original input, configured word/image/extent and selected Expr
role. Native advice retains the independently selected Native syntax gate;
unknown Native policy remains unavailable rather than borrowing a C profile.

Runtime original C command deletion uses the shared Core release-selected
Implementation/Imports coordinator. C84/85 withdraw the actual implementation
before depth-first imported commands; C86/90/91 retire imports first. Both
mandatory stages complete after a retained first refusal. The Runtime captures
its existing saved cleanup currency, caller frame birth and caller namespace
incarnation before traces, and keeps the original exact command generation
through recursive unlink, rename, hide and namespace/OO automatic cleanup.
Command epochs follow the selected entry/unlink predicates; namespace lookup
and compiler mutation occur after traces. The pure order and namespace frontier
are retained through mandatory nested cleanup and supply no getter or token
ownership authority.

The actual C ObjCommand delete callback is a once-only role separate from its
Rust dispatch allocation. It runs during true deletion even while that command
is executing. Alias registration and prefix Native roles are also separately
withdrawn; retained Rust alias views observe their allocation and terminally
refuse a retired stored role. Genuine forwarded argv references retain their
existing independent schedule. HostEntry captures the existing Rust command,
registrar table and panic-slot roles under a short read before argument getters,
then releases its raw registration-box borrow before any host callback. No
additional Tcl reference protects the registration box or an alias prefix.

Selected C creation consumes this same original-generation deletion kernel
while preserving its actual old import roster: C84 creation detaches that roster
before Delete, and C85+ uses the redefinition purpose. Its original creation
entry, missing-source import reattachment and idle callback-recreated record
disposal retain their distinct producer contracts. Ensemble replacement keeps
its existing retargeting purpose. Jim and authored neutral record retirement
use their independent final-record callback schedule; the C eager callback
contract is not applied to Jim.

Nine Runtime controls and two actual Engine HostEntry controls inspect active
self-deletion/recreation, imports, alias stored/forwarded roles, first refusal,
recursive and moved generations, namespace/hidden/OO cleanup and the selected
creation handoff. They are software controls with genuine registration/storage
setup. Source joins and the independently owned active-ObjCommand observer do
not establish client-data backing-free equivalence, procedure/import parity,
Native reference-count outcomes or aggregate interpreter teardown coverage.

The genuine OwnedRuntimeShim implementation callback also withdraws its private
HostEntry backing availability before later C84/85 import traces. A subsequent
lookup of the still-visible original token cannot freshly read that retired
Rust Box. This private backing guard is independent of the opaque ObjCommand
final objProc-null timing; it acquires no Native role and makes no Native
client-data backing-free equivalence claim. Its additional software control
reaches actual imported-command traces after private backing retirement and
retains that first typed refusal through mandatory original-token cleanup.


### Conditional roles at a Logical source lookup point

`SourceInvocationBinding::original_conditional_operand_layout_advice` shares the
existing declaration and compiler layout owners, with a separate positively
Logical source route when those observations are absent. That route consumes
the actual immutable post-operand lookup state and the complete original
input, registry, source image, word vector, lexer configuration, namespace,
variable context and target correspondence. It calls the same
`original_site_operand_layout_advice` worker and requires a closed original
registry-backed target set. Unknown observers, absent or replaced targets,
foreign source scopes and changed vectors refuse the projection.

Conditional structure, schema, expression roles, body topology and Logical
scalar source modelling use this shared selector. Native compiler preparation,
inline control, physical frames, completed handlers and Normal results retain
their independent owners; source layout cannot supply those purposes.


The conditional Logical body-topology entry joins the caller's complete
supplied input to the original invocation metadata before selecting runtime,
compiler or source layout advice. Sharing a registry does not permit another
lexer configuration or availability context to donate that source purpose.
Native body-topology routes retain their existing separate owners.

Original Logical argv-child declarations retain the complete original parent
vector, source image, full input/configuration, namespace and declaration frame.
The existing checked substitution collector supplies each complete child vector.
Literal prefixes and lookup-stable literal child commands permit later whole-word
children in the same conditional source world. Variable, template, expansion,
ambiguous nested or unknown callback prefixes stop this projection. The existing
declaration-layout issuer records only these child words under the same original
owner; it creates no RootScript frame, runtime lookup, handler or Native receipt.
Actual Native entries and rejected current Native owners retain their separate
protocols.

### Host callback text and outside-core name projections

`Vm::original_callback_unicode_words` converts each host callback operand
through the existing original String worker and immediately projects its counted
units with `NativeStringProtocol::counted_unicode_text`. It requires an already
entered operation and an actual selected native String provider. Supplied-scope
`OriginalNameConsumerScope` capture retains the original frame, namespace and
world without creating another entry. Its same-stage checker participates in
String materialisation and cache publication. Intrinsic String failures are
retained by the actual entered caller inside the shared Value/compound worker
before post-stage checks or backing retirement; neutral `None` workers retain
their existing explicit-independent purpose. Literal registration composes its
original episode checks and full-cause owner through this same worker.

The Unicode projection is a host text requirement, not a Unicode storage getter
or a name lookup. It retains counted NUL, combines representable C8 surrogate
units and requires scalar units for C9 and Jim. Unrepresentable units retain the
host's supplied refusal; byte consumers remain byte consumers. Pack vocabulary
callbacks and nested error reporting use this sequential text boundary.

Outside-core constructed descriptor keys remove one root marker with
`unroot_rooted_key`; they retain literal-colon components. This includes registered
pack whitelist displays, inferred procedure descriptors, extension reporting
rows and resolved diagram Registry labels. Written pack vocabulary classification
and declared binding spelling use `canonical_written_command` for their explicit
authored source purpose and retain the original spelling separately. Actual
unknown dispatch attempts command lookup before vocabulary classification. These
pure projections grant no Native lookup, package, event, source-edit or compiler
body authority. Rust module paths and restricted stock descriptor labels keep
their independent owners.


Registry descriptor-key projections retain the exact selected descriptor label.
Literal punctuation in that label supplies no constructed root marker, written
command parsing, source policy or runtime binding. Owned invocation facts and
the selected descriptor expose this same pure view; the semantic-operation
roster projects its existing descriptor rows without changing availability.

Compiler execution-trace hazard keys join the selected trace transition to its
actual frozen non-head operand, post-head ordinal and retained namespace lookup.
The canonical native source-spelling worker must round-trip each possible slot
under the original lookup and publication recipe. Opaque, missing or noninjective
inputs leave the hazard dynamic. These keys are possible spelling hazards, not
trace-registration tokens or successful invocation receipts.

At an actual Native invocation, the original complete source input, vector,
current head producer, source instance and frame must match the retained point.
Lookup and execution must select the same complete target identity, generation,
allocation and captured prefixes. The existing source/entered/step observer and
Native entry token owner then supplies trace absence. Its initial token trace
bit is checked even when the module spelling inventory is empty. Aliases, imports
and renames retain their independently selected token and observer obligations;
a descriptor label or current slot spelling cannot replace that owner.

The original binding and module entry issue a separate request-purpose query.
A supplied Native entry retains Execution purpose even when its raw assistance
input permits Logical naming, as Jim does without a Native name protocol.
Execution purpose grants no dispatch, handler entry, compilation or trace
absence. Logical spelling advice requires the actual retained Logical source
owner; Standalone requires the original explicit tagged owner. AuthoredSimulation
requires the same retained entry/state and an explicitly retained naming policy
whose authority is AuthoredSimulation; it remains software source advice and
supplies no Native dispatch, admission or trace absence. Missing purpose
declines without selecting from a profile or repairing from availability.

Implicit descriptor-only operations have no actual invocation receiver and
cannot obtain Native trace absence from a spelling inventory. The original AOT
and Wasm direct-procedure adapters carry their actual call tokens; a procedure
published by source without a matching Native entry row remains unavailable to
this observer check. Explicit authored compatibility joins its original source
metadata before using its separate spelling model. Constructed procedure keys
remove exactly one root marker. Wasm mutation trust receives the exact selected
descriptor label and retains its independent binding and mutation owner.


Selected scope-alias source positions come from the resolved descriptor's
scope-alias trait and complete authored argument roles. Effective VarWrite
ordinals and immediately preceding VarRead ordinals remain role metadata;
they do not identify a target cell, successful link, frame or variable contents.
The authored Registry helpers preserve its exact descriptor lookup/root fallback
and apply their lexical static-name filter after selection. Actual type widening
uses the complete original invocation, input and retained purpose before the
selected traits; missing or foreign ownership widens instead of issuing a
nominal type from a reporting name.

Logical inline-frame eligibility, procedure-purity fallback, source lattice
constant substitution, executable chain advice and raise assistance require the
original retained Logical purpose. A Native entry remains Execution even when
its raw Jim assistance input permits Logical names. Original checked Native and
AuthoredSimulation layouts remain source hints and keep their independent
handler/layout owners. Method reachability uses that same actual invocation
purpose; an unavailable selected branch remains unbounded. Explicit unprofiled
Standalone dispatch compatibility stays separately tagged and cannot repair a
supplied missing or Native entry. Source geometry and metadata do not confer a
splice, erasure, cell, observer, dispatch or completion proof.

### Original authored operand projections

`OriginalOperationObservation` captures the actual operation context, physical interpreter, Host and selected profiles through the existing currency owner. Its private fields expose only capture and currentness checks. An absent entered operation remains absent; the observation grants no Native invocation, saved cleanup, recipe or source/compiler authority.

`OriginalAuthoredOperandScope` combines that observation with the existing original Name world, namespace and frame checker. It captures these owners before the first operand getter. Genuine entered callers retain their existing operation; independent authored callers keep an exact descriptive frame observation, including the absence of a frame. Allocation leases preserve original header access without acquiring Tcl references.

| Projection | Existing worker and purpose |
| --- | --- |
| `string` | Existing resident-first operand getter; stringless values require the actual captured String recipe and Host formatter. A profile label cannot supply that recipe. |
| `raw_bytes` | The legacy `Value` raw String worker with formatter None and checks inside each reached compound child. Retained Native compounds keep their actual recipe; independent scalar rendering keeps its original behavior. |
| `unicode` | The same raw worker followed by strict Rust UTF-8 access. Counted native Unicode decoding remains a separate explicit callback purpose. |
| `list` | The existing object materialization purpose and List kernel, with the same String provider and original checks; owned member extraction preserves the existing reference schedule. |
| `finish` | Retire the original allocation-only lease and settle the same captured owner after mandatory caller cleanup. |

The actual authored binary selectors, namespace keyword table, string selector/counting helpers and static-domain/configuration fronts consume this shared owner. Intrinsic errors are retained inside the String worker before compound backing or unused child ownership retires. Original currentness and header refusals use the same unconditional full-cause latch. A later failed check cannot replace that first cause or publish a refused result. The independent neutral `Value` getters continue through the same renderer with checks absent, preserving its ordering, depth cap, quoting, cache and formatter behavior.

### Compiler source advice purpose

Compiler source-role consumers use `OriginalInvocationSourcePurpose` from the
retained source owner. A held Native entry selects `Execution`, an independently
retained Logical input selects `Logical`, and an explicit retained software
policy selects `AuthoredSimulation`. Explicit standalone metadata keeps its
separate compatibility purpose. Raw input eligibility does not select a purpose.

An actual invocation keeps its original point and full vector checks. Native or
authored declaration and compiler layouts can retain readonly source geometry
through their own exact advice owner when an invocation is unavailable. Neither
layout supplies a reached frame, handler, compilation admission or trace absence.
Module, CFG and materialized-value consumers retain their original input,
configuration, image and command world before querying the same purpose owner.
Missing or foreign ownership remains unknown; existing Native and authored
operation guards stay independent of Logical source analysis.


Reference-body import candidates retain exact closed original Registry advisory
labels under the same Module, whole source input and original argument tokens.
Caller labels are collected before any reference appendix changes the source
image. Written tails and descriptor punctuation cannot select a backing row.
The actual lowered one-procedure backing supplies its constructed declaration
key; Registry's descriptor lookup must select the same backing row for that
produced key. Existing own-definition precedence, live pack/reference facts,
body shape/text, appendix mapping, replayable sites and runtime ReferenceBody
claims remain independent. Candidate or descriptor metadata supplies neither
Native command/namespace identity nor splice or admission authority.


The embedding `OriginalOperandProjection` observes the actual operation/context, Name world, frame and namespace through `OriginalAuthoredOperandScope`; it exposes no currency, entry or Native provider. Raw result/argument bytes keep the existing formatter-None String worker. Selected callback strings use the genuine captured recipe and require strict Rust UTF-8 immediately before the next operand; Tcl/Jim counted Unicode is a separate existing interface. Completion/error cleanup precedes projection settlement through the real outer Host boundary. IRule event enrolment retains the same original per-word owner through actual child publication, and its typed child-registration sibling shares the existing target restoration worker.

Explicit measured event inputs use the same observed-name/storage replacement and invalidation worker under the original issuer. Only the exact forward generation at callback-free field/NameWorld/guard/profile publication can extend that same currency lineage; no new entry or recipe is issued, and unscoped setters still invalidate. The typed publication hook runs before actual cache retirement and the same full original checks run after each reached cache clear. A first refusal never skips mandatory remaining clears. Checked cache retirement uses the captured physical target for each clear while preserving a callback-selected child; neutral cache retirement retains its original direct order. Event storage then naming inputs restore on the captured physical target, with refused Completion ownership retired inside that attachment; callback-caused Host/profile/world/frame/header changes and foreign targets remain terminal. Neutral replacement preserves its prior order and cache algorithms. These are software owner contracts, not physical Native callback or reference-count chronology claims.


Binary format and scan retain one original operand observation through the
selected String, byte-array, numeric and List doors. The captured object purpose
supplies the byte-array result recipe at the existing allocation stage; authored
and standalone materialization keep their independent formatter-None purpose.
The format grammar consumes the original literal-NUL extent through the shared
byte owner. Bad-field units use the selected C character formatter or Jim
scripted byte field and retain their distinct error-code update. The same scan
worker emits each reached field before parsing the next field, so an earlier
variable assignment survives a later grammar failure. The neutral collector
uses that same worker and retains its prior conversion and collection order.
Original operand headers, frame, Name world, Host and full first cause remain
checked across getters, backing retirement and actual assignment callbacks.

Authored rule-callable owner and local target operands retain strict legacy
Unicode access through the same checked raw String worker. Existing RuleIdentity
and procedure-target domain validators, unqualified declaration rule, private
namespace projection, Procedure publication and lookup remain their independent
owners. The observation issues neither a Native entry nor a provider. Missing
providers, retired headers and non-text bytes stay typed refusals before later
operand reads or publication.


Engine materialized argument and returned value ports use the opaque `OriginalOperandProjection` captured before their first getter and retained through unused operand/result/options cleanup. `Vm::original_operand_native_raw_string` invokes the same native String worker with its actual selected carrier protocol and formatter None; legacy raw bytes, selected callback String formatting and strict Rust UTF-8 remain separate interfaces. ByteArray payload and resident/cache snapshots preserve their original non-materializing purpose. Completion option rows and exact-key lookup share the existing cached Dictionary/List algorithm with an optional original delegate; neutral callers preserve their previous worker and cleanup order. No observation issues a Native entry or provider.

The `tcl-engine-tclvm` registrar inherits its actual callback's original projection. Sequential exported operands check the same caller/header before and after each reached getter and before the next word or callback. Returned completion option/key/result/options exports retain that original owner. Terminal public evaluation projects only its genuinely returned Completion; nested registrar evaluation retains the caller's existing owner across execution. Actual NativeExecutionError and ValueAccess failures cross as EngineError::ExecutionFailure, with the first cause retained before mandatory callback result/argument retirement. Context-free primitive-state diagnostics remain descriptive; genuine Tcl compilation and Guest diagnostics retain their separate carriers. Operational OriginalObject duplicate/List/publication and the completion metadata publication worker require their own checked canonical ports; this getter contract grants no such authority or physical Native callback/refcount chronology.

HostCommandShim retirement preserves its existing Host-only failure policy: Script, ScriptBytes and Budget-shaped retirement errors still become actual Host refusals, while an existing typed EngineExecutionFailure retains that full cause. Invocation Guest conversion is not used for retirement. The cause is latched before mandatory original-owner settlement.

Runtime command trace selection retains the original checked CommandId and its generation through add/remove/info. Namespaces::original_rename_source_of selects the original provider-specific rename source then the current physical C token slot. Alias-loop probes and destination publication reuse this adapter; Jim qualified source keys remain separate. Current reports serve callbacks and trace reporting only, and in-flight suppression follows the genuine generation.

Authored/Standalone source name-role ownership is sealed in
`tcl-compiler::registry_invocation::authored_source_structure`. The original
lookup baseline retains its immutable `RegistrySnapshot`; complete original
words, configuration and tagged metadata must agree before consumers reborrow
that store. Current target consensus and captured-prefix refusals remain part of
the role issuer. Supplied, Native, Hosted and unknown-entry ownership cannot
enter the Standalone route. Name-role candidates and unknown coverage grant no
execution, Normal result, physical frame or slot authority. Variable-escape
known-name and implicit-driver consumers use these actual owners and preserve
empty and literal-sigil values through shared word/list rules. The controls are
`registry_invocation::authored_source_structure::tests::authored_source_roles_require_the_actual_store_and_complete_original_vector`,
`var_escape::known_names::tests::original_authored_name_roles_preserve_empty_and_literal_sigils`,
and `var_escape::api::tests::implicit_escape_driver_uses_the_retained_actual_registry`.

Rendered-property source analysis borrows the CFG's retained complete analysis
input and the actual FunctionUnit lexer configuration. Its unanimous statement
carrier must match the recorded original command extent and vector/configuration
before the original source-point owner and selected normal handler can supply
result traits. `NormalRepresentationInvocation::rendered_value_traits` exposes
only the existing path, unescape and HTTP normalisation bits of that normal
handler; conditional layouts and synthetic iteration supply none. Effective
assignment words retain their real alias/written correspondence. A nested result
is classified only from its original whole command word and inherited bindings,
without trimming and reparsing a flattened value. Missing/changed source or
metadata and unsupported compound words yield conservative properties. Only the
explicit Standalone CFG mode retains nominal catalogue classification. These
properties prove neither actual invocation nor Native value production, mutation
erasure or a successful completion. The actual C5/Jim source regression is
`rendered_properties::tests::original_source_result_hints_use_actual_whole_words_and_current_input`.

Runtime retains a genuine original Jim namespace object context independently of C namespace-tree coordinates. `JimNamespaceName::Unmaterialized` carries no name bytes or compiled coordinate. `Namespaces::retain_unmaterialized_jim_namespace` allocates that actual owner; it does not intern a guessed name. `Interp::materialize_jim_namespace` reaches the original getter only under the actual interpreter operation, caller namespace, physical frame birth and same original holder, then `record_jim_namespace_materialized` records those exact resident bytes. Invalidated strings withdraw observed coordinates. A Jim-only context cannot acquire a C constructed path or namespace-name token when the selected policy changes; the genuine global C root stays independently available.

Actual procedure and frame namespace references remain separate native roles. The context indexes only weak owners, prioritising the actual entered frame, so deleting a procedure cannot withdraw a still-live frame's original namespace. Allocation leases preserve safety without delaying a native reference decrement. Getter, cache replacement and cleanup checks preserve the first execution refusal. The command cache accepts identical original namespace pointers without a getter; unequal pointers obtain the cached then current namespace strings, and a cold lookup obtains the namespace before the command name, including an absolute command head. Host-refusal display observes existing coordinates or reports explicit unavailability without acquiring them. Original alias-prefix queries retain the actual issuer and reached lookup owner; a stringless namespace cannot by itself reject a same-pointer cache hit, and a missing command remains separate from a non-alias.

Jim apply retains original list children and uses its actual `apply lambdaExpr` dummy argv head with the original supplied argument objects. Native frame preparation records allocation leases, without creating a lambda display string or reading argument strings merely to install invocation metadata. Error/body/query consumers select their own reached getters. These Runtime adapters reuse existing Syntax/Registry naming recipes and grant no C/F5/Jim-coroutine or full lifecycle equivalence. The original fixed ten-case substitution and seven-case frame-argument SDK captures retain their bounded observation scopes independently of Runtime execution.

An actual rejected original operand does not erase its attempted update syntax.
`OriginalOperandEvaluationUpdate` requires that independently retained failure
point, positive Execution source purpose, exact source input/Registry, closed
original descriptor and direct written argument layout. At least one original
write-name word must be static; expression, body, frame and deferred-script
grammars decline. Syntactic arity describes written slots, never a completed
runtime argv after a failed read. The scalar hook builder retains all original
words, written head, namespace context and guard without entering the structured
body dispatcher or attaching handler expression topology. Generic Call/Barrier
results leave the prior fallback intact. The original rejected-before-handler
guard remains terminal for handler/body admission. Native lowering still needs
its separate original scalar/read/trace/quiet-store proofs and must preserve the
original attempted read and error; this immutable syntax receipt grants none of
those effects, successful completion or executable erasure.
