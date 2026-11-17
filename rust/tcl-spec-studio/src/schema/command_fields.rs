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

//! Every editable field of `CommandSpec`, in `CommandSpec::DEFAULT` order.

use super::{
    ADVANCED, ARGS, AVAILABILITY, BEHAVIOUR, DEPRECATION, DOCS, EFFECTS, FieldKind, FieldSchema,
    HOOKS, IDENTITY, OPTS, SUBS, TAINT, TYPES, f,
};

/// Every editable field of `CommandSpec`, in `CommandSpec::DEFAULT` order.
pub const COMMAND_FIELDS: &[FieldSchema] = &[
    f(
        "name",
        "Command name",
        IDENTITY,
        FieldKind::Text,
        "Command name as written in Tcl — `for`, `dict`, `HTTP::header`.",
    ),
    f(
        "traits",
        "Traits",
        BEHAVIOUR,
        FieldKind::FlagSet {
            catalogue: "traits",
            optional: false,
        },
        "Behavioural trait flags every consumer reads instead of naming the command.",
    ),
    f(
        "surface",
        "Surface",
        AVAILABILITY,
        FieldKind::FlagSet {
            catalogue: "dialects",
            optional: true,
        },
        "Where the command exists, as the dialects its surface rows cover. Unset means everywhere.",
    ),
    f(
        "arity",
        "Arity",
        ARGS,
        FieldKind::Arity,
        "Argument-count constraint, counted after the command name.",
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
        "arg_roles",
        "Argument roles",
        ARGS,
        FieldKind::RoleMap,
        "Static role per 0-based argument index, for fixed-layout commands.",
    ),
    f(
        "arg_role_resolver",
        "Argument-role resolver",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(my_resolver)",
        },
        "Callback assigning roles from the actual argument list; wins over `arg_roles`.",
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
        "Formatter layout override per 0-based argument index; body arguments are block-expanded unless declared inline.",
    ),
    f(
        "repeated_args",
        "Repeated argument layouts",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "&[RepeatedArgLayout::strided(ArgRole::VarWrite, 0, 2)]",
        },
        "Roles that recur at a fixed stride over the argument tail (`global a b c`, `variable n v n v`).",
    ),
    f(
        "frame_effect",
        "Frame effect",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(frame_effect::UPVAR)",
        },
        "How the command crosses stack frames: level word, frame-selected variable args, and caller-frame scripts.",
    ),
    f(
        "clause_shape_check",
        "Clause-shape checker",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(clause_shape::if_chain)",
        },
        "Validator for a clause chain whose shapes are not a single min..=max range.",
    ),
    f(
        "command_prefixes",
        "Command-prefix positions",
        ARGS,
        FieldKind::PrefixMap,
        "Argument indices carrying a callback command prefix, with the arity appended to it.",
    ),
    f(
        "command_prefix_resolver",
        "Command-prefix resolver",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(my_prefix_resolver)",
        },
        "Callback locating command-prefix positions that depend on the argument list.",
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
        "Callback assigning SameInvocation, Deferred, or ReferenceOnly to executable positions from the actual argument list.",
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
        "The intrep of the value `[cmd …]` yields.",
    ),
    f(
        "var_write_typing",
        "Variable-write typing",
        TYPES,
        FieldKind::RustExpr {
            hint: "VarWriteTyping::Fixed(TclType::String)",
        },
        "How the command types the variables it writes, when that differs from its return value.",
    ),
    f(
        "variable_write_min_args",
        "Variable-write argument floor",
        TYPES,
        FieldKind::OptCount,
        "Minimum post-head argument count at which the command can write a variable; unset makes no claim beyond its role descriptors.",
    ),
    f(
        "return_type_hook",
        "Return-type hook",
        TYPES,
        FieldKind::Enum {
            catalogue: "returnTypeHook",
            optional: true,
        },
        "Algorithm that types a call whose result shape moves with the call.",
    ),
    f(
        "return_elements",
        "Return elements",
        TYPES,
        FieldKind::RustExpr {
            hint: "Some(ReturnElements::ListOfArgs { from: 0 })",
        },
        "How the result relates to container element structure.",
    ),
    f(
        "var_elements_effect",
        "Variable element effect",
        TYPES,
        FieldKind::RustExpr {
            hint: "Some(VarElementsEffect::AppendsListElements { values_from: 1 })",
        },
        "How the command evolves the container elements of the variable it writes in place.",
    ),
    f(
        "representation_effect",
        "Representation effect",
        TYPES,
        FieldKind::RustExpr {
            hint: "Some(RepresentationEffect::copy_on_write_container(0, 2))",
        },
        "Effect on Tcl's dual string/internal representation or shared-object storage.",
    ),
    f(
        "arg_types",
        "Argument type hints",
        TYPES,
        FieldKind::ArgTypeMap,
        "Expected intrep, shimmer risk, and transparent source types per argument index.",
    ),
    f(
        "subcommands",
        "Subcommands",
        SUBS,
        FieldKind::SubCommands,
        "Ensemble subcommands, each a self-contained metadata bundle.",
    ),
    f(
        "allow_unknown_subcommands",
        "Allow unknown subcommands",
        SUBS,
        FieldKind::Bool,
        "Accept a subcommand word that is not declared, without a W001 warning.",
    ),
    f(
        "option_prefix_words",
        "Positional prefix words",
        OPTS,
        FieldKind::Count,
        "Fixed positional words before the selected invocation's option run.",
    ),
    f(
        "prefix_matching",
        "Prefix matching",
        SUBS,
        FieldKind::Enum {
            catalogue: "prefixMatching",
            optional: false,
        },
        "Whether this command's keyword tables accept unique-prefix \
         abbreviations (`Tcl_GetIndexFromObj`) or only exact spellings.",
    ),
    f(
        "default_form_first_word",
        "Default-form first word",
        SUBS,
        FieldKind::Enum {
            catalogue: "defaultFormFirstWord",
            optional: true,
        },
        "The value shape a non-subcommand first word may take (`after 200 …`).",
    ),
    f(
        "hover",
        "Hover documentation",
        DOCS,
        FieldKind::Hover,
        "Summary, synopsis lines, prose, source, examples, and return value.",
    ),
    f(
        "documentation_examples",
        "Annotated examples",
        DOCS,
        FieldKind::DocumentationExamples,
        "Worked examples whose notes point at exact text on each line, checked like the built-in examples.",
    ),
    f(
        "forms",
        "Invocation forms",
        DOCS,
        FieldKind::Forms,
        "Documented synopsis and getter/setter classification; does not select semantic routing.",
    ),
    f(
        "command_forms",
        "Invocation refinements",
        ADVANCED,
        FieldKind::Refinements,
        "Per-form overlays: arity, literal-prefix selection, and the traits, mutator status and effects one call shape replaces.",
    ),
    f(
        "semantic_operation",
        "Semantic operation",
        HOOKS,
        FieldKind::SemanticOperation,
        "Target-neutral operation identity selected before backend dispatch.",
    ),
    f(
        "semantic_operation_windows",
        "Semantic operation windows",
        HOOKS,
        FieldKind::RustExpr {
            hint: "SEMANTIC_OPERATION_WINDOWS",
        },
        "Per-release semantic operations, for a command whose target-neutral operation differs across Tcl releases. Empty unless it does; the plain operation is the fallback where no window covers the primary release, and a point that does not settle the release dispatches plain.",
    ),
    f(
        "completion",
        "Completion contract",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Some(CompletionDescriptor::exact(COMPLETION_CODES))",
        },
        "Possible Tcl completion codes and result/options payload obligations.",
    ),
    f(
        "assigns_variable_at",
        "Assigns variable at",
        ARGS,
        FieldKind::OptIndex,
        "Legacy shorthand: the single argument index naming a variable the command writes.",
    ),
    f(
        "safe_on_uninit",
        "Safe on uninitialised",
        AVAILABILITY,
        FieldKind::FlagSet {
            catalogue: "dialects",
            optional: true,
        },
        "Dialects where the command safely initialises an uninitialised variable.",
    ),
    f(
        "const_fold",
        "Constant folder",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(const_fold::fold_string_length)",
        },
        "Compile-time folder returning the command's constant result.",
    ),
    f(
        "const_fold_versioned",
        "Versioned constant folder",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(const_fold::fold_format)",
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
        "Per-command lowering specialisation in the compiler's dispatch table.",
    ),
    f(
        "codegen_hook",
        "Bytecode codegen hook",
        HOOKS,
        FieldKind::Enum {
            catalogue: "codegenHook",
            optional: true,
        },
        "Per-command TclVM bytecode emitter. Unset uses the generic invoke emitter.",
    ),
    f(
        "codegen_hook_windows",
        "Bytecode codegen hook windows",
        HOOKS,
        FieldKind::RustExpr {
            hint: "CODEGEN_HOOK_WINDOWS",
        },
        "Per-release bytecode emitters, for a command whose TclVM emitter differs across Tcl releases. Empty unless it does; the plain hook is the fallback where no window covers the primary release, and a point that does not settle the release dispatches plain.",
    ),
    f(
        "inline_codegen_hook",
        "Inline codegen hook",
        HOOKS,
        FieldKind::Enum {
            catalogue: "inlineCodegenHook",
            optional: true,
        },
        "Emitter for the value-position and catch-body paths.",
    ),
    f(
        "inline_codegen_hook_windows",
        "Inline codegen hook windows",
        HOOKS,
        FieldKind::RustExpr {
            hint: "INLINE_CODEGEN_HOOK_WINDOWS",
        },
        "Per-release value-position emitters; the contract of the bytecode codegen hook windows.",
    ),
    f(
        "bpf_op",
        "BPF-Tcl lowering descriptor",
        HOOKS,
        FieldKind::RustExpr {
            hint: "Some(&bpf_op::XDP_PASS)",
        },
        "Typed BPF-Tcl lowering descriptor; the BPF front-end dispatches on this, never on the command name.",
    ),
    f(
        "native_lowering",
        "Native lowering shape",
        HOOKS,
        FieldKind::RustExpr {
            hint: "Some(NativeLowering::Structured(LoweringHookId::Set))",
        },
        "Which native code shape the executable-IR lowering gives this command; stamped beside the lowering hook or intrinsic it mirrors. Unset is the generic argv invocation.",
    ),
    f(
        "native_lowering_windows",
        "Native lowering windows",
        HOOKS,
        FieldKind::RustExpr {
            hint: "NATIVE_LOWERING_WINDOWS",
        },
        "Per-release native lowering shapes; the contract of the bytecode codegen hook windows. A windowed shape is not a basis for a derived value-transfer specialisation.",
    ),
    f(
        "semantics",
        "Value-transfer declaration",
        HOOKS,
        FieldKind::RustExpr {
            hint: "SemanticsDeclaration::Declared(&value_transfer::builtins::LIST_LENGTH)",
        },
        "The value-transfer specialisation declared at command scope — what an invocation computes, which storage it writes, and the route that computes it (below) — or an explicit abstention (`Declined`). Unset inherits, or derives from a descriptor stating the same operation. A shipped, compiled-in specialisation is nameable but not reconstructable, so it also reads unset here.",
    ),
    f(
        "analyser_hook",
        "Analyser hook",
        HOOKS,
        FieldKind::Enum {
            catalogue: "analyserHook",
            optional: true,
        },
        "Per-command handler family in the analyser's central dispatch.",
    ),
    f(
        "command_table_effect",
        "Command-table effect",
        EFFECTS,
        FieldKind::Enum {
            catalogue: "commandTableEffect",
            optional: true,
        },
        "How the command mutates the interpreter's command table.",
    ),
    f(
        "side_effects",
        "Side effects",
        EFFECTS,
        FieldKind::SideEffects,
        "Structured declarations of the state the command reads and writes, each with its own release window.",
    ),
    f(
        "side_effects_knowledge",
        "Side-effect completeness",
        EFFECTS,
        FieldKind::Knowledge,
        "How completely the side effects are known: unknown, partial (a lower bound), or complete for the release windows evidenced from the implementation.",
    ),
    f(
        "world_effects",
        "World effects",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Knowledge::Partial(WorldEffectDescriptor::EMPTY)",
        },
        "Target-neutral mutable-world footprint for common compiler analysis.",
    ),
    f(
        "world_effects_knowledge",
        "World-effect completeness",
        EFFECTS,
        FieldKind::Knowledge,
        "How completely the world effects are known: unknown, partial (a lower bound), or complete for the release windows evidenced from the implementation.",
    ),
    f(
        "state_transitions",
        "State transitions",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Some(StateTransitionDescriptor::EMPTY)",
        },
        "Target-neutral command, namespace, interpreter, trace, and alias transitions.",
    ),
    f(
        "dispatch_dependencies",
        "Dispatch dependencies",
        EFFECTS,
        FieldKind::RustExpr {
            hint: "Some(DispatchDependencyDescriptor::replace(DispatchDependencies::BASE))",
        },
        "Mutable Tcl domains that must remain stable before specialisation.",
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
            hint: "Some(ResultStability::ReferentiallyTransparent)",
        },
        "Whether repeated calls return the same value, or depend on mutable or volatile state.",
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
        "Registry callback for relationships and member sets within statically-known arguments.",
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
        "required_package",
        "Required package",
        AVAILABILITY,
        FieldKind::OptText,
        "Package that must be `require`d before the command is visible.",
    ),
    f(
        "tk_geometry",
        "Tk geometry manager",
        ADVANCED,
        FieldKind::TkGeometry,
        "Structured container, placement-form, and release semantics for a Tk geometry manager.",
    ),
    f(
        "excluded_events",
        "Excluded iRules events",
        AVAILABILITY,
        FieldKind::TextList,
        "iRules events the command is not valid in.",
    ),
    f(
        "unsafe_command",
        "Unsafe command",
        BEHAVIOUR,
        FieldKind::Bool,
        "Allows context escalation in sandboxed dialects — drives IRULE2003.",
    ),
    f(
        "closed_value_args",
        "Closed value arguments",
        OPTS,
        FieldKind::IndexList,
        "Argument indices whose declared values are an exhaustive legal set (W127).",
    ),
    f(
        "arg_values_accept_prefix",
        "Values accept a prefix",
        OPTS,
        FieldKind::Bool,
        "Whether a closed value is accepted as a unique prefix rather than an exact match.",
    ),
    f(
        "event_requires",
        "Event requirements",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(EventRequires { .. })",
        },
        "Layer-based iRules event requirements used by the IRULE1001 validity check.",
    ),
    f(
        "event_requirement_forms",
        "Event requirement forms",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "&[EventRequirementForm { .. }]",
        },
        "Argument-prefix-specific iRules event contracts that override the command-level requirements.",
    ),
    f(
        "event_emits",
        "Event emissions",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(EventEmission { .. })",
        },
        "The iRules events this command raises. The mirror of event_requires: that says where a command may be written, this says what running it starts.",
    ),
    f(
        "event_emission_forms",
        "Event emission forms",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "&[EventEmissionForm { .. }]",
        },
        "Argument-prefix-specific emissions that override the command-level one. A form declaring no events is a positive fact, not an absence.",
    ),
    f(
        "data_collection",
        "Data collection",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(HTTP_COLLECT)",
        },
        "Registry descriptor for a collect, release, or payload operation.",
    ),
    f(
        "irules_source_flow",
        "iRules source flow",
        ADVANCED,
        FieldKind::Enum {
            catalogue: "irulesSourceFlow",
            optional: true,
        },
        "Conditional source flow grammar for connection drops, DNS returns or event control. No handler, reached effect or body entry is granted.",
    ),
    f(
        "side_switch_target",
        "Side-switch target",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(SideSwitchTarget::Client)",
        },
        "Connection-side context selected while evaluating this command's body.",
    ),
    f(
        "event_handler_priority",
        "Event-handler priority",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(BIGIP_EVENT_HANDLER_PRIORITY)",
        },
        "Runtime default and implicit-priority policy for an event-handler command.",
    ),
    f(
        "irules_top_level_effect",
        "iRules top-level effect",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(IrulesTopLevelEffect::Priority)",
        },
        "Stateful effect of an iRules declaration, such as the priority inherited by later handlers.",
    ),
    f(
        "options",
        "Options",
        OPTS,
        FieldKind::Options,
        "Declared option flags, their values, roles, and dialect gates.",
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
        "Registry-declared relations between this command's options and arguments \
         (E-R14) — mutual exclusion, directional requires, requires-one-of, and forbids, \
         each checked natively with no VM entry.",
    ),
    f(
        "option_effect_families",
        "Option-effect families",
        OPTS,
        FieldKind::RustExpr {
            hint: "&[OptionEffectFamily { name: \"match\", base: FamilyBase::Only(EffectAxis::PatternLanguage(PatternType::Glob)), \
                   combine: FamilyCombine::LastWins, surface: None }]",
        },
        "The families this command's option effects cite — where each axis starts \
         and how two options of the family combine.",
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
        "reserved_trailing_words",
        "Reserved trailing words",
        OPTS,
        FieldKind::Count,
        "Trailing words C Tcl's own option scan never treats as option candidates.",
    ),
    f(
        "arg_values",
        "Argument values",
        OPTS,
        FieldKind::ArgValueMap,
        "Enumerable positional values per argument index, for completion; each carries its own Tcl floor and release window.",
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
        "procedure_definition",
        "Native procedure grammar",
        BEHAVIOUR,
        FieldKind::RustExpr {
            hint: "Some(NativeProcedureDefinitionSpec::Core)",
        },
        "Native definition argv, result and static-storage protocol; requires live native implementation proof.",
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
        "Whether evaluated body arguments run in the current interpreter or one selected by an invocation argument.",
    ),
    f(
        "body_arg_implicit_args",
        "Body implicit arguments",
        ARGS,
        FieldKind::Count,
        "Runtime-supplied positional arguments the body's first command receives.",
    ),
    f(
        "taint_output_sink",
        "Output-sink code",
        TAINT,
        FieldKind::OptText,
        "Diagnostic code emitted when tainted data reaches the output position (`T101`).",
    ),
    f(
        "taint_output_sink_subcommands",
        "Output-sink subcommands",
        TAINT,
        FieldKind::TextList,
        "Restricts the output sink to these subcommands. Empty means every invocation.",
    ),
    f(
        "taint_log_sink",
        "Log-sink code",
        TAINT,
        FieldKind::OptText,
        "Log-injection sink diagnostic code (`IRULE3003`).",
    ),
    f(
        "taint_network_sink_args",
        "Network-sink arguments",
        TAINT,
        FieldKind::OptIndexList,
        "Argument indices taking a network address — SSRF sinks.",
    ),
    f(
        "taint_code_sink_args",
        "Code-sink arguments",
        TAINT,
        FieldKind::OptIndexList,
        "The specific slots where a tainted value reaches eval-style evaluation.",
    ),
    f(
        "taint_numeric_coercion",
        "Numeric-coercion operands",
        TAINT,
        FieldKind::Enum {
            catalogue: "taintNumericCoercion",
            optional: true,
        },
        "Which argument words a call reads as numbers — a T100 numeric-coercion sink.",
    ),
    f(
        "taint_interp_eval_subcommands",
        "Cross-interpreter eval subcommands",
        TAINT,
        FieldKind::TextList,
        "Subcommands evaluating code in another interpreter (T105).",
    ),
    f(
        "taint_source",
        "Taint source colours",
        TAINT,
        FieldKind::FlagSet {
            catalogue: "taintColour",
            optional: true,
        },
        "Colour bits the return value carries when the command is a taint source.",
    ),
    f(
        "taint_transform",
        "Taint transform colours",
        TAINT,
        FieldKind::FlagSet {
            catalogue: "taintColour",
            optional: true,
        },
        "Colour bits the command adds to a tainted value it returns.",
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
        "Input colour whose presence means this command would double-encode (T106).",
    ),
    f(
        "taint_sink_safe_colour",
        "Sink-safe colour",
        TAINT,
        FieldKind::FlagSet {
            catalogue: "taintColour",
            optional: true,
        },
        "Colour that suppresses the dangerous-sink warning for this sink.",
    ),
    f(
        "taint_sink_gate",
        "Taint-sink gate",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(subst_evaluates_commands)",
        },
        "Predicate over the call's own flags deciding whether the sink applies.",
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
        "credential_options",
        "Credential options",
        TAINT,
        FieldKind::TextList,
        "Option flags whose value carries a secret (W310).",
    ),
    f(
        "sensitive_headers",
        "Sensitive headers",
        TAINT,
        FieldKind::TextList,
        "HTTP header names whose values are secrets.",
    ),
    f(
        "setter_constraints",
        "Setter constraints",
        TAINT,
        FieldKind::SetterConstraints,
        "Required argument prefixes on setter forms (IRULE3101).",
    ),
    f(
        "pattern_type",
        "Pattern type",
        OPTS,
        FieldKind::Enum {
            catalogue: "patternType",
            optional: true,
        },
        "The pattern language the command's pattern argument uses.",
    ),
    f(
        "pattern_arg_resolver",
        "Pattern-argument resolver",
        OPTS,
        FieldKind::RustExpr {
            hint: "Some(my_pattern_resolver)",
        },
        "Native hook selecting pattern positions and languages for a concrete call.",
    ),
    f(
        "format_string_type",
        "Format-string type",
        OPTS,
        FieldKind::Enum {
            catalogue: "formatType",
            optional: true,
        },
        "The format-string language the command's format argument uses.",
    ),
    f(
        "tcllib_package",
        "Tcllib package",
        AVAILABILITY,
        FieldKind::OptText,
        "Tcllib package providing the command, for per-document activation.",
    ),
    f(
        "introduced_version",
        "Introduced in",
        AVAILABILITY,
        FieldKind::OptText,
        "Dotted version of the owning package that introduced the command.",
    ),
    f(
        "deprecated_version",
        "Deprecated in",
        AVAILABILITY,
        FieldKind::OptText,
        "First package version where the command still exists but should warn.",
    ),
    f(
        "retired_version",
        "Retired in",
        AVAILABILITY,
        FieldKind::OptText,
        "First package version without the command (exclusive upper bound).",
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
        "warn_missing_import",
        "Warn on missing import",
        AVAILABILITY,
        FieldKind::Bool,
        "Whether W120 fires when the command is used without a `package require`.",
    ),
    f(
        "is_namespace_exported",
        "Namespace exported",
        AVAILABILITY,
        FieldKind::Bool,
        "Whether the source namespace exports the bare name.",
    ),
    f(
        "xc_translatable",
        "XC translatable",
        DEPRECATION,
        FieldKind::OptBool,
        "Cross-compile translatability override. Unset uses the default rules.",
    ),
    f(
        "deprecated_replacement",
        "Deprecated replacement",
        DEPRECATION,
        FieldKind::OptText,
        "Replacement command name surfaced by the deprecation code action.",
    ),
    f(
        "source_deprecation_advice",
        "Source deprecation advice",
        DEPRECATION,
        FieldKind::Enum {
            catalogue: "sourceDeprecationAdvice",
            optional: true,
        },
        "Registry-owned source shape advice; original words support a review proposal, without a runtime equivalence claim.",
    ),
    f(
        "deprecated_replacement_drop_in",
        "Replacement is drop-in",
        DEPRECATION,
        FieldKind::Bool,
        "Whether the replacement accepts the deprecated argument list unchanged.",
    ),
    f(
        "alias_of",
        "Alias of",
        IDENTITY,
        FieldKind::OptText,
        "The shipped builtin this pack command is — the only admissible source \
         of a builtin identity for a pack command. Unset for every shipped \
         command and every pack command that names no target.",
    ),
    f(
        "runtime_backing",
        "Runtime backing",
        IDENTITY,
        FieldKind::RuntimeBacking,
        "How the command's behaviour reaches the runtime: `none`, \
         `host-native`, `shipped-builtin ID`, `tcl-body {-package-source \
         PATH ?-evaluate?}` or `tcl-body {-pack-text {TEXT} ?-evaluate?}`. \
         Every shipped core command declares it; a command that declares \
         nothing reads as `none` — nothing executes it.",
    ),
    f(
        "byte_array_payload",
        "Byte-array payload",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(BytePayloadSpec::DEFAULT)",
        },
        "`<proto>::payload` layout for the S110 byte-array-corruption check.",
    ),
    f(
        "byte_array_effect",
        "Byte-array effect",
        BEHAVIOUR,
        FieldKind::Enum {
            catalogue: "byteArrayEffect",
            optional: false,
        },
        "How the command transforms a byte-array operand it derives its result from.",
    ),
    f(
        "definition_body",
        "Definition-body grammar",
        ADVANCED,
        FieldKind::DefinitionBody,
        "Body grammar for a class or type definer — a shipped grammar by name, or member rows that each state their effect — so the generic walker can recurse.",
    ),
    f(
        "manufacturer_methods",
        "Manufacturer methods",
        ADVANCED,
        FieldKind::ManufacturerMethods,
        "Class-command methods that create an instance, including their name, body, and constructor-argument positions.",
    ),
    f(
        "case_list",
        "Case list",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(&CaseListSpec::SWITCH)",
        },
        "The `{pattern body …}` clause list the command takes as its final braced word.",
    ),
    f(
        "oo_context_facts",
        "TclOO context facts",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "&[(\"class\", OoContextFact::DefiningClass)]",
        },
        "Keyword words whose value the enclosing TclOO method frame fixes, so the optimiser can fold them.",
    ),
    f(
        "self_receiver_words",
        "TclOO self-receiver words",
        ADVANCED,
        FieldKind::TextList,
        "Argument-0 closed words for which a bracketed `[cmd ?word?]` dispatch head denotes the current TclOO receiving object, same target as `my` (e.g. `self`'s `object`).",
    ),
    f(
        "object_class",
        "Object class",
        ADVANCED,
        FieldKind::ObjectClass,
        "Class metadata for a factory whose `new`/`create` returns a dispatchable handle.",
    ),
    f(
        "defines_symbol",
        "Defines symbol",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(SymbolDef { .. })",
        },
        "Descriptor for an argument binding a name the document outline should list.",
    ),
    f(
        "body_scope",
        "Body scope",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(&scoped::REPORT_DEFSTYLE)",
        },
        "Extra commands available only inside the command's body argument.",
    ),
    f(
        "binds_handle",
        "Binds handle",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(&handle_binding::SET_BINDS_HANDLE)",
        },
        "Which argument becomes an object handle, and which says its class.",
    ),
    f(
        "remote_method",
        "Remote method",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(&remote_method::ILX_CALL_METHOD)",
        },
        "Cross-language RPC role: which word is the handle, which is the method name.",
    ),
    f(
        "creates_instance_at",
        "Creates instance at",
        SUBS,
        FieldKind::OptIndex,
        "Argument index naming an object command of this spec's own object class.",
    ),
    f(
        "defines_command_at",
        "Defines command at",
        SUBS,
        FieldKind::OptIndex,
        "Argument index whose literal value becomes a callable command name.",
    ),
    f(
        "context_gate",
        "Context gate",
        ADVANCED,
        FieldKind::RustExpr {
            hint: "Some(irules_return_gate)",
        },
        "Validity gate keyed on lexical or dispatch context rather than argument shape.",
    ),
    f(
        "implementation_namespace",
        "Implementation namespace",
        SUBS,
        FieldKind::OptText,
        "Namespace the ensemble's subcommands are also individually callable under.",
    ),
];
