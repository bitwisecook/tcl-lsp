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

//! Family-B runtime contract — the state-mutation protocol shared across Tcl
//! runtimes.
//!
//! The emitter↔runtime contract is a *state-mutation protocol*, not a
//! value-passing interface: the runtime is a reified, mutable store (namespace
//! tree, frame stack, variable tables, traces, command table) that compiled or
//! interpreted code reaches into. This crate is the published contract for that
//! store — the completion type, opaque handles, the `CompileService` injection
//! point, and a set of small **role traits** generic over an associated
//! `Value`. It deliberately contains no implementations; a runtime such as the
//! bytecode VM (`tcl-vm`) satisfies it over its own value/storage model.
//!
//! See `docs/design/runtime/family-b-routing.md` §1.

// The value-less vocabulary (the completion `Code`, the generic `Completion<V>`,
// and the opaque arena handles) lives in the dependency-free `tcl-core-types`
// leaf crate. This crate's own only dependency is that leaf — the concrete
// bytecode artifact is kept out of [`CompileService`] (an associated `Module`
// type) precisely so a shared command-core crate (`tcl-cmd-core`) can depend on
// these role traits without pulling in `tcl-bytecode`. Re-exported here so
// existing `tcl_runtime_api::{Code, Completion, NsId, …}` consumers are unaffected.
pub use tcl_core_types::{
    ByteNamespacePath, Code, CommandId, CommandSlot, Completion, FrameId, GLOBAL_FRAME, NameBytes,
    NsId, OoId, ProcedureId, ProcedureIdIssuer, ROOT_NS, VarId,
};

/// Original native script bytes with explicit source-channel semantics.
pub use tcl_lexer::{SourceChannel, SourceImage};

/// Single-use proposals inside a concrete private Native transcript consumer.
pub mod closed_native_consumer;
/// Actual Jim LIFO header reuse with separate exact and neutral observations.
pub mod jim_object_pool;
/// Independent supported-backend C ABI issuance for native hash owners.
pub mod native_c_string_abi;
pub mod native_hash_abi;
pub mod native_list_insertion;
/// Exact object-allocation births, independent of reused header addresses.
pub mod native_object_incarnation;
pub mod native_return_literal;

pub mod native_each_loop;
/// Same-entry backend TclOO allocations, roles and method observations.
pub mod native_oo;
/// Actual original substitution-template compiler purpose.
pub mod native_substitution;

pub mod native_procedure_compilation;
/// Native procedure roles shared by concrete declaration and frame owners.
pub mod native_procedure_roles;
pub use native_procedure_compilation::OriginalProcedureCompileRequest;

/// Direct callbacks registered on actual stable variable cells.
pub mod native_variable_trace;

/// Shared structured identities and one-way static display projections.
pub mod command_identity;
pub mod command_inventory;

/// Standard Tcl return-option construction policy.
pub mod completion_options;

/// Proved compilation failures and their shared native error presentation.
pub mod native_compilation_error;

pub use native_compilation_error::{
    NativeCompilationAdmissionError, NativeCompilationAdmissionScope, NativeCompilationError,
    NativeCompilationErrorCommand, NativeCompilationPreflight,
};

/// Neutral host execution errors, separate from catch-visible completions.
pub mod native_execution_error;
pub use native_execution_error::{
    NativeCompileServiceRefusal, NativeExecutionError, NativeExpressionFailure,
    NativeExpressionRefusal, NativeHostCommandRefusal,
};

pub mod authored_tmm;
/// Authenticated scheduled-activation outcomes, separate from guest errors.
pub mod retained_activation;

/// Shared TIP 348 structured error-stack state and validation.
pub mod error_stack;

/// Jim's native evaluation-frame error capture and raw explicit trace values.
pub mod jim_error_stack;

/// Actual Jim frame storage reuse, independently of activation currency.
pub mod jim_call_frame;
pub mod jim_interpreter;
/// Jim private return counters and owned catch/try metadata.
pub mod jim_return_state;

mod array_access;
mod binding_identity;
mod commands;
mod compile_service;
mod namespaces;
mod procedures;
mod variables;

pub use array_access::{
    ArrayDefaultSetFailure, ArrayDefaultState, ArrayElementRead, ArrayInvalidation,
    ArrayReadFailure, ArrayReadMiss, ArrayTarget, NativeArrayDefaultProtocol,
};
pub use binding_identity::{
    CommandBindingGuard, CommandBindingIdentity, CompiledNamespaceContext, ProcedureBindingIdentity,
};
pub use commands::{
    AliasPrefixLookup, Aliases, Commands, NativeJimAliasLookupFailure,
    NativeJimCommandInventoryKind,
};
pub use compile_service::{
    CommandEvaluation, CompileError, CompileFailure, CompileService, CompileUnit, FatalTail,
    ProcedureCompileTarget, ProcedureCompileTargetBytes, ProcedureDispatch, ReleaseBinding,
    ScriptCommandPlan, ScriptCompileTarget, ScriptCompileTargetBytes, SourceCompilationScope,
    UnitKind,
};
pub use namespaces::Namespaces;
pub use procedures::{
    Introspect, OriginalProcedureBodyValue, ProcBodyBytesQuery, ProcBodyValueQuery,
    ProcDefaultValue, ProcInfo, ProcParam, Procs,
};
pub use variables::{
    FrameLinkOrigin, Frames, Traces, VarStore, VarUnsetError, VariableUpdateResult,
};

/// An owned, byte-preserving script completion for host and embedding
/// boundaries.
///
/// `result` is the Tcl result (the error message for [`Code::Error`]) and
/// `options` is its return-options dict, both projected as their exact Tcl
/// string-representation bytes. Runtime engines must construct this before a
/// host adapter chooses any text encoding; a terminal, JavaScript bridge, or
/// other explicitly textual consumer may decode the byte fields afterwards.
pub type ScriptCompletion = Completion<Vec<u8>>;

#[cfg(test)]
mod command_binding_tests;

#[cfg(test)]
mod procedure_binding_tests;

/// Target-neutral compiler/runtime code-generation ABI descriptors and wasm32
/// transport layout constants.
pub mod codegen_abi;

/// Runtime-issued guards for speculative compiler fast paths.
pub mod guard;

pub mod engine_command_replay;
/// Closed live interpreter snapshots for runtime compilation.
pub mod engine_interpreter_identity;
pub mod native_command_name;
pub mod native_compilation;
pub mod native_compiler_pass;

/// Retained expression grammar and evaluation issuers, separate from CPP facts.
pub mod expression_policy;
pub mod native_literal;
/// Original namespace-name descriptors and independently retained lifecycle state.
pub mod native_namespace_name;

pub use native_compilation::NativeCompilationEntry;
/// What a runtime reports it backs, by command name.
mod backing;
pub use backing::{BackingReport, RegisteredBacking};

/// What a specialised site claims about the spec-pack facts it rests on.
mod site_claim;
pub use site_claim::{BackingKind, PackFactStamp, SiteClaim};

/// What an artefact says about the world it was compiled for, and the
/// runtime's counterpart to compare it against.
pub mod manifest;
pub use manifest::{
    ArtefactIdentityManifest, ManifestDecodeError, ManifestField, Rung, RungSet, RuntimeContext,
};

pub use tcl_syntax::naming::NativeNamespaceReportParts;

pub mod script_source_location;
pub mod variable_destruction;

/// Checked allocation for concrete identity-domain owners.
pub mod checked_counter;
/// Thread-owned host diagnostic output policy, independent of Native authority.
pub mod diagnostic_output;
