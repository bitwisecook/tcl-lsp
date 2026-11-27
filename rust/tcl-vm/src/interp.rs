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

//! The interpreter engine (`Vm`): a tree of interpreters sharing one native
//! call stack, each holding its own call-frame stack, command table,
//! compiled-proc registry, and variable/command/eval surface
//! (`InterpState`). Cross-interp evaluation — `interp eval`, alias crossings
//! in any direction, `interp invokehidden` — switches the engine's current
//! interpreter by swapping arena-held state (`Vm::in_interp`), so it composes
//! with every native re-entry (coroutine resume, `lsort -command`, trace/event
//! callbacks) exactly as C Tcl's shared-C-stack, different-`Tcl_Interp*`
//! model does.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::io::Write;
use std::rc::{Rc, Weak};

mod bootstrap;
mod commands;
mod compilation;
mod configuration;
mod errors;
mod evaluation;
mod expressions;
mod frames;
mod interpreters;
mod limits;
mod namespaces;
mod native_result;
mod packages;
mod runtime_api;
mod traces;
mod variables;

pub(crate) use commands::CommandSidecarKey;
pub use commands::NativeCommandLookupUnavailable;
use compilation::CommandSemantics;
pub(crate) use errors::ErrorLogged;
#[cfg(test)]
pub(crate) use interpreters::CommandVisibilityError;
#[cfg(test)]
use interpreters::native_visibility_completion;
pub(crate) use limits::LIMIT_TYPES;
pub use limits::LimitKind;
#[cfg(test)]
pub(crate) use namespaces::NativeNamespaceAddress;
use namespaces::root_hash_order;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use packages::initial_package_prefer;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use packages::initial_package_prefer;
pub(crate) use variables::UpvarLinkError;

type OutputWriter = Rc<RefCell<Box<dyn Write>>>;

use tcl_dialect::{PackagePrefer, model::SurfaceQuery};

use native_error_stack::NativeErrorStack;
#[cfg(test)]
use tcl_bytecode::ProcedureProvenance;
use tcl_bytecode::{FunctionAsm, ModuleAsm};
use tcl_core_types::NameBytes;
use tcl_platform::Host;
use tcl_runtime_api::guard::{GuardDomain, GuardIdentity, GuardManager};
#[cfg(test)]
use tcl_runtime_api::guard::{GuardDomains, GuardError, GuardToken};
use tcl_runtime_api::jim_error_stack::{JimErrorStack, NativeErrorStackProtocol};
use tcl_runtime_api::{
    Code, CommandId, CompileService, Completion, FatalTail, Namespaces, NsId, ROOT_NS, VarId,
};
#[cfg(test)]
use tcl_runtime_api::{Commands, FrameId, Frames, Introspect, RegisteredBacking, VarStore};
#[cfg(test)]
use tcl_syntax::expr::eval;
#[cfg(test)]
use tcl_syntax::naming::NativeCommandVisibilityFailure;

mod authored_math;
mod logical_file_source;
pub(crate) use logical_file_source::LogicalSourceFileEntry;
mod logical_eval_source;
pub(crate) use logical_eval_source::LogicalEvalSourceEntry;
mod logical_uplevel_source;
pub(crate) use logical_uplevel_source::LogicalUpLevelSourceEntry;
mod authored_package;
pub(crate) use authored_package::PackageTableReceipt;
mod authored_rule_callables;
mod authored_tmm_static;
pub(crate) mod closed_native_execution;
mod execution_name_policy;
pub(crate) mod jim_local;
mod jim_table_keys;
mod jim_teardown;
pub(crate) mod namespace_context;
mod namespace_stack;
pub(crate) use namespace_context::{
    NamespaceExecutionContext, OriginalJimNamespaceContext, ProcedureNamespace,
};
mod native_alias_registration;
#[cfg(test)]
mod native_alias_registration_tests;
mod native_append;
#[cfg(test)]
mod native_bootstrap;
#[cfg(test)]
mod native_cache_tests;
mod native_command_names;
pub(crate) use native_command_names::ResolvedHead;
mod native_command_slots;
mod native_compiled_append;
pub(crate) mod native_compiled_locals;
mod native_coroutine_names;
pub(crate) mod native_dictionary;
#[cfg(test)]
mod native_embed_registration_tests;
mod native_ensemble_objects;
mod native_error_stack;
mod native_error_variables;
mod native_event_context;
mod native_execution_constants;
#[cfg(test)]
mod native_expression_bool_tests;
mod native_index_lookup;
mod native_interpreter_paths;
mod native_introspection;
mod native_jim_import;
mod native_jim_increment;
mod native_jim_interpreter_crossing;
mod native_jim_links;
pub(crate) mod native_jim_lookup;
#[cfg(test)]
mod native_legacy_increment_tests;
pub(crate) mod native_name_consumers;
mod native_name_operands;
mod native_name_queries;
mod native_name_world;
#[cfg(test)]
mod native_namespace_address_tests;
#[cfg(test)]
mod native_namespace_command_query_tests;
mod native_namespace_names;
#[cfg(test)]
mod native_source_manifest_tests;
pub(crate) use native_namespace_names::OriginalEnsembleNamespaceResult;
#[cfg(test)]
mod native_namespace_origin_tests;
#[cfg(test)]
mod native_namespace_root_tests;
pub(crate) mod native_numeric_ops;
pub(crate) mod native_operation_currency;
mod native_original_lookup;
mod native_package_files;
mod native_procedure_artifacts;
mod native_rename;
pub(crate) mod native_scalar_ingress;
#[cfg(test)]
#[path = "interp/native_upvar_info_exists_tests.rs"]
mod native_upvar_info_exists_tests;
mod native_variable_names;
mod native_variable_observers;
#[cfg(test)]
mod native_variable_query_tests;
mod object_materialization;
pub(crate) mod oo_cleanup;
pub(crate) mod original_authored_operands;
#[cfg(test)]
mod original_name_residual_tests;
mod selected_frame;
use namespace_stack::NamespaceResolutionTail;
pub(crate) mod command_cleanup;
mod variable_cleanup;
mod variable_container;
mod variable_trace_outcomes;

pub(crate) use jim_local::OriginalJimLocalCommand;
use native_procedure_artifacts::ProcedureCacheKey;
pub(crate) use selected_frame::SelectedFrameRestore;

use crate::command::{BuiltinFn, Command, EnsembleDef, ProcDef};
use crate::compiled::{CompiledUnit, CompilerProvenance, NativeCompilerCacheEpoch};
#[cfg(test)]
use crate::expr::ExprEval;
use crate::frame::CallFrame;
// The default capability host, picked by target. Everywhere with an operating
// system under it — including `wasm32-wasip1`, where `std::time` and `std::env`
// both work — that is the full-capability std-backed `NativeHost`. On
// `wasm32-unknown-unknown` there is no OS at all and `std::time::SystemTime::now`
// *panics*, so the browser host reads the wall clock from JavaScript instead
// (see the WASM host below). `runtime/rust`'s `Interp::new` makes the same
// split for the same reason.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use crate::host_native::NativeHost as DefaultHost;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use crate::host_wasm::BrowserHost as DefaultHost;

/// The platform's standard host: the full-capability `NativeHost`, or the
/// browser host on `wasm32-unknown-unknown` (whose wall clock reads from
/// JavaScript). Hosts pass it to [`Vm::with_native_core`] when they have no
/// host of their own.
#[must_use]
pub fn standard_host() -> Rc<dyn Host> {
    Rc::new(DefaultHost::new())
}

/// What a sibling native-core interpreter is built from: the actual engine
/// profile and the core bootstrap inputs ([`Vm::native_core_recipe`]).
pub(crate) type NativeCoreRecipe = (
    &'static tcl_dialect::DialectProfile,
    tcl_registry::special_vars::NativeBootstrapInputs,
);
use crate::value::Value;
use crate::vars::{NameLink, VarArena, VarState as Local, VarTable};

use native_name_world::NameWorldCell;
pub use native_name_world::{
    NativePreparedPublication, NativePublicationCommit, NativePublicationError,
    NativePublicationKey, NativePublicationPurpose, NativePublicationService,
};

type CommandSlot = tcl_runtime_api::CommandSlot<tcl_core_types::NameBytes>;
type NamespacePath = tcl_core_types::ByteNamespacePath;

/// The proc-call recursion bound (C Tcl's default `interp recursionlimit`).
/// A deeper nesting is a catchable error, not a native stack overflow.
pub(crate) const RECURSION_LIMIT: usize = 1000;

/// Build an `OK` completion (empty options dict).
pub(crate) fn ok(result: Value) -> Completion<Value> {
    Completion::new(Code::Ok, result, Value::empty())
}

/// Build an `ERROR` completion from a message (empty options dict).
pub(crate) fn err(message: impl AsRef<[u8]>) -> Completion<Value> {
    Completion::new(
        Code::Error,
        Value::from_string_bytes(message.as_ref()),
        Value::empty(),
    )
}

/// Build an `ERROR` completion carrying the canonical
/// `wrong # args: should be "usage"` arity message (the
/// [`tcl_cmd_core::CmdError::wrong_args`] catalogue text).
pub(crate) fn err_wrong_args(vm: &mut Vm, usage: &str) -> Completion<Value> {
    crate::command::native_wrong_args(vm, usage)
}

/// Stable identity of one interpreter in this VM's arena (a slot index).
/// Identities are never reused: a deleted interp's id stays dead forever, so
/// an alias or pending callback holding a stale id can never accidentally
/// address a same-named successor (C's `Tcl_Interp*` identity rule —
/// tclsh-pinned: recreating a deleted target does not resurrect its aliases).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct InterpId(usize);

/// The root interpreter's id (slot 0, never deleted).
pub(crate) const ROOT_INTERP: InterpId = InterpId(0);

/// One arena slot: an interp's parked state plus the lifecycle bookkeeping
/// that must stay addressable while the state itself is executing.
struct InterpSlot {
    /// The interp's state while it is NOT the currently-executing one.
    /// Exactly one live interp has `None` here at any time — the current one,
    /// whose state is in [`Vm::state`].
    parked: Option<Box<InterpState>>,
    /// In-flight evaluations addressing this interp (the innermost current
    /// activation plus every buried re-entry). Guards teardown: `interp
    /// delete` on an active interp defers the state drop until this reaches
    /// zero (C's `Tcl_Preserve`/`Tcl_Release` — tclsh-pinned: a child's
    /// in-flight eval runs to completion after its deletion).
    active: u32,
    /// Deleted while active: unreachable by name/path, state torn down when
    /// the last in-flight evaluation unwinds.
    dying: bool,
    /// The command that names this interp is being deleted: its delete traces
    /// are running and its own deletion has not yet retired the interp.
    command_dying: bool,
    /// The interp that created this one (`None` for the root).
    parent: Option<InterpId>,
}

/// Engine-level record of one cross-interp alias: deleting the TARGET interp
/// must remove the alias command from its SOURCE interp (C keeps a per-interp
/// target table for exactly this sweep — tclsh-pinned: after `interp delete
/// b`, an `a`-side alias into `b` is gone from `info commands`).
struct AliasBackref {
    source: InterpId,
    key: CommandSidecarKey,
    target: InterpId,
}

/// The bytecode VM: the engine driving a tree of interpreters.
///
/// Every interpreter's state lives in one arena (`interps`), addressed by a
/// stable [`InterpId`]; the currently-executing interp's state is held
/// directly in `state` (hot: `Deref` target, one pointer hop). Cross-interp
/// evaluation — `interp eval`, alias crossings, `interp invokehidden` — is a
/// plain nested native call with the two states swapped (C's shared C stack
/// with a different `Tcl_Interp*`), so re-entering an interpreter that is
/// already executing deeper on the stack is legal: its persistent state stays
/// addressable in the arena throughout.
pub struct Vm {
    /// Process-unique identity for embedder artefacts. A reusable compiled
    /// handle belongs to the VM that compiled it; generations alone are not a
    /// cross-VM identity domain.
    pub(crate) owner_nonce: u64,
    pub(crate) authored_timers: crate::retained_activation::AuthoredTimers,
    authored_tmm_static: authored_tmm_static::AuthoredTmmStaticDomain,
    /// The currently-executing interpreter's state ([`Deref`] target).
    state: Box<InterpState>,
    /// Which interp `state` belongs to.
    cur: InterpId,
    /// All interpreters ever created; slot 0 is the root. Slots are never
    /// removed — a deleted interp's slot stays, `parked: None`, `dying`.
    interps: Vec<InterpSlot>,
    /// Count of nested [`Vm::drive`](crate::exec) invocations — the host
    /// re-entry counter. A `yield` is legal only at the depth its coroutine's
    /// driver started at; a deeper depth means a `catch`/`uplevel`/`eval`/
    /// `lsort -command`/OO-method/cross-interp re-entry sits between, so the
    /// suspend is rejected (`cannot yield: C stack busy`) — including across
    /// an interp boundary, exactly as C Tcl's non-NRE cross-interp eval
    /// (tclsh-pinned).
    pub(crate) activation_depth: usize,
    /// Engine-wide refusal survives child interpreter switching. It is never
    /// translated into a Tcl result or consumed by a guest catch boundary.
    pub(crate) execution_refusal: Option<tcl_runtime_api::NativeExecutionError>,
    /// Synchronous host entries can nest outside an activation drive. Only the
    /// outermost entry may return or invariant-refuse a retained host error.
    pub(crate) host_execution_depth: usize,
    /// A pin made from a native command, with the native-command depth it was
    /// made at: installed at the first command boundary after that command
    /// returns, so the command finishes under the context it entered with.
    deferred_pin: Option<(tcl_registry::model::PinnedContext, usize)>,
    /// Native commands running: a pin made from one waits until it returns.
    pub(crate) native_command_depth: usize,
    /// Mutable standard-channel handles shared by every interpreter in this
    /// VM, just like the process-wide handles in C Tcl.
    pub(crate) standard_channels: RefCell<tcl_cmd_core::channel::StandardChannelConfigs>,
    /// Mutable `encoding system` state shared by the interpreter tree. The
    /// host supplies its initial value; changing it affects only channels
    /// opened afterwards.
    system_encoding: tcl_platform::SystemEncoding,
    default_system_encoding: tcl_platform::SystemEncoding,
    /// Cross-interp alias records for the target-death sweep. Appended when a
    /// [`Command::CrossAlias`] registers; entries are validated lazily at
    /// sweep time (a stale entry — alias since removed or source dead — is
    /// skipped).
    alias_backrefs: Vec<AliasBackref>,
}

/// A captured command generation in one actual interpreter and namespace token.
/// Its private storage key is never a written Tcl name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeRegisteredCommandToken {
    owner: u64,
    interpreter: u64,
    slot: CommandSlot,
    generation: u64,
}

/// The whole storage of a namespace token whose deletion is waiting for its
/// last active frame — C's `activationCount > 0` branch of
/// `Tcl_DeleteNamespace`, which marks `NS_DYING`, unlinks the parent edge and
/// leaves the contents in place.
///
/// C can leave them where they are: a `Namespace` owns its own `cmdTable`,
/// `varTable` and `childTable`. The VM's tables are flat and keyed by
/// canonical name, so a retained `N::q` and the `N::q` of a namespace
/// recreated under the same spelling would be one entry. The retained token's
/// subtree therefore moves out of the live tables into this record, which only
/// the frames holding the token can reach.
struct NativeCachedModule {
    module: Rc<ModuleAsm>,
    entry: std::sync::Arc<tcl_runtime_api::NativeCompilationEntry>,
}

/// The accepted original prefix root and the setter's reached native length.
/// The length supplies no cached List or member-object identity.
#[derive(Clone, Copy)]
struct ActualNativeProtocols {
    string: Option<tcl_syntax::native_string::NativeStringProtocol>,
    name: Option<tcl_syntax::naming::NativeNameProtocol>,
    name_policy: Option<tcl_syntax::naming::NamePolicyProtocol>,
    error_stack: Option<NativeErrorStackProtocol>,
}

#[derive(Clone)]
struct NativeUnknownHandler {
    root: Value,
    length: usize,
}

struct RetainedNamespace {
    /// The retained token's own id — the key this record is filed under.
    root: NsId,
    /// Every canonical namespace name at or below the token, with its id.
    subtree: HashMap<NamespacePath, NsId>,
    commands: HashMap<String, Command>,
    generations: HashMap<String, u64>,
    imported: HashMap<String, ImportBinding>,
    builtin_identities: HashMap<String, String>,
    command_orders: HashMap<NsId, tcl_cmd_core::namespace::TclStringHashOrder>,
    child_orders: HashMap<NsId, tcl_cmd_core::namespace::TclStringHashOrder>,
    exports: HashMap<NsId, Vec<NameBytes>>,
    paths: HashMap<NsId, Vec<NsId>>,
    unknowns: HashMap<NsId, NativeUnknownHandler>,
}

/// Namespace-token deferral state: how many call frames are running in each
/// token (C's `Namespace.activationCount`, which `Tcl_PushCallFrame` bumps for
/// every frame — proc, `apply`, `TclOO` method, `namespace eval`/`inscope`),
/// and the storage of the tokens a non-zero count is keeping alive.
#[derive(Default)]
struct NamespaceDeferral {
    counts: HashMap<NsId, u32>,
    /// Exact namespace births owned by stock ensemble bootstrap, never inferred
    /// from namespace spelling or the presence of a similarly named command.
    bootstrap_ensembles: HashMap<&'static str, NsId>,
    /// Retained storage of each deferred token, keyed by that token's id.
    retained: HashMap<NsId, RetainedNamespace>,
    /// Every token id inside a retained subtree → the id of the record holding
    /// it. A frame may hold a retained *descendant*, when the delete recursion
    /// never reached it.
    owners: HashMap<NsId, NsId>,
}

/// One namespace-import command token. The source edge alone is not a stable
/// identity: a delete trace can re-import the same source at the same name
/// while the original token is being retired. The generation distinguishes
/// that replacement so the outer deletion leaves it intact.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct CommandTokenIdentity {
    key: CommandSidecarKey,
    generation: u64,
}

/// The actual global command selected for C8.4's direct import preload call.
/// Only the namespace import owner can create this entered callable snapshot.
pub(crate) struct OriginalCImportPreload {
    token: CommandTokenIdentity,
    command: Command,
}

type VmCommandGuards = tcl_runtime_api::guard::OwnedGuardManager<CommandTokenIdentity>;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ImportBinding {
    origin: CommandTokenIdentity,
    generation: u64,
    compiler_hook: tcl_runtime_api::native_compilation::NativeCompilerHookPresence,
    procedure_header: Option<tcl_dialect::NativeProcedureHeaderCompilation>,
    compiler: Option<tcl_runtime_api::native_compilation::NativeCommandCompiler>,
}

/// A stable snapshot of one imported command token. Its direct import refs are
/// queried only after this token's delete trace has run, in Tcl `ImportRef`
/// order (newest first), so callback-created imports join the dying token's
/// cascade while imports of a same-name replacement token survive.
#[derive(Clone, Debug)]
struct ImportRetirementNode {
    token: CommandTokenIdentity,
    binding: ImportBinding,
}

/// Relocation-aware identity held by an in-flight command operation.
///
/// Hide, expose, and rename move a command's sidecar domain while its body may
/// still be running. Every active consumer shares this cell, so a relocation
/// updates coroutine parking/retirement and execution-trace leave delivery as
/// one operation.
#[derive(Clone, Debug)]
pub(crate) struct CommandSidecarHandle(Rc<RefCell<Option<CommandSidecarKey>>>);

/// Metadata and any release-hidden destination displaced while a command is
/// temporarily installed under a new name.  A rename can still be refused
/// after installation when it would make an alias loop, so this keeps the
/// command-table mutation reversible as one semantic operation.
pub(crate) struct CommandRenameTransaction {
    old_key: String,
    new_key: String,
    /// Retained namespace record that owns the source table entry, if any.
    /// The entry stays in that record through validation and is materialised
    /// only once the rename can no longer be rejected.
    source_retained_root: Option<NsId>,
    source_generation: u64,
    source_import_binding: Option<ImportBinding>,
    source_builtin_identity: Option<String>,
    /// Whether the source was an engine-installed `TclOO` root. Like
    /// `source_builtin_identity`, this is an *identity* that must travel with
    /// the command: `rename oo::configurable myconf` leaves a command that is
    /// still the registry's `oo::configurable`, so it must still vanish on a
    /// release that has no such builtin.
    source_registry_object_root: Option<String>,
    cross_alias_target: Option<InterpId>,
}

impl std::ops::Deref for Vm {
    type Target = InterpState;

    #[inline]
    fn deref(&self) -> &InterpState {
        &self.state
    }
}

impl std::ops::DerefMut for Vm {
    #[inline]
    fn deref_mut(&mut self) -> &mut InterpState {
        &mut self.state
    }
}

/// Deferred control requests handed from builtins to the explicit VM stack.
#[derive(Default)]
pub(crate) struct PendingControl {
    /// Actual procedure lease and original argv deferred without source serialization.
    pub(crate) procedure_call: Option<crate::exec::OriginalProcedureCallReq>,
    /// Body deferred by `eval`/`uplevel`/`apply` to the explicit stack.
    pub(crate) eval: Option<crate::exec::EvalReq>,
    /// Catch body whose completion is absorbed by the catch epilogue.
    pub(crate) catch: Option<crate::exec::CatchReq>,
    /// Scanner-driven substitution request.
    pub(crate) subst: Option<crate::exec::SubstReq>,
    /// Runtime `foreach`/`lmap` request.
    pub(crate) each_loop: Option<crate::exec::EachLoopReq>,
    /// Generic native if/while/for continuation, shared by every caller.
    pub(crate) control: Option<crate::cmd_control::ControlState>,
    /// Shared AST continuation for native and bytecode expression evaluation.
    pub(crate) expression: Option<crate::exec::ExpressionReq>,
    /// Next body, handler, or `finally` phase of a `try`.
    pub(crate) try_phase: Option<crate::cmd_try::TryReq>,
    /// How the emulated release compiles the command the current invoke now
    /// runs ([`tcl_bytecode::Instruction::release_compiled`]): a command the
    /// release compiles reads its words, logs and binds as that compiled
    /// command would. Taken by the command that consumes it.
    pub(crate) release_compiled: Option<tcl_bytecode::ReleaseCompiled>,
    /// The command the outermost activation was running when it completed
    /// with an exceptional code: what C's `TclEvalEx` logs as an error leaves
    /// a command run with no command active ([`Vm::settle_top_level`]).
    pub(crate) top_level_command: Option<crate::exec::LoggedCommand>,
    /// The activation being ticked is the bottom one of the outermost drive,
    /// so a command it invokes runs with no other command active.
    pub(crate) bottom_activation: bool,
    /// The command being dispatched is a top-level statement's own command
    /// (C's `numLevels == 0`); `coroutine` takes it.
    pub(crate) top_level_invocation: bool,
}

/// Written invocation and selected native implementation are independent.
#[derive(Default)]
pub(crate) struct NativeInvocationState {
    pub(crate) arguments: Option<crate::NativeListItems>,
    pub(crate) usage_rewrites: Vec<crate::command::NativeArgumentUsageRewrite>,
    pub(crate) usage_revision: u64,
    name: Option<String>,
    original_name: Option<Value>,
    builtin_identity: Option<String>,
    pub(crate) sidecar: Option<CommandSidecarKey>,
}

/// Handler metadata survives reentrant commands independently of Tcl's resettable
/// ensemble usage rewrite. It owns no snapshot of the active rewrite.
pub(crate) struct NativeHandlerMetadata {
    name: Option<String>,
    original_name: Option<Value>,
    builtin_identity: Option<String>,
    sidecar: Option<CommandSidecarKey>,
}

pub(crate) struct NativeUsageScope {
    previous: Vec<crate::command::NativeArgumentUsageRewrite>,
    revision: u64,
}

struct DebugHookActivation {
    active: Rc<std::cell::Cell<bool>>,
    previous: bool,
}

#[derive(Default)]
struct InterpreterDebugHooks {
    /// Optional debug hook fired once per source command (the execution-control
    /// seam a step debugger drives). `None` in normal runs — the only
    /// per-instruction cost is an `Option` check.
    debug_hook: Option<crate::debug::DebugHook>,
    debug_hook_active: Rc<std::cell::Cell<bool>>,
    #[cfg(test)]
    observer_probe: Option<fn(&Vm)>,
    /// Optional shared cell tracking the source line of the instruction being
    /// dispatched — the lightweight sibling of the debug hook, for an embedder
    /// that needs "what line is the interpreter on" (the `SpecTcl`
    /// pack-evaluation loader attributes captured registrations with it)
    /// without paying for a per-command [`crate::debug::DebugSnapshot`].
    /// `None` in normal runs — one `Option` check per instruction.
    line_watch: Option<std::rc::Rc<std::cell::Cell<u32>>>,
    /// The `(line, span-start)` key of the last command the debug hook fired
    /// for, so it fires once per source command rather than per instruction
    /// (`startCommand` is emitted only conditionally, so it cannot be the
    /// boundary marker).
    last_debug_key: Option<u64>,
}

struct InterpreterCounters {
    /// Monotonic counter for minting fresh channel ids.
    chan_counter: u32,
    /// Monotonic counter minting auto-generated child names (`interp0`, …).
    interp_counter: u64,
    /// Commands dispatched by this interp since the last
    /// [`Vm::reset_command_count`] — the counter the `commands` limit is
    /// charged against, and the fuel gauge an embedder reads
    /// ([`Vm::commands_run`]). Free-running: an interp with no limit set still
    /// counts, because the count costs one increment and answers "how much did
    /// that body cost" without arming anything.
    commands_run: u64,
    /// Free-running counter that throttles wall-clock polling for the `time`
    /// limit (see [`Vm::limit_check_tick`]). Vm-scoped, not per-activation, so
    /// it accumulates across the short-lived activations a command-driven loop
    /// (`$while {1} {…}`) spins through.
    limit_tick: u32,
}

struct FreshSemanticEnvironment {
    guards: GuardManager,
    profile: &'static tcl_dialect::DialectProfile,
    surface: SurfaceQuery<'static>,
}

#[derive(Default)]
struct PackageState {
    identity: Rc<()>,
    /// Provided packages → version (`package provide`/`require`).
    packages: HashMap<tcl_core_types::NameBytes, tcl_core_types::NameBytes>,
    version_objects: HashMap<tcl_core_types::NameBytes, Value>,
    package_entry_order: tcl_core_types::NativeEntryLedger,
    package_files: HashMap<tcl_core_types::NameBytes, Value>,
    package_file_inventory_active: bool,
    package_file_scopes: Vec<tcl_core_types::NameBytes>,
    package_source_paths: Vec<Vec<u8>>,
    package_loader_origins:
        HashMap<(tcl_core_types::NameBytes, tcl_core_types::NameBytes), Vec<u8>>,
    /// Package name → version → loader script (`package ifneeded`).
    package_ifneeded: HashMap<tcl_core_types::NameBytes, Vec<(tcl_core_types::NameBytes, Vec<u8>)>>,
    /// Selected package loaders currently being evaluated. Tcl uses the
    /// selected loader's required name/version as a circular-dependency guard;
    /// the stack matters because loaders may require other packages. A
    /// `package forget` retires the guard (C deletes the `Package` record
    /// holding it) while the loader itself still runs.
    package_loading: Vec<PackageLoadingMarker>,
    /// Libraries loaded into this interpreter, `(file name, prefix)` in the
    /// order they were loaded (`info loaded`). The file name is empty for one
    /// linked into the program.
    loaded_libraries: Vec<(String, String)>,
}

/// One running package loader's circular-dependency guard.
struct PackageLoadingMarker {
    name: tcl_core_types::NameBytes,
    version: tcl_core_types::NameBytes,
    forgotten: bool,
}

/// The guard machinery's record of command tokens, by token generation.
#[derive(Default)]
struct GuardedCommands {
    /// The stable semantic identities attested for each command token, by the
    /// token's generation. A generation follows its command through rename and
    /// hide, and a replacement or deletion leaves the old entry unreachable, so
    /// an attestation lasts exactly as long as the command it describes.
    /// Ordinary command registration cannot authorise a fast path.
    attested: std::cell::RefCell<HashMap<u64, BTreeSet<GuardIdentity>>>,
    /// The command tokens an embedder registered through
    /// [`Vm::register_guarded_builtin`]. Such a handler is the embedder's
    /// implementation of a command and never this VM's own, so a unit
    /// specialised for the shipped builtin of its name is not admitted over it,
    /// as it is not over a native command or a procedure. It follows its token
    /// through rename and hide, as an attestation does.
    host: HashSet<u64>,
}

#[derive(Clone, Copy)]
struct NativeCReturnState {
    code: i32,
    level: i64,
}

/// The actual result and selected interpreter state surrounding a C callback.
pub(crate) struct NativeTraceResultState {
    result: Value,
    interpreter: Option<NativeTraceInterpreterState>,
    recipe: tcl_registry::native_variable_trace::NativeTraceStateRecipe,
    physical_owner: variable_trace_outcomes::TracePhysicalOwner,
}

struct NativeTraceInterpreterState {
    error: native_error_variables::NativeErrorTraceState,
    return_state: NativeCReturnState,
    stack: Option<NativeErrorStack>,
}

/// C interpreter error roots and their publication/logging state. Field order
/// retains the original native owner release order.
struct NativeErrorState {
    error_info: Option<Vec<u8>>,
    native_error_info: Option<Value>,
    native_error_info_len: usize,
    native_error_result: Option<crate::value::WeakNativeObject>,
    native_return_options: Option<Value>,
    native_c_return_state: NativeCReturnState,
    c84_execution_names: Option<native_error_variables::C84ExecutionEnvironmentErrorNames>,
    native_error_legacy_copy: bool,
    c84_global_code_set: bool,
    /// Primitive interpreter error-code state, separate from guest globals.
    /// None is the fresh interpreter's native NONE state.
    primitive_error_code: Option<Value>,
    /// C's `ERR_ALREADY_LOGGED`: set once the current command level has logged
    /// its frame (so the same bytecode frame is not re-logged), cleared at a
    /// real frame boundary (a nested `eval`/`[subst]`, a proc/control body) so
    /// the enclosing command logs its own `invoked from within` frame.
    error_logged: ErrorLogged,
    /// TIP 348's interpreter-local structured error stack. Its lazy reset
    /// keeps the last caught stack introspectable until a new error is logged.
    error_stack: NativeErrorStack,
    error_line: u32,
}

/// Explicit authored simulation providers, independent of actual native
/// interpreter registration and object issuers.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
struct LogicalProviders {
    /// Explicit simulation capability. It never contributes a native compiler
    /// or hardware result receipt; native policy remains independently selected.
    quote: Option<tcl_registry::invocation_words::LogicalExpressionQuoteProvider>,
    source_words: Option<tcl_registry::invocation_words::LogicalSourceWordProvider>,
    source_file: Option<tcl_registry::source_file::LogicalSourceFileProvider>,
    eval_object: Option<tcl_registry::native_eval_object::LogicalEvalObjectProvider>,
    expression_parse: Option<tcl_registry::invocation_words::LogicalExpressionParseProvider>,
    numeric: Option<tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation>,
    math_functions: Option<tcl_runtime_api::expression_policy::AuthoredMathFunctionProvider>,
    /// Explicit authored name/string simulation, independent of the host engine.
    names: Option<tcl_syntax::naming::NamePolicyProtocol>,
    /// Explicit compiler-local simulation, separate from name and host authority.
    compiled_variables:
        Option<tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider>,
}

/// Interpreter, command and binding epochs, policy stamp and surface profile.
type CompilationCommandsKey = (
    tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
    u64,
    u64,
    u64,
    usize,
);

/// A compilation command inventory and whether it is complete, by key.
type CompilationCommandsMemo = (
    CompilationCommandsKey,
    tcl_runtime_api::native_compilation::NativeCompilationCommands,
    bool,
);

/// Everything [`Vm::native_compiler_policy`] reads: interned profile handles,
/// the emulated release and the explicit authored providers.
#[derive(Clone, Copy, PartialEq, Eq)]
struct NativeCompilerPolicyInputs {
    pinned: usize,
    active: Option<usize>,
    engine: Option<usize>,
    release: tcl_dialect::TclVersion,
    providers: LogicalProviders,
}

/// Everything the activation freshness query reads outside its frame. While
/// it is unchanged, a frame found fresh stays fresh.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ActivationFreshnessWorld {
    profile: u64,
    compiler: u64,
    policy: NativeCompilerPolicyInputs,
    name_world: u64,
    namespace: namespace_context::NamespaceExecutionContext,
    execution_profile: usize,
}

#[derive(Clone)]
struct NativeBootstrapPurpose {
    protocol: tcl_registry::special_vars::NativeBootstrapProtocol,
    inputs: tcl_registry::special_vars::NativeBootstrapInputs,
}

/// Hidden command allocations and their exact token and provenance indexes.
#[derive(Default)]
struct HiddenCommandState {
    commands: HashMap<String, Command>,
    tokens_by_name: HashMap<NameBytes, String>,
    names_by_key: HashMap<String, NameBytes>,
    next_key: u64,
    generations: HashMap<String, u64>,
    imports: HashMap<String, ImportBinding>,
    builtin_identities: HashMap<String, String>,
}

/// Native cache epochs and command-semantic disposition, updated together at
/// original interpreter mutation boundaries.
struct NativeCompilationEpochs {
    trace_deopt_epoch: std::cell::Cell<u64>,
    /// Actual native compilation invalidation, independent of lookup guards.
    native_compiler_epoch: std::cell::Cell<NativeCompilerCacheEpoch>,
    /// Resolver epoch keyed by actual namespace incarnation.
    native_namespace_epochs: HashMap<NsId, u64>,
    native_ensemble_namespace_epochs: RefCell<HashMap<NsId, u64>>,
    /// Whether command-binding generation tracking has passed bootstrap.
    command_semantics: std::cell::Cell<CommandSemantics>,
    cmd_resolve_cache: RefCell<CommandResolveCache>,
    cmd_epoch: std::cell::Cell<u64>,
    /// Advanced by every change any of the above observes, whether or not the
    /// compiler cache treats it as invalidating.
    binding_epoch: std::cell::Cell<u64>,
    /// Compiled functions whose command and procedure bindings last held, with
    /// the binding stamp they held under.
    held_bindings: RefCell<Vec<(std::rc::Weak<FunctionAsm>, BindingStamp)>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct CommandLookupStamp {
    binding_epoch: u64,
    cmd_epoch: u64,
    trace_deopt_epoch: u64,
    name_world: u64,
    interpreter: usize,
    profile: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct BindingStamp {
    binding_epoch: u64,
    cmd_epoch: u64,
    trace_deopt_epoch: u64,
    interpreter: usize,
    profile_generation: u64,
    compiler_generation: u64,
}

struct NamespaceResolutionStacks {
    ns_stack: Vec<Option<NameBytes>>,
    ns_id_stack: Vec<NamespaceExecutionContext>,
    activation_births: Vec<Rc<()>>,
}

type CommandResolveCache = (u64, HashMap<(NsId, NameBytes, bool), Option<String>>);
type ImportedSourceCommand = (NameBytes, Command, Option<String>, CommandTokenIdentity);

/// One interpreter's command table, namespaces, frames and execution state.
/// The engine swaps this state at interpreter boundaries.
pub struct InterpState {
    /// The Tcl release whose number/expr grammar this VM emulates —
    /// threaded from `DialectProfile::vm_runtime_version` (dialect-profile
    /// model §5.4). The profile pins the release-specific grammar, command,
    /// and variable-resolution semantics; the VM never infers them from a
    /// dialect name.
    runtime_version: tcl_dialect::TclVersion,
    /// The profile used solely for builtin command-surface availability.
    /// Normally identical to the pinned dialect profile, but an embedding host
    /// may expose a broader Tcl host surface while retaining a vendor grammar
    /// and bytecode identity (for example, the iRules simulation harness).
    command_surface_profile: &'static tcl_dialect::DialectProfile,
    /// Explicit embedder-owned native capabilities. A broader host surface
    /// alone never changes user command grammar.
    pub(crate) framework_builtins: Vec<crate::embed::FrameworkBuiltinCapability>,
    pub(crate) active_native_profile: Option<&'static tcl_dialect::DialectProfile>,
    /// The actual embedding engine, explicitly selected independently of source
    /// grammar, logical emulation and command availability. An absent explicit
    /// engine supplies no measured Native execution point; availability cannot set it.
    actual_engine_profile: Option<&'static tcl_dialect::DialectProfile>,
    /// Native protocols of the last actual execution profile, keyed by its address.
    actual_protocols: std::cell::Cell<Option<(usize, ActualNativeProtocols)>>,
    /// Full context of the genuinely bootstrapped core, independent of user source.
    /// Selecting a profile cannot create or renew this core receipt.
    native_core_pin: Option<tcl_registry::model::PinnedContext>,
    jim_object_context: RefCell<Option<Rc<crate::value::NativeJimObjectContext>>>,
    native_c_result: RefCell<Option<Value>>,
    /// Actual interpreter execution-environment Boolean constant roots.
    native_execution_booleans: Option<[Value; 2]>,
    jim_teardown_started: bool,
    native_operation_context: Rc<native_operation_currency::NativeOperationContext>,
    logical_providers: LogicalProviders,
    /// The last [`Vm::native_compiler_policy`] with the inputs it projects and
    /// a stamp that changes whenever a projection differs from the previous one.
    /// The last compilation command inventory with the epochs it was taken at.
    compilation_commands_memo: RefCell<Option<CompilationCommandsMemo>>,
    /// Compiler-selection registrations matched at a command-binding epoch.
    matched_prerequisites: RefCell<(
        u64,
        Vec<std::sync::Arc<tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite>>,
    )>,
    compiler_policy_memo: RefCell<
        Option<(
            NativeCompilerPolicyInputs,
            crate::compiled::NativeCompilerPolicy,
            u64,
        )>,
    >,
    observed_names: Option<execution_name_policy::ObservedNameSelection>,
    observed_frame_storage: Option<execution_name_policy::ObservedFrameStorage>,
    authored_math: Option<authored_math::AuthoredMathState>,
    /// The availability registry for [`Self::command_surface_profile`] —
    /// its environment's registry generation, resolved once at pin time
    /// through the ingress seam ([`crate::environment::store_for_profile`];
    /// the generation cache guards itself with a lock, and this is
    /// consulted on every command resolution). `None` for the permissive
    /// fallback profile, which gates nothing.
    profile_registry: Option<&'static tcl_registry::CommandRegistry>,
    /// The availability point [`Self::command_surface_profile`]'s environment
    /// answers the builtin-surface gate under — its **document authoring
    /// mask** ([`crate::environment::surface_point`]), resolved at pin time
    /// for the same reason [`Self::profile_registry`] is: the generation
    /// lookup takes a lock and this is read on every command resolution, where
    /// the retired `profile.surface_query()` was a field read. Equal to that
    /// mask for every profile an ingress can produce, pinned by the seam's own
    /// sweep.
    command_surface_point: Option<SurfaceQuery<'static>>,
    /// Monotonic invalidation generation for bytecode that depends on the
    /// selected profile's grammar or command surface. It is deliberately
    /// independent of `cmd_epoch`: command resolution can be recomputed from
    /// names, whereas specialised bytecode must be recompiled from source.
    profile_generation: u64,
    /// Monotonic invalidation generation for the injected compile service.
    compiler_generation: u64,
    /// Call-frame stack; `frames[0]` is the global scope.
    frames: Vec<CallFrame>,
    jim_storage: tcl_runtime_api::jim_call_frame::JimCallFrameStoragePool,
    /// Variable cells outlive the name table that currently binds them. This
    /// is interpreter-global so parked coroutine frames retain their `VarId`s.
    var_arena: VarArena,
    #[cfg(test)]
    pub(crate) actual_dictionary_search_entries: usize,
    /// Variable name tables belong to namespace tokens, not their reusable
    /// spellings. A retained and recreated `::N` therefore remain disjoint.
    ns_vars: HashMap<NsId, VarTable>,
    /// Stable cells declared through TIP 677 `const`.
    const_vars: rustc_hash::FxHashSet<VarId>,
    /// Command table (builtins + user procs), keyed by canonical name — a
    /// builtin's simple name, or a proc's namespace-qualified name without the
    /// leading `::` (e.g. `foo::bar`; a global proc is just `bar`).
    commands: HashMap<String, Command>,
    alias_registrations:
        tcl_registry::native_alias_registration::NativeAliasRegistrationTable<CommandTokenIdentity>,
    /// Sole owned namespace/command-address authority, shared by weak native services.
    name_world: Rc<NameWorldCell>,
    /// Structured slots, private storage projections, and stable command-token
    /// generations/handles under one identity owner.
    /// The fixed C math-function table used by pre-TIP-232 `expr` (Tcl 8.4).
    ///
    /// Normal command registration also installs these handlers under
    /// `tcl::mathfunc::*` for Tcl 8.5+, but an 8.4 expression must keep using
    /// this immutable builtin table even if a script later creates or replaces
    /// a similarly named command. The registry decides when this table applies;
    /// this map merely retains the registered implementation.
    fixed_math: FixedMathTable,
    /// Pre-compiled proc bodies from modules, keyed by exact canonical source
    /// identity. Multiple definitions of one command name may coexist: module
    /// admission must not make a later valid body depend on merge order.
    module_procs:
        HashMap<ProcedureCacheKey, native_procedure_artifacts::ProcedureArtifactCandidates>,
    /// Current-namespace stack (canonical, no leading `::`; `""` = global). The
    /// top governs `proc`/command/variable name resolution. `namespace eval`
    /// and proc activation push/pop it.
    resolution_stacks: NamespaceResolutionStacks,
    /// Stable token identity parallel to `ns_stack`. Names cease to be interned
    /// when their namespace is deleted, but an activation in that namespace
    /// retains the old token until it returns. Keeping the id on the execution
    /// stack also distinguishes that dead token from a same-named recreation.

    /// Existing namespaces (canonical names; `""` global is implicit).
    /// Namespace tokens currently inside `TclTeardownNamespace`. Each exact
    /// namespace is already non-existent to Tcl callbacks, but its command
    /// table remains live until token deletion completes. Descendants are not
    /// marked dying until their own recursive teardown begins.
    /// Namespace-owned ensemble tokens in C's intrusive-list order: oldest at
    /// the front, newest at the back. `Tcl_CreateEnsemble` pushes at the head
    /// and `Tcl_DeleteNamespace` repeatedly removes that live head, so a token
    /// created re-entrantly by a delete callback becomes the very next victim.
    /// Weak entries mirror command-token ownership without extending a deleted
    /// ensemble's lifetime.
    namespace_ensembles:
        HashMap<NsId, Vec<Weak<tcl_cmd_core::ensemble::EnsembleToken<EnsembleDef, NameBytes>>>>,
    /// Captured real Jim library generations; Some(empty) retains intentional deletion.
    scripted_distribution_libraries: HashMap<
        tcl_registry::native_scripted_distribution::NativeScriptedLibrary,
        Vec<(String, NativeRegisteredCommandToken)>,
    >,
    /// Reached source-extension selections, independent of automatic Distribution bootstrap.
    explicit_scripted_libraries:
        Vec<tcl_registry::native_scripted_distribution::NativeScriptedLibrary>,
    /// Actual core-constructor purpose excludes subsequently loaded library scripts.
    bootstrap: NativeBootstrapPurpose,
    native_literal_world: Rc<RefCell<crate::literal_pool::NativeLiteralWorld>>,
    native_compiler_pass_owner: tcl_runtime_api::native_compiler_pass::NativeCompilerPassOwner,
    /// Export patterns per namespace (canonical name → glob patterns), set by
    /// `namespace export` and consulted by `namespace import`.
    ns_exports: HashMap<NsId, Vec<NameBytes>>,
    /// Import provenance: canonical key of a command created by `namespace
    /// import` → the canonical FQN of its origin (source) command. Lets
    /// `namespace forget` remove *only* imported commands (C
    /// `Tcl_ForgetImport`, which matches on `deleteProc == DeleteImportedCmd`),
    /// leaving a real command of the same name intact. Cleared whenever the key
    /// is re-registered (a redefine) or taken (a `rename`).
    imported_commands: HashMap<String, ImportBinding>,
    /// Stable registry identity for a builtin that has moved away from its
    /// registered spelling (via import, rename, or hide/expose).  This is
    /// deliberately distinct from the mutable `namespace origin` provenance:
    /// `rename lassign escaped` remains the `lassign` builtin for release
    /// availability purposes.
    builtin_identities: HashMap<String, String>,
    /// The `Command::Object` entries the engine installs on the registry's
    /// behalf rather than a script creating them — the `TclOO` roots
    /// (`oo::object`, `oo::class`, `oo::configurable`). They carry a registry
    /// availability gate the way a builtin does; every other object command is
    /// user-created and release-invariant. Populated at bootstrap
    /// (`cmd_oo::register`), read by
    /// [`builtin_command_visible_for_identity`](Self::builtin_command_visible_for_identity).
    ///
    /// Table key → **registry identity**, not a bare set, for the same reason
    /// [`Self::builtin_identities`] is a map: `rename oo::configurable myconf`
    /// leaves a command that is still the registry's `oo::configurable`, so
    /// the release gate must keep dating it by that name rather than by
    /// `myconf` (which the registry has never heard of, and would therefore
    /// wave through on every release).
    registry_object_roots: HashMap<String, String>,
    /// Namespace-name ⇆ opaque `NsId` arena for the Family-B `Frames`/`Namespaces`
    /// contract. The VM resolves namespaces by their canonical `String` name; this
    /// side-table mints stable `NsId` handles for them (`ns_arena[id]` is the name,
    /// `ROOT_NS` = 0 = `""`), bridging the handle-based trait to the string model.
    /// Immutable parent edge for every namespace token ever minted.
    /// Live parent/simple-name edge to the current child token.
    /// Actual Jim namespace objects, retained independently of constructed paths.
    /// Namespace-token identities permanently invalidated by deletion. Arena
    /// ids are monotonic and never reused; the global root is never dead.
    /// Namespace-token activation counts and the storage of the tokens whose
    /// deletion they are deferring.
    /// Per-namespace retained `TCL_STRING_KEYS` child-table state. The shared
    /// owner preserves resize and deletion history for `namespace children`.
    /// Per-namespace retained `TCL_STRING_KEYS` command-table state, keyed by
    /// the holder namespace of each command's canonical key. `commands` is one
    /// flat map, so the bucket order `TclTeardownNamespace` snapshots lives
    /// here instead. Builtins are registered in the VM's own bootstrap order
    /// rather than `Tcl_CreateInterp`'s, so the global holder's order is ours,
    /// not C's; every per-namespace user table is exact.
    /// Per-namespace command resolution path (`namespace path`): canonical
    /// namespace name (no leading `::`, `""` = global) → the ordered list of
    /// namespaces (canonical) consulted after the current namespace and before
    /// the global one during command lookup. Absent / empty = the default
    /// (current → global only).
    /// Per-namespace `namespace unknown` handlers (TIP 181): canonical
    /// namespace name → the handler command prefix. Consulted on a
    /// resolution miss for the *current* namespace, then the global
    /// namespace's handler (the interp default), then the plain `unknown`
    /// proc. NOT inherited by child namespaces (tclsh 8.6.16 /
    /// 9.0.4-pinned). Absent / empty = use the default chain.
    ns_unknowns: HashMap<NsId, NativeUnknownHandler>,
    /// Reentrancy guard: non-zero while a `namespace unknown` handler itself
    /// is being dispatched, so a handler whose own head is unresolvable falls
    /// through to a hard `invalid command name` instead of recursing.
    package_state: PackageState,
    authored_packages: Option<authored_package::AuthoredPackageState>,
    authored_rule_providers: authored_rule_callables::AuthoredRuleProviders,
    package_table_purpose: authored_package::PackageTablePurpose,
    /// Command prefix invoked by `package require` when no suitable package is
    /// known yet (`package unknown`).
    package_unknown: Option<Vec<u8>>,
    /// Interpreter-global `package prefer` selection policy. Tcl starts in
    /// stable mode unless the process opted into latest mode, and the latter
    /// is a one-way latch for the lifetime of the interpreter.
    package_prefer: PackagePrefer,
    /// Script and native observers attached to actual variable cells.
    variable_observers: VariableObservers,
    /// Source of [`VarTrace::id`]s; never reused within an interpreter.
    next_var_trace_id: u64,
    /// `trace add command` registrations (`rename`/`delete` ops), keyed by the
    /// command's table key.  Entries follow the command through `rename` and
    /// are dropped when it is deleted or overwritten (M16.3).
    pub(crate) cmd_traces: HashMap<CommandSidecarKey, Vec<Rc<CmdTraceEntry>>>,
    /// `trace add execution` registrations (`enter`/`leave`/`enterstep`/
    /// `leavestep` ops), keyed like [`Self::cmd_traces`] and moved on rename.
    pub(crate) exec_traces: HashMap<CommandSidecarKey, Vec<Rc<CmdTraceEntry>>>,
    /// Source→destination pairs for the `rename` windows currently open,
    /// innermost last. C creates the destination hash entry, fires the
    /// `rename` traces and only then deletes the source, and *both* entries
    /// reference the one `Command` — so for the callbacks' duration the
    /// vacating name **is** the destination command: it answers `trace info`,
    /// takes `trace add`/`remove`, and a `rename` or delete through it moves
    /// or destroys the destination (tclsh-pinned, 8.6.16 and 9.0.4 identical).
    /// The VM keys its tables by table key, so each open window records that
    /// equivalence, and [`Self::relocate_rename_state`] retargets every
    /// enclosing window when a nested rename moves the destination on again.
    rename_windows: Vec<(CommandSidecarKey, CommandSidecarKey)>,
    /// Weak registry of in-flight sidecar identities. Relocation updates live
    /// handles without retaining completed invocations.
    active_sidecar_handles: Vec<Weak<RefCell<Option<CommandSidecarKey>>>>,
    /// Step-trace scopes currently active: one entry per `enterstep`/
    /// `leavestep`-bearing trace of each traced proc whose body is running.
    /// Every command dispatched while non-empty fires these (C's step
    /// semantics); the traced proc's frame pops its own pushes on completion.
    pub(crate) exec_step_scopes: Vec<crate::exec::ExecStepScope>,
    /// Runtime lookup/observer guard generation, separate from the actual
    /// native compiler and namespace resolver cache epochs. Active chunks use
    /// it to revalidate their retained command selection boundaries.
    compilation_epochs: NativeCompilationEpochs,
    /// Set for the duration of [`Vm::run_cmd_trace_callback`]'s evaluation:
    /// C's `INTERP_TRACE_IN_PROGRESS` (`tclTrace.c` 9.0.4:1765, set only by
    /// `TraceExecutionProc`) — while an **execution** trace callback is
    /// running, no further interp-wide trace firing happens for commands
    /// *inside* that callback (`TclCheckInterpTraces` returns immediately).
    /// Without this, a step-traced proc's `enterstep` callback (e.g. `puts
    /// "…"`) would itself be step-observed, and its own `puts` dispatch would
    /// recurse into firing traces again. A `rename`/`delete` callback does
    /// **not** raise it — C's `CallCommandTraces` sets no flag — so
    /// [`Vm::run_command_trace_callback`] leaves it alone. A `Cell`, not
    /// a plain `bool` field, so a read-only dispatch-site check
    /// (`self.trace_in_progress.get()`) doesn't need `&mut self` — and so it
    /// doesn't count toward `clippy::struct_excessive_bools` alongside the
    /// interp's genuine bool-valued state (`is_safe`, `debug_frame`, …).
    pub(crate) trace_in_progress: std::cell::Cell<bool>,
    /// The commands whose `rename`/`delete` traces are currently firing. C
    /// guards those per `Command` (`CMD_TRACE_ACTIVE`, and `CMD_DYING` for a
    /// deletion), not interpreter-wide: a callback that deletes a *different*
    /// command still fires that command's own traces, nested.
    firing_cmd_traces: Vec<CommandSidecarKey>,
    /// A traced dispatch that deferred its body to a pushed frame parks its
    /// leave context here for `drive_loop` to move onto that frame — set and
    /// drained within one trampoline step.
    pub(crate) pending_exec_leave: Option<crate::exec::ExecLeaveCtx>,
    /// M16.4 — the command-resolution memo: `(epoch, cxt‹U+1›name → key)`.
    /// C caches a resolution on the name object and invalidates by interp
    /// epoch (`cmdRefEpoch`); the VM has no per-object intreps, so the memo
    /// lives here, valid only while its stored epoch equals
    /// [`Self::cmd_epoch`].  Interior-mutable because resolution is `&self`.
    /// Bumped by every mutation that can change what a name resolves to:
    /// command registration/removal (any path), `namespace path` writes,
    /// namespace deletion sweeps, and runtime-version flips (the 8.4 path-
    /// tier gate).  See `bump_cmd_epoch`.
    /// Runtime-issued speculative guard tokens and mutation-domain snapshots.
    guards: std::cell::RefCell<VmCommandGuards>,
    /// The `::unknown` binding last seen at a guard door
    /// ([`Vm::synchronise_unknown_binding_guards`]): `None` before the first
    /// door, `Some(None)` when `::unknown` was unbound there.
    #[allow(clippy::option_option)]
    guard_unknown_binding: std::cell::RefCell<Option<Option<CommandTokenIdentity>>>,
    /// What is known of command tokens by generation: their guard
    /// attestations and which of them an embedder registered.
    guarded_commands: GuardedCommands,
    /// Resolved variable cells whose traces are currently firing. Tcl's guard
    /// lives on each `Var`, so distinct elements of one array remain distinct.
    active_traces: Vec<VarId>,
    /// Frame depths at which the currently-executing `namespace eval`/`inscope`
    /// bodies started (innermost last). A namespace body runs in the frame that
    /// invoked it, so when the current frame depth matches the innermost entry
    /// we are directly in a namespace script (unqualified names alias namespace
    /// variables); inside a proc called from one, the depths differ and
    /// unqualified names are proc locals.
    ns_script_frames: Vec<usize>,
    // Shared with child interpreters (`interp create`): a child writes `puts`
    // output to the same sink and compiles dynamic scripts with the same
    // (stateless) compile service, so these are `Rc` rather than owned.
    out: Rc<RefCell<Box<dyn Write>>>,
    compiler: Option<Rc<dyn CompileService<Module = ModuleAsm>>>,
    /// The spec-pack facts this VM runs under ([`Vm::set_pack_facts`]): a
    /// unit whose site claims a stamp not among them is not admitted.
    /// Empty — the default — admits exactly the units that claim nothing.
    pack_facts: Vec<tcl_runtime_api::PackFactStamp>,
    /// The script library [`Vm::init_library`] sourced here, which a trusted
    /// child runs too, as C's `CreateChild` `Tcl_Init` does.
    initialised_library: Option<Value>,
    /// The world this interpreter is pinned to ([`Vm::pin_context`]), the
    /// registry generation it holds, and the identity those state together
    /// with the pack facts above — what a compiled unit's manifest is
    /// compared with. Its profile is the dialect profile this VM validates its
    /// builtin command surface against: [`Self::builtin_command_visible_for_surface`]
    /// consults its availability point, so a command the emulated release does
    /// not have (`lassign` at 8.4, `lpop` before 9.0) resolves like C Tcl — to
    /// `invalid command name`. Defaults to the permissive fallback profile,
    /// which hides nothing; `set_runtime_version` pins the matching plain-Tcl
    /// profile and `set_dialect_profile` pins a vendor one.
    pin: tcl_registry::model::PinnedContext,
    /// Source-command debug hook and its once-per-command delivery state.
    debug_hooks: InterpreterDebugHooks,
    /// Set by the `exit` command to the requested process code. The VM library
    /// never terminates the process itself — a standalone driver (the `tclvm`
    /// CLI) translates this into `std::process::exit`, while an embedder (the
    /// debugger, `tcl-irule-test`, `f5 explain-flow --simulate`) sees the
    /// unwinding completion and survives. `None` until `exit` runs.
    pending_exit: Option<i32>,
    /// Cache of compiled scripts for the runtime-`eval` / command-substitution
    /// path (`eval_source`), keyed by constructed namespace and source text.
    /// Compilation is a pure function of those values and the currently
    /// selected profile, so a script
    /// re-evaluated every loop iteration — a `switch`/`if`/`while` body, a
    /// `[subst]`ed command, a tcltest `-body` — compiles once instead of each
    /// time. This is the dominant cost in the tcltest workload.
    eval_cache: HashMap<(NamespacePath, tcl_lexer::SourceImage), NativeCachedModule>,
    /// Plain-dispatch counterpart of [`Self::eval_cache`] — the SAME source
    /// text compiled with [`CompileService::compile_plain_dispatch_for_profile`]
    /// instead of
    /// [`compile`](CompileService::compile), used whenever
    /// [`Self::requires_plain_command_dispatch`] is true. A step-traced proc's
    /// `if`/`while`/`foreach`/`eval`/`uplevel`/`catch`/`try`/… bodies all funnel through
    /// `eval_source`/`compile_source_cached` (their runtime builtins evaluate
    /// bodies that way), so gating the cache choice here — rather than at
    /// each call site — makes every one of them trace-visible with one change.
    /// Kept as a wholly separate map (not a mode-keyed
    /// entry in `eval_cache`) so the two compiled forms of the same source
    /// never collide or get served to the wrong mode. Both maps are cleared
    /// when the profile changes.
    eval_cache_plain: HashMap<(NamespacePath, tcl_lexer::SourceImage), NativeCachedModule>,
    /// The accumulating `errorInfo` source trace (C's `iPtr->errorInfo`): the
    /// error message followed by `while executing` / `invoked from within`
    /// frames, built up as the error unwinds through commands. `None` until the
    /// first frame is logged (which selects `while executing`). Consumed and
    /// reset when an error is caught (`catch`) or published.
    native_errors: NativeErrorState,
    pub(crate) jim_errors: JimEvaluationState,
    /// The 1-based source line of the innermost command logged into the current
    /// `errorInfo` trace (C's `iPtr->errorLine`) — the line the `(procedure …
    /// line N)` / `("while" body line N)` frames report.
    /// The word a builtin was invoked under (the source `objv[0]`, before
    /// namespace-path resolution). Lets a builtin report its invocation name in
    /// error messages — e.g. `::tcl::mathop::!` reached via `namespace path` says
    /// `wrong # args: should be "! boolean"`, not the resolved full name.
    pub(crate) native_invocation: NativeInvocationState,
    /// Open I/O channels (file handles), keyed by channel id (`file3`, …). The
    /// predefined `stdin`/`stdout`/`stderr` are not stored here; commands
    /// special-case those names.
    channels: HashMap<String, crate::cmd_chan::Channel>,
    /// Stack of file paths currently being evaluated by `source`. The top is
    /// what `info script` returns; empty when not inside a `source`.
    script_stack: Vec<String>,
    /// Active call-nesting depth — C Tcl's `interp recursionlimit`. Bounds proc
    /// recursion so an infinite loop is a *catchable* error rather than a native
    /// stack overflow (the trampoline avoids host-stack growth for a *single*
    /// activation, but `uplevel`/`catch`/`[subst]` re-enter `eval_source`, which
    /// does recurse on the host stack). Tracked on every call-frame push/pop.
    recursion_depth: usize,

    /// Native-stack safety counter for `TclOO` method dispatch (`cmd_oo.rs`'s
    /// `oo_dispatch`) — see [`OO_DISPATCH_DEPTH_LIMIT`]'s doc comment.
    /// A separate counter from `recursion_depth` for the same
    /// reason native dispatch needs: purely stack-safety bookkeeping,
    /// no Tcl-visible meaning, not swapped on coroutine suspend/resume.
    oo_dispatch_depth: usize,
    /// The host environment: the capability seam (`tcl-platform`) through which
    /// every command reaches the filesystem, clock, env, stdio, subprocess, and
    /// sockets. Defaults to the host for the target being built: the
    /// full-capability [`NativeHost`](crate::host_native::NativeHost) anywhere
    /// with an operating system under it, and `host_wasm::BrowserHost` — a
    /// JavaScript wall clock and nothing else — on `wasm32-unknown-unknown`,
    /// where the std clock is a panic rather than a syscall.
    /// [`Vm::set_host`] swaps it (e.g. for a sandboxed, WASM-posture host in
    /// capability tests). An `Rc` (not `Box`) so
    /// a command can clone a handle and pass `&dyn Host` *alongside* a `&mut Vm`
    /// borrow (the VM is itself the `ValueOps` a shared helper takes).
    host: Rc<dyn Host>,
    /// `expr rand()` / `srand()` state — the Park–Miller minimal-standard
    /// generator's 31-bit seed (`tclExecute.c`). Seeded deterministically so a
    /// fresh VM is reproducible; `srand(n)` resets it.
    rand_seed: i64,
    /// Child interpreters (`interp create`), keyed by name. Each id addresses
    /// a full [`InterpState`] in the engine's arena, sharing this one's output
    /// sink and compile service; command tables, namespaces, variables, and
    /// channels are isolated. A child is reachable both here and as a command
    /// (`Command::ChildInterp`) in this interp.
    children: HashMap<NameBytes, InterpId>,
    /// Whether this interp is safe (`interp create -safe` / `interp issafe`).
    is_safe: bool,
    /// Per-interp recursion bound (`interp recursionlimit`), default
    /// [`RECURSION_LIMIT`]. A child carries its own, independent of the parent's.
    recursion_limit: usize,
    /// Interpreter-local allocation and execution polling counters.
    counters: InterpreterCounters,
    /// Hidden token, allocation and import state owned by this interpreter.
    hidden: HiddenCommandState,
    /// `interp debug -frame` — a one-way switch (once on, stays on).
    debug_frame: bool,
    /// `interp bgerror` — the background-error handler command prefix.
    bgerror_handler: Value,
    /// `interp limit` configuration. Both limit types are enforced: `time`
    /// by [`Vm::limit_check_tick`], `commands` by [`Vm::charge_command`].
    limits: LimitSet,
    /// The `TclOO` object system's runtime state — the class/object registries,
    /// the active method-call stack (for `self`/`my`/`next`), and the current
    /// definition target (`oo::define`/`oo::objdefine`). See [`crate::cmd_oo`].
    pub(crate) oo: crate::cmd_oo::OoState,
    /// The coroutine subsystem: live coroutines, the active-driver stack (for
    /// `[info coroutine]` + the yield-boundary check), and the pending
    /// `yield`/`yieldto` request. See [`crate::cmd_coro`].
    pub(crate) coro: crate::cmd_coro::CoroSystem,
    /// Requests staged by control builtins and drained into stack ticks.
    pub(crate) pending: PendingControl,
    /// The event loop's pending timer/idle events (`after`/`vwait`/`update`).
    /// The scheduler half of the coroutine subsystem. See [`crate::cmd_event`].
    pub(crate) events: crate::cmd_event::EventQueue,
    pub(crate) timer_epoch: Rc<crate::frame::ActivationIdentity>,
    pub(crate) timer_rules: HashMap<Vec<u8>, Rc<crate::frame::ActivationIdentity>>,
    pub(crate) timer_anchor: Option<crate::retained_activation::RetainedActivation>,
    pub(crate) timer_contexts: Vec<crate::retained_activation::TimerContext>,
    /// The `thread` package's per-interpreter state — disabled until the
    /// embedder calls [`Vm::enable_threads`]. See [`crate::cmd_thread`].
    pub(crate) thread: crate::cmd_thread::ThreadSystem,
}

/// A runtime script prepared according to the compiler's command-at-a-time
/// parse plan. `prefix` contains every complete command before a malformed
/// tail; `fatal_tail` is raised only if that prefix finishes normally.
pub(crate) struct PreparedScript {
    pub(crate) prefix: Option<CompiledUnit>,
    pub(crate) fatal_tail: Option<FatalTail>,
}

pub(crate) struct OriginalJimEvaluationFrame {
    pub(crate) evaluation_frame_entered: bool,
    pub(crate) procedure_level: u32,
    pub(crate) command: Option<native_jim_lookup::OriginalJimCommandName>,
    pub(crate) is_procedure: bool,
    pub(crate) script: Option<tcl_runtime_api::jim_error_stack::JimScriptLocation<Value>>,
    pub(crate) invocation: Value,
}

#[derive(Default)]
pub(crate) struct JimEvaluationState {
    pub(crate) pending_return: tcl_runtime_api::jim_return_state::JimReturnState,
    pub(crate) stack: JimErrorStack<Value>,
    pub(crate) frames: Vec<OriginalJimEvaluationFrame>,
    pub(crate) outer: Vec<OriginalJimEvaluationFrame>,
    pub(crate) vectors: Vec<crate::exec::NativeJimVectorFrame>,
}

/// A suspended coroutine's saved per-flow execution context: the call/namespace
/// stack tails **above** the shared global entry, plus the scalar error/script
/// state and the OO execution stacks. Exchanged with the live context by
/// [`Vm::swap_flow`]. `default()` is a fresh flow rooted at the global level (an
/// about-to-start coroutine, before it has pushed any frame).
#[derive(Default)]
pub(crate) struct ParkedFlow {
    timer_anchor: Option<crate::retained_activation::RetainedActivation>,
    frames: Vec<CallFrame>,
    namespaces: NamespaceResolutionTail,
    ns_script_frames: Vec<usize>,
    recursion_depth: usize,
    error_info: Option<Vec<u8>>,
    native_error_info: Option<Value>,
    native_error_info_len: usize,
    native_error_result: Option<crate::value::WeakNativeObject>,
    native_return_options: Option<Value>,
    native_c_return_state: NativeCReturnState,
    native_error_legacy_copy: bool,
    c84_global_code_set: bool,
    /// Primitive interpreter error-code state, separate from guest globals.
    /// None is the fresh interpreter's native NONE state.
    primitive_error_code: Option<Value>,
    error_logged: ErrorLogged,
    error_stack: NativeErrorStack,
    pub(crate) jim_errors: JimEvaluationState,
    error_line: u32,
    invoked_name: Option<String>,
    original_invoked_name: Option<Value>,
    invoked_builtin_identity: Option<String>,
    script_stack: Vec<String>,
    oo: crate::cmd_oo::OoExec,
}

/// `interp limit` configuration for one interpreter — the `commands` and `time`
/// limit types. Stored, queried, and enforced.
#[derive(Clone)]
pub(crate) struct LimitSet {
    cmd_command: Value,
    cmd_granularity: i64,
    cmd_value: Option<i64>,
    time_command: Value,
    time_granularity: i64,
    /// Combined seconds + milliseconds, normalised; `None` when unset.
    time_value: Option<(i64, i64)>,
    /// Ceiling on the byte length of a single value the interp will build,
    /// `None` when unset. Not an `interp limit` type — C Tcl has no such
    /// limit — but the one bound the `commands` and `time` limits cannot
    /// express: a single opcode may allocate without dispatching a command or
    /// spending measurable time.
    value_bytes: Option<u64>,
    /// Whether a store must stay in the running procedure's own frame. Not
    /// an `interp limit` type either: an embedder's sandbox bound, so a
    /// hosted body can leave no state behind that a later call reads
    /// ([`Vm::confine_store`]).
    confined_stores: bool,
    /// The limit the running evaluation outran, kept until the limit is set
    /// again or an embedder begins the next invocation: every later dispatch
    /// fails with the same error, so a body that catches the error cannot run
    /// on past its budget, and the embedder reads which limit it was whatever
    /// the body did with the error (C's sticky `Tcl_LimitExceeded`).
    exceeded: Option<LimitKind>,
}

/// Bidirectional command-table placement, token, and presentation identity.
///
/// Most storage keys equal the ordinary constructed display spelling. A
/// collision receives an opaque key that never participates in Tcl name
/// parsing or rendering. External handles intern the token generation rather
/// than a mutable name.
#[derive(Default)]
struct CommandIdentityIndex {
    by_slot: HashMap<CommandSlot, String>,
    slot_by_key: HashMap<String, CommandSlot>,
    display_by_key: HashMap<String, tcl_core_types::NameBytes>,
    next_storage_key: u64,
    generations: CommandGenerations,
    next_generation: u64,
    handles: RefCell<CmdArena>,
}

/// Live command token generations by key, indexed both ways. One generation
/// may stay bound under several keys.
#[derive(Default)]
struct CommandGenerations {
    by_key: HashMap<String, u64>,
    keys: rustc_hash::FxHashMap<u64, Vec<String>>,
}

/// External command handles intern one exact command-token generation. Names
/// are movable placements and may be reused, so an id must never intern an FQN.
#[derive(Default)]
struct CmdArena {
    ids: HashMap<u64, u32>,
    tokens: Vec<u64>,
}

/// The name table containing one variable binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum VarTableOwner {
    Frame(usize),
    CompiledLocal { level: usize, slot: usize },
    ProcedureStatics(usize),
    Namespace(NsId),
}

/// A resolved name-table slot. The bound cell may still be absent.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct VarBinding {
    owner: VarTableOwner,
    name: tcl_core_types::NameBytes,
}

/// An alias retains either a selected C Tcl cell or a dialect-defined name.
enum VariableLinkTarget {
    Cell(VarId),
    Name(NameLink),
}

/// One resolved variable access, including an optional array element cell.
#[derive(Clone, Debug)]
struct ResolvedVar {
    binding: VarBinding,
    base_id: Option<VarId>,
    id: Option<VarId>,
    elem: Option<tcl_core_types::NameBytes>,
}

/// The direct name-table slot retained by an array operation reference.
#[derive(Clone, Debug)]
enum ArrayOperationBinding {
    Variable {
        binding: VarBinding,
        table: std::rc::Weak<()>,
        frame: Option<std::rc::Weak<crate::frame::ActivationIdentity>>,
    },
    Element,
}

/// A fixed native expression table and its registration incarnations share one
/// interpreter owner, independently of same-spelled command registrations.
#[derive(Default)]
struct FixedMathTable {
    handlers: HashMap<String, BuiltinFn>,
    registrations: HashMap<String, (u64, u64, String)>,
    generation: u64,
}

/// Actual builtin handler retained by one selected original expression site.
#[derive(Clone)]
pub(crate) struct NativeFixedMathCall {
    pub(crate) binding: tcl_runtime_api::native_compilation::NativeMathFunctionBinding,
    interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
    profile_generation: u64,
    spec: &'static tcl_registry::CommandSpec,
    handler: BuiltinFn,
}

/// Native and script observers share storage identity and absence queries.
#[derive(Default)]
struct VariableObservers {
    /// Script registrations are oldest-first within their stable cell.
    script_traces: rustc_hash::FxHashMap<VarId, Vec<VarTrace>>,
    /// Native registration is hidden from script trace introspection/removal.
    precision_cell: Option<VarId>,
    native_error_cells:
        rustc_hash::FxHashMap<VarId, tcl_registry::special_vars::NativeErrorStorageVariable>,
    /// The global `env` array cell C's `TclSetupEnv` hooks to the host
    /// environment (`EnvTraceProc`): hidden from script trace introspection
    /// and removal, and run after the script traces on the array.
    env_cell: Option<VarId>,
}

/// A single registered variable trace.
#[derive(Clone)]
struct VarTrace {
    /// Identity for the firing walk. C walks the live list through
    /// `active.nextTracePtr` and `Tcl_UntraceVar2` rewrites every active walk
    /// as it unlinks a record (`tclTrace.c` 9.0.4:2824-2842), so a trace a
    /// callback removes never fires in that pass. A minted id, never reused,
    /// survives the `Vec` reshuffling a removal causes where an index would
    /// not, and identifies one *registration* where a `(ops, command)` pair
    /// would match its own duplicates.
    id: u64,
    /// Operations this trace fires on (`read`/`write`/`unset`/`array`).
    ops: Vec<String>,
    /// The command prefix invoked as `command name1 name2 op`.
    command: Option<Value>,
    native: Option<(
        tcl_runtime_api::native_variable_trace::NativeVariableTraceToken,
        Rc<
            dyn tcl_runtime_api::native_variable_trace::NativeVariableObserver<
                    Vm,
                    Error = tcl_cmd_core::CmdError,
                >,
        >,
    )>,
    /// Registered through the deprecated 8.x `trace variable` form (C's
    /// `TCL_TRACE_OLD_STYLE`), which calls the callback with the single
    /// `rwua` letter rather than the operation name. It is deliberately not
    /// part of the `trace remove`/`trace vdelete` match, exactly as C masks
    /// the flag out there.
    old_style: bool,
}

/// The resolved storage cell used by both trace registration and firing.
///
/// Tcl marks the reached `Var` active, not its whole variable table entry: an
/// array and each one of its elements therefore have separate identities.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct VarTraceCell {
    id: Option<VarId>,
    array: Option<VarId>,
    elem: Option<tcl_core_types::NameBytes>,
}

/// One captured update retains the selected cell and its actual namespace owner.
struct CapturedVariableUpdate {
    binding: VarBinding,
    cell: VarTraceCell,
    explicit_array: Option<VarId>,
    namespace: Option<NsId>,
}

/// Amount acceptance is either an actual native conversion receipt or an
/// explicitly authored logical simulation value; the latter grants no native effects.
enum PreparedIncrementAmount {
    Native(tcl_cmd_core::native_increment::PreparedLegacyIncrementAmount),
    Authored(Value),
}

/// A trace list detached from a variable before unset callbacks run.
struct TakenVarTraces {
    cell: VarTraceCell,
    entries: Vec<VarTrace>,
    /// Whether an element access may also walk its containing array's live
    /// traces. Whole-array destruction disables this for its element pass.
    include_array: bool,
}

/// One group of variable traces a firing walks: the array's or the variable's
/// own list, still in the table, or a list an unset has already taken out of it.
enum TraceGroup {
    Live(VarId),
    Taken(Vec<VarTrace>),
}

/// One step of that walk: a live registration, re-found by id before it runs,
/// or an already-taken entry, which nothing can remove any more.
enum Step {
    Live(u64),
    Taken(VarTrace),
}

/// One variable-trace access, kept together across cell selection and firing.
#[derive(Clone, Copy)]
struct VarTraceInvocation<'a> {
    leave_error_message: bool,
    name: &'a [u8],
    op: &'a str,
    reported: (&'a [u8], Option<&'a [u8]>),
    element_access: bool,
}

/// One `trace add command|execution` registration: the op set
/// (`rename`/`delete` or `enter`/`leave`/`enterstep`/`leavestep`) and the
/// callback command prefix.  `firing` disables the entry while its own
/// callback runs (C's re-entrancy rule).
pub(crate) struct CmdTraceEntry {
    pub(crate) ops: Vec<String>,
    pub(crate) callback: Vec<u8>,
    /// The generation of the command **token** this entry hangs off, or `None`
    /// when the binding had none.
    ///
    /// C keeps the list on the `Command` itself and frees exactly that list
    /// when the token dies. The sidecar tables are keyed by command *name*, so
    /// a delete callback that binds a replacement at the same key registers on
    /// a different token under that key; only the generation tells the two
    /// lists apart. Generations are minted in binding order, so a trace on a
    /// token installed after the dying one compares greater. A hide, expose or
    /// rename moves the entry with its token, and re-stamps it.
    token: std::cell::Cell<Option<u64>>,
    firing: std::cell::Cell<bool>,
    /// Set by [`Vm::remove_cmd_trace_bytes`] when it unlinks this registration, so a
    /// walk already under way skips it — C's `Tcl_UntraceCommand` unlinks from
    /// the list the walk is following (`tclBasic.c` 9.0.4:4020-4045).
    ///
    /// The flag, rather than the entry's presence in `cmd_traces`, is what
    /// cancels a pending callback: a **delete** walk holds the dying token's
    /// list, and a callback that re-creates the command under the same name
    /// takes the *table* entry over without touching that list, so the
    /// remaining callbacks still run. Table membership cannot tell those two
    /// apart. Measured on tclsh 8.6.16 and 9.0.4.
    untraced: std::cell::Cell<bool>,
}

#[derive(Clone, Copy)]
struct ProcedureCompilationOwner<'a> {
    declaration: &'a ProcDef,
    command_namespace: Option<NsId>,
}

#[derive(Clone, Copy)]
struct ProcedureBodySource<'a> {
    image: &'a tcl_runtime_api::SourceImage,
    declaration: Option<&'a ProcDef>,
}

#[derive(Clone, Copy)]
struct ProcedureCompilationTarget<'a> {
    ordinary: tcl_runtime_api::ProcedureCompileTargetBytes<'a>,
    original: Option<&'a tcl_runtime_api::OriginalProcedureCompileRequest>,
}

struct ProcedureCompilationInputs<'a> {
    prefix: tcl_runtime_api::SourceImage,
    parameters: Vec<tcl_core_types::NameBytes>,
    namespace: &'a NamespacePath,
    original: Option<tcl_runtime_api::OriginalProcedureCompileRequest>,
}

#[cfg(test)]
impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod family_b_tests;

#[cfg(test)]
mod native_namespace_variable_fixture_tests;

#[cfg(test)]
mod native_hidden_byte_names;

#[cfg(test)]
mod compiled_variable_provider_tests;

#[cfg(test)]
mod native_array_unset_lifetime_tests;

#[cfg(test)]
mod native_result_tests;

#[cfg(test)]
#[path = "interp/native_command_resolver_tests.rs"]
mod native_command_resolver_tests;

#[cfg(test)]
mod native_child_alias_publication_tests;

#[cfg(test)]
mod byte_array_result_tests;

#[cfg(test)]
mod debug_owned_cell_tests;

#[cfg(test)]
mod native_child_constructor_tests;

#[cfg(test)]
mod original_host_publication_tests;

#[cfg(test)]
mod native_hidden_namespace_context_tests;

#[cfg(test)]
mod original_c_import_preload_cleanup_tests;

#[cfg(test)]
mod ordinary_entry_observer_tests;
