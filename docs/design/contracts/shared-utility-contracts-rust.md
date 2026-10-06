# Shared-utility contracts (Rust workspace)

The low-level logic every crate must share rather than reimplement, and why.
Behaviour drifts between the Rust crates because equivalent low-level
logic (namespace-name splitting, number parsing, unique-prefix option
matching, canonical error texts, backslash decoding, list codec) is
reimplemented locally. The drifts are subtle and user-visible: a naive
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

<!-- owner-resolution-manifest -->
| Surface | Owner source paths | Public entry points | Dialect/release axis | Drift gate |
| --- | --- | --- | --- | --- |
| names / namespaces | `rust/tcl-syntax/src/naming.rs`; `rust/tcl-syntax/src/naming/native.rs`; `rust/tcl-cmd-core/src/namespace.rs` | `qualifier_segments`; `qualify`; `qualify_namespace`; `canonical_written_command`; `key_tail`; `key_holder_and_tail`; `key_segments`; `root_unrooted_key`; `unroot_rooted_key`; `autoload_command_candidates`; `native_autoload_command_candidates`; `command_resolution_candidates`; `command_resolution_candidates_from_namespace_keys`; `qualifiers`; `tail`; `exists`; `exists_bytes`; `parent`; `parent_bytes`; `children`; `children_bytes`; `which_request`; `which_command`; `which_command_bytes`; `which_command_bytes_checked`; `which_variable_bytes_checked`; `which_variable`; `variable_fqn`; `variable_fqn_bytes`; `import_pattern`; `origin`; `origin_bytes`; `origin_bytes_checked`; `TclStringHashOrder`; `TclStringHashOrder::statistics` | written input, authored constructed-text projection and retained native component geometry remain distinct; `key_segments` and holder/tail helpers choose an analytical split, not an inverse of colliding native displays; `which_variable`'s alternate global candidate is release-selected and 9.0 drops it | `xtask-resolution-drift` |
| authored path constant scope and transport | `rust/tcl-compiler/src/auto_path_eval.rs`; `rust/tcl-compiler/src/auto_path_eval/path_constants.rs`; `rust/tcl-lsp-core/src/workspace_index.rs` | `PathConstantAssignments`; `PathConstantAssignments::naming_policy`; `PathConstantAssignments::before`; `FoldedPathConstants`; `FoldedPathConstants::at`; `FoldedPathConstants::agreement`; `constant_path_assignments_with_naming_policy`; `SourceResolver` | retained authored naming issuer even on empty inventories, original namespace body intervals and source offsets, selected local/global homes and compatible batch/import lifecycles; only global homes export, equal values require equal issuers on every route; unsupported scopes withdraw rather than borrow enclosing values; navigation advice grants no native cell, frame or completion authority | `xtask-owner-resolution` |
| authored naming policy and signature publication | `rust/tcl-registry/src/native_string_materialization.rs`; `rust/tcl-syntax/src/naming/native.rs`; `rust/tcl-compiler/src/signature_scan/mod.rs`; `rust/tcl-compiler/src/signature_scan/scope.rs`; `rust/tcl-compiler/src/signature_scan/types.rs`; `rust/tcl-compiler/src/signature_scan/ctx.rs`; `rust/tcl-compiler/src/signature_scan/handlers.rs`; `rust/tcl-compiler/src/signature_scan/walker.rs`; `rust/tcl-compiler/src/signature_scan/factory.rs` | `InvocationDialect::authored_name_policy`; `NamePolicyAuthority::AuthoredSimulation`; `extract_signatures`; `SignatureScanResult`; `SignatureNamespaceScope`; `SignatureSourceCommand`; `procedures_for_written_name`; `SignatureCommandAliasTarget::selected_global_name`; `SignatureCommandAliasTarget::reported_global_key`; `SignatureCommandAliasTarget::checked_global_key` | pure authored C release/Jim naming, original procedure publication and namespace/rename/alias purposes; unversioned Tcl explicitly selects the C8.6 analysis abstraction, while vendor compatibility does not issue a native policy; C8.4/8.5 reject colon-prefixed procedure tails outside root; procedure/class declaration inventories retain authored slots independently of reported labels; colliding label maps abstain, checked written lookup uses retained C components or Jim flat keys, and WrittenCaller aliases have no global target key; scan records supply source assistance only, without command tokens, compiler selection, entered frames or runtime lookup authority | `xtask-owner-resolution` |
| original array search chains | `rust/tcl-core-types/src/native_array_search.rs`; `rust/tcl-syntax/src/native_array_search.rs`; `rust/tcl-registry/src/native_variable_table.rs`; `rust/tcl-runtime-api/src/native_hash_abi.rs`; `rust/tcl-cmd-core/src/native_array_search.rs` | `NativeArraySearchAbi`; `NativeArraySearchCache`; `NativeArraySearchChain`; `NativeArraySearchProtocol`; `native_array_search_protocol`; `supported_backend_array_search_abi`; `NativeArraySearchBackend`; `NativeArraySearchOperand`; `dispatch`; `resolve` | actual C unsigned-long and int layouts are independent receipts; borrowed original handle/bytes/cache carrier preserves conversion phase without acquiring a header role; C8 resident cache retains signed ID and byte offset while C9 retains the original handle; the owning chain belongs to the original array incarnation, drops handles at native invalidation and does not pin variable cells; undefined-shell collection does not invalidate; C8 descriptors have NULL free, duplicate and updater hooks, while Rust cache cleanup belongs to the sole object-allocation auxiliary owner; Jim has no native Search members | `xtask-owner-resolution` |
| original Jim command and variable lookup caches | `rust/tcl-syntax/src/native_jim_lookup.rs`; `rust/tcl-registry/src/native_jim_lookup.rs`; `rust/tcl-vm/src/interp/native_jim_lookup.rs`; `rust/tcl-vm/src/value/native_jim_lookup.rs`; `rust/tcl-vm/src/frame.rs`; `rust/tcl-vm/src/vars.rs`; `runtime/rust/src/interp/native_jim_lookup.rs`; `runtime/rust/src/frame/jim_lookup.rs`; `runtime/rust/src/obj/native_jim_lookup.rs` | `NativeJimLookupProtocol`; `native_jim_lookup_protocol`; `command_is_current`; `variable_is_current` | actual pinned Jim issuer; command cache owns original namespace only and validates native procedure epoch, counted namespace equality and actual live binding/invocation; an invocation retains its selected node through synchronous or NRE completion while the original command cache never owns the callable; variable cache owns no cell/value/frame and validates selected current/top frame incarnation; new unqualified variable table key owns the original object; absolute key is separate; misses preserve the original primary; metadata/type names grant no lookup authority; internal implementation: `read_original_named_variable`, `store_original_named_variable`, `WeakJimVariableCell` | `xtask-owner-resolution` |
| original Jim local commands and upcalls | `rust/tcl-syntax/src/native_jim_local.rs`; `rust/tcl-registry/src/native_jim_local.rs`; `rust/tcl-vm/src/interp/jim_local.rs`; `rust/tcl-vm/src/interp/native_jim_lookup.rs`; `runtime/rust/src/interp/jim_local.rs`; `runtime/rust/src/namespace/jim_local.rs` | `NativeJimLocalProtocol`; `native_jim_local_protocol`; `changes_epoch`; `follows_previous`; `accepts_upcall`; `accepts_rename` | actual pinned Jim issuer and owning previous-node edges; occupied local publication transfers the previous worker and changes procedure epoch; original top command cache is installed before live upcall traversal; frame cleanup retains original result-name objects and restores bindings in reverse order without an epoch change; nonempty rename with a previous edge refuses; pure policy and cache metadata grant no node lifetime authority | `xtask-owner-resolution` |
| native root bootstrap purposes | `rust/tcl-registry/src/special_vars.rs`; `rust/tcl-vm/src/interp.rs`; `runtime/rust/src/interp.rs` | `NativeBootstrapPurpose`; `NativeBootstrapVariable`; `NativeBootstrapInputs`; `NativeBootstrapProtocol`; `native_bootstrap_protocol`; `allocations`; `registers_core_binary`; `registers_core_try`; `registers_core_throw`; `initializes_tcl_oo`; `with_native_core` | actual constructor root births are separate from Tcl_Main arguments, real script-library initialization, Jim extensions and ordinary embedding-host defaults; undefined cells remain physical births; core-only command gates exclude distribution-owned binary/try/throw/autoload and unsupported OO initialization independently of root variables; build path bytes are independent inputs; logical/vendor profiles cannot issue physical inventory | `xtask-owner-resolution` |
| direct native variable callbacks | `rust/tcl-runtime-api/src/native_variable_trace.rs`; `rust/tcl-registry/src/native_variable_trace.rs`; `rust/tcl-vm/src/interp/native_variable_observers.rs`; `runtime/rust/src/interp/native_variable_observers.rs` | `NativeVariableObserver`; `NativeVariableTraceAccess`; `NativeVariableTraceOperation`; `NativeVariableTraceToken`; `NativeVariableTraceProtocol`; `NativeTraceStatePurpose`; `NativeTraceStateRecipe`; `native_variable_trace_protocol`; `native_trace_state_recipe`; `saves_chain_result`; `saves_script_result`; `add_native_variable_observer`; `remove_native_variable_observer` | selected actual C issuer and original stable cell; callbacks share newest-first trace ordering, active-chain suppression, removal and unset retirement, but remain absent from guest trace introspection; direct callbacks create no script frame or result reset; C8.5+ preserves the interpreter state once per chain; C8.4 script callbacks transfer the result independently, with command-script return code preserved separately | `xtask-owner-resolution` |
| native hidden error-variable callbacks | `rust/tcl-registry/src/special_vars.rs`; `runtime/rust/src/interp/native_error_variables.rs`; `rust/tcl-vm/src/interp/native_error_variables.rs`; `rust/tcl-cmd-core/src/native_append.rs` | `NativeErrorVariableProtocol`; `NativeErrorVariableRead`; `native_error_variable_protocol` | C8.5+ interpreter creation registers hidden callbacks on actual root cells; reads copy retained private objects only under the legacy-copy flag; unset recreates an undefined cell; reset publishes code before info and releases private owners; C8.4 and Jim have no such callbacks; both backends retain original private error objects and carried option values; modern trace chains preserve their selected interpreter state, script callback entry resets the selected error state, and nonempty errorInfo mutations use the shared counted-byte append owner | `xtask-owner-resolution` |
| native variable entry order | `rust/tcl-core-types/src/native_hash_order.rs`; `rust/tcl-runtime-api/src/native_hash_abi.rs`; `rust/tcl-registry/src/native_variable_table.rs`; `rust/tcl-runtime-api/src/lib.rs` | `NativeHashAbi`; `NativeHashOrder`; `NativeEntryLedger`; `supported_backend_hash_abi`; `NativeVariableTableProtocol`; `native_variable_table_protocol`; `authored_variable_table_protocol`; `Frames::var_names_bytes_checked`; `Namespaces::vars_in_bytes_checked` | actual plain-char promotion and unsigned-int/size_t widths are independently issued by the supported backend; C8.4/8.5 variable hashing uses plain char, C8.6 unsigned bytes with unsigned int, C9 unsigned bytes with size_t; Jim requires retained seed and lazy table allocation; authored simulation has independent authority; real entry birth/retirement includes undefined shells, while compiled declaration slots remain ordered and distinct | `xtask-owner-resolution` |
| native name input and reporting | `rust/tcl-syntax/src/naming/native.rs`; `rust/tcl-syntax/src/naming/jim_enumeration.rs`; `rust/tcl-syntax/src/naming.rs`; `rust/tcl-syntax/src/naming/aliases.rs` | `NativeNameProtocol`; `NativeNameProtocol::rename_alias_loop_name`; `NativeNameContext`; `NamePolicyProtocol`; `NativeNameProjection`; `NativeVariableProjection`; `command_publication_slot`; `command_c_api_publication_slot`; `variable_root_input`; `combined_variable_input`; `separate_variable_input`; `trace_registration_input`; `namespace_subcommand_input`; `interpreter_subcommand_input`; `namespace_variable_query_input`; `jim_namespace_canonical_input`; `jim_namespace_construction`; `NativeJimNamespaceConstruction`; `jim_info_command_names`; `jim_procedure_namespace`; `ensemble_publication_slot`; `report_native_namespace_lookup_error`; `report_native_namespace_operation_error`; `report_native_dictionary_missing_key`; `oo_method_input`; `oo_object_publication_slot`; `package_key`; `formal_storage_name_input`; `formal_name_input`; `c_family_local_alias_name_bytes`; `hidden_token_input`; `report_native_name_bytes`; `report_native_variable_access_trace_names`, `report_native_variable_diagnostic`; `report_native_variable_diagnostic_at`; `native_constant_failure_verb`; `checked_command_slot_utf8`; `checked_namespace_path_utf8`; `native_command_source_spelling`; `native_namespace_source_spelling`; `native_jim_namespace_source_spelling` | actual audited engine/build, explicit authored simulation authority, purpose, original byte input form and actual namespace context; command/C API publication, scalar, combined/separate element, trace receiver and report remain separate; checked analytical aliases require exact native round-trip; release-independent authored C alias projection declines unqualified NUL-bearing names and grants no physical lookup or release authority | none |
| native TclOO variable declarations | `rust/tcl-syntax/src/naming/oo_variables.rs`; `rust/tcl-syntax/src/native_glob.rs` | `NativeOoVariableSlotOperation`; `NativeOoVariableSlotSelection`; `NativeOoVariableError`; `native_oo_variable_slot`; `validate_native_oo_variable`; `apply_native_oo_slot_records`; `apply_native_oo_variable_slot` | actual C8.6/C9 variable slot methods; full counted ObjHash membership, original selected declaration object, CString validation and reporting are independent; properties use their own object comparison owner | none |
| original source literal arrays | `rust/tcl-runtime-api/src/native_literal.rs`; `rust/tcl-vm/src/literal_pool.rs`; `runtime/rust/src/interp/native_literal_pool.rs`; `rust/tcl-registry/src/native_eval_object.rs` | `command_literal_partition`; `registered_c84_long`; `NativeSourceLiteralAction`; `source_literal_action`; `source_literal_empty_result_is_unshared`; `permits_direct_source_operands` | actual source object identity, matching literal allocation issuer and real interpreter registrations; C8.6/C9 replaces identical source literals with fresh string-only objects; C8.5 uses hidden command literals and an unshared empty-script result; C8.4 retains each source slot registered in the same live world and withdraws registered local slots at actual interpreter table cleanup; both ports own original ordered local-array members and release actual registration leases before local native refs; registered C8.4 decimal primaries consume the explicit physical native-long width; private constant List members are fresh original String objects; array size grants no ownership authority; equal bytes alone grant no source identity; selected C8.4 direct evaluation retains a plain source control plan and creates fresh original string operands per reached word, with no native literal registration, compiled layout or compiler cache; internal implementation: `NativeRuntimeLiteralArray`, `create_native_literal_array`, `create_native_literal_array_with_actions`, `NativeRuntimeLiteral::UnsharedOriginal`, `finalize_original_source_pool`, `direct_source_unit` | `xtask-owner-resolution` |
| original C variable-name primaries and canonical local names | `rust/tcl-syntax/src/native_variable_name.rs`; `rust/tcl-registry/src/native_variable_name.rs`; `rust/tcl-runtime-api/src/native_literal.rs`; `rust/tcl-vm/src/interp/native_variable_names.rs`; `runtime/rust/src/interp/native_variable_names.rs`; `runtime/rust/src/obj/native_variable_name.rs`; `runtime/rust/src/frame/native_variable_names.rs` | `NativeVariableNameProtocol`; `NativeVariableNameLookupPurpose`; `NativeVariableNameProtocol::alias_local_input`; `alias_local_is_element`; `native_variable_name_protocol`; `NativeParsedVariableName`; `NativeLocalVariableName`; `NativeLiteralWorld`; `NativeLocalNameTable` | actual matching native C name policy; C8.4 failed simple lookup retires only a primary with a physical free hook, C8.5/8.6 retire all, C9 preserves a prior primary on a miss; parsed array parts own original root plus release-specific element storage; compiled local caches validate the actual procedure or canonical name and indexed cell; read, write, link and unset prepare the original cache under their actual frame and lookup purpose; QuietUnset suppresses guest missing-cell presentation while preserving host refusal, and explicit root/index lookup cannot borrow the combined-name parsed-array primary; C8.5+ scalar/root hash entries retain their original key independently of the variable cell; canonical names share one reusable procedure header and unpartitioned global registrations; internal implementation: `native_c_variable_name_protocol`, `prepare_native_name_cell`, `retain_native_key` | `xtask-owner-resolution` |
| retained C scalar aliases and array keys | `runtime/rust/src/frame/native_scalar_alias.rs`; `runtime/rust/src/frame/native_element_entry.rs`; `runtime/rust/src/frame/native_variable_names.rs`; `runtime/rust/src/frame/destruction.rs`; `rust/tcl-vm/src/vars.rs`; `rust/tcl-vm/src/interp/native_variable_names.rs`; `rust/tcl-runtime-api/src/lib.rs`; `rust/tcl-vm/src/interp.rs`; `runtime/rust/src/interp.rs` | `Frames::link`; `VarStore::get_bytes`; `VarStore::set_bytes`; `Vm::try_eval_source_image`; `Interp::eval_str` | each new scalar alias holds its exact variable entry; borrowed link views share that hold; undefined unset and recreation preserve the entry and original key until its final alias retires; namespace retirement releases table ownership independently; C8.4 array keys own bytes; C8.5+ object-key tables retain the original separate index, while combined C8.5/8.6 parsed bytes create a fresh CString key header and C9 retains the original counted parsed index; each element alias holds its exact entry without another key-header reference; single-element unset preserves an undefined aliased entry and key, while whole-array retirement releases each key at actual hash-entry destruction and aliases retain only the disconnected Var until their final hold ends; retired array generations cannot redirect aliases into replacements; internal implementation: `NativeScalarAliasEntry`, `NativeElementAliasEntry`, `retain_native_scalar_alias`, `retain_element_alias`, `prepare_original_native_element`, `retain_original_array_key`, `retire_released_native_alias_entries` | `xtask-owner-resolution` |
| native compiled-variable selection | `rust/tcl-syntax/src/naming/compiled_variables.rs`; `rust/tcl-registry/src/native_compiled_variables.rs`; `rust/tcl-bytecode/src/lib.rs` | `NativeCompiledVariableProtocol`; `NativeCompiledVariableRecipe`; `NativeCompiledVariableEnvironment`; `NativeCompiledVariableLookup`; `CompiledVariableTarget`; `compiled_local_names_equal`; `dynamic_local_names_equal`; `substitution_lookup`; `command_lookup`; `supports_environment`; `native_compiled_variable_protocol`; `authored_logical_compiled_variable_protocol`; `NativeCompiledScalarName`; `NativeCompiledVariableRecipe::scalar_name`; `LocalVarTable::intern_anonymous`; `LocalVarTable::from_native_names`; `from_native_slot_names`; `native_slot_names`; `set_native_protocol`; `find_native`; `intern_native`; `is_source_local_native` | independently selected actual compiler or explicit authored logical provider; counted original names and token geometry; compiler length-plus-CString comparison, dynamic frame-local comparison and dynamic hash keys remain distinct | none |
| native control compiler preparation | `rust/tcl-registry/src/native_control_compilation.rs` | `NativeControlPreparationStep`; `NativeControlOutcome`; `NativeControlCompilation`; `NativeLocalScalarProjection`; `project_native_local_scalar`; `native_control_body_span` | original compiler words, operand indices and exact body spans under the selected C compiler/frame; ordered local, anonymous-slot, word, literal, private Integer/List, script-context, speculative script and expression visits remain independent from Inline, Generic or Rejected outcomes; a decline retains completed declarations and allocations; unavailable geometry is a typed refusal | `xtask-owner-resolution` |
| native conditional and loop instructions | `rust/tcl-registry/src/native_control_instructions.rs`; `rust/tcl-bytecode/src/lib.rs` | `NativeControlBody`; `NativeControlTest`; `NativeConditionalClause`; `NativeControlInstruction`; `NativeCatchProtocol`; `result_before_options`; `Instruction::catch_start`; `Instruction::catch_end`; `NativeControlInstructionUnavailable`; `native_control_instruction` | original if/while/for/catch compiler selection; static script extents and dynamic original operands are distinct; fresh NULL Boolean probes determine pruning and compiler visits; for retains separate body/next loop ranges; catch retains release-selected substitution/range order, stack layout and result/options store order; `Instruction::catch_start` and `catch_end` separate BEGIN lifetime from protected body extent, and C8.4 speculative rejection preserves completed preparation; recipes grant no callable or completed operand authority | `xtask-owner-resolution` |
| native expression operand programs | `rust/tcl-registry/src/native_expression_program.rs` | `NativeExpressionTree`; `NativeExpressionProgram`; `NativeExpressionProgram::compiler_steps`; `NativeExpressionProgramUnavailable`; `NativeExpressionInstruction`; `native_expression_boolean_operator`; `NativeLogicalExpressionCompilation`; `native_logical_expression_compilation`; `native_expression_boolean_word84`; `native_expression_instruction`; `prepare_native_expression_program` | exact counted original expression bytes and direct span mapping with independently selected parser/diagnostic axes; one checked tree or proved syntax rejection, static single SIMPLE_WORD versus original dynamic EXPR_STK operands; original C8.4 fixed-function lookup and child compiler visits stay ordered through `compiler_steps`, independently of command lookup; C8.4 logical preparation registers original pooled 0 then 1 after the left program and before the right program; runtime normalizes the selected left original and preserves eager LAND/LOR versus branch-result recipes; no math-function binding or object-cache authority | `xtask-owner-resolution` |
| original compiled expression number headers | `rust/tcl-bytecode/src/lib.rs`; `rust/tcl-syntax/src/expr/eval.rs`; `rust/tcl-registry/src/native_expression_program.rs`; `rust/tcl-compiler/src/codegen/native_control.rs`; `rust/tcl-vm/src/literal_pool.rs`; `runtime/rust/src/interp/native_body_artifact.rs`; `runtime/rust/src/interp/native_body_artifact/native_control.rs` | `NativeExpressionNumberLiteral`; `NativeExpressionNumberLiteral::number`; `NativeLiteralAllocation::PrivateExpressionNumber`; `LiteralTable::register_private_expression_number`; `NativeLiteralAction::AdoptExpressionNumber`; `LiteralTable::intern_expression_number`; `ExprOps::prepared_node`; `native_expression_boolean_operator`; `native_expression_private_logical_boolean85`; `NativeLiteralAllocation::PrivateLogicalBoolean85`; `LiteralTable::register_private_logical_boolean85` | actual C8.5+ issuer and independently admitted constant operator subtree; exact Integer, Double bits or arbitrary-precision magnitude becomes an absent-string original numeric header in an ordered private slot; resident folded results use chronological cache transfer into the same untyped registered header after deduplication, preserving resident bytes and any earlier typed primary; ordinary Boolean results use registered original zero/one literals; a folded C8.5 logical subtree retains the temporary compiler's same original Boolean header after its registration retires, while C8.6+ resident folds remain registered; prepared-node reuse addresses the exact retained tree before leaves, while ordinary evaluation declines; deferred Syntax packets retain original message/options headers; payloads and mathematical values grant no compiler or object authority | `xtask-owner-resolution` |
| original generic foreach and lmap scheduling | `rust/tcl-runtime-api/src/native_each_loop.rs`; `rust/tcl-registry/src/native_each_loop.rs`; `rust/tcl-cmd-core/src/native_each_loop.rs`; `rust/tcl-vm/src/cmd_control.rs`; `rust/tcl-vm/src/exec.rs`; `runtime/rust/src/cmd_control.rs` | `NativeEachLoopKind`; `NativeEachLoopRecipe`; `NativeEachLoopProtocol`; `native_each_loop_protocol`; `EachLoopState`; `EachLoopState::advance`; `EachLoopState::entered_body`; `EachLoopState::body_completion`; `preparation`; `setter_failure` | actual C/Jim preparation, refresh/setter/body order and completion; cursor advance precedes callbacks, and entered-body progress is recorded only on the Body action; concrete adapters retain original variable/member objects and distinguish lifetime transport from native owners; Jim argv List roots remain borrowed, Foreach owns actual emptyObj as result, and lmap owns its result List while borrowing emptyObj only for padding; native compiler auxiliary storage remains a separate owner | `xtask-owner-resolution` |
| native compiled iterator preparation and storage | `rust/tcl-registry/src/native_each_compilation.rs`; `rust/tcl-bytecode/src/lib.rs` | `NativeEachCollection`; `NativeCompiledEachStorage`; `native_compiled_each_storage`; `NativeEachGroup`; `NativeEachInstruction`; `NativeEachUnavailable`; `compile_native_each`; `NativeEachAuxiliary` | actual C foreach/lmap compiler, original variable-list members/value operands and exact protected body extent; declaration prefix survives decline; real auxiliary slot indices select C8.4 per-setter local refresh, C8.5 per-group List copy, or C8.6+ retained stack lists; value-list/counter temporaries are unnamed physical frame cells, independently of generic iterator handlers | `xtask-owner-resolution` |
| native compiled try handlers and cleanup | `rust/tcl-registry/src/native_try_compilation.rs` | `NativeTryCondition`; `NativeTryCondition::code`; `NativeTryHandler`; `NativeTryInstruction`; `NativeTryUnavailable`; `compile_native_try` | actual C8.6+ compiler and original protected body/handler/finally operands; clauses retain their own result/options bindings and fallthrough body target; release-selected matcher literals/private objects, anonymous completion slots, numeric table and exception ranges belong to compiler preparation; execution preserves original result/options headers, RETURN_STK and original -during dictionaries | `xtask-owner-resolution` |
| original TclOO method records and call-chain caches | `rust/tcl-registry/src/native_tcloo_method_cache.rs`; `rust/tcl-vm/src/cmd_oo/native_method_cache.rs`; `rust/tcl-vm/src/value/native_method_name.rs`; `runtime/rust/src/cmd_oo/native_method_cache.rs`; `runtime/rust/src/obj/native_method_name.rs` | `NativeTclOoMethodCacheProtocol`; `NativeTclOoMethodCacheStamp`; `native_tcloo_method_cache_protocol`; `NativeTclOoMethodCacheProtocol::strings`; `flags`; `reusable` | actual C8.6+ receiver/class creation, Foundation/object mutation epochs and public/private/class-cache flags; actual method tables own mutable records, replacement preserves a record while deletion/recreation creates a new one; cached/reached chains retain original records and original table keys, same original selector primary owns its chain and duplicate shares it; equal distinct headers and metadata never issue a primary or lookup authority; internal owners: MethodTable, MethodSlot, NativeMethodWorld, NativeMethodChain, ReachedMethodChain | `xtask-owner-resolution` |
| native string materialisation | `rust/tcl-syntax/src/native_string.rs`; `rust/tcl-syntax/src/native_tcl_utf.rs`; `rust/tcl-syntax/src/scalar_getter.rs` | `NativeStringProtocol`; `NativeStringInput`; `materialize_native_string`; `NativeStringUnavailable` | resident string bytes preserve exact extent independently of ByteArray backing; pure C ByteArray maps each byte through native U+00XX units; Jim pure ByteArray unavailable; object cache installation and physical empty storage remain adapter facts; no numeric/compiler authority | none |
| original script bytes and lexical boundaries | `rust/tcl-lexer/src/source_map.rs`; `rust/tcl-lexer/src/lexer.rs`; `rust/tcl-lexer/src/script.rs`; `rust/tcl-lexer/src/parse_cut.rs`; `rust/tcl-lexer/src/native_word.rs`; `rust/tcl-runtime-api/src/lib.rs`; `rust/tcl-bytecode/src/lib.rs`; `rust/tcl-compiler/src/compile_service.rs`; `rust/tcl-compiler/src/command_binding/executed_script_source.rs`; `rust/tcl-compiler/src/parsing/syntax/build.rs`; `rust/tcl-compiler/src/segmenter.rs`; `rust/tcl-compiler/src/codegen/emit.rs` | `SourceImage`; `SourceChannel`; `NativeWord`; `NativeWord::from_group`; `ExecutedScriptSource`; `build_document_image`; `segment_commands_image_with_offset_and_config`; `Emit::emit_command_image`; `SourceMap::from_image`; `source_bytes`; `token_bytes`; `Lexer::with_bytes`; `with_source_image`; `group_commands_bytes`; `first_parse_cut_bytes`; `first_parse_cut_image`; `ScriptCompileTargetBytes`; `ProcedureCompileTargetBytes`; `LocalVarTable::from_names`; `intern_bytes`; `CompileService::script_command_plan_bytes_with_entry` | immutable original bytes and explicit native-value/document channel; selected grammar remains independent of bytes and interpreter authority; exact token spans and constructed namespace segments; actual formal storage keys enter one native LVT; checked Unicode views cannot replace an opaque buffer; unsupported byte services return typed refusal; original byte/channel/name and nested parse-cut controls | none |
| executable substitution arena | `rust/tcl-lexer/src/executable_parts.rs`; `rust/tcl-lexer/src/word_parts.rs`; `rust/tcl-lexer/src/native_word.rs` | `ExecutablePartArena`; `ExecutablePart`; `SpannedExecutablePart`; `ExecutableText`; `PartListId`; `decompose`; `decompose_template`; `NativeWord::executable_parts`; `source_span` | one exact SourceImage, original byte extents, flags and separately selected word/template grammar; existing scanners queue arbitrary index nesting into flat ordered lists; Text distinguishes original from decoded bytes; whole spans and native token spans remain distinct; IDs belong to their arena; syntax errors and geometry refusal remain explicit; no command/value/object/effect authority from lexical geometry; advisory tree budget cannot truncate execution; fixed 2,000-index opaque-byte native control, 64 KiB construction/drop, decoded/raw/token span and Jim template discriminants | none |
| retained native namespace context and source keys | `rust/tcl-runtime-api/src/native_compilation.rs`; `rust/tcl-runtime-api/src/lib.rs`; `rust/tcl-compiler/src/command_binding/namespace_context.rs`; `rust/tcl-bytecode/src/lib.rs`; `rust/tcl-vm/src/interp/native_procedure_artifacts.rs` | `NativeNamespaceContext`; `NativeCompilationEntry::retained_namespace_context`; `NativeCompilationEntry::command_lookup_cursor`; `NativeCommandLookupCursor::next_candidate`; `NativeCommandLookupCandidate`; `CompiledNamespaceContext`; `CompiledNamespaceContext::path`; `CommandBindingIdentity::with_namespace_context`; `ProcedureBindingIdentity::with_namespace_context`; `SourceNamespaceKey`; `SourceNamespaceKey::to_compiled_context`; `Instruction::source_namespace_context`; `NativeOperationSelectionSite::replay_namespace_context`; `ProcedureProvenance::namespace_context` | actual interpreter owner, namespace incarnation and exact component boundaries; optional compiled binding/replay context is authoritative when supplied; Native retains actual identity, ConstructedPath retains geometry without a native token; stop-aware lookup validates fallback only when reached; VM binding checks and replay validate native interpreter/token/path/lifetime without display parsing or substitute lookup; procedure-artifact keys retain exact body component geometry and source identity independently of displayed declaration names; constructed paths partition candidates but native reuse requires the selected actual incarnation, and metadata adds no owner reference or dormant-candidate activation; authored contexts and source allocations remain distinct; internal implementation: `SourceCommandKey`, `SourceCommandTable`, `SourceNamespaceMap`, `SourceNamespaceSet` | `xtask-owner-resolution` |
| immutable native compiler byte lookup | `rust/tcl-runtime-api/src/native_compilation.rs`; `rust/tcl-compiler/src/native_byte_compilation.rs` | `NativeCompilationEntry::lookup_command_bytes`; `NativeCommandLookupUnavailable` | original bytes and namespace incarnation resolve against one immutable actual snapshot; unique registration, closed absence and typed residual remain distinct; C CString/path/global lookup and Jim namespace-object keys are selected independently; constant heads require absent compiler hooks while dynamic heads never borrow runtime hooks; every bracket child retains its own admission obligation; no runtime handler/effect or opcode authority; opaque names, hook presence, path search, retained deletion and nested-child certificate controls; internal implementation: `closed_generic_byte_compilation` | none |
| native command-source error logging | `rust/tcl-registry/src/native_error_log.rs`; `rust/tcl-syntax/src/native_tcl_utf.rs` | `NativeErrorLogProtocol`; `native_error_log_protocol`; `excerpt`; `observes_error_words`; `object_vector_command`; `command_log_excerpt`; `character_complete` | actual C release and original counted source; original argument updaters at public object-vector failure, release-selected already-logged observation, C-string formatting extent, native incomplete-character rules and clipping; no Jim, authored-grammar or host UTF-8 fallback | none |
| runtime command-table identity | `rust/tcl-core-types/src/lib.rs`; `rust/tcl-core-types/src/name_bytes.rs`; `rust/tcl-runtime-api/src/command_identity.rs` | `NameBytes`; `ByteNamespacePath`; `ByteCommandSlot`; `NativeByteCommandSlot`; `CommandSlot`; `StaticNamespacePath`; `StaticCommandSlot`; `display_namespace`; `display_command`; `encode_command_slot`; `CommandId`; `NsId`; `OoId` | invariant; written-name parsing remains selected at ingress by the document/runtime dialect | none |
| VM native command placement | `rust/tcl-vm/src/interp.rs`; `rust/tcl-vm/src/interp/native_name_world.rs`; `rust/tcl-vm/src/embed.rs`; `rust/tcl-runtime-api/src/lib.rs` | `Namespaces::find_command_bytes_checked`; `Namespaces::find_namespace_bytes_checked`; `Namespaces::command_name_bytes`; `Vm::command_names_bytes`; `Vm::retain_commands_bytes`; `Vm::command_names`; `NativePublicationService::prepare`; `Vm::native_publication_service`; `Vm::registered_command_token_bytes` | one interpreter-owned `Rc<RefCell<NativeNameWorld>>`; exact namespace token and `NameBytes` select placement; private String keys are storage indexes, never written names; slot, display and generation queries return owned results before lifecycle callbacks or interpreter switches; reserved slots without generations are absent; token handles cannot follow a same-named replacement; Unicode enumeration returns a typed decoding error for unrepresentable names; Unicode whitelists remove opaque unlisted names; callbacks consume owned entries without table guards; command-trace operands use byte list quoting and the shared source ingress, whose unsupported byte source remains an explicit refusal; internal implementation: `NativeNameWorld`, `command_slot`, `command_storage_key_at_slot`, `live_command_generation`, `command_display_key_bytes`, `command_sidecar_display_bytes`, `resolve_command_bytes_checked`, `namespace_object_lookup`, `native_procedure_publication` | none |
| positioned command navigation | `rust/tcl-compiler/src/command_binding/command_reference.rs`; `rust/tcl-compiler/src/command_binding/linked_definition.rs`; `rust/tcl-compiler/src/signature_scan/types.rs`; `rust/tcl-compiler/src/analyser/types.rs` | `SourceCommandReference`; `SourceCommandReferenceBinding`; `SourceCommandDefinition`; `SourceCommandDefinitionKind`; `command_reference`; `evaluated_command_reference`; `linked_definition`; `written`; `retain_reference`; `clear_positioned_reference`; `proc_for_definition`; `class_for_definition` | exact post-argument slot and current implementation allocation, full source origin and selected grammar; evaluated command receipts are independent of source-exact contributor edits; called definition differs from terminal navigation, observed/opaque alias edges decline; no opcode, normal-completion or object-class licence | none |
| actual native compilation entry and artifact guards | `rust/tcl-runtime-api/src/native_compilation.rs`; `rust/tcl-runtime-api/src/lib.rs`; `rust/tcl-compiler/src/compile_service.rs`; `rust/tcl-compiler/src/codegen/emitter/mod.rs`; `rust/tcl-compiler/src/command_binding.rs` | `NativeCompilationEntry`; `NativeCompilationEntry::same_compilation_world`; `NativeNamespaceVariableTable`; `NativeCompilationBinding`; `NativeCompilationNamespace`; `NativeCommandImplementation`; `NativeInterpreterIdentity`; `CommandBindingGuard`; `CommandBindingIdentity`; `CompileError`; `CompileService`; `BytecodeCompileService::native_entry_options`; `native_entry_config`; `SourceAnalysisOptions::native_compiler_dialect`; `SourceAnalysisOptions::logical_invocation_dialect`; `SourceAnalysisOptions::native_lexer_config`; `SourceCompilationScope`; `ModuleEmissionScope`; `codegen_module_with_emission_scope`; `codegen_procedure_module_with_emission_scope` | typed runtime targets exclude deferred declaration analysis and lowering before CFG preparation and emit only entered source; explicit AOT retains owned procedure inventory; direct options and adapter share exact live-policy precedence; measured source grammar retains driver coordinates/modes independently from logical absence; independently retained source profile, logical invocation policy and physical engine point; missing logical/physical evidence never borrows the other or editor assistance; original argv materialisation retains logical lexer/word rules; actual interpreter identity and command allocations; alias and namespace-unknown prefix capture retains exact argv presence without materialising stored objects; distinct chunk-entry and before-argument validation; independently captured original namespace variable-root tables contain allocation membership only, including undefined and linked entries; source-cache equality includes these rows without changing native compiler epochs or running-frame freshness; unsupported adapters decline optimized compilation; typed compile-service admission refusal reaches the retained host channel without becoming a guest completion; internal implementation: `compile_script_with_entry`, `compile_procedure_with_entry`; internal implementation: `InterpState::source_profile` | none |
| original namespace-name objects | `rust/tcl-syntax/src/native_namespace_name.rs`; `rust/tcl-registry/src/native_namespace_name.rs`; `rust/tcl-runtime-api/src/native_namespace_name.rs`; `rust/tcl-vm/src/interp/native_namespace_names.rs`; `runtime/rust/src/interp/native_namespace_names.rs`; `runtime/rust/src/namespace/native_namespace_name.rs`; `runtime/rust/src/obj/native_namespace_name.rs`; `rust/tcl-cmd-core/src/namespace.rs`; `rust/tcl-engine-api/src/lib.rs`; `rust/tcl-runtime-api/src/lib.rs`; `rust/tcl-syntax/src/native_glob.rs` | `NativeNamespaceNameRecipe`; `native_namespace_name_protocol`; `authored_namespace_result_recipe`; `NativeNamespaceNameToken`; `NativeNamespaceNameCache`; `NamespaceObjectBackend`; `current_original`; `parent_original`; `children_tokens_checked`; `children_original`; `namespace_objects_original`; `NamespaceDeleteBackend`; `delete_original`; `OriginalObjectResult::NamespaceName`; `namespace_children_lookup`; `Namespaces::find_namespace_child_bytes_checked` | actual original primary, closed namespace incarnation and reference context, physical lifecycle and separate parent detachment; C84 updater and unresolved conversion remain distinct from later no-updater primaries; actual C producers and explicitly authored logical result factories grant separate permissions; serialized bytes and metadata never grant cache authority; internal implementation: `native_namespace_object_lookup` | `xtask-owner-resolution` |
| native command-object cache and compiler literal actions | `rust/tcl-registry/src/native_command_literal.rs`; `rust/tcl-runtime-api/src/native_command_name.rs`; `rust/tcl-vm/src/interp/native_command_names.rs`; `rust/tcl-bytecode/src/lib.rs`; `rust/tcl-engine-api/src/lib.rs` | `NativeCommandNameProtocol`; `native_command_name_protocol`; `native_compiled_command_literal`; `native_compiled_command_name_literal`; `native_compiled_command_name_literal_from_lookup`; `native_compiled_selected_command_name_literal_from_lookup`; `NativeCommandNamePrimingAuthority`; `cache_is_current`; `preserves_primed_cache`; `invalidates_path_before_object_create`; `NativeLiteralContext`; `NativeCommandNameCache`; `NativeLiteralAction`; `NativeLiteralAction::RetainSyntaxErrorInfo`; `LiteralTable::retain_syntax_error_info`; `LiteralTable::intern_native_command_bytes`; `prime_native_command_name`; `hide_native_literal`; `OriginalObjectResult::CommandName` | actual C release, original object, retained interpreter/namespace incarnation, node and reference epochs; ordered allocation/priming/hiding remains independent of byte deduplication; C91 Syntax retention validates original local message/options array slots and installs that SAME message as the private Dict error-info member; CompilerSelected retains the original public registration/configuration and selected worker, without lookup by its reporting bytes; Jim and explicit logical simulation use selected lookup without C cache; reporting bytes and serialized snapshots grant no live node or original-object authority; internal implementation: `native_command_from_original`, `native_namespace_command_name` | `xtask-owner-resolution` |
| ensemble usage rewrite reset order | `rust/tcl-registry/src/native_ensemble_rewrite.rs`; `runtime/rust/src/interp/stock_ensembles.rs` | `NativeEnsembleRewriteProtocol`; `EnsembleRewriteResetEvent`; `native_ensemble_rewrite_protocol`; `ensemble_rewrite_protocol` | actual engine authority; C8.5 clears after successful ordinary lookup, C8.6+ before lookup and bytecode entry; internal invocation preserves, parser usage adaptation has separate exact-object scoped lifetime; C8.4/Jim have no C rewrite state, explicit logical provider grants no native authority | none |
| selected primitive scalar getter | `rust/tcl-syntax/src/scalar_getter.rs`; `rust/tcl-syntax/src/scalar_getter/float.rs`; `rust/tcl-syntax/src/scalar_getter/errors.rs`; `rust/tcl-syntax/src/native_tcl_utf.rs`; `rust/tcl-registry/src/invocation_words.rs`; `rust/tcl-registry/src/native_numeric_error.rs` | `native_scalar_getter_protocol`; `NativeScalarGetterProtocol`; `materialize`; `cached_conversion`; `fresh_conversion`; `fresh_conversion_with_range_error`; `NativeScalarGetterConversion::into_parts`; `failure_presentation`; `NativeScalarGetterErrorCode`; `NativeTclUtf`; `expression_operand_error_code_policy` | actual native engine/build and independent getter kind, original RawString/ByteArray materialization, full cache magnitude and preparation before mutation; Jim signed-boundary Wide requires retained native range state or abstains; primitive getter presentation and expression-stage codes remain separate; no parser, effects, expression source, numeric constant or opcode authority | none |
| conditional original expression advice | `rust/tcl-registry/src/conditional_expression.rs`; `rust/tcl-compiler/src/command_binding/normal_result.rs`; `rust/tcl-compiler/src/type_infer.rs` | `ConditionalExpressionEvaluation`; `ConditionalExpressionEvaluation::prepare`; `ConditionalExpressionEvaluation::tree`; `ConditionalExpressionEvaluation::lexer_grammar`; `ConditionalExpressionEvaluation::context`; `ConditionalExpressionEvaluation::numeric_reentry_is_idempotent`; `ConditionalExpressionEvaluation::normal_result_representation`; `ConditionalExpressionPoolState` | checked original syntax and retained input occurrences; OriginalSource analytical advice meets only same-original source/frame/namespace issuers, while handler-time EvaluatedSource or materialised observations neither donate nor invalidate it; conditional normal representation is separate from reached completion, PreparedExpressionWitness, fixed math registration, native CPP and physical headers; source templates retain Conditional reachability; TypeInfer uses the original stamped source/declaration receipt for result, AssignExpr and Return advice; current authored numeric-pool state meets independently of stock object class and withdraws on representation-changing getters, numeric command-name conversion or unknown callbacks; unknown functions/scripts and unsupported syntax decline | `xtask-owner-resolution` |
| actual native numeric thread state | `rust/tcl-platform/src/numeric_environment.rs`; `rust/tcl-cmd-core/src/native_numeric.rs`; `rust/tcl-syntax/src/expr/errors.rs` | `NumericEnvironment`; `NumericErrorState`; `c84_nonfinite_error`; `fresh_c84_conversion`; `NativeFloatError::classify` | reached actual host errno, independent EDOM/ERANGE facts, selected reset and unsigned/double calls; original cache lookup precedes fresh conversion; finite/NaN observations preserve thread state, infinity requires authentic domain/range state; raw unknown errno stays unknown, and absent capability refuses rather than inventing baseline state; both normalizers use this owner and selected C8.4 arithmetic error publication retains an actual String header | `xtask-owner-resolution` |
| reached original numeric operand cache | `rust/tcl-registry/src/native_numeric_conversion.rs`; `rust/tcl-registry/src/runtime_expr_validation.rs` | `integer_relational_operand_conversion`; `integer_index_operand_conversion`; `NativeOperandNumericCacheProduction`; `with_original_cache_class` | retained prepared C8.5+ numeric `<` getter branch or selected runtime scalar index stage, exact original object/current cache class and successful integer-only conversion; Double preservation differs from fresh String/List/ByteArray conversion, unknown class stays Numeric; immediate bytecode indices and grouped children cannot donate original cache; no canonical-string, value, effects or opcode proof | none |
| native integer arithmetic | `rust/tcl-dialect/src/arithmetic.rs`; `rust/tcl-registry/src/invocation_words.rs`; `rust/tcl-syntax/src/expr/wide.rs`; `rust/tcl-syntax/src/number_tower.rs`; `rust/tcl-compiler/src/tcl_expr_eval.rs` | `NativeArithmetic`; `arithmetic`; `normalizes_expression_result`; `WideError`; `literal`; `binary`; `unary`; `BigIntOps`; `FoldPolicy`; `for_retained_entry`; `with_invocation_dialect`; `InvocationDialect::characters` | selected native integer tower and embedding character model independently of catalogue and numeral grammar; retained lexer overlay does not replace native engine axes; explicit undefined-native outcomes and unchanged-string results | none |
| native double string representation | `rust/tcl-dialect/src/double_string.rs`; `rust/tcl-syntax/src/number.rs`; `rust/tcl-registry/src/special_vars.rs`; `rust/tcl-vm/src/value.rs`; `rust/tcl-vm/src/interp.rs`; `rust/tcl-registry/src/invocation_words.rs`; `rust/tcl-dialect/src/profile.rs` | `DoubleStringPolicy`; `DoubleFormat`; `double_string_policy`; `format_double_selected`; `parse_double_precision`; `SpecialVariableHook`; `name_in`; `changes_read_value` | exact native engine identity; thread-shared C8.x precision and lazy first string materialisation; C9 shortest and Jim fixed twelve digits; implicit read/write hooks are storage-identity observers; internal implementation: `DoubleFormatContext`, `native_double`, `with_native_double_format`, `double_representation` | none |
| selected deferred script navigation | `rust/tcl-registry/src/selected_script_timing.rs`; `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-compiler/src/registry_invocation.rs` | `InvocationFacts::deferred_script_argument_indices` | retained selected command/subcommand/form, frozen cardinality, authored roles and available option grammar; unknown value-dependent timing, selector, expansion or concatenated payload retains explicit unknown coverage; mapped original operands are positive navigation metadata only, never callback absence, future entry, erasure or edit-completeness permission | none |
| typed callback input metadata | `rust/tcl-registry/src/registry.rs`; `rust/tcl-compiler/src/taint.rs`; `rust/tcl-compiler/src/registry_invocation.rs` | `callback_taint_inputs_words`; `instance_callback_taint_inputs_words`; `callback_taint_inputs`; `ResolvedStatementInvocation::with_argument_words` | exact evaluated argv cardinality and selected deferred positional body; unknown captured prefixes preserve slots; unproved option layout or expansion abstains without placeholder bytes | none |
| neutral native execution refusal | `rust/tcl-runtime-api/src/lib.rs`; `rust/tcl-runtime-api/src/native_execution_error.rs`; `rust/tcl-vm/src/interp.rs`; `rust/tcl-vm/src/exec.rs`; `rust/tcl-vm/src/embed.rs`; `rust/tcl-engine-api/src/lib.rs`; `rust/tcl-engine-tclvm/src/lib.rs` | `NativeExecutionError`; `NativeExpressionRefusal`; `NativeExpressionFailure`; `CompileError::Message`; `CompileError::NativeCompilationAdmission`; `CompileError::Unsupported`; `NativeCompileServiceRefusal`; `NativeHostCommandRefusal`; `refuse_host_command`; `EngineError`; `VmCompilationError`; `try_compile_function`; `try_define_procedure`; `try_invoke_function`; `try_eval_source`; `try_eval_expr`; `try_invoke_command`; `try_run_module`; `try_run_function` | unresolved native compilation admission and reached unsupported expression domains remain host errors; immutable expression bytes/full profile/frame/interpreter/namespace diagnostic context, without a resumable continuation; engine retention survives interpreter switches; prior effects stay applied, guest catch/try/finally, completion writes and traces cannot consume refusal | none |
| original pre-handler argument rejection | `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/command_binding/pre_handler_failure.rs`; `rust/tcl-compiler/src/registry_invocation.rs` | `SourceCommandBindings` | internal implementation: `original_arguments_rejected_before_handler`, `logical_structured_invocation`; unchanged original words and source occurrence, exact typed namespace/frame and actual-only all-Error operand routes without normal continuation; actual handler dispatch or unrepresented possible body entry withdraws normal-transfer rejection; unknown namespace uncertainty covers every context, exact known keys filter selected views; declaration previews supply neither actual rejection nor uncertainty; original AST/declaration grammar remains analytical without physical entry, CPP, effects or read-value authority | none |
| source and IR command bindings | `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/command_binding/retained_source.rs`; `rust/tcl-compiler/src/command_binding/body_template.rs`; `rust/tcl-compiler/src/command_binding/compiler_inventory.rs`; `rust/tcl-compiler/src/realm.rs`; `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/command_binding/compiled_invocation.rs` | `SourceCommandBindings`; `RetainedSourceModuleBindings`; `SourceAnalysisOptions`; `SourceAnalysisEntry`; `TrustedPackageLoader`; `SourceInvocationBinding`; `proved_class_definition_factory`; `SourceRuntimeReachability`; `native_compiler_dialect`; `native_compilation_admission_selection`; `SourceCommandTarget`; `SourceCommandTarget::registry_identity`; `analyse_command_binding`; `CommandBindingRealm`; `variable_accesses_for_invocation_args`; `document_realm_bindings`; `document_realm_bindings_with_config`; `restore_script_bindings`; `BodyProofScope`; `NativeBodyTemplate`; `BodySourceProofs`; `prepare_native_body_template`; `restore_cfg`; `restore_statement`; `for_each_statement`; `for_each_script_mut`; `child_scripts`; `child_scripts_mut`; `tokens_mut` | selected runtime family, positioned execution namespace, exact invocation offsets, separate compiler coverage and runtime reachability, explicit entry and loader contracts; registry_identity retains the original semantic descriptor identifier independently of renamed/imported live slots and grants no lookup spelling; compiler-only recipes cannot provide reached effects; editor projections retain uncertainty; immutable lowering projections are reused only with exact source image, grammar, registry semantic key, entry policy, compilation scope and frame identity; internal implementation: `source_analysis_frame` | none |
| positioned command-slot advice | `rust/tcl-compiler/src/command_binding/command_presence.rs`; `rust/tcl-compiler/src/command_binding/future_bodies.rs` | `SourceCommandSlotPresence`; `selected_slot_presence`; `selected_slot_diagnostic_presence`; `diagnostic_slot_presence_at`; `SourceFutureBodyInventory::invocation_at` | exact evaluated head, raw alias/import wrapper, independent namespace paths and retained document origin; default autoload possibility remains separate, custom/unknown fallback withdraws absence advice; no execution or registry-key licence | none |
| declared command assistance | `rust/tcl-registry/src/model/declaration.rs` | `DeclaredCommand`; `DeclaredSurface`; `DocumentCommandSurface`; `declared_command` | authored argument shape and retained trust provenance; slot applicability comes from the source owner, independently of runtime command presence | none |
| original declaration navigation | `rust/tcl-compiler/src/registry_invocation/declaration_assistance.rs`; `rust/tcl-compiler/src/command_binding/declaration_layout.rs`; `rust/tcl-compiler/src/realm.rs` | `OriginalDeclarationAssistance`; `DeclarationArgument`; `CommandBindingRealm::original_declaration_assistance`; `stamp_original_tokens` | unchanged source and retained declaration frame; only OriginalDeclaration layouts supply declaration advice, with unanimous allocation, words, grammar and typed context; EnteredActivation observations remain independent and cannot donate or veto declaration semantics; frozen alias arguments and mapped written positions; no prefix-only or expansion token; no execution, physical cell or store authority | none |
| effective invocation words | `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/word_subst.rs` | `EffectiveCommandWords`; `InvocationWordOrigin`; `ResolvedStatementInvocation`; `RegistryInvocationAssistance`; `RegistryInvocationShape`; `written_argument_roles`; `invocation_argument_role_assistance`; `invocation_argument_role_consensus`; `CatalogueInvocationAssistance`; `registry_invocation_assistance`; `catalogue_invocation_assistance`; `DeclaredInvocationAssistance`; `resolved_declared_assistance`; `resolved_statement_invocation`; `resolved_tokens_invocation`; `effective_command_words`; `argument_spellings`; `argument_presentations`; `with_argument_words`; `argument_literal`; `written_argument`; `resolve_command_tokens`; `resolve_word_exprs`; `resolve_word_exprs_with_dialect`; `inherit_nested_bindings`; `lifted_source_expressions` | selected escape/list/brace rules and implementation provenance; statement facts and effective arguments share one proof; alias values have no written argument span; role assistance unions retained candidates and applicable declarations without granting execution or contents reads; definite grammar roles require every actual candidate, a complete accepted argv layout and no opaque or absent residual; selected invocation role mapping retains its existing proof strength; internal implementation: `evaluated_arguments`, `source_expression_from_invocation`; private body-assistance facade maps original operand roles, candidate case-list/definition grammars and nominal loop operations for navigation, while scoped definitions and alias/value-copy summaries require its separate accepted definite invocation; offset-free assistance explicitly withdraws every definite field; name-effect traversal uses original lifted child receipts and preserves an opaque residual for unsupported or truncated substitution inventories, while braced data never becomes a script; internal implementation: `checked_lifted_calls` | none |
| evaluated expression source | `rust/tcl-compiler/src/command_binding/executed_expression_source.rs`; `rust/tcl-compiler/src/ssa.rs` | `ExecutedExpressionSource::from_arguments`; `ExecutedExpressionSource::variable_source`; `SsaSourceView::read_executed_expression_variable` | selected native expression argument grammar and shared concatenation; exact piecewise original operand read sites; decoded, captured, expanded and cross-piece references have no authored-site proof; grants neither compiler preparation nor source edit eligibility | none |
| nested normal handler lookup | `rust/tcl-registry/src/native_handler_path.rs`; `rust/tcl-registry/src/native_compilation.rs` | `NativeHandlerLookupPaths::select`; `NormalHandlerImplementationLookup::RequiredPath`; `SuccessfulHandlerSpec::stock_native_workers` | exact frozen selector and cardinality choose an authored ordered ensemble path; each intermediate map, command token and terminal worker requires independent live identity proof; stock inventory enumerates every eligible edge without selecting a path; neither projection supplies compiler operation, hook, result or observer proof; native handler assertions are explicitly excluded by SpecTcl and Studio | none |
| temporal constructor entry | `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/command_binding/deferred_method.rs`; `rust/tcl-registry/src/definer.rs` | `SourceConstructorEntry`; `SourceInvocationBinding::constructor_entry`; `is_instance_constructor`; `preserves_local_constructor_entry` | current class incarnation and original constructor declaration, formals, evaluated body and receiver frame; original native manufacturer payload and shared formal arity; later redefinition, argument callbacks and unknown inherited/filter chains abstain; advisory parameter edges confer no manufactured class or completion proof | none |
| temporal receiver navigation | `rust/tcl-compiler/src/command_binding/receiver_self.rs`; `rust/tcl-compiler/src/command_binding/object_instance.rs`; `rust/tcl-compiler/src/command_binding/deferred_method.rs`; `rust/tcl-lsp-core/src/receiver_identity.rs` | `SourceReceiverMethodEntry`; `is_exported`; `SourceMethodReceiver`; `class_definition_method_entries`; `object_receiver_method_entry`; `named_object_receiver_method_entry`; `receiver_self_method_entry`; `frozen_object_receiver_method_entry` | exact class allocation; internal consumer projections: `original_class`, `method_at_command`, `method_at_cursor`, `class_at_read` and original member source/frame and declaring class independently of actual receiver class; static byte-exact selector required for edits, separate Class/Instance tables, original variable-object read or installed named command retained through argv; actual analysis realm and lexer map only original local declarations; definition, hover, references, rename and type-definition share the projection; nominal completion and absent method-name inventories supply no editing or execution licence | none |
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
| variable cell identity and overlap | `rust/tcl-compiler/src/place.rs`; `rust/tcl-compiler/src/var_resolve.rs`; `rust/tcl-compiler/src/raw_binding.rs`; `rust/tcl-compiler/src/allocated_instance.rs`; `rust/tcl-compiler/src/var_resolve/cell_key.rs` | `RawBindingArena`; `RawBindingSlotId`; `capture_callable_statics`; `install_callable_statics`; `retarget_raw_binding`; `raw_static_unset_error`; `detach_retained_binding`; `AllocatedInstanceLinkOutcome`; `link_allocated_instance_variables`; `retire_allocated_instance_variables`; `CellOwner`; `CellGeneration`; `CellIdentity`; `Place`; `overlap`; `places_read_to_form`; `ResolveContext`; `ContentsOrigin`; `VariableExecutionFrame`; `NamespaceCellPresence`; `canonical_variable_name`; `canonical_namespace_variable`; `VariableCellKey`; `VariableCellKeyQuery`; `VariableCellTable`; `VariableCellSet`; `VariableNamespaceSet`; `compatibility_name`; `canonical_binding_value_key`; `canonical_place_key`; `canonical_literal_variable_key`; `namespace_contains`; `cell_key`; `resolve_place`; `resolve_access`; `resolve_literal_access`; `resolve_substitution_access`; `resolve_target_access`; `resolve_alias_destination_slot`; `enter_called_frame`; `selected_frame_context`; `restore_execution_frame`; `project_access`; `resolve_dict_path` | selected Tcl/Jim lookup policy, actual activation owner, cell lifetime and trace observation; `ReceiverMethod` has a known procedure-local activation while receiver namespace/instance ownership remains unknown; actual bounded First/Second object allocations may separately supply typed instance-cell owners, sequential local links and exact-owner retirement; outward frame restoration and alpha relocation preserve those cells from retained typed slots, never class or object names; native NamespaceIdentity retains actual interpreter/incarnation/components; primary fact queries require exact typed keys, while string queries address only Authored storage; compatibility names are diagnostic presentation and cannot recover native, retained, activation or lifetime identities | none |
| point-specific variable bindings | `rust/tcl-compiler/src/variable_bindings.rs`; `rust/tcl-compiler/src/place_bridge.rs` | `PointResolveContexts`; `build_point_resolve_contexts`; `build_point_resolve_contexts_with_entry`; `before_statement`; `after_statement`; `before_terminator`; `source_tokens_at`; `transfer_statement`; `read_places`; `def_places`; `write_observed_by_unknown_access`; `uncertain_variable_output_addresses`; `source_variable_write_places_with_output_order`; `transfer_source_namespace_cells_with_output_order` | registry-owned binding/frame/trace transitions and CFG successful/abrupt joins under the selected entry; actual selected output order can invalidate later physical destinations after an earlier observer, while unknown order retains every possibility and namespace destruction remains a physical clobber without an invented SSA value | none |
| original statement source carriers | `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/variable_bindings.rs` | `retained_source_tokens_for_statement`; `source_input_tokens_at` | owned Script statements or exact CFG block/index; one private selection helper requires original direct and consumed carriers to agree; typed synthetic evaluation phases retain their own explicit input carrier independently of runtime replay sites; Script assistance rejects synthetic phases | none |
| structured source word references | `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/word_expr.rs`; `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/native_lowering/cells.rs` | `SourceVariableAccess`; `SourceStockLiteralObject`; `WordExpr::from_word`; `sole_variable_substitution`; `native_lexer_config`; `SourceCommandBindings::variable_accesses_for_invocation_args`; `SourceCommandBindings::variable_accesses_during_expression_invocation` | selected ingress lexer grammar and original component source extent; variable components retain verbatim spelling independently of compatibility argv text; quoted sole references retain their individual sites, compound and braced data words supply no sole read; substituted and literal index syntax stays distinct without proving physical scalar/element storage; actual native-family/frame lookup remains separate; current physical read receipts additionally require unique original site and spelling; private frozen literal RHS/store/read carriers retain effect-only stock object classes without strict representation, numeric contents or freshness permission; unknown effects and missing joins withdraw them; private prepared-argv read receipts retain original source/site/spelling, selected word mapping, physical contents origin and full source attestation; command observer boundaries withdraw them and native leaf consumers never re-evaluate index words; private per-context selected addresses freeze evaluated index parts before later effects and before the outer read observer, relocate actual cells and contexts together, and withdraw conflicting or unknown keys; sole-child expression type advice validates exact evaluation-parent ownership, unchanged source, literal preceding words and declaration-local scope before referencing an existing SSA version; earlier effects, formal inputs, aliases, traces and unavailable declaration flow withdraw it; it adds no SSA use, normal fact or executable read; no execution-admission authority; internal implementation: `declaration_expression_reads`, `DeclarationExpressionReadAdvice::diagnostic_version`, `variable_word_place`, `variable_reference_place` | none |
| source-owned formal caller advice | `rust/tcl-compiler/src/command_binding/formal_call_advice.rs`; `rust/tcl-compiler/src/command_binding/formal_value.rs`; `rust/tcl-compiler/src/analyser/diagnostics/var_command.rs`; `rust/tcl-compiler/src/analyser/diagnostics.rs`; `rust/tcl-compiler/src/interprocedural.rs` | `Analyser::emit_cfg_ssa_diagnostics_with_cu`; `collect_call_by_name_reads` | original callee allocation, accepted declaration frame and incoming/copy provenance; direct unentered projection requires exact conditional handler-site incoming provenance, separate from definedness/literals, intersected at joins and withdrawn by writes/removes/aliases/possible outputs/unknown effects; original caller argument-entry flow, native formal binder and effective written or captured-prefix operand origin; literal candidates support W307 absence advice only; unknown external calls remain possible; prior unknown effects, changed allocation, overwritten formals and unsupported bindings withdraw candidates; no actual activation, selected target, SSA value or executable authority; internal implementation: `declaration_formal_call_values`, `DeclarationFormalCallValue::value_in_source`, `symbolic_declaration_formal_value_at`, `symbolic_declaration_formal_components_in_word`, `symbolic_formal_value_at`, `conditional_handler_keeps_incoming` | none |
| reachable cell contents | `rust/tcl-compiler/src/ssa.rs`; `rust/tcl-compiler/src/def_use.rs`; `rust/tcl-compiler/src/cell_state_ssa.rs`; `rust/tcl-compiler/src/memory_ssa.rs`; `rust/tcl-compiler/src/variable_bindings.rs`; `rust/tcl-compiler/src/var_resolve.rs`; `rust/tcl-compiler/src/compilation_unit.rs` | `SsaFunction`; `SsaStatement::destruction_defs`; `SsaSourceView`; `SsaReadReference`; `is_positioned`; `read_spelling`; `source_tokens`; `read_reference`; `read_word`; `read_expression_variable`; `read_expression_incoming_slot`; `SsaIncomingSlotRead`; `read_completion_at`; `read_contents_presence_at`; `SsaReadContents`; `read_contents_at`; `read_word_contents`; `read_expression_variable_contents`; `source_symbols_at`; `at_statement`; `at_terminator`; `symbol`; `source_names`; `source_symbols`; `reaching_binding`; `reaching_bindings`; `context_before`; `DefUseResult`; `build_ssa_with_context`; `cell_key`; `cell_keys`; `cell_name`; `cell_names`; `cell_symbol`; `var_name`; `var_symbol_at`; `var_symbol_at_terminator`; `chain_for`; `build_cell_state_ssa`; `build_memory_ssa_with_cfg`; `VariableProofRelocation`; `relocated_variable_proofs`; `restored_source_proofs` | point-specific cell address and generation; reachability, traces and unknown alias exposure; SSA cell_key/cell_symbol and typed DefUse chains retain primary storage identity; cell_name/var_name are presentation/source projections; MemoryLocation.storage_key is separate from name, and deterministic presentation ordering supplies no identity; destruction versions invalidate contents without values, representations or WrittenAt store provenance; conditional transitions retain predecessor use and may-def membership, and taint tombstones preserve predecessor influence | none |
| actual read success, store contents and current representation | `rust/tcl-compiler/src/var_resolve.rs`; `rust/tcl-compiler/src/ssa.rs`; `rust/tcl-compiler/src/contents_source.rs`; `rust/tcl-compiler/src/command_binding/normal_variable_continuation.rs`; `rust/tcl-compiler/src/command_binding/source_representation.rs`; `rust/tcl-compiler/src/command_binding/container_coercion.rs`; `rust/tcl-compiler/src/command_binding/native_result.rs`; `rust/tcl-compiler/src/native_numeric.rs` | `read_produces_value`; `contents_already_native_numeric_at`; `contents_native_string_access_closed_at`; `contents_native_numeric_category_at`; `SourceNativeNumericShape`; `read_word_produces_value`; `normal_store_contents_preserved`; `normal_variable_continuation`; `read_word_representation`; `read_word_representation_alternatives`; `read_expression_representation`; `read_expression_representation_alternatives`; `ClosedContainerRepresentations` | actual native scalar/array or dictionary-container policy, defined current contents, cell incarnation, full-origin attestation and observer closure; read success is independent of an SSA address/version; the normal store receipt preserves semantic contents only, while current representation requires unanimous exact read evidence and may change without a contents write; private frozen result receipts carry representation independently of bytes and require a known unchanged shared representation epoch; frame restoration retains the issuance high-water mark, while divergent joined histories permanently withdraw further stamps; a typed immediate constructor result may pass its native result boundary and publish its actual shape to the final unobserved store without becoming a frozen receipt; selected successful Increment stores, actual native ListLength count results and private-minted prepared runtime arithmetic/bitwise result descriptors publish numeric intrep independently of unknown bytes before callbacks, which may withdraw it; literal/pooled/forwarded/Boolean expression results cannot borrow the arithmetic production; ordinary Value formals may transfer only an actual frozen result receipt to their own Incoming activation slot; the point-local numeric shape intersects all live normal producer alternatives of the same engine, retaining equal Int/Double categories or Numeric for disagreement independently of joined writer origin; missing/nonnumeric/opaque alternatives withdraw it; this can exclude simultaneous ordinary List/Dict identity but supplies no concrete numeric value, operand, epoch or object identity; exact frozen operand bytes may exclude disjoint current container objects from a coercion footprint, without claiming unique native allocation; closed List/Dict alternatives are conversion-cost advice only, require every original read to succeed in a live unobserved cell, and keep strict representation Unknown when alternatives disagree; selected native store/result numeric shape may only exclude an ordinary container at a successful current read, retaining strict representation Unknown and no concrete numeric contents | none |
| reached original numeric operand cache | `rust/tcl-registry/src/native_numeric_conversion.rs`; `rust/tcl-compiler/src/command_binding/numeric_operand_cache.rs`; `rust/tcl-compiler/src/native_numeric.rs` | `NativeOperandNumericCacheProduction`; `NativeNumericOperandClass`; `integer_relational_operand_conversion`; `integer_index_operand_conversion`; `with_original_cache_class` | actual prepared C85+ relational numeric branch or selected runtime scalar index getter, original word/site and physical read, integer-only numeric branch, independent current cache class and closed conversion effects; only the same live normal continuation publishes current cache; original bytes can coexist with Double and cannot prove Int or a value; initial lindex requires original Integer cache, excludes grouped child conversion; Inline literal indices and unknown compilation decline; C84/Jim relational recipe abstains; no primitive Wide, value, epoch, completion, freshness or erasure donation; private source capture/finish keeps pre-getter evidence separate from result setters and read/store schedules | none |
| captured read representation cost | `rust/tcl-compiler/src/ssa.rs`; `rust/tcl-compiler/src/shimmer/commit.rs`; `rust/tcl-compiler/src/shimmer/hints.rs`; `rust/tcl-compiler/src/shimmer/use_site.rs`; `rust/tcl-compiler/src/shimmer/expr.rs` | `CommitWalker`; `CommitFacts::walker`; `compute_commit_facts`; `SsaSourceView::read_word_representation`; `SsaSourceView::read_expression_representation` | one common captured physical read projection and cost reconciliation; actual current ordinary List/Dict/String supersedes earlier SSA commitment, closed container alternatives remain May advice, native numeric shape is independent of unknown container representation; native named operands require post-argv read inventory; no constant, erasure, object identity or compiler admission licence; internal implementation: `captured_read_representations`, `cost_at_read`, `at_captured_representation`, `representation_cost_type`; internal implementation: `SsaReadRepresentationAdvice`, `read_word_representation_advice`, `read_expression_representation_advice`, `native_read_representation_advice`, `RepresentationCost`, `cost_for_word`, `cost_for_expression`, `cost_for_native_read` | none |
| temporal contents folding and existence | `rust/tcl-compiler/src/sccp.rs`; `rust/tcl-compiler/src/existence_query.rs`; `rust/tcl-compiler/src/var_resolve.rs`; `rust/tcl-compiler/src/place_bridge.rs` | `existence_constant_branches_with_ssa`; `ContentsPresence`; `def_places_with_continuation` | exact nested dispatch, frozen target names and actual read/write observers; future effects cannot erase earlier SSA versions; actual `in_expr_at` stays separate from original diagnostic existence advice, whose name dependencies assert no contents and grant no SCCP branch erasure; internal implementation: `in_expr_for_diagnostics_at`, `in_tokens_for_diagnostics` | none |
| evaluated script source | `rust/tcl-compiler/src/command_binding/executed_script_source.rs` | `ExecutedScriptSource`; `ExecutedScriptMapping`; `from_word`; `materialised`; `list_element`; `base` | exact frozen script bytes and semantic origin; literal word/list byte equality proves contiguous mapping; decoded, substituted and concatenated bodies retain distinct materialised origins and origin-qualified source proofs | none |
| cardinality-only argument roles | `rust/tcl-registry/src/spec.rs`; `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-spectcl/src/loader.rs` | `ArgRoleResolverInput`; `ArgRoleCountResolver`; `InvocationSemantics::arg_role_resolver_input`; `CommandSpec::arg_role_count_resolver`; `SubCommand::arg_role_count_resolver` | exact evaluated argc independently of operand values; foreach body position and scan/binary-scan output slots use this axis even when input or format values are unknown; unknown expansion cardinality retains incomplete roles; count callbacks receive no fabricated values; SpecTcl accepts known native descriptors and excludes unsupported Tcl count hooks; Studio field coverage and SpecTcl round-trip tests | none |
| structured argument layouts and consensus | `rust/tcl-registry/src/resolved_invocation.rs`; `rust/tcl-registry/src/registry_role_consensus.rs`; `rust/tcl-registry/src/spec.rs`; `rust/tcl-registry/src/invocation_words.rs` | `ArgRoleLayoutResolver`; `InvocationOptions::available`; `arg_role_assignments_consensus`; `RoleOperandAlternatives` | exact argv cardinality and selected option availability; unknown values remain unknown; closed bounded alternatives donate only unanimously identical roles, never a singleton value, operation or runtime handler; value/count/layout resolver declarations are mutually exclusive; native regex availability and layout-conflict controls; SpecTcl/Studio round trips | none |
| deferred script entry frame and possible callers | `rust/tcl-registry/src/body_execution.rs`; `rust/tcl-compiler/src/command_binding/future_bodies.rs`; `rust/tcl-compiler/src/unit_scope.rs`; `rust/tcl-compiler/src/ir.rs` | `BodyExecutionSpec::DeferredGlobalScript`; `deferred_entry_frame`; `DeferredBodyFrame`; `SourceFutureBodyInventory`; `SourceFutureCallSite`; `Module::future_call_sites` | selected exact Body operands and actual native scheduling policy; native after/default and idle enter the global frame later; registering-call normal effects and independently possible future effects remain separate; unknown host policy supplies no frame proof; exact possible future bindings retract caller seeds without producing them; no normal continuation/body-entry licence | none |
| procedure analysis body inventories | `rust/tcl-compiler/src/command_binding/generated_procedures.rs`; `rust/tcl-compiler/src/lowering/installed_bodies.rs`; `rust/tcl-compiler/src/specialise_factories.rs`; `rust/tcl-compiler/src/command_binding/origin_inventory.rs` | `SourceInstalledProcedureBody`; `SourceProcedureImplementationBody`; `SourceCommandBindings::selected_source_in_context`; `installed_procedure_body_units`; `original_declaration_body_units`; `specialise_factories_with_cap` | installed inventory requires exact current singleton allocation/source instance; original declaration inventory independently retains each original allocation/source/key despite later deletion, replacement or opacity, transported by Module.original_declaration_body_units; neither inventory belongs to executable roots or grants entry/CPP; evaluated native formals and authoritative namespace_key distinct from namespace/command presentation; exact selected_source_in_context retains point/compiler/read observations only in that original context; analysis-only body coverage and bounded metadata, original factory calls retained, no callable installation or compilation licence; Script.namespace_context transports the body key into CFG and the central function_source_entry, while original-name projections remain diagnostic | none |
| conditional provider definition installation | `rust/tcl-registry/src/definer.rs`; `rust/tcl-compiler/src/command_binding/snit_definition.rs`; `rust/tcl-compiler/src/command_binding.rs` | `DefinitionCommandInstallation`; `DefinitionDispatcher`; `TrustedPackageLoader::definition_dispatchers`; `lookup_dependencies`; `installed_lookup_dependencies`; `SourceInvocationBinding::nominal_definition_name_result` | driver-attested exact factory recipe, live installed token/generation; preload lookups must already resolve before loading, installed lookups are checked against the actual published commands after loading, and both sets remain current prerequisites; private/core lookup, namespace-state and execution-observer prerequisites; distinct SnitType/ItclClass installation recipes and qualified/written name-result protocols; normal returned name is advisory callable-name possibility only, with no object existence, class, constructor closure, opcode or specialization permission | none |
| actual source-file entry and caller correlation | `rust/tcl-registry/src/source_file.rs`; `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/unit_scope.rs` | `SourceFileGrammar`; `SourceFileSelection`; `select`; `completion_route`; `TrustedSourceModuleLoader`; `SourceOriginId::loaded`; `SourceAnalysisEntry::trusted_source_modules`; `CommandAllocation::matches_source_declaration`; `scan_source_call_sites_with_source_entry` | selected native file argv and return grammar; driver-attested path, encoding, EOF-decoded bytes and file implementation identity; reached loader runs in the caller frame with a retained Loaded origin; declaration correlation preserves actual allocation and grants no executable or edit proof | none |
| native script compilation | `rust/tcl-registry/src/native_compilation.rs` | `NativeCompilationMode`; `NativeCompilationFrame`; `NativeCompilationGrammar::ProcedureHookFrom`; `NativeCompilationGrammar::NamespaceOrigin`; `NativeCompilationGuard`; `NativeCompilationWordShape`; `NativeCompilationContext`; `with_inline_exception_range`; `NativeCompilationSelection`; `NativeCompilationFailure`; `NativeCompilationFailureScope`; `NativeCompiledBodyContext`; `NativeCompiledBodyOperand`; `entered_context`; `NativeCompiledBodies`; `NativeCompilationStep`; `NativeCompilationSteps`; `NativeCompiledExpressionOperand`; `NativeCompiledExpressionErrorContext`; `NativeExpressionCompilation`; `compilation_steps`; `validate`; `compiled_substitutions`; `NativeCompilationGrammar`; `NativeAppendKind`; `validates_empty_result`; `NativeBodyCompilation`; `NativeCompilationSpec`; `select`; `failure_for_selection`; `compiled_bodies`; `body_context`; `body_context_for_operand` | actual direct/bytecode entry and compiler-local-table capability, separately from physical variable frames; selected complete no-body roles establish empty compiler traversal independently of runtime expanded argv length, while unresolved body roles and compiler selection retain uncertainty; C 8.4 chunk guard vs later before-argv guard; Jim late dispatch | none |
| native compilation error presentation and admission | `rust/tcl-runtime-api/src/native_compilation_error.rs`; `runtime/rust/src/interp/native_compilation.rs`; `rust/tcl-bytecode/src/lib.rs`; `rust/tcl-compiler/src/codegen/native_failure.rs` | `NativeCompilationError`; `NativeCompilationErrorCommand`; `NativeCompilationPreflight`; `NativeCompilationAdmissionError`; `error_info_for_procedure`; `validate_native_compilation_entry`; `native_compilation_admission_error` | proved result/error code plus ordered raw compiler-command contexts and authored expression/child-body notes; actual procedure name supplied at pre-formal admission; missing presentation retains a host preflight obligation; failed commands retain native guard dependencies; UTF-8-safe native 150/50-byte excerpts; internal implementation: `retain_dependencies`; retained rejection stops before executable source emission, retaining reached original prefix registrations/enclosing compiler selections/fixed math as chunk-entry prerequisites; later dependencies stay outside rejected traversal; an earlier unresolved compiler visit withdraws later guest error presentation and retains native provider admission | none |
| native compiler backend admission | `rust/tcl-compiler/src/native_compilation_admission.rs`; `rust/tcl-runtime-api/src/native_compilation.rs`; `rust/tcl-compiler/src/codegen/wasm/pipeline.rs`; `rust/tcl-runtime-api/src/native_compilation_error.rs` | `NativeCompilationAdmission`; `NativeCompilationAdmissionScope`; `NativeCompilationAdmissionPlan`; `script_admission_plan`; `retained_script_admission`; `NativeMathFunctionPrerequisite`; `NativeMathFunctionTable`; `NativeMathFunctionBinding` | exact executed source origin, compiler-token dependencies and actual fixed math registration identity; unresolved provider obligations remain independent of proved failures; complete host script entry precedes stores and procedure-table installation; source procedure admission precedes formal binding; unavailable source refuses host execution | none |
| original procedure body creation | `rust/tcl-registry/src/native_procedure_body.rs`; `rust/tcl-vm/src/interp.rs`; `runtime/rust/src/interp/native_procedure_body.rs` | `NativeProcedureBodyCreationProtocol`; `NativeProcedureBodyAction`; `native_procedure_body_creation_protocol` | actual supported native string owner, original sharing observed before temporary ownership and counted resident spelling; shared ordinary C bodies become fresh string-only objects before formal parsing, unshared bodies and Jim retain the same original; precompiled ProcBody objects require separate authority; compilation attaches to the chosen body, never a bytes-equal input substitute; internal implementation: `capture_procedure_body_value`, `choose_original_procedure_body` | `xtask-owner-resolution` |
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
| lambda invocation selection | `rust/tcl-registry/src/lambda_invocation.rs` | `LambdaInvocation`; `LambdaInvocationSelection`; `select_lambda_invocation` | native family, selected list grammar, formal parameters, body, explicit/default namespace and trailing arguments | none |
| diagnostic source and overwrite advice | `rust/tcl-compiler/src/script_binds.rs`; `rust/tcl-compiler/src/registry_invocation/store_advice.rs`; `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/analyser/diagnostics.rs`; `rust/tcl-compiler/src/realm.rs` | `Analyser::emit_cfg_ssa_diagnostics_for_function_full`; `Analyser::emit_cfg_ssa_diagnostics_with_cu`; `CommandBindingRealm::original_declaration_assistance` | original source image/channel and byte geometry; unchanged accepted authored declaration/formals for potential local-read diagnostics; declaration-owned candidate layout for conditional lexical bodies, preserving opaque runtime alternatives; exact adjacent original setters, same local cell lifetime and previous contents writer; strict overwrite advice requires closed execution/variable/object observers, while conditional overwrite advice retains object-callback uncertainty; missing or conflicting premises decline; diagnostic advice only; no reached activation, physical undefined contents, executable body entry, successful dispatch or dead-store elimination; internal implementation: `script_image_binds_name`, `authored_procedure_read_advice`, `AuthoredProcedureReadAdvice::owns`, `overwritten_local_store_advice`, `conditional_overwritten_local_store_advice`, `conditional_unread_local_store_advice`, `conditional_body_topology_advice`, `SourceCommandBindings::declaration_operand_layout_advice`, `SourceInvocationBinding::declaration_operand_layout_advice`, `SourceInvocationBinding::declaration_read_occurrences`, `DeclarationReadOccurrenceAdvice::diagnostic_version`, `SourceInvocationBinding::declaration_flow_report`, `DeclarationFlowReport::invocation_may_be_reached`, `conditional_index_access_advice`, `SourceInvocationBinding::original_compilation_lookup_advice` | none |
| evaluated body execution | `rust/tcl-registry/src/body_execution.rs`; `rust/tcl-compiler/src/execution_region.rs`; `rust/tcl-compiler/src/executable_ir.rs`; `rust/tcl-compiler/src/semantic_analysis.rs`; `rust/tcl-registry/src/native_compilation.rs`; `rust/tcl-compiler/src/registry_invocation.rs` | `BodyExecutionSpec`; `CapturedLifecycleSpec`; `BodyExecutionSelection`; `ExecutionRegionDependency`; `EvaluatedBodyRegion`; `ExecutionPhase`; `RegionSelection`; `RegionTarget`; `WrapperEffectProjection`; `EvaluatedRegionCompletion`; `PossibleBodyRegion`; `ConditionalBodySource`; `PossibleBodyCondition`; `PossibleBodyTopology`; `possible_body_invocation`; `GenericInvoke`; `registry_specialisation_arguments_exact`; `evaluated_regions` | selected literal argument grammar, exact provider version/hook prerequisites, ordered caller-frame phases and residual effects | none |
| case-list possible body locations | `rust/tcl-registry/src/case_bodies.rs`; `rust/tcl-registry/src/spec.rs`; `rust/tcl-compiler/src/command_binding/case_bodies.rs`; `rust/tcl-registry/src/script_body_flow.rs`; `rust/tcl-registry/src/native_compilation.rs` | `CaseBodyOperands`; `CaseListSpec::possible_body_operands`; `script_body_flow_in_registry`; `ScriptBodyFlow::CaseBodies`; `PossibleBodyTopology::CaseAlternatives` | shared authored option/clause/list grammar; frozen effective argv and native list policy; each body retains its original argument and decoded list element; unknown subject preserves selection/error residual; actual entered scripts retain exact source origin; partial unknown option layouts remain unresolved | none |
| invocation completion routing | `rust/tcl-registry/src/completion_route.rs`; `rust/tcl-registry/src/registry.rs` | `ReturnStateEffect`; `native_return_state_effect`; `ReturnCompletionRoute`; `InvocationCompletionRoute`; `ProcedureCompletionPolicy`; `procedure_completion_policy`; `invocation_completion_words`; `exact_invocation_completion_words`; `invocation_completion_knowledge`; `invocation_completion_route`; `normal_possible`; `abrupt_possible`; `immediate_code`; `through_procedure_boundary`; `through_procedure_boundary_in` | native option grammar retains eventual code and unwind level; only procedure boundaries settle pending returns; unknown retains normal and abrupt continuations; C/Jim raw control-code boundaries differ | none |
| native package database and original option lookup | `rust/tcl-registry/src/native_package.rs`; `rust/tcl-registry/src/native_index_lookup.rs`; `rust/tcl-core-types/src/native_index.rs`; `rust/tcl-vm/src/interp/native_index_lookup.rs`; `runtime/rust/src/interp/native_index_lookup.rs`; `runtime/rust/src/obj/native_index.rs`; `rust/tcl-vm/src/interp/native_package_files.rs`; `rust/tcl-vm/src/interp.rs`; `rust/tcl-vm/src/cmd_package.rs`; `runtime/rust/src/cmd_package.rs` | `NativePackageProtocol`; `native_package_protocol`; `c_members`; `member_usage`; `retains_version_object`; `tracks_files`; `failure_error_code`; `hash_recipe`; `NativeIndexLookupProtocol`; `NativeStaticIndexTable`; `native_index_lookup_protocol`; `NativeIndexLookupFlags` | actual C8.4–9.1/Jim protocol independent of logical grammar; native byte version conversion and copied script buffers; native bad-member, preference, version and loader-completion failures publish the selected private error-code header before returning carried metadata; C86+ original version headers, C9 original private files List; ABI-aware STRING_KEYS births and removal; original Index cache hit before getter and foreign-origin refusal; retained immutable table authority independent of spelling; exact native table stride, C9 TEMP_TABLE and NULL_OK; canonical Index usage without replacing the original header; internal implementation: `native_index_from_original`, `native_index_operand`, `native_index_from_original_with_flags`, `native_index_usage_bytes`, `record_package_source_file`, `package_version_object` | `xtask-owner-resolution` |
| frame selector and package policy | `rust/tcl-registry/src/frame_effect.rs`; `rust/tcl-registry/src/state_transition.rs`; `rust/tcl-registry/src/invocation_words.rs`; `rust/tcl-dialect/src/profile.rs`; `rust/tcl-dialect/src/version.rs` | `FrameLevel`; `FrameLevelWord`; `FrameArgumentResolution`; `FrameEffectSpec`; `VariableAliasDestination`; `VariableAliasFrame`; `is_active_in_frame`; `is_active_in_frame_with_policy`; `resolve_arguments`; `FrameLevelPresence`; `CompletionOptionsPolicy`; `catch_positional_arity`; `completion_options_policy`; `VariableLookupPolicy`; `VariableLinkBinding`; `variable_link_binding`; `VariableContainerModel`; `variable_container_model`; `PackageProtocol`; `NamespaceImportBinding`; `upvar_level_presence`; `uplevel_level_presence`; `package_protocol`; `namespace_import_binding`; `validate_version_for`; `validate_requirement_for`; `compare_versions_for`; `select_package_version_for` | optional-level presence separate from numeral grammar; C release and Jim runtime family remain independent; no-runtime policies abstain | none |
| lists | `rust/tcl-syntax/src/list.rs` | `find_element`; `split_list`; `list_element`; `join_list`; `append_list_element`; `junk_fragment` | invariant | none |
| native list-result serialization | `rust/tcl-syntax/src/list_result.rs`; `rust/tcl-registry/src/native_result.rs`; `rust/tcl-registry/src/spec.rs`; `rust/tcl-registry/src/invocation_words.rs` | `NativeListResultSerialization`; `InvocationDialect::list_result_serialization`; `ordinary_range_literal_result`; `constant_range_literal_result`; `run_const_fold_in` | actual C84/C85+/Jim native list-object bytes after shared element parsing and selected index grammar; portable folds require byte agreement; rendering grants values independently of current input class, callbacks, completion and erasure | none |
| dictionary scope binding | `rust/tcl-compiler/src/dictionary_bindings.rs`; `rust/tcl-compiler/src/var_resolve.rs`; `rust/tcl-registry/src/dictionary_scope.rs` | `enter_dictionary_scope`; `finish_dictionary_scope`; `scope_marker_effects`; `DictionaryScopeId`; `DictionaryScopePlan`; `DictionaryWritebackFailure`; `writeback_failure_route`; `stock_scripted_wrappers` | frozen original keys/path and live physical cells at completion-sensitive epilogue; actual C/Jim completion policy and exact joins/relocation retain the source allocation | none |
| dicts | `rust/tcl-syntax/src/list.rs`; `rust/tcl-syntax/src/value.rs`; `rust/tcl-cmd-core/src/dict.rs` | `find_element`; `split_list`; `canonical_dict_slots`; `ValueOps::dict_pairs`; `ValueOps::dict_hash_bucket_count`; `ValueOps::new_dict_with_hash_bucket_count`; `worded_parse_error`; `dict::info` | invariant | none |
| native glob matching | `rust/tcl-syntax/src/native_glob.rs`; `rust/tcl-syntax/src/native_glob_case_mapping.rs`; `rust/tcl-syntax/src/raw_string.rs` | `NativeGlobProtocol`; `NativeGlobObject`; `NativeNameGlobPurpose`; `match_native_name_pattern`; `match_native_glob_objects`; `match_c_string_glob`; `equal_c_strings`; `name_pattern_uses_exact_lookup` | independently selected actual or authored issuer; original object type/cache/resident bytes; C counted Unicode, pure byte array and CString dispatch; exact name lookup separate from scans; pinned native simple casing and Jim numeric units; explicit CString entry points preserve terminated extents and opaque bytes without selecting an object getter; absent storage or unsupported reached access refuses; fixed native object/CString matrices | none |
| Unicode glob compatibility | `rust/tcl-syntax/src/glob.rs` | `string_match`; `string_match_bytes`; `string_case_match` | Unicode analysis/display strings and Rust casing; no native byte/object or lookup proof | none |
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
| native compiler operands and cache mutation | `rust/tcl-registry/src/native_compilation.rs`; `rust/tcl-registry/src/native_instruction_plan.rs`; `rust/tcl-registry/src/native_procedure.rs`; `rust/tcl-registry/src/native_ensemble.rs`; `runtime/rust/src/namespace.rs`; `runtime/rust/src/interp/native_compilation.rs` | `NativeCompilationSpec::requires_original_word_preparation`; `native_instruction_plan`; `NativeInstructionPlan`; `native_compiler_cache_invalidated`; `configuration_compiler_mutations`; `Interp::native_compiler_cache_epochs` | original complete compiler words, retained physical selection/context, actual raw compiler attachment, independent interpreter/compiler and namespace/resolver counters; copied import attachments, genuine token retirement, real setter transactions and first/last execution trace transitions; recipe projection supplies no indexed-local or executable capability | none |
| original compiled named invocation | `rust/tcl-registry/src/native_instruction_plan.rs`; `rust/tcl-registry/src/native_command_literal.rs`; `runtime/rust/src/interp/native_body_artifact/native_named.rs` | `NativeNamedInvocationInstruction`; `NativeNamedInvocationWord`; `native_named_invocation_instruction`; `NativeInstructionPlan::NamedInvocation`; `NativeNamedWorkerCompilation`; `native_named_worker_instruction`; `native_compiled_command_name_literal_from_lookup` | independently admitted selection, retained original operands/expansion flags and canonical replacement layout; same private pooled head at compilation, Direct before operand preparation and Rewrite after original/canonical argv preparation; actual handler resolves by ordinary lookup after argv effects; pure layout grants no binding or callable identity, and unavailable dynamic rewrite extent remains a typed residual | `xtask-owner-resolution` |
| native compiler parser word projection | `rust/tcl-registry/src/native_compiler_word_projection.rs` | `NativeCompilerWordOperand`; `NativeProjectedCompilerWord`; `project_native_compiler_words` | original C8.5+ pure TEXT argument expansion, native source channel and exact list member spans; malformed, backslash-generated and substituted expansions remain expanded; no runtime List header, callback or executable authority | none |
| native namespace binding compilation | `rust/tcl-registry/src/native_namespace_binding_compilation.rs`; `rust/tcl-registry/src/native_namespace_upvar_compilation.rs`; `rust/tcl-compiler/src/codegen/native_namespace_upvar.rs`; `runtime/rust/src/interp/native_body_artifact.rs` | `NativeNamespaceBindingKind`; `NativeNamespaceBindingCompilation`; `NativeNamespaceBindingOutcome`; `NativeNamespaceBindingVisit`; `compile_native_namespace_bindings`; `compile_native_namespace_upvar` | actual C8.5–9.1 procedure compiler; release-specific final TEXT tails and counted local names; global/variable declare locals before name/value visits, while C8.5 monolithic Upvar visits its original namespace then each target before local declaration; raw member and SIMPLE_WORD locals gate selection; prefix pools/LVT survive Generic instruction rollback without executing partial bindings; Runtime retains the original namespace across targets and repeats its native lookup after each target; handler identity, live aliases and original name caches remain independent | none |
| original native compiler source selection | `rust/tcl-compiler/src/registry_invocation/native_compilation_source.rs`; `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/command_binding/compiler_inventory.rs` | `native_compilation_syntax`; `SourceCommandBindings::switch_compilation_at`; `SourceCommandBindings::namespace_binding_preparation_at` | exact full parser-word vector and original source channel; purpose-specific namespace and switch recipes retain original spans and compiler visits, while changed, truncated or missing source withdraws selection; internal implementation: `OriginalNativeCompilerInvocation`, `OriginalNativeCompilerPreparation`, `original_native_compilation`, `original_native_compiler_words` | none |
| original structured compiler preparation | `rust/tcl-compiler/src/command_binding/compiled_invocation.rs`; `rust/tcl-compiler/src/command_binding/compiler_inventory.rs`; `rust/tcl-registry/src/native_instruction_plan.rs` | `SourceCommandBindings::structured_compilation_at`; `SourceInvocationBinding::original_structured_compilation`; `SourceNativeStructuredPreparation::compilation_site`; `SourceNativeStructuredPreparation::recipe`; `SourceNativeStructuredPreparation::dependency`; `NativeInstructionPlan::expression_program`; `NativeInstructionPlan::body_error_context`; `NativeInstructionPlan::expression_error_context` | unanimous original allocation, complete unchanged word vector, source image/channel, compiler policy, namespace, table and actual registration dependency; ordered preparation survives Generic decline and exact Rejected failures remain distinct from unavailable evidence; original operand projections supply checked expression trees and native error-context annotations without generic-argv body-index recovery, normal-handler effects or reached execution authority | none |
| original namespace compiler preparation | `rust/tcl-compiler/src/command_binding/compiled_invocation.rs`; `rust/tcl-compiler/src/command_binding/compiler_inventory.rs` | `SourceNativeNamespaceBindingPreparation`; `namespace_binding_preparation_at` | original complete source vector, exact allocation and independent compiler dependency; retained declaration geometry includes declined prefixes and grants no live alias, store or executable operation | none |
| native return compilation and original option merge | `rust/tcl-registry/src/native_return_compilation.rs`; `rust/tcl-cmd-core/src/native_return_merge.rs` | `NativeReturnInstruction`; `NativeReturnOptionsOperand`; `NativeReturnExit`; `native_return_instruction`; `NativeReturnMergeObjects`; `MergedNativeReturnOptions`; `merge`; `merge_stack` | actual C8.5–9.1 compiler selection, original static or stack options, version-specific recursive Dict/List merge and retained private headers; control receipts remain separate from procedure-boundary settlement | none |
| original Error compiler instruction | `rust/tcl-registry/src/native_error_compilation.rs`; `rust/tcl-registry/src/native_instruction_plan.rs`; `rust/tcl-compiler/src/codegen/native_error.rs`; `runtime/rust/src/interp/native_body_artifact/native_error.rs` | `compile_native_error`; `native_error_instruction`; `NativeErrorInstruction`; `NativeErrorStep` | original parser vector and expansion-member geometry; selected C8.6/C9.0 List pairs or C9.1 Dictionary insertion, message then keyword/value operand order and immediate Error Return; C8.4/C8.5 decline remains generic; pure recipe supplies no installed compiler hook, source attestation, original header or Return-command grammar authority | `xtask-owner-resolution` |
| native switch compilation | `rust/tcl-registry/src/native_switch_compilation.rs` | `NativeSwitchInstruction`; `NativeSwitchArm`; `NativeSwitchMode`; `NativeSwitchMatch`; `native_switch_instruction` | actual selected C compiler, original subject and exact pattern/body geometry, ordered arm compilation and version-specific matching operations; omitted or masked bodies supply no preparation authority | none |
| native regexp cache and equivalent-glob conversion | `rust/tcl-syntax/src/native_regex.rs` | `NativeRegexpRecipe`; `NativeRegexpRange`; `NativeRegexpCache`; `equivalent_glob` | actual C8.4–9.1 RegExp flags and retained compiled artifact, counted TclReToGlob conversion and selected zero-range subject access; equivalent glob does not replace full capture-range matching | none |
| original object append and working transports | `rust/tcl-syntax/src/native_object_append.rs`; `rust/tcl-registry/src/native_object_append.rs`; `rust/tcl-cmd-core/src/native_append.rs`; `runtime/rust/src/value_ops.rs` | `NativeObjectAppendProtocol`; `NativeObjectCatProtocol`; `native_object_append_protocol`; `NativeAppendObjects`; `PreparedAppendValue`; `append_object`; `append_unicode_units`; `append_counted_bytes`; `NativeAppendVariable`; `append_operands`; `RuntimeAppendValue` | actual native string issuer, original source/receiver primary and resident storage, selected Unicode/binary append and real copy-on-write ownership; working transports keep borrowed originals separate from retained prepared objects, with private constructors and no external native-reference authority | none |
| original regexp compilation and matching | `rust/tcl-cmd-core/src/regex.rs`; `rust/tcl-syntax/src/native_tcl_utf.rs`; `rust/tcl-regex/src/jim.rs` | `NativeRegexSource`; `NativeRegexObjects`; `PreparedOriginalRegex`; `prepare_pattern_original`; `prepare_search_pattern_original`; `execute_pattern_original`; `compiled_match_original`; `NativeTclUtf`; `TclUtfUnit`; `jim::Flags::new`; `with_nocase`; `with_lineanchor`; `with_linestop`; `with_expanded`; `jim::Flags::key` | selected native engine, original pattern cache and subject primary, flags and counted character units; cache hits precede getters and reached output setters retain original target objects; Jim Flags builder retains actual 2/4/8/32 bits independently of C ARE flags and cannot grant compiled-program identity | none |
| native TclOO helper compilation and registration | `rust/tcl-registry/src/native_tcloo_compilation.rs`; `rust/tcl-registry/src/native_tcloo_registration.rs`; `rust/tcl-syntax/src/native_namespace_name.rs` | `NativeTclOoHelper`; `NativeTclOoInstruction`; `NativeTclOoObjectInfo`; `NativeTclOoObjectInfo::uses_execution_constant`; `select_original`; `instruction`; `definition_identity`; `compilation`; `NativeNamespaceObjectProducer::ObjectNamespace` | actual qualified helper or definition worker registration, original compiler words and selected C8.6–9.1 expansion grammar; method helpers independently require an issued frame, while ObjectInfo retains its original operand and selected object getter; Class uses the retained original class-name String, Namespace uses the actual object namespace incarnation, standalone IsObject uses interpreter execution constants and CreationId is C9.1; spelling and pure instruction geometry supply no object, frame or cache authority | none |
| original native unset compilation | `rust/tcl-registry/src/native_unset_compilation.rs`; `rust/tcl-compiler/src/codegen/native_unset.rs`; `runtime/rust/src/interp/native_body_artifact/native_unset.rs`; `runtime/rust/src/interp/native_variable_names/native_unset.rs` | `compile_native_unset`; `NativeUnsetInstruction`; `NativeUnsetVariable`; `NativeUnsetReceiver`; `NativeUnsetUnavailable` | actual C8.6+ hook and complete original parser projection; flag/known-word validation precedes preparation, each original root/index operand precedes its own unset, and complain semantics remain explicit; grouped words and literal expansion members retain different geometry; old-release no-hook and native declines stay Generic, unavailable geometry is a typed refusal; no live cell, runtime cache, trace order or successful destruction follows from selection | `xtask-owner-resolution` |
| native property lookup and invalidation | `rust/tcl-registry/src/native_property_lookup.rs` | `NativePropertyLookupProtocol`; `NativePropertyInvalidation`; `native_property_lookup_protocol` | actual C9.1 property declaration, original counted member and accessor names, TEMP_TABLE lookup and direction-specific mutation invalidation; property List ownership remains independent of variable declaration tables | none |
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
| runtime command guard ownership | `rust/tcl-runtime-api/src/guard.rs`; `rust/tcl-vm/src/interp.rs`; `runtime/rust/src/interp.rs` | `GuardManager`; `OwnedGuardManager` | shared checked epoch and token allocation; exact live command allocation captured at issuance; equal semantic identities on replacement allocations cannot revive issued guards; namespace and command mutations invalidate dependent snapshots while untouched allocations retain their attested identities | `xtask-owner-resolution` |
| retained module semantic ingress | `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/lowering/mod.rs` | `Module::resolved_profile`; `Module::resolved_registry`; `Module::number_syntax`; `Module::word_values` | immutable actual registry snapshot, resolved custom profile, exact lexer configuration and independent invocation policies; explicit legacy compatibility fallback only where ingress carriers are absent | none |
| retained analyser semantic ingress | `rust/tcl-compiler/src/analyser/input.rs`; `rust/tcl-compiler/src/analyser/state.rs`; `rust/tcl-compiler/src/analyser/types.rs`; `rust/tcl-registry/src/model/assembly.rs`; `rust/tcl-lsp-core/src/lib.rs` | `ResolvedAnalysisInput`; `analyser_profile`; `unit_profile`; `context_registry`; `lexer_config`; `with_resolved_input`; `resolved_analysis_input`; `resolved_profile`; `resolved_registry`; `retained_command_realm`; `with_command_store`; `profile_for_analysis`; `registry_for_analysis` | exact analyser/unit profiles, body grammar and immutable availability generation; full/chunked/incremental/isolated inputs and cache keys retain the same axes; original temporal realm compares semantic inventories rather than derived caches; editing metadata remains independent of native execution admission; name assistance only for legacy results without retained input; missing temporal realm remains unknown | none |
| attested native mathop handler selection | `rust/tcl-cmd-core/src/mathop.rs`; `rust/tcl-vm/src/cmd_mathop.rs`; `runtime/rust/src/cmd_mathop.rs` | `operation_for_handler_identity` | only an attested registered mathop implementation identity selects the operation; rename, alias, import and current argv remain independent; successful dispatch never reads argv0 for the operation, while the selected native arity presenter retains the original head | none |
| native procedure references and retirement | `rust/tcl-runtime-api/src/native_procedure_roles.rs`; `rust/tcl-vm/src/command/native_procedure.rs`; `runtime/rust/src/interp/native_procedure_resources.rs`; `runtime/rust/src/obj/native_lambda_expression.rs` | `NativeProcedureRoleLedger`; `NativeProcedureReference`; `NativeProcedureBinding` | actual command, frame and primary roles independent of allocation-only transports; final resource retirement; same-binding client-data replacement; borrowed queries acquire no role; commandless methods retain genuine Proc clientData and apply retains its original lambdaExpr primary plus absolute namespace object; a lambda-header duplicate acquires real Proc/namespace roles, while method-copy declarations create a new Proc and string-only body with the same original defaults; internal implementation: `NativeCallableProcedure`, `duplicate_lambda_role`, `duplicate_method`, `LambdaExpression` | none |
| actual empty procedure compilation | `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-runtime-api/src/native_compilation.rs`; `rust/tcl-bytecode/src/lib.rs`; `rust/tcl-registry/src/native_procedure.rs` | `NativeProcedureNoOpPlan`; `native_procedure_noop_plan`; `NativeProcedureHeaderPrerequisite`; `NativeCompilerSelectionSite`; `procedure_header_compilation` | actual callable/header and namespace allocation, original lookup word, body-object compilation state, chunk/before-argv validation and retained selection across substitutions | none |
| closed BPF conditional syntax | `rust/tcl-registry/src/bpf_op.rs`; `rust/tcl-registry/src/registry.rs`; `rust/tcl-compiler/src/lowering/mod.rs`; `rust/bpf-tcl-ir/src/source.rs` | `BpfOpKind`; `CommandRegistry::bpf_conditional_operands`; `source_nesting_limit` | independently authored static language descriptor and shared validated clause grammar; actual original literal words and affine source bytes, no expansion or dynamic source; existing construction bound and actual lexer configuration; derived bodies retain syntax spans and discard independent Tcl execution/admission receipts; unexpanded conditions are explicitly rejected | none |
| structured script operand extraction | `rust/tcl-compiler/src/codegen/mod.rs`; `rust/tcl-compiler/src/lowering/structured.rs`; `rust/tcl-compiler/src/optimiser/helpers/tokens.rs`; `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/optimiser/mod.rs` | `extract_body_text`; `Module::lexer_config`; `PassContext::lexer_config`; `CodegenCtx::lexer_config` | complete written-word ranges; exact retained ingress grammar; original static word value; verbatim evaluated script bytes and completion-preserving splice; internal implementation: `script_word_span` | none |
| word values (brace / list axes, braced-word recognition) | `rust/tcl-syntax/src/word_rules.rs` | `WordValueRules`; `collapse_braced_word`; `split_list`; `split_list_tolerant`; `split_word_names`; `whole_braced_word` | `LexerGrammar::brace_backslash_newline` and `list_parse` per dialect, carried together because a word-shaped list asks both; brace *balance* is release-invariant, so `whole_braced_word` takes no rules | none |
| numbers | `rust/tcl-syntax/src/number.rs`; `rust/tcl-dialect/src/expr_number.rs`; `rust/tcl-dialect/src/grammar.rs` | `parse`; `parse_whole_with`; `is_expr_number`; `scan_expr_number`; `scan_nan_payload`; `native_int32_low_bits`; `NumberSyntax` | `NumberSyntax` and expression-word grammar per release; low signed 32-bit projection follows an independently selected native getter, validated operand range and physical width, and grants no conversion success or width authority | `xtask-number-drift` |
| backslash escapes | `rust/tcl-lexer/src/substitution.rs`; `rust/tcl-syntax/src/backslash.rs`; `rust/tcl-dialect/src/grammar.rs` | `backslash_subst`; `backslash_subst_in`; `decode_bytes_in`; `backslash_fragment_in`; `BackslashFragment`; `EscapedInputUnit`; `native_source_escape_in`; `native_source_string_bytes_in`; `NativeSourceUnavailable`; `EscapeSyntax` | selected lexical grammar and independently selected native original-unit decoder/encoder; native source token extent and bounded value decoding, separately from Unicode presentation and subst processing | none |
| boolean words | `rust/tcl-syntax/src/boolean.rs` | `parse_boolean_word`; `truthiness_with` | fixed boolean vocabulary; number axis per release | none |
| quotes / braces / word spans | `rust/tcl-lexer/src/ranges.rs` | `close_quote_offset`; `word_closer_offset`; `word_span_at`; `braced_var_name_end` | `${...}` close rule per release (`BracedVarStyle`); tmsh brace mode per dialect | none |
| array-index source scan | `rust/tcl-lexer/src/ranges.rs`; `rust/tcl-dialect/src/grammar.rs` | `scan_array_index`; `ArrayIndexSyntax` | `LexerGrammar::array_index` per release | none |
| word substitution components | `rust/tcl-lexer/src/word_parts.rs` | `decompose`; `decompose_spanned`; `scan_var_ref`; `scan_expression_sugar`; `command_subst_close`; `quoted_word_close`; `SubstFlags`; `WordPart`; `SpannedPart`; `WordBody`; `VarRef`; `RawVarRef`; `MISSING_QUOTE`; `MISSING_CLOSE_BRACKET`; `MISSING_CLOSE_BRACE`; `MISSING_PAREN`; `EXTRA_AFTER_CLOSE_BRACE` | `LexerConfig` per emulated release (`${...}` close rule, array-index source mask, escape grammar); compiled-word vs source-word `$` spelling | none |
| runtime compatibility substitution callbacks | `runtime/rust/src/subst.rs` | `resolve_with`; `resolve_with_expression` | caller-supplied variable, command and independent expression callbacks; no retained invocation evidence; unresolved callbacks contribute no value; actual Runtime evaluation owns completion/refusal propagation | none |
| indices | `rust/tcl-cmd-core/src/index.rs` | `resolve_with`; `drill` | grammar-parameterised, inheriting the number axis | none |
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
| Jim trim byte cuts and physical result | `rust/tcl-syntax/src/raw_string.rs`; `rust/tcl-syntax/src/value.rs`; `rust/tcl-cmd-core/src/string.rs`; concrete VM/Runtime value adapters | `RawString::jim084_trim_plan`; `JimStringTrimPlan`; `ValueOps::jim_string_trim_result`; `string::trim` | exact Jim084 default set and numeric-unit membership; native backward start-byte scanning; the concrete owner performs string conversion, unique suffix reuse with retained count, shared copy/recount and fresh left-cut results; diagnostic argv shares the actual argument owner and copies values only at error capture; no compiler, purity, source or semantic type grant | none |
| command errors | `rust/tcl-cmd-core/src/error.rs` | `CmdError`; `wrong_args`; `bad_choice`; `with_error_details` | invariant | none |
| channel output configuration / encoding | `rust/tcl-platform/src/lib.rs`; `rust/tcl-cmd-core/src/channel.rs`; `rust/tcl-registry/src/commands/tcl/fconfigure_.rs` | `SystemEncoding`; `Host::system_encoding`; `ChannelConfig`; `StandardChannelConfigs`; `OpenAccess`; `resolve_open_access_mode`; `ChannelDirection`; `ChannelEncoding`; `EncodingProfile`; `OutputTranslation`; `resolve_fconfigure_option`; `config_list`; `config_value`; `set_config_value`; `encode_output`; `encode_output_bytes`; `EncodedOutput`; `EILSEQ_ERROR_CODE` | system encoding per host locale and interpreter tree; option availability per dialect profile; open-access validation and profile/binary defaults per Tcl release; mutable direction-specific state per channel | none |
| native original concat | `rust/tcl-dialect/src/version.rs`; `rust/tcl-syntax/src/value.rs`; `rust/tcl-cmd-core/src/list.rs`; `rust/tcl-cmd-core/src/list/native_concat.rs`; `rust/tcl-vm/src/value_ops/native_concat.rs`; `runtime/rust/src/value_ops/native_concat.rs` | `ConcatPolicy`; `ValueOps::concat_policy`; `NativeConcatListShape`; `NativeConcatFirstElement`; `ValueOps::native_concat_list_shape`; `ValueOps::native_concat_empty_list`; `ValueOps::native_concat_copy_list`; `ValueOps::native_concat_append_list`; `ValueOps::native_concat_first_bytes`; `ValueOps::release_native_concat_first`; `ValueOps::native_concat_string_bytes`; `ValueOps::native_concat_string_result`; `ValueOps::discard_native_concat_result`; `list::concat_selected` | actual release selects pure/canonical/abstract List eligibility, original header copy and borrowed member append independently of string bytes; C8.6+ later hash-head checks preserve original getter order and retire partial results; Runtime arithmetic-series Index children are fresh rc0 temporaries bounced after append; VM has no corresponding actual abstract primary; C8.5+ slow results own unknown-count String caches and allocated bytes, including empty results; successful string concat supplies no canonical List guarantee | none |
| Original script-object evaluation | `rust/tcl-registry/src/native_eval_object.rs`; `rust/tcl-vm/src/cmd_control.rs`; `runtime/rust/src/interp.rs`; `runtime/rust/src/list.rs` | `InvocationDialect::native_eval_object_protocol`; `InvocationDialect::eval_object_protocol`; `EvalObjectPurpose`; `LogicalEvalObjectProvider`; `NativeEvalObjectProtocol::dispatches_list` | actual engine and evaluation purpose select original List dispatch independently of script grammar and physical string materialisation; retained backing protects original element identity through callbacks without extra child references; C8.4 direct eval and concatenated namespace bodies require absent resident bytes; C8.4 single namespace bodies and uplevel use source evaluation; Jim requires absent resident bytes, while later C accepts its retained canonical List flag; source paths retain counted bytes, source location and the selected physical frame; internal implementation: `NativeListBacking`, `eval_original_body_framed` | none |
| Original-object return options / completion-code conversion | `rust/tcl-cmd-core/src/return_options.rs`; `rust/tcl-registry/src/native_return_options.rs` | `InvocationDialect::return_options_protocol`; `ReturnOptionsOps`; `prepare_return`; `prepare_option_pairs`; `prepare_prefix_error_options`; `parse_completion_code` | actual engine selects merge/sequential conversion, original list storage, primitive Int/Long probes and completion-code cache; internal dictionary ingestion is a distinct purpose; an explicit logical F5 provider grants simulation only | none |
| original native instruction-name error contexts | `rust/tcl-syntax/src/native_instruction_name.rs`; `rust/tcl-vm/src/value.rs`; `rust/tcl-vm/src/interp/native_error_stack.rs`; `runtime/rust/src/obj/native_instruction_name.rs`; `runtime/rust/src/interp/native_error_headers.rs` | `NativeInstructionName`; `NativeReturnInstructionName`; `NativeInstructionName::for_return`; `opcode`; `string_bytes` | actual C8.6/C9.0/C9.1 instname primary stores native opcode with absent bytes, NULL duplicate/free hooks and selected updater; interpreter innerContext is a real List retaining SAME original stack operands, independently of errorStack and snapshot roles; pure descriptor metadata grants no reached instruction or object authority | `xtask-owner-resolution` |
| Tcl completion options / structured error stacks | `rust/tcl-runtime-api/src/completion_options.rs`; `rust/tcl-runtime-api/src/error_stack.rs` | `completion_options::plan`; `completion_options::plan_with_origin`; `completion_options::retained_array_read_options`; `completion_options::ControlOptionPolicy`; `completion_options::ErrorOptions`; `completion_options::OptionValue`; `error_stack::ErrorStack`; `error_stack::validate_error_stack`; `error_stack::ErrorStackValueError` | standard option overlay follows completion code/level; control commands independently select inherited/fresh body options and forwarded/settled success options; TIP 348 `-errorstack` is available from Tcl 8.6; shifted contexts use the concrete runtime's frame count | none |
| native automatic Jim error capture | `rust/tcl-runtime-api/src/jim_error_stack.rs`; `rust/tcl-runtime-api/src/script_source_location.rs`; `rust/tcl-registry/src/invocation_words.rs`; `rust/tcl-test-support/src/automatic_errors.rs` | `NativeErrorStackProtocol`; `JimEvaluationFrame`; `JimScriptLocation`; `capture_jim_error_frames`; `JimErrorStack`; `JimErrorTrace`; `ScriptSourceLocation`; `error_stack_protocol`; `JIM_AUTOMATIC_ERROR_CASES` | actual selected Jim 0.84 protocol, separate procedure/evaluation nesting and selected variable frames; materialized argv and resolved command receipts captured before unwind; explicit traces remain raw; literal source locations are retained at value creation, never inferred from later execution; shared actual Jim corpus in VM and standalone Runtime | none |
| VM scheduled tailcall activation | `rust/tcl-vm/src/frame.rs`; `rust/tcl-vm/src/interp.rs`; `rust/tcl-vm/src/exec.rs`; `rust/tcl-registry/src/native_compilation.rs` | `NativeTailcallStack` | request belongs to the actual issuing frame; catch observes return code 2 while the request remains scheduled; empty tailcall cancels; only successful procedure settlement dispatches the replacement after leave callbacks; retained lookup namespace is independent of the caller variable frame; unresolved native compiler protocol requires provider evidence; internal implementation: `CallFrame::tailcall`, `TailcallReq`, `schedule_tailcall`, `dispatch_tailcall` | none |
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
| original literal-pool object effects | `rust/tcl-compiler/src/command_binding/literal_object_pool.rs` | `SourceStockLiteralObject` | actual fresh authored Tcl entry and original frozen unchanged Source operand ownership; source-root pool identity participates in semantic equality/hash and intersects on joins; unknown host/object effects or runtime-table installation permanently withdraw it; constructor epoch recovery cannot restore withdrawn provenance; closes native method effects only, with no List representation, list validity, completion, result or compiler admission; known text and actual command-table snapshots do not prove pooled intrep; internal implementation: `SourceOrdinaryLiteralPool`, `SourceOrdinaryLiteralObject::capture` | none |
| shared-object representation coercion | `rust/tcl-registry/src/representation.rs` | `RepresentationEffect::coercion_selection`; `RepresentationCoercionSelection`; `successful_ordinary_container_coercions`; `OrdinaryContainerCoercion` | selected native implementation, actual family/release, exact argv layout and independent ordinary-object representation proof; list-length, dictionary-size and foreach value slots expose normal-success conversion policies; C 9 abstract-list objects remain excluded; empty/unknown bytes can retain a May preservation branch; shared aliases join representations rather than gain a Must result shape; expression-source parsing and reached lazy operands can shimmer every alias without changing bytes or cells | none |
| typed binary packing / unpacking | `rust/tcl-cmd-core/src/binary.rs`; `rust/tcl-syntax/src/value.rs` | `FormatValueOps`; `format_values`; `integer_value`; `ScanValue`; `scan_values` | field/count grammar is shared; byte, integer, double and list coercions use the selected runtime; numeric fields retain native values without materialising a precision-dependent string | none |
| native fixed-function expression preparation | `rust/tcl-syntax/src/expr/jim_function_tree.rs`; `rust/tcl-registry/src/runtime_expr_validation.rs` | `NativeFunctionTree`; `prepare_jim_function_tree`; `requires_fixed_function_preparation`; `preparation_needs_actual_table`; `prepare_fixed_function_expression`; `PreparedExpressionWitness`; `ExpressionPreparationProof`; `prepare_expression_witness`; `NativeExpressionResultRecipe`; `numeric_result_recipe`; `native_expression_constant_pooling` | exact expression bytes and full parser axes; actual selected native parser and installed function identity/arity; entry validation includes runtime-lazy branches, prepared native tree and interpreter/table prerequisite separately from reached calls; warm caches revalidate actual table evidence | none |
| expression grammar / evaluation | `rust/tcl-syntax/src/expr/parser.rs`; `rust/tcl-syntax/src/expr/eval.rs`; `rust/tcl-registry/src/expr_surface.rs`; `rust/tcl-syntax/src/expr/checked.rs` | `parse_expr`; `ExprParseContext`; `CheckedExprParse`; `native_diagnostic_with_context`; `eval`; `RuntimeExprSurface` | complete lexer/operator/host/native parser axes; proved syntax failure distinct from unsupported recovery, limits or unknown syntax; `RuntimeExprSurface` per release | none |
| original expression terms and Jim preparation | `rust/tcl-lexer/src/expr_lexer.rs`; `rust/tcl-syntax/src/expr/native_objects.rs`; `rust/tcl-syntax/src/expr/jim_function_tree.rs`; `rust/tcl-registry/src/runtime_expr_validation.rs`; `rust/tcl-syntax/src/expr/eval.rs`; `rust/tcl-syntax/src/scalar_getter.rs`; `rust/tcl-syntax/src/expr/checked.rs` | `expression_term`; `expression_terms`; `ExprTerm`; `ExprTermKind`; `JimExpressionCacheAction`; `JimExpressionObjects`; `JimExpressionTermValue`; `jim_integer_expression_message`; `JimScriptPreparation`; `jim_expression_number_for_kind`; `prepare_fixed_function_expression_bytes_with_preparation`; `prepare_expr_bytes_checked_with_context`; `ExprOps::command_bytes_at`; `ExprOps::literal_bytes_at`; `ExprOps::string_bytes_at` | exact original token/body extents and raw native line deltas; asynchronous requests retain inclusive AST positions; Jim cache actions distinguish tokenization, completeness and tree outcomes independently of displayed errors; numeric term construction retains the original scanner-selected integer/double kind and is separate from existing-object getters; fixed-function preparation retains the same independent primary action; pure receipts do not issue interpreter filename identity; the concrete prepared holder owns all original terms by span, including lazy branches; safe Jim integer expressions consume original token bodies and preserve reached primary/cache effects | none |
| full Jim Script token ownership | `rust/tcl-lexer/src/lexer.rs`; `rust/tcl-lexer/src/jim_script_tokens.rs`; `rust/tcl-syntax/src/jim_script_layout.rs` | `jim_script_tokens`; `JimScriptTokens`; `JimScriptTokenKind`; `JimScriptLine`; `prepare_jim_script_layout`; `JimScriptLayout`; `JimScriptLayoutEntry` | original native source image, complete consumed spans and undecoded token bodies; separators and malformed tails precede command-cut filtering; raw native line deltas, parser completeness line and missing marker remain separate; the pure WORD/LINE projection retains original roster indices and native expansion counters rather than conventional word counts; genuine expansion prefixes are omitted from prepared backing; native escape decoding and original filename/interpreter object ownership belong to the concrete interpreter; translated document input cannot issue this native receipt | none |
| concrete Jim Script object ownership | `rust/tcl-syntax/src/jim_script_objects.rs`; `rust/tcl-vm/src/value_script.rs`; `runtime/rust/src/native_script.rs` | `JimScriptObjects`; `JimScriptObjectConstruction`; `JimScriptCommand` | full selected native roster and layout; original filename object, signed source baseline, actual interpreter empty/null-script identities and weak context origin; LINE has resident empty bytes and a ScriptLine primary, WORD has a pure integer primary; Script owns each real original token once; active execution retains the same backing across parent shimmer and restores it on guest completion; Jim duplicate retires Script to an untyped resident string; indexed-variable and specialized interpolation cache access remain checked capability boundaries; original native Script storage fixtures; internal implementation: `prepare_native_jim_script`, `native_script::prepare` | none |
| Jim interpreter retirement | `rust/tcl-runtime-api/src/jim_interpreter.rs`; `rust/tcl-vm/src/interp/jim_teardown.rs`; `runtime/rust/src/interp/jim_teardown.rs` | `JimInterpreterTeardown`; `teardown_jim_interpreter`; `JimInterpreterObjectRole`; `release_jim_call_frame_objects` | selected actual Jim ownership runs stored frame defers while command storage is live; normal and retiring frames release local commands, original procedure arguments, original procedure body, original namespace object and variables in that order; namespace indexes hold weak references after transferring their original object to a declaration or frame; interpreter retirement releases frames and commands before the interpreter's individual original object references, then invalidates the native procedure epoch and releases package storage; weak context associations and numeric access decline after retirement; storage is taken under short guards and released outside them; no association-data or trace-command object is installed in these backends; ordering and lifetime controls | none |
| selected normal operand cache costs | `rust/tcl-compiler/src/registry_invocation.rs`; `rust/tcl-compiler/src/shimmer/hints.rs`; `rust/tcl-compiler/src/shimmer/commit.rs`; `rust/tcl-compiler/src/shimmer/use_site.rs` | `stock_length_operand_cache` | selected actual Length protocol plus original captured native class; C86 numeric conversion versus C9 cache preservation reused by warning, replay and loop cost accounting; missing/semantic-only class or wrong operand cannot close cost; no value, completion or erasure grant; paired selected-Length cost/replay tests cover cache preservation, conversion and missing-class refusal; internal implementation: `operand_preserves_captured_cache` | none |
| reached native math-function folding | `rust/tcl-compiler/src/math_function_binding.rs`; `rust/tcl-compiler/src/tcl_expr_eval.rs`; `rust/tcl-syntax/src/expr/eval.rs`; `rust/tcl-compiler/src/sccp.rs`; `rust/tcl-compiler/src/command_binding.rs`; `rust/tcl-compiler/src/command_binding/math_occurrence.rs`; `rust/tcl-compiler/src/command_binding/origin_inventory.rs` | `ExpressionMathBindings`; `resolved_call_for_value_analysis`; `resolved_call`; `proves_intrinsic_for_erasure`; `proved_invocation`; `SccpResult`; `eval_tcl_expr_with_resolved_math_bindings`; `eval_tcl_expr_with_math_bindings`; `ExprOps::call_at`; `SourceImplicitMathInvocation`; `SourceMathInvocation`; `SourceMathInvocation::reached`; `SourceMathInvocation::conditional`; `SourceConditionalMathInvocation`; `math_invocations_for_script`; `implicit_math_invocations_for_script` | original occurrence carrier retains Reached or Conditional purpose; explicit reached() selection precedes actual identity, fold or erasure; conditional topology retains exact source/frame/namespace/arity without registration, entry, header or totality and keeps MathBindingPrerequisiteRequired; recorded previews remain Conditional; exact source origin and AST call offset after reached operands; mutable command identity, namespace lookup and observer absence; independently stamped fixed-table implementation and interpreter prerequisite; stamped global contract spelling is projected through registry `global_command_bare_name`, never a tail or arity match; only reached calls with independently closed operand callback effects consume fold dependencies; implementation-only analysis and pure intrinsic mathematical evaluation cannot establish erasure | none |
| reached expression operand coercion | `rust/tcl-compiler/src/tcl_expr_eval.rs` | `RetainedNativeOperandProof`; `NativeOperandProofs`; `eval_tcl_expr_with_proved_operands`; `analyse_tcl_expr_with_resolved_math_bindings`; `FoldEvaluation`; `NativeCoercionObligation`; `NativeExpressionResultDependency` | value constants do not prove native object identity or representation; fresh operation values are separate from retained objects; numeric, truth, numeric comparison, membership and final-result paths preserve required conversions; already native numeric reads require actual interpreter/object incarnation and exact dialect evidence; consumers retain and validate object evidence before replacing execution; skipped lazy operands never require conversion proofs; original-AST analysis values retain reached coercion obligations independently of erasure; Jim selected-object results retain original bytes/sharing obligations; synthetic literal substitution requires actual operand evidence; source/SSA currently carry live cell List/Unknown evidence, not native numeric object identity; execution folding declines required retained-object conversions until the owner supplies that evidence; internal implementation: `FoldValue` | none |
| grouped variable-store rewrite eligibility | `rust/tcl-compiler/src/ir.rs`; `rust/tcl-compiler/src/optimiser/store_packing.rs` | `StatementResultUse`; `Script::statement_result_use` | normal result discard is separate from abrupt completion, ordered physical outputs, failure text, observers and shared object representations; replacement command candidates require exact retained lookup and native engine point; synthetic boundaries do not consume a result; existing metadata does not close grouped-store schedule/object equivalence, so O119 remains hint-only; C84 foreach and set have different array-store error text; internal implementation: `assess_grouped_store_rewrite`, `StorePackingAssessment`, `StorePackingDecline` | none |
| purpose-aware scalar value facts | `rust/tcl-compiler/src/sccp.rs`; `rust/tcl-compiler/src/compilation_unit.rs`; `rust/tcl-compiler/src/static_loops.rs` | `ValueFactInputs`; `execution_value_facts`; `semantic_value_facts`; `SemanticValueFacts`; `SemanticValueProjection`; `FunctionUnit::semantic_values`; `ExpressionEvaluationPoint`; `ExpressionAnalysis`; `SemanticValueFacts::loop_analysis`; `StaticLoopAnalysis` | one shared phi/store/read/reachability solver; immutable analysis contents preserve original producer coercion, result-object, preparation and reached-handler obligations; existing SCCP values remain execution-erasure proof; Jim raw selected-result bytes remain separate from numeric interpretation; bounded loop analysis retains ordered expression and increment obligations; caller contents enter analysis through exact closed ordinary Incoming reads across every retained activation, without manufacturing a Symbol/version; diagnostic consumers use the typed immutable view; semantic constants never enter legacy erasure maps without producer proof; internal implementation: `DiagnosticValueFacts` | none |
| selected native constant command substitution | `rust/tcl-compiler/src/const_subst.rs` | `ConstSubstCtx::fold_cmd_subst_in`; `ResolvedConstSubst` | retained `InvocationDialect` selects grammar, native query and numeric release independently of catalogue profile and row order; recursive substitutions retain the same axes and exact command dependencies; binding trust remains a separate required contract; explicit unknown native identity declines rather than selecting a catalogue fallback; source rewrites additionally retain the original operand/evaluation effects; known bytes do not attest native object representation | none |
| reached native substitution template | `rust/tcl-registry/src/substitution.rs`; `rust/tcl-compiler/src/command_binding/substitution_template.rs`; `rust/tcl-registry/src/body_execution.rs` | `BodyExecutionSpec::SubstitutionTemplate`; `SubstitutionTemplateSelection`; `TemplateParseErrors`; `ResolvedInvocation::native_substitution_template` | actual frozen argv cardinality, retained available option table and prefix grammar; selected native family and exact template origin; ordered scalar reads preserve observer effects and partial C parse errors; result bytes carry no numeric or object representation proof; command substitutions, array-index substitutions and unmodelled Jim parser acceptance retain unknown effects; the original command remains executable | none |
| pattern operand source projection | `rust/tcl-registry/src/registry.rs`; `rust/tcl-compiler/src/regex_source.rs`; `rust/tcl-compiler/src/registry_invocation.rs` | `arg_indices_for_role_words`; `pattern_source_argument_index` | selected registry layout, original structured source words and actual retained `InvocationDialect`; dynamic option positions and expansions decline; fixed option values retain their occupied slots; normal-handler diagnostics map only exact written origins, never captured alias-prefix values into source spans; role selection supplies no opcode, store or rewrite licence; internal implementation: `source_pattern_index` | none |
| reached native numeric representation | `runtime/rust/src/obj.rs`; `rust/tcl-registry/src/mathfunc.rs` | `native_math_protocol` | actual reached conversion updates shared internal representation while preserving existing source bytes and lazy double context; catalogue spelling and a separately constructed numeric result cannot license conversion | none |
| expr math functions and the `rand` generator | `rust/tcl-syntax/src/expr/mathfunc.rs`; `rust/tcl-syntax/src/expr/rand.rs` | `NumValue`; `dispatch`; `dispatch_with_backend_int_width`; `try_dispatch_with_backend_int_width`; `try_dispatch_with_backend_protocol`; `NativeMathProtocol`; `jim_numeric_operand`; `IntWidth`; `MathFuncError`; `MathFuncSince`; `spec`; `all`; `added_in`; `seed_from_wide`; `next_draw`; `seed_and_draw` | `MathFuncSince` per release for the function surface; selected native math protocol separates C floating power/domain errors from Jim integer power/wrap/NaN; integer parsing and conversion remain selected native axes; dispatch policy does not prove handler presence; the Park-Miller generator is release-invariant | none |
| command / word segmentation | `rust/tcl-lexer/src/script.rs`; `rust/tcl-lexer/src/native_word.rs`; `rust/tcl-compiler/src/segmenter.rs`; `rust/tcl-compiler/src/parsing/syntax/build.rs`; `rust/tcl-compiler/src/parsing/syntax/segment.rs` | `group_commands`; `group_commands_bytes`; `CommandSpan`; `WordSpan`; `WordKind`; `NativeWord::from_group`; `SegmentedCommand`; `segment_commands` | `LexerConfig` per document dialect | `xtask-segmentation-drift` |
| nested command-substitution words and entered expression operands | `rust/tcl-compiler/src/word_subst.rs` | `nested_command_words`; `NestedWordsDecline`; `lifted_calls`; `lifted_exprs`; `LiftedCall` | original source instance and unchanged operand bytes; retained child handler, reached dispatch and logical expression grammar/number policy; an entered child supplies diagnostic/read evidence even if its parent never dispatches; missing, replaced, unentered or misaligned child evidence declines; no normal-parent or erasure authority; internal implementation: `entered_expression_evaluations`, `EnteredExpressionEvaluation` | none |
| parse-error cut | `rust/tcl-lexer/src/parse_cut.rs` | `first_parse_cut`; `first_parse_cut_in`; `ParseCut`; `EXTRA_AFTER_CLOSE_QUOTE` | `LexerConfig` per emulated release, inherited from the segmentation and word-component owners it walks | `xtask-segmentation-drift` |
| script completeness / reparse windows | `rust/tcl-lexer/src/structural_index.rs` | `script_is_complete`; `command_boundaries`; `reparse_window`; `BracketIndex`; `BraceIndex`; `ExprParenIndex`; `ParenBalance` | dialect-blind by construction: one byte scan of stock 8.6/9.x brace, quote, `${…}`-nesting, comment and terminator structure, so an editor keystroke costs no tokenise. The two grammar axes that really do move a command boundary — the F5 `BraceLineContinuation::Continues` next-line-`{` rule and the 8.x `BracedVarStyle::FirstClose` name rule — are pinned as measured divergences by `differential_boundaries`, never silently absorbed | `xtask-segmentation-drift` |
| iRules execution boundaries and placement | `rust/tcl-syntax/src/event_handler.rs`; `rust/tcl-registry/src/events.rs`; `rust/tcl-registry/src/registry.rs`; `rust/tcl-irules/src/when_block.rs`; `rust/tcl-irules/src/executable.rs` | `event_handlers`; `event_handlers_with_head_predicate`; `script_commands`; `top_level_when_handlers_with_registry_and_head_resolver`; `IrulesDeclarationArguments`; `IrulesExecutionContext`; `IrulesCommandPlacement`; `IrulesTopLevelDeclaration`; `IrulesTopLevelEffect`; `CommandRegistry::irules_command_placement`; `CommandRegistry::irules_event_declaration`; `CommandRegistry::irules_top_level_declaration`; `CommandRegistry::irules_top_level_declaration_shape`; `CommandRegistry::irules_top_level_effect`; `when_blocks`; `irules_executable_commands` | caller-supplied `LexerConfig`; offset-keyed resolved command identity; exact single-braced declaration body; declaration-only top level; known-event roots; call-reachable procedure bodies; stateful priority (`0..=1000`, default 500) | `xtask-gen-ai-diagnostics` |
| text similarity | `rust/tcl-compiler/src/text.rs` | `edit_distance`; `rank_suggestions`; `rank_containment_suggestions` | invariant | none |
| per-command knowledge | `rust/tcl-registry/src/spec.rs`; `rust/tcl-registry/src/hooks.rs`; `rust/tcl-registry/src/registry.rs` | `CommandSpec`; `SubCommand`; `CommandRegistry`; `RegistrySnapshot`; `RegistrySemanticKey`; `semantic_key`; `CommandRegistry::snapshot`; `RegistrySnapshot::registry`; `RegistrySnapshot::shared_registry` | per release/dialect; immutable full registry equality with authored spec order, profile snapshot, packages, and body grammar | `xtask-command-backing` |
| dialect / release facts | `rust/tcl-dialect/src/profile.rs`; `rust/tcl-dialect/src/grammar.rs`; `rust/tcl-dialect/src/version.rs`; `rust/tcl-dialect/data/reference-toolchains.tsv` | `DialectProfile`; `DialectProfileKey`; `DialectProfile::cache_key`; `DialectProfileKey::profile`; `DialectProfile::intern`; `LexerGrammar`; `TclVersion`; `TclVersion::patchlevel`; `TclVersion::reference_source_tag`; `TclVersion::has_error_stack`; `find` | the resolved dialect/release/build and complete lexer/runtime/availability/library snapshot; profile caches use value keys, bytecode retains its supplied strict handle; exact pinned reference patchlevel/source tag | `xtask-editor-extensions` |
| reference interpreter discovery and execution | `rust/tcl-test-support/src/lib.rs`; `rust/tcl-test-support/src/jim.rs`; `rust/tcl-test-support/jim-reference.txt` | `reference_patchlevel`; `reference_source_tag`; `locate_tclsh`; `available_tclshs`; `required_tclshs`; `run_script`; `run_script_file`; `run_script_fixture`; `locate_source_tree`; `Tclsh`; `TclSourceTree`; `ScriptOutcome`; `jim_reference`; `locate_jimsh`; `require_jimsh`; `JimReference`; `Jimsh`; `JimCapability` | exact C interpreter/source agreement; deliberately pinned current Jim revision and measured extension capabilities | none |
| executable conformance vectors | `rust/tcl-syntax/src/execution_conformance.rs` | `ExecutionDomain`; `ExecutionVector`; `vectors`; `RewriteCase`; `rewrite_cases`; `FilesystemExecutionCase`; `filesystem_cases` | C release expectations and independently measured Jim results/feature absences | none |
| interpreter platform bootstrap | `rust/tcl-platform/src/lib.rs` | `bootstrap::Values`; `bootstrap::Snapshot`; `bootstrap::snapshot`; `bootstrap::entries`; `bootstrap::HOST_ARRAYS`; `bootstrap::HOST_PATH_GLOBALS`; `bootstrap::safe_scrub_keys`; `bootstrap::SHARED_LIBRARY_EXTENSION` | key, selected-host snapshot, rebootstrap-clear, safe-scrub, and canonical Unix shared-library suffix invariant; runtime identity supplied per engine | none |
| shared plain types | `rust/tcl-core-types/src/lib.rs`; `rust/tcl-core-types/src/diag_code.rs` | `OoId`; `DiagCode` | interpreter-local OO identity and diagnostic codes are invariant across dialects | `xtask-diag-tables` |
| diagnostic suppression directives | `rust/tcl-compiler/src/analyser/utils.rs` | `parse_file_suppression`; `parse_noqa_marker`; `parse_noqa_line_suppressions_for_dialect`; `apply_preceding_noqa`; `line_suppressed`; `FILE_SUPPRESS_KEY` | directive shapes are release-invariant; the noqa pre-scan segments under the document dialect's `LexerConfig` | none |
| SslicTcl declaration model | `rust/tcl-sslictcl/src/model.rs` | `SslicModel`; `TlsFacts`; `Policy` | vocabulary version (`dsl::SUPPORTED_VOCABULARY`); no Tcl release axis — the document is never evaluated | none |
| SslicTcl document loading | `rust/tcl-sslictcl/src/dsl.rs`; `rust/tcl-sslictcl/src/vocabulary.rs` | `load_with_diagnostics`; `DslDiagnostic`; `DECLARATIONS` | vocabulary version; open/closed block rule per declaration | none |
| SslicTcl finding identity | `rust/tcl-sslictcl/src/policy.rs` | `evaluate_policy`; `PolicyFinding` | invariant `(check id, endpoint)` identity; the `grade` id is reserved | none |
| SslicTcl embedded source data | `rust/tcl-sslictcl/src/trust.rs` | `embedded_dataset` | pinned upstream revisions, recorded with hashes and licences in `data/provenance.json` | `xtask-sslictcl-data` |
| SslicTcl declaration surface | `rust/tcl-registry/src/commands/sslictcl/mod.rs`; `rust/tcl-registry/src/definer.rs` | `sslictcl_command_specs`; `SSLICTCL_GRAMMARS` | the `sslictcl` authoring surface (`SpecSurface::SSLICTCL`); Tcl 9.0 core underneath | none |
| SslicTcl editor projection | `rust/tcl-lsp-core/src/sslictcl_diagnostics.rs`; `rust/tcl-lsp-core/src/declaration_outline.rs` | `applies_to`; `diagnostics`; `SUPERSEDED_ANALYSER_CODES`; `supersede_analyser_diagnostics`; `is_declaration_document`; `declarations` | resolved authoring surface (the `sslictcl` package) per document | none |
<!-- end-owner-resolution-manifest -->

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
- `key_holder_and_tail` and `key_segments` operate on analytical constructed
  text keys. They cannot reconstruct every native path from a display: `[a:, b]`
  and `[a, :b]` both render as `::a:::b`. Preserve original `ByteNamespacePath`
  boundaries and actual interpreter/namespace incarnation independently. Written
  source operands still use the selected written-name grammar; rendering a native
  slot supplies neither those operands nor a globally callable spelling receipt.
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
    about. The rules differ only on names containing `{`, `}` or `\`, and
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

### `tcl-compiler` — nested command-substitution words

- `word_subst::nested_command_words` is the one recovery of the words
  inside a `[cmd …]`. The word snapshot keeps a substitution as a
  single opaque spelling, so the words are recovered by running the
  canonical segmenter over its recorded lexical extent — never a
  bespoke bracket scan — and anything but exactly one complete command
  declines. Consumers: the shimmer and SSA lifts
  (`word_subst::lifted_calls` / `lifted_exprs`), the native lowering's
  `nested_words`, and the WASM leaf-invoke planner's. Analysis and the
  two AOT tiers therefore cannot disagree about what a substitution
  runs, which they could while each kept its own copy.
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
  that renders a finding knows which line it lands on — so every consumer asks
  through `line_suppressed`: the language server's analyser, compiler-check,
  optimiser, source-style, XC and SslicTcl lifts, the `diag` / `lint` /
  `validate` CLI verbs, and the W305 producer that the non-Tcl F5 adapters
  share. That is what makes a directive mean the same thing in the editor and
  on the command line, as
  [`docs/kcs/kcs-howto-suppress-diagnostics.md`](../../kcs/kcs-howto-suppress-diagnostics.md)
  promises. A surface that reimplements the check drifts on the wildcard entry
  or the file-level bucket, which silences a different set of findings than
  the directive names.

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
and following statements retain invalidation. Type inference widens registry-clobbered versions independently of binding
and taint lineage. An unknown prior container contributes unknown elements
to later updates, preventing a stale class from surviving a handler. A narrow
straight-line proof retains executable type provenance for empty source-class
factories: registry manufacturer descriptors identify the empty declarations
and argument-free factories; any opaque invocation, binding transition,
nonempty class body or control edge withdraws the proof. The homogeneous and mixed-class collection assertions check this precision.

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

Coroutine native compilation follows the pinned `TclCompileYieldCmd` and
`TclCompileYieldToCmd` implementations. Yield accepts zero/one source
operands. Relay compilation requires a target in C8.6/9.0; C9.1 compiles
the empty relay and expanded source operands, retaining its runtime error
for a missing target. The emitted relay captures current namespace before
arguments and constructs the native namespace-prefixed command list.
Original handler identity and before-argument compiler guards are retained
in statement, substitution and protected body positions. SpecTcl's typed
inline-hook catalogue includes both hooks; native compiler contracts retain
the documented explicit authoring gap.

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
have no additional private path. Mutable ensembles, unknown registrations and
unsupported recipes retain the provider obligation. Nested compiler hooks must
also close before the whole script can be admitted. A registered plan requires
an emitter for that exact operation; it grants no normal-handler effect, purity,
value, current cell or runtime object-class facts. Command/namespace mutation
withdraws cache reuse and validates the retained prerequisites at their declared
boundary before argument evaluation.

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

`NativeListItems` is the VM's closed whole-storage carrier. A native header owns
one backing reference; a lifetime view retains the table without acquiring a
native header or copying child object handles. Inspectors, renderers, original
callback members, and direct List invocation use lifetime views. A generated
argument vector uses `invocation_view`; that diagnostic container is not an
actual native List backing. `Value::invocation_list_view` retains the original
argument storage, while `capture_invocation_list` acquires real member
references when the native consumer captures its arguments.

`NativeListBacking` crosses the engine boundary with an engine-private storage
receipt, actual C release, interpreter scope, whole-storage shape, and the same
live canonical flag. These are separate from resident string allocation and
transport representation authority. The recovering engine validates its private
receipt and the header's current attachment before applying a retained backing.
Public member data and canonical booleans describe fresh List construction and
cannot recreate original storage authority. A foreign scope, forged receipt,
retired attachment, or diagnostic argv view returns a typed host refusal.

The C9 shim duplicates an actual original header through
`OriginalObject::duplicate_native_header`. Its duplicate shares the member table
and flag until a reached copy-on-write mutation. Refresh preserves that same
table when the original attachment remains current. Native updater flag changes
use `NativeStringProtocol`; transport lifetime pins do not make backing or child
objects appear shared. The shim's C9 ABI accepts only its actual C9 issuer.

Use `list_backing` and `list_backing_matches` for retained storage, and the
explicit fresh `OriginalObjectResult::List` for newly constructed member data.
Do not infer ownership from `Rc` counts or rendered List text. Tests cover native
header duplication, lifetime-only inspection, repeated member slots, actual
copy-on-write child references, live flags, foreign scope and forged receipt
rejection, and retired original attachment rejection.


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
