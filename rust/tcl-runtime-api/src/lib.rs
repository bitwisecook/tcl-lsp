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
    NsId, OoId, ROOT_NS, VarId,
};

/// Original native script bytes with explicit source-channel semantics.
pub use tcl_lexer::{SourceChannel, SourceImage};

/// Independent supported-backend C ABI issuance for native hash owners.
pub mod native_hash_abi;
pub mod native_return_literal;

pub mod native_each_loop;

/// Native procedure roles shared by concrete declaration and frame owners.
pub mod native_procedure_roles;

/// Direct callbacks registered on actual stable variable cells.
pub mod native_variable_trace;

/// Shared structured identities and one-way static display projections.
pub mod command_identity;

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

pub mod jim_interpreter;
/// Jim private return counters and owned catch/try metadata.
pub mod jim_return_state;

/// An owned, byte-preserving script completion for host and embedding
/// boundaries.
///
/// `result` is the Tcl result (the error message for [`Code::Error`]) and
/// `options` is its return-options dict, both projected as their exact Tcl
/// string-representation bytes. Runtime engines must construct this before a
/// host adapter chooses any text encoding; a terminal, JavaScript bridge, or
/// other explicitly textual consumer may decode the byte fields afterwards.
pub type ScriptCompletion = Completion<Vec<u8>>;

/// A stored variable value and return options retained from its reached read.
/// Consumers apply both fields, including when the update itself succeeds.
#[derive(Debug, Clone)]
pub struct VariableUpdateResult<V> {
    /// Value read back from the retained receiver after the store callbacks.
    pub value: V,
    /// Authentic read metadata carried on the resulting successful completion.
    pub options: V,
}

/// A command-level variable removal rejected by the store.
///
/// Storage-only consumers use [`VarStore::unset`] when they have already
/// performed the Tcl command's policy checks. Command implementations use
/// [`VarStore::unset_command`] so runtimes preserve structured failures such
/// as Tcl 9's immutable-variable rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarUnsetError {
    /// The resolved variable cell is a Tcl 9 `const`.
    IsConstant,
}

/// Why a call-frame link exists.
///
/// Tcl's `info consts` excludes ordinary `global`/`upvar`/`variable` aliases,
/// but includes the automatic instance-variable projections installed for a
/// `TclOO` method when their target is constant. Keeping this on the binding
/// makes that distinction available to every runtime without teaching the
/// shared `info` command core about `TclOO` command names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FrameLinkOrigin {
    /// An ordinary Tcl variable alias.
    #[default]
    Ordinary,
    /// An automatic `TclOO` method-frame instance-variable projection, in a
    /// method whose compiled body has **no** local slot for the name.
    TclOoInstance,
    /// The same projection, in a method whose body *did* compile a local slot
    /// for the name — because it references it (`$pub`, `info exists pub`).
    ///
    /// The link behaves identically; the distinction exists only for
    /// enumeration. C lists a projection the body never mentions and stops
    /// listing one it reads, so `info consts` reports [`Self::TclOoInstance`]
    /// and not this (#2173). A *dynamic* read (`set $n`) compiles no slot and
    /// therefore stays the plain variant.
    TclOoInstanceCompiled,
}

/// Actual C Tcl 9 array-default command recipe, without issuer authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeArrayDefaultProtocol {
    /// TIP 508 storage and operation order in the native Tcl 9 command.
    Tcl9,
}

/// State of the exact variable selected before an array-operation callback.
#[derive(Debug, Clone)]
pub enum ArrayDefaultState<V> {
    /// No selected variable, or its current contents are undefined.
    Undefined,
    /// The selected variable currently contains a scalar or an array element.
    NonArray,
    /// The selected array and its original default object, when defined.
    Array {
        /// Original object retained by the selected physical array cell.
        default: Option<V>,
    },
}

/// Guest failure after the array-default setter's fresh creating lookup.
#[derive(Debug, Clone)]
pub enum ArrayDefaultSetFailure {
    /// The creating lookup selected an array element rather than an array root.
    Element,
    /// The selected root is a defined scalar.
    Scalar,
    /// Native variable lookup failed before either storage classification.
    Lookup(tcl_syntax::naming::NativeVariableDiagnosticProjection),
}

/// One array name located for the duration of an `array` ensemble operation.
///
/// Tcl's `LocateArray` resolves the spelling once before firing an `array`
/// trace. Enumeration continues against that exact cell even when the callback
/// retargets an `upvar` alias; runtimes without stable variable identities use
/// the name-addressed form until they acquire that capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayTarget {
    frame: FrameId,
    name: tcl_core_types::NameBytes,
    cell: Option<VarId>,
}

/// Result of one trace-aware element read performed for `array get`.
///
/// The command snapshots keys from [`ArrayTarget`] but reads each candidate
/// through the source spelling. A callback can therefore make the element
/// disappear without invalidating the walk, or destroy/retype the captured
/// array, which is a command error. Keeping that distinction in the runtime
/// contract prevents the shared command core from guessing storage identity.
#[derive(Debug, Clone)]
pub enum ArrayElementRead<V> {
    /// The selected element still has a value after its read traces. Pointer
    /// runtimes return this with one transient ownership hold already acquired;
    /// the shared command core releases it after list materialisation.
    Value(V),
    /// The element is absent, or its read trace errored. Tcl skips it while the
    /// captured base remains an array; the runtime retains any swallowed trace
    /// completion in its interpreter error state.
    Missing(ArrayReadMiss),
    /// A read trace failed and invalidated the captured base in the same
    /// callback. Unlike an ordinary trace miss, Tcl propagates the callback's
    /// contextual variable-read error rather than replacing it with a generic
    /// array invalidation diagnostic.
    TraceError(ArrayReadFailure),
    /// The array cell captured for the operation is no longer an array.
    ArrayInvalidated(ArrayInvalidation),
}

/// A variable-read trace failure which an `array get` candidate must
/// propagate because that same callback invalidated the captured array.
///
/// The message is already the variable subsystem's contextual `can't read
/// "a(k)": reason` result. The remaining fields preserve the callback's
/// accumulated error trace across the shared command/runtime boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayReadFailure {
    message: Vec<u8>,
    code: Vec<u8>,
    info: Option<Vec<u8>>,
    line: Option<i64>,
}

impl ArrayReadFailure {
    /// Construct a hard traced-read failure.
    #[must_use]
    pub fn new(
        message: impl Into<String>,
        code: Vec<u8>,
        info: Option<Vec<u8>>,
        line: Option<i64>,
    ) -> Self {
        Self::new_bytes(message.into().into_bytes(), code, info, line)
    }

    /// Construct a traced-read failure without projecting its native bytes.
    #[must_use]
    pub fn new_bytes(
        message: Vec<u8>,
        code: Vec<u8>,
        info: Option<Vec<u8>>,
        line: Option<i64>,
    ) -> Self {
        Self {
            message,
            code,
            info,
            line,
        }
    }

    /// Consume the failure into its command-error parts.
    #[must_use]
    pub fn into_parts(self) -> (Vec<u8>, Vec<u8>, Option<Vec<u8>>, Option<i64>) {
        (self.message, self.code, self.info, self.line)
    }
}

/// Completion metadata retained when `array get` swallows one candidate's
/// failed variable read.
///
/// Tcl exposes the variable subsystem's read classification on the surrounding
/// successful completion (`TCL READ VARNAME`, or `TCL LOOKUP VARNAME name`
/// when an operation trace retargeted the source alias away from an array). A
/// callback error also contributes its accumulated trace and source line; a
/// callback that merely removes the element contributes no error trace and
/// does not publish the error globals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayReadMiss {
    code: Vec<u8>,
    info: Option<Vec<u8>>,
    line: Option<i64>,
}

impl Default for ArrayReadMiss {
    fn default() -> Self {
        Self::missing()
    }
}

impl ArrayReadMiss {
    /// A read that found no value after otherwise-successful callbacks.
    #[must_use]
    pub fn missing() -> Self {
        Self {
            code: b"TCL READ VARNAME".to_vec(),
            info: None,
            line: None,
        }
    }

    /// A source-spelling lookup that no longer resolves to an array.
    #[must_use]
    pub fn lookup(error_code: Vec<u8>) -> Self {
        Self {
            code: error_code,
            info: None,
            line: None,
        }
    }

    /// A read whose trace callback errored before `array get` swallowed it.
    #[must_use]
    pub fn trace_error(error_info: Option<Vec<u8>>, error_line: i64) -> Self {
        Self {
            code: b"TCL READ VARNAME".to_vec(),
            info: error_info,
            line: Some(error_line),
        }
    }

    /// Merge a later candidate miss into this operation's retained metadata.
    ///
    /// Tcl exposes the most recent read classification, but an ordinary later
    /// miss has no callback trace of its own and must not erase the earlier
    /// callback's `errorInfo` or source line.
    #[must_use]
    pub fn followed_by(self, later: Self) -> Self {
        Self {
            code: later.code,
            info: later.info.or(self.info),
            line: later.line.or(self.line),
        }
    }
}

/// Why a captured array can no longer supply a candidate element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayInvalidation {
    /// The captured variable is now undefined.
    Unset,
    /// The captured variable is now a defined non-array.
    Retyped,
}

impl ArrayTarget {
    /// Construct a name-addressed target.
    #[must_use]
    pub fn named(frame: FrameId, name: impl Into<String>) -> Self {
        Self {
            frame,
            name: tcl_core_types::NameBytes::from(name.into()),
            cell: None,
        }
    }

    /// Construct a target backed by one stable variable cell.
    #[must_use]
    pub fn cell(frame: FrameId, name: impl Into<String>, cell: VarId) -> Self {
        Self {
            frame,
            name: tcl_core_types::NameBytes::from(name.into()),
            cell: Some(cell),
        }
    }

    /// The frame in which the source spelling was resolved.
    #[must_use]
    pub const fn frame(&self) -> FrameId {
        self.frame
    }

    /// The source spelling used for paths that Tcl deliberately re-resolves.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name
            .try_utf8()
            .expect("byte-valued array target requires name_bytes")
    }

    /// Construct an exact name-addressed target without decoding its operand.
    #[must_use]
    pub fn named_bytes(frame: FrameId, name: impl Into<tcl_core_types::NameBytes>) -> Self {
        Self {
            frame,
            name: name.into(),
            cell: None,
        }
    }

    /// Construct an exact stable-cell target retaining the original spelling.
    #[must_use]
    pub fn cell_bytes(
        frame: FrameId,
        name: impl Into<tcl_core_types::NameBytes>,
        cell: VarId,
    ) -> Self {
        Self {
            frame,
            name: name.into(),
            cell: Some(cell),
        }
    }

    /// Exact source spelling; it does not infer or authenticate the cell.
    #[must_use]
    pub fn name_bytes(&self) -> &[u8] {
        self.name.as_bytes()
    }

    /// The stable reached cell, when the runtime provides one.
    #[must_use]
    pub const fn cell_id(&self) -> Option<VarId> {
        self.cell
    }
}

/// A compiler assumption about one runtime command binding.
///
/// `resolution_namespace` is the unrooted constructed namespace key at the
/// source binding site; `name` is the spelling whose live resolution must be
/// checked there (and may be an alias); `identity` is the registry command
/// implementation the specialised operation was compiled for. Keeping all
/// three parts makes aliases and inlined cross-namespace bodies first-class
/// without teaching a runtime which source names happen to compile specially.
/// Boundary at which a compiled native binding requirement is checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandBindingGuard {
    /// Native Tcl 8.4 compiled hooks capture the selected implementation when
    /// the chunk is admitted; later script mutations do not change that opcode.
    ChunkEntry,
    /// Modern Tcl and Jim check the selected implementation before this
    /// command's argument substitutions execute.
    BeforeArguments,
}

/// Exact lookup context retained by an executable compiler dependency.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompiledNamespaceContext {
    /// An original native namespace owner and incarnation.
    Native(native_compilation::NativeNamespaceContext),
    /// Exact geometry from a source allocation which has no actual token yet.
    ConstructedPath(ByteNamespacePath),
}

impl CompiledNamespaceContext {
    /// Exact component boundaries, independent of all presentation strings.
    #[must_use]
    pub fn path(&self) -> &ByteNamespacePath {
        match self {
            Self::Native(context) => &context.path,
            Self::ConstructedPath(path) => path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandBindingIdentity {
    /// Actual namespace owner and incarnation for a native compilation site.
    /// When present, this is authoritative; the text field is presentation only.
    pub namespace_context: Option<CompiledNamespaceContext>,
    /// Constructed namespace key in which the source binding resolved.
    ///
    /// This uses the runtime ABI's unrooted representation: `""` is global,
    /// `"n"` is `::n`, and every byte of a literal-colon namespace segment is
    /// otherwise preserved.
    pub resolution_namespace: String,
    /// Source binding to resolve in [`Self::resolution_namespace`].
    pub name: String,
    /// Stable registry identity expected at that binding.
    pub identity: String,
    /// Validation boundary selected by the native compilation policy.
    pub guard: CommandBindingGuard,
}

impl CommandBindingIdentity {
    /// Construct a binding requirement in the global namespace.
    #[must_use]
    pub fn new(name: impl Into<String>, identity: impl Into<String>) -> Self {
        Self {
            namespace_context: None,
            resolution_namespace: String::new(),
            name: name.into(),
            identity: identity.into(),
            guard: CommandBindingGuard::BeforeArguments,
        }
    }

    /// Construct a binding requirement in an unrooted constructed namespace.
    #[must_use]
    pub fn in_namespace(
        resolution_namespace: impl Into<String>,
        name: impl Into<String>,
        identity: impl Into<String>,
    ) -> Self {
        Self {
            namespace_context: None,
            resolution_namespace: resolution_namespace.into(),
            name: name.into(),
            identity: identity.into(),
            guard: CommandBindingGuard::BeforeArguments,
        }
    }

    /// Retain the actual namespace incarnation used by native source lookup.
    #[must_use]
    pub fn with_native_namespace(
        mut self,
        context: Option<native_compilation::NativeNamespaceContext>,
    ) -> Self {
        self.namespace_context = context.map(CompiledNamespaceContext::Native);
        self
    }

    /// Retain an exact native or source-allocated lookup context.
    #[must_use]
    pub fn with_namespace_context(mut self, context: Option<CompiledNamespaceContext>) -> Self {
        self.namespace_context = context;
        self
    }

    /// Select the native admission/dispatch boundary for this requirement.
    #[must_use]
    pub fn with_guard(mut self, guard: CommandBindingGuard) -> Self {
        self.guard = guard;
        self
    }

    /// Construct a binding requirement from the compiler's rooted constructed
    /// namespace representation.
    ///
    /// Exactly one global-root marker is removed. This must not use written
    /// Tcl name canonicalisation: an unrooted key may itself begin with colons
    /// because they can be literal namespace-segment bytes.
    #[must_use]
    pub fn in_rooted_namespace(
        resolution_namespace: &str,
        name: impl Into<String>,
        identity: impl Into<String>,
    ) -> Self {
        Self::in_namespace(
            resolution_namespace
                .strip_prefix("::")
                .unwrap_or(resolution_namespace),
            name,
            identity,
        )
    }
}

#[cfg(test)]
mod command_binding_tests {
    use super::CommandBindingIdentity;

    #[test]
    fn rooted_constructed_namespace_loses_exactly_one_root_marker() {
        assert_eq!(
            CommandBindingIdentity::in_rooted_namespace("::n", "expr", "expr").resolution_namespace,
            "n",
        );
        assert_eq!(
            CommandBindingIdentity::in_rooted_namespace(":::", "expr", "expr").resolution_namespace,
            ":",
            "a literal-colon namespace segment must not be canonicalised as Tcl source",
        );
    }
}

/// A compiler assumption about one exact user-procedure binding.
///
/// Unlike [`CommandBindingIdentity`], this is not a registry implementation:
/// `resolution_namespace` and `invocation_name` identify the source binding
/// whose call the compiler erased, while `name` is the canonical rooted
/// constructed command key selected there. `parameters` and `body` identify
/// the source definition copied into the caller. A runtime may execute that
/// caller only while the source binding still resolves to that exact command
/// key and it still holds an equivalent user procedure.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcedureBindingIdentity {
    /// Actual source lookup namespace, independent of its printed spelling.
    pub namespace_context: Option<CompiledNamespaceContext>,
    /// Constructed namespace key in which the source invocation resolved.
    ///
    /// This uses the runtime ABI's unrooted representation: `""` is global,
    /// `"n"` is `::n`, and literal namespace-segment bytes are preserved.
    pub resolution_namespace: String,
    /// Source invocation to resolve in [`Self::resolution_namespace`].
    pub invocation_name: String,
    /// Canonical rooted constructed procedure name (for example `::ns::p`).
    pub name: String,
    /// Raw formal-parameter list value, including defaults.
    pub parameters: String,
    /// Raw procedure body value copied by the inliner.
    pub body: String,
}

impl ProcedureBindingIdentity {
    /// Construct an exact user-procedure binding requirement in the global
    /// namespace.
    #[must_use]
    pub fn new(
        invocation_name: impl Into<String>,
        name: impl Into<String>,
        parameters: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self::in_namespace("", invocation_name, name, parameters, body)
    }

    /// Construct an exact user-procedure binding requirement in an unrooted
    /// constructed namespace.
    #[must_use]
    pub fn in_namespace(
        resolution_namespace: impl Into<String>,
        invocation_name: impl Into<String>,
        name: impl Into<String>,
        parameters: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            namespace_context: None,
            resolution_namespace: resolution_namespace.into(),
            invocation_name: invocation_name.into(),
            name: name.into(),
            parameters: parameters.into(),
            body: body.into(),
        }
    }

    /// Construct a requirement from the compiler's rooted constructed
    /// namespace representation.
    ///
    /// Exactly one global-root marker is removed. Written Tcl name
    /// canonicalisation must not be applied to a constructed namespace key.
    #[must_use]
    pub fn in_rooted_namespace(
        resolution_namespace: &str,
        invocation_name: impl Into<String>,
        name: impl Into<String>,
        parameters: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self::in_namespace(
            resolution_namespace
                .strip_prefix("::")
                .unwrap_or(resolution_namespace),
            invocation_name,
            name,
            parameters,
            body,
        )
    }

    /// Retain the actual namespace incarnation used by the source invocation.
    #[must_use]
    pub fn with_native_namespace(
        mut self,
        context: Option<native_compilation::NativeNamespaceContext>,
    ) -> Self {
        self.namespace_context = context.map(CompiledNamespaceContext::Native);
        self
    }

    /// Retain an exact native or source-allocated lookup context.
    #[must_use]
    pub fn with_namespace_context(mut self, context: Option<CompiledNamespaceContext>) -> Self {
        self.namespace_context = context;
        self
    }
}

#[cfg(test)]
mod procedure_binding_tests {
    use super::ProcedureBindingIdentity;

    #[test]
    fn rooted_constructed_namespace_loses_exactly_one_root_marker() {
        let binding =
            ProcedureBindingIdentity::in_rooted_namespace("::n", "p", "::n::p", "", "return ok");
        assert_eq!(binding.resolution_namespace, "n");
        assert_eq!(binding.invocation_name, "p");
        assert_eq!(binding.name, "::n::p");

        assert_eq!(
            ProcedureBindingIdentity::in_rooted_namespace(":::", "p", ":::::p", "", "return ok",)
                .resolution_namespace,
            ":",
            "a literal-colon namespace segment must not be canonicalised as Tcl source",
        );
    }
}

/// Target-neutral compiler/runtime code-generation ABI descriptors and wasm32
/// transport layout constants.
pub mod codegen_abi;

/// Runtime-issued guards for speculative compiler fast paths.
pub mod guard;

pub mod native_command_name;
/// Closed live interpreter snapshots for runtime compilation.
pub mod native_compilation;
pub mod native_literal;
/// Original namespace-name descriptors and independently retained lifecycle state.
pub mod native_namespace_name;

pub use native_compilation::NativeCompilationEntry;

// -- Compile service (the EVAL_STK / dynamic-code injection point) --

/// A compilation failure surfaced by [`CompileService`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// Ordinary source validation diagnostic from the compile service.
    Message(String),
    /// The compile service cannot honour the requested execution contract.
    /// This operational refusal cannot become a Tcl completion.
    Unsupported(String),
    /// The native compiler provider has not admitted the compilation boundary.
    /// This is a host refusal and cannot become a Tcl completion.
    NativeCompilationAdmission(NativeCompilationAdmissionError),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(message) | Self::Unsupported(message) => formatter.write_str(message),
            Self::NativeCompilationAdmission(error) => std::fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for CompileError {}

/// The source-level context required to compile a Tcl procedure body.
///
/// Procedure bodies are not scripts: unqualified variables live in a local
/// variable table seeded by the formal parameter names, `return` terminates a
/// procedure activation, and command resolution starts in the procedure's
/// namespace. Keeping this target typed prevents a runtime compiler from
/// accidentally compiling a body through the top-level script entry point.
#[derive(Debug, Clone, Copy)]
pub struct ProcedureCompileTarget<'a> {
    /// Body source, with offsets relative to the body itself.
    pub source: &'a str,
    /// Formal parameter names in declaration order (defaults are a runtime
    /// binding concern and do not affect bytecode generation).
    pub parameters: &'a [String],
    /// Canonical unrooted constructed namespace key in which the procedure
    /// body resolves commands. The empty string denotes the global namespace;
    /// this is an identity, not a written Tcl name, so a literal `:` segment is
    /// retained verbatim.
    pub namespace: &'a str,
}

/// Which source bodies a compilation request prepares.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SourceCompilationScope {
    /// Authored analysis and AOT artifacts include deferred declaration bodies.
    #[default]
    WholeModule,
    /// Prepare the entered source; declaration bodies remain original values
    /// until their own activation requests compilation.
    EnteredSource,
}

/// A runtime script together with the namespace in which its command words
/// resolve.
///
/// Dynamic `eval`/command-substitution bodies are script frames, not procedure
/// frames, but their compiler provenance still needs the exact constructed
/// namespace. Keeping this target typed prevents a compiler from silently
/// stamping global binding assumptions onto a namespaced evaluation.
#[derive(Debug, Clone, Copy)]
pub struct ScriptCompileTarget<'a> {
    /// Script source, with offsets relative to this string.
    pub source: &'a str,
    /// Canonical unrooted constructed namespace key. Empty denotes global.
    pub namespace: &'a str,
}

/// A native script in its exact constructed byte namespace.
///
/// The namespace contains already selected segments. No display spelling is
/// reparsed, and the source image retains original bytes and channel policy.
#[derive(Debug, Clone, Copy)]
pub struct ScriptCompileTargetBytes<'a> {
    /// Immutable original script source, relative to its first byte.
    pub source: &'a SourceImage,
    /// Constructed command-resolution namespace; an empty path is root.
    pub namespace: &'a tcl_core_types::ByteNamespacePath,
}

/// A native procedure body with the actual bound formal storage keys.
#[derive(Debug, Clone, Copy)]
pub struct ProcedureCompileTargetBytes<'a> {
    /// Immutable original body bytes, without a Unicode or escaped surrogate.
    pub source: &'a SourceImage,
    /// Exact local storage keys in declaration order; defaults stay runtime values.
    pub parameters: &'a [tcl_core_types::NameBytes],
    /// Constructed command-resolution namespace, never recovered from display.
    pub namespace: &'a tcl_core_types::ByteNamespacePath,
}

/// Command-dispatch form requested for a procedure-body compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcedureDispatch {
    /// Registry-specialised bytecode guarded by its command-binding summary.
    Optimised,
    /// Ordinary runtime dispatch for every command.
    Plain,
}

/// Compiler-owned command-at-a-time parse plan for a runtime script.
///
/// Tcl parses scripts one command at a time.  A malformed command therefore
/// does not prevent earlier complete commands from running; after that prefix
/// completes normally, the malformed tail raises its parse error without
/// performing substitutions.  The compiler owns the lexer grammar needed to
/// find that boundary, while runtimes own execution, so this small value is
/// the seam between them.
#[derive(Debug, Clone)]
pub struct ScriptCommandPlan {
    /// Byte length of the complete-command prefix in the original source.
    /// Native byte source requires no Unicode character boundary.
    pub complete_prefix_len: usize,
    /// How many complete commands that prefix holds.
    ///
    /// Not derivable from `complete_prefix_len`: the prefix of a script whose
    /// *first* command is malformed still spans any leading whitespace and
    /// comments, so a nonzero length can carry **no** command at all.  A
    /// runtime distinguishing "ran nothing" from "ran something" must test
    /// this rather than the byte length.
    pub complete_prefix_commands: usize,
    /// Parse error raised if the complete prefix finishes normally.
    pub fatal_tail: Option<FatalTail>,
}

/// The parse error a [`ScriptCommandPlan`]'s malformed tail raises, with the
/// context C logs the `while executing` frame from.
///
/// The message alone is not enough: native error contexts retain the original
/// command extent and its line. Compilation and runtime reporting can select
/// different extents from the same parser geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FatalTail {
    /// The Tcl parse message (`missing "`, `missing close-brace`, …).
    pub message: String,
    /// Original source extent selected by C's runtime parse-error logger.
    ///
    /// C slices `source[commandStart ..= parsePtr->term]` — through the
    /// character that opened the unterminated construct, **not** to the end
    /// of the source. Truncation to 150 bytes is the logger's job, not this
    /// value's.
    pub command_text: Vec<u8>,
    /// Independently proved C8.4 compilation context. Its full remaining-source
    /// extent differs from the runtime logger's inclusive-terminator extent.
    /// Absence grants no compiler presentation; an empty proved extent is valid.
    pub compilation_command_text: Option<Vec<u8>>,
    /// One-based line of the malformed command's first byte, for the
    /// enclosing `(procedure …)` / `("eval" body line N)` frames.
    pub line: u32,
}

impl FatalTail {
    /// A tail carrying only its message, for a caller with no source context.
    ///
    /// No source extent or compilation context is proved by this constructor.
    /// This differs from a proved, genuinely empty compilation extent.
    #[must_use]
    pub fn message_only(message: String) -> Self {
        Self {
            message,
            command_text: Vec::new(),
            compilation_command_text: None,
            line: 0,
        }
    }
}

impl ScriptCommandPlan {
    /// A clean script whose whole source is executable.
    ///
    /// `commands` is left unset (`usize::MAX` would be a lie and zero would
    /// claim nothing runs), so this constructor takes it explicitly where the
    /// count is known; [`Self::complete`] is for the no-cut case, where the
    /// distinction the count exists for cannot arise.
    #[must_use]
    pub fn complete(source_len: usize) -> Self {
        Self {
            complete_prefix_len: source_len,
            // No cut, so every command in the source is in the prefix. The
            // exact count is not needed: callers consult it only to tell an
            // empty prefix from a non-empty one before a fatal tail, and
            // there is no fatal tail here.
            complete_prefix_commands: usize::from(source_len > 0),
            fatal_tail: None,
        }
    }
}

/// Compiles a Tcl source string to a runtime-executable module at runtime.
///
/// `eval`/`uplevel`/dynamic command names compile a string while the program
/// runs, so a VM that supports them needs a compiler available during
/// execution. Injecting it as a trait keeps the VM crate lean and
/// compiler-optional: the embedder wires a real (`tcl-compiler`-backed)
/// implementation; a program that never hits `eval` can use a stub. Mirrors C
/// Tcl always carrying its bytecode compiler.
///
/// The produced module is an associated type (e.g. the VM sets it to
/// `tcl_bytecode::ModuleAsm`) so this contract crate stays free of any concrete
/// bytecode dependency — see the crate-level note.
pub trait CompileService {
    /// The runtime-executable artifact produced (the bytecode VM's `ModuleAsm`).
    type Module;

    /// Compile original native script bytes in their constructed namespace.
    /// A service must implement byte ingress explicitly; decoding or escaping
    /// opaque source would change names and literal objects.
    fn compile_script_bytes_for_profile(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        let _ = (target, profile);
        Err(CompileError::Unsupported(
            "CompileService does not support original byte scripts".into(),
        ))
    }

    /// Compile byte scripts with ordinary dispatch for every command.
    fn compile_plain_script_bytes_for_profile(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        let _ = (target, profile);
        Err(CompileError::Unsupported(
            "CompileService does not support plain byte scripts".into(),
        ))
    }

    /// Compile byte scripts against the retained actual interpreter entry.
    /// The entry's physical compiler and logical policy remain independent.
    fn compile_script_bytes_with_entry(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<Self::Module, CompileError> {
        let _ = (target, profile, entry);
        Err(CompileError::Unsupported(
            "CompileService does not support retained-entry byte scripts".into(),
        ))
    }

    /// Compile byte scripts with ordinary dispatch under the exact native entry.
    /// Services must preserve actual compiler prerequisites even when semantic
    /// command specializations are disabled.
    fn compile_plain_script_bytes_with_entry(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<Self::Module, CompileError> {
        let _ = (target, profile, entry);
        Err(CompileError::Unsupported(
            "CompileService does not support retained-entry plain byte scripts".into(),
        ))
    }

    /// Compile byte procedure bodies using actual formal storage identities.
    fn compile_procedure_bytes_for_profile(
        &self,
        target: ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        let _ = (target, profile, dispatch);
        Err(CompileError::Unsupported(
            "CompileService does not support byte procedure bodies".into(),
        ))
    }

    /// Compile a byte procedure against the retained actual interpreter entry.
    fn compile_procedure_bytes_with_entry(
        &self,
        target: ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        let _ = (target, profile, entry, dispatch);
        Err(CompileError::Unsupported(
            "CompileService does not support retained-entry byte procedures".into(),
        ))
    }

    /// Locate a byte script's complete-command prefix before executing it.
    /// A service with no native byte parser refuses instead of reporting an
    /// encoding failure as an empty, successfully parsed script.
    fn script_command_plan_bytes_for_profile(
        &self,
        source: &SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<ScriptCommandPlan, CompileError> {
        let _ = (source, profile);
        Err(CompileError::Unsupported(
            "CompileService does not expose its native byte parser".into(),
        ))
    }

    /// Locate complete byte commands using the actual compiler-entry grammar.
    /// A supplied entry cannot be replaced by a catalogue grammar guess.
    fn script_command_plan_bytes_with_entry(
        &self,
        source: &SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<ScriptCommandPlan, CompileError> {
        let _ = (source, profile, entry);
        Err(CompileError::Unsupported(
            "CompileService does not expose entry-aware byte parsing".into(),
        ))
    }

    /// Compile `src` to a [`Module`](Self::Module), or report why it could not.
    fn compile(&self, src: &str) -> Result<Self::Module, CompileError>;

    /// Compile `src` for the dialect profile currently selected by the
    /// interpreter. A VM may change profile after it has cached dynamic
    /// bodies, so reusing a compiler constructed for an older registry/grammar
    /// is not sound: an unavailable command could already have been lowered to
    /// bytecode and bypass normal command dispatch.
    ///
    /// Profile-aware services must override this and select the profile's
    /// registry, lexer grammar, and expression dialect for this invocation.
    /// A fixed-profile compiler is permitted only for the permissive fallback;
    /// named-profile dynamic compilation is rejected rather than silently
    /// stamping older bytecode as current.
    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        if profile.is_fallback() {
            self.compile(src)
        } else {
            Err(CompileError::Unsupported(format!(
                "CompileService does not support dialect profile {}",
                profile.name
            )))
        }
    }

    /// Compile a runtime script in its exact command-resolution namespace.
    ///
    /// The root-namespace default preserves existing compiler services. A
    /// non-root target fails closed because delegating to
    /// [`Self::compile_for_profile`] would attach incorrect global binding
    /// provenance to namespaced bytecode.
    fn compile_script_for_profile(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        if target.namespace.is_empty() {
            self.compile_for_profile(target.source, profile)
        } else {
            Err(CompileError::Unsupported(
                "CompileService does not support namespaced script compilation".to_string(),
            ))
        }
    }

    /// Compile `src` with every registry-driven inline/structured lowering
    /// hook suppressed: every command compiles to a plain dispatch, so
    /// execution traces — including `enterstep`/`leavestep` step traces —
    /// observe it (C Tcl's `DONT_COMPILE_CMDS_INLINE`, `tclTrace.c`; a step
    /// trace forces the traced proc "out of bytecode" so no inner command,
    /// including `set`/`incr`/`if`/`while`, is invisible to the trace).
    ///
    /// Used to recompile a proc's (or any dynamically-evaluated script's)
    /// body once a step-capable execution trace targets it, and reverted the
    /// same way once the last such trace is removed — the VM never leaves a
    /// proc permanently de-optimised. The default fails closed: a compiler
    /// service must explicitly implement plain dispatch before the runtime can
    /// safely recover from command mutation or expose step-visible execution.
    fn compile_traced(&self, src: &str) -> Result<Self::Module, CompileError> {
        let _ = src;
        Err(CompileError::Unsupported(
            "CompileService does not support plain command dispatch".to_string(),
        ))
    }

    /// Profile-aware counterpart of [`Self::compile_traced`]. See
    /// [`Self::compile_for_profile`] for why dynamic recompilation must select
    /// the VM's current profile rather than a compiler's construction-time
    /// profile. The default follows the same fixed-profile rejection rule.
    fn compile_traced_for_profile(
        &self,
        src: &str,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        if profile.is_fallback() {
            self.compile_traced(src)
        } else {
            Err(CompileError::Unsupported(format!(
                "CompileService does not support dialect profile {}",
                profile.name
            )))
        }
    }

    /// Compile `src` with command invocations preserved as ordinary runtime
    /// dispatches for `profile`.
    ///
    /// This is the semantic name for the de-optimised form shared by two
    /// runtime conditions: step-capable execution traces must observe every
    /// command, and a command-table mutation may have replaced a builtin that
    /// an optimised unit would otherwise bypass. Both require exactly the
    /// same compiler contract, so the runtime selects one path rather than
    /// maintaining parallel trace and mutation compilers.
    ///
    /// The existing trace-aware method remains the implementation seam for
    /// compile services that already override it. Its default is deliberately
    /// fail-closed: an optimising compiler must explicitly provide this
    /// capability before the runtime may use it for invalidation recovery.
    fn compile_plain_dispatch_for_profile(
        &self,
        src: &str,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        self.compile_traced_for_profile(src, profile)
    }

    /// Plain-dispatch counterpart of [`Self::compile_script_for_profile`].
    /// The same fail-closed namespace rule applies even though the returned
    /// artifact must have no specialised command dependencies: structured
    /// body lowering and nested definitions still consume the script context.
    fn compile_plain_script_for_profile(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        if target.namespace.is_empty() {
            self.compile_plain_dispatch_for_profile(target.source, profile)
        } else {
            Err(CompileError::Unsupported(
                "CompileService does not support namespaced plain script compilation".to_string(),
            ))
        }
    }

    /// Compile against actual live interpreter bindings. A service without
    /// this capability must use explicit plain dispatch, never infer a fresh
    /// stock table from catalogue availability.
    fn compile_script_with_entry(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<Self::Module, CompileError> {
        let _ = entry;
        self.compile_plain_script_for_profile(target, profile)
    }

    /// Compile a procedure against actual live interpreter entry facts.
    /// Unsupported entry-aware services retain only explicit plain dispatch.
    fn compile_procedure_with_entry(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        let _ = (entry, dispatch);
        self.compile_procedure_for_profile(target, profile, ProcedureDispatch::Plain)
    }

    /// Locate the complete-command prefix and optional fatal parse tail of a
    /// runtime script under `profile`'s exact lexer grammar.
    ///
    /// The default preserves compatibility with compile services that do not
    /// expose their parser: the later whole-script compile remains their error
    /// boundary. Compiler-backed services should override this so `catch` and
    /// `try` can execute a valid prefix before observing a malformed tail.
    ///
    /// # Errors
    /// Returns unavailable lexical ownership or invalid original-source
    /// geometry separately from a guest parse tail in a successful plan.
    fn script_command_plan_for_profile(
        &self,
        src: &str,
        _profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<ScriptCommandPlan, CompileError> {
        Ok(ScriptCommandPlan::complete(src.len()))
    }

    /// Compile a procedure body for an exact dialect profile and dispatch
    /// mode. The default deliberately fails closed: silently delegating to
    /// [`Self::compile_for_profile`] would create script-context bytecode with
    /// no parameter LVT and incorrect `return`/local-variable semantics.
    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        let _ = (target, profile, dispatch);
        Err(CompileError::Unsupported(
            "CompileService does not support procedure-body compilation".to_string(),
        ))
    }
}

// -- Family-B role traits --
//
// Small, composable traits over an associated `Value`, each mirroring a
// `runtime/rust` storage module. A consumer depends only on the subset it
// needs; do not collapse them into one umbrella `Interp` trait. Impls grow
// over time; the trait surface is the contract.

/// Variable storage: scalars, arrays, and `upvar`/`global`/`variable` links,
/// addressed by call frame. Corresponds to `frame.rs`'s `Var`/`VarTable`.
pub trait VarStore {
    /// The runtime's value type.
    type Value;

    /// Read a scalar variable in `frame`, following links.
    fn get(&self, frame: FrameId, name: &str) -> Option<Self::Value>;
    /// Write a scalar variable in `frame` (firing write traces).
    fn set(&mut self, frame: FrameId, name: &str, value: Self::Value);
    /// Remove a variable in `frame`; returns whether it existed.
    fn unset(&mut self, frame: FrameId, name: &str) -> bool;
    /// Remove a variable as a Tcl command operation, preserving policy errors.
    ///
    /// An absent variable is not an error and returns `Ok(false)`. The default
    /// keeps storage implementations source-compatible; runtimes with
    /// immutable bindings override it to reject them before mutation.
    fn unset_command(&mut self, frame: FrameId, name: &str) -> Result<bool, VarUnsetError> {
        Ok(self.unset(frame, name))
    }
    /// Whether a variable exists in `frame`.
    fn exists(&self, frame: FrameId, name: &str) -> bool;

    /// Present a reached physical variable failure through the adapter's
    /// independently authenticated naming issuer. Input parts and failure site
    /// remain separate from the selected binding/cell identity.
    fn variable_diagnostic_at(
        &self,
        operation: tcl_syntax::naming::NativeVariableDiagnosticOperation,
        reason: tcl_syntax::naming::NativeVariableDiagnosticReason,
        site: tcl_syntax::naming::NativeVariableFailureSite,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
    ) -> Result<tcl_syntax::naming::NativeVariableDiagnosticProjection, tcl_syntax::value::ValueError>
    {
        let _ = (operation, reason, site, input);
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "variable diagnostic",
        ))
    }

    /// Read an exact combined native name. Unicode-only adapters refuse an
    /// unrepresentable input before performing any lookup.
    fn get_bytes(
        &self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<Option<Self::Value>, tcl_syntax::value::ValueError> {
        let name = tcl_syntax::raw_string::RawString::from_bytes(name).unicode()?;
        Ok(self.get(frame, &name))
    }
    /// Write an exact combined native name, without repairing its spelling.
    fn set_bytes(
        &mut self,
        frame: FrameId,
        name: &[u8],
        value: Self::Value,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let name = tcl_syntax::raw_string::RawString::from_bytes(name).unicode()?;
        self.set(frame, &name, value);
        Ok(())
    }
    /// Remove an exact combined native name.
    fn unset_bytes(
        &mut self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        let name = tcl_syntax::raw_string::RawString::from_bytes(name).unicode()?;
        Ok(self.unset(frame, &name))
    }
    /// Command removal with operational access refusal outside the guest
    /// immutable-binding outcome.
    fn unset_command_bytes(
        &mut self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<Result<bool, VarUnsetError>, tcl_syntax::value::ValueError> {
        let name = tcl_syntax::raw_string::RawString::from_bytes(name).unicode()?;
        Ok(self.unset_command(frame, &name))
    }
    /// Test an exact combined native name.
    fn exists_bytes(
        &self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        let name = tcl_syntax::raw_string::RawString::from_bytes(name).unicode()?;
        Ok(self.exists(frame, &name))
    }
    /// Read native root and element operands without reconstructing a combined
    /// name. The adapter selects each operand's native name protocol.
    fn get_elem_bytes(
        &self,
        frame: FrameId,
        root: &[u8],
        element: &[u8],
    ) -> Result<Option<Self::Value>, tcl_syntax::value::ValueError> {
        let root = tcl_syntax::raw_string::RawString::from_bytes(root).unicode()?;
        let element = tcl_syntax::raw_string::RawString::from_bytes(element).unicode()?;
        Ok(self.get_elem(frame, &root, &element))
    }
    /// Write native root and element operands separately.
    fn set_elem_bytes(
        &mut self,
        frame: FrameId,
        root: &[u8],
        element: &[u8],
        value: Self::Value,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let root = tcl_syntax::raw_string::RawString::from_bytes(root).unicode()?;
        let element = tcl_syntax::raw_string::RawString::from_bytes(element).unicode()?;
        self.set_elem(frame, &root, &element, value);
        Ok(())
    }

    // Array-element access. The `name` is the array *base* and `key` the element,
    // already split from `base(key)` — runtimes differ on whether the by-name
    // accessors parse `a(k)`, so element ops are explicit. Implementations must
    // preserve this pair through storage resolution: recomposing `base(key)` and
    // parsing it again is ambiguous when the base itself contains `(`. Mirror
    // the scalar ops.

    /// Read array element `name(key)` in `frame`, following links.
    fn get_elem(&self, frame: FrameId, name: &str, key: &str) -> Option<Self::Value>;
    /// Write array element `name(key)` in `frame` (firing write traces).
    fn set_elem(&mut self, frame: FrameId, name: &str, key: &str, value: Self::Value);
    /// Remove array element `name(key)`; returns whether it existed.
    fn unset_elem(&mut self, frame: FrameId, name: &str, key: &str) -> bool;
    /// Whether array element `name(key)` exists.
    fn exists_elem(&self, frame: FrameId, name: &str, key: &str) -> bool;

    /// The element keys of array `name` in `frame`, in the runtime's storage
    /// order, or `None` if `name` is not an array (a scalar, or unset). An array
    /// with no elements yields `Some(vec![])` — the existence signal `array
    /// exists`/`info exists` need. This is the **enumeration** surface the
    /// otherwise-deliberately-listing-free state traits expose for the `array`
    /// family (`names`/`get`/`size`/`exists`/`unset`).
    fn array_keys(&self, frame: FrameId, name: &str) -> Option<Vec<String>>;

    /// Root storage protocol of this variable adapter. The legacy trait contract
    /// uses distinct C Tcl scalar/array cells; dictionary-valued adapters must
    /// select their protocol explicitly.
    fn variable_container_model(&self) -> tcl_dialect::VariableContainerModel {
        tcl_dialect::VariableContainerModel::DistinctArray
    }

    /// Enumerate an array value, preserving dictionary decoding failures.
    /// Distinct scalar/array stores return ordinary non-array absence; a
    /// dictionary-valued store distinguishes malformed contents from absence.
    fn array_keys_checked_at(
        &self,
        target: &ArrayTarget,
    ) -> Result<Option<Vec<String>>, tcl_syntax::value::ValueError> {
        Ok(self.array_keys_at(target))
    }

    /// Enumerate exact key bytes from the captured array. Unicode-only stores
    /// inherit their checked key projection; byte-capable stores override it.
    fn array_key_bytes_checked_at(
        &self,
        target: &ArrayTarget,
    ) -> Result<Option<Vec<Vec<u8>>>, tcl_syntax::value::ValueError> {
        self.array_keys_checked_at(target)
            .map(|keys| keys.map(|keys| keys.into_iter().map(String::into_bytes).collect()))
    }

    /// Read an exact key in the captured array, preserving native host refusal.
    /// A store whose key API requires Unicode checks that projection first.
    fn array_read_elem_bytes_at(
        &mut self,
        target: &ArrayTarget,
        key: &[u8],
    ) -> Result<ArrayElementRead<Self::Value>, tcl_syntax::value::ValueError> {
        let key = tcl_syntax::raw_string::RawString::from_bytes(key).unicode()?;
        Ok(self.array_read_elem_at(target, &key))
    }

    /// Locate an array spelling before its operation trace fires. The default
    /// remains name-addressed; a stable-cell runtime overrides this and the
    /// `*_at` methods below so callbacks cannot steer enumeration elsewhere.
    fn array_target(&self, frame: FrameId, name: &str) -> ArrayTarget {
        ArrayTarget::named(frame, name)
    }

    /// Capture an exact native array root before operation callbacks.
    fn array_target_bytes(
        &self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<ArrayTarget, tcl_syntax::value::ValueError> {
        let name = tcl_syntax::raw_string::RawString::from_bytes(name).unicode()?;
        Ok(self.array_target(frame, &name))
    }

    /// Inspect the same physical root after its array-operation callbacks.
    /// Missing native storage support is distinct from an undefined variable.
    fn array_default_state_at(
        &self,
        _target: &ArrayTarget,
    ) -> Result<ArrayDefaultState<Self::Value>, tcl_syntax::value::ValueError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "array default storage",
        ))
    }

    /// Clear the default on the exact selected root without a new name lookup.
    fn unset_array_default_at(
        &mut self,
        _target: &ArrayTarget,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "array default storage",
        ))
    }

    /// Perform the setter's fresh creating lookup after array callbacks.
    /// A successful native C setter owns two references to `value`, returns no
    /// value, and does not fire variable-write callbacks.
    fn set_array_default_bytes(
        &mut self,
        _frame: FrameId,
        _name: &[u8],
        _value: Self::Value,
    ) -> Result<Result<(), ArrayDefaultSetFailure>, tcl_syntax::value::ValueError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "array default storage",
        ))
    }

    /// Enumerate the cell captured by [`array_target`](Self::array_target).
    fn array_keys_at(&self, target: &ArrayTarget) -> Option<Vec<String>> {
        self.array_keys(target.frame(), target.name())
    }

    /// Physical element-table keys captured for an active array search.
    /// Unlike [`array_keys_at`](Self::array_keys_at), this includes attached
    /// undefined cells created by traces or links: Tcl's hash iterator sees
    /// those entries and skips them only when each candidate is reached.
    fn array_search_keys_at(&self, target: &ArrayTarget) -> Option<Vec<String>> {
        self.array_keys_at(target)
    }

    /// Exact physical search keys, with operational refusal outside absence.
    fn array_search_key_bytes_at(
        &self,
        target: &ArrayTarget,
    ) -> Result<Option<Vec<Vec<u8>>>, tcl_syntax::value::ValueError> {
        Ok(self
            .array_search_keys_at(target)
            .map(|keys| keys.into_iter().map(String::into_bytes).collect()))
    }

    /// Test an exact physical search key without firing read callbacks.
    fn array_elem_exists_bytes_at(
        &self,
        target: &ArrayTarget,
        key: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        let key = tcl_syntax::raw_string::RawString::from_bytes(key).unicode()?;
        Ok(self.array_elem_exists_at(target, &key))
    }

    /// Whether one candidate in the captured array currently has a value,
    /// without firing its read trace.
    fn array_elem_exists_at(&self, target: &ArrayTarget, key: &str) -> bool {
        self.exists_elem(target.frame(), target.name(), key)
    }

    /// Structural revision of the captured array cell, when the runtime can
    /// distinguish invalidation of an active Tcl array search. The value
    /// changes on Tcl-level key insertion/removal or an explicit element
    /// unset; defining or replacing an attached element does not invalidate
    /// the search, nor does garbage-collecting an undefined trace shell.
    fn array_revision_at(&self, _target: &ArrayTarget) -> Option<u64> {
        None
    }

    /// Remove an element from the cell captured by
    /// [`array_target`](Self::array_target).
    fn unset_elem_at(&mut self, target: &ArrayTarget, key: &str) -> bool {
        self.unset_elem(target.frame(), target.name(), key)
    }

    /// Remove an exact byte key from the captured array. Unicode-only stores
    /// refuse an unrepresentable key before attempting the mutation.
    fn unset_elem_bytes_at(
        &mut self,
        target: &ArrayTarget,
        key: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        let key = tcl_syntax::raw_string::RawString::from_bytes(key).unicode()?;
        Ok(self.unset_elem_at(target, &key))
    }

    /// Read one `array get` candidate through variable-read traces.
    ///
    /// The default preserves storage-only implementations. Tcl runtimes
    /// override this to pin the live element before callbacks, retain a
    /// swallowed callback error in interpreter metadata, and distinguish a
    /// missing element from destruction of `target`.
    fn array_read_elem_at(
        &mut self,
        target: &ArrayTarget,
        key: &str,
    ) -> ArrayElementRead<Self::Value> {
        let value = self.get_elem(target.frame(), target.name(), key);
        if self.array_keys_at(target).is_none() {
            let invalidation = if self.exists(target.frame(), target.name()) {
                ArrayInvalidation::Retyped
            } else {
                ArrayInvalidation::Unset
            };
            ArrayElementRead::ArrayInvalidated(invalidation)
        } else {
            value.map_or_else(
                || ArrayElementRead::Missing(ArrayReadMiss::missing()),
                ArrayElementRead::Value,
            )
        }
    }
}

/// The call-frame stack: proc-call frames and the `uplevel` active-level dance.
/// Mirrors `runtime/rust`'s `frame.rs` `FrameStack` (`framePtr`/`varFramePtr`).
pub trait Frames {
    /// Push a new call frame whose namespace context is `ns`; returns its id.
    fn push(&mut self, ns: NsId) -> FrameId;
    /// Pop the current call frame.
    fn pop(&mut self);
    /// The current (top) frame.
    fn current(&self) -> FrameId;
    /// Install a link (`upvar`/`global`/`variable`) in `here` to `target`'s
    /// variable `target_name`.
    fn link(&mut self, here: FrameId, target: FrameId, local: &str, target_name: &str);

    // -- active-frame variable enumeration (backs the *frame-local* half of `info
    // vars`/`info locals`, the namespace-free counterpart to `Namespaces::vars_in`).

    /// Whether the active frame is a **procedure** activation (vs the global or a
    /// `namespace eval` frame) — `info vars` lists the frame's own variables in a
    /// proc, the current namespace's variables otherwise (C's `InfoVarsCmd`).
    fn in_proc(&self) -> bool;
    /// The variable names of the **active** frame. Genuine locals (scalars,
    /// arrays) are always included; `upvar`/`global`/`variable` **links** are
    /// included iff `include_links` — `info vars` lists links (by their local
    /// alias), `info locals` does not.
    fn var_names(&self, include_links: bool) -> Vec<String>;

    /// Exact active-frame bindings, preserving every native name.
    fn var_names_bytes(&self, include_links: bool) -> Vec<Vec<u8>> {
        self.var_names(include_links)
            .into_iter()
            .map(String::into_bytes)
            .collect()
    }

    /// Ordered owned frame inventory. Concrete native adapters validate their
    /// selected table protocol before returning names, including duplicate
    /// compiled declarations. The compatibility default preserves the supplied
    /// inventory without claiming an ABI or deriving a native hash order.
    fn var_names_bytes_checked(
        &self,
        include_links: bool,
    ) -> Result<Vec<Vec<u8>>, tcl_syntax::value::ValueError> {
        Ok(self.var_names_bytes(include_links))
    }

    /// The active frame's `info consts` bindings: direct constants plus typed
    /// `TclOO` instance projections whose target is constant. Ordinary link
    /// aliases are excluded even though `info constant alias` follows them.
    fn const_names(&self) -> Vec<String>;

    /// [`const_names`](Self::const_names) without a lossy UTF-8 round trip.
    fn const_names_bytes(&self) -> Vec<Vec<u8>> {
        self.const_names()
            .into_iter()
            .map(String::into_bytes)
            .collect()
    }
}

/// The command table and dispatch: builtins, procs, aliases, imports,
/// ensembles, child interps.'s `interp.rs` `Command`.
pub trait Commands {
    /// The runtime's value type.
    type Value;

    /// Dispatch a command by name with its argv, resolving the name in the
    /// current context.
    fn dispatch(&mut self, name: &str, argv: &[Self::Value]) -> Completion<Self::Value>;

    /// Dispatch a command already resolved to a [`CommandId`] (by
    /// [`Namespaces::find_command`]) with its argv — the resolve-then-invoke
    /// pairing (mirrors Tcl's `Tcl_GetCommandFromObj` + `Tcl_NRCallObjProc`). A
    /// stale or fabricated id yields an error completion. This is what makes a
    /// `CommandId` *do* something: resolve once via `find_command`, invoke here.
    fn dispatch_id(&mut self, cmd: CommandId, argv: &[Self::Value]) -> Completion<Self::Value>;
}

/// The namespace tree and name resolution. (Contract surface; not yet
/// implemented.)
pub trait Namespaces {
    /// Whether imports retain source tokens or resolve source names on use.
    fn namespace_import_binding(&self) -> Option<tcl_dialect::NamespaceImportBinding> {
        None
    }

    /// The full target prefix of an ordinary same-interpreter alias.
    /// Byte-native runtimes preserve the target words verbatim.
    fn command_alias_prefix_bytes(&self, _cmd: CommandId) -> Option<Vec<Vec<u8>>> {
        None
    }

    /// Variable lookup/listing policy of this execution engine, when selected.
    fn variable_lookup_policy(&self) -> Option<tcl_dialect::VariableLookupPolicy> {
        None
    }

    /// Resolve `name` (qualified or unqualified) from context `cxt` to the
    /// command it names, following the `cxt → namespace path → root` order. The
    /// returned handle is invoked via [`Commands::dispatch_id`].
    fn find_command(&self, cxt: NsId, name: &str) -> Option<CommandId>;
    /// The current namespace.
    fn current(&self) -> NsId;
    /// The fully-qualified name of namespace `ns` (`"::"` for the global root) —
    /// what `namespace current` reports.
    fn name(&self, ns: NsId) -> String;
    /// The fully-qualified name a [`CommandId`] names (the inverse of
    /// [`find_command`](Self::find_command)), or `None` for a stale/unknown id —
    /// backs `namespace which`.
    fn command_name(&self, cmd: CommandId) -> Option<String>;

    // -- namespace-tree navigation (mirrors C's `Namespace` struct: `nsId`
    // identity, `parentPtr`, `childTable`). A namespace *is* a handle; its
    // FQN/parent/children are queried from it. `name(ns)` (above) is its
    // `fullName`. These back `namespace exists`/`parent`/`children`.

    /// Resolve namespace `name` (qualified or unqualified) from context `cxt` to
    /// its table handle, or `None` if no such namespace table exists. During
    /// namespace teardown Tcl can retain that table for command enumeration
    /// after the public namespace token is dead; consumers requiring a public
    /// token must also consult [`Namespaces::namespace_is_live`].
    fn find_namespace(&self, cxt: NsId, name: &str) -> Option<NsId>;
    /// Whether a namespace table handle still denotes a public namespace token.
    /// Runtimes without a distinct teardown interval use the default.
    fn namespace_is_live(&self, ns: NsId) -> bool {
        let _ = ns;
        true
    }
    /// The handle of `ns`'s parent (`parentPtr`), or `None` for the global root.
    fn parent(&self, ns: NsId) -> Option<NsId>;
    /// The handles of `ns`'s direct child namespaces (`childTable`) in creation
    /// order.
    fn children(&self, ns: NsId) -> Vec<NsId>;
    /// The same live children in the observable `Tcl_FirstHashEntry` order of
    /// Tcl's retained string-key hash table. Adapters that model only a tree may
    /// use the creation-order default; `TclVM` adapters override this with the
    /// shared hash-table owner so resize and deletion history is preserved.
    fn children_hash_order(&self, ns: NsId) -> Vec<NsId> {
        self.children(ns)
    }

    // -- command enumeration (a namespace's `cmdTable`; backs `info commands`/
    // `info procs`). Direct members only — one level, not descendants — returned
    // as **unqualified** tail names. This is the command-listing **enumeration**
    // surface, the namespace analogue of `VarStore::array_keys`.

    /// The unqualified names of commands defined **directly** in `ns`. Mirrors a
    /// walk of `Namespace.cmdTable`; backs `info commands`.
    fn commands_in(&self, ns: NsId) -> Vec<String>;
    /// The unqualified names of **user procedures** defined directly in `ns` (the
    /// `TclIsProc` subset of [`commands_in`](Self::commands_in)); backs `info procs`.
    fn procs_in(&self, ns: NsId) -> Vec<String>;
    /// The unqualified names of **variables** defined directly in `ns` (a walk of
    /// `Namespace.varTable`); backs `info vars ::ns::*` and `info globals` (the
    /// global namespace's variables). The variable analogue of
    /// [`commands_in`](Self::commands_in).
    fn vars_in(&self, ns: NsId) -> Vec<String>;
    /// The direct `const` bindings in `ns`; link aliases are excluded.
    fn consts_in(&self, ns: NsId) -> Vec<String>;

    // -- resolution accessors the shared `namespace which -variable` / `origin`
    // cores need beyond navigation (`tcl_cmd_core::namespace`).

    /// Does namespace `ns`'s **own** variable table hold an entry named
    /// `simple` (an unqualified name)? This is `Tcl_FindNamespaceVar`'s single
    /// probe: the namespace's `varTable` only — never the call frame, so a
    /// proc local of the same name is invisible here. Backs
    /// [`which_variable`](../tcl_cmd_core/namespace/fn.which_variable.html).
    fn namespace_var_exists(&self, ns: NsId, simple: &str) -> bool;

    /// The command `cmd` was ultimately imported from — C's
    /// `TclGetOriginalCommand`, which is itself the whole walk (`while
    /// (cmdPtr->deleteProc == DeleteImportedCmd) cmdPtr = realCmdPtr`), not a
    /// single hop. `None` when `cmd` is not an imported command.
    ///
    /// The walk stays with the runtime because an import link is a command
    /// *token*, and a runtime whose tokens are name-keyed needs its own
    /// disambiguation (the VM's hidden/visible domains: `interp hide {} a b`
    /// leaves a hidden token `b` whose provenance must not be confused with an
    /// unrelated visible command also called `b`). Backs
    /// [`origin`](../tcl_cmd_core/namespace/fn.origin.html).
    fn command_origin(&self, cmd: CommandId) -> Option<CommandId>;

    // -- byte-valued spellings ------------------------------------------------
    //
    // A Tcl name is a byte string, not text: `set [binary format c 255] 1`
    // names a variable no `&str` can hold without a lossy round trip. The
    // Text adapters may implement the Unicode methods. Their byte defaults
    // perform checked projection and decline unrepresentable names; they never
    // manufacture a replacement spelling. Byte-native adapters override these
    // methods and preserve every stored row in byte enumeration and snapshots.

    /// [`find_command`](Self::find_command) over a byte-valued name.
    fn find_command_bytes(&self, cxt: NsId, name: &[u8]) -> Option<CommandId> {
        core::str::from_utf8(name)
            .ok()
            .and_then(|name| self.find_command(cxt, name))
    }

    /// Resolve an original native command operand while preserving host refusal
    /// separately from a proved missing binding.
    ///
    /// # Errors
    /// The adapter must issue a checked actual-context lookup; the default refuses.
    fn find_command_bytes_checked(
        &self,
        _cxt: NsId,
        _name: &[u8],
    ) -> Result<Option<CommandId>, tcl_syntax::value::ValueError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "command lookup",
        ))
    }

    /// Resolve a native namespace operand without converting unsupported
    /// protocol/context into a negative namespace result.
    ///
    /// # Errors
    /// The adapter must issue a checked actual-context lookup; the default refuses.
    fn find_namespace_bytes_checked(
        &self,
        _cxt: NsId,
        _name: &[u8],
    ) -> Result<Option<NsId>, tcl_syntax::value::ValueError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "namespace lookup",
        ))
    }

    /// Probe an actual parent's child table by its original physical member key.
    /// This operation does not parse a written namespace name or consult a cache.
    ///
    /// # Errors
    /// Refuses when the backend cannot authenticate the selected child table.
    fn find_namespace_child_bytes_checked(
        &self,
        _parent: NsId,
        _member: &[u8],
    ) -> Result<Option<NsId>, tcl_syntax::value::ValueError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "namespace child-table lookup",
        ))
    }

    /// Resolve the native namespace-variable query operation and report the
    /// followed namespace cell name. This is distinct from ordinary scalar
    /// access and must preserve the query operation's operand extent.
    ///
    /// # Errors
    /// Refuses when the adapter cannot authenticate the actual query protocol.
    fn namespace_variable_name_bytes_checked(
        &self,
        _cxt: NsId,
        _name: &[u8],
    ) -> Result<Option<Vec<u8>>, tcl_syntax::value::ValueError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "namespace variable query",
        ))
    }

    /// [`find_namespace`](Self::find_namespace) over a byte-valued name.
    fn find_namespace_bytes(&self, cxt: NsId, name: &[u8]) -> Option<NsId> {
        core::str::from_utf8(name)
            .ok()
            .and_then(|name| self.find_namespace(cxt, name))
    }

    /// [`namespace_var_exists`](Self::namespace_var_exists) over a
    /// byte-valued simple name.
    fn namespace_var_exists_bytes(&self, ns: NsId, simple: &[u8]) -> bool {
        core::str::from_utf8(simple).is_ok_and(|simple| self.namespace_var_exists(ns, simple))
    }

    /// [`name`](Self::name) as the bytes the namespace is actually keyed by.
    fn name_bytes(&self, ns: NsId) -> Vec<u8> {
        self.name(ns).into_bytes()
    }

    /// [`command_name`](Self::command_name) as the bytes the command is
    /// actually keyed by.
    fn command_name_bytes(&self, cmd: CommandId) -> Option<Vec<u8>> {
        self.command_name(cmd).map(String::into_bytes)
    }

    /// Direct command-table members without Unicode decoding.
    fn commands_in_bytes(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.commands_in(ns)
            .into_iter()
            .map(String::into_bytes)
            .collect()
    }

    /// Direct procedure-table members without Unicode decoding.
    fn procs_in_bytes(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.procs_in(ns)
            .into_iter()
            .map(String::into_bytes)
            .collect()
    }

    /// [`vars_in`](Self::vars_in) without a lossy UTF-8 round trip.
    fn vars_in_bytes(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.vars_in(ns)
            .into_iter()
            .map(String::into_bytes)
            .collect()
    }

    /// Ordered owned namespace inventory. Native adapters validate actual
    /// table selection; this compatibility default does not issue a hash recipe.
    fn vars_in_bytes_checked(
        &self,
        ns: NsId,
    ) -> Result<Vec<Vec<u8>>, tcl_syntax::value::ValueError> {
        Ok(self.vars_in_bytes(ns))
    }

    /// [`consts_in`](Self::consts_in) without a lossy UTF-8 round trip.
    fn consts_in_bytes(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.consts_in(ns)
            .into_iter()
            .map(String::into_bytes)
            .collect()
    }
}

/// Variable traces, preserving operational refusal outside guest completion.
pub trait Traces {
    /// The runtime's value type.
    type Value;

    /// Fire any traces registered for `var` on operation `op`
    /// (`"read"`/`"write"`/`"unset"`); a trace error aborts the access.
    fn fire(&mut self, var: &str, op: &str) -> Result<(), Self::Value>;

    /// Fire traces for an exact native name. The outer error is a host access
    /// refusal; the inner error is the callback's guest error result. Unicode
    /// adapters may use this checked default without replacing native bytes.
    fn fire_bytes(
        &mut self,
        var: &[u8],
        op: &str,
    ) -> Result<Result<(), Self::Value>, tcl_syntax::value::ValueError> {
        let name = tcl_syntax::raw_string::RawString::from_bytes(var).unicode()?;
        Ok(self.fire(&name, op))
    }
}

/// Runtime introspection backing the `info` family: retained proc bodies, the
/// per-frame argv, and the command-frame stack (`errorInfo`/`info frame`).
/// (Contract surface; not yet implemented.)
pub trait Introspect {
    /// The runtime's value type.
    type Value;

    /// The current call-stack depth (`info level`).
    fn level(&self) -> usize;
    /// The argv of the call at `level` (`info level N`), if retained.
    fn level_argv(&self, level: usize) -> Option<Self::Value>;
}

/// Procedure introspection backing `info body`/`args`/`default`: the retained
/// formal parameters (name + optional default) and source body of a user
/// procedure. Mirrors the `Proc`/`CompiledLocal` chain C's `InfoBodyCmd`/
/// `InfoArgsCmd`/`InfoDefaultCmd` walk.
///
/// Metadata queries return owned bytes. Default-value queries retain the
/// original runtime object, independently of its resident spelling or cache.
pub trait Procs: tcl_syntax::value::ValueOps {
    /// The formals + body of the user procedure `name` resolves to (following
    /// `namespace import` redirects, exactly as a call would), or `None` if
    /// `name` is not a user procedure (a builtin, alias, or unknown command).
    fn proc_info(&self, name: &str) -> Option<ProcInfo>;

    /// Resolve a procedure using the original native string bytes. Unicode-only
    /// adapters refuse an unrepresentable name before looking it up.
    fn proc_info_bytes(
        &self,
        name: &[u8],
    ) -> Result<Option<ProcInfo>, tcl_syntax::value::ValueError> {
        let name = tcl_syntax::raw_string::RawString::from_bytes(name).unicode()?;
        Ok(self.proc_info(&name))
    }

    /// Project a retained formal storage key or a query operand for native
    /// `info args`/`default`. This is introspection, not formal binding.
    fn formal_introspection_name_bytes(
        &self,
        name: &[u8],
    ) -> Result<Vec<u8>, tcl_syntax::value::ValueError> {
        tcl_syntax::raw_string::RawString::from_bytes(name).unicode()?;
        Ok(name.to_vec())
    }

    /// Stored formal names without materialising their default objects.
    fn proc_formal_names_bytes(
        &self,
        name: &[u8],
    ) -> Result<Option<Vec<Vec<u8>>>, tcl_syntax::value::ValueError> {
        Ok(self.proc_info_bytes(name)?.map(|info| {
            info.params
                .into_iter()
                .map(|parameter| parameter.name)
                .collect()
        }))
    }

    /// Retain body bytes without inspecting or materialising any default object.
    fn proc_body_bytes(
        &self,
        name: &[u8],
    ) -> Result<Option<Vec<u8>>, tcl_syntax::value::ValueError> {
        Ok(self.proc_info_bytes(name)?.map(|info| info.body))
    }

    /// Resolve a default under the selected formal introspection grammar.
    /// Concrete engines return the original retained default object.
    fn proc_default_value_bytes(
        &mut self,
        name: &[u8],
        arg: &[u8],
    ) -> Result<ProcDefaultValue<Self::Value>, tcl_syntax::value::ValueError> {
        let selected = self.formal_introspection_name_bytes(arg)?;
        let Some(info) = self.proc_info_bytes(name)? else {
            return Ok(ProcDefaultValue::MissingProcedure);
        };
        for parameter in info.params {
            if self.formal_introspection_name_bytes(&parameter.name)? == selected {
                return Ok(ProcDefaultValue::Declared(
                    parameter.default.map(|bytes| self.new_bytes(&bytes)),
                ));
            }
        }
        Ok(ProcDefaultValue::MissingParameter)
    }
}

/// Result of native default lookup, preserving missing-definition distinctions.
#[derive(Debug, Clone)]
pub enum ProcDefaultValue<V> {
    /// The selected command is not a user procedure.
    MissingProcedure,
    /// No formal matches the original query under native introspection rules.
    MissingParameter,
    /// An actual formal, with its original default when one was declared.
    Declared(Option<V>),
}

/// A user procedure's introspectable definition — the [`Procs::proc_info`] answer.
#[derive(Debug, Clone)]
pub struct ProcInfo {
    /// The procedure body source (`info body`), byte-exact.
    pub body: Vec<u8>,
    /// The formal parameters, in declaration order (`info args`/`default`).
    pub params: Vec<ProcParam>,
}

/// One formal parameter of a [`ProcInfo`]: a name with an optional default value.
#[derive(Debug, Clone)]
pub struct ProcParam {
    /// The parameter name.
    pub name: Vec<u8>,
    /// The declared default value, if the parameter has one.
    pub default: Option<Vec<u8>>,
}

pub mod script_source_location;
pub mod variable_destruction;

/// Checked allocation for concrete identity-domain owners.
pub mod checked_counter;
