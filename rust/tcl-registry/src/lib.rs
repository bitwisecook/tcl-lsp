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
//! - [`profiles`] — F5 profile types (65 profiles), protocol namespaces (113),
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
pub mod base_objects;
pub mod bigip;
pub mod body_kind;
pub mod bpf_op;
pub mod byte_array_effect;
pub mod cache;
pub mod clause_shape;
pub mod command_snapshot;
pub mod command_table;
pub mod commands;
pub mod completion;
pub mod const_fold;
pub mod definer;
pub mod deprecation;
pub mod dialects;
pub mod dispatch_stability;
mod event_descriptions;
pub mod event_facts;
pub mod events;
pub mod expr_surface;
pub mod forms;
pub mod frame_effect;
pub mod handle_binding;
pub mod hooks;
pub mod hover;
pub mod intrinsic;
pub mod invocation_words;
pub mod lifecycle;
pub mod literal_validation;
pub mod mathfunc;
pub mod pack_hooks;
pub mod patterns;
pub mod presentation;
pub mod private_tcl_namespaces;
pub mod profile_defaults;
pub mod profile_queries;
pub mod profiles;
pub mod registry;
pub mod repeated;
pub mod representation;
pub mod resolved_invocation;
pub mod result_stability;
pub mod scoped;
pub mod semantic_operation;
pub mod side_effects;
pub mod snapshot;
pub mod spec;
pub mod special_vars;
pub mod state_transition;
pub mod stub_overlay;
pub mod symbol_def;
pub mod taint;
pub mod traits;
pub mod types;
pub mod version;
pub mod version_range;
pub mod world_effect;

pub use crate::hover::first_positional_index;

/// Convenience prelude for command spec files.
///
/// `use crate::prelude::*;` in each command file brings in all the
/// types needed to construct a `CommandSpec`.
pub mod prelude {
    pub use crate::abbrev::{KeywordMatch, KeywordTable, PrefixMatching};
    pub use crate::arg_role::{AppendedArity, AppendedAritySet, ArgRole};
    pub use crate::arity::Arity;
    pub use crate::body_kind::BodyKind;
    pub use crate::bpf_op::{
        BpfDeclKind, BpfEffects, BpfOpKind, BpfOpSpec, BpfProgTypeSet, BpfScalarWidth,
        BpfVerdictKind,
    };
    pub use crate::byte_array_effect::ByteArrayEffect;
    pub use crate::clause_shape::{ClauseShapeChecker, ClauseShapeError};
    pub use crate::command_table::CommandTableEffect;
    pub use crate::completion::{
        CompletionCode, CompletionCodeDomain, CompletionDescriptor, CompletionPayloadObligation,
        CompletionPayloadObligations,
    };
    pub use crate::definer::{
        DefinerFamily, DefinitionBodyGrammar, MemberKind, MemberRefKind, MemberRetraction,
        MemberSpec, MemberVisibility, RetractionWords,
    };
    pub use crate::deprecation::{DeprecationFixHook, DeprecationFixSafety};
    pub use crate::dialects::DialectSet;
    pub use crate::dispatch_stability::{
        DispatchDependencies, DispatchDependencyComposition, DispatchDependencyDescriptor,
        DispatchDependencyDomain,
    };
    pub use crate::events::{
        ASM_PAYLOAD, BIGIP_EVENT_HANDLER_PRIORITY, CACHE_PAYLOAD, DIAMETER_PAYLOAD,
        DataCollectionAction, DataCollectionOperation, EventHandlerPriority, EventRequirementForm,
        EventRequires, GTP_PAYLOAD, HTTP_COLLECT, HTTP_PAYLOAD, HTTP_RELEASE, MQTT_COLLECT,
        MQTT_PAYLOAD, MQTT_RELEASE, MR_COLLECT, MR_PAYLOAD, MR_RELEASE, REWRITE_PAYLOAD,
        RTSP_COLLECT, RTSP_PAYLOAD, RTSP_RELEASE, SCTP_COLLECT, SCTP_PAYLOAD, SCTP_RELEASE,
        SIP_PAYLOAD, SSL_COLLECT, SSL_PAYLOAD, SSL_RELEASE, TCP_COLLECT, TCP_PAYLOAD, TCP_RELEASE,
        UDP_PAYLOAD, WS_COLLECT, WS_PAYLOAD, WS_RELEASE, XML_PAYLOAD,
    };
    pub use crate::forms::{CommandForm, SubCommandForm};
    pub use crate::frame_effect::{FrameArgLayout, FrameEffectSpec, FrameLevel, FrameLevelWord};
    pub use crate::handle_binding::{
        BoundHandle, HandleBindingSpec, HandleClassSource, HandleKeyword, HandleName,
    };
    pub use crate::hooks::{
        AnalyserHookId, ArgTypeHint, CodegenHookId, LoweringHookId, TclVersion,
        VersionedConstFoldFn,
    };
    pub use crate::hover::{
        ArgValue, FormKind, FormSpec, HoverSnippet, IntegerDomain, OptionArg, OptionArity,
        OptionSpec, OptionValue, OptionValueHook, OptionValueOutcome, first_positional_index,
    };
    pub use crate::intrinsic::IntrinsicId;
    pub use crate::invocation_words::{CommandPrefixArguments, InvocationArguments};
    pub use crate::lifecycle::{Lifecycle, LifecycleState};
    pub use crate::literal_validation::{
        LiteralArgumentIssue, LiteralArgumentIssueReason, LiteralArgumentValidation,
        LiteralValidationDecline,
    };
    pub use crate::patterns::{FormatType, PatternType};
    pub use crate::presentation::ArgPresentation;
    pub use crate::repeated::RepeatedArgLayout;
    pub use crate::representation::RepresentationEffect;
    pub use crate::result_stability::ResultStability;
    pub use crate::scoped::{ScopedCommand, ScopedCommandEnv};
    pub use crate::semantic_operation::{InlineBodyErrorContext, SemanticOperationId};
    pub use crate::side_effects::{
        ConnectionSide, SideEffect, SideEffectTarget, SideSwitchTarget, StorageType,
    };
    pub use crate::spec::{
        BytePayloadSpec, CaseListSpec, CommandSpec, ContextGate, DefaultFormFirstWord,
        InlineCaseClause, ObjectClassSpec, OoContextFact, OptionConstraint, SubCommand,
        SubSubCommand, VersionedArgValue,
    };
    pub use crate::state_transition::{
        CallerFrameSelection, ChildInterpreterSafety, CommandBindingDefinitionKind,
        CommandBindingTransition, InterpreterTransition, NamespaceTransition,
        NamespaceTransitionTarget, ObjectDispatchKind, ObjectDispatchLayer, ObjectDispatchTarget,
        ObjectDispatchTransition, ObjectPrivateNamespace, StateTransition,
        StateTransitionArgumentShape, StateTransitionCommit, StateTransitionComposition,
        StateTransitionDescriptor, StateTransitionDomain, StateTransitionOperandLayout,
        StateTransitionResolver, StateTransitionWideningRule, StateTransitions, TraceOperation,
        TraceOperationSet, TraceTarget, TraceTransition, TransitionSubject, VariableAliasTarget,
        VariableCellAliasTransition,
    };
    pub use crate::symbol_def::{DefinedSymbolKind, SymbolDef};
    pub use crate::taint::{SetterConstraint, TaintColour, TaintColourAtom};
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
pub use body_kind::BodyKind;
pub use byte_array_effect::ByteArrayEffect;
pub use cache::{
    registry_for_dialect, registry_for_profile, registry_for_profile_with_overlay,
    registry_handle_for_dialect, registry_handle_for_profile,
};
pub use clause_shape::{ClauseShapeChecker, ClauseShapeError};
pub use command_table::CommandTableEffect;
pub use completion::{
    CompletionCode, CompletionCodeDomain, CompletionDescriptor, CompletionPayloadObligation,
    CompletionPayloadObligations,
};
pub use dialects::{
    DETECT_SCAN_BYTES, KNOWN_DIALECTS, available_dialects, detect_dialect,
    detect_dialect_directive, detect_dialect_from_source, dialect_from_extension,
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
pub use frame_effect::{FrameArgLayout, FrameEffectSpec, FrameLevel, FrameLevelWord};
pub use handle_binding::{
    BoundHandle, HandleBindingSpec, HandleClassSource, HandleKeyword, HandleName,
};
pub use hover::ArgValue;
pub use intrinsic::IntrinsicId;
pub use invocation_words::{
    CommandPrefixArguments, InvocationArguments, InvocationWord, InvocationWordKind,
    InvocationWords,
};
pub use literal_validation::{
    LiteralArgumentIssue, LiteralArgumentIssueReason, LiteralArgumentValidation,
    LiteralArgumentValidator, LiteralValidationDecline,
};
pub use patterns::{FormatType, PatternType};
pub use presentation::ArgPresentation;
pub use profile_queries::{ProfileQueries, VendorSurface};
pub use registry::{
    CommandRegistry, FormatStringArg, MethodDispatchKind, ResolvedCall, ResolvedTerminator,
};
pub use repeated::RepeatedArgLayout;
pub use representation::RepresentationEffect;
pub use resolved_invocation::{
    InvocationFacts, InvocationOptions, InvocationResolutionUnresolved, InvocationSemantics,
    OwnedSubcommandResolution, ResolvedForm, ResolvedInvocation, ResolvedSubcommand,
    StructuredInvocationResolution, SubcommandResolution, SubcommandResolutionKind,
};
pub use result_stability::ResultStability;
pub use semantic_operation::{InlineBodyErrorContext, SemanticOperationId};
pub use side_effects::SideSwitchTarget;
pub use spec::{
    BytePayloadSpec, CaseListSpec, CommandSpec, ContextGate, DefaultFormFirstWord,
    InlineCaseClause, ObjectClassSpec, OoContextFact, OptionConstraint, SubCommand, SubSubCommand,
    VersionedArgValue,
};
pub use special_vars::{
    SPECIAL_VARS, SpecialVarKey, SpecialVarKind, SpecialVarSpec, VarAccess, VarOrigin,
    is_externally_read, is_special_var, special_var, special_var_in_dialect,
    special_var_read_taint, special_var_write_effect, special_vars_for_dialect,
};
pub use state_transition::{
    AbruptTransitionTransfer, CallerFrameSelection, ChildInterpreterSafety,
    CommandBindingDefinitionKind, CommandBindingTransition, InterpreterTransition,
    NamespaceTransition, NamespaceTransitionTarget, ObjectDispatchKind, ObjectDispatchLayer,
    ObjectDispatchTarget, ObjectDispatchTransition, ObjectPrivateNamespace,
    ResolvedStateTransitions, StateTransition, StateTransitionArgumentShape, StateTransitionCommit,
    StateTransitionComposition, StateTransitionDescriptor, StateTransitionDomain,
    StateTransitionFact, StateTransitionKnowledge, StateTransitionOperandLayout,
    StateTransitionResolver, StateTransitionWidening, StateTransitionWideningRule,
    StateTransitions, TraceOperation, TraceOperationSet, TraceTarget, TraceTransition,
    TransitionSubject, VariableAliasTarget, VariableCellAliasTransition,
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
