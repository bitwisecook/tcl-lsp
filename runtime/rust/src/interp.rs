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

//! The interpreter: `Tcl_Interp` + the eval loop + command dispatch.
//!
//! Builds on the value model, parse/subst, and the frame/var
//! store. This is the **interpreter-fallback** path — the AOT compiler
//! is the primary route (north star); this runs what isn't (yet) AOT-compiled
//! and what genuinely needs runtime interpretation (`eval $dynamic`, etc.).
//!
//! Closes the **command** half of the substitution engine's seam: a `[cmd]`
//! substitution recursively evaluates its inner script through this loop.
//!
//! ## Why no deferred-free queue
//!
//! A drain-queue design defers frees to
//! survive an aliasing hazard: releasing a command's argv after dispatch could
//! free the result if it aliased an argv element. We avoid the queue (and match
//! `tclObj.c`'s immediate `TclFreeObj`) because [`set_result`] **retains** the
//! result into the interp's result slot — so the slot holds an independent +1,
//! and releasing argv can never free a still-referenced result. Immediate free
//! + retain-into-result is the whole discipline.

type OriginalJimCapturedOptionPairs = Vec<(
    &'static [u8],
    tcl_runtime_api::completion_options::OptionValue<obj::Owned>,
)>;

/// Non-owning argument transport; the existing caller retains both objects.
#[derive(Clone, Copy)]
struct OriginalProcedureObjects {
    parameters_obj: Option<*mut TclObj>,
    body_obj: *mut TclObj,
}

#[cfg(test)]
mod anonymous_cell_cleanup_tests;
#[cfg(test)]
mod bootstrap_context_tests;
#[cfg(test)]
mod byte_store_retirement_tests;
#[cfg(test)]
mod c_api_error_tests;
mod cached_increment;
mod captured_rmw;
mod codegen_depth_diagnostic;
#[cfg(test)]
mod command_publication_tests;
#[cfg(test)]
pub(crate) mod constructor_diagnostics;
#[cfg(test)]
mod container_access_tests;
#[cfg(test)]
mod container_removal_tests;
mod jim_local;
mod jim_teardown;
mod native_alias_registration;
mod native_alias_roles;
mod native_append;
pub(crate) mod native_body_artifact;
mod native_children;
mod native_command_names;
mod native_command_recreation;
mod native_command_retirement;
pub use native_alias_roles::NativeAliasHeader;
#[cfg(not(target_arch = "wasm32"))]
mod native_coroutine_names;
#[cfg(test)]
mod original_error_code_setter_tests;
#[cfg(test)]
mod original_trace_outcome_tests;
pub use native_children::ChildInterpreterCommand;
mod native_compilation;
mod native_dictionary;
#[cfg(feature = "engine")]
pub(crate) mod native_host_publication;
mod native_introspection;
pub(crate) use native_dictionary::DictionaryScopeRead;
pub(crate) mod namespace_address;
mod native_ensemble_objects;
mod native_error_variables;
pub(crate) use native_error_variables::OriginalJimErrorCodeCapture;
mod command_guard_lookup;

#[cfg(test)]
fn remember_guard_sync(
    previous: Option<u64>,
    revision: Option<u64>,
    previous_policy: Option<u64>,
    policy: Option<u64>,
) {
    command_guard_lookup::remember_sync(previous, revision, previous_policy, policy);
}

#[cfg(not(test))]
const fn remember_guard_sync(_: Option<u64>, _: Option<u64>, _: Option<u64>, _: Option<u64>) {}
mod guarded_definition;
mod native_execution_constants;
mod native_hidden_invocation;
pub(crate) mod native_i64_body;
mod native_index_lookup;
mod native_jim_import;
mod native_jim_increment;
pub(crate) use native_jim_import::{OriginalJimImportEachLoop, OriginalJimImportTraversal};
#[cfg(test)]
mod native_jim_invocation_tests;
mod native_jim_links;
mod native_jim_lookup;
mod native_jim_subcommands;
pub(crate) use native_jim_lookup::OriginalJimLookupNamespace;
pub(crate) use native_jim_subcommands::OriginalJimNamespaceSubcommandOutcome;
mod native_jim_namespace;
mod native_jim_namespace_info;
pub(crate) use native_jim_namespace_info::{
    OriginalNamespaceInfoEachLoop, OriginalNamespaceInfoFrame,
};
mod native_jim_result;
pub(crate) mod native_jim_tailcall;
pub(crate) mod native_literal_pool;
mod native_namespace_names;
mod native_precision;
mod native_procedure_body;
#[cfg(test)]
mod native_procedure_relocation_tests;
mod native_procedure_resources;
pub use native_procedure_resources::NativeProcedureCommand;
pub(crate) use native_procedure_resources::OriginalProcedureBinding;

/// The actual insertion receipt, before any post-publication owner retirement.
/// A refused publication has already retained its full execution cause.
enum OriginalCommandPublication {
    Published(u64),
    Refused,
}

struct OriginalBuiltinPublication {
    namespace: NsId,
    tail: Vec<u8>,
    generation: u64,
}

struct OriginalProcedurePublication {
    namespace: NsId,
    tail: Vec<u8>,
    generation: u64,
    binding: OriginalProcedureBinding,
}
pub(crate) use native_procedure_resources::{NativeCallableProcedure, NativeProcedureDefinition};
mod execution_name_policy;
mod native_argument_usage;
pub(crate) mod native_arguments;
mod native_command_rename;
pub(crate) mod native_encoding_source;
#[cfg(test)]
#[path = "interp/native_eval_object_tests.rs"]
mod native_eval_object_tests;
mod native_event_context;
mod native_name_queries;
mod native_object_vector;
pub(crate) mod native_operation_currency;
pub(crate) mod native_scalar_context;
mod native_script;
mod native_unicode_getter;
pub(crate) use native_script::{OriginalSourceCommandReturn, OriginalSourceConstruction};
mod native_substitution;
mod native_variable_names;
mod native_variable_observers;
mod native_variable_teardown;
mod stock_ensembles;
mod variable_names;

use core::ffi::{c_char, c_int, c_void};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use native_error_headers::{NativeErrorStack, NativeReturnOptions};
use tcl_core_types::OoId;
use tcl_runtime_api::guard::{
    GuardDomain, GuardDomains, GuardError, GuardIdentity, GuardToken, OwnedGuardManager,
};
#[cfg(test)]
use tcl_runtime_api::jim_error_stack::JimErrorTrace;
use tcl_runtime_api::jim_error_stack::{JimErrorStack, JimScriptLocation};
use tcl_runtime_api::CommandEvaluation;
#[cfg(test)]
use tcl_runtime_api::RegisteredBacking;

use crate::frame::FrameStack;
#[cfg(test)]
use crate::namespace::RenameOutcome;
use crate::namespace::{CommandBinding, Namespaces, NsId, GLOBAL};
use crate::obj::{self, TclObj};
use crate::parse;

pub use tcl_core_types::Code;

/// Parse a completion-code integer the way `TclGetIntFromObj` does: the emulated
/// release's integer grammar (an optional sign, the radix prefixes that release
/// has, and octal-by-leading-zero up to 8.6), accepting the full signed **and**
/// unsigned 32-bit range (`-2147483648 ..= 4294967295`) and reducing it to an
/// `int` (so `0xFFFFFFFF` → `-1`, `2147483648` → `-2147483648`), matching C.
/// Shared by `return -code` and `try on`.
///
/// The grammar comes from the one number facility
/// ([`tcl_syntax::number::parse_whole_with`]) under `TCL_PARSE_INTEGER_ONLY`, so
/// this cannot drift from what `expr`/`incr`/`format` accept: `return -code 0d5`
/// is an error before 9.0, `0o17` before 8.5, and `-code 010` is 8 up to 8.6 but
/// 10 from 9.0. A magnitude beyond a wide (a `Big`), a float, or a NaN is not an
/// `int` — `Tcl_GetIntFromObj` rejects all three.
#[must_use]
pub(crate) fn parse_completion_int(b: &[u8]) -> Option<i32> {
    tcl_cmd_core::return_options::completion_code_integer(
        core::str::from_utf8(b).ok()?.trim(),
        tcl_syntax::number::Numbers::Runtime,
    )
}

/// A built-in command handler. Receives the full argv (`argv[0]` is the command
/// name, like Tcl's `objv`); sets the result via [`Interp::set_result`] /
/// [`Interp::set_result_bytes`] and returns a [`Code`].
pub type BuiltinFn = fn(&mut Interp, &[*mut TclObj]) -> Code;

type NativeCommandFrameDescription = Vec<(Vec<u8>, Vec<u8>)>;

/// One entry of the source-location stack (`cmdFramePtr`; PC-5) — the runtime
/// state `info frame` reports. One is pushed per script-evaluation level Tcl
/// tracks: the top-level script, a proc call, an `eval`/`uplevel` body, and a
/// `source`d file — but **not** a `[cmd]` substitution or an inline
/// `if`/`while`/`for`/`foreach` body (those run in the enclosing frame). The
/// `cmd`/`line` are updated to the currently-executing command of the
/// frame-owning script as the eval loop steps through it.
pub(crate) struct CmdFrame {
    /// The frame's location `type` (`eval`/`proc`/`source`). Explicit rather than
    /// derived: an `uplevel` body is `type eval` yet still names the invoking
    /// proc, and an `eval` body inherits the enclosing kind.
    kind: FrameKind,
    /// The file this script came from (`source`d / a proc defined in one) — the
    /// `file` key (present for `source` frames).
    file: Option<Rc<[u8]>>,
    /// The proc FQN this frame runs in (a proc call; `eval`/`uplevel` bodies
    /// inherit the enclosing proc) — the `proc` key. `None` at the global level.
    proc: Option<Vec<u8>>,
    /// The proc (call) level this frame runs in; the `level` key is the distance
    /// from the current level (`current_level - this`).
    level: usize,
    /// Omit the `level` key — C drops it when the frame's CallFrame is not on the
    /// current var-scope chain, which is the `uplevel` case (its body runs in a
    /// redirected scope).
    omit_level: bool,
    /// The **stack index** (identity, not logical level) of the CallFrame this
    /// cmd-frame runs in — C's `framePtr->framePtr`. `level` alone can't identify
    /// the frame (an `uplevel`-invoked proc shares its caller's level), so the
    /// `info frame` `level` reachability test (TclInfoFrame) walks the caller
    /// chain by this index. `0` is the global frame.
    frame_index: usize,
    /// Added to a body-relative line to get the reported `line`. `0` for
    /// top-level / `eval` / eval-defined procs (body-relative, matching tclsh);
    /// for a proc defined in a `source`d file it is the file line where the body
    /// began minus one, so its commands report file-absolute lines. An inline
    /// body (`if`/`while`/`catch`) temporarily re-points this at the sub-body's
    /// own base while it runs (`eval_shared_located_body`).
    line_base: u32,
    /// The `line_base` of the **enclosing `codePtr->source`** — the proc/lambda/
    /// eval body this frame's commands ultimately belong to — captured at frame
    /// creation and *not* moved by the inline-body `line_base` shifts above. The
    /// body-relative `errorLine` for `MakeProcError` is `line_base + <raw line> -
    /// proc_line_base` (C computes `errorLine` against `codePtr->source`, which an
    /// inline `catch`/`if` body shares with its proc).
    proc_line_base: u32,
    /// The currently-executing command at this level (the `cmd` key) and its
    /// reported source line (the `line` key).
    cmd: Vec<u8>,
    original_command: Option<obj::Owned>,
    line: u32,
    /// TclOO method context for `info frame`: `(method-name, declarer-kind,
    /// declarer-name)` where kind is `class`/`object`. Present for a method
    /// body, where C reports `method`/`class`|`object` instead of `proc`.
    /// `method-name` is empty for a constructor/destructor.
    oo: Option<(Vec<u8>, Vec<u8>, Vec<u8>)>,
    /// The lambda expression for an `apply` body's `info frame` (`lambda <expr>`
    /// in place of `proc`, C's `TclInfoFrame`). `None` for a normal proc/method.
    lambda: Option<Vec<u8>>,
}

/// A TIP 280 literal-argument location: `(objPtr, file, line)` (C's `lineLABCPtr`
/// entry) — see [`Interp::arg_locs`].
type ArgLoc = (*mut TclObj, Option<Rc<[u8]>>, u32);

/// One variable-trace callback collected during namespace teardown: the
/// variable's reported name (including any `(element)`), the trace's command
/// prefix, and whether it was registered through the deprecated 8.x
/// `trace variable` form — which decides the op word its callback receives,
/// here exactly as on the explicit-unset path.
type VarTeardownCallback = (Vec<u8>, Vec<u8>, native_variable_observers::Callback, bool);

/// A `CmdFrame`'s location type (`info frame`'s `type` key).
#[derive(Clone, Copy, PartialEq, Eq)]
enum FrameKind {
    /// Top level / an `eval` or `uplevel` body (`type eval`).
    Eval,
    /// A proc body (`type proc`).
    Proc,
    /// A `source`d file, or a proc defined in one (`type source`).
    Source,
}

impl FrameKind {
    fn as_bytes(self) -> &'static [u8] {
        match self {
            FrameKind::Eval => b"eval",
            FrameKind::Proc => b"proc",
            FrameKind::Source => b"source",
        }
    }
}

impl CmdFrame {
    /// The top-level script frame (`type eval`, global level).
    fn root() -> Self {
        CmdFrame {
            kind: FrameKind::Eval,
            file: None,
            proc: None,
            level: 0,
            omit_level: false,
            frame_index: 0,
            line_base: 0,
            proc_line_base: 0,
            cmd: Vec::new(),
            original_command: None,
            line: 1,
            oo: None,
            lambda: None,
        }
    }
}

/// The error stack-trace accumulator — the runtime's analogue of `iPtr`'s
/// `errorInfo`/`errorCode`/`errorLine`/`ERR_ALREADY_LOGGED` (PC-4). The trace is
/// built **incrementally as the error unwinds** (`TclLogCommandInfo` +
/// `MakeProcError`, `proc-call-and-stack-traces.md` §1.5), not at the throw, and
/// retained after an outermost eval. Native hidden variable traces publish the
/// original objects on a read or result reset; catch consumes the exception.
#[derive(Default, Clone)]
pub(crate) struct ExceptionState {
    /// The accumulating `errorInfo`. `None` until the first frame is appended
    /// (C's `errorInfo == NULL`) — which selects `while executing` over `invoked
    /// from within` and seeds the buffer from the result message.
    info: Option<Vec<u8>>,
    /// Actual C private object ownership and legacy-copy state.
    native: native_error_variables::NativeErrorObjects,
    /// `::errorCode` (empty ⇒ the `NONE` default is applied when published,
    /// unless [`code_explicit`](Self::code_explicit) is set).
    code: Vec<u8>,
    /// Whether `code` was set by an explicit `-errorcode` (e.g. `error m i {}`):
    /// an explicit empty code reads back empty, not the `NONE` default
    /// (error-4.5). Absent on every implicit error, so the default applies.
    code_explicit: bool,
    /// `ERR_ALREADY_LOGGED`: the current command has already been logged deeper
    /// in the same script, so its enclosing command must not re-log it.
    already_logged: bool,
    /// Primitive result retained until an actual script propagation boundary.
    primitive_getter: Option<Box<tcl_syntax::scalar_getter::NativeScalarGetterError>>,
    expression_error_stage:
        Option<Box<tcl_registry::native_expression_error::NativeExpressionErrorStage>>,
}

/// The Guest part of one actual variable trace outcome. Host refusals travel
/// separately in the producer's outer Result and never permit Guest formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VariableTraceGuestOutcome {
    /// No callback error aborts this operation.
    Complete,
    /// A genuine callback error has populated the operation's pending message.
    GuestError,
}

impl VariableTraceGuestOutcome {
    fn is_guest_error(self) -> bool {
        self == Self::GuestError
    }
}

/// How one variable access presents itself to the trace machinery — see
/// [`Interp::trace_access`], which is the only place these four are decided.
struct TraceAccess {
    /// `name1` handed to the callback: the access spelling C passes through as
    /// `part1`, with the array-element split C's `TclCallVarTraces` applies.
    reported: Vec<u8>,
    /// The element registered traces are matched against — the spelling's, or
    /// the one a link resolved to.
    match_elem: Option<Vec<u8>>,
    /// `name2` handed to the callback (C's `part2` at the point of the call).
    report_elem: Option<Vec<u8>>,
    /// The element the *access spelling* named, which is what an aborting
    /// trace's `(<type> trace on "…")` errorInfo frame reports: C snapshots
    /// `element = part2` before recovering one from a linked `Var`.
    spelling_elem: Option<Vec<u8>>,
    /// Whether the containing array's whole-array traces take part.
    whole_array: bool,
}

/// A captured slice of [`ExceptionState`] — the `errorInfo`/`errorCode`
/// accumulation — moved between flows by [`Interp::snapshot_error`] /
/// [`Interp::restore_error`] (see `coroprobe`).
pub(crate) struct ErrorSnapshot {
    currency: native_operation_currency::SavedNativeOperationCurrency,
    native: native_error_variables::NativeErrorObjects,
    jim_error_stack: JimErrorStack<obj::Owned>,
    info: Option<Vec<u8>>,
    code: Vec<u8>,
    code_explicit: bool,
    already_logged: bool,
    primitive_getter: Option<Box<tcl_syntax::scalar_getter::NativeScalarGetterError>>,
    expression_error_stage:
        Option<Box<tcl_registry::native_expression_error::NativeExpressionErrorStage>>,
}

/// Full saved error state for mandatory failed-constructor cleanup. Its real
/// entered receipt survives callback effects independently of byte projections.
pub(crate) struct ErrorStateSnapshot {
    currency: native_operation_currency::SavedNativeOperationCurrency,
    result: Result<Vec<u8>, tcl_runtime_api::NativeExecutionError>,
    exception: ExceptionState,
    jim_stack: JimErrorStack<obj::Owned>,
}

#[cfg(test)]
impl ErrorSnapshot {
    /// Observe the retained code without reaching a getter or settling an error.
    pub(crate) fn code_bytes(&self) -> &[u8] {
        &self.code
    }
}

/// Evaluation metadata retains command identity so later cold lookups remain
/// visible to error capture without acquiring a native command-name reference.
struct OriginalJimEvaluationFrame {
    procedure_level: u32,
    command: Option<u64>,
    is_procedure: bool,
    script: Option<JimScriptLocation<Vec<u8>>>,
    invocation: Vec<u8>,
}

/// Borrow argv until its synchronous invocation or parked coroutine returns.
struct JimBorrowedInvocation {
    frame_index: usize,
    argv: Vec<*mut TclObj>,
    // The active original Script owns this filename through the evaluation scope.
    script_filename: Option<*mut TclObj>,
    source: Option<Rc<native_script::OriginalSourceInvocation>>,
}

/// A coroutine's saved execution context: the per-flow interpreter state that
/// is swapped in while the coroutine runs and swapped back out when it yields
/// (`cmd_coro` / [`Interp::swap_coro_ctx`]). Shared definitions (namespaces,
/// commands, classes, channels) are *not* here — coroutines share them.
pub(crate) struct CoroContext {
    frames: FrameStack,
    cmd_frames: Vec<CmdFrame>,
    current_ns: NsId,
    recursion_depth: usize,
    script_stack: Vec<Vec<u8>>,
    return_code: Code,
    return_level: usize,
    return_options: NativeReturnOptions,
    array_operation_targets: Vec<ArrayOperationTarget>,
    active_var_trace_scopes: Vec<crate::cmd_trace::VarTraceScope>,
    native_compilation: native_compilation::CompilationExecution,
    exc: ExceptionState,
    error_stack: NativeErrorStack,
    jim_error_stack: JimErrorStack<obj::Owned>,
    jim_evaluation_frames: Vec<OriginalJimEvaluationFrame>,
    jim_invocation_borrows: Vec<JimBorrowedInvocation>,
    jim_procedure_level: u32,
    native_dispatch_depth: u32,
    deferred_tailcalls: Vec<(u32, crate::frame::PendingTailcall)>,
    error_line: u32,
    arg_lines: Vec<u32>,
    eval_depth: u32,
    oo: crate::cmd_oo::OoExec,
}

impl CoroContext {
    /// A fresh context for a new coroutine: an empty call/`info frame` stack
    /// running in `ns` (the namespace `coroutine` was invoked from, so the body
    /// resolves commands there), with default return/error/OO state.
    pub(crate) fn fresh(ns: NsId) -> CoroContext {
        CoroContext {
            frames: FrameStack::new(),
            cmd_frames: Vec::new(),
            current_ns: ns,
            recursion_depth: 0,
            script_stack: Vec::new(),
            return_code: Code::Ok,
            return_level: 1,
            return_options: NativeReturnOptions::default(),
            array_operation_targets: Vec::new(),
            active_var_trace_scopes: Vec::new(),
            native_compilation: native_compilation::CompilationExecution::default(),
            exc: ExceptionState::default(),
            error_stack: NativeErrorStack::default(),
            jim_error_stack: JimErrorStack::default(),
            jim_evaluation_frames: Vec::new(),
            jim_invocation_borrows: Vec::new(),
            jim_procedure_level: 0,
            native_dispatch_depth: 0,
            deferred_tailcalls: Vec::new(),
            error_line: 1,
            arg_lines: Vec::new(),
            eval_depth: 0,
            oo: crate::cmd_oo::OoExec::default(),
        }
    }

    /// A throwaway context used only as a temporary placeholder while the real
    /// one is swapped (immediately overwritten).
    fn placeholder() -> CoroContext {
        CoroContext::fresh(GLOBAL)
    }
}

/// A registered command. [`Command::ObjCmd`] holds an extension's
/// `Tcl_ObjCmdProc` (on `wasm32`, an index into the shared function table); see
/// `docs/design/runtime/c-extension-abi.md` §4.5.
///
/// `Clone` but not `Copy`: the dispatch lookup clones the small handle out of the
/// command table (a fn-pointer copy for `Builtin`; the target name + frozen
/// prefix for `Alias`). Cloning detaches the handle from the table so dispatch
/// can mutate the interp (and the table) without holding a borrow.
/// Stable identity of one imported-command binding. Deletion candidates retain
/// this identity across trace callbacks so a callback may replace/recreate the
/// binding without the old deletion subsequently removing the new command.
#[derive(Default)]
pub struct ImportToken;

/// `Tcl_ObjCmdProc`: the C procedure an extension registers for a command
/// (`tcl.h`). Called with the `clientData` it registered, the interpreter, and
/// the call's words; answers a completion code and leaves the result in the
/// interpreter. On `wasm32` a function pointer is an index into the shared
/// `__indirect_function_table`, so an extension module that grew that table and
/// stored its procedure there registers a procedure this type calls
/// (`c-extension-abi.md` §4.5).
pub type TclObjCmdProc =
    unsafe extern "C" fn(*mut c_void, *mut Interp, c_int, *const *mut TclObj) -> c_int;

/// `Tcl_CmdDeleteProc`: run with the `clientData` when the command is deleted.
pub type TclCmdDeleteProc = unsafe extern "C" fn(*mut c_void);

/// An extension's actual command procedure, client data and delete procedure.
/// Selected C true deletion withdraws the once-only callback while an active
/// dispatch keeps only the command allocation. Neutral and Jim record lifetime
/// still call an unconsumed delete procedure when its final handle drops.
/// Selected idle creation discard retains its independent producer contract.
pub struct ObjCommand {
    proc_: TclObjCmdProc,
    client_data: *mut c_void,
    delete_proc: Cell<Option<TclCmdDeleteProc>>,
    original_deleted: Cell<bool>,
    host_backing_retired: Cell<bool>,
    creation_purpose:
        Option<tcl_registry::native_command_creation::NativeObjectCommandCreationPurpose>,
}

impl ObjCommand {
    /// A command over `proc_`. Selected C deletion withdraws `delete_proc`
    /// once independently of the dispatch allocation; neutral and Jim record
    /// retirement calls an unconsumed callback when the last handle drops.
    /// Selected idle creation discard has its own no-callback purpose.
    #[must_use]
    pub fn new(
        proc_: TclObjCmdProc,
        client_data: *mut c_void,
        delete_proc: Option<TclCmdDeleteProc>,
    ) -> Self {
        Self {
            proc_,
            client_data,
            delete_proc: Cell::new(delete_proc),
            original_deleted: Cell::new(false),
            host_backing_retired: Cell::new(false),
            creation_purpose: None,
        }
    }

    /// Observe the actual once-only callback withdrawal in a genuine Host bridge.
    #[cfg(all(test, feature = "engine"))]
    pub(crate) fn original_delete_role_is_withdrawn(&self) -> bool {
        self.delete_proc.get().is_none() && self.original_deleted.get()
    }

    /// Withdraw before invoking, so recursive deletion cannot replay this role.
    pub(super) fn retire_original_delete_procedure(&self) {
        if let Some(delete_proc) = self.delete_proc.take() {
            if self.creation_purpose == Some(
                tcl_registry::native_command_creation::NativeObjectCommandCreationPurpose::OwnedRuntimeShim,
            ) {
                // Withdraw this actual private backing role before its Drop
                // can reenter. The C token's final objProc-null stage is separate.
                self.host_backing_retired.set(true);
            }
            // SAFETY: the actual command supplied this callback and client data.
            unsafe { delete_proc(self.client_data) };
        }
    }

    #[cfg(all(test, feature = "engine"))]
    pub(crate) fn original_host_backing_is_retired(&self) -> bool {
        self.host_backing_retired.get() && self.delete_proc.get().is_none()
    }

    /// Retain the actual producer's record purpose independently of dispatch,
    /// native name/address and object ownership permissions.
    pub(crate) fn creation_purpose(
        &self,
    ) -> Option<tcl_registry::native_command_creation::NativeObjectCommandCreationPurpose> {
        self.creation_purpose
    }

    /// Runtime-owned shim storage keeps its own destructor contract; it cannot
    /// inherit the external legacy C callback's intermediate-record discard.
    pub(crate) fn new_owned_runtime_shim(
        proc_: TclObjCmdProc,
        client_data: *mut c_void,
        delete_proc: Option<TclCmdDeleteProc>,
    ) -> Self {
        let mut command = Self::new(proc_, client_data, delete_proc);
        command.creation_purpose = Some(
            tcl_registry::native_command_creation::NativeObjectCommandCreationPurpose::OwnedRuntimeShim,
        );
        command
    }
}

impl Drop for ObjCommand {
    fn drop(&mut self) {
        if let Some(delete_proc) = self.delete_proc.take() {
            // SAFETY: the extension registered this procedure for this client
            // data; the handle is dropped once, so it is called once.
            unsafe { delete_proc(self.client_data) };
        }
    }
}

/// Result of resolving a retained command generation. A surface-gated token
/// is distinct from an absent token: imports may follow the Tcl by-name
/// replacement fallback only after their exact source generation has retired.
enum CommandGenerationLookup {
    Missing,
    Unavailable,
    Found { fqn: Vec<u8>, command: Command },
}

/// Import queries share a generation walk while retaining their separate
/// surface and retained-ensemble contracts.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ImportQueryPurpose {
    /// Procedure/ensemble introspection respects actual command availability.
    Introspection,
    /// Namespace origin projects the original command-table binding.
    Origin,
    /// OO object lookup follows the same retained imported-command tokens.
    OriginalObject,
}

/// A query result retains the actual binding generation and its current name.
/// An imported ensemble can retain a live configuration independently from its
/// source binding; its command is not replaced by a fabricated ensemble binding.
struct ImportQueryTarget {
    fqn: Vec<u8>,
    generation: u64,
    command: Command,
    retained_ensemble: Option<Rc<crate::ensemble::EnsembleToken>>,
    followed_import: bool,
}

thread_local! {
    /// Depth of active cross-interp (`ParentAlias`) calls. The Safe Base requires
    /// genuine re-entrant recursion across the parent/child boundary (a child's
    /// aliased `source` calls back into the parent, which calls `interp
    /// invokehidden $child …` back into the *same* child while its outer eval is
    /// still on the stack — exactly as C's nested `Tcl_Eval` does). The recursion
    /// is sound by construction: each interp is an `Rc<InterpState>` reached
    /// through a cloned handle, and its state is per-field interior-mutable, so a
    /// re-entry shares the state via `Rc` + `RefCell` rather than aliasing a
    /// `&mut`. This counter only **bounds** the nesting to cap native-stack growth
    /// (each cross-interp hop adds real frames).
    static CROSS_INTERP_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };

    /// Depth of active alias redirects (`Command::Alias`). An alias whose target
    /// resolves to another alias trampolines through `dispatch_alias` → `invoke`
    /// → `dispatch_alias` in native frames, so an alias *cycle* would exhaust the
    /// native stack (a WASM trap) rather than raise a Tcl error. The definition
    /// gate (`Namespaces::alias_chain_loops`, C's `TclPreventAliasLoop`)
    /// refuses every cycle at `interp alias` / `rename` time, so this counter is
    /// defence in depth: it bounds the nesting so a cycle arriving by some other
    /// route still surfaces as a catchable error.
    static ALIAS_DISPATCH_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

mod child_interpreters;
mod commands;
mod compiled_frames;
mod dispatch;
mod errors;
mod evaluation;
mod limits;
mod namespaces;
mod results;
mod setup;
mod variables;

use commands::BuiltinSurfaceVisibility;
pub use commands::Command;
#[cfg(all(test, debug_assertions))]
pub(crate) use commands::CommandVisibilityOutcome;
pub(crate) use commands::{CommandVisibilityOp, OoCommandRole};
pub(crate) use dispatch::{CallMeta, MethodFrameWhat, ProcFrame};
use evaluation::line_of;
#[cfg(debug_assertions)]
use limits::trace_native_depth_refusal;
use limits::NATIVE_EVAL_DEPTH_LIMIT;
pub(crate) use limits::{LimitSet, LIMIT_TYPES};
#[cfg(test)]
pub(crate) use setup::default_host;

/// One formal parameter of a [`ProcDef`]: a name and an optional default value.
#[derive(Clone)]
pub struct Param<O = obj::Owned> {
    pub name: Vec<u8>,
    pub default: Option<O>,
}

/// A compiled `proc` definition: its parameters, body script, and the namespace
/// it was defined in (which becomes the current namespace while it runs).
pub struct ProcDef {
    pub params: Vec<Param<obj::ProcedureObject>>,
    /// The chosen original body object; queries and entry materialise it at
    /// their own native string-access boundary.
    pub body: obj::ProcedureObject,
    pub(crate) compiler_header: Cell<tcl_dialect::NativeProcedureHeaderCompilation>,
    location: RefCell<ProcLocation>,
    /// The file the proc was defined in (`source`d), if any — makes its body
    /// frame `type source` with this `file` (`info frame`).
    pub source: Option<Rc<[u8]>>,
    /// The line base for the body's `info frame` lines: `0` (body-relative) for
    /// an eval-defined proc, or the defining file line minus one for one defined
    /// in a `source`d file (so its commands report file-absolute lines).
    pub body_line_base: u32,
    /// The compiled body, when a generated module supplied one
    /// (`tcl_codegen_proc_define_native`). `None` — the only value the `proc`
    /// command ever produces — means the source `body` is the only body.
    ///
    /// It lives *on the definition*, not in a side table, so every existing
    /// lifecycle rule already covers it: redefining the proc builds a fresh
    /// `ProcDef` with `native: None` and the new source body runs interpreted;
    /// `rename` moves the same definition and its compiled entry;
    /// `rename p ""` / `namespace delete` drop it with the definition. Nothing
    /// has to remember to invalidate anything.
    pub native: Option<NativeProcEntry>,
    future_i64_add: std::cell::OnceCell<native_i64_body::FutureNativeI64AddPlan>,
    /// Persistent raw static slots retained for this command definition.
    pub(crate) jim_parameters: Option<obj::ProcedureObject>,
    pub(crate) native_resources: native_procedure_resources::NativeProcedureResources,
    pub(crate) native_local_names: RefCell<Option<native_variable_names::NativeProcedureNameTable>>,
}

#[derive(Clone)]
pub(crate) struct ProcLocation {
    pub(crate) namespace: NsId,
    pub(crate) qualified_name: Vec<u8>,
    pub(crate) jim_namespace: Option<Rc<obj::Owned>>,
}

impl ProcDef {
    /// Namespace token selected by future procedure invocations.
    #[must_use]
    pub fn namespace(&self) -> NsId {
        self.location.borrow().namespace
    }

    /// Current command placement used by future body source frames.
    #[must_use]
    pub fn qualified_name(&self) -> Vec<u8> {
        self.location.borrow().qualified_name.clone()
    }

    pub(crate) fn location(&self) -> ProcLocation {
        self.location.borrow().clone()
    }

    pub(crate) fn relocate(&self, location: ProcLocation) {
        *self.location.borrow_mut() = location;
    }
}

/// Whether a command trace hangs off the token being deleted. Generations are
/// interpreter-unique and travel with the token through rename, hide, and
/// expose, so equality is the complete ownership test. `None` remains only as
/// a conservative fallback for an unbound internal caller.
fn cmd_trace_owned_by(token: Option<u64>, dying: Option<u64>) -> bool {
    match (token, dying) {
        (Some(token), Some(dying)) => token == dying,
        (None, None) => true,
        _ => false,
    }
}

/// The entry point of a natively lowered proc body — a wasm32 function-table
/// index on the emitted side, an ordinary function pointer here.
///
/// `argv`/`argc` are the bound call arguments. Bodies do not read them today
/// (they read their formals as named cells, which `run_proc` has already
/// bound); they are a reserved seam for a future native formal binder that
/// would read them directly. `out` is
/// caller-provided, zeroed completion storage — the same [`TclCompletionAbi`]
/// layout `tcl_invoke_argv` writes in the other direction.
///
/// # The frame contract, which only this comment can settle
///
/// A native proc entry pushes **neither** an activation **nor** a Tcl call
/// frame. [`Interp::run_proc`] has already pushed the variable frame in the
/// proc's own namespace, recorded `info level`'s words and bound the formals
/// by name, and [`Interp::run_native_body`] holds the compiled activation and
/// the `CmdFrame` for the body. This is *not* the shape a native-tier function
/// has when it is emitted as a stand-alone module function: that shape opens
/// with `tcl_codegen_activation_enter` + `tcl_codegen_frame_push`
/// (`pushes_frame = !top_level`), and reusing it here would push a second,
/// nameless frame at the caller's namespace — `namespace current`, `upvar 1`
/// and `info level` would all be off by one level. The emitter must therefore
/// give a proc entry its own prologue-free shape; it may not reuse the
/// module-function one.
///
/// Result: [`NATIVE_PROC_STATUS_RAN`] after writing `out` with one owned
/// reference on each non-null pointer, or [`NATIVE_PROC_STATUS_DECLINED`]
/// **before any observable effect**, leaving `out` untouched.
///
/// # Safety
/// The implementation must respect the whole contract above: `argv[..argc]`
/// are borrowed live objects it must not release, and on `RAN` it transfers
/// one owned reference on `out.result` and `out.options` to the runtime.
pub type NativeProcEntry = unsafe extern "C" fn(
    argv: *const *mut TclObj,
    argc: i32,
    out: *mut crate::codegen_abi::TclCompletionAbi,
) -> i32;

/// A `Tcl_Interp` handle. Cheap to clone (an `Rc` bump); all clones share one
/// [`InterpState`].
///
/// **Re-entrant cross-interp recursion** — the Safe Base's child→parent→child
/// `source`/`invokehidden` cycle, i.e. C's nested `Tcl_Eval` — works by *cloning
/// the handle* of the interp to re-enter and calling through that clone. The
/// shared state is reached via the `Rc` (a shared `&InterpState`) plus per-field
/// interior mutability, so there is never an aliased `&mut`, and a borrow
/// discipline slip is a clean panic rather than UB. Single-threaded throughout:
/// `Rc` + `RefCell`/`Cell`, no locks.
#[derive(Clone)]
pub struct Interp(Rc<InterpState>);

/// Weak actual interpreter holder. Upgrading grants no operation or native purpose.
#[derive(Clone)]
pub(crate) struct WeakInterp(Weak<InterpState>);

impl WeakInterp {
    pub(crate) fn upgrade(&self) -> Option<Interp> {
        self.0.upgrade().map(Interp)
    }
}

impl Interp {
    pub(crate) fn downgrade(&self) -> WeakInterp {
        WeakInterp(Rc::downgrade(&self.0))
    }
}

impl core::ops::Deref for Interp {
    type Target = InterpState;
    fn deref(&self) -> &InterpState {
        &self.0
    }
}

/// The shared, interior-mutable state behind an [`Interp`] handle. Owns the frame
/// stack, the command table, and the current result object (a `+1` it holds;
/// never null after `new`).
///
/// Each field is borrowed only for the span of a single operation — **never
/// across a sub-eval** — so re-entrancy (proc recursion, cross-interp calls)
/// re-borrows freshly instead of aliasing. The command resolver returns *cloned*
/// `Command` handles precisely so dispatch holds no table borrow.
/// The command-identity arena backing `Namespaces::find_command` /
/// `Commands::dispatch_id`: a bijection between an exact `(FQN, generation)`
/// token and a dense raw `CommandId`. Retained and recreated namespaces may
/// expose the same FQN simultaneously, so the display name alone is not an id.
#[derive(Default)]
struct CmdArena {
    ids: std::collections::HashMap<(Vec<u8>, u64), u32>,
    commands: Vec<(Vec<u8>, u64)>,
}

#[derive(Clone, Debug)]
struct ArrayOperationTarget {
    target: crate::vars::ArrayCellTarget,
}

#[derive(Clone, Copy)]
struct InvocationProtocols {
    string: Option<tcl_syntax::native_string::NativeStringProtocol>,
    name: Option<tcl_syntax::naming::NativeNameProtocol>,
    /// The execution point's policy when it shares the native name recipe.
    name_policy: Option<tcl_syntax::naming::NamePolicyProtocol>,
}

pub struct InterpState {
    /// Owned native cache authority; neither allocation addresses nor display names issue it.
    native_command_interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
    native_compiler_pass_owner:
        std::cell::OnceCell<tcl_runtime_api::native_compiler_pass::NativeCompilerPassOwner>,
    native_literal_world:
        Rc<RefCell<tcl_runtime_api::native_literal::NativeLiteralWorld<obj::Owned>>>,
    native_literal_arrays:
        RefCell<Vec<std::rc::Weak<native_literal_pool::NativeRuntimeLiteralArray>>>,
    native_execution_constants: RefCell<Option<[obj::Owned; 2]>>,
    jim_local_depth: Cell<usize>,
    jim_active_command_workers: RefCell<Vec<native_jim_tailcall::JimCommandReference>>,
    jim_command_references: RefCell<Vec<Weak<native_jim_tailcall::JimCommandReferenceData>>>,
    pub(crate) frames: RefCell<FrameStack>,
    /// The command-table-as-core-service: the namespace tree + the one
    /// `resolve(currentNs, name)` resolver.
    namespaces: RefCell<Namespaces>,
    native_alias_registrations:
        RefCell<tcl_registry::native_alias_registration::NativeAliasRegistrationTable<u64>>,
    /// Exact array bindings retained by nested `array` operations. The public
    /// runtime contract carries only the opaque `VarId`; this stack keeps the
    /// standalone table owner and slot private while callbacks re-enter Tcl.
    /// It is swapped with [`CoroContext::array_operation_targets`] so a yield
    /// cannot make one flow pop and release another flow's target.
    array_operation_targets: RefCell<Vec<ArrayOperationTarget>>,
    /// Variable cells whose trace callbacks are active in the current flow.
    /// This must move with coroutine execution context: a suspended callback
    /// cannot suppress or unbalance another coroutine's trace walk.
    active_var_trace_scopes: RefCell<Vec<crate::cmd_trace::VarTraceScope>>,
    /// Runtime-issued speculative guards and explicitly attested builtin IDs.
    guards: RefCell<OwnedGuardManager<u64, Option<crate::namespace::CommandGuardLookup>>>,
    guard_lookup_environment_revision: Cell<Option<u64>>,
    guard_command_lookup_policy_revision: Cell<Option<u64>>,
    /// The `::unknown` binding last seen at a guard door, so a definition,
    /// rename or deletion of it moves `UnknownHandling`.
    guard_unknown_binding: Cell<Option<Option<u64>>>,
    guarded_definitions: RefCell<Vec<guarded_definition::DefinitionRequest>>,
    guarded_commands:
        RefCell<std::collections::BTreeMap<u64, std::collections::BTreeSet<GuardIdentity>>>,
    /// Registry identity of each engine-installed builtin command generation.
    /// A command keeps this key across rename and hide, so imports of a moved
    /// token continue to apply the final builtin's dialect availability rather
    /// than treating its new display spelling as an unrelated extension.
    registry_builtin_names: RefCell<std::collections::HashMap<u64, Vec<u8>>>,
    original_byte_command_registrations: RefCell<
        std::collections::HashMap<u64, native_command_names::OriginalByteCommandRegistration>,
    >,
    original_byte_command_registration_revision:
        RefCell<Option<crate::namespace::OriginalPublicationRevision>>,
    native_compilation: native_compilation::CompilationCell,
    stock_ensembles: RefCell<stock_ensembles::StockEnsembles>,
    /// The generations of the builtins registered only to refuse a call with a
    /// "not supported" error ([`Interp::register_unsupported`]), which the
    /// backing report tells apart from a handler.
    unsupported_builtins: RefCell<std::collections::HashSet<u64>>,
    /// The current namespace for command resolution (the eval context; a proc
    /// runs in its *defining* namespace — wired with procs). Global at top level.
    current_ns: Cell<NsId>,
    /// Active proc-call nesting depth — C Tcl's `interp recursionlimit`. Bounds
    /// recursion so an infinite proc loop raises a catchable error instead of
    /// overflowing the (wasm) stack.
    recursion_depth: Cell<usize>,
    /// Per-interp recursion bound (`interp recursionlimit`), default
    /// [`RECURSION_LIMIT`]. Each child carries its own, so raising a child's
    /// limit does not affect the parent.
    recursion_limit: Cell<usize>,
    /// The package database (`package provide`/`require`/`ifneeded`/`unknown`).
    pub(crate) packages: RefCell<crate::cmd_package::PackageState>,
    /// The `source` script stack (`info script` — the file being sourced).
    script_stack: RefCell<Vec<Vec<u8>>>,
    /// Open channels (`open`/`read`/`gets`/`puts`/`close`).
    pub(crate) channels: RefCell<crate::cmd_chan::ChannelTable>,
    /// Pending `return -code`/`-level` state (`TclUpdateReturnInfo`): the code to
    /// complete with once `-level` boundaries are unwound.
    return_code: Cell<Code>,
    return_level: Cell<usize>,
    /// Non-control pairs carried by the current `return` completion. Byte
    /// storage is sufficient at this portable boundary and preserves arbitrary
    /// pre-TIP/custom option spellings for `catch`, `try`, and host adapters.
    return_options: RefCell<NativeReturnOptions>,
    /// Variable-trace registry (`trace add|remove|info variable`).
    pub(crate) traces: RefCell<crate::cmd_trace::TraceTable>,
    native_error_cells: RefCell<
        Vec<(
            tcl_registry::special_vars::NativeErrorStorageVariable,
            tcl_runtime_api::VarId,
        )>,
    >,
    /// Physical cell carrying the hidden precision trace, including relocation
    /// through an alias spelling during unset.
    native_precision_cell: Cell<Option<tcl_runtime_api::VarId>>,
    /// The error stack-trace accumulator (PC-4).
    exc: RefCell<ExceptionState>,
    /// How many errors the C API has stated ([`Interp::c_api_error`],
    /// `Tcl_SetObjErrorCode`, a refused `Tcl_PkgProvideEx`), so a C command that
    /// returns `TCL_ERROR` without stating one is told apart from one that did
    /// ([`Interp::invoke_obj_cmd`]).
    c_api_errors: Cell<u64>,
    /// `iPtr->errorLine`: the 1-based source line of the innermost command
    /// logged into the error trace, within its own script. Unlike the
    /// accumulating [`ExceptionState`], this is **persistent** interp state — it
    /// is written only by [`log_command_info`](Self::log_command_info) (C's
    /// `TclLogCommandInfo`), survives `catch` and the start of a fresh error
    /// (`error msg info` / `throw` do not touch it, matching `ERR_ALREADY_LOGGED`
    /// suppressing the log), and is read by `MakeProcError`'s `line N`.
    error_line: Cell<u32>,
    /// Child interpreters (`interp create`), each a shared [`Interp`] handle keyed
    /// by name. The name is also a command in this interp
    /// (`Command::ChildInterp`).
    children: RefCell<std::collections::BTreeMap<Vec<u8>, Interp>>,
    /// Counter for auto-generated child names (`interp0`, `interp1`, …).
    interp_counter: Cell<usize>,
    /// Hidden commands (`interp hide`): removed from the command table but
    /// invocable via `interp invokehidden`. A safe interp hides the dangerous
    /// commands here.
    hidden: RefCell<std::collections::BTreeMap<Vec<u8>, CommandBinding>>,
    /// While this interp runs as a child (`$parent eval`/`$child eval`), a `Weak`
    /// handle to its parent — for cross-interp aliases that delegate to a parent
    /// command. `Weak` (not `Rc`) so the parent→child ownership has no cycle;
    /// upgraded for the call's duration. Set on entry to a child eval
    /// ([`eval_in_child`]/[`with_child`]) and restored after.
    ///
    /// [`eval_in_child`]: Interp::eval_in_child
    /// [`with_child`]: Interp::with_child
    parent: RefCell<Weak<InterpState>>,
    /// Actual child creation remains independent of the temporary reentrant parent handle.
    native_child_interpreter: Cell<bool>,
    native_child_command_generation: Cell<Option<u64>>,
    /// Whether this interp is safe (`interp create -safe` / `interp issafe`).
    is_safe: Cell<bool>,
    /// How many of *this* interp's evals are currently on the stack (as a child:
    /// [`eval_in_child`]/[`with_child`] bump it). A child may be deleted *during*
    /// its own eval — e.g. its aliased `exit` calls `interp delete` on itself —
    /// so a non-zero count means teardown must be deferred (`pending_delete`)
    /// until the last eval unwinds, or a re-entry still on the stack would see a
    /// half-torn-down interp.
    ///
    /// [`eval_in_child`]: Interp::eval_in_child
    /// [`with_child`]: Interp::with_child
    eval_active: Cell<usize>,
    /// Marks the actual interpreter deleted. Existing activation leases keep
    /// its storage alive while every later evaluation entry rejects it.
    pending_delete: Cell<bool>,
    /// The capability host — the platform seam every file/`env`/`clock`/
    /// subprocess facility is reached through (instead of direct `std::fs`/
    /// `std::env`/`std::time`). A [`NativeHost`](tcl_host_native::NativeHost)
    /// with the full capability set on native builds; a restricted
    /// `WasiHost`/`BrowserHost` on the WASM targets (where `host.process()` /
    /// `host.sockets()` report absence rather than panicking). `RefCell` so a
    /// test (or a future safe-interp) can swap in a sandboxed host via
    /// [`set_host`](Interp::set_host); the `Rc` makes [`host`](Interp::host)
    /// hand out an independent handle, sidestepping the borrow conflict when a
    /// command needs both `&mut self` (its `ValueOps`) and the host at once.
    host: RefCell<Rc<dyn tcl_platform::Host>>,
    /// TclOO object system state (classes, objects, the method-call stack).
    /// Reached variable resolvers hold weak handles to this same current owner.
    pub(crate) oo: Rc<RefCell<crate::cmd_oo::OoState>>,
    /// The source-location stack (`cmdFramePtr`; PC-5) — what `info frame` reads.
    cmd_frames: RefCell<Vec<CmdFrame>>,
    /// TIP 280 argument lines of the command currently being dispatched: each
    /// word's file-absolute source line. A body-defining command reads its body
    /// word's line to stamp the body's source provenance (so a method/proc body
    /// reports file-relative `info frame` lines). Set per command just before
    /// dispatch; consumers must read it before re-entering the eval loop.
    arg_lines: RefCell<Vec<u32>>,
    /// TIP 280 literal-argument locations (C's `lineLABCPtr`): a stack of
    /// `(objPtr, file, line)` for each literal word of every command executing in
    /// a *sourced* context. When such a literal is later evaluated as a script
    /// (`eval`/`uplevel $bodyVar`), the eval reports `type source` at the
    /// literal's original file+line instead of `type eval` — the test-body case
    /// (tcltest's `uplevel 1 $script`). Entries are pushed before a command
    /// dispatches and truncated after it returns (dynamic scope), so the obj
    /// pointers stay valid for the lookup.
    arg_locs: RefCell<Vec<ArgLoc>>,
    /// How the emulated release evaluates the commands of the script now
    /// running: compiled, as it does a body, an `eval` or `uplevel` script, a
    /// `source`d file or stdin, or directly, as the script file `tclsh` is
    /// given. A canonical-list script is one command dispatched directly.
    evaluation: Cell<CommandEvaluation>,
    /// The evaluation the next script loop takes, where its entry names one;
    /// a loop that owns a new frame otherwise compiles, and one that shares
    /// its caller's (a command substitution) takes the caller's.
    next_evaluation: Cell<Option<CommandEvaluation>>,
    /// `(argv, index)` of each literal word of every compiled command
    /// dispatching whose command word is literal, the words the release's
    /// compiler sees as it compiles the command (`compiled_words`).
    /// Pushed before the command dispatches and truncated after, as
    /// `arg_locs`, so the argv each names stays live.
    compiled_literals: RefCell<Vec<(usize, usize)>>,
    /// Whether the commands now running are compiled as part of a procedure
    /// body, whose compile creates the locals it names (C's
    /// `envPtr->procPtr`): a body and the literal scripts and command
    /// substitutions compiled with it, not an `eval` or `uplevel` script.
    proc_body: Cell<bool>,
    /// Whether the next script loop that owns a frame runs a procedure body.
    next_proc_body: Cell<bool>,
    /// Whether the next script loop that owns a frame runs the script file
    /// `tclsh` was given, whose commands run with no command active.
    next_top_level_file: Cell<bool>,
    /// Whether the release compiles the control command whose body
    /// [`Interp::eval_control_body`] runs next, where its caller says.
    compiled_control_body: Cell<Option<bool>>,
    /// The command a namespace body runs under, which its error frame names:
    /// `eval`, or `inscope` while `namespace inscope` runs its script
    /// (`(in namespace inscope "::ns" script line N)`).
    namespace_frame_command: Cell<&'static str>,
    /// `eval_str` nesting depth. The outermost eval (depth returning to 0)
    /// publishes the accumulated error trace to the `::errorInfo`/`::errorCode`
    /// globals; nested evals (proc bodies, `[cmd]` subst, control bodies) just
    /// accumulate.
    eval_depth: Cell<u32>,
    /// The variable-trace generation: bumped every time the set of variable
    /// traces changes, through the one `GuardDomain::VariableTrace`
    /// invalidation chokepoint every add / remove / frame teardown / unset
    /// already goes through. It is what makes the per-cell trace bit
    /// (`frame::Cell::traced`) safe to cache — a stale entry is recomputed, not
    /// trusted. Starts at `1` so `0` can be the never-computed sentinel.
    var_trace_epoch: Cell<u64>,
    /// Count of commands dispatched (`info cmdcount`).
    cmd_count: Cell<u64>,
    /// Count of proc bodies run through a native entry
    /// ([`NativeProcEntry`]) rather than the source body.
    ///
    /// A test boundary, like the codegen ABI's outstanding-call-frame ledger:
    /// the two paths produce identical Tcl results by construction, so without
    /// a counter no test can tell which one ran, and a binding regression
    /// would pass every behavioural assertion silently.
    native_proc_dispatches: Cell<u64>,
    /// The code an `exit` requested, if any. `exit` does **not** terminate the
    /// host process (that would kill the embedding LSP/analysis server); it
    /// records the code here, unwinds uncatchably (`catch` re-propagates while it
    /// is set), and the embedder consumes it via [`Interp::take_exit`].
    exit_code: Cell<Option<i32>>,
    /// The last `timerate -calibrate` measurement overhead (µs per iteration),
    /// C's process-global `static double measureOverhead`. It is the default
    /// `-overhead` subtracted from a plain `timerate`; zero until calibrated.
    /// Per-interp here (not process-global) so the per-thread interps of the
    /// `thread` package do not race on it.
    measure_overhead: Cell<f64>,
    /// The `interp bgerror` handler command prefix (a Tcl list). Empty means the
    /// default. A background error (e.g. a destructor failing during implicit
    /// teardown) is reported to it: `{*}$handler $message $options`.
    bgerror: RefCell<Option<obj::Owned>>,
    /// Queued background errors `(message, options)`, drained by `update` (Tcl
    /// defers them to the event loop rather than firing at the error site).
    bg_queue: RefCell<Vec<(obj::Owned, obj::Owned)>>,
    /// The event loop's pending timer + idle events (`after`/`vwait`/`update`).
    events: RefCell<crate::cmd_event::EventQueue>,
    /// Live coroutines (`coroutine`/`yield`), keyed by command name. Each holds
    /// the coroutine's saved execution context (swapped in/out on resume/yield)
    /// and the handoff channels to its worker thread (`cmd_coro`).
    coros: RefCell<std::collections::BTreeMap<u64, crate::cmd_coro::CoroEntry>>,
    /// Original error objects awaiting the serialised coroutine probe handoff.
    /// The worker stores this before its acknowledgement and then parks; the
    /// caller consumes it before another worker can run.
    #[cfg(not(target_arch = "wasm32"))]
    coro_probe_error: RefCell<Option<ErrorSnapshot>>,
    /// The active ensemble-rewrite, if any (C's `iPtr->ensembleRewrite`): the
    /// original command words a forward / ensemble / constructor dispatch
    /// replaced, so a downstream `wrong # args` can report the call as the user
    /// wrote it. `removed` is how many leading words of `source` map to the
    /// rewritten prefix. Set at the root dispatch, cleared when it returns.
    ensemble_rewrite: RefCell<Option<EnsembleRewrite>>,
    /// Exact parser views of active stock workers, separate from ensemble state.
    handler_usage_adapters: RefCell<Vec<stock_ensembles::HandlerUsageAdapter>>,
    /// The `expr rand()`/`srand()` PRNG seed (C's `iPtr->randSeed`); `None`
    /// until first seeded (lazily from a nondeterministic source on first
    /// `rand()`, or explicitly by `srand()`). Kept in `[1, 2^31-2]`.
    #[cfg(have_tommath)]
    rand_seed: Cell<Option<i64>>,
    /// The TIP 348 error stack (`info errorstack` / the options-dict
    /// `-errorstack`): a flat list of element *values* built bottom-up as an
    /// error unwinds — `INNER <ctx>` for the innermost command, `CALL <info
    /// level 0>` per proc frame, `UP <delta>` per `uplevel` boundary. Rendered to
    /// a Tcl list on demand.
    error_stack: RefCell<NativeErrorStack>,
    jim_error_stack: RefCell<JimErrorStack<obj::Owned>>,
    jim_evaluation_frames: RefCell<Vec<OriginalJimEvaluationFrame>>,
    jim_invocation_borrows: RefCell<Vec<JimBorrowedInvocation>>,
    jim_procedure_level: Cell<u32>,
    native_dispatch_depth: Cell<u32>,
    /// Actual execution-context currency, independent of speculative command policy.
    /// Exhaustion permanently refuses new receipts; ordinary Guest mutations do not move it.
    native_operation_context_epoch: Cell<Option<u64>>,
    /// Original entered ABI operations, inherited by synchronous nested work.
    entered_native_operations: RefCell<Vec<(u64, native_operation_currency::NativeOperationEntry)>>,
    native_unicode_getter: RefCell<Option<Rc<native_unicode_getter::NativeUnicodeGetterEntry>>>,
    deferred_tailcalls: RefCell<Vec<(u32, crate::frame::PendingTailcall)>>,
    /// The `try` exception-chaining link (TIP 329 `-during`): when a `try`
    /// handler or `finally` script throws, the options dict of the *prior*
    /// exception it superseded is stashed here so the next error-options build
    /// ([`completion_options`](crate::cmd_error::completion_options)) splices it
    /// in as `-during`. Holds an
    /// owning reference (released when overwritten, cleared, or the interp drops).
    /// Cleared when an error is published/caught ([`publish_error`](Self::publish_error)),
    /// since the chain is then consumed.
    during: Cell<Option<*mut TclObj>>,
    result: Cell<*mut TclObj>,
    /// Exact command token ⇆ dense raw `CommandId` arena for
    /// `Namespaces::find_command` and `Commands::dispatch_id`.
    cmd_arena: RefCell<CmdArena>,
    /// `interp limit` configuration. The `time` limit is enforced by the loop
    /// commands; `commands` is stored for query/set only.
    limits: RefCell<LimitSet>,
    /// The limits an embedder hosting this interpreter as an engine sets on one
    /// evaluation, and where the running one stands ([`crate::budget`]).
    pub(crate) budget: RefCell<crate::budget::Budget>,
    /// Whether every store must land in the running procedure's own frame
    /// ([`Interp::confine_stores`]).
    stores_confined: Cell<bool>,
    /// Free-running counter that throttles wall-clock polling for the `time`
    /// limit (see [`Interp::limit_check_tick`]).
    limit_tick: Cell<u32>,
    /// `interp debug -frame` — the TIP 280 frame-debug switch. A one-way latch
    /// (once on, stays on), seeded from `env(TCL_INTERP_DEBUG_FRAME)` at create.
    debug_frame: Cell<bool>,
    /// The Tcl release this interpreter emulates — the single value every
    /// release-dependent semantic derives from (see
    /// [`Interp::set_runtime_version`]).  Per-interp, so a child
    /// (`interp create`) or safe interpreter can emulate a different release
    /// from its parent, exactly as each owns its own global namespace.
    runtime_version: Cell<tcl_dialect::TclVersion>,
    /// `native_invocation_dialect` projected from `dialect_profile` and
    /// `runtime_version`; cleared whenever either is replaced.
    invocation_dialect: Cell<Option<tcl_registry::InvocationDialect>>,
    /// Native protocols of `invocation_dialect`, cleared with it.
    invocation_protocols: Cell<Option<InvocationProtocols>>,
    jim_object_context: RefCell<Option<Rc<crate::native_source::NativeJimObjectContext>>>,
    jim_teardown_started: Cell<bool>,
    /// The dialect profile this interpreter validates its builtin command
    /// surface against and derives its lexing grammar from. Defaults to the
    /// permissive fallback profile, which hides nothing and lexes with the
    /// modern grammar; [`Interp::set_runtime_version`] pins the matching
    /// plain-Tcl profile.
    dialect_profile: Cell<&'static tcl_dialect::DialectProfile>,
    observed_names: RefCell<Option<execution_name_policy::ObservedNameSelection>>,
    observed_frame_storage: RefCell<Option<execution_name_policy::ObservedFrameStorage>>,
    logical_name_provider: Cell<Option<tcl_syntax::naming::NamePolicyProtocol>>,
    logical_eval_object_provider: Cell<
        Option<(
            tcl_registry::native_eval_object::LogicalEvalObjectProvider,
            &'static tcl_dialect::DialectProfile,
        )>,
    >,
    logical_source_word_provider: Cell<
        Option<(
            tcl_registry::invocation_words::LogicalSourceWordProvider,
            tcl_dialect::DialectProfileKey,
        )>,
    >,
    logical_expression_parse_provider: Cell<
        Option<(
            tcl_registry::invocation_words::LogicalExpressionParseProvider,
            tcl_dialect::DialectProfileKey,
        )>,
    >,
    /// The availability registry for `dialect_profile` — its environment's
    /// registry generation, resolved once at pin time through the ingress
    /// seam ([`crate::environment::store_for_profile`]; the generation
    /// cache guards itself with a lock, and this is consulted on every
    /// command dispatch). `None` for the permissive fallback profile,
    /// which gates nothing.
    profile_registry: Cell<Option<&'static tcl_registry::CommandRegistry>>,
    /// The availability mask `dialect_profile`'s environment answers the
    /// builtin-surface gate under — its **document authoring mask**
    /// ([`crate::environment::surface_point`]), resolved at pin time for the
    /// same reason `profile_registry` is: the generation lookup takes a
    /// lock and this is read on every command dispatch, where the retired
    /// `profile.availability_mask` was a field read. Equal to that mask for
    /// every profile an ingress can produce, pinned by the seam's own
    /// sweep.
    dialect_point: Cell<Option<tcl_dialect::model::SurfaceQuery<'static>>>,
    /// The world this interpreter is pinned to ([`Interp::pin_context`]), the
    /// registry generation it holds, and the identity those state — what the
    /// runtime reports of itself to a host comparing a module's manifest
    /// (`tcl_runtime_identity`). It holds no pack facts.
    pin: RefCell<tcl_registry::model::PinnedContext>,
    /// The `Command::OoObject` entries the engine installs on the registry's
    /// behalf (the TclOO roots `::oo::object`, `::oo::class`,
    /// `::oo::configurable`, `::oo::abstract`, `::oo::singleton`) rather than
    /// a script creating them. They carry the registry's release gate the way
    /// a builtin does; every other object command is user-created and
    /// release-invariant. Filled at bootstrap (`cmd_oo::install`), read by
    /// [`Interp::resolve_dispatchable`].
    registry_object_roots: RefCell<std::collections::HashMap<OoId, Vec<u8>>>,
    /// Builtin surface visibility per name, for the version, profile and
    /// registry it was computed under.
    builtin_surface_visibility: RefCell<BuiltinSurfaceVisibility>,
    /// TclOO owners whose visible, retained, and hidden commands are currently
    /// being retired. Delete traces are re-entrant, so the stable owner guards
    /// the one retirement transaction instead of any mutable spelling.
    retiring_oo_commands: RefCell<std::collections::HashSet<OoId>>,
    original_command_retirement_recipes:
        RefCell<Vec<native_command_retirement::OriginalCommandDeletionRecipe>>,
}

/// An ensemble-rewrite record (C's `iPtr->ensembleRewrite`, see
/// `InterpState::ensemble_rewrite`). `Tcl_WrongNumArgs` prints the first
/// `removed` words of `source` in place of the `inserted` leading words of the
/// actual (rewritten) call.
#[derive(Clone)]
pub(crate) enum EnsembleRewriteWord {
    Owned(crate::obj::Owned),
    Borrowed(crate::obj::NativeObjectLifetime),
}
impl EnsembleRewriteWord {
    fn as_ptr(&self) -> *mut TclObj {
        match self {
            Self::Owned(word) => word.as_ptr(),
            Self::Borrowed(word) => word.as_ptr(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct EnsembleRewrite {
    birth: Rc<()>,
    /// The original command words as the user wrote them (e.g. `foo test 1 2 3`),
    /// with the subcommand spell-fixed to its resolved name.
    pub source: Vec<EnsembleRewriteWord>,
    /// How many leading `source` words to print (C's `numRemovedObjs`).
    pub removed: usize,
    /// How many leading words of the rewritten call the inserted prefix occupies
    /// (C's `numInsertedObjs`); `inserted - 1` formal parameters are already
    /// filled and so dropped from the usage message.
    pub inserted: usize,
}

impl Interp {
    /// The namespace tree (read) — for the `namespace` builtin's queries. The
    /// returned `Ref` must not be held across a call that mutably borrows the
    /// namespaces (it would panic); callers use it for a single query.
    pub(crate) fn namespaces(&self) -> std::cell::Ref<'_, Namespaces> {
        self.namespaces.borrow()
    }

    /// Observe the actual table owner's policy revision before any guard door.
    /// A mutable namespace borrow alone is not a lookup-policy mutation.
    fn synchronise_command_environment_guards(&self) {
        let revision = self.namespaces().guard_lookup_environment_revision();
        let policy = self.namespaces().guard_command_lookup_policy_revision();
        let previous = self.guard_lookup_environment_revision.replace(revision);
        let previous_policy = self.guard_command_lookup_policy_revision.replace(policy);
        remember_guard_sync(previous, revision, previous_policy, policy);
        let unknown = self
            .resolve_dispatchable_with_generation(self.current_ns.get(), b"::unknown")
            .and_then(|(_, generation)| generation);
        if self
            .guard_unknown_binding
            .replace(Some(unknown))
            .is_some_and(|seen| seen != unknown)
        {
            self.invalidate_guard_domain(GuardDomain::UnknownHandling);
        }
        if revision.is_none() || policy.is_none() {
            let mut guards = self.guards.borrow_mut();
            for domain in [
                GuardDomain::CommandEnvironment,
                GuardDomain::Namespace,
                GuardDomain::UnknownHandling,
            ] {
                guards.poison(domain);
            }
        } else {
            if previous != revision {
                self.invalidate_namespace_structure_guards();
            }
            if previous_policy != policy {
                self.invalidate_guard_domain(GuardDomain::CommandEnvironment);
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn original_variable_storage_borrows_are_available(&self) -> bool {
        self.frames.try_borrow_mut().is_ok() && self.namespaces.try_borrow_mut().is_ok()
    }

    /// Whether the active variable frame is a proc call frame (vs. global /
    /// `namespace eval` scope).
    pub(crate) fn in_proc(&self) -> bool {
        self.frames.borrow().in_proc()
    }

    /// Record the code an `exit` requested. See [`InterpState::exit_code`].
    pub(crate) fn set_exit(&self, code: i32) {
        self.exit_code.set(Some(code));
    }

    /// The `timerate` calibration overhead (µs/iteration).
    /// See [`InterpState::measure_overhead`].
    pub(crate) fn measure_overhead(&self) -> f64 {
        self.measure_overhead.get()
    }

    /// Update the `timerate` calibration overhead (µs/iteration).
    pub(crate) fn set_measure_overhead(&self, us: f64) {
        self.measure_overhead.set(us);
    }

    /// Whether an `exit` is pending — the unwinding completion propagates
    /// uncatchably (C Tcl's `Tcl_Exit`), so `catch` re-propagates while it holds.
    #[must_use]
    pub fn exit_pending(&self) -> bool {
        self.exit_code.get().is_some()
    }

    /// Take the pending `exit` code, if any. An embedder calls this after an
    /// eval to learn a script asked to exit (and with what code); the runtime
    /// itself never terminates the process.
    pub fn take_exit(&self) -> Option<i32> {
        self.exit_code.take()
    }

    /// The invoking command words at call `level` (`info level N`), or `None`
    /// when the level has none.
    pub(crate) fn level_words(&self, level: usize) -> Option<Vec<Vec<u8>>> {
        self.frames
            .borrow()
            .words_at(level)
            .filter(|w| !w.is_empty())
            .map(<[Vec<u8>]>::to_vec)
    }

    /// Mutable access to the event loop's pending-event queue (`after`/`vwait`/
    /// `update`, in `cmd_event`). Callers must drop the borrow before evaluating
    /// an event script.
    pub(crate) fn events_mut(&self) -> std::cell::RefMut<'_, crate::cmd_event::EventQueue> {
        self.events.borrow_mut()
    }

    /// A second owning handle to the same interpreter state (an `Rc` clone) —
    /// handed to a coroutine worker thread (`cmd_coro`).
    pub(crate) fn clone_handle(&self) -> Interp {
        Interp(Rc::clone(&self.0))
    }

    /// Commands dispatched so far (`info cmdcount`).
    pub(crate) fn cmd_count(&self) -> u64 {
        self.cmd_count.get()
    }

    /// `expr srand(n)`: reset the PRNG seed to `n` (C's `ExprSrandFunc` — mask
    /// to 31 bits, avoid the LCG's two fixed points), then return the first
    /// `rand()` of the new sequence.
    #[cfg(have_tommath)]
    pub(crate) fn srand(&self, n: i64) -> f64 {
        self.rand_seed
            .set(Some(tcl_syntax::expr::rand::seed_from_wide(n)));
        self.rand_next()
    }

    /// `expr rand()`: advance the Park–Miller minimal-standard LCG and return a
    /// double in `(0, 1)` (C's `ExprRandFunc`). Seeds nondeterministically on
    /// first use if `srand` hasn't run.
    #[cfg(have_tommath)]
    pub(crate) fn rand_next(&self) -> f64 {
        // The generator itself — step, seed nudge and C's reciprocal-multiply
        // scaling — is the shared owner's (`tcl_syntax::expr::rand`), so this
        // engine and the VM cannot drift on a seeded stream. What stays here
        // is the seed *storage* and the nondeterministic first-seed policy.
        let mut seed = self.rand_seed.get().unwrap_or_else(|| {
            // Nondeterministic first seed, kept in [1, 2^31-2]. The wall clock
            // comes from the host (so the browser/WASI hosts seed it too).
            let t = self.host().clock().now_millis() as i64;
            tcl_syntax::expr::rand::seed_from_wide(t)
        });
        let draw = tcl_syntax::expr::rand::next_draw(&mut seed);
        self.rand_seed.set(Some(seed));
        draw
    }
}

/// Discard a freshly created (`rc 0`) object that is not going to be stored
/// (the error path of a builtin that already minted its result object).
pub(crate) fn drop_fresh(obj: *mut TclObj) {
    // SAFETY: `obj` is a live rc-0 object; retain-then-release frees it without
    // tripping the double-free guard (which fires on releasing at rc 0).
    unsafe {
        obj::incr_ref_count(obj);
        obj::decr_ref_count(obj);
    }
}

impl Default for Interp {
    fn default() -> Self {
        Interp::new()
    }
}

impl Drop for InterpState {
    fn drop(&mut self) {
        if let Some(owner) = self.native_compiler_pass_owner.get() {
            owner.retire_ordinary_entry_observation();
        }
        native_literal_pool::retire_arrays(&self.native_literal_arrays);
        if let Some(mut bindings) = self
            .namespaces
            .get_mut()
            .c_procedure_bindings_for_retirement()
        {
            bindings.extend(self.hidden.get_mut().values().filter_map(|binding| {
                match &binding.command {
                    Command::Proc(procedure) => Some(procedure.clone()),
                    _ => None,
                }
            }));
            for binding in bindings {
                binding.retire();
            }
        }
        // Selected Jim retirement runs with the last live handle so deferred
        // scripts can still dispatch. Other engines release their residual
        // result here. Child handles own independent interpreter storage.
        // SAFETY: `result` is the interp's owned reference, dropped once.
        let result = self.result.replace(core::ptr::null_mut());
        if !result.is_null() {
            unsafe { obj::decr_ref_count(result) };
        }
        // Release any pending `-during` chain link the interp still owns.
        if let Some(d) = self.during.take() {
            // SAFETY: `during` held an owning reference; drop it once.
            unsafe { obj::decr_ref_count(d) };
        }
        drop(self.native_execution_constants.get_mut().take());
    }
}

// Object byte helpers.

/// A fresh (`rc 0`) string object holding `bytes`.
pub(crate) fn new_string(bytes: &[u8]) -> *mut TclObj {
    // SAFETY: `bytes` is a valid readable slice.
    unsafe { obj::new_string_obj(bytes.as_ptr() as *const c_char, bytes.len() as obj::TclSize) }
}

/// Copy an object's string rep (shimmering if needed) into owned bytes.
pub(crate) fn obj_bytes(obj: *mut TclObj) -> Vec<u8> {
    // SAFETY: `obj` is a live object; `get_string` returns a borrowed pointer
    // into its (possibly just-generated) string rep, which we copy immediately.
    unsafe {
        let mut len: obj::TclSize = 0;
        let p = obj::get_string(obj, &mut len);
        if p.is_null() {
            return Vec::new();
        }
        core::slice::from_raw_parts(p as *const u8, len as usize).to_vec()
    }
}

/// Build a Tcl error-code value from byte-valued list elements using the
/// runtime list codec. Error codes are Tcl lists, not space-concatenated text:
/// a command or subcommand containing whitespace must remain one fourth
/// element (`TCL LOOKUP COMMAND {not here}`).
pub(crate) fn error_code_list(elements: &[&[u8]]) -> Vec<u8> {
    let mut code = Vec::new();
    for (index, element) in elements.iter().enumerate() {
        if index != 0 {
            code.push(b' ');
        }
        crate::list::append_list_element(&mut code, element, false);
    }
    code
}

/// Release each object in `objs` (each holds a `+1` taken by `eval_command`).
fn release_all(objs: &[*mut TclObj]) {
    for &o in objs {
        // SAFETY: every argv element was retained when pushed; this balances it.
        unsafe { obj::decr_ref_count(o) };
    }
}

#[cfg(test)]
fn jim_list_bytes(words: impl IntoIterator<Item = Vec<u8>>) -> Vec<u8> {
    let mut result = Vec::new();
    for (index, word) in words.into_iter().enumerate() {
        if index > 0 {
            result.push(b' ');
        }
        crate::list::append_list_element(&mut result, &word, index == 0);
    }
    result
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod native_error_log_tests;

#[cfg(test)]
mod native_error_code_projection_tests;
#[cfg(test)]
mod native_import_chain_tests;

mod native_command_traces;
mod native_error_headers;
mod native_return_instruction;
mod native_trace_result;

#[cfg(test)]
mod native_child_alias_publication_tests;

#[cfg(test)]
#[path = "interp/native_result_boundaries_tests.rs"]
mod native_result_boundaries_tests;

#[cfg(test)]
#[path = "interp/original_limit_tick_cfg_tests.rs"]
mod original_limit_tick_cfg_tests;

#[cfg(test)]
mod c86_original_source_text_tests;
