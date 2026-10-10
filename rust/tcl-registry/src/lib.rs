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

//! Command registry — single source of truth for Tcl command metadata.
//!
//! This crate defines [`CommandSpec`], [`SubCommand`], and the
//! [`CommandRegistry`] lookup facade. Every consumer (compiler,
//! analyser, codegen, LSP, formatter) reads command metadata from
//! here. No command-specific knowledge is hardcoded elsewhere.
//!
//! ## Architecture
//!
//! - [`arg_role`] — what role each argument plays (`Body`, `Expr`, `VarWrite`, ...).
//! - [`arity`] — argument count constraints.
//! - [`traits`] — behavioural trait bitflags replacing ~35 boolean fields.
//! - [`dialects`] — compact dialect membership sets.
//! - [`types`] — Tcl internal representation types (`TclType`).
//! - [`spec`] — [`CommandSpec`] and [`SubCommand`] definitions.
//! - [`registry`] — [`CommandRegistry`] lookup facade.
//! - [`commands`] — one file per command, one directory per dialect.
//! - [`events`] — iRules event metadata (176 events, firing order, flow chains).
//! - [`profiles`] — F5 profile types (66 profiles), protocol namespaces (113),
//!   and stack modification commands.
//! - [`special_vars`] — dialect-versioned interpreter-provided variables
//!   (`auto_path`, `env`, `tcl_platform`, the iRules `static::` namespace).
//!
//! ## One file per command
//!
//! Each command lives in its own `.rs` file under `commands/<dialect>/`.
//! Command files return a [`CommandSpec`] with all metadata declared
//! inline. Use `..CommandSpec::DEFAULT` to fill unset fields.

#![deny(missing_docs)]

pub mod abbrev;
pub mod arg_role;
pub mod arity;
pub mod array_iteration;
pub mod authored_math_functions;
pub mod base_objects;
pub mod bigip;
pub mod body_execution;
pub mod body_kind;
pub mod bpf_op;
pub mod byte_array_effect;
pub(crate) mod cache;
pub mod case_bodies;
pub mod catch_invocation;
pub mod clause_shape;
pub mod command_lookup;
pub mod command_prefix_target;
pub mod command_snapshot;
pub mod command_table;
pub mod commands;
pub mod completion;
pub mod completion_route;
pub mod conditional_expression;
pub mod const_fold;
pub mod definer;
pub mod deprecation;
pub mod dialects;
pub mod dictionary_scope;
pub mod dispatch_stability;
pub mod documentation;
mod event_descriptions;
pub mod event_facts;
pub mod events;
pub mod expr_surface;
pub mod f5;
pub mod forms;
pub mod frame_effect;
pub mod handle_binding;
pub mod hooks;
pub mod hover;
pub mod intrinsic;
pub mod invocation_words;
pub mod irules_policy;
pub mod iteration_entry;
pub mod lambda_invocation;
pub mod lifecycle;
pub mod list_object_methods;
pub mod literal_validation;
pub mod logical_core_simulation;
pub mod mathfunc;
pub mod model;
pub mod native_array_compilation;
pub mod native_binary_usage;
pub mod native_binary_value;
pub mod native_command_literal;
pub mod native_compilation;
pub mod native_compiled_variables;
pub mod native_compiler_word_projection;
pub mod native_compiler_words;
pub mod native_control_compilation;
pub mod native_control_instructions;
pub mod native_coroutine_compilation;
pub mod native_dictionary;
pub mod native_dictionary_compilation;
pub mod native_dictionary_scope_compilation;
pub mod native_each_compilation;
#[cfg(test)]
mod native_each_try_compilation_tests;
pub mod native_error_compilation;
pub mod native_expression_program;
pub mod native_introspection_compilation;
pub mod native_mathop_compilation;
pub mod native_namespace_binding_compilation;
pub mod native_namespace_string_compilation;
pub mod native_namespace_upvar_compilation;
pub mod native_scalar_compilation;
pub mod native_string_compilation;
pub mod native_string_trim_compilation;
pub mod native_switch_compilation;
pub mod native_tcloo_bootstrap;
pub mod native_tcloo_compilation;
pub mod native_tcloo_info;
pub mod native_tcloo_method_cache;
pub mod native_tcloo_method_definition;
pub mod native_tcloo_registration;
pub mod native_try_compilation;

pub mod native_bytecode;
pub mod native_compiler_pass;
pub mod native_each_loop;
pub mod native_ensemble;
pub mod native_ensemble_rewrite;
/// Actual native command-source error-info formatting.
pub mod native_error_log;
pub mod native_error_objects;
pub mod native_eval_object;
pub mod native_event;
pub mod native_expression_error;
pub mod native_handler_path;
pub mod native_index_lookup;
pub mod native_info_exists_compilation;
/// Independent native info version-reporting sources.
pub mod native_info_version;
pub mod native_instruction_plan;
pub mod native_lambda;
pub mod native_list_assignment;
pub mod native_list_index;
pub mod native_list_index_compilation;
pub mod native_list_operations_compilation;
pub mod native_lowering;
pub mod native_namespace_code;
/// Authenticated native C namespace-name primary semantics.
pub mod native_namespace_name;
pub mod native_namespace_upvar;
pub mod native_numeric_conversion;
pub mod native_numeric_error;
pub mod native_object_append;
pub mod native_object_vector;
pub mod native_package;
pub mod native_procedure;
pub mod native_procedure_body;
pub mod native_result;
pub mod native_return_compilation;
pub mod native_return_options;
pub mod native_rmw;
pub mod native_scripted_distribution;
pub mod native_selected_worker;
pub mod native_stock_list;
pub mod native_string_length;
pub mod native_string_materialization;
mod native_try;
pub mod native_unset_compilation;
pub mod native_unset_options;
pub mod native_upvar_compilation;
pub mod native_vwait;
pub use native_try::{NativeTryClauseArgument, NativeTryClauseFailure};
pub mod native_regex;
pub mod native_usage;
pub mod native_variable_destruction;
pub mod native_variable_name;
/// Selected native and explicitly authored logical argument-count error metadata.
pub mod native_wrong_arguments;
pub mod pack_hooks;
pub mod patterns;
pub mod presentation;
pub mod private_tcl_namespaces;
pub mod profile_defaults;
pub(crate) mod profile_queries;
pub mod profiles;
pub mod registry;
pub mod relation;
pub mod remote_method;
pub mod repeated;
pub mod representation;
pub mod resolved_invocation;
pub mod result_stability;
pub mod return_type;
pub mod runtime_expr_validation;
pub mod scoped;
pub mod script_body_flow;
pub mod security_floor;
pub mod semantic_operation;
pub mod side_effects;
pub mod snapshot;
pub mod source_file;
pub mod source_navigation;
pub mod source_path;
pub mod spec;
pub mod special_vars;
pub mod state_transition;
pub mod substitution;
pub mod symbol_def;
pub mod taint;
pub mod tk_geometry;
pub mod traits;
pub mod types;
pub mod variable_output;
pub mod version;
pub mod version_range;
pub mod world_effect;

pub use crate::hover::{first_positional_index, leading_option_specs};

/// Convenience prelude for command spec files.
///
/// `use crate::prelude::*;` in each command file brings in all the
/// types needed to construct a `CommandSpec`.
pub mod prelude {
    pub use crate::ScriptLookupScope;
    pub use crate::abbrev::{KeywordMatch, KeywordTable, PrefixMatching};
    pub use crate::arg_role::{AppendedArity, AppendedAritySet, ArgRole};
    pub use crate::arity::Arity;
    pub use crate::body_kind::{BodyInterpreter, BodyKind};
    pub use crate::bpf_op::{
        BpfDeclKind, BpfEffects, BpfOpKind, BpfOpSpec, BpfProgTypeSet, BpfScalarWidth,
        BpfVerdictKind,
    };
    pub use crate::byte_array_effect::ByteArrayEffect;
    pub use crate::clause_shape::{
        ClauseShapeChecker, ClauseShapeError, ClauseShapeIssue, ClauseShapeRepair,
    };
    pub use crate::command_table::CommandTableEffect;
    pub use crate::completion::{
        CompletionCode, CompletionCodeDomain, CompletionDescriptor, CompletionPayloadObligation,
        CompletionPayloadObligations, CompletionValueSemantics,
    };
    pub use crate::definer::{
        DefinerFamily, DefinitionBodyGrammar, MemberCurrentNamespace, MemberKind, MemberRefKind,
        MemberRetraction, MemberSpec, MemberVisibility, RetractionWords,
    };
    pub use crate::deprecation::{
        DeprecationFixHook, DeprecationFixSafety, SourceDeprecationAdvice,
    };
    pub use crate::dispatch_stability::{
        DispatchDependencies, DispatchDependencyComposition, DispatchDependencyDescriptor,
        DispatchDependencyDomain,
    };
    pub use crate::documentation::{DocumentationAnnotation, DocumentationExample};
    pub use crate::events::{
        ASM_PAYLOAD, BIGIP_EVENT_HANDLER_PRIORITY, CACHE_PAYLOAD, DIAMETER_PAYLOAD,
        DataCollectionAction, DataCollectionOperation, EventEmission, EventEmissionCertainty,
        EventEmissionForm, EventHandlerPriority, EventRequirementForm, EventRequires, GTP_PAYLOAD,
        HTTP_COLLECT, HTTP_PAYLOAD, HTTP_RELEASE, IrulesTopLevelEffect, MQTT_COLLECT, MQTT_PAYLOAD,
        MQTT_RELEASE, MR_COLLECT, MR_PAYLOAD, MR_RELEASE, REWRITE_PAYLOAD, RTSP_COLLECT,
        RTSP_PAYLOAD, RTSP_RELEASE, SCTP_COLLECT, SCTP_PAYLOAD, SCTP_RELEASE, SIP_PAYLOAD,
        SSL_COLLECT, SSL_PAYLOAD, SSL_RELEASE, TCP_COLLECT, TCP_PAYLOAD, TCP_RELEASE, UDP_PAYLOAD,
        WS_COLLECT, WS_PAYLOAD, WS_RELEASE, XML_PAYLOAD,
    };
    pub use crate::forms::{CommandForm, LiteralArgumentPrefix, SubCommandForm};
    pub use crate::frame_effect::{
        FrameArgLayout, FrameEffectSpec, FrameLevel, FrameLevelWord, FrameSuccessProjection,
        NativeFrameLevelCache, NativeFrameLevelProtocol,
    };
    pub use crate::handle_binding::{
        BoundHandle, HandleBindingSpec, HandleClassSource, HandleKeyword, HandleName,
    };
    pub use crate::hooks::{
        AnalyserHookId, ArgTypeHint, CodegenHookId, LoweringHookId, ReturnTypeHookId, TclVersion,
        VersionedConstFoldFn,
    };
    pub use crate::hover::{
        ArgValue, CallbackTaintInput, FormKind, FormSpec, HoverSnippet, IntegerDomain, OptionArg,
        OptionArity, OptionSpec, OptionValue, OptionValueHook, OptionValueOutcome, ScriptTiming,
        VariableScope, first_positional_index, leading_option_specs,
    };
    pub use crate::intrinsic::IntrinsicId;
    pub use crate::invocation_words::{CommandPrefixArguments, InvocationArguments};
    pub use crate::lifecycle::{Lifecycle, LifecycleState};
    pub use crate::literal_validation::{
        LiteralArgumentIssue, LiteralArgumentIssueReason, LiteralArgumentValidation,
        LiteralValidationDecline,
    };
    pub use crate::native_lowering::{ArityRule, CellUpdate, NativeLowering, ScopeKind};
    pub use crate::patterns::{FormatType, PatternArg, PatternType};
    pub use crate::presentation::ArgPresentation;
    pub use crate::relation::{
        Relation, RelationFactSource, RelationKind, RelationMode, RelationTermKind,
        RelationVerdict, RelationViolation, TermHolds,
    };
    pub use crate::repeated::RepeatedArgLayout;
    pub use crate::representation::RepresentationEffect;
    pub use crate::result_stability::ResultStability;
    pub use crate::scoped::{ScopedCommand, ScopedCommandEnv};
    pub use crate::semantic_operation::{InlineBodyErrorContext, SemanticOperationId};
    pub use crate::side_effects::{
        ConnectionSide, SideEffect, SideEffectTarget, SideSwitchTarget, StorageType,
    };
    pub use crate::source_path::SourcePathOperation;
    pub use crate::spec::{
        BytePayloadSpec, CaseForceListShape, CaseListSpec, CommandSpec, ConstraintReport,
        ConstraintSlot, ConstraintsHook, ContextGate, DefaultFormFirstWord, InlineCaseClause,
        ObjectClassSpec, OoContextFact, OptionFacts, OptionPlacement, OptionRelation, OptionScope,
        OptionTerm, ScriptTimingResolver, SubCommand, SubSubCommand, VersionedArgValue,
        leading_option_word_count, leading_option_word_count_with, resolve_option_prefix,
        resolve_option_prefix_with,
    };
    pub use crate::state_transition::{
        CallerFrameSelection, ChildInterpreterSafety, CommandBindingDefinitionKind,
        CommandBindingTransition, CommandResolutionImpact, InterpreterTransition,
        NamespaceTransition, NamespaceTransitionTarget, ObjectDispatchKind, ObjectDispatchLayer,
        ObjectDispatchTarget, ObjectDispatchTransition, ObjectPrivateNamespace, StateTransition,
        StateTransitionArgumentShape, StateTransitionCommit, StateTransitionComposition,
        StateTransitionDescriptor, StateTransitionDomain, StateTransitionOperandLayout,
        StateTransitionResolver, StateTransitionWideningRule, StateTransitions, TraceOperation,
        TraceOperationSet, TraceTarget, TraceTransition, TransitionSubject,
        VariableAliasDestination, VariableAliasFrame, VariableAliasTarget,
        VariableCellAliasTransition,
    };
    pub use crate::symbol_def::{DefinedSymbolKind, SymbolDef};
    pub use crate::taint::{
        SetterConstraint, TaintColour, TaintColourAtom, TaintNumericCoercion,
        TaintTransformCondition,
    };
    pub use crate::tk_geometry::{
        GRID_GEOMETRY, PACK_GEOMETRY, PLACE_GEOMETRY, TkGeometryContainerPolicy,
        TkGeometryManagerSpec, is_widget_path, is_widget_path_or_root, widget_path_is_within,
    };
    pub use crate::traits::Traits;
    pub use crate::types::{ReturnElements, TclType, VarElementsEffect, VarWriteTyping};
    pub use crate::world_effect::{
        CallbackEffect, CallbackKinds, EffectAccessMode, Reentrancy, StaticEffectAccess,
        StaticEffectFootprint, StaticInterpreterScope, StaticNamespaceScope, StaticSubjectScope,
        TransitionEffectCoverage, WorldEffectComposition, WorldEffectDescriptor,
        WorldEffectDynamicFallback, WorldEffectResolver, WorldEffectWriteSource, WorldStateDomain,
    };
}

// Re-export key types at crate root.
pub use arg_role::{AppendedArity, AppendedAritySet, ArgRole};
pub use arity::Arity;
pub use bigip::{BigipObjectSpec, BigipPropertySpec, BigipRegistry, ValueKind};
pub use body_kind::{BodyInterpreter, BodyKind};
pub use byte_array_effect::ByteArrayEffect;
pub use cache::{
    core_surface_generation, default_registry, register_core_surface_specs,
    registry_for_profile_with_overlay, safe_interp_hidden_commands,
};
pub use clause_shape::{ClauseShapeChecker, ClauseShapeError, ClauseShapeIssue, ClauseShapeRepair};
pub use command_prefix_target::CommandPrefixTarget;
pub use command_table::CommandTableEffect;
pub use completion::{
    CompletionCode, CompletionCodeDomain, CompletionDescriptor, CompletionPayloadObligation,
    CompletionPayloadObligations, CompletionValueSemantics,
};
pub use dialects::{
    DETECT_SCAN_BYTES, detect_dialect, detect_dialect_directive, detect_dialect_from_source,
    dialect_from_extension,
};
pub use dispatch_stability::{
    DispatchDependencies, DispatchDependencyComposition, DispatchDependencyDescriptor,
    DispatchDependencyDomain, ResolvedDispatchDependencies,
};
pub use events::{
    CollectionReleaseRequirement, DataCollectionAction, DataCollectionOperation,
    DataCollectionProtocol, EventHandlerPriority, PayloadCollectionRequirement,
    PayloadCollectionRequirementForm,
};
pub use frame_effect::{
    FrameArgLayout, FrameEffectSpec, FrameLevel, FrameLevelWord, FrameSuccessProjection,
    NativeFrameLevelCache, NativeFrameLevelProtocol,
};
pub use handle_binding::{
    BoundHandle, HandleBindingSpec, HandleClassSource, HandleKeyword, HandleName,
};
pub use hover::{ArgValue, CallbackTaintInput, ScriptTiming, VariableScope};
pub use intrinsic::IntrinsicId;
pub use invocation_words::{
    CommandPrefixArguments, CompletionOptionsPolicy, EnsembleImplementationFamily,
    InvocationArgument, InvocationArguments, InvocationDialect, InvocationWord, InvocationWordKind,
    InvocationWords, NativeArgumentUsageHeader, RoleOperandAlternatives, RoleOperandValues,
    VariableReadProjection, VariableWriteProjection,
};
pub use literal_validation::{
    LiteralArgumentIssue, LiteralArgumentIssueReason, LiteralArgumentValidation,
    LiteralArgumentValidator, LiteralValidationDecline,
};
pub use native_lowering::{ArityRule, CellUpdate, NativeLowering, ScopeKind};
pub use patterns::{FormatType, PatternType};
pub use presentation::ArgPresentation;
pub use profile_queries::VendorSurface;
pub use registry::{
    CommandRegistry, EffectiveCommandSemantics, EffectiveRegistrySemantics, FormatStringArg,
    MethodDispatchKind, NameProviders, ProcedureWords, RegistrySemanticKey, RegistrySnapshot,
    ResolvedCall, ResolvedTerminator, TryClauseKind, TryCompletionSelector, TryControlClause,
    TryControlInvocation, selected_try_control_invocation,
};
pub use relation::{
    Relation, RelationFactSource, RelationKind, RelationMode, RelationTermKind, RelationVerdict,
    RelationViolation, TermHolds,
};
pub use repeated::RepeatedArgLayout;
pub use representation::RepresentationEffect;
pub use resolved_invocation::{
    AuthoredSourceAppendArguments, AuthoredSourceArity, AuthoredSourceCaseBody,
    AuthoredSourceCommandPublication, AuthoredSourceCommandPublicationKind,
    AuthoredSourceDescriptors, AuthoredSourceExpressionArguments, AuthoredSourceLambdaCall,
    AuthoredSourceOption, AuthoredSourceOptionBoundary, AuthoredSourceOptionRelationships,
    AuthoredSourceOptionScan, AuthoredSourceSubcommandDiagnostic, InvocationArgumentCount,
    InvocationFacts, InvocationOptions, InvocationResolutionUnresolved, InvocationSemantics,
    NamedObjectFactory, OwnedSubcommandResolution, ResolvedForm, ResolvedInvocation,
    ResolvedSubcommand, StructuredInvocationResolution, SubcommandResolution,
    SubcommandResolutionKind,
};
pub use result_stability::ResultStability;
pub use semantic_operation::{InlineBodyErrorContext, SemanticOperationId};
pub use side_effects::SideSwitchTarget;
pub use spec::{
    BytePayloadSpec, CaseForceListShape, CaseListSpec, CommandSpec, ConstraintReport,
    ConstraintSlot, ConstraintsHook, ContextGate, DefaultFormFirstWord, InlineCaseClause,
    ObjectClassSpec, OoContextFact, OptionFacts, OptionPlacement, OptionRelation, OptionScope,
    OptionTerm, ScriptTimingResolver, SubCommand, SubSubCommand, VersionedArgValue,
};
pub use special_vars::{
    SPECIAL_VARS, SpecialVarKey, SpecialVarKind, SpecialVarSpec, StartupBinding, VarAccess,
    VarOrigin, is_externally_read, is_initially_bound, is_lazily_readable, is_readable_at_startup,
    is_special_var, special_var, special_var_in_dialect, special_var_read_taint,
    special_var_write_effect, special_vars_for_dialect,
};
pub use state_transition::{
    AbruptTransitionTransfer, AliasTargetLookup, CallerFrameSelection, ChildInterpreterSafety,
    CommandBindingDefinitionKind, CommandBindingTransition, CommandResolutionImpact,
    InterpreterTransition, NamespaceTransition, NamespaceTransitionTarget, ObjectDispatchKind,
    ObjectDispatchLayer, ObjectDispatchTarget, ObjectDispatchTransition, ObjectPrivateNamespace,
    ResolvedStateTransitions, StateTransition, StateTransitionArgumentShape, StateTransitionCommit,
    StateTransitionComposition, StateTransitionDescriptor, StateTransitionDomain,
    StateTransitionFact, StateTransitionKnowledge, StateTransitionOperandLayout,
    StateTransitionResolver, StateTransitionWidening, StateTransitionWideningRule,
    StateTransitions, TraceOperation, TraceOperationSet, TraceTarget, TraceTransition,
    TransitionSubject, VariableAliasDestination, VariableAliasFrame, VariableAliasTarget,
    VariableCellAliasTransition,
};
pub use symbol_def::{DefinedSymbolKind, SymbolDef};
pub use taint::{SetterConstraint, TaintColour, TaintColourAtom};
pub use traits::{FRAME_REACH_TRAITS, Traits, UNIT_LINKAGE_TRAITS};
pub use types::{ReturnElements, TclType, VarElementsEffect, VarWriteTyping};
pub use world_effect::{
    CallbackEffect, CallbackKinds, EffectAccess, EffectAccessMode, EffectFootprint,
    InterpreterScope, LegacyEffectBridge, NamespaceScope, Reentrancy, ResolvedWorldEffects,
    StaticEffectAccess, StaticEffectFootprint, StaticInterpreterScope, StaticNamespaceScope,
    StaticSubjectScope, SubjectScope, TransitionEffectCoverage, TransitionEffectCoverages,
    WorldEffectComposition, WorldEffectDescriptor, WorldEffectDynamicFallback, WorldEffectResolver,
    WorldEffectWriteSource, WorldStateDomain,
};

/// Crate version string.
///
/// ```
/// assert!(!tcl_registry::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod script_lookup_scope;
pub use script_lookup_scope::ScriptLookupScope;

mod selected_script_timing;

pub mod native_jim_local;
/// Selected variable-table hash and entry-order policy.
pub mod native_jim_lookup;
pub mod native_jim_switch;
pub mod native_variable_table;

/// Actual direct C variable trace registration policy.
pub mod native_variable_trace;

/// Actual interpreter option declarations and two-stage native lookup.
pub mod native_interpreter_options;

/// Jim Enum and immediate-string original cache recipes.
pub mod native_jim_enum;

pub mod native_property_lookup;

pub use deprecation::SourceDeprecationAdvice;
pub use source_path::SourcePathOperation;
