// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Every editable field of `SubCommand`, in `SubCommand::DEFAULT` order.

use super::{
    ADVANCED, ARGS, AVAILABILITY, BEHAVIOUR, DOCS, EFFECTS, FieldKind, FieldSchema, HOOKS,
    IDENTITY, OPTS, SUBS, TAINT, TYPES, f,
};

/// Every editable field of `SubCommand`, in `SubCommand::DEFAULT` order.
pub const SUBCOMMAND_FIELDS: &[FieldSchema] = &[
    f(
        "name",
        "Subcommand name",
        IDENTITY,
        FieldKind::Text,
        "Subcommand word — `create`, `length`, `for`.",
    ),
    f(
        "traits",
        "Traits",
        BEHAVIOUR,
        FieldKind::FlagSet {
            catalogue: "traits",
            optional: false,
        },
        "Subcommand-shaped trait bits; they compose with the command's own.",
    ),
    f(
        "arity",
        "Arity",
        ARGS,
        FieldKind::Arity,
        "Argument-count constraint, counted after the subcommand word.",
    ),
    f(
        "arity_windows",
        "Arity windows",
        ARGS,
        FieldKind::RustExpr {
            hint: "ARITY_WINDOWS",
        },
        "Per-release signature shapes, for a command whose argument count changed across its owning package's releases. Empty unless it did; the plain arity is the fallback whenever no window covers the resolved floor.",
    ),
    f(
        "detail",
        "Detail",
        DOCS,
        FieldKind::Text,
        "Short description for the completion list.",
    ),
    f(
        "synopsis",
        "Synopsis",
        DOCS,
        FieldKind::Text,
        "Invocation synopsis.",
    ),
    f(
        "hover",
        "Hover documentation",
        DOCS,
        FieldKind::Hover,
        "Summary, synopsis lines, prose, source, examples, and return value.",
    ),
    f(
        "arg_roles",
        "Argument roles",
        ARGS,
        FieldKind::RoleMap,
        "Static role per index, counted after the subcommand word.",
    ),
    f(
        "arg_role_resolver",
        "Argument-role resolver",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(my_resolver)",
        },
        "Callback assigning roles from the actual argument list.",
    ),
    f(
        "arg_role_count_resolver",
        "Cardinality argument-role resolver",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(my_count_resolver)",
        },
        "Callback assigning roles from exact evaluated argument count without reading values. Unknown expansion cardinality keeps roles incomplete.",
    ),
    f(
        "arg_role_layout_resolver",
        "Structured argument-layout resolver",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(my_layout_resolver)",
        },
        "Callback assigning roles from structured argv and the selected available option table; unknown operand values remain unknown. Mutually exclusive with value/count callbacks.",
    ),
    f(
        "arg_role_resolver_roles",
        "Resolver role capabilities",
        ARGS,
        FieldKind::EnumList {
            catalogue: "argRole",
        },
        "Closed set of roles the dynamic resolver can emit when an invocation cannot be resolved precisely.",
    ),
    f(
        "clause_grammar",
        "Clause grammar",
        ARGS,
        FieldKind::ClauseGrammar,
        "The word grammar of a clause chain (`if`/`elseif`/`else`, `try`/`on`/`trap`/`finally`, a loop's fixtures and body): where each keyword, condition, script and binder sits, and when each body runs. Its walk answers the argument roles and the chain's structural defect.",
    ),
    f(
        "arg_presentation",
        "Argument presentation",
        ARGS,
        FieldKind::PresentationMap,
        "Formatter layout override per index, counted after the subcommand word.",
    ),
    f(
        "repeated_args",
        "Repeated argument layouts",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "&[RepeatedArgLayout::strided(ArgRole::VarWrite, 1, 2)]",
        },
        "Roles that recur at a fixed stride over the tail, counted after the subcommand word.",
    ),
    f(
        "command_prefixes",
        "Command-prefix positions",
        ARGS,
        FieldKind::PrefixMap,
        "Callback command-prefix indices after the subcommand word, with appended arity.",
    ),
    f(
        "command_prefix_resolver",
        "Command-prefix resolver",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(my_prefix_resolver)",
        },
        "Callback locating command-prefix positions after the subcommand word.",
    ),
    f(
        "script_lookup_scope",
        "Executable lookup frame",
        BEHAVIOUR,
        FieldKind::Enum {
            catalogue: "scriptLookupScope",
            optional: true,
        },
        "Invoking, global or triggering frame at executable entry; independent of timing, operand ownership and command table proof.",
    ),
    f(
        "script_timing_resolver",
        "Script-timing resolver",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(my_timing_resolver)",
        },
        "Callback assigning timing to executable positions after the subcommand word.",
    ),
    f(
        "callback_taint_inputs",
        "Callback external inputs",
        TAINT,
        FieldKind::CallbackTaintMap,
        "User-controlled substitutions injected into deferred positional callback arguments.",
    ),
    f(
        "return_type",
        "Return type",
        TYPES,
        FieldKind::Enum {
            catalogue: "tclType",
            optional: true,
        },
        "The intrep of the value the subcommand yields.",
    ),
    f(
        "var_write_typing",
        "Variable-write typing",
        TYPES,
        FieldKind::RustExpr {
            hint: "VarWriteTyping::Destructured",
        },
        "Overrides the command's variable-write typing when this subcommand matches.",
    ),
    f(
        "return_elements",
        "Return elements",
        TYPES,
        FieldKind::RustExpr {
            hint: "Some(ReturnElements::ElementOf { container: 0 })",
        },
        "Result-to-element-structure fact for this subcommand.",
    ),
    f(
        "var_elements_effect",
        "Variable element effect",
        TYPES,
        FieldKind::RustExpr {
            hint: "Some(VarElementsEffect::SetsDictValue)",
        },
        "In-place element evolution of the written variable for this subcommand.",
    ),
    f(
        "representation_effect",
        "Representation effect",
        TYPES,
        FieldKind::RustExpr {
            hint: "Some(RepresentationEffect::copy_on_write_container(0, 2))",
        },
        "Subcommand-specific Tcl representation or shared-object storage effect.",
    ),
    f(
        "arg_types",
        "Argument type hints",
        TYPES,
        FieldKind::ArgTypeMap,
        "Expected intrep and shimmer risk per argument index.",
    ),
    f(
        "mutator",
        "Mutator",
        BEHAVIOUR,
        FieldKind::Bool,
        "Mutates state.",
    ),
    f(
        "const_fold",
        "Constant folder",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(const_fold::fold_string_length)",
        },
        "Compile-time folder for this subcommand.",
    ),
    f(
        "const_fold_versioned",
        "Versioned constant folder",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(const_fold::fold_string_is)",
        },
        "Tcl-version-aware folder; takes priority over the plain folder.",
    ),
    f(
        "lowering_hook",
        "Lowering hook",
        HOOKS,
        FieldKind::Enum {
            catalogue: "loweringHook",
            optional: true,
        },
        "Lowering specialisation for this subcommand.",
    ),
    f(
        "codegen_hook",
        "Bytecode codegen hook",
        HOOKS,
        FieldKind::Enum {
            catalogue: "codegenHook",
            optional: true,
        },
        "TclVM bytecode emitter for this subcommand.",
    ),
    f(
        "codegen_hook_windows",
        "Bytecode codegen hook windows",
        HOOKS,
        FieldKind::RustExpr {
            hint: "CODEGEN_HOOK_WINDOWS",
        },
        "Per-release bytecode emitters for this subcommand. A subcommand whose windows state nothing at the primary release inherits the command's hook; one whose windows the point does not settle dispatches plain.",
    ),
    f(
        "inline_codegen_hook",
        "Inline codegen hook",
        HOOKS,
        FieldKind::Enum {
            catalogue: "inlineCodegenHook",
            optional: true,
        },
        "Value-position emitter, overriding the command's when this subcommand matches.",
    ),
    f(
        "inline_codegen_hook_windows",
        "Inline codegen hook windows",
        HOOKS,
        FieldKind::RustExpr {
            hint: "INLINE_CODEGEN_HOOK_WINDOWS",
        },
        "Per-release value-position emitters for this subcommand; the contract of its bytecode codegen hook windows.",
    ),
    f(
        "semantics",
        "Value-transfer declaration",
        HOOKS,
        FieldKind::RustExpr {
            hint: "SemanticsDeclaration::Declared(&value_transfer::builtins::STRING_LENGTH)",
        },
        "The value-transfer specialisation declared for this subcommand, overriding the command's, or an explicit abstention that stops the command's from applying here. Its route is edited below.",
    ),
    f(
        "analyser_hook",
        "Analyser hook",
        HOOKS,
        FieldKind::Enum {
            catalogue: "analyserHook",
            optional: true,
        },
        "Analyser handler family, overriding the command's for this subcommand.",
    ),
    f(
        "command_table_effect",
        "Command-table effect",
        EFFECTS,
        FieldKind::Enum {
            catalogue: "commandTableEffect",
            optional: true,
        },
        "Command-table mutation for this subcommand (`interp alias`).",
    ),
    f(
        "options",
        "Options",
        OPTS,
        FieldKind::Options,
        "Option flags declared on this subcommand.",
    ),
    f(
        "option_relations",
        "Option relations",
        OPTS,
        FieldKind::RustExpr {
            hint: "&[OptionRelation { kind: RelationKind::MutuallyExclusive, subject: None, \
                   terms: &[OptionTerm::Option(\"-a\"), OptionTerm::Option(\"-b\")], \
                   surface: None, lifecycle: Lifecycle::UNSPECIFIED, message: None }]",
        },
        "Subcommand-specific option relations (E-R14), evaluated by the same \
         native checker as the command-level ones.",
    ),
    f(
        "option_effect_families",
        "Option-effect families",
        OPTS,
        FieldKind::RustExpr {
            hint: "&[OptionEffectFamily { name: \"match\", base: FamilyBase::AllOn, \
                   combine: FamilyCombine::Accumulate, surface: None }]",
        },
        "The families this subcommand's option effects cite.",
    ),
    f(
        "option_placement",
        "Option placement",
        OPTS,
        FieldKind::Enum {
            catalogue: "optionPlacement",
            optional: false,
        },
        "Where this invocation's declared options may appear: a leading run that \
         stops at the first non-option word (every core Tcl command), or anywhere \
         between the positional words up to an explicit `--` (`http::geturl`).",
    ),
    f(
        "min_abbrev",
        "Minimum abbreviation",
        OPTS,
        FieldKind::OptIndex,
        "Documented minimum abbreviation length for this subcommand's name, \
         when longer than uniqueness alone requires. Unset = uniqueness only.",
    ),
    f(
        "option_prefix_words",
        "Positional prefix words",
        OPTS,
        FieldKind::Count,
        "Fixed positional words before the selected invocation's option run.",
    ),
    f(
        "reserved_trailing_words",
        "Reserved trailing words",
        OPTS,
        FieldKind::Count,
        "Trailing positional words excluded from this subcommand's option scan.",
    ),
    f(
        "prefix_matching",
        "Prefix matching",
        OPTS,
        FieldKind::Enum {
            catalogue: "prefixMatching",
            optional: false,
        },
        "Whether this subcommand's own keyword tables accept unique-prefix \
         abbreviations or only exact spellings.",
    ),
    f(
        "arg_values",
        "Argument values",
        OPTS,
        FieldKind::ArgValueMap,
        "Enumerable values per index after the subcommand word.",
    ),
    f(
        "versioned_arg_values",
        "Versioned argument values",
        AVAILABILITY,
        FieldKind::RustExpr {
            hint: "VERSIONED_ARG_VALUES",
        },
        "Package-version gates for literal positional argument values.",
    ),
    f(
        "subcommand_forms",
        "Invocation refinements",
        ADVANCED,
        FieldKind::Refinements,
        "Per-form overlays matched after the subcommand word by arity and longest-static literal prefix.",
    ),
    f(
        "semantic_operation",
        "Semantic operation",
        HOOKS,
        FieldKind::SemanticOperation,
        "Target-neutral operation identity overriding the parent command.",
    ),
    f(
        "semantic_operation_windows",
        "Semantic operation windows",
        HOOKS,
        FieldKind::RustExpr {
            hint: "SEMANTIC_OPERATION_WINDOWS",
        },
        "Per-release semantic operations for this subcommand; the contract of its bytecode codegen hook windows.",
    ),
    f(
        "completion",
        "Completion contract",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Knowledge::Partial(CompletionDescriptor::exact(COMPLETION_CODES))",
        },
        "Subcommand-specific Tcl completion and payload obligations.",
    ),
    f(
        "completion_knowledge",
        "Completion completeness",
        EFFECTS,
        FieldKind::Knowledge,
        "How completely this subcommand's completion codes are known.",
    ),
    f(
        "surface",
        "Surface",
        AVAILABILITY,
        FieldKind::FlagSet {
            catalogue: "dialects",
            optional: true,
        },
        "Where the subcommand exists. Unset inherits the parent command's surface.",
    ),
    f(
        "introduced_version",
        "Introduced in",
        AVAILABILITY,
        FieldKind::OptText,
        "Dotted version of the owning package that introduced the subcommand.",
    ),
    f(
        "deprecated_version",
        "Deprecated in",
        AVAILABILITY,
        FieldKind::OptText,
        "First package version where the subcommand still exists but should warn.",
    ),
    f(
        "retired_version",
        "Retired in",
        AVAILABILITY,
        FieldKind::OptText,
        "First package version without the subcommand (exclusive upper bound).",
    ),
    f(
        "deprecation_fix",
        "Deprecation quick-fix hook",
        AVAILABILITY,
        FieldKind::RustExpr {
            hint: "Some(DeprecationFixHook::ReplaceMatchedWord { replacement: \"new\", description: \"Use new\", safety: DeprecationFixSafety::SemanticsEquivalent })",
        },
        "Registry-owned edit plan or contextual callback for replacing deprecated syntax.",
    ),
    f(
        "safe_on_uninit",
        "Safe on uninitialised",
        AVAILABILITY,
        FieldKind::FlagSet {
            catalogue: "dialects",
            optional: true,
        },
        "Dialects where the subcommand safely initialises an uninitialised variable.",
    ),
    f(
        "loop_list_header",
        "Loop-list header",
        BEHAVIOUR,
        FieldKind::Bool,
        "A CFG header whose arguments are list expressions (`foreach` / `lmap`).",
    ),
    f(
        "creates_scope_alias",
        "Creates scope alias",
        BEHAVIOUR,
        FieldKind::Bool,
        "Creates an upvar-like binding.",
    ),
    f(
        "inferred_storage_type",
        "Inferred storage type",
        TYPES,
        FieldKind::Enum {
            catalogue: "storageType",
            optional: true,
        },
        "The container kind the target variable is inferred to hold.",
    ),
    f(
        "body_kind",
        "Body kind",
        BEHAVIOUR,
        FieldKind::Enum {
            catalogue: "bodyKind",
            optional: false,
        },
        "Whether body arguments run in the caller's frame or a separate context.",
    ),
    f(
        "variable_receivers",
        "Variable receiver purposes",
        BEHAVIOUR,
        FieldKind::VariableReceivers,
        "Argv variable-name receiver positions and their combined or trace-subject naming purpose. Unset inherits; declared empty withdraws. This metadata grants no successful access, trace installation, compiler preparation or whole-array root-only interpretation.",
    ),
    f(
        "successful_handler",
        "Native normal handler effects",
        BEHAVIOUR,
        FieldKind::RustExpr {
            hint: "Some(tcl_registry::native_compilation::SuccessfulHandlerSpec::VariableOperands)",
        },
        "Audited normal transfer after actual native handler identity converges; does not license compiler selection or body execution. Excluded from pack-authored strong analysis.",
    ),
    f(
        "native_compilation",
        "Native compiler grammar",
        BEHAVIOUR,
        FieldKind::RustExpr {
            hint: "Some(NativeCompilationSpec { .. })",
        },
        "Native compiler-hook syntax and script entry protocol; requires actual source compilation and implementation proofs.",
    ),
    f(
        "body_execution",
        "Body execution contract",
        BEHAVIOUR,
        FieldKind::RustExpr {
            hint: "Some(BodyExecutionSpec::CapturedLifecycle(...))",
        },
        "Versioned evaluated-body grammar and completion contract; expansion also requires proved implementation and hook dependencies.",
    ),
    f(
        "body_interpreter",
        "Body interpreter",
        BEHAVIOUR,
        FieldKind::BodyInterpreter,
        "Whether body arguments run in the current interpreter or one selected by an invocation argument.",
    ),
    f(
        "byte_array_effect",
        "Byte-array effect",
        BEHAVIOUR,
        FieldKind::Enum {
            catalogue: "byteArrayEffect",
            optional: false,
        },
        "How the subcommand transforms a byte-array operand (S110).",
    ),
    f(
        "closed_value_args",
        "Closed value arguments",
        OPTS,
        FieldKind::IndexList,
        "Indices after the subcommand word whose values are an exhaustive set (W127).",
    ),
    f(
        "arg_values_accept_prefix",
        "Values accept a prefix",
        OPTS,
        FieldKind::Bool,
        "Whether a closed value is accepted as a unique prefix rather than an exact match.",
    ),
    f(
        "body_arg_implicit_args",
        "Body implicit arguments",
        ARGS,
        FieldKind::Count,
        "Runtime-supplied positional arguments the body's first command receives.",
    ),
    f(
        "taint_transform",
        "Taint transform colours",
        TAINT,
        FieldKind::FlagSet {
            catalogue: "taintColour",
            optional: true,
        },
        "Colour bits this subcommand adds to a tainted value it returns.",
    ),
    f(
        "taint_transform_when",
        "Transform condition",
        TAINT,
        FieldKind::Enum {
            catalogue: "taintTransformCondition",
            optional: true,
        },
        "Argument-shape proof a call must pass before the transform colour is claimed.",
    ),
    f(
        "taint_double_encode_colour",
        "Double-encode colour",
        TAINT,
        FieldKind::FlagSet {
            catalogue: "taintColour",
            optional: true,
        },
        "Input colour whose presence means this subcommand would double-encode (T106).",
    ),
    f(
        "taint_output_sink",
        "Output-sink code",
        TAINT,
        FieldKind::OptText,
        "Diagnostic code for a subcommand-shaped XSS or header-injection sink.",
    ),
    f(
        "channel_configuration",
        "Channel configuration",
        BEHAVIOUR,
        FieldKind::Enum {
            catalogue: "channelConfiguration",
            optional: true,
        },
        "Conditional original encoding/translation option relationship; no channel state or runtime acceptance.",
    ),
    f(
        "credential_arg",
        "Credential argument",
        TAINT,
        FieldKind::OptIndex,
        "Credential-value index, counted WITH the subcommand word at 0 \
         (unlike every other subcommand index field) — `HTTP::header insert \
         name value` declares 2.",
    ),
    f(
        "sensitive_headers",
        "Sensitive headers",
        TAINT,
        FieldKind::TextList,
        "HTTP header names whose values are secrets.",
    ),
    f(
        "pattern_type",
        "Pattern type",
        OPTS,
        FieldKind::Enum {
            catalogue: "patternType",
            optional: true,
        },
        "Pattern-language override, taking priority over the command's.",
    ),
    f(
        "format_string_type",
        "Format-string type",
        OPTS,
        FieldKind::Enum {
            catalogue: "formatType",
            optional: true,
        },
        "Format-string override, taking priority over the command's.",
    ),
    f(
        "side_effects",
        "Side effects",
        EFFECTS,
        FieldKind::SideEffects,
        "Structured side-effect declarations for this subcommand.",
    ),
    f(
        "side_effects_knowledge",
        "Side-effect completeness",
        EFFECTS,
        FieldKind::Knowledge,
        "How completely this subcommand's side effects are known.",
    ),
    f(
        "world_effects",
        "World effects",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Knowledge::Partial(WorldEffectDescriptor::EMPTY)",
        },
        "Subcommand-specific mutable-world footprint.",
    ),
    f(
        "world_effects_knowledge",
        "World-effect completeness",
        EFFECTS,
        FieldKind::Knowledge,
        "How completely this subcommand's world effects are known.",
    ),
    f(
        "state_transitions",
        "State transitions",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Knowledge::Partial(StateTransitionDescriptor::EMPTY)",
        },
        "Subcommand-specific Tcl state transitions.",
    ),
    f(
        "state_transitions_knowledge",
        "State-transition completeness",
        EFFECTS,
        FieldKind::Knowledge,
        "How completely the state transitions are known: unknown, partial (a lower bound), or complete for the release windows evidenced from the implementation.",
    ),
    f(
        "dispatch_dependencies",
        "Dispatch dependencies",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Some(DispatchDependencyDescriptor::replace(DispatchDependencies::BASE))",
        },
        "Subcommand-specific live-dispatch stability requirements.",
    ),
    f(
        "native_result",
        "Native result contract",
        BEHAVIOUR,
        FieldKind::RustExpr {
            hint: "Some(NativeResultContract::...)",
        },
        "Native selected-cell/argument result dependency; requires actual implementation, observer and completion proofs.",
    ),
    f(
        "result_stability",
        "Result stability",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Knowledge::Partial(ResultStability::ReferentiallyTransparent)",
        },
        "Subcommand-specific result stability declaration.",
    ),
    f(
        "result_stability_knowledge",
        "Result-stability completeness",
        EFFECTS,
        FieldKind::Knowledge,
        "How completely this subcommand's result stability is known.",
    ),
    f(
        "constraints",
        "Option-relation escape hatch",
        HOOKS,
        FieldKind::RustExpr {
            hint: "Some(check_option_constraints)",
        },
        "The rare E-R14 escape hatch, consulted only when every declarative \
         option relation reported nothing. Prefer an `option_requires` / \
         `option_forbids` / `option_requires_one_of` row: those are checked \
         natively with no VM entry.",
    ),
    f(
        "source_index_bounds",
        "Source index bounds",
        BEHAVIOUR,
        FieldKind::Enum {
            catalogue: "sourceIndexBounds",
            optional: true,
        },
        "Conditional original index/container relationship; no native handler or current result.",
    ),
    f(
        "literal_argument_validator",
        "Literal argument validator",
        HOOKS,
        FieldKind::RustExpr {
            hint: "Some(validate_literal_arguments)",
        },
        "Subcommand-specific registry callback for related literal arguments.",
    ),
    f(
        "destructive",
        "Destructive",
        BEHAVIOUR,
        FieldKind::Bool,
        "An irreversible operation (`file delete`).",
    ),
    f(
        "returns_path",
        "Returns a path",
        BEHAVIOUR,
        FieldKind::Bool,
        "Returns a filesystem path.",
    ),
    f(
        "source_path_operation",
        "Source path operation",
        BEHAVIOUR,
        FieldKind::Enum {
            catalogue: "sourcePathOperation",
            optional: true,
        },
        "Conditional source path algebra; no runtime handler, current filename or filesystem fact.",
    ),
    f(
        "source_configuration_reference",
        "Source configuration reference",
        BEHAVIOUR,
        FieldKind::Enum {
            catalogue: "sourceConfigurationReference",
            optional: true,
        },
        "Registry-owned source operand grammar for a conditional configuration-object reference.",
    ),
    f(
        "is_unescape",
        "Unescapes",
        BEHAVIOUR,
        FieldKind::Bool,
        "Performs unescaping or decoding.",
    ),
    f(
        "cfg_rewrite_name",
        "CFG rewrite name",
        HOOKS,
        FieldKind::OptText,
        "Lowered command name for an ensemble subcommand the lowering pass rewrites.",
    ),
    f(
        "sub_subcommands",
        "Second-level subcommands",
        SUBS,
        FieldKind::SubSubCommands,
        "Operations selected by the word after this subcommand (`info object <op>`), each with its own release window.",
    ),
    f(
        "defines_command_at",
        "Defines command at",
        SUBS,
        FieldKind::OptIndex,
        "Index after the subcommand word whose literal value becomes a command name.",
    ),
    f(
        "max_leading_option_words",
        "Max leading option words",
        OPTS,
        FieldKind::OptIndex,
        "Cap on leading option words consumed; further option-shaped words are positional.",
    ),
];
