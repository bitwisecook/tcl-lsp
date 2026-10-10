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

mod captured_rmw;
mod jim_local;
mod jim_teardown;
mod native_append;
pub(crate) mod native_body_artifact;
mod native_children;
mod native_command_names;
#[cfg(not(target_arch = "wasm32"))]
mod native_coroutine_names;
pub use native_children::ChildInterpreterCommand;
mod native_compilation;
mod native_dictionary;
#[cfg(feature = "engine")]
pub(crate) mod native_host_publication;
mod native_introspection;
pub(crate) use native_dictionary::DictionaryScopeRead;
mod native_ensemble_objects;
mod native_error_variables;
mod native_execution_constants;
mod native_index_lookup;
mod native_jim_increment;
mod native_jim_links;
mod native_jim_lookup;
mod native_jim_namespace;
pub(crate) mod native_literal_pool;
mod native_namespace_names;
mod native_precision;
mod native_procedure_body;
#[cfg(test)]
mod native_procedure_relocation_tests;
mod native_procedure_resources;
pub use native_procedure_resources::NativeProcedureCommand;
pub(crate) use native_procedure_resources::{NativeCallableProcedure, NativeProcedureDefinition};
mod execution_name_policy;
mod native_command_rename;
mod native_event_context;
mod native_object_vector;
mod native_script;
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
use tcl_core_types::{OoId, RecursionLimit};
use tcl_runtime_api::RegisteredBacking;
use tcl_runtime_api::codegen_abi::NATIVE_PROC_STATUS_DECLINED;
use tcl_runtime_api::error_stack::validate_error_stack;
use tcl_runtime_api::guard::{
    GuardDomain, GuardDomains, GuardError, GuardIdentity, GuardManager, GuardToken,
    OwnedGuardManager,
};
use tcl_runtime_api::jim_error_stack::{
    JimErrorStack, JimErrorTrace, JimEvaluationFrame, JimScriptLocation, NativeErrorStackProtocol,
    capture_jim_error_frames,
};

use crate::builtins;
use crate::frame::{FrameStack, Link, VarError};
use crate::namespace::{CommandBinding, GLOBAL, Namespaces, NsId, RenameOutcome};
use crate::obj::{self, TclObj};
use crate::parse::{self, WordBody, WordPart};

/// Tcl completion codes (`tcl.h` `TCL_OK`..`TCL_CONTINUE`, plus arbitrary
/// user codes from `return -code N` / `try on N`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    Ok,
    Error,
    Return,
    Break,
    Continue,
    /// A non-standard completion code (any `int` other than 0..4), produced by
    /// `return -code N`. It propagates like an exception until a `catch`/`try`
    /// reports it; it is never `0..=4` (those canonicalise to the named variants
    /// via [`Code::from_int`]).
    Other(i32),
}

impl Code {
    /// The Tcl integer completion code (`TCL_OK`=0 … `TCL_CONTINUE`=4, or the
    /// raw value for [`Code::Other`]) — what `catch` returns and `return -code` /
    /// the `-code` options-dict entry use.
    #[must_use]
    pub(crate) fn as_int(self) -> i64 {
        match self {
            Code::Ok => 0,
            Code::Error => 1,
            Code::Return => 2,
            Code::Break => 3,
            Code::Continue => 4,
            Code::Other(n) => i64::from(n),
        }
    }

    /// Map an integer completion code to a [`Code`]: `0..=4` to the named
    /// variants, anything else to [`Code::Other`] (`TclProcessReturn` /
    /// `TclGetCompletionCodeFromObj`).
    #[must_use]
    pub(crate) fn from_int(n: i32) -> Code {
        match n {
            0 => Code::Ok,
            1 => Code::Error,
            2 => Code::Return,
            3 => Code::Break,
            4 => Code::Continue,
            other => Code::Other(other),
        }
    }
}

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
    use tcl_syntax::number::{Number, ParseFlags};

    let s = core::str::from_utf8(b).ok()?.trim();
    let flags = ParseFlags {
        integer_only: true,
        ..ParseFlags::default()
    };
    // The facility consumes the sign itself, so hand it the signed text whole —
    // stripping one here and letting it read another would accept `--5`.
    let value = match tcl_syntax::number::parse_whole_with(s, flags)? {
        // Parsed wide so `i32::MIN` and the unsigned half are both reachable;
        // the range check below is what makes this an `int`.
        Number::Int(v) => v,
        Number::Big { .. } | Number::Double(_) | Number::Nan { .. } => return None,
    };
    if value < i64::from(i32::MIN) || value > i64::from(u32::MAX) {
        return None;
    }
    Some(value as i32)
}

/// A built-in command handler. Receives the full argv (`argv[0]` is the command
/// name, like Tcl's `objv`); sets the result via [`Interp::set_result`] /
/// [`Interp::set_result_bytes`] and returns a [`Code`].
pub type BuiltinFn = fn(&mut Interp, &[*mut TclObj]) -> Code;

type NativeCommandFrameDescription = Vec<(Vec<u8>, Vec<u8>)>;

/// The kind of body-frame a proc-style caller appends to the error trace when
/// its body throws (`MakeProcError` / `MakeLambdaError`, `tclProc.c`). Both
/// truncate the name to 60 bytes (`...` on overflow) and cite the body-relative
/// `error_line` left by the innermost logged command.
pub(crate) enum ProcFrame<'a> {
    /// `(procedure "NAME" line N)` — a named `proc`. `NAME` is the invoked name.
    Proc(&'a [u8]),
    /// `(lambda term "LAMBDA" line N)` — an `apply` lambda. `LAMBDA` is the whole
    /// lambda-expression string (`{params body ?ns?}`, i.e. `argv[1]`).
    Lambda(&'a [u8]),
    /// A TclOO method body (`CommonMethErrorHandler`, `tclOOMethod.c`):
    /// `(KIND "OWNER" method "NAME" line N)`, or `(KIND "OWNER" constructor|
    /// destructor line N)`. `kind` is `object`/`class` per the declaring entity,
    /// `owner` is that entity's name, and `what` selects the method/ctor/dtor.
    Method {
        kind: &'a [u8],
        owner: &'a [u8],
        what: MethodFrameWhat<'a>,
    },
}

/// What a TclOO method-body error frame names: a method (`method "NAME"`) or a
/// constructor/destructor (a bare keyword).
pub(crate) enum MethodFrameWhat<'a> {
    Named(&'a [u8]),
    Constructor,
    Destructor,
}

/// What a proc/lambda call contributes to the diagnostic stacks: the errorInfo
/// frame (PC-4) plus the `info frame` proc FQN and defining-source (PC-5).
pub(crate) struct CallMeta<'a> {
    /// Borrowed from the active dispatcher; the procedure frame owns no extra
    /// object references merely to record its original invocation.
    pub original_argv: Option<&'a [*mut TclObj]>,
    /// The errorInfo `(procedure/lambda ...)` frame.
    pub err: ProcFrame<'a>,
    /// The proc's FQN — the `info frame` `proc` key (`None` for a lambda).
    pub fqn: Option<&'a [u8]>,
    /// The file the body was defined in (`source`d) — makes its `info frame`
    /// `type source` with this `file`.
    pub source: Option<Rc<[u8]>>,
    /// The body's `info frame` line base (file-absolute for a source-defined
    /// proc, 0 otherwise).
    pub body_line_base: u32,
    /// `(local, target)` instance-variable links to pre-install into the call
    /// frame: the method's declared variables, where `target` is the namespace
    /// storage name (== `local` for public vars, a mangled name for TIP 500
    /// private vars). Empty for procs/lambdas.
    pub link_vars: &'a [(Vec<u8>, Vec<u8>)],
    /// Current declaring-provider resolver, issued only by the reached OO body.
    pub oo_variable_resolver: Option<crate::cmd_oo::native_variables::NativeOoVariableResolver>,
    /// Return a body-level `break`/`continue` as the raw `Code` instead of the
    /// `invoked "break" outside of a loop` error. Set for TIP 558 property
    /// accessor methods, whose `configure` caller maps the loop codes to its
    /// own diagnostics.
    pub keep_loop_codes: bool,
    /// Run the body at the *current* call-frame level rather than pushing a new
    /// one (it still gets its own locals). Set for a TclOO method reached via
    /// `next`: every method in a single call chain shares the level of the
    /// original invocation, so `info level` / `upvar` / `uplevel` see through
    /// the chain to the original caller (C's call-chain execution).
    pub same_level: bool,
    /// The command prefix to use in a `wrong # args` message instead of
    /// `usage_called` (which still names the `info level` words). For a TclOO
    /// method this is the invoking `obj method` (or a forward's rewritten
    /// original invocation); `None` keeps `usage_called`.
    pub usage_prefix: Option<Vec<u8>>,
    /// The exact words to record for `info level N` of this frame, when they
    /// differ from the default `usage_called` + supplied args. Set for a TclOO
    /// constructor (the `create`/`new` invocation, e.g. `oo::object create foo`)
    /// so `info level 0` reflects the instantiation, not `<constructor>`.
    pub level_words: Option<Vec<Vec<u8>>>,
    /// List-quote the command name in a `wrong # args` message (Bug 942757) — a
    /// genuine single-word proc name (`a b  c` → `{a b  c}`, `` → `{}`). Off for
    /// `apply`/TclOO whose `usage_called`/`usage_prefix` is a pre-joined
    /// multi-word string (`apply lambdaExpr`, `obj method`) that must stay raw.
    pub quote_name: bool,
    /// The compiled body to run instead of the source one, from
    /// [`ProcDef::native`]. `None` for `apply` and every TclOO method: only a
    /// ordinary command can carry that entry; commandless methods and lambdas use
    /// their genuine original Proc body through `c_procedure`.
    pub native: Option<NativeProcEntry>,
    /// Persistent native procedure slots shared by the definition and activations.
    pub statics: Option<Rc<crate::frame::StaticVariables>>,
    pub c_procedure: Option<&'a Rc<ProcDef>>,
    /// Actual TclOO ProcedureMethod clientData, retained only after compilation.
    pub c_method_client_data: Option<&'a NativeCallableProcedure>,
    pub jim_parameters: Option<*mut TclObj>,
    pub jim_body: Option<*mut TclObj>,
    pub jim_namespace: Option<&'a Rc<obj::Owned>>,
}

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

/// The outcome of an ensemble `-unknown` handler (`EnsembleUnknownCallback`).
enum EnsembleUnknown {
    /// A non-empty result: the replacement command prefix to dispatch.
    Prefix(obj::Owned),
    /// An empty result: the handler defined the subcommand — reparse the call.
    Reparse,
    /// The handler errored (or returned a bad code); `Code` carries the failure.
    Failed(Code),
}

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

/// The 1-based source line of byte `offset` in `src` — `1 + count('\n' in
/// src[0..offset])`, C's exact `TclLogCommandInfo` loop (encoding-agnostic).
fn line_of(src: &[u8], offset: usize) -> u32 {
    1 + src[..offset.min(src.len())]
        .iter()
        .filter(|&&b| b == b'\n')
        .count() as u32
}

/// Count the newlines in `s` (the line delta between two source offsets).
fn count_newlines(s: &[u8]) -> u32 {
    s.iter().filter(|&&b| b == b'\n').count() as u32
}

/// For each element of the list literal `src`, its `(newlines-before-the-element,
/// is-literal)` — the offset-aware complement to `split_list`, used to line-track
/// `{*}`-expanded literal elements (C's `TclListLines`). Returns `None` for a
/// non-UTF-8 / malformed list (the caller then falls back to body-relative).
fn scan_list_offsets(src: &[u8]) -> Option<Vec<(u32, bool)>> {
    let s = core::str::from_utf8(src).ok()?;
    let mut out = Vec::new();
    let mut pos = 0;
    loop {
        match tcl_syntax::list::find_element(s, pos) {
            Ok(Some(e)) => {
                out.push((count_newlines(&src[..e.value.start]), e.literal));
                pos = e.next;
            }
            Ok(None) => break,
            Err(_) => return None,
        }
    }
    Some(out)
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
    native: native_error_variables::NativeErrorObjects,
    jim_error_stack: JimErrorStack<obj::Owned>,
    info: Option<Vec<u8>>,
    code: Vec<u8>,
    code_explicit: bool,
    primitive_getter: Option<Box<tcl_syntax::scalar_getter::NativeScalarGetterError>>,
    expression_error_stage:
        Option<Box<tcl_registry::native_expression_error::NativeExpressionErrorStage>>,
}

/// Diagnostic view of an executing argv. The dispatch caller owns every
/// object until the corresponding invocation scope returns, including while
/// a coroutine's native stack is parked. This receipt neither retains objects
/// nor generates their string representations.
struct JimBorrowedInvocation {
    frame_index: usize,
    argv: Vec<*mut TclObj>,
    // The active original Script owns this filename through the evaluation scope.
    script_filename: Option<*mut TclObj>,
}

/// Restore the diagnostic borrow stack before the caller releases its argv.
/// Coroutine handoff swaps this stack with its evaluation frames; resumption
/// restores the same flow before this native invocation can return or unwind.
struct JimInvocationScope {
    interp: Interp,
    previous_len: usize,
}

/// Restore the actual original-script command frame before its argv retire.
struct JimEvaluationScope {
    interp: Interp,
    previous_len: usize,
    previous_borrows_len: usize,
}

impl Drop for JimEvaluationScope {
    fn drop(&mut self) {
        self.interp
            .jim_invocation_borrows
            .borrow_mut()
            .truncate(self.previous_borrows_len);
        self.interp
            .jim_evaluation_frames
            .borrow_mut()
            .truncate(self.previous_len);
    }
}

impl Drop for JimInvocationScope {
    fn drop(&mut self) {
        self.interp
            .jim_invocation_borrows
            .borrow_mut()
            .truncate(self.previous_len);
    }
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
    jim_evaluation_frames: Vec<JimEvaluationFrame<Vec<u8>>>,
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

/// Why a hidden-table move did or did not happen. The variants are in C's
/// own check order, which is observable when more than one applies —
/// `finish_command_visibility` is the only thing that words them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandVisibilityOutcome {
    Moved,
    /// The source did not resolve (hide) or is not in the hidden table
    /// (expose).
    Missing,
    /// The source resolved outside the global namespace (hide only).
    NonGlobal,
    /// The destination is already taken.
    Collision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandVisibilityOp {
    Hide,
    Expose,
}

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

/// An extension's command: the procedure, its `clientData` and its delete
/// procedure, registered through `Tcl_CreateObjCommand`.
///
/// The delete procedure runs when the last handle to the command goes: at the
/// deletion, a replacement or a `rename` to the empty name when the command is
/// idle, and once the call that is running it returns when it is not — which
/// keeps the `clientData` live for as long as the command's own procedure
/// executes, where C Tcl runs the procedure at the deletion itself. The table,
/// a displaced binding and an interpreter's teardown are all just drops of the
/// handle.
pub struct ObjCommand {
    proc_: TclObjCmdProc,
    client_data: *mut c_void,
    delete_proc: Option<TclCmdDeleteProc>,
}

impl ObjCommand {
    /// A command over `proc_`; `delete_proc` is called with `client_data` once,
    /// when the command's last handle drops.
    #[must_use]
    pub fn new(
        proc_: TclObjCmdProc,
        client_data: *mut c_void,
        delete_proc: Option<TclCmdDeleteProc>,
    ) -> Self {
        Self {
            proc_,
            client_data,
            delete_proc,
        }
    }
}

impl Drop for ObjCommand {
    fn drop(&mut self) {
        if let Some(delete_proc) = self.delete_proc {
            // SAFETY: the extension registered this procedure for this client
            // data; the handle is dropped once, so it is called once.
            unsafe { delete_proc(self.client_data) };
        }
    }
}

#[derive(Clone)]
pub enum Command {
    /// A native Rust handler.
    Builtin(BuiltinFn),
    /// An extension's command, registered through `Tcl_CreateObjCommand`: a
    /// C procedure called with the call's words as `objv`. Behind an `Rc` so
    /// the dispatch-time clone is a count, and so the delete procedure runs
    /// exactly once however the binding goes (see [`ObjCommand`]).
    ObjCmd(Rc<ObjCommand>),
    /// An `interp alias`: dispatch re-resolves `target` **by name, anchored at
    /// the global namespace, on every call** (so it lazily observes the target's
    /// *deletion* but does NOT follow its *rename* — the stored name simply stops
    /// resolving), then prepends the frozen `prefix` words to the caller's args.
    /// See `docs/design/runtime/rename-alias.md` §4.
    Alias {
        target: Vec<u8>,
        prefix: Vec<Vec<u8>>,
        /// Registration report retained independently from the lookup slot.
        publication_name: Vec<u8>,
        /// Jim aliases retain their actual original prefix objects.
        jim_prefix: Option<obj::Owned>,
        /// Per-instance identity — see [`Command::is_same_binding`].
        identity: Rc<()>,
    },
    /// A `namespace import` redirect. The source generation is the retained
    /// command-token identity; `source` is its mutable Tcl-facing projection
    /// and the deliberate fallback after command replacement adopts a fresh
    /// generation. Together they let every import follow rename/hide/expose
    /// without switching to a replacement installed at the vacated spelling.
    /// Ensemble imports additionally retain the ensemble's configuration token.
    Imported {
        source: Vec<u8>,
        source_generation: u64,
        ensemble: Option<Rc<crate::ensemble::EnsembleToken>>,
        identity: Rc<ImportToken>,
    },
    /// A `namespace ensemble`: dispatch maps `argv[1]` (a subcommand) to a target
    /// command prefix (`-map`, else `<ns>::<sub>`) and forwards `argv[2..]` — the
    /// generalised `dict for`→`::tcl::dict::for` redirect. See [`crate::ensemble`].
    Ensemble(Rc<crate::ensemble::EnsembleToken>),
    /// A user procedure (`proc`). Dispatch pushes a call frame, binds the args to
    /// the params (defaults + an `args` catch-all), runs the body in the proc's
    /// defining namespace, and maps a body-level `return` to `Ok`. Behind an `Rc`
    /// so the dispatch-time clone of the command handle is O(1), not a body copy.
    Proc(NativeProcedureCommand),
    /// A child interpreter, addressable as a command (`$child eval …`). The
    /// `Vec<u8>` is the child's name; dispatch routes the subcommand to the child
    /// `Interp` stored in [`Interp::children`].
    ChildInterp(ChildInterpreterCommand),
    /// A TclOO object or class, addressable as a command (`$obj method …`,
    /// `Class new`). The opaque token remains stable across rename and same-name
    /// recreation; Tcl-facing names are projections owned by `OoState`.
    OoObject(tcl_core_types::OoId),
    /// The private per-object `my` dispatcher, carrying the same stable owner
    /// identity as the object command rather than rediscovering it by name.
    OoMy(tcl_core_types::OoId),
    /// The private per-object `myclass` dispatcher.
    OoMyClass(tcl_core_types::OoId),
    /// A cross-interp alias installed in a *child* interp that delegates to a
    /// command in the *parent* (`interp alias child name {} parentCmd …`). When
    /// invoked, it runs `target` (+ `prefix` + the call args) in the parent.
    ParentAlias {
        target: Vec<u8>,
        prefix: Vec<Vec<u8>>,
        /// Selected child registration spelling, independent of command lookup.
        publication_name: Vec<u8>,
        /// Jim's original prefix List belongs to the live parent interpreter.
        /// Reported byte fields do not substitute for this original owner.
        jim_prefix: Option<obj::Owned>,
        /// Per-instance identity — see [`Command::is_same_binding`].
        identity: Rc<()>,
    },
}

/// Which command token owned by one TclOO identity a binding exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OoCommandRole {
    Object,
    My,
    MyClass,
}

const OO_COMMAND_RETIREMENT_ORDER: [OoCommandRole; 3] = [
    OoCommandRole::Object,
    OoCommandRole::MyClass,
    OoCommandRole::My,
];
const OO_PRIVATE_COMMAND_ROLES: [OoCommandRole; 2] = [OoCommandRole::MyClass, OoCommandRole::My];

impl Command {
    /// Stable TclOO identity and role carried by this command, if any.
    pub(crate) fn oo_binding(&self) -> Option<(OoId, OoCommandRole)> {
        match self {
            Self::OoObject(id) => Some((*id, OoCommandRole::Object)),
            Self::OoMy(id) => Some((*id, OoCommandRole::My)),
            Self::OoMyClass(id) => Some((*id, OoCommandRole::MyClass)),
            _ => None,
        }
    }

    /// The object lifecycle token this command owns. Private dispatchers carry
    /// the same identity but their replacement does not destroy the object.
    pub(crate) fn oo_object(&self) -> Option<OoId> {
        match self {
            Self::OoObject(id) => Some(*id),
            _ => None,
        }
    }

    /// Whether `self` and `other` are the **same** command binding rather than
    /// two bindings that happen to look alike.
    ///
    /// C answers this with the `Command *` token, and a command-delete trace is
    /// exactly where the difference shows: a callback that re-creates the
    /// command it is being told about (`proc foo {} …`) leaves a *different*
    /// command at the same name, and C's deletion — which owns a captured token
    /// whose hash entry the new command has taken over — must leave it alone.
    /// Every shape that owns an `Rc` compares by pointer for that reason; the
    /// rest carry their whole identity in their fields.
    pub(crate) fn is_same_binding(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Builtin(a), Self::Builtin(b)) => std::ptr::fn_addr_eq(*a, *b),
            (Self::ObjCmd(a), Self::ObjCmd(b)) => Rc::ptr_eq(a, b),
            (Self::Proc(a), Self::Proc(b)) => a.is_same_binding(b),
            (Self::Ensemble(a), Self::Ensemble(b)) => Rc::ptr_eq(a, b),
            (Self::Imported { identity: a, .. }, Self::Imported { identity: b, .. }) => {
                Rc::ptr_eq(a, b)
            }
            // Aliases carry a token like every other `Rc`-owning shape: a
            // delete trace that recreates `foo` as an *identical* alias
            // leaves a different command at the name, and C's deletion must
            // leave it alone. Structural equality said "same binding" and
            // deleted the new one.
            (Self::Alias { identity: a, .. }, Self::Alias { identity: b, .. })
            | (Self::ParentAlias { identity: a, .. }, Self::ParentAlias { identity: b, .. }) => {
                Rc::ptr_eq(a, b)
            }
            (Self::ChildInterp(a), Self::ChildInterp(b)) => a.same_allocation(b),
            (Self::OoObject(a), Self::OoObject(b))
            | (Self::OoMy(a), Self::OoMy(b))
            | (Self::OoMyClass(a), Self::OoMyClass(b)) => a == b,
            _ => false,
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

/// A selected miss retains its own lookup context instead of restarting lookup
/// in the variable frame restored after a tailcall or alias invocation.
enum CommandDispatchSelection {
    Unselected,
    LookupAt(NsId, tcl_registry::command_lookup::CommandLookupOrigin),
    Bound(Vec<u8>, CommandBinding),
    Missing {
        lookup: NsId,
        caller: NsId,
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    },
}

/// Prepared original unknown-prefix words and the selected handler. Keeping
/// preparation off the recursive invocation stack leaves only the actual
/// callback ownership and namespace restoration live during re-entry.
struct PreparedMissingCommand {
    owned: Vec<obj::Owned>,
    words: Vec<*mut TclObj>,
    selected: Option<(Command, Option<u64>)>,
    fqn: Option<Vec<u8>>,
}

impl From<Option<(Vec<u8>, CommandBinding)>> for CommandDispatchSelection {
    fn from(binding: Option<(Vec<u8>, CommandBinding)>) -> Self {
        binding.map_or(Self::Unselected, |(name, binding)| {
            Self::Bound(name, binding)
        })
    }
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

/// Maximum nested alias-redirect depth — the native-stack bound on
/// alias-of-alias chains, matching [`NATIVE_EVAL_DEPTH_LIMIT`]'s budget. Real
/// chains are a hop or two; only a cycle the definition gate somehow missed
/// gets anywhere near it.
const MAX_ALIAS_DISPATCH_DEPTH: u32 = 128;

/// Maximum nested cross-interp call depth (the native-stack bound for the
/// re-entrant parent⇄child recursion the Safe Base needs). Generous enough for
/// safe-base setup (a handful of hops) while still catching runaway recursion.
const MAX_CROSS_INTERP_DEPTH: u32 = 80;

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
    jim_active_command_workers: RefCell<Vec<(u64, Command)>>,
    pub(crate) frames: RefCell<FrameStack>,
    /// The command-table-as-core-service: the namespace tree + the one
    /// `resolve(currentNs, name)` resolver.
    namespaces: RefCell<Namespaces>,
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
    guards: RefCell<OwnedGuardManager<u64>>,
    guarded_commands:
        RefCell<std::collections::BTreeMap<u64, std::collections::BTreeSet<GuardIdentity>>>,
    /// Registry identity of each engine-installed builtin command generation.
    /// A command keeps this key across rename and hide, so imports of a moved
    /// token continue to apply the final builtin's dialect availability rather
    /// than treating its new display spelling as an unrelated extension.
    registry_builtin_names: RefCell<std::collections::HashMap<u64, Vec<u8>>>,
    native_compilation: RefCell<native_compilation::CompilationState>,
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
    jim_evaluation_frames: RefCell<Vec<JimEvaluationFrame<Vec<u8>>>>,
    jim_invocation_borrows: RefCell<Vec<JimBorrowedInvocation>>,
    jim_procedure_level: Cell<u32>,
    native_dispatch_depth: Cell<u32>,
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
    #[cfg(have_tommath)]
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
    /// TclOO owners whose visible, retained, and hidden commands are currently
    /// being retired. Delete traces are re-entrant, so the stable owner guards
    /// the one retirement transaction instead of any mutable spelling.
    retiring_oo_commands: RefCell<std::collections::HashSet<OoId>>,
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

/// `interp limit` configuration for one interpreter — the `commands` and `time`
/// limit types. The `time` limit is enforced (polled by the loop commands);
/// `commands` is stored for query/set only.
#[derive(Clone)]
pub(crate) struct LimitSet {
    cmd_command: Vec<u8>,
    cmd_granularity: i64,
    cmd_value: Option<i64>,
    time_command: Vec<u8>,
    time_granularity: i64,
    /// Absolute wall-clock deadline as `(seconds, milliseconds)`; `None` unset.
    time_value: Option<(i64, i64)>,
}

impl Default for LimitSet {
    fn default() -> Self {
        Self {
            cmd_command: Vec::new(),
            cmd_granularity: 1,
            cmd_value: None,
            time_command: Vec::new(),
            time_granularity: 10,
            time_value: None,
        }
    }
}

/// The proc-call recursion bound (C Tcl's default `interp recursionlimit`).
const RECURSION_LIMIT: usize = 1000;

/// A native-stack safety net over **every** script-body evaluation and nested
/// command-dispatch entry —
/// control-flow bodies (`if`/`while`/`for`/`foreach`/…), proc bodies,
/// `eval`/`uplevel`/`source`, and command substitution — checked against
/// [`Interp::eval_depth`] in [`Interp::eval_script_mode`] and the actual nested
/// dispatch depth, independently of
/// [`RECURSION_LIMIT`]/[`Interp::recursion_limit`].
///
/// This is a genuinely different concern from `recursion_limit`:
/// `recursion_limit` is the user-configurable, Tcl-visible `interp
/// recursionlimit` budget (bounding *proc-call* nesting only, matching C
/// Tcl's `iPtr->numLevels`), whereas this crate's interpreter is a
/// tree-walking evaluator — unlike C Tcl's bytecode-compiled control
/// structures (which execute via a flat instruction loop, no per-nesting-
/// level native recursion), *every* nested body here costs one more group
/// of native Rust stack frames (`eval_command` → command dispatch →
/// `eval_control_body`/`run_proc` → `eval_framed`/`eval_shared_located_body`
/// → `eval_script_mode`, recursively). C Tcl has no equivalent native-stack
/// hazard for compiled control flow, so there is no directly-analogous
/// upstream constant to match here.
///
/// Empirically measured on this crate's native (non-WASM) build, run on a
/// plain 2 MiB thread stack (`cargo test`'s per-test default — the same
/// class of ambient stack budget the analyser also runs under): unguarded
/// nested `foreach` bodies overflow the stack (SIGABRT)
/// between depth 200 and 250, and — more surprisingly — plain unbounded
/// recursive *proc calls* overflow **before ever reaching the existing
/// `RECURSION_LIMIT` of 1000**, meaning that pre-existing, purely
/// Tcl-semantic cap was never actually a safe backstop against a native
/// crash on an ordinary thread stack, let alone the smaller stack a WASM
/// host may give this module (this crate is `#[cfg(not(target_arch =
/// "wasm32"))]`-agnostic and, per this crate's `Cargo.toml`, "eventually
/// builds for wasm32 as a cdylib" — a WASM host's stack budget is entirely
/// outside this crate's control, unlike a native embedding where a caller
/// can choose to run `eval_str` on a generously-sized thread the way
/// `tcl-lsp-server`/`tcl-debugger`/etc. do).
///
/// 128 is deliberately conservative — comfortably under half the measured
/// 2 MiB-stack crash threshold, so it holds real margin even against a
/// meaningfully smaller WASM stack, while still being far more headroom
/// than realistic (even generated/templated) Tcl needs: legitimate scripts
/// essentially never combine proc-call depth, control-flow nesting, and
/// command-substitution nesting to a combined total anywhere near 128.
/// Tripping it raises the same catchable `"too many nested evaluations
/// (infinite loop?)"` error `RECURSION_LIMIT` uses — the failure mode is
/// conceptually identical (too much nesting), just caught earlier for
/// native-safety reasons independent of the user-configurable budget.
const NATIVE_EVAL_DEPTH_LIMIT: RecursionLimit = RecursionLimit(128);

/// Parse an `interp recursionlimit` integer the way C's `Tcl_GetIntFromObj`
/// reports: a decimal that overflows `i64` is "too large to represent", a
/// non-numeric value is "expected integer but got …".
fn parse_recursion_limit(bytes: &[u8]) -> Result<i64, Vec<u8>> {
    let not_int = || {
        let mut m = b"expected integer but got \"".to_vec();
        m.extend_from_slice(bytes);
        m.push(b'"');
        m
    };
    let s = match std::str::from_utf8(bytes) {
        Ok(s) => s.trim(),
        Err(_) => return Err(not_int()),
    };
    if let Ok(n) = s.parse::<i64>() {
        return Ok(n);
    }
    // A run of decimal digits (with an optional sign) that failed to parse
    // overflowed the integer range; anything else is simply not an integer.
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    if !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit()) {
        return Err(b"integer value too large to represent".to_vec());
    }
    Err(not_int())
}

/// Build a Tcl dict (flat key/value list) object from `pairs`, releasing the
/// builder's references once the list has taken its own.
fn dict_obj(interp: &Interp, pairs: &[(&[u8], Vec<u8>)]) -> *mut TclObj {
    let mut elems: Vec<*mut TclObj> = Vec::with_capacity(pairs.len() * 2);
    for (k, v) in pairs {
        elems.push(obj::new_string_bytes(k));
        elems.push(obj::new_string_bytes(v));
    }
    let list = interp.new_list_object(&elems);
    for e in elems {
        drop_fresh(e);
    }
    list
}

/// Render an optional limit integer: the decimal bytes, or empty when unset.
fn opt_int(v: Option<i64>) -> Vec<u8> {
    v.map(|n| n.to_string().into_bytes()).unwrap_or_default()
}

/// Resolve an `interp limit` option by unambiguous prefix against `opts`
/// (C's `Tcl_GetIndexFromObj`) — through the one shared owner.
///
/// `OptionTable::abbreviating` is used rather than a hand-rolled
/// `starts_with` filter: a naive prefix filter can only ever report `bad
/// option`, but the empty word is a prefix of *every* option and C reports
/// that case as `ambiguous option ""`, not `bad option ""`. The option
/// table also owns the `", or"` enumeration in the error message, matching
/// `prefix::choice_list_bytes` rather than duplicating it.
fn resolve_limit_opt(arg: &[u8], opts: &[&[u8]]) -> Result<Vec<u8>, Vec<u8>> {
    let table = tcl_cmd_core::prefix::OptionTable::abbreviating("option", opts);
    match table.index_of(arg) {
        Ok(i) => Ok(opts[i].to_vec()),
        Err(m) => Err(m),
    }
}

/// `interp debug`'s one option word (`debugTypes[]`, `tclInterp.c`): C resolves
/// it with `Tcl_GetIndexFromObj(…, "debug option", 0)`, so `-f`/`-fr`
/// abbreviate and a miss is `bad debug option "…": must be -frame` — a
/// one-entry table is never `ambiguous`, not even for the empty word. Shared
/// with the bytecode VM through the one `tcl-cmd-core::prefix` matcher.
const DEBUG_OPTIONS: tcl_cmd_core::prefix::OptionTable<'static, &[u8]> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("debug option", &[b"-frame"]);

/// `interp limit`'s type word (`limitTypes[]`, `tclInterp.c`): the noun is
/// `limit type` and the flags are `0`, so `c`/`t` abbreviate and the empty
/// word — a prefix of both entries — is `ambiguous limit type ""`.
pub(crate) const LIMIT_TYPES: tcl_cmd_core::prefix::OptionTable<'static, &[u8]> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("limit type", &[b"commands", b"time"]);

/// Parse an `interp limit` integer option value (`expected integer but got "X"`).
fn parse_limit_int(bytes: &[u8]) -> Result<i64, Vec<u8>> {
    if let Ok(s) = std::str::from_utf8(bytes) {
        if let Ok(n) = s.trim().parse::<i64>() {
            return Ok(n);
        }
    }
    let mut m = b"expected integer but got \"".to_vec();
    m.extend_from_slice(bytes);
    m.push(b'"');
    Err(m)
}

/// The release a fresh [`Interp`] emulates until an embedder pins another with
/// [`Interp::set_runtime_version`] — the same default as `tcl_vm::Vm` and as
/// `tcl_syntax::number`'s ambient grammar, so an interpreter nobody configures
/// reads numerals exactly as the release it reports.
const DEFAULT_RUNTIME_VERSION: tcl_dialect::TclVersion = tcl_dialect::TclVersion::V9_0;

/// Install `version`'s build-time facts as this thread's ambient ones: the C
/// regex engine's implicit compile flags (see
/// [`crate::regex_capi::set_runtime_release`]), and the numeric-literal
/// grammar, so every numeral this runtime reads (`expr`, `format`, `dict`, `incr`, the
/// bignum tower — all of which parse through `tcl_syntax::number::parse_whole`
/// with `ParseFlags::default()`) follows the emulated release: `0755` is 493
/// under 8.4/8.6 and 755 under 9.0, `0b`/`0o` exist from 8.5, and `0d` plus `_`
/// digit separators from 9.0.
///
/// C settles this at build time (`#define`/`#undef KILL_OCTAL` in
/// `tclStrToD.c`), so it is a property of the runtime rather than of each
/// conversion — hence ambient state rather than an argument threaded through
/// every `Tcl_GetIntFromObj`-shaped call. Called from [`Interp::new`] and
/// [`Interp::set_runtime_version`], i.e. everywhere a release is established.
fn install_ambient_release(version: tcl_dialect::TclVersion) {
    tcl_syntax::number::set_runtime_syntax(version.number_syntax());
    crate::regex_capi::set_runtime_release(version);
    let dialect = tcl_registry::InvocationDialect::for_version(version);
    parse::install_native_list_policy(
        dialect.lexer_grammar.list_parse,
        dialect.lexer_grammar.escapes,
    );
    obj::install_double_string_policy(tcl_dialect::DoubleStringPolicy::for_tcl_version(version));
}

pub(crate) fn default_host() -> Rc<dyn tcl_platform::Host> {
    #[cfg(not(target_arch = "wasm32"))]
    let host = Rc::new(tcl_host_native::NativeHost::new()) as Rc<dyn tcl_platform::Host>;
    #[cfg(all(target_arch = "wasm32", target_os = "wasi"))]
    let host = Rc::new(crate::host_wasm::WasiHost::new()) as Rc<dyn tcl_platform::Host>;
    #[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
    let host = Rc::new(crate::host_wasm::BrowserHost::new()) as Rc<dyn tcl_platform::Host>;
    host
}

impl Interp {
    /// Create an interp: global frame, the built-in command set, an empty
    /// result, and the predefined variables that C installs in
    /// `Tcl_CreateInterp`.
    pub fn new() -> Interp {
        Self::with_host(default_host())
    }

    /// Create an interpreter whose first bootstrap reads from `host`.
    ///
    /// Restricted and synthetic embedders should prefer this constructor so
    /// no process-host values are ever installed, even transiently.
    pub fn with_host(host: Rc<dyn tcl_platform::Host>) -> Interp {
        Self::with_host_bootstrap(host, None)
    }

    /// Construct the selected native core variable inventory before any library
    /// or CLI arguments. Host/build path bytes are supplied independently.
    /// The registered command set remains this backend's supported commands.
    pub fn with_native_core(
        host: Rc<dyn tcl_platform::Host>,
        profile: &'static tcl_dialect::DialectProfile,
        inputs: tcl_registry::special_vars::NativeBootstrapInputs,
    ) -> Option<Interp> {
        let protocol =
            tcl_registry::InvocationDialect::of_profile(profile).native_bootstrap_protocol()?;
        if protocol.names().tcl_version().is_some() {
            tcl_syntax::list::split_native_list_bytes(
                &inputs.package_path,
                protocol.names().string_protocol(),
            )
            .ok()?;
        }
        Some(Self::with_host_bootstrap(
            host,
            Some((profile, protocol, inputs)),
        ))
    }

    fn with_host_bootstrap(
        host: Rc<dyn tcl_platform::Host>,
        native: Option<(
            &'static tcl_dialect::DialectProfile,
            tcl_registry::special_vars::NativeBootstrapProtocol,
            tcl_registry::special_vars::NativeBootstrapInputs,
        )>,
    ) -> Interp {
        let profile = native.as_ref().map_or_else(
            || crate::environment::profile_for_dialect(""),
            |(profile, _, _)| *profile,
        );
        let version = native
            .as_ref()
            .map_or(DEFAULT_RUNTIME_VERSION, |(profile, _, _)| {
                profile.vm_runtime_version
            });
        let result = obj::new_obj();
        let system_encoding = host.system_encoding();
        // SAFETY: `result` is freshly created; the interp takes the owning ref.
        unsafe { obj::incr_ref_count(result) };
        let mut guards = GuardManager::default();
        // Object dispatch still has mutation sites spread through the TclOO
        // engine. Fail closed until those sites have one mutation owner. The
        // interpreter-policy surface is centralised on
        // `invalidate_interpreter_policy` below and can issue live guards.
        guards.poison(GuardDomain::ObjectDispatch);
        let mut interp = Interp(Rc::new(InterpState {
            native_literal_world: Rc::new(RefCell::new(Default::default())),
            native_literal_arrays: RefCell::new(Vec::new()),
            native_execution_constants: RefCell::new(None),
            native_command_interpreter:
                tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
                    owner:
                        tcl_runtime_api::native_compilation::NativeInterpreterIdentity::fresh_owner(
                        ),
                    interpreter: 0,
                },
            native_compiler_pass_owner: std::cell::OnceCell::new(),
            jim_local_depth: Cell::new(0),
            jim_active_command_workers: RefCell::new(Vec::new()),
            frames: RefCell::new(FrameStack::new()),
            namespaces: RefCell::new(Namespaces::new()),
            array_operation_targets: RefCell::new(Vec::new()),
            active_var_trace_scopes: RefCell::new(Vec::new()),
            guards: RefCell::new(OwnedGuardManager::new(guards)),
            guarded_commands: RefCell::new(std::collections::BTreeMap::new()),
            registry_builtin_names: RefCell::new(std::collections::HashMap::new()),
            native_compilation: RefCell::new(native_compilation::CompilationState::default()),
            stock_ensembles: RefCell::new(stock_ensembles::StockEnsembles::default()),
            unsupported_builtins: RefCell::new(std::collections::HashSet::new()),
            current_ns: Cell::new(GLOBAL),
            recursion_depth: Cell::new(0),
            recursion_limit: Cell::new(RECURSION_LIMIT),
            packages: RefCell::new(crate::cmd_package::PackageState::with_core(version)),
            script_stack: RefCell::new(Vec::new()),
            channels: RefCell::new(crate::cmd_chan::ChannelTable::new(version, system_encoding)),
            return_code: Cell::new(Code::Ok),
            return_level: Cell::new(1),
            return_options: RefCell::new(NativeReturnOptions::default()),
            traces: RefCell::new(crate::cmd_trace::TraceTable::default()),
            native_error_cells: RefCell::new(Vec::new()),
            native_precision_cell: Cell::new(None),
            exc: RefCell::new(ExceptionState::default()),
            c_api_errors: Cell::new(0),
            error_line: Cell::new(1),
            children: RefCell::new(std::collections::BTreeMap::new()),
            interp_counter: Cell::new(0),
            hidden: RefCell::new(std::collections::BTreeMap::new()),
            parent: RefCell::new(Weak::new()),
            native_child_interpreter: Cell::new(false),
            native_child_command_generation: Cell::new(None),
            is_safe: Cell::new(false),
            eval_active: Cell::new(0),
            pending_delete: Cell::new(false),
            host: RefCell::new(host),
            oo: Rc::new(RefCell::new(crate::cmd_oo::OoState::default())),
            cmd_frames: RefCell::new(Vec::new()),
            arg_lines: RefCell::new(Vec::new()),
            arg_locs: RefCell::new(Vec::new()),
            eval_depth: Cell::new(0),
            var_trace_epoch: Cell::new(1),
            cmd_count: Cell::new(0),
            native_proc_dispatches: Cell::new(0),
            exit_code: Cell::new(None),
            measure_overhead: Cell::new(0.0),
            bgerror: RefCell::new(None),
            bg_queue: RefCell::new(Vec::new()),
            events: RefCell::new(crate::cmd_event::EventQueue::default()),
            coros: RefCell::new(std::collections::BTreeMap::new()),
            #[cfg(not(target_arch = "wasm32"))]
            coro_probe_error: RefCell::new(None),
            ensemble_rewrite: RefCell::new(None),
            handler_usage_adapters: RefCell::new(Vec::new()),
            #[cfg(have_tommath)]
            rand_seed: Cell::new(None),
            error_stack: RefCell::new(NativeErrorStack::default()),
            jim_error_stack: RefCell::new(JimErrorStack::default()),
            jim_evaluation_frames: RefCell::new(Vec::new()),
            jim_invocation_borrows: RefCell::new(Vec::new()),
            jim_procedure_level: Cell::new(0),
            native_dispatch_depth: Cell::new(0),
            deferred_tailcalls: RefCell::new(Vec::new()),
            during: Cell::new(None),
            result: Cell::new(result),
            cmd_arena: RefCell::new(CmdArena::default()),
            limits: RefCell::new(LimitSet::default()),
            budget: RefCell::new(crate::budget::Budget::default()),
            stores_confined: Cell::new(false),
            #[cfg(have_tommath)]
            limit_tick: Cell::new(0),
            debug_frame: Cell::new(false),
            runtime_version: Cell::new(version),
            jim_object_context: RefCell::new(None),
            jim_teardown_started: Cell::new(false),
            // The "no dialect pinned" ingress: the lenient environment,
            // whose unit profile is the permissive fallback that hides
            // nothing. `set_dialect_profile` replaces all three together.
            dialect_profile: Cell::new(profile),
            observed_names: RefCell::new(None),
            observed_frame_storage: RefCell::new(None),
            logical_name_provider: Cell::new(None),
            logical_expression_parse_provider: Cell::new(None),
            logical_source_word_provider: Cell::new(None),
            logical_eval_object_provider: Cell::new(None),
            profile_registry: Cell::new(
                native
                    .as_ref()
                    .map(|(profile, _, _)| crate::environment::store_for_profile(profile)),
            ),
            dialect_point: Cell::new(Some(crate::environment::surface_point(profile))),
            pin: RefCell::new(tcl_registry::model::PinnedContext::for_profile(profile)),
            registry_object_roots: RefCell::new(std::collections::HashMap::new()),
            retiring_oo_commands: RefCell::new(std::collections::HashSet::new()),
        }));
        interp.frames.borrow_mut().variable_container_model =
            profile.variable_container_model().unwrap_or_default();
        interp.frames.borrow_mut().variable_lookup_policy = profile
            .variable_lookup_policy()
            .unwrap_or(tcl_dialect::VariableLookupPolicy::Tcl);
        interp.frames.borrow_mut().variable_string_protocol =
            interp.native_invocation_dialect().native_string_protocol();
        interp.namespaces.borrow_mut().variable_string_protocol =
            interp.native_invocation_dialect().native_string_protocol();
        interp.namespaces.borrow_mut().variable_container_model =
            profile.variable_container_model().unwrap_or_default();
        interp.namespaces.borrow_mut().variable_lookup_policy = profile
            .variable_lookup_policy()
            .unwrap_or(tcl_dialect::VariableLookupPolicy::Tcl);
        interp.namespaces.borrow_mut().variable_link_binding = profile
            .variable_link_binding()
            .unwrap_or(tcl_dialect::VariableLinkBinding::StableCell);
        interp.namespaces.borrow_mut().variable_name_protocol =
            interp.name_policy_protocol().map(|policy| policy.recipe());
        interp.namespaces.borrow_mut().ns_var_global_fallback =
            version.namespace_var_global_fallback();
        interp.namespaces.borrow_mut().set_native_command_version(
            interp
                .native_invocation_dialect()
                .native_command_name_protocol()
                .map(|protocol| protocol.version()),
        );
        interp.install_variable_table_recipe();
        if native.is_some() {
            let dialect = interp.native_invocation_dialect();
            parse::install_native_list_policy(
                dialect.lexer_grammar.list_parse,
                dialect.lexer_grammar.escapes,
            );
            if let Some(policy) = profile.double_string_policy() {
                obj::install_double_string_policy(policy);
            }
        }
        // The numeric grammar is thread-ambient and may have been left on
        // another release by an interpreter built earlier on this thread, so a
        // fresh interp installs its own rather than inheriting whatever is
        // there (`set_runtime_version` re-installs when an embedder repins).
        install_ambient_release(version);
        interp.error_stack.borrow_mut().configure(
            interp
                .native_invocation_dialect()
                .native_error_objects_protocol(),
        );
        if let Some((_, protocol, _)) = &native {
            builtins::install_native_core(&mut interp, *protocol);
        } else {
            builtins::install(&mut interp);
        }
        interp.install_jim_local_commands();
        interp.seal_native_compiler_tokens();
        // C sets `tcl_version`/`tcl_patchLevel` in `Tcl_CreateInterp`
        // (9.0.4 `generic/tclBasic.c:1346-1347`), **not** in `Tcl_Init` — so
        // they exist in an interpreter that never sources `init.tcl`.
        // The ordinary embedding host also supplies application and library
        // globals. The explicit native constructor consumes only its selected
        // physical root allocation plan.
        if let Some((_, protocol, inputs)) = native {
            interp.bootstrap_native_core(protocol, &inputs);
            if protocol.names().string_protocol()
                == tcl_syntax::native_string::NativeStringProtocol::Jim084
            {
                match interp.native_jim_object_context() {
                    Ok(context) => interp.set_result(context.empty_object().as_ptr()),
                    Err(error) => {
                        interp.report_cmd_error(error.into());
                    }
                }
            } else {
                // Tcl_CreateInterp returns its fresh untyped empty result.
                interp.set_result_bytes(b"");
            }
        } else {
            interp.set_startup_globals();
        }
        interp
    }

    // capability host

    /// The capability host (filesystem/`env`/`clock`/subprocess seam). Returns an
    /// independent `Rc` handle, not a borrow, so a command can hold the host
    /// while still taking `&mut self` for its `ValueOps` (e.g. the `exec`
    /// adapter, which needs both at once).
    #[must_use]
    pub(crate) fn host(&self) -> Rc<dyn tcl_platform::Host> {
        self.0.host.borrow().clone()
    }

    /// Swap the capability host (e.g. a test installing a sandboxed,
    /// no-subprocess host to prove the capability gate, or a safe interp taking
    /// a restricted one). Interior-mutable since the interp is shared via `Rc`.
    pub fn set_host(&self, host: Rc<dyn tcl_platform::Host>) {
        self.invalidate_interpreter_policy();
        let system_encoding = host.system_encoding();
        *self.0.host.borrow_mut() = host;
        self.channels
            .borrow()
            .reset_process_state_if_owner(self.runtime_version(), system_encoding);
        let mut interp = self.clone();
        interp.rebootstrap_host_globals();
        if interp.is_safe.get() {
            interp.scrub_host_globals_for_safe();
        }
    }

    // emulated Tcl release

    /// Pin the Tcl release this interpreter emulates.
    ///
    /// Mirrors [`tcl_vm::Vm::set_runtime_version`]'s contract for the
    /// tree-walking runtime: every release-dependent *semantic* is derived
    /// from this one value rather than being set independently, so the two
    /// engines cannot drift apart by having one of them updated and not the
    /// other. Today that is the numeric-literal grammar (see
    /// [`install_ambient_release`]), the namespace-scope variable fallback
    /// (TIP 278), and the release-reporting globals.
    ///
    /// The fallback is a property of the **namespace table**, which every
    /// interpreter owns privately, so a child (`interp create`) and a safe
    /// interpreter each resolve against their own global namespace — setting
    /// it here never reaches across an interpreter boundary.
    pub fn set_runtime_version(&mut self, version: tcl_dialect::TclVersion) {
        // A bare release pin is the matching plain-Tcl profile: the emulated
        // release is one fact carrying the runtime semantics, the lexing
        // grammar, and the command-surface availability mask. The release
        // name is a dialect *name*, so it resolves through the one ingress
        // seam (`crate::environment`) rather than through `by_name`.
        self.set_dialect_profile(crate::environment::profile_for_dialect(
            version.dialect_name(),
        ));
    }

    /// Install an independently authored script-object evaluation recipe.
    /// The separately selected actual host supplies physical materialisation.
    #[must_use]
    pub fn set_logical_eval_object_provider(
        &mut self,
        provider: tcl_registry::native_eval_object::LogicalEvalObjectProvider,
        actual_host: &'static tcl_dialect::DialectProfile,
    ) -> bool {
        if tcl_registry::InvocationDialect::of_profile(actual_host)
            .execution_point()
            .is_none()
            || self
                .native_invocation_dialect()
                .eval_object_protocol(Some(provider))
                .is_none()
        {
            return false;
        }
        if !self
            .logical_eval_object_provider
            .get()
            .is_some_and(|(installed, host)| {
                installed == provider && host.cache_key() == actual_host.cache_key()
            })
        {
            self.invalidate_interpreter_policy();
            self.logical_eval_object_provider
                .set(Some((provider, actual_host)));
        }
        true
    }

    fn eval_object_protocol(
        &self,
    ) -> Option<tcl_registry::native_eval_object::NativeEvalObjectProtocol> {
        self.native_invocation_dialect()
            .invocation_eval_object_protocol(
                self.logical_eval_object_provider
                    .get()
                    .map(|(provider, _)| provider),
            )
    }

    pub(crate) fn eval_frame_dialect(&self) -> tcl_registry::InvocationDialect {
        let dialect = self.native_invocation_dialect();
        if dialect.native_eval_object_protocol().is_some() {
            return dialect;
        }
        self.logical_eval_object_provider.get().map_or_else(
            || dialect,
            |(_, host)| tcl_registry::InvocationDialect::of_profile(host),
        )
    }

    fn eval_object_bytes(
        &mut self,
        original: *mut TclObj,
    ) -> Result<Vec<u8>, tcl_syntax::value::ValueError> {
        let dialect = self.eval_frame_dialect();
        let protocol = dialect.native_string_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native script-object string materialisation",
            ),
        )?;
        crate::dict::native_object_bytes_with_integer_formatter(
            original,
            protocol,
            self.host().native_integer_formatter(),
        )
    }

    /// Install source-word materialisation independently of physical object
    /// conversion and expression/name providers. The host engine is explicit.
    #[must_use]
    pub fn set_logical_source_word_provider(
        &mut self,
        provider: tcl_registry::invocation_words::LogicalSourceWordProvider,
        actual_host: &'static tcl_dialect::DialectProfile,
    ) -> bool {
        if tcl_registry::InvocationDialect::of_profile(actual_host)
            .execution_point()
            .is_none()
            || self
                .native_invocation_dialect()
                .logical_source_string_protocol(provider, self.dialect_profile())
                .is_none()
        {
            return false;
        }
        let capability = Some((provider, actual_host.cache_key()));
        if self.logical_source_word_provider.get() != capability {
            self.invalidate_interpreter_policy();
            self.logical_source_word_provider.set(capability);
        }
        true
    }

    pub(crate) fn source_string_protocol(
        &self,
    ) -> Option<tcl_syntax::native_string::NativeStringProtocol> {
        let dialect = self.native_invocation_dialect();
        dialect
            .native_source_string_protocol()
            .filter(|protocol| protocol.escape_syntax() == self.lexer_config().escapes)
            .or_else(|| {
                self.logical_source_word_provider
                    .get()
                    .and_then(|(provider, _)| {
                        dialect.logical_source_string_protocol(provider, self.dialect_profile())
                    })
            })
    }

    /// Install the embedding host's authored logical parser capability with
    /// an independently selected actual host engine. Compatibility versions
    /// and command availability cannot supply that host receipt.
    #[must_use]
    pub fn set_logical_expression_parse_provider(
        &mut self,
        provider: tcl_registry::invocation_words::LogicalExpressionParseProvider,
        actual_host: &'static tcl_dialect::DialectProfile,
    ) -> bool {
        let dialect = tcl_registry::InvocationDialect::of_profile(self.dialect_profile());
        if tcl_registry::InvocationDialect::of_profile(actual_host)
            .execution_point()
            .is_none()
            || dialect
                .logical_expression_parse_context(provider, self.dialect_profile())
                .is_none()
        {
            return false;
        }
        let capability = Some((provider, actual_host.cache_key()));
        if self.logical_expression_parse_provider.get() != capability {
            self.invalidate_interpreter_policy();
            self.logical_expression_parse_provider.set(capability);
        }
        true
    }

    pub(crate) fn logical_expression_parse_policy(
        &self,
    ) -> Option<(
        tcl_registry::invocation_words::LogicalExpressionParseProvider,
        tcl_dialect::DialectProfileKey,
    )> {
        self.logical_expression_parse_provider
            .get()
            .filter(|(provider, _)| {
                self.native_invocation_dialect()
                    .logical_expression_parse_context(*provider, self.dialect_profile())
                    .is_some()
            })
    }

    pub(crate) fn expression_evaluation_policy(
        &self,
    ) -> Option<tcl_runtime_api::expression_policy::ExpressionEvaluationPolicy> {
        use tcl_registry::native_expression_program::{
            authored_expression_evaluation_policy, native_expression_evaluation_policy,
        };
        if let Some((provider, _)) = self.logical_expression_parse_policy() {
            return authored_expression_evaluation_policy(self.dialect_profile(), provider, None);
        }
        let dialect = self.native_invocation_dialect();
        let point = dialect.execution_point()?;
        let profile = if tcl_registry::InvocationDialect::of_profile(self.dialect_profile())
            .execution_point()
            == Some(point)
        {
            self.dialect_profile()
        } else {
            tcl_dialect::DialectProfile::find(dialect.tcl_version?.dialect_profile_name())?
        };
        native_expression_evaluation_policy(profile, point)
    }

    #[cfg(have_tommath)]
    pub(crate) fn expression_parse_context(&self) -> tcl_syntax::expr::parser::ExprParseContext {
        let dialect = self.native_invocation_dialect();
        self.logical_expression_parse_policy()
            .and_then(|(provider, _)| {
                dialect.logical_expression_parse_context(provider, self.dialect_profile())
            })
            .unwrap_or_else(|| dialect.expression_parse_context(Some(self.dialect_profile())))
    }

    /// Pin the dialect profile this interpreter emulates — the profile form
    /// of [`Self::set_runtime_version`], for hosts whose dialect is a vendor
    /// profile rather than a plain Tcl release. The runtime version follows
    /// the profile's pinned `vm_runtime_version`, scripts are lexed with the
    /// profile's grammar, and the profile's availability mask becomes the
    /// builtin command-surface filter.
    pub fn set_dialect_profile(&mut self, profile: &'static tcl_dialect::DialectProfile) {
        self.install_pin(tcl_registry::model::PinnedContext::for_profile(profile));
    }

    /// Pin the world this interpreter runs in: the environment, the release
    /// point within it, the build, the package floors and the registry overlay
    /// generation, resolved through the same ingress the compiler uses. The
    /// profile the environment resolves to is what
    /// [`Self::set_dialect_profile`] would pin, and the generation at the
    /// context's overlay is held for as long as the pin stands.
    ///
    /// # Errors
    ///
    /// [`PinError`](tcl_registry::model::PinError) when the ingress does not
    /// agree with the context — no such environment, a release or build that
    /// is not the environment's point, or an overlay nothing has installed,
    /// which is an error and never the un-overlaid generation under another
    /// name. The pin is unchanged.
    pub fn pin_context(
        &mut self,
        context: &tcl_runtime_api::RuntimeContext,
    ) -> Result<(), tcl_registry::model::PinError> {
        self.install_pin(crate::environment::pin_context(context)?);
        Ok(())
    }

    /// The world this interpreter is pinned to.
    #[must_use]
    pub fn runtime_context(&self) -> tcl_runtime_api::RuntimeContext {
        self.0.pin.borrow().context.clone()
    }

    /// The identity this interpreter states of itself — its pinned context,
    /// this build's ABI, intrinsic table and embedded library, and no pack
    /// facts — in the shape a compiled artefact states its own.
    #[must_use]
    pub fn held_identity(&self) -> tcl_runtime_api::ArtefactIdentityManifest {
        self.0.pin.borrow().identity().clone()
    }

    fn install_pin(&mut self, pin: tcl_registry::model::PinnedContext) {
        let profile = pin.profile;
        *self.0.pin.borrow_mut() = pin;
        let version = profile.vm_runtime_version;
        // Ahead of the unchanged-profile short-circuit: the numeric grammar is
        // *thread*-ambient, not per-interp, so "this interp already emulates
        // `version`" does not imply the thread's grammar is this interp's. A
        // second interpreter constructed on a thread where an earlier one
        // installed 8.4 must re-install its own release even when its version
        // field needs no change.
        install_ambient_release(version);
        let dialect = tcl_registry::InvocationDialect::of_profile(profile);
        parse::install_native_list_policy(
            dialect.lexer_grammar.list_parse,
            dialect.lexer_grammar.escapes,
        );
        if let Some(policy) = profile.double_string_policy() {
            obj::install_double_string_policy(policy);
        }
        if std::ptr::eq(self.dialect_profile(), profile) {
            return;
        }
        self.invalidate_interpreter_policy();
        self.0.dialect_profile.set(profile);
        if dialect.return_options_protocol()
            == Some(tcl_cmd_core::return_options::ReturnOptionsProtocol::Jim084)
        {
            self.return_level.set(0);
            self.return_code.set(Code::Ok);
        }
        self.refresh_native_math_function_table();
        self.0
            .profile_registry
            .set((!profile.is_fallback()).then(|| crate::environment::store_for_profile(profile)));
        self.0
            .dialect_point
            .set(Some(crate::environment::surface_point(profile)));
        self.0.runtime_version.set(version);
        self.channels
            .borrow()
            .reset_standard_channels_if_owner(version);
        self.namespaces.borrow_mut().ns_var_global_fallback =
            version.namespace_var_global_fallback();
        let variable_policy = profile
            .variable_lookup_policy()
            .unwrap_or(tcl_dialect::VariableLookupPolicy::Tcl);
        let container_model = profile.variable_container_model().unwrap_or_default();
        let variable_hash_recipe = self
            .selected_variable_table_protocol()
            .map(|protocol| protocol.recipe());
        self.frames.borrow_mut().variable_hash_recipe = variable_hash_recipe;
        self.namespaces.borrow_mut().variable_hash_recipe = variable_hash_recipe;
        self.frames.borrow_mut().variable_container_model = container_model;
        self.frames.borrow_mut().variable_string_protocol =
            self.native_invocation_dialect().native_string_protocol();
        self.namespaces.borrow_mut().variable_container_model = container_model;
        self.namespaces.borrow_mut().variable_string_protocol =
            self.native_invocation_dialect().native_string_protocol();
        self.frames.borrow_mut().variable_lookup_policy = variable_policy;
        self.namespaces.borrow_mut().variable_lookup_policy = variable_policy;
        self.namespaces.borrow_mut().variable_link_binding = profile
            .variable_link_binding()
            .unwrap_or(tcl_dialect::VariableLinkBinding::StableCell);
        self.namespaces.borrow_mut().variable_name_protocol =
            self.name_policy_protocol().map(|policy| policy.recipe());
        self.namespaces.borrow_mut().set_native_command_version(
            self.native_invocation_dialect()
                .native_command_name_protocol()
                .map(|protocol| protocol.version()),
        );
        self.error_stack.borrow_mut().configure(
            self.native_invocation_dialect()
                .native_error_objects_protocol(),
        );
        self.write_release_globals();
        self.install_native_precision_trace();
        // `package provide Tcl` is a release fact, not a runtime constant, and
        // the pre-provided entries were written against the *previous* pin —
        // re-derive them.
        match self.native_invocation_dialect().native_package_protocol() {
            Some(tcl_registry::native_package::NativePackageProtocol::C(release)) => {
                self.packages.borrow_mut().provide_core(release)
            }
            Some(tcl_registry::native_package::NativePackageProtocol::Jim084) | None => {
                self.packages.borrow_mut().clear_core()
            }
        }
        self.refresh_stock_ensembles();
        self.install_jim_local_commands();
        crate::cmd_proc::install_stock_scripted_wrappers(self);
        // Scripted wrapper installation changes the command generation epoch.
        // Re-attest only the actual untouched stock String allocation and map.
        self.attest_stock_string_implementation();
        self.seal_native_compiler_attachments();
        if self.native_invocation_dialect().native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            match self.native_jim_object_context() {
                Ok(context) => self.set_result(context.empty_object().as_ptr()),
                Err(error) => {
                    self.report_cmd_error(error.into());
                }
            }
        }
    }

    /// The Tcl release this interpreter emulates (see
    /// [`Self::set_runtime_version`]).
    #[must_use]
    pub fn runtime_version(&self) -> tcl_dialect::TclVersion {
        self.0.runtime_version.get()
    }

    /// The mutable `encoding system` value shared by this interpreter tree.
    #[must_use]
    pub(crate) fn system_encoding(&self) -> tcl_platform::SystemEncoding {
        self.channels.borrow().system_encoding()
    }

    /// Replace the system encoding used to initialise subsequently opened
    /// channels. Existing channel handles retain their own configuration.
    pub(crate) fn set_system_encoding(&self, encoding: tcl_platform::SystemEncoding) {
        self.channels.borrow().set_system_encoding(encoding);
    }

    /// The dialect profile this interpreter validates its command surface
    /// against (see [`Self::set_dialect_profile`]).
    #[must_use]
    pub fn dialect_profile(&self) -> &'static tcl_dialect::DialectProfile {
        self.0.dialect_profile.get()
    }

    /// Actual Jim interpreter objects; source grammar cannot issue this context.
    pub(crate) fn native_jim_object_context(
        &self,
    ) -> Result<Rc<crate::native_source::NativeJimObjectContext>, tcl_syntax::value::ValueError>
    {
        self.native_jim_object_context_in(self.native_invocation_dialect())
    }

    pub(crate) fn associate_native_jim_arguments(
        &self,
        values: &[*mut TclObj],
    ) -> Result<(), tcl_syntax::value::ValueError> {
        if self.native_invocation_dialect().native_string_protocol()
            != Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return Ok(());
        }
        let context = self.native_jim_object_context()?;
        for &value in values {
            crate::native_source::bind_context(value, &context)?;
        }
        Ok(())
    }

    fn associate_native_jim_variable_value(&self, value: *mut TclObj) -> Result<(), VarError> {
        self.associate_native_jim_arguments(&[value])
            .map_err(|error| {
                self.clone().refuse_native_access(
                    error
                        .native_access_refusal()
                        .expect("Jim context association is a host refusal"),
                );
                VarError::NameProtocolUnavailable
            })
    }

    pub(crate) fn native_jim_object_context_in(
        &self,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<Rc<crate::native_source::NativeJimObjectContext>, tcl_syntax::value::ValueError>
    {
        if dialect.native_string_protocol()
            != Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim original source context",
            ));
        }
        if let Some(context) = self.jim_object_context.borrow().as_ref() {
            if !context.is_live() {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "retired Jim interpreter",
                ));
            }
            context.select_numeric_host(self.host());
            return Ok(Rc::clone(context));
        }
        let context = crate::native_source::NativeJimObjectContext::new(dialect)?;
        context.select_numeric_host(self.host());
        self.namespaces
            .borrow_mut()
            .adopt_jim_root_namespace(context.empty_object().clone());
        let namespace = self
            .namespaces
            .borrow_mut()
            .take_jim_namespace_owner(crate::namespace::GLOBAL)
            .expect("authentic Jim root namespace reference");
        self.frames
            .borrow_mut()
            .retain_jim_root_namespace(namespace);
        *self.jim_object_context.borrow_mut() = Some(Rc::clone(&context));
        Ok(context)
    }

    /// Policies of the actual native interpreter, independently of an
    /// unpinned source-assistance profile. The engine's default C release is known.
    fn install_variable_table_recipe(&self) {
        let recipe = self
            .selected_variable_table_protocol()
            .map(|protocol| protocol.recipe());
        self.frames.borrow_mut().variable_hash_recipe = recipe;
        self.namespaces.borrow_mut().variable_hash_recipe = recipe;
    }

    pub(crate) fn selected_variable_table_protocol(
        &self,
    ) -> Option<tcl_registry::native_variable_table::NativeVariableTableProtocol> {
        let abi = tcl_runtime_api::native_hash_abi::supported_backend_hash_abi(Some(0))?;
        let policy = self.name_policy_protocol()?;
        let dialect = self.native_invocation_dialect();
        match policy.authority() {
            tcl_syntax::naming::NamePolicyAuthority::Native => {
                dialect.native_variable_table_protocol(abi)
            }
            tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation => {
                dialect.authored_variable_table_protocol(policy, abi)
            }
        }
    }

    pub(crate) fn native_invocation_dialect(&self) -> tcl_registry::InvocationDialect {
        let dialect = tcl_registry::InvocationDialect::of_profile(self.dialect_profile());
        if dialect.family() == Some(tcl_dialect::model::Family::Tcl)
            && dialect.tcl_version.is_none()
            && dialect.core_point.is_none()
        {
            tcl_registry::InvocationDialect::for_version(self.runtime_version())
        } else {
            dialect
        }
    }

    /// The lexer configuration scripts evaluate under: the pinned profile's
    /// grammar — `{*}` expansion off and the first-close `${…}` rule on when
    /// the interpreter emulates Tcl 8.4.
    pub(crate) fn lexer_config(&self) -> tcl_lexer::LexerConfig {
        tcl_lexer::LexerConfig::from_grammar(self.dialect_profile().grammar)
    }

    /// Whether a builtin command is exposed on this interpreter's selected
    /// runtime surface. The registry recognises versioned builtin entries;
    /// unrecognised names remain available for user-defined commands.
    ///
    /// Two registry-backed cases: the math-function surface
    /// (`::tcl::mathfunc::*` vs the 8.4 fixed table), and release
    /// availability — a builtin whose registry spec the pinned profile's
    /// availability mask does not admit (`lassign` at 8.4, `lpop` before
    /// 9.0) resolves like C Tcl, to `invalid command name`. Only
    /// registry-known builtins are gated: user procs and names the registry
    /// does not know remain callable, so a polyfill proc shadowing a hidden
    /// builtin keeps working.
    pub(crate) fn builtin_command_visible_for_surface(&self, name: &[u8]) -> bool {
        core::str::from_utf8(name).map_or(true, |name| {
            tcl_registry::expr_surface::RuntimeExprSurface::for_tcl_version(self.runtime_version())
                .permits_builtin_math_function_command(name)
                && self.profile_admits_registry_builtin(name)
        })
    }

    /// Record `owner` as an engine-installed TclOO root object command. The
    /// registry spelling is retained only as the dialect availability key;
    /// command rename changes the display projection, not this identity.
    pub(crate) fn declare_registry_object_root(&self, owner: OoId, registry_name: &[u8]) {
        self.0
            .registry_object_roots
            .borrow_mut()
            .insert(owner, registry_name.to_vec());
    }

    /// Drop an engine-installed root marking with the OO identity it described.
    pub(crate) fn forget_registry_object_root(&self, owner: OoId) {
        self.0.registry_object_roots.borrow_mut().remove(&owner);
    }

    /// Whether `fqn` is an engine-installed TclOO root that this release does
    /// **not** have (e.g. `::oo::configurable` on an 8.6 surface). Such a root
    /// is invisible to every dispatch and enumeration path, so for anything
    /// that asks "is this name taken?" it must read as free — real tclsh 8.6
    /// has no `::oo::configurable`, and a script may define one.
    pub(crate) fn is_gate_hidden_object_root(&self, fqn: &[u8]) -> bool {
        let owner = match self.0.namespaces.borrow().resolve(GLOBAL, fqn) {
            Some(Command::OoObject(owner)) => owner,
            _ => return false,
        };
        self.0
            .registry_object_roots
            .borrow()
            .get(&owner)
            .is_some_and(|registry_name| !self.builtin_command_visible_for_surface(registry_name))
    }

    /// Retire the engine root hidden at `fqn` by the selected dialect before a
    /// script claims that spelling. Its OO lifecycle must finish before the
    /// replacement creates or adopts the same instance namespace.
    pub(crate) fn retire_gate_hidden_object_root(&mut self, fqn: &[u8]) -> bool {
        let owner = match self.0.namespaces.borrow().resolve(GLOBAL, fqn) {
            Some(Command::OoObject(owner)) => owner,
            _ => return false,
        };
        let hidden = self
            .0
            .registry_object_roots
            .borrow()
            .get(&owner)
            .is_some_and(|registry_name| !self.builtin_command_visible_for_surface(registry_name));
        if hidden {
            self.oo_command_renamed(owner, None);
        }
        hidden
    }

    /// Retire a hidden stock OO root only at the exact selected publication slot.
    pub(crate) fn retire_gate_hidden_object_in_slot(&mut self, ns: NsId, tail: &[u8]) {
        let owner = self.namespaces().command_in(ns, tail).and_then(|command| {
            let owner = command.oo_object()?;
            self.0
                .registry_object_roots
                .borrow()
                .get(&owner)
                .is_some_and(|name| !self.builtin_command_visible_for_surface(name))
                .then_some(owner)
        });
        if let Some(owner) = owner {
            self.oo_command_renamed(owner, None);
        }
    }

    /// The availability half of [`Self::builtin_command_visible_for_surface`]:
    /// whether the pinned profile admits registry builtin `name`. Names the
    /// registry does not know are always admitted — they may be engine
    /// extensions the registry has no spec for.
    fn profile_admits_registry_builtin(&self, name: &str) -> bool {
        let Some(registry) = self.0.profile_registry.get() else {
            return true; // the permissive fallback profile gates nothing
        };
        if let Some(admitted) = tcl_registry::CommandRegistry::native_stock_registration_admission(
            tcl_registry::InvocationDialect::of_profile(self.dialect_profile()),
            name,
        ) {
            return admitted;
        }
        registry.get(name).is_none()
            || registry
                .get_for_surface(name, self.0.dialect_point.get())
                .is_some()
    }

    /// Apply the selected dialect's command-surface gate to an already-bound
    /// command token. Engine builtins retain their registry identity by exact
    /// generation across rename and hide; other native commands use their
    /// current Tcl-facing spelling as before.
    fn command_visible_for_surface_at(
        &self,
        command: &Command,
        fqn: &[u8],
        generation: Option<u64>,
    ) -> bool {
        match command {
            Command::Builtin(_) => {
                if generation.is_some_and(|token| self.is_stock_scripted_worker(token)) {
                    return true;
                }
                let registry_name = generation.and_then(|generation| {
                    self.0
                        .registry_builtin_names
                        .borrow()
                        .get(&generation)
                        .cloned()
                });
                self.builtin_command_visible_for_surface(registry_name.as_deref().unwrap_or(fqn))
            }
            Command::OoObject(id) => self
                .0
                .registry_object_roots
                .borrow()
                .get(id)
                .is_none_or(|name| self.builtin_command_visible_for_surface(name)),
            _ => true,
        }
    }

    /// Resolve `name` (from namespace `origin`) to a command handle, applying
    /// the availability gate of [`Self::builtin_command_visible_for_surface`]
    /// to the **final** resolved builtin identity: a builtin the emulated
    /// release does not carry resolves to `None`, exactly as if no command of
    /// that name existed.
    ///
    /// This is the single owner of the gate on the dispatch side. Without it,
    /// a resolve-then-[`Self::invoke`] shape — the alias trampoline
    /// ([`Self::dispatch_alias`]) or the `namespace import` redirect
    /// ([`Command::Imported`]) — could reach the builtin behind an ungated
    /// second resolution, making a release-hidden builtin callable through an
    /// alias or an imported spelling. Every name→`Command` step that feeds
    /// `invoke` goes through here, so a new dispatch path cannot silently
    /// reopen that hole.
    ///
    /// A gated miss is deliberately indistinguishable from a deleted command:
    /// each caller then reports the miss the way it already reports a target
    /// that genuinely does not exist (`invalid command name "<target>"`,
    /// naming the resolved target rather than the alias — matching real tclsh
    /// 8.6/9.0 for `interp alias {} la {} nosuchcmd; la`), which is precisely
    /// the "this release does not have that command" contract this gate
    /// implements.
    pub(crate) fn resolve_dispatchable(&self, origin: NsId, name: &[u8]) -> Option<Command> {
        self.resolve_dispatchable_with_generation(origin, name)
            .map(|(command, _)| command)
    }

    /// Resolve an invocation and retain the selected token generation together.
    /// Native implementation identity must not be reconstructed from display argv.
    pub(crate) fn resolve_dispatchable_with_generation(
        &self,
        origin: NsId,
        name: &[u8],
    ) -> Option<(Command, Option<u64>)> {
        let (command, fqn, generation) = {
            let ns = self.namespaces.borrow();
            (
                ns.resolve(origin, name)?,
                ns.resolve_fqn(origin, name)?,
                ns.resolve_generation(origin, name),
            )
        };
        self.command_visible_for_surface_at(&command, &fqn, generation)
            .then_some((command, generation))
    }

    /// Convert `obj` to the byte view consumed by Tcl's `binary` command.
    ///
    /// Byte-array objects retain their own raw payload. Ordinary string objects
    /// are converted through the release profile instead: Tcl 8 truncates a
    /// wide code point to one byte, while Tcl 9 rejects it. Keeping this at the
    /// interpreter boundary makes every `binary` subcommand use the same
    /// dual-representation and version rule.
    pub(crate) fn binary_bytes(&mut self, value: *mut TclObj) -> Result<Vec<u8>, Code> {
        let Some(policy) = self.native_invocation_dialect().binary_data_conversion() else {
            return Err(self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "binary conversion",
                ),
            ));
        };
        self.native_binary_bytes_with(value, policy, true)
    }

    pub(crate) fn native_binary_bytes_with(
        &mut self,
        value: *mut TclObj,
        policy: tcl_registry::native_binary_value::NativeBinaryByteConversion,
        cache: bool,
    ) -> Result<Vec<u8>, Code> {
        if policy == tcl_registry::native_binary_value::NativeBinaryByteConversion::Utf8 {
            return Ok(obj::bytes_of(value));
        }
        let recipe = self.byte_array_string_recipe()?;
        crate::bytearray::native_binary_bytes(value, policy, cache, recipe)
            .map_err(|error| self.error_with_code(error.message().as_bytes(), b"TCL VALUE BYTES"))
    }

    /// Invoke a Tcl 8.4 fixed-table `expr` math function selected by the
    /// registry. This bypasses the command table deliberately: TIP 232 had not
    /// introduced `::tcl::mathfunc::*` command wrappers yet, so a similarly
    /// named proc or alias cannot replace the C function-table entry.
    #[cfg(have_tommath)]
    pub(crate) fn eval_fixed_math_call(
        &mut self,
        spec: &'static tcl_registry::CommandSpec,
        args: &[*mut TclObj],
    ) -> Code {
        let name = new_string(spec.name.as_bytes());
        let mut argv = Vec::with_capacity(args.len() + 1);
        // SAFETY: the fresh name and each live argument gain the ownership the
        // temporary argv carries until `release_all` below.
        unsafe { obj::incr_ref_count(name) };
        argv.push(name);
        for &arg in args {
            unsafe { obj::incr_ref_count(arg) };
            argv.push(arg);
        }
        let code = crate::cmd_mathfunc::mathfunc_identity(self, &argv, spec.name.as_bytes());
        release_all(&argv);
        code
    }

    /// Write the release-reporting globals (`tcl_version` / `tcl_patchLevel`)
    /// for the currently-emulated release.  Called at interpreter creation
    /// (C does this in `Tcl_CreateInterp`) and again whenever
    /// [`Self::set_runtime_version`] changes the answer.
    pub(crate) fn write_release_globals(&mut self) {
        let version = self.runtime_version();
        // These are owned bootstrap slots, not guest name-lookup operations.
        // Installing an unsupported native profile must not manufacture an
        // unrelated variable-access refusal before the requested operation.
        for (name, val) in [
            (&b"tcl_version"[..], version.version_string()),
            (b"tcl_patchLevel", version.patchlevel()),
        ] {
            let value = obj::Owned::fresh(new_string(val.as_bytes()));
            let _ = self
                .namespaces_mut()
                .var_table_mut(GLOBAL)
                .store_scalar(name, value.as_ptr());
        }
    }

    // command registry

    /// Register a built-in command (a possibly-qualified `name`, creating
    /// intermediate namespaces; overwrites any existing command of `name`).
    pub fn register_builtin(&mut self, name: &[u8], f: BuiltinFn) {
        self.bind_builtin(name, f);
    }

    /// Bind a builtin through its actual publication owner.
    fn bind_builtin(&mut self, name: &[u8], f: BuiltinFn) -> u64 {
        let Some((ns, tail)) = self.namespaces_mut().command_publication_at(GLOBAL, name) else {
            let _ = self.refuse_host_command("builtin publication has no actual namespace");
            return 0;
        };
        let report = self.namespaces().jim_command_table_key_at(
            GLOBAL,
            name,
            tcl_syntax::naming::NativeNamePurpose::AliasPublication,
        );
        self.bind_command_replacement_with_jim_key(ns, &tail, Command::Builtin(f), report);
        let (fqn, generation) = {
            let namespaces = self.namespaces.borrow();
            (
                namespaces.command_fqn_at(ns, &tail),
                namespaces
                    .command_generation(ns, &tail)
                    .expect("the builtin command was just bound"),
            )
        };
        self.0
            .registry_builtin_names
            .borrow_mut()
            .insert(generation, fqn.clone());
        self.record_scripted_helper_registration(&fqn, generation);
        generation
    }

    /// Bind a builtin and attest `identities` for the token it is bound under.
    /// The token it displaces takes its attestation with it.
    fn bind_attested_builtin(
        &mut self,
        name: &[u8],
        f: BuiltinFn,
        identities: std::collections::BTreeSet<GuardIdentity>,
    ) {
        let displaced = self.namespaces.borrow().resolve_generation(GLOBAL, name);
        let generation = self.bind_builtin(name, f);
        if generation != 0 {
            self.attest(generation, displaced, identities);
        }
    }

    /// The one writer of the attestation table: attest `identities` for the
    /// command token `generation` and drop the entry of the token it displaced.
    /// What is already attested for `generation` stays.
    fn attest(
        &self,
        generation: u64,
        displaced: Option<u64>,
        identities: std::collections::BTreeSet<GuardIdentity>,
    ) {
        let mut attested = self.guarded_commands.borrow_mut();
        if let Some(displaced) = displaced {
            attested.remove(&displaced);
        }
        if !identities.is_empty() {
            attested.entry(generation).or_default().extend(identities);
        }
    }

    /// Register a builtin with a stable semantic identity understood by
    /// offline-generated code. Ordinary registration never infers identity
    /// from a spelling or function address.
    pub fn register_guarded_builtin(&mut self, name: &[u8], f: BuiltinFn, identity: GuardIdentity) {
        self.register_builtin(name, f);
        self.retire_guarded_command_identities();
        if let Some(generation) = self.namespaces.borrow().resolve_generation(GLOBAL, name) {
            self.guarded_commands
                .borrow_mut()
                .entry(generation)
                .or_default()
                .insert(identity);
        }
    }

    /// Register a builtin and derive every semantic identity from its registry
    /// command, subcommand, and invocation-form descriptors.
    pub fn register_spec_builtin(&mut self, spec: &tcl_registry::CommandSpec, f: BuiltinFn) {
        self.register_builtin(spec.name.as_bytes(), f);
        self.attest_spec_implementation(spec);
    }

    pub(crate) fn attest_spec_implementation(&self, spec: &tcl_registry::CommandSpec) {
        self.retire_guarded_command_identities();
        let identities: std::collections::BTreeSet<_> = spec
            .intrinsic_ids()
            .into_iter()
            .flat_map(|id| {
                id.guard_semantics_variants().iter().map(move |semantics| {
                    GuardIdentity::registry_intrinsic_with_semantics(id.stable_id(), *semantics)
                })
            })
            .collect();
        if !identities.is_empty() {
            if let Some(generation) = self
                .namespaces
                .borrow()
                .resolve_generation(GLOBAL, spec.name.as_bytes())
            {
                self.guarded_commands
                    .borrow_mut()
                    .insert(generation, identities);
            }
        }
    }

    /// Register a builtin that only refuses: a call answers "not supported"
    /// where an unregistered command would answer `invalid command name`. The
    /// backing report says so ([`RegisteredBacking::Unsupported`]) rather than
    /// counting it as a handler.
    pub(crate) fn register_unsupported(&mut self, name: &[u8], f: BuiltinFn) {
        let generation = self.bind_builtin(name, f);
        self.0.unsupported_builtins.borrow_mut().insert(generation);
    }

    /// Attach the registry's intrinsic identities to the builtins this
    /// interpreter registered, from the generation it is pinned to.
    ///
    /// The sweep reads that generation's shipped store and nothing an overlay
    /// installed: a pack's command is not a runtime implementation, and only
    /// the runtime may attest. A name is attested only while the command bound
    /// at it is still the builtin registered there, so a name a script has
    /// since redefined or renamed over gains no attestation for a procedure,
    /// and a builtin moved to another name keeps the one it already has (the
    /// table is keyed by the command token's generation, which a rename
    /// carries). It runs once, at the end of registration: the table survives
    /// the profile pin, and a later sweep would attest whatever an embedder
    /// registered at a registry name as the registry's command.
    pub(crate) fn attach_identities(&mut self) {
        let registry = crate::environment::store_for_profile(self.dialect_profile());
        self.attach_identities_from(registry);
    }

    /// The sweep over one generation's store. Only [`Self::attach_identities`]
    /// chooses the store in production: the generation the interpreter is
    /// pinned to.
    fn attach_identities_from(&mut self, registry: &tcl_registry::CommandRegistry) {
        for name in registry.command_names() {
            let Some(spec) = registry.get_exact(name) else {
                continue;
            };
            let identities: std::collections::BTreeSet<GuardIdentity> = spec
                .intrinsic_ids()
                .into_iter()
                .flat_map(|id| {
                    id.guard_semantics_variants().iter().map(move |semantics| {
                        GuardIdentity::registry_intrinsic_with_semantics(id.stable_id(), *semantics)
                    })
                })
                .collect();
            if identities.is_empty() {
                continue;
            }
            let Some(generation) = self.shipped_builtin_generation(spec.name.as_bytes()) else {
                continue;
            };
            self.attest(generation, None, identities);
        }
    }

    /// The token generation of the builtin this interpreter registered at
    /// `name`, when that is still the command `name` resolves to from the
    /// global namespace.
    fn shipped_builtin_generation(&self, name: &[u8]) -> Option<u64> {
        let namespaces = self.namespaces.borrow();
        let generation = namespaces.resolve_generation(GLOBAL, name)?;
        let fqn = namespaces.resolve_fqn(GLOBAL, name)?;
        (self.0.registry_builtin_names.borrow().get(&generation) == Some(&fqn))
            .then_some(generation)
    }

    /// What this interpreter's handler table backs, by command name: the
    /// runtime's own answer to "what did you register", which a spec's
    /// `runtime_backing` declaration is held to.
    ///
    /// Every command bound in any namespace is classified by what it is — a
    /// native handler, an engine-installed `TclOO` root, or a handler that only
    /// refuses. Two kinds of name are reported that no table entry holds: the
    /// commands the object system binds in every object's namespace, which
    /// exist once the roots do; and, in a build that embeds the Tcl library,
    /// the commands that library defines, which exist once `init_library` has
    /// sourced it. A build without the numeric tower reports the commands a
    /// build with it registers as needing the tower. Names are without a
    /// leading `::`, and a name this runtime does not mention is absent.
    #[must_use]
    pub fn backing_report(&self) -> Vec<(String, RegisteredBacking)> {
        let mut report = std::collections::BTreeMap::new();
        {
            let namespaces = self.namespaces.borrow();
            let unsupported = self.0.unsupported_builtins.borrow();
            let roots = self.0.registry_object_roots.borrow();
            for ns in namespaces.descendant_ids(GLOBAL) {
                for name in namespaces.command_names(ns) {
                    let backing = match namespaces.command_in(ns, name) {
                        Some(Command::Builtin(_)) => {
                            let refuses = namespaces
                                .command_generation(ns, name)
                                .is_some_and(|generation| unsupported.contains(&generation));
                            if refuses {
                                RegisteredBacking::Unsupported
                            } else {
                                RegisteredBacking::Builtin
                            }
                        }
                        Some(Command::OoObject(id)) if roots.contains_key(&id) => {
                            RegisteredBacking::Object
                        }
                        _ => continue,
                    };
                    let fqn = namespaces.command_fqn_at(ns, name);
                    report.insert(
                        String::from_utf8_lossy(&fqn)
                            .trim_start_matches("::")
                            .to_owned(),
                        backing,
                    );
                }
            }
            if !roots.is_empty() {
                for name in crate::cmd_oo::object_namespace_command_names() {
                    report.insert(name.to_owned(), RegisteredBacking::Object);
                }
            }
        }
        #[cfg(feature = "wasm_stdlib")]
        for (name, file) in crate::embedded_stdlib::DEFINED_COMMANDS {
            report
                .entry((*name).to_owned())
                .or_insert(RegisteredBacking::Stdlib { file });
        }
        #[cfg(not(have_tommath))]
        for name in builtins::tower_command_names() {
            report
                .entry(name)
                .or_insert(RegisteredBacking::NeedsNumericTower);
        }
        report.into_iter().collect()
    }

    /// The identities attested for the command `name` resolves to from the
    /// current namespace. The name is resolved afresh, so a command that was
    /// replaced, renamed away, hidden, or is no longer admitted by the pinned
    /// surface answers `None`, and one restored by `rename` or `expose` answers
    /// what it did before.
    fn attested_identities(
        &self,
        name: &[u8],
    ) -> Option<std::collections::BTreeSet<GuardIdentity>> {
        let current = self.current_ns.get();
        let (fqn, generation, command) = {
            let namespaces = self.namespaces.borrow();
            (
                namespaces.resolve_fqn(current, name)?,
                namespaces.resolve_generation(current, name)?,
                namespaces.resolve(current, name)?,
            )
        };
        if !self.command_visible_for_surface_at(&command, &fqn, Some(generation)) {
            return None;
        }
        self.guarded_commands.borrow().get(&generation).cloned()
    }

    /// Verify live command identity and issue a guard over `domains`.
    ///
    /// A request for a registry intrinsic must cover the domains its family
    /// requires ([`tcl_registry::IntrinsicId::family`]) whatever the caller
    /// asked for: a Family-B member reaches the variable store, so its guard is
    /// refused while a variable trace exists and stales when one is added.
    pub fn prepare_command_guard(
        &self,
        name: &[u8],
        expected: GuardIdentity,
        domains: GuardDomains,
    ) -> Result<GuardToken, GuardError> {
        if !domains.covers(tcl_registry::IntrinsicId::required_guard_domains(expected)) {
            return Err(GuardError::DomainsInsufficient);
        }
        let traces = self.traces.borrow();
        if (domains.contains(GuardDomain::CommandTrace) && !traces.cmd_traces.is_empty())
            || (domains.contains(GuardDomain::VariableTrace) && !traces.traces.is_empty())
        {
            return Err(GuardError::PrerequisiteUnsatisfied);
        }
        drop(traces);
        let generation = self
            .namespaces
            .borrow()
            .resolve_generation(self.current_ns.get(), name)
            .ok_or(GuardError::IdentityUnavailable)?;
        let observed = self
            .guarded_commands
            .borrow()
            .get(&generation)
            .and_then(|identities| {
                Some(if identities.contains(&expected) {
                    expected
                } else {
                    *identities.first()?
                })
            });
        self.guards
            .borrow_mut()
            .prepare(expected, observed, domains, generation)
    }

    /// Re-check a guard against the current resolved implementation identity.
    #[must_use]
    pub fn check_command_guard(&self, token: GuardToken, name: &[u8]) -> bool {
        let Some(generation) = self
            .namespaces
            .borrow()
            .resolve_generation(self.current_ns.get(), name)
        else {
            return false;
        };
        let identities = self.guarded_commands.borrow();
        let Some(identities) = identities.get(&generation) else {
            return false;
        };
        identities.iter().any(|identity| {
            self.guards
                .borrow()
                .check(token, Some(*identity), &generation)
        })
    }

    /// Re-check a guard for one exact registry intrinsic identity.
    ///
    /// This is the current-interpreter boundary used by generated code: it
    /// refuses a token minted for another intrinsic form even when both forms
    /// share one command head.
    #[must_use]
    pub fn check_command_guard_identity(
        &self,
        token: GuardToken,
        name: &[u8],
        expected: GuardIdentity,
    ) -> bool {
        let Some(generation) = self
            .namespaces
            .borrow()
            .resolve_generation(self.current_ns.get(), name)
        else {
            return false;
        };
        let identities = self.guarded_commands.borrow();
        if !identities
            .get(&generation)
            .is_some_and(|identities| identities.contains(&expected))
        {
            return false;
        }
        self.guards
            .borrow()
            .check_expected(token, expected, Some(expected), &generation)
    }

    /// Execute one registry intrinsic over arguments after command and
    /// subcommand dispatch. `None` declines to the caller's slow path.
    pub fn execute_intrinsic(
        &mut self,
        intrinsic: tcl_registry::IntrinsicId,
        args: &[*mut TclObj],
    ) -> Option<Code> {
        match (intrinsic, args) {
            (tcl_registry::IntrinsicId::StringLength, [value]) => {
                Some(match tcl_cmd_core::string::length(self, value) {
                    Ok(result) => {
                        self.set_result(result);
                        Code::Ok
                    }
                    Err(error) => self.report_cmd_error(error),
                })
            }
            _ => None,
        }
    }

    /// Release one runtime guard token exactly once.
    #[must_use]
    pub fn release_command_guard(&self, token: GuardToken) -> bool {
        self.guards.borrow_mut().release(token)
    }

    pub(crate) fn invalidate_guard_domain(&self, domain: GuardDomain) {
        if domain == GuardDomain::VariableTrace {
            // Every change to the variable-trace set already funnels through
            // here, so this is the one place the per-cell trace bit's epoch
            // needs to move.
            self.var_trace_epoch
                .set(self.var_trace_epoch.get().wrapping_add(1).max(1));
        }
        self.guards.borrow_mut().invalidate(domain);
    }

    /// Invalidate speculative assumptions about this interpreter's visibility,
    /// safety, topology/lifecycle, capability, and execution policy.
    ///
    /// Keep every write to the corresponding private [`InterpState`] fields
    /// behind a method that calls this owner. The epoch is deliberately broader
    /// than any one intrinsic needs: registry dispatch dependencies describe
    /// interpreter policy as an irreducible live-runtime fact.
    fn invalidate_interpreter_policy(&self) {
        self.invalidate_guard_domain(GuardDomain::Interpreter);
    }

    fn retire_guarded_command_identities(&self) {
        self.guarded_commands.borrow_mut().retain(|generation, _| {
            self.raw_command_location_by_generation(*generation)
                .is_some()
        });
    }

    /// Invalidate the guard domains that depend on the command lookup
    /// environment: namespace structure and lifecycle, and interpreter topology.
    ///
    /// A command-table mutation does not come here. What a guard needs of its
    /// command is decided when it is checked, by resolving the guarded name to
    /// a token generation and finding an attestation there
    /// ([`Self::attested_identities`]), so replacing, deleting, renaming,
    /// hiding, or aliasing one command invalidates that command's guards and no
    /// other's.
    fn invalidate_command_environment(&self) {
        let mut guards = self.guards.borrow_mut();
        guards.invalidate(GuardDomain::CommandEnvironment);
        guards.invalidate(GuardDomain::Namespace);
        guards.invalidate(GuardDomain::UnknownHandling);
    }

    /// Command names in the current namespace, filtered through the selected
    /// runtime surface (`info commands`).
    #[must_use]
    pub fn command_names(&self) -> Vec<Vec<u8>> {
        self.visible_command_names_in(self.current_ns.get())
    }

    /// `rename old new` (or `rename old ""` to delete), relative to the current
    /// namespace. Drives the one command table: the guards C's
    /// `TclRenameCommand` applies before anything observable happens, then
    /// [`Self::move_bound_command`] or [`Self::delete_bound_command`].
    pub(crate) fn rename_command(&mut self, old: &[u8], new: &[u8]) -> RenameOutcome {
        // Inside an open `rename` window the vacating name still resolves, but
        // it *is* the destination command — C's two hash entries reference the
        // one `Command` — so a callback's `rename <old> <third>` or `rename
        // <old> {}` moves or destroys that command rather than a second copy
        // standing at the vacating name. Resolved up front, because every guard
        // below and both halves address the binding through this name.
        let through_window = self
            .resolve_cmd_fqn(old)
            .and_then(|fqn| self.renamed_cmd_key(&fqn));
        let old: &[u8] = through_window.as_deref().unwrap_or(old);
        // C's `TclPreventAliasLoop` guards `rename` too, and refuses before
        // anything observable happens — no rename trace fires for a rename that
        // does not take place.
        if self
            .namespaces
            .borrow_mut()
            .rename_creates_alias_loop(self.current_ns.get(), old, new)
        {
            return RenameOutcome::AliasLoop;
        }
        // C's `TclRenameCommand` checks the destination's hash table before
        // touching `old`'s (tclBasic.c), so an occupied destination — self-
        // rename onto the same slot included — is refused before anything
        // observable happens, same as the alias-loop guard above. A
        // release-gated TclOO root this build hides reads as free here too
        // (`is_gate_hidden_object_root`), same as every other "is this name
        // taken?" check.
        let occupant_fqn = self
            .namespaces
            .borrow_mut()
            .destination_occupant_fqn(self.current_ns.get(), new);
        if let Some(occupant_fqn) = occupant_fqn {
            if self.is_gate_hidden_object_root(&occupant_fqn) {
                self.retire_gate_hidden_object_root(&occupant_fqn);
            } else {
                return RenameOutcome::TargetExists;
            }
        }
        // A builtin the emulated release does not carry is not there to be
        // renamed or deleted: rebinding it under a name the registry has no
        // spec for would hand it back ungated, defeating the availability
        // mask outright. Checked before any observable effect, like the
        // alias-loop refusal above.
        let bound = self
            .namespaces
            .borrow()
            .resolve(self.current_ns.get(), old)
            .is_some();
        if bound
            && self
                .resolve_dispatchable(self.current_ns.get(), old)
                .is_none()
        {
            return RenameOutcome::NoSuchCommand;
        }
        if new.is_empty() {
            self.delete_bound_command(old)
        } else {
            self.move_bound_command(old, new)
        }
    }

    /// The `rename old new` half, after [`Self::rename_command`]'s guards.
    ///
    /// C's `TclRenameCommand` (`tclBasic.c` 9.0.4) creates the destination hash
    /// entry, fires the `rename` traces, and only *then* deletes the source
    /// one; both entries reference the one `Command`, and the traces hang off
    /// that rather than off either entry. So for the callbacks' duration the
    /// vacating name **is** the destination command: both names resolve and are
    /// callable, `trace info` answers the same list through either, a `trace
    /// add`/`remove` through either edits it, and a `rename` or delete through
    /// either moves or destroys that one command. Everything the command
    /// carries therefore moves to the destination *before* the callbacks run,
    /// and the window
    /// ([`rename_windows`](crate::cmd_trace::TraceTable::rename_windows))
    /// records the equivalence our name-keyed registries cannot express.
    fn move_bound_command(&mut self, old: &[u8], new: &[u8]) -> RenameOutcome {
        let Some(old_fqn) = self.resolve_cmd_fqn(old) else {
            return RenameOutcome::NoSuchCommand;
        };
        let oo_object = match self.namespaces.borrow().resolve(self.current_ns.get(), old) {
            Some(Command::OoObject(id)) => Some(id),
            _ => None,
        };
        let Some(publication) = self.namespaces.borrow_mut().publish_rename_destination(
            self.current_ns.get(),
            old,
            new,
        ) else {
            return RenameOutcome::NoSuchCommand;
        };
        self.retire_pending_native_ensemble_roles();
        let new_fqn = publication.destination_fqn.clone();
        // The trace list (and any OO object) follows to the new name, and so
        // does every `namespace import` redirect of the old name — C's imports
        // hold the source's command token, so they survive a source rename
        // (tclsh-pinned; see `Namespaces::retarget_imports`). All of it happens
        // here, before the callbacks, because C had moved its one `Command`
        // before it fired them.
        self.move_cmd_traces(&old_fqn, &new_fqn, publication.generation);
        self.retarget_import_sources(publication.generation, &new_fqn);
        crate::cmd_coro::on_command_renamed(self, publication.generation, &new_fqn);
        if let Some(object) = oo_object {
            self.oo_command_renamed(object, Some(&new_fqn));
        }
        self.traces
            .borrow_mut()
            .rename_windows
            .push((old_fqn.clone(), new_fqn.clone()));
        self.fire_cmd_trace_of_token(
            &new_fqn,
            &old_fqn,
            &new_fqn,
            crate::cmd_trace::ops::RENAME,
            Some(publication.generation),
        );
        self.traces.borrow_mut().rename_windows.pop();
        // C's `Tcl_DeleteHashEntry(oldHPtr)` at the tail of `TclRenameCommand`:
        // a plain table removal, firing no `delete` trace, after which the
        // command stands under its new name alone. Whatever occupies the slot
        // goes — C's captured entry pointer does not care either, and a
        // callback that rebound the vacating name left C dereferencing freed
        // memory, so there is no behaviour there to match.
        self.namespaces
            .borrow_mut()
            .retire_rename_source(&publication);
        RenameOutcome::Renamed
    }

    /// The `rename old ""` half, after [`Self::rename_command`]'s guards — C's
    /// `Tcl_DeleteCommandFromToken`, whose `delete` traces fire while the
    /// command is still bound under its name.
    fn delete_bound_command(&mut self, old: &[u8]) -> RenameOutcome {
        let (ensemble_token, import_token) =
            match self.namespaces.borrow().resolve(self.current_ns.get(), old) {
                Some(Command::Ensemble(token)) => (Some(token), None),
                Some(Command::Imported { identity, .. }) => (None, Some(identity)),
                _ => (None, None),
            };
        // Command traces fire *before* the table mutation (the command still
        // exists under its name during the callback), with the fully-qualified
        // old name and an empty new one. C deletes the command *token* it
        // captured here, not whatever the name holds when the callback returns
        // — so the binding is captured alongside the name.
        let bound_before = self.namespaces.borrow().resolve(self.current_ns.get(), old);
        let oo_object = match &bound_before {
            Some(Command::OoObject(id)) => Some(*id),
            _ => None,
        };
        // The token whose trace list this deletion frees, captured before the
        // callbacks can bind a replacement at the same name.
        let dying_token = self.resolve_cmd_token(old);
        let old_fqn = self.resolve_cmd_fqn(old);
        if let Some(of) = &old_fqn {
            if !self.traces.borrow().cmd_traces.is_empty() {
                self.fire_cmd_trace_of_token(
                    of,
                    of,
                    b"",
                    crate::cmd_trace::ops::DELETE,
                    dying_token,
                );
            }
        }
        let existed = old_fqn.is_some();
        // Deleting a suspended coroutine's command tears down its worker first.
        if let Some(generation) = dying_token {
            crate::cmd_coro::on_command_deleted(self, generation);
        }
        // An imported command carries a stable identity. Its delete trace may
        // force-reimport or otherwise replace the same binding; delete the
        // captured old identity wherever it moved, never the callback's fresh
        // command at the old name.
        let removed_import_fqn = import_token
            .as_ref()
            .and_then(|identity| self.remove_import_identity(identity));
        // A delete-trace callback that re-creates the command (`proc foo {} …`)
        // has bound a *new* command at the old name. C's captured token is
        // `CMD_DYING` and no longer owns the hash entry, so its deletion leaves
        // the fresh command standing (`Tcl_DeleteCommandFromToken`,
        // tclBasic.c) — `foo` still exists, and calls the new body. Deleting
        // "whatever is at the name now" would remove the callback's work
        // instead. This is the command half of the rule the import branch just
        // above already applies to its own identity.
        let recreated = match (
            &bound_before,
            self.namespaces.borrow().resolve(self.current_ns.get(), old),
        ) {
            (Some(before), Some(now)) => !before.is_same_binding(&now),
            _ => false,
        };
        let raw = if recreated {
            RenameOutcome::Deleted
        } else if import_token.is_some() {
            if removed_import_fqn.is_some() {
                RenameOutcome::Deleted
            } else {
                RenameOutcome::NoSuchCommand
            }
        } else if self
            .namespaces
            .borrow_mut()
            .delete(self.current_ns.get(), old)
        {
            RenameOutcome::Deleted
        } else {
            RenameOutcome::NoSuchCommand
        };
        // A delete-trace callback may itself delete the command (e.g. by
        // deleting the object's namespace). C captured the command token before
        // the callback, so the deletion still succeeds — treat "existed at
        // entry, gone now" as a normal delete (cleanup is idempotent) rather
        // than reporting "command doesn't exist".
        let outcome = if existed && matches!(raw, RenameOutcome::NoSuchCommand) {
            RenameOutcome::Deleted
        } else {
            raw
        };
        // The command is gone; the dying token's traces and OO registry entry
        // go with it. A replacement the delete callback bound at the same name
        // keeps its own traces (C frees only `cmdPtr->tracePtr`).
        if let (Some(of), RenameOutcome::Deleted) = (old_fqn, outcome) {
            self.remove_cmd_traces_of_token(&of, dying_token);
            let tokens: Vec<_> = ensemble_token.into_iter().collect();
            let mut origins: Vec<u64> = dying_token.into_iter().collect();
            if let Some((removed_fqn, removed_generation)) = removed_import_fqn {
                if removed_fqn != of {
                    self.remove_cmd_traces_of_token(&removed_fqn, Some(removed_generation));
                }
                origins.push(removed_generation);
            }
            self.remove_imports_for_deleted_origins(origins, &tokens);
            if let Some(object) = oo_object {
                self.oo_command_renamed(object, None);
            }
        }
        self.retire_pending_native_ensemble_roles();
        outcome
    }

    /// Install an `interp alias` redirect named `name` → `target ?prefix...?`.
    ///
    /// A definition that would close an alias loop is refused the way C's
    /// `TclPreventAliasLoop` refuses it: bind first, walk the chain, and unbind
    /// again on a hit — which is why a refused definition also destroys the
    /// command it displaced (`proc x …; interp alias {} x {} x` leaves no `x`
    /// at all, tclsh 8.6/9.0-pinned). `Err` carries the alias's simple command
    /// name for the caller's error message.
    pub(crate) fn install_alias(
        &mut self,
        name: &[u8],
        target: Vec<u8>,
        prefix: Vec<Vec<u8>>,
    ) -> Result<(), Vec<u8>> {
        self.install_alias_with_original(name, target, prefix, None)
    }

    /// Install through the same publication owner. A genuine Jim prefix is
    /// authoritative; byte fields are used only by aliases without that owner.
    pub(crate) fn install_alias_with_original(
        &mut self,
        name: &[u8],
        target: Vec<u8>,
        prefix: Vec<Vec<u8>>,
        jim_prefix: Option<obj::Owned>,
    ) -> Result<(), Vec<u8>> {
        let publication_name = self
            .name_policy_protocol()
            .ok_or_else(|| b"alias publication protocol unavailable".to_vec())?
            .recipe()
            .alias_publication_input(tcl_syntax::naming::NativeNameContext::root(), name)
            .map_err(|_| b"alias publication input unavailable".to_vec())?
            .alias_registration_report_bytes()
            .ok_or_else(|| b"alias report unavailable".to_vec())?
            .to_vec();
        let current = self.current_ns.get();
        let publication = self.namespaces_mut().alias_publication_at(current, name);
        let (ns, simple) =
            publication.ok_or_else(|| b"alias publication namespace unavailable".to_vec())?;
        let report = self.namespaces().jim_command_table_key_at(
            current,
            name,
            tcl_syntax::naming::NativeNamePurpose::AliasPublication,
        );
        self.bind_command_replacement_with_jim_key(
            ns,
            &simple,
            Command::Alias {
                target,
                prefix,
                publication_name,
                jim_prefix,
                identity: Rc::new(()),
            },
            report,
        );
        // Jim's native alias command admits cycles; its scripted namespace
        // import helper performs its own source-chain preflight before binding.
        if self
            .name_policy_protocol()
            .is_some_and(|policy| policy.recipe().is_jim084())
        {
            return Ok(());
        }
        let mut namespaces = self.namespaces.borrow_mut();
        if namespaces.alias_chain_loops(ns, &simple) {
            namespaces.unbind_in(ns, &simple);
            return Err(simple);
        }
        Ok(())
    }

    /// Install a cross-interp alias in child `child`: `name` (in the child)
    /// delegates to `target ?prefix...?` run in this (the parent) interp.
    /// Returns whether the child exists.
    pub(crate) fn install_parent_alias(
        &mut self,
        child: &[u8],
        name: &[u8],
        target: Vec<u8>,
        prefix: Vec<Vec<u8>>,
    ) -> bool {
        match self.install_parent_alias_with_original(child, name, target, prefix, None) {
            Ok(installed) => installed,
            Err(error) => {
                self.report_cmd_error(error.into());
                false
            }
        }
    }

    fn install_parent_alias_with_original(
        &mut self,
        child: &[u8],
        name: &[u8],
        target: Vec<u8>,
        prefix: Vec<Vec<u8>>,
        jim_prefix: Option<obj::Owned>,
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        use tcl_syntax::value::ValueError;
        let producer =
            self.name_policy_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "parent alias publication issuer",
                ))?;
        self.with_child(child, |c| {
            let receiver =
                c.name_policy_protocol()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "child alias publication issuer",
                    ))?;
            if producer != receiver {
                return Err(ValueError::CommandProtocolUnavailable(
                    "child alias producer realm",
                ));
            }
            let selected = receiver
                .recipe()
                .child_alias_publication_input(tcl_syntax::naming::NativeNameContext::root(), name)
                .map_err(|_| {
                    ValueError::CommandProtocolUnavailable("child alias publication input")
                })?;
            let publication_name = selected
                .alias_registration_report_bytes()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "child alias registration report",
                ))?
                .to_vec();
            let current = c.current_ns.get();
            let report = c.namespaces().jim_command_table_key_at(
                current,
                name,
                tcl_syntax::naming::NativeNamePurpose::ChildAliasPublication,
            );
            let (namespace, simple) = c
                .namespaces_mut()
                .child_alias_publication_at(current, name)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "child alias publication holder",
                ))?;
            c.bind_command_replacement_with_jim_key(
                namespace,
                &simple,
                Command::ParentAlias {
                    target,
                    prefix,
                    publication_name,
                    jim_prefix,
                    identity: Rc::new(()),
                },
                report,
            );
            Ok(true)
        })
        .unwrap_or(Ok(false))
    }

    /// The `(target, prefix)` of the alias bound to `name` (the query form), or
    /// `None` if `name` resolves to something that isn't an alias.
    pub(crate) fn alias_info(
        &self,
        name: &[u8],
    ) -> Result<Option<(Vec<u8>, Vec<Vec<u8>>)>, tcl_syntax::value::ValueError> {
        self.alias_info_at(self.current_ns.get(), name)
    }

    pub(crate) fn alias_info_at(
        &self,
        namespace: NsId,
        name: &[u8],
    ) -> Result<Option<(Vec<u8>, Vec<Vec<u8>>)>, tcl_syntax::value::ValueError> {
        let Some(command) = self.namespaces.borrow().resolve(namespace, name) else {
            return Ok(None);
        };
        let Some(mut words) = self.alias_prefix_bytes_checked(&command)? else {
            return Ok(None);
        };
        if words.is_empty() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original alias prefix target",
            ));
        }
        let target = words.remove(0);
        Ok(Some((target, words)))
    }

    /// Return the actual Jim prefix header selected by the original name object.
    pub(crate) fn original_alias_prefix_value(
        &mut self,
        original_name: *mut TclObj,
    ) -> Result<tcl_runtime_api::AliasPrefixLookup<*mut TclObj>, tcl_syntax::value::ValueError>
    {
        if self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_none()
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Jim alias prefix issuer",
            ));
        }
        let Some((command, _)) = self.resolve_original_command(original_name)? else {
            return Ok(tcl_runtime_api::AliasPrefixLookup::MissingCommand);
        };
        match command {
            Command::Alias {
                jim_prefix: Some(original),
                ..
            } => {
                obj::check_native_liveness(original.as_ptr())?;
                Ok(tcl_runtime_api::AliasPrefixLookup::Prefix(
                    original.as_ptr(),
                ))
            }
            Command::Alias { .. } => {
                Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original Jim alias prefix object",
                ))
            }
            _ => Ok(tcl_runtime_api::AliasPrefixLookup::NotAlias),
        }
    }

    /// Select the actual interned generation before projecting alias bytes.
    pub(crate) fn command_alias_prefix_by_id_checked(
        &self,
        id: u32,
    ) -> Result<Option<Vec<Vec<u8>>>, tcl_syntax::value::ValueError> {
        let (_, generation) = self.command_identity(id).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("alias prefix command token"),
        )?;
        let Some(command) = self.raw_command_by_generation(generation) else {
            return Ok(None);
        };
        self.alias_prefix_bytes_checked(&command)
    }

    /// Project bytes only for an actual byte query. Alias definition and
    /// object introspection keep every original prefix member untouched.
    pub(crate) fn alias_prefix_bytes_checked(
        &self,
        command: &Command,
    ) -> Result<Option<Vec<Vec<u8>>>, tcl_syntax::value::ValueError> {
        let Command::Alias {
            target,
            prefix,
            jim_prefix,
            ..
        } = command
        else {
            return Ok(None);
        };
        if let Some(original) = jim_prefix {
            let protocol = self
                .native_invocation_dialect()
                .native_string_protocol()
                .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "alias prefix string protocol",
                ))?;
            obj::check_native_liveness(original.as_ptr())?;
            let members = crate::list::list_elements_native_checked(original.as_ptr(), protocol)?;
            return members
                .into_iter()
                .map(|member| {
                    self.native_object_string_bytes(member)
                        .map(|bytes| bytes.to_vec())
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Some);
        }
        let mut words = Vec::with_capacity(prefix.len() + 1);
        words.push(target.clone());
        words.extend_from_slice(prefix);
        Ok(Some(words))
    }

    /// The interp-path (from `self`) to the target interpreter of the alias
    /// `alias` in the interpreter addressed by `path` — `interp target path
    /// alias`. `None` when `path` doesn't resolve, or names no alias there.
    ///
    /// Every alias this runtime supports targets either its own interpreter
    /// (`Command::Alias`, a same-interp `interp alias`) or its immediate
    /// parent (`Command::ParentAlias`, the child-side half of a cross-interp
    /// alias) — so the target's path from `self` is either `path` itself or
    /// `path` with its last element dropped. C's general `Tcl_GetInterpPath`
    /// walk (`tclInterp.c`) collapses to exactly that for every alias shape
    /// this runtime can construct.
    pub(crate) fn alias_target_path(
        &mut self,
        path: &[Vec<u8>],
        alias: &[u8],
    ) -> Option<Vec<Vec<u8>>> {
        let path_owned = path.to_vec();
        self.with_child_path(path, move |c| {
            match c.namespaces.borrow().resolve(c.current_ns.get(), alias) {
                Some(Command::Alias { .. }) => Some(path_owned),
                Some(Command::ParentAlias { .. }) => {
                    Some(path_owned[..path_owned.len().saturating_sub(1)].to_vec())
                }
                _ => None,
            }
        })
        .flatten()
    }

    /// Delete the command bound to `name` (the alias-clear form); returns whether
    /// it existed.
    pub(crate) fn delete_command(&mut self, name: &[u8]) -> bool {
        // If `name` is a suspended coroutine, terminate its worker first.
        let source_generation = self.resolve_cmd_token(name);
        if let Some(generation) = source_generation {
            crate::cmd_coro::on_command_deleted(self, generation);
        }
        let ensemble_token = match self
            .namespaces
            .borrow()
            .resolve(self.current_ns.get(), name)
        {
            Some(Command::Ensemble(token)) => Some(token),
            _ => None,
        };
        let deleted = self
            .namespaces
            .borrow_mut()
            .delete(self.current_ns.get(), name);
        self.retire_pending_native_ensemble_roles();
        if deleted {
            if let Some(source_generation) = source_generation {
                let tokens: Vec<_> = ensemble_token.into_iter().collect();
                self.remove_imports_for_deleted_origins([source_generation], &tokens);
            }
        }
        deleted
    }

    /// Remove one command generation without recovering an address from its report.
    pub(crate) fn delete_command_generation(&mut self, generation: u64) -> bool {
        crate::cmd_coro::on_command_deleted(self, generation);
        let slot = self.namespaces().native_command_slot_at_node(generation);
        let removed = if let Some((namespace, simple)) = slot {
            self.namespaces_mut().remove_in(namespace, &simple)
        } else {
            let hidden = self.hidden.borrow().iter().find_map(|(name, binding)| {
                (binding.generation == generation).then(|| name.clone())
            });
            hidden.is_some_and(|name| self.hidden.borrow_mut().remove(&name).is_some())
        };
        if removed {
            self.namespaces_mut().retire_native_command_node(generation);
            self.remove_imports_for_deleted_origins([generation], &[]);
            self.retire_pending_native_ensemble_roles();
        }
        removed
    }

    /// Install a stock ensemble using the selected C registration owner.
    pub(crate) fn create_ensemble(&mut self, name: &[u8], cfg: crate::ensemble::EnsembleConfig) {
        let (ns, tail) = self
            .namespaces
            .borrow_mut()
            .command_publication_at(self.current_ns.get(), name)
            .expect("stock ensemble has a native publication context");
        self.create_ensemble_in_slot(ns, &tail, cfg);
    }

    /// Install one ensemble in its already selected physical command holder.
    pub(crate) fn create_ensemble_in_slot(
        &mut self,
        ns: NsId,
        tail: &[u8],
        mut cfg: crate::ensemble::EnsembleConfig,
    ) {
        self.prepare_original_ensemble_configuration(&mut cfg);
        let fqn = self.namespaces.borrow().command_fqn_at(ns, tail);
        let native_entry = self
            .namespaces
            .borrow_mut()
            .native_command_creation_entry(ns, tail);
        let displaced = self.namespaces.borrow().command_in(ns, tail);
        let old_token = match displaced.as_ref() {
            Some(Command::Ensemble(token)) => Some(token),
            _ => None,
        };
        let new_token = Rc::new(
            crate::ensemble::EnsembleToken::with_configuration_retirement(
                cfg,
                fqn.clone(),
                tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent,
                crate::ensemble::retire_configuration,
            ),
        );

        // Creating an ensemble at an occupied name is command replacement, not
        // in-place token mutation. The old token must become dead (an active
        // unknown callback observes UNKNOWN_DELETED), while imports of the
        // occupied source binding are explicitly reattached to the new token.
        if let Some(old_token) = old_token.as_ref() {
            self.retire_ensemble_identity(old_token);
        } else {
            self.on_command_replaced(&fqn);
        }
        self.namespaces
            .borrow_mut()
            .bind_after_native_creation_entry(
                native_entry,
                tail,
                Command::Ensemble(Rc::clone(&new_token)),
            );
        self.retire_pending_native_ensemble_roles();
        new_token.config().originals.activate();
        self.namespaces
            .borrow_mut()
            .advance_ensemble_export_epoch(new_token.config().ns);
        let (new_fqn, new_generation) = {
            let namespaces = self.namespaces.borrow();
            (
                namespaces.command_fqn_at(ns, tail),
                namespaces
                    .command_generation(ns, tail)
                    .expect("the new ensemble was just bound"),
            )
        };
        if displaced.is_some() {
            self.reattach_missing_import_sources(&new_fqn, new_generation);
        }
        self.retarget_imports_to_ensemble(new_generation, &new_token);
    }

    /// Define a user proc (`proc name params body`). The proc's defining
    /// namespace (where its body runs, and where it is bound) is the namespace
    /// `name` lands in — **relative to the current namespace** (so `proc next`
    /// inside `namespace eval counter` binds `::counter::next`, not a global).
    #[cfg(test)]
    pub(crate) fn define_proc(&mut self, name: &[u8], params: Vec<Param>, body_obj: *mut TclObj) {
        self.define_proc_native(name, params, body_obj, None);
    }

    /// Test constructor forwarding an explicit compiled entry to the
    /// original procedure storage owner.
    #[cfg(test)]
    pub(crate) fn define_proc_native(
        &mut self,
        name: &[u8],
        params: Vec<Param>,
        body_obj: *mut TclObj,
        native: Option<NativeProcEntry>,
    ) {
        self.define_proc_storage(name, params, body_obj, native, None);
    }

    pub(crate) fn define_proc_storage(
        &mut self,
        name: &[u8],
        params: Vec<Param>,
        body_obj: *mut TclObj,
        native: Option<NativeProcEntry>,
        statics: Option<Rc<crate::frame::StaticVariables>>,
    ) {
        self.define_proc_original_storage(name, params, None, body_obj, native, statics);
    }

    pub(crate) fn define_proc_original_storage(
        &mut self,
        name: &[u8],
        params: Vec<Param>,
        parameters_obj: Option<*mut TclObj>,
        body_obj: *mut TclObj,
        native: Option<NativeProcEntry>,
        statics: Option<Rc<crate::frame::StaticVariables>>,
    ) {
        let _ = self.install_proc_original_storage(
            name,
            params,
            parameters_obj,
            body_obj,
            native,
            statics,
        );
    }

    /// Install original procedure storage and return its actual command generation.
    pub(crate) fn install_proc_original_storage(
        &mut self,
        name: &[u8],
        params: Vec<Param>,
        parameters_obj: Option<*mut TclObj>,
        body_obj: *mut TclObj,
        native: Option<NativeProcEntry>,
        statics: Option<Rc<crate::frame::StaticVariables>>,
    ) -> Option<u64> {
        if self.host_refusal_pending() {
            return None;
        }
        let chosen = match self.choose_original_procedure_body(body_obj) {
            Ok(chosen) => chosen,
            Err(error) => {
                self.report_cmd_error(error.into());
                return None;
            }
        };
        self.install_proc_chosen_storage(
            name,
            params,
            parameters_obj,
            (body_obj, chosen),
            native,
            statics,
        )
    }

    pub(crate) fn define_proc_chosen_storage(
        &mut self,
        name: &[u8],
        params: Vec<Param>,
        parameters_obj: Option<*mut TclObj>,
        body: (*mut TclObj, obj::Owned),
        native: Option<NativeProcEntry>,
        statics: Option<Rc<crate::frame::StaticVariables>>,
    ) {
        let _ =
            self.install_proc_chosen_storage(name, params, parameters_obj, body, native, statics);
    }

    /// Bind the chosen original procedure and return its installed generation.
    pub(crate) fn install_proc_chosen_storage(
        &mut self,
        name: &[u8],
        params: Vec<Param>,
        parameters_obj: Option<*mut TclObj>,
        body: (*mut TclObj, obj::Owned),
        native: Option<NativeProcEntry>,
        statics: Option<Rc<crate::frame::StaticVariables>>,
    ) -> Option<u64> {
        if self.host_refusal_pending() {
            return None;
        }
        let (original_body, body) = body;
        if self.native_invocation_dialect().native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            if let Err(error) = self.native_jim_object_context() {
                self.report_cmd_error(error.into());
                return None;
            }
        }
        let publication = self
            .namespaces_mut()
            .procedure_publication_at(self.current_ns.get(), name);
        let (binding_ns, tail) = match publication {
            Some(slot) => slot,
            None => {
                self.refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "procedure publication namespace",
                    ),
                );
                return None;
            }
        };
        let ns = if self
            .name_policy_protocol()
            .is_some_and(|protocol| protocol.recipe().is_jim084())
        {
            let context = self.namespaces_mut().jim_procedure_context(&tail);
            match context {
                Some(context) => context,
                None => {
                    self.refuse_native_access(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("Jim procedure namespace object"));
                    return None;
                }
            }
        } else {
            binding_ns
        };
        let fqn = self.namespaces().command_fqn_at(binding_ns, &tail);
        // The proc's body frame reports `type source` with file-absolute lines
        // when its body argument is a located literal (TIP 280 LABC) — the body
        // word, not the `proc` command, carries the location, so a `proc` whose
        // body opens on a later line than the command is still file-accurate, and
        // a *dynamic* body (`proc p {} $bodyVar`, or a body from a dynamically
        // built list) has no location and stays body-relative (`type proc`),
        // matching C's literal line table rather than a whole-file "am I
        // sourcing" flag.
        let (source, body_line_base) = match self.arg_loc(original_body) {
            Some((file @ Some(_), line)) => (file, line.saturating_sub(1)),
            _ => (None, 0),
        };
        let jim = self.native_invocation_dialect().native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084);
        let jim_namespace = if jim {
            self.namespaces_mut().take_jim_namespace_owner(ns)
        } else {
            None
        };
        let command = NativeProcedureCommand::new(
            params,
            body,
            native_procedure_resources::NativeProcedureDefinition {
                location: ProcLocation {
                    namespace: ns,
                    qualified_name: fqn.clone(),
                    jim_namespace,
                },
                jim_parameters: if jim {
                    parameters_obj.map(obj::Owned::retain)
                } else {
                    None
                },
                source,
                body_line_base,
                native,
                statics,
            },
        );
        let def = command.declaration();
        let report = self.namespaces().jim_command_table_key_at(
            self.current_ns.get(),
            name,
            tcl_syntax::naming::NativeNamePurpose::CommandPublication,
        );
        self.bind_command_replacement_with_jim_key(
            binding_ns,
            &tail,
            Command::Proc(command),
            report,
        );
        let generation = self.namespaces().command_generation(binding_ns, &tail);
        let header = match self.original_procedure_compiler_header(parameters_obj, original_body) {
            Ok(header) => header,
            Err(error) => {
                self.report_cmd_error(error.into());
                return None;
            }
        };
        def.compiler_header.set(header);
        if let Some(generation) = generation {
            let recipe = match header {
                tcl_dialect::NativeProcedureHeaderCompilation::Absent => {
                    crate::namespace::NativeCompilerRecipe::Absent
                }
                tcl_dialect::NativeProcedureHeaderCompilation::NoOp => {
                    crate::namespace::NativeCompilerRecipe::ProcedureNoOp
                }
                tcl_dialect::NativeProcedureHeaderCompilation::Unknown => {
                    crate::namespace::NativeCompilerRecipe::Unknown
                }
            };
            self.namespaces_mut()
                .set_native_compiler_recipe(generation, recipe);
        }
        generation
    }

    /// Install a fresh command token at an exact namespace binding, applying
    /// Tcl's command-replacement lifecycle first. The displaced command's
    /// delete trace runs while it is still visible, and all command/execution
    /// trace sidecars belonging to that old token are discarded before the new
    /// command is bound. Fresh bindings use the same funnel (the lifecycle step
    /// is then a no-op).
    pub(crate) fn bind_command_replacement(&mut self, ns: NsId, tail: &[u8], command: Command) {
        self.bind_command_replacement_with_jim_key(ns, tail, command, None);
    }

    fn bind_command_replacement_with_jim_key(
        &mut self,
        ns: NsId,
        tail: &[u8],
        command: Command,
        incoming: Option<tcl_syntax::naming::NativeJimCommandTableKey>,
    ) {
        let native_entry = self
            .namespaces
            .borrow_mut()
            .native_command_creation_entry_with_jim_key(ns, tail, incoming);
        if self.jim_local_depth.get() != 0
            && self
                .native_invocation_dialect()
                .native_jim_local_protocol()
                .is_some()
        {
            self.namespaces
                .borrow_mut()
                .bind_jim_local(native_entry, tail, command);
            self.retire_pending_native_ensemble_roles();
            return;
        }
        let displaced = self.namespaces.borrow().command_in(ns, tail);
        self.on_bound_command_replaced(ns, tail);
        if let Some(owner) = displaced.as_ref().and_then(Command::oo_object) {
            // The old object command's delete trace has run while the token was
            // still visible. Unlink that exact binding before TclOO teardown so
            // identity-based cleanup cannot remove the replacement installed
            // below, then run the displaced owner's destructor and cascades.
            let still_displaced =
                self.namespaces
                    .borrow()
                    .command_in(ns, tail)
                    .is_some_and(|current| {
                        displaced
                            .as_ref()
                            .is_some_and(|old| old.is_same_binding(&current))
                    });
            if still_displaced {
                self.namespaces.borrow_mut().remove_in(ns, tail);
            }
            self.oo_command_renamed(owner, None);
        }
        self.namespaces
            .borrow_mut()
            .bind_after_native_creation_entry(native_entry, tail, command);
        self.retire_pending_native_ensemble_roles();
        if displaced.is_some() {
            let (fqn, generation) = {
                let namespaces = self.namespaces.borrow();
                (
                    namespaces.command_fqn_at(ns, tail),
                    namespaces
                        .command_generation(ns, tail)
                        .expect("the replacement command was just bound"),
                )
            };
            self.reattach_missing_import_sources(&fqn, generation);
        }
    }

    /// A command at `fqn` is being replaced or deleted: fire its `delete`
    /// command traces, then drop every command/execution trace on it (the
    /// command — and its trace list — go away). No-op when it has no traces.
    ///
    /// Only the *dying token's* traces are dropped. C frees `cmdPtr->tracePtr`,
    /// so a trace the callback adds to the command being deleted dies with it,
    /// while one it registers on a replacement it bound at the same name lives
    /// on that new token.
    fn on_command_replaced(&mut self, fqn: &[u8]) {
        let dying = self.resolve_cmd_token(fqn);
        self.fire_delete_traces_of_token(fqn, dying);
    }

    /// [`on_command_replaced`] for an exact binding the caller already holds.
    /// A fully-qualified name is not a token identity once a retained
    /// namespace and a same-named recreation both hold `::N::q`: resolving the
    /// name again would find the live one and fire its traces instead. An
    /// unbound slot has no token, so — like C's `TclCreateObjCommandInNs`,
    /// which only deletes when `Tcl_FindHashEntry` found an entry — nothing
    /// fires and nothing is dropped.
    pub(crate) fn on_bound_command_replaced(&mut self, ns: NsId, tail: &[u8]) {
        let (fqn, dying) = {
            let namespaces = self.namespaces.borrow();
            (
                namespaces.command_fqn_at(ns, tail),
                namespaces.command_generation(ns, tail),
            )
        };
        if dying.is_none() {
            return;
        }
        self.fire_delete_traces_of_token(&fqn, dying);
        if let Some(generation) = dying {
            crate::cmd_coro::on_command_deleted(self, generation);
        }
    }

    /// Current visible, retained, and hidden locations of an owner's exact
    /// command tokens, optionally restricted to one TclOO dispatcher role.
    fn oo_command_locations(
        &self,
        owner: OoId,
        role: Option<OoCommandRole>,
    ) -> Vec<(Vec<u8>, u64)> {
        let mut locations = self
            .namespaces
            .borrow()
            .oo_command_role_locations(owner, role);
        locations.extend(
            self.hidden
                .borrow()
                .iter()
                .filter(|(_, binding)| {
                    binding.command.oo_binding().is_some_and(|(id, candidate)| {
                        id == owner && role.is_none_or(|expected| expected == candidate)
                    })
                })
                .map(|(name, binding)| {
                    let mut fqn = b"::".to_vec();
                    fqn.extend_from_slice(name);
                    (fqn, binding.generation)
                }),
        );
        locations
    }

    /// Fire one delete-trace walk per command generation, resolving the
    /// generation's current placement immediately before its turn. An earlier
    /// callback may rename or hide a later token. The callback may also move
    /// the token currently firing, so clean its trace sidecar again at every
    /// post-callback placement rather than falling back to a display name.
    pub(crate) fn fire_oo_command_delete_traces(
        &mut self,
        owner: OoId,
        role: Option<OoCommandRole>,
    ) {
        let generations: std::collections::BTreeSet<u64> = self
            .oo_command_locations(owner, role)
            .into_iter()
            .map(|(_, generation)| generation)
            .collect();
        for generation in generations {
            let location = self
                .oo_command_locations(owner, role)
                .into_iter()
                .find(|(_, candidate)| *candidate == generation);
            if let Some((fqn, _)) = location {
                self.fire_delete_traces_of_token(&fqn, Some(generation));
            }
            for (fqn, _) in self
                .oo_command_locations(owner, role)
                .into_iter()
                .filter(|(_, candidate)| *candidate == generation)
            {
                self.remove_cmd_traces_of_token(&fqn, Some(generation));
            }
        }
    }

    /// Fire an owner's public and private command-token traces in TclOO's
    /// semantic teardown order. Resolving each role immediately before its
    /// turn lets an earlier callback relocate a later dispatcher.
    pub(crate) fn fire_oo_command_role_delete_traces(&mut self, owner: OoId) {
        for role in OO_COMMAND_RETIREMENT_ORDER {
            self.fire_oo_command_delete_traces(owner, Some(role));
        }
    }

    /// Remove already-prefired command roles without replaying their delete
    /// callbacks. Object teardown uses this for the private dispatchers before
    /// instance-variable traces run; the public command remains until after
    /// that namespace phase.
    pub(crate) fn remove_prefired_oo_private_commands(&mut self, owner: OoId) {
        self.remove_oo_command_roles(owner, &OO_PRIVATE_COMMAND_ROLES);
    }

    /// Retire every command token carried by one TclOO owner identity. The
    /// namespace arena covers both visible and retained generations; the
    /// interpreter-owned hidden table is folded into the same transaction.
    /// Delete-trace callbacks may move a token between those stores, so the
    /// commands are rescanned by identity after callbacks before removal.
    pub(crate) fn retire_oo_command_identity(&mut self, owner: OoId) {
        self.retire_oo_command_identity_impl(owner, true);
    }

    /// Remove an owner's command tokens after its semantic role walk already
    /// fired. A later-role callback may add a trace to an earlier-role token;
    /// final removal discards that late trace without revisiting the role.
    pub(crate) fn retire_prefired_oo_command_identity(&mut self, owner: OoId) {
        self.retire_oo_command_identity_impl(owner, false);
    }

    fn retire_oo_command_identity_impl(&mut self, owner: OoId, fire_traces: bool) {
        if !self.0.retiring_oo_commands.borrow_mut().insert(owner) {
            return;
        }

        if fire_traces {
            self.fire_oo_command_role_delete_traces(owner);
        }

        self.remove_oo_command_roles(owner, &OO_COMMAND_RETIREMENT_ORDER);
        self.forget_registry_object_root(owner);
        self.0.retiring_oo_commands.borrow_mut().remove(&owner);
    }

    fn remove_oo_command_roles(&mut self, owner: OoId, roles: &[OoCommandRole]) {
        let mut removed: Vec<(Vec<u8>, u64)> = self
            .namespaces
            .borrow_mut()
            .remove_oo_command_roles(owner, roles);
        let hidden_names: Vec<Vec<u8>> = self
            .hidden
            .borrow()
            .iter()
            .filter(|(_, binding)| {
                binding
                    .command
                    .oo_binding()
                    .is_some_and(|(id, role)| id == owner && roles.contains(&role))
            })
            .map(|(name, _)| name.clone())
            .collect();
        for name in hidden_names {
            if let Some(binding) = self.hidden.borrow_mut().remove(&name) {
                self.namespaces
                    .borrow_mut()
                    .retire_native_command_node(binding.generation);
                let mut fqn = b"::".to_vec();
                fqn.extend_from_slice(&name);
                removed.push((fqn, binding.generation));
            }
        }
        let origins: std::collections::BTreeSet<u64> =
            removed.iter().map(|(_, generation)| *generation).collect();
        self.remove_imports_for_deleted_origins(origins, &[]);
        for (fqn, generation) in removed {
            self.remove_cmd_traces_of_token(&fqn, Some(generation));
        }
    }

    /// Fire `fqn`'s `delete` traces and drop the ones the dying token owned.
    fn fire_delete_traces_of_token(&mut self, fqn: &[u8], dying: Option<u64>) {
        if self
            .traces
            .borrow()
            .cmd_traces
            .iter()
            .all(|t| t.name != fqn)
        {
            return;
        }
        self.fire_cmd_trace_of_token(fqn, fqn, b"", crate::cmd_trace::ops::DELETE, dying);
        self.remove_cmd_traces_of_token(fqn, dying);
    }

    /// The generation of the command token `name` resolves to — the identity a
    /// command trace hangs off.
    pub(crate) fn resolve_cmd_token(&self, name: &[u8]) -> Option<u64> {
        self.namespaces
            .borrow()
            .resolve_generation(self.current_ns.get(), name)
    }

    /// Resolve one retained command token wherever rename or visibility moves
    /// placed it, distinguishing an unavailable token from a retired one.
    fn command_by_generation(&self, generation: u64) -> CommandGenerationLookup {
        let Some((fqn, command)) = self.raw_command_location_by_generation(generation) else {
            return CommandGenerationLookup::Missing;
        };
        if !self.command_visible_for_surface_at(&command, &fqn, Some(generation)) {
            return CommandGenerationLookup::Unavailable;
        }
        CommandGenerationLookup::Found { fqn, command }
    }

    /// Resolve an exact generation and its current projection without applying
    /// dispatch-surface policy.
    fn raw_command_location_by_generation(&self, generation: u64) -> Option<(Vec<u8>, Command)> {
        if let Some((fqn, command)) = self.namespaces.borrow().command_by_generation(generation) {
            return Some((fqn, command));
        }
        self.hidden.borrow().iter().find_map(|(name, binding)| {
            if binding.generation != generation {
                return None;
            }
            let mut fqn = b"::".to_vec();
            fqn.extend_from_slice(name);
            Some((fqn, binding.command.clone()))
        })
    }

    /// The command half of [`Self::raw_command_location_by_generation`].
    fn raw_command_by_generation(&self, generation: u64) -> Option<Command> {
        self.raw_command_location_by_generation(generation)
            .map(|(_, command)| command)
    }

    /// Whether an exact imported-command source chain reaches another command
    /// generation. The import loop gate must not substitute a recreated
    /// same-FQN namespace token for a retained source.
    pub(crate) fn import_chain_contains(&self, source_generation: u64, needle: u64) -> bool {
        let mut generation = source_generation;
        let mut visited = std::collections::BTreeSet::new();
        while visited.insert(generation) {
            if generation == needle {
                return true;
            }
            let Some(Command::Imported {
                source_generation, ..
            }) = self.raw_command_by_generation(generation)
            else {
                return false;
            };
            generation = source_generation;
        }
        false
    }

    /// Resolve an import's retained source token. Tcl redirects follow the
    /// exact generation through rename and visibility moves. Once that token
    /// truly retires, command replacement deliberately falls back through the
    /// mutable source projection; a merely surface-gated token is still
    /// present and must not take that fallback.
    fn resolve_import_source(&self, source: &[u8], generation: u64) -> Option<Command> {
        match self.command_by_generation(generation) {
            CommandGenerationLookup::Found { command, .. } => Some(command),
            CommandGenerationLookup::Unavailable => None,
            CommandGenerationLookup::Missing => self.resolve_dispatchable(GLOBAL, source),
        }
    }

    /// Drop the command/execution traces on `fqn` that belonged to the token
    /// being deleted — C's "free the whole `cmdPtr->tracePtr` list".
    ///
    /// Generations are minted in binding order, so a trace registered on a
    /// replacement the delete callback bound at this same name compares
    /// greater and survives. An unidentifiable dying token (an unbound or
    /// hidden name) takes the whole list, as it did before tokens were
    /// tracked.
    fn remove_cmd_traces_of_token(&mut self, fqn: &[u8], dying: Option<u64>) {
        let mut traces = self.traces.borrow_mut();
        let old_len = traces.cmd_traces.len();
        traces
            .cmd_traces
            .retain(|t| t.name != fqn || !cmd_trace_owned_by(t.token, dying));
        let removed = traces.cmd_traces.len() != old_len;
        drop(traces);
        if removed {
            self.invalidate_guard_domain(GuardDomain::CommandTrace);
        }
    }

    /// The reported `line` of the command currently executing at the top of the
    /// `info frame` stack (for fixing a source-defined proc's body line base).
    fn current_cmd_line(&self) -> u32 {
        self.cmd_frames.borrow().last().map_or(1, |f| f.line)
    }

    /// The file-absolute source line of argument `idx` of the command currently
    /// being dispatched (TIP 280); falls back to the command line. Read by a
    /// body-defining command for its body word.
    pub(crate) fn arg_line(&self, idx: usize) -> u32 {
        let lines = self.arg_lines.borrow();
        lines
            .get(idx)
            .copied()
            .unwrap_or_else(|| self.current_cmd_line())
    }

    /// Snapshot / restore the current argument lines (TIP 280) — used when a
    /// command re-dispatches a sub-slice of its own words (e.g. the
    /// single-command `oo::define <target> <sub> …` form), so the dispatched
    /// subcommand's body word is found at the right index.
    pub(crate) fn arg_lines_snapshot(&self) -> Vec<u32> {
        self.arg_lines.borrow().clone()
    }
    pub(crate) fn set_arg_lines(&self, lines: Vec<u32>) {
        *self.arg_lines.borrow_mut() = lines;
    }

    /// Whether `name` resolves to an ensemble command (`namespace ensemble
    /// exists`).
    pub(crate) fn is_ensemble(&self, name: &[u8]) -> bool {
        self.ensemble_config_at(name).is_some()
    }

    /// The configuration of the ensemble command `name` resolves to (or `None`
    /// if `name` is not an ensemble), plus the fully-qualified name of the
    /// command that actually **owns** that config.
    ///
    /// `namespace import` is followed to its source: in C an imported command
    /// shares the source's command token, and the ensemble config hangs off
    /// that token, so configuring through an alias configures the origin and
    /// both spellings observe one config (tclsh 9.0.4-pinned). Reading through
    /// the alias likewise reads the origin's config, and the alias stays an
    /// alias — `namespace origin` still answers the source.
    pub(crate) fn ensemble_config_at(
        &self,
        name: &[u8],
    ) -> Option<Rc<crate::ensemble::EnsembleToken>> {
        let mut cur = self
            .namespaces
            .borrow()
            .resolve(self.current_ns.get(), name)?;
        // Bounded walk: an import chain cannot outlive the table, and a
        // malformed cycle terminates instead of spinning.
        for _ in 0..64 {
            match cur {
                Command::Ensemble(token) => return Some(token),
                Command::Imported {
                    source,
                    source_generation,
                    ensemble,
                    ..
                } => {
                    if let Some(token) = ensemble {
                        if !token.is_deleted() {
                            return Some(token);
                        }
                    }
                    cur = self.resolve_import_source(&source, source_generation)?;
                }
                _ => return None,
            }
        }
        None
    }

    /// Every alias command's name across the whole tree (`interp aliases`).
    pub(crate) fn alias_names(&self) -> Vec<Vec<u8>> {
        self.namespaces.borrow().alias_names()
    }

    /// The current namespace (the eval context) — for the `namespace` builtin.
    pub(crate) fn current_ns(&self) -> NsId {
        self.current_ns.get()
    }

    /// Set the current namespace context directly, with no frame push and no
    /// restore-on-return of its own — the caller saves/restores
    /// [`Self::current_ns`] around whatever it runs. Used by `interp
    /// invokehidden`'s `-global`/`-namespace` evaluation-context switch,
    /// which invokes one command rather than evaluating a script body, so it
    /// needs no `namespace eval`-style frame.
    pub(crate) fn set_current_ns(&self, ns: NsId) {
        self.current_ns.set(ns);
    }

    /// Enter a **compiled activation** — the eval-loop activation a generated
    /// function or ABI dispatch stands in for.
    ///
    /// The eval loop's outermost-eval rule (`eval_script_mode`, depth 0) is what
    /// publishes an uncaught error's trace and drains the background-error
    /// queue. Compiled code that dispatches a command without entering that loop
    /// therefore runs at depth 0, and any command that evaluates a body — `catch`
    /// above all — sees the rule fire *inside* its body, resetting the exception
    /// state before it can read `error_code()`. Holding an activation for the
    /// span of compiled work restores the invariant that interpreted Tcl always
    /// has: the enclosing activation is depth ≥ 1, so only the true outermost
    /// completion publishes.
    ///
    /// Returns `false` — with the interpreter's error set, and **no** activation
    /// entered, so the caller must not leave one — when the activation would
    /// exceed the native nesting bound. Pair every `true` with exactly one
    /// [`codegen_activation_leave`](Self::codegen_activation_leave).
    pub(crate) fn codegen_activation_enter(&mut self) -> bool {
        if !self.evaluation_is_live() {
            return false;
        }
        if NATIVE_EVAL_DEPTH_LIMIT.exceeded(self.eval_depth.get() + 1) {
            self.error(b"too many nested evaluations (infinite loop?)");
            return false;
        }
        self.reset_outermost_native_error();
        if self.uses_jim_error_stack() {
            self.jim_error_stack.borrow_mut().mark_reset();
        }
        self.eval_depth.set(self.eval_depth.get() + 1);
        true
    }

    /// Leave a compiled activation entered by
    /// [`codegen_activation_enter`](Self::codegen_activation_enter), applying the
    /// outermost-eval rule with `code` as the activation's completion.
    ///
    /// This is the same two-step tail `eval_script_mode` runs after decrementing
    /// the depth — publish an uncaught error's trace to
    /// `::errorInfo`/`::errorCode`, then drain the background-error queue — so
    /// the policy lives in one place and a compiled statement at the true top
    /// level leaves exactly the error state its interpreted twin would.
    pub(crate) fn codegen_activation_leave(&mut self, code: Code) {
        self.codegen_activation_leave_unpublished();
        self.finish_outermost_eval(code);
    }

    fn codegen_activation_leave_unpublished(&self) {
        self.eval_depth.set(self.eval_depth.get().saturating_sub(1));
    }

    /// Record the activation `Tcl_PushCallFrame` adds to the namespace token a
    /// new call frame runs in. Every frame counts — proc, `apply`, TclOO
    /// method, `namespace eval`/`inscope` — and a namespace deleted while the
    /// count is non-zero keeps its contents until the matching pop.
    pub(crate) fn enter_namespace_activation(&self, ns: NsId) {
        self.namespaces.borrow_mut().activation_enter(ns);
    }

    /// Give back the activation a popped frame held. When it was the last one
    /// holding a namespace whose deletion waited on it, the teardown runs here,
    /// exactly as C's `Tcl_PopCallFrame` calls `Tcl_DeleteNamespace` again —
    /// with the frame already gone and the caller's namespace current.
    pub(crate) fn leave_namespace_activation(&mut self, popped: Option<NsId>) {
        let Some(ns) = popped else { return };
        if self.namespaces.borrow_mut().activation_leave(ns) {
            self.delete_namespace_by_id(ns);
        }
    }

    /// Enter a generated procedure body using the ordinary Tcl variable frame.
    pub(crate) fn pop_native_call_frame(&mut self) -> Option<NsId> {
        self.release_native_procedure_execution();
        self.clean_current_jim_local_commands();
        let departed = self.frames.borrow_mut().take_frame_for_pop();
        let (namespace, owners) = departed?;
        let storage = owners.release();
        self.frames.borrow_mut().recycle_jim_storage(storage);
        Some(namespace)
    }

    pub(crate) fn retain_native_jim_frame_namespace(
        &mut self,
        namespace: NsId,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        if self.native_invocation_dialect().native_string_protocol()
            != Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return Ok(());
        }
        let _context = self.native_jim_object_context()?;
        let owner = self
            .namespaces
            .borrow_mut()
            .take_jim_namespace_owner(namespace)
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim activation namespace object",
            ))?;
        self.frames
            .borrow_mut()
            .retain_jim_activation_objects(None, None, owner);
        Ok(())
    }

    pub(crate) fn codegen_frame_push(&mut self) {
        let ns = self.current_ns.get();
        self.enter_namespace_activation(ns);
        self.frames.borrow_mut().push(ns);
        if let Err(error) = self.retain_native_jim_frame_namespace(ns) {
            self.report_cmd_error(error.into());
        }
    }

    /// Leave a generated procedure body and restore its caller's namespace.
    pub(crate) fn codegen_frame_pop(&mut self) {
        self.clean_current_jim_local_commands();
        let popped = {
            let mut frames = self.frames.borrow_mut();
            let popped = frames.take_frame_for_pop();
            self.current_ns.set(frames.frame_ns(frames.current_level()));
            popped
        };
        let popped = popped.map(|(namespace, owners)| {
            let storage = owners.release();
            self.frames.borrow_mut().recycle_jim_storage(storage);
            namespace
        });
        self.leave_namespace_activation(popped);
    }

    /// Associate an indexed generated local with its name-addressable Tcl cell.
    pub(crate) fn codegen_bind_slot(
        &self,
        slot: usize,
        name: &[u8],
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.frames.borrow_mut().bind_compiled_slot(slot, name);
        self.refresh_native_local_name_table()
    }

    /// Resolve a generated local index to its Tcl-visible name.
    pub(crate) fn codegen_slot_name(&self, slot: usize) -> Option<Vec<u8>> {
        self.frames
            .borrow()
            .compiled_slot_name(slot)
            .map(<[u8]>::to_vec)
    }

    /// Whether any variable trace can observe accesses to `name` — the runtime
    /// half of a guarded `TraceBarrier`.
    ///
    /// The name is resolved to the same `(home, simple name)` identity trace
    /// *firing* uses, so this follows `upvar`/`global`/`variable` links to the
    /// target's traces and reports an array as traced when any of its elements
    /// is. It is deliberately conservative in that direction: a `true` may be
    /// broader than the exact access, but a `false` is a promise that nothing
    /// can observe the cell.
    pub(crate) fn var_is_traced(&self, name: &[u8]) -> bool {
        if self.traces.borrow().traces.is_empty() {
            return false;
        }
        let Ok((base, _)) = self.variable_name_parts(name) else {
            return true;
        };
        let home = self.trace_identity(&base);
        self.traces.borrow().traces.iter().any(|t| {
            crate::cmd_trace::same_variable(t, &home.base, home.ns, home.level)
                && t.binding_id == home.binding_id
        })
    }

    /// [`var_is_traced`](Self::var_is_traced) for a compiled slot, answered from
    /// the cell's own cached bit when it is current for this interpreter's
    /// variable-trace epoch.
    pub(crate) fn codegen_slot_is_traced(&self, slot: usize) -> bool {
        let epoch = self.var_trace_epoch.get();
        if let Some(cached) = self.frames.borrow().compiled_slot_trace_flag(slot, epoch) {
            return cached;
        }
        let Some(name) = self.codegen_slot_name(slot) else {
            return false;
        };
        let traced = self.var_is_traced(&name);
        self.frames
            .borrow()
            .set_compiled_slot_trace_flag(slot, epoch, traced);
        traced
    }

    /// The value a generated local slot addresses, by the **O(1) cell path**.
    ///
    /// Taken only when nothing can observe the read differently from a plain
    /// cell load: the cell holds a scalar (not a link, which crosses tables and
    /// is the coordinator's walk), and the interpreter has no variable traces at
    /// all. `None` means "no fast path" — an unbound slot, an undefined or
    /// linked cell, or a traced interpreter — and the caller takes the name path,
    /// which owns the link walk, the trace firing, and the error text.
    pub(crate) fn codegen_slot_scalar(&self, slot: usize) -> Option<*mut TclObj> {
        if self.has_variable_traces() {
            return None;
        }
        match &*self.frames.borrow().compiled_slot_var(slot)? {
            crate::frame::Var::Scalar(value) => Some(*value),
            _ => None,
        }
    }

    /// Begin an ensemble-rewrite (a forward / ensemble / constructor replacing
    /// the original command words). Returns `true` if this is the *root* rewrite
    /// (no rewrite was active) — the caller must `clear_ensemble_rewrite` when
    /// its dispatch returns. Nested rewrites retain the root's original words
    /// and compose their removed/inserted counts.
    pub(crate) fn begin_ensemble_rewrite(
        &self,
        source: Vec<crate::obj::Owned>,
        removed: usize,
        inserted: usize,
    ) -> bool {
        self.begin_original_ensemble_rewrite(
            source.into_iter().map(EnsembleRewriteWord::Owned).collect(),
            removed,
            inserted,
        )
    }

    fn begin_original_ensemble_rewrite(
        &self,
        source: Vec<EnsembleRewriteWord>,
        removed: usize,
        inserted: usize,
    ) -> bool {
        let mut rw = self.ensemble_rewrite.borrow_mut();
        match rw.as_mut() {
            None => {
                *rw = Some(EnsembleRewrite {
                    source,
                    removed,
                    inserted,
                });
                true
            }
            // A nested rewrite chains onto the root (C's `TclInitRewriteEnsemble`):
            // the root `source` is kept, but its removed/inserted counts absorb the
            // inner step so a deeply forwarded `wrong # args` still prints the full
            // original prefix.
            Some(r) => {
                if r.inserted < removed {
                    r.removed += removed - r.inserted;
                    r.inserted = inserted;
                } else {
                    r.inserted = (r.inserted + inserted).saturating_sub(removed);
                }
                false
            }
        }
    }

    /// Clear the active ensemble-rewrite (paired with a root `begin_…`).
    pub(crate) fn clear_ensemble_rewrite(&self) {
        *self.ensemble_rewrite.borrow_mut() = None;
    }

    /// The active ensemble-rewrite, if any.
    pub(crate) fn ensemble_rewrite(&self) -> Option<EnsembleRewrite> {
        self.ensemble_rewrite.borrow().clone()
    }

    pub(super) fn reset_native_ensemble_rewrite(
        &mut self,
        event: tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent,
    ) -> Result<(), Code> {
        use tcl_registry::native_ensemble_rewrite::LogicalEnsembleRewriteProvider;
        let Some(protocol) = self
            .native_invocation_dialect()
            .ensemble_rewrite_protocol(Some(LogicalEnsembleRewriteProvider::Tcl84CoreSimulation))
        else {
            return Err(self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "ensemble rewrite reset",
                ),
            ));
        };
        if protocol.resets_at(event) {
            self.clear_ensemble_rewrite();
        }
        Ok(())
    }

    /// The namespace tree (read) — for the `namespace` builtin's queries. The
    /// returned `Ref` must not be held across a call that mutably borrows the
    /// namespaces (it would panic); callers use it for a single query.
    pub(crate) fn namespaces(&self) -> std::cell::Ref<'_, Namespaces> {
        self.namespaces.borrow()
    }

    /// The namespace tree (mutable) — for the `namespace` builtin's mutations
    /// (`export`/`import`/`forget`/`path`).
    pub(crate) fn namespaces_mut(&self) -> std::cell::RefMut<'_, Namespaces> {
        // This escape hatch is used only by namespace/OO mutators. Invalidate
        // conservatively before exposing mutable namespace state.
        self.namespaces.borrow_mut()
    }

    /// Bootstrap the standard library like C's `Tcl_Init`: source
    /// `$tcl_library/init.tcl`. After this the
    /// pure-Tcl `unknown`/auto-load/`package` machinery is live, so
    /// `package require` works through `pkgIndex.tcl`/`tclIndex`.
    /// Install the embedding host's globals, including application arguments
    /// and a selected library path. Native core construction is separate.
    pub(crate) fn set_startup_globals(&mut self) {
        let set = |i: &mut Interp, name: &[u8], val: &[u8]| {
            let o = new_string(val);
            if i.var_set(name, o).is_err() {
                drop_fresh(o);
            }
        };
        // `tcl_version`/`tcl_patchLevel` are NOT set here: C sets them in
        // `Tcl_CreateInterp`, and so does this runtime (see `Interp::new`).
        // Re-derived rather than re-literalled so a non-9.0
        // `set_runtime_version` is not silently overwritten by `Tcl_Init`.
        self.write_release_globals();
        set(self, b"::tcl_interactive", b"0");
        set(self, b"::argv", b"");
        set(self, b"::argv0", b"");
        set(self, b"::argc", b"0");
        self.rebootstrap_host_globals();
    }

    fn bootstrap_native_core(
        &mut self,
        protocol: tcl_registry::special_vars::NativeBootstrapProtocol,
        inputs: &tcl_registry::special_vars::NativeBootstrapInputs,
    ) {
        use tcl_registry::special_vars::{NativeBootstrapPurpose, NativeBootstrapVariable as V};
        let snapshot =
            tcl_platform::bootstrap::snapshot(&*self.host(), "treewalk", env!("CARGO_PKG_VERSION"));
        for variable in protocol
            .allocations(
                NativeBootstrapPurpose::CreateInterpreter,
                inputs.default_library.is_some(),
            )
            .expect("selected native core allocation plan")
        {
            let name = variable.name().as_bytes();
            match variable {
                V::ErrorInfo | V::ErrorCode | V::Precision => {
                    self.ensure_trace_variable(name)
                        .expect("selected native root shell");
                }
                V::Environment | V::Platform => {
                    self.ensure_array(name).expect("fresh native root array");
                    let entries: Vec<(&str, &str)> = if variable == V::Environment {
                        snapshot
                            .environment()
                            .iter()
                            .map(|(key, value)| (key.as_str(), value.as_str()))
                            .collect()
                    } else {
                        snapshot
                            .platform()
                            .iter()
                            .map(|(key, value)| (*key, value.as_str()))
                            .collect()
                    };
                    for (key, value) in entries {
                        self.set_global_element_raw(name, key.as_bytes(), value.as_bytes());
                    }
                }
                V::PatchLevel => {
                    self.set_global_raw(name, self.runtime_version().patchlevel().as_bytes())
                }
                V::Version => {
                    self.set_global_raw(name, self.runtime_version().version_string().as_bytes())
                }
                V::PackagePath => {
                    let members = tcl_syntax::list::split_native_list_bytes(
                        &inputs.package_path,
                        protocol.names().string_protocol(),
                    )
                    .expect("validated build package path");
                    let members: Vec<_> = members
                        .into_iter()
                        .map(|member| obj::Owned::fresh(new_string(&member)))
                        .collect();
                    let pointers: Vec<_> = members.iter().map(obj::Owned::as_ptr).collect();
                    let value = obj::Owned::fresh(crate::list::new_list_obj_native(
                        &pointers,
                        protocol.names().string_protocol(),
                    ));
                    self.var_set_at(name, value.as_ptr(), 0)
                        .expect("fresh native package path");
                }
                V::AutoPath => self.set_global_raw(name, &inputs.package_path),
                V::DefaultLibrary => self.set_global_raw(
                    name,
                    inputs
                        .default_library
                        .as_deref()
                        .expect("selected platform default library"),
                ),
                V::Interactive => self.set_global_raw(name, b"0"),
                V::Argv0 | V::Argc | V::Argv => {
                    unreachable!("constructor plan excludes main arguments")
                }
            }
        }
        self.install_native_error_variable_traces();
        self.install_native_precision_trace();
    }

    fn set_global_raw(&mut self, name: &[u8], value: &[u8]) {
        let object = new_string(value);
        if self.var_set_at(name, object, 0).is_err() {
            drop_fresh(object);
        }
    }

    fn set_global_element_raw(&mut self, name: &[u8], key: &[u8], value: &[u8]) {
        let object = new_string(value);
        if self.var_set_elem_at(name, key, object, 0).is_err() {
            drop_fresh(object);
        }
    }

    /// Replace the complete host-derived bootstrap surface without firing Tcl
    /// variable traces between its clear and install phases.
    fn rebootstrap_host_globals(&mut self) {
        let snapshot =
            tcl_platform::bootstrap::snapshot(&*self.host(), "treewalk", env!("CARGO_PKG_VERSION"));
        for name in tcl_platform::bootstrap::HOST_ARRAYS {
            self.quiet_var_unset_at(format!("::{name}").as_bytes(), 0);
        }
        for name in tcl_platform::bootstrap::HOST_PATH_GLOBALS {
            self.quiet_var_unset_at(format!("::{name}").as_bytes(), 0);
        }
        self.ensure_array(b"::tcl_platform")
            .expect("fresh tcl_platform array");
        self.ensure_array(b"::env").expect("fresh env array");
        self.set_global_raw(b"::tcl_library", snapshot.tcl_library().as_bytes());
        self.set_global_raw(b"::auto_path", b"");
        for (name, value) in snapshot.platform() {
            self.set_global_element_raw(b"::tcl_platform", name.as_bytes(), value.as_bytes());
        }
        for (name, value) in snapshot.environment() {
            self.set_global_element_raw(b"::env", name.as_bytes(), value.as_bytes());
        }
    }

    pub fn init_library(&mut self) -> Code {
        let lib = self.host().env().get("TCL_LIBRARY").unwrap_or_default();
        // Source init.tcl, which sets up unknown/auto-load/package + appends
        // tcl_library (and its parent) to auto_path.
        let init_path = format!("{lib}/init.tcl");
        let bytes = self
            .host()
            .filesystem()
            .and_then(|fs| fs.read(&init_path).ok());
        match bytes {
            Some(bytes) => {
                let active = self.begin_package_initialization();
                let code = self.eval_sourced(&bytes, init_path.as_bytes());
                self.end_package_initialization(active);
                code
            }
            None => {
                let mut m = b"can't find ".to_vec();
                m.extend_from_slice(init_path.as_bytes());
                m.extend_from_slice(b" (set TCL_LIBRARY)");
                self.error(&m)
            }
        }
    }

    /// Record `return -level L -code C` state (set by the `return` command).
    pub(crate) fn set_return_state(&mut self, level: usize, code: Code) {
        self.return_level.set(level);
        self.return_code.set(code);
    }

    /// Replace the arbitrary option pairs carried by the current `return`.
    pub(crate) fn set_return_options(&self, options: Vec<(Vec<u8>, Vec<u8>)>) {
        self.set_return_option_objects(
            options
                .into_iter()
                .map(
                    |(key, value)| tcl_cmd_core::return_options::ReturnOptionPair {
                        key: obj::Owned::fresh(obj::new_string_bytes(&key)),
                        value: obj::Owned::fresh(obj::new_string_bytes(&value)),
                        key_bytes: key,
                    },
                )
                .collect(),
        );
    }

    /// Retain exact original option objects across return and coroutine boundaries.
    pub(crate) fn set_return_option_objects(
        &self,
        options: Vec<tcl_cmd_core::return_options::ReturnOptionPair<obj::Owned>>,
    ) {
        let options = match NativeReturnOptions::new(
            options,
            self.native_invocation_dialect()
                .native_error_objects_protocol(),
        ) {
            Ok(options) => options,
            Err(error) => {
                self.clone().report_cmd_error(error.into());
                return;
            }
        };
        let retired = self.return_options.replace(options);
        drop(retired);
    }

    pub(crate) fn pending_return_option_objects(
        &self,
    ) -> Vec<tcl_cmd_core::return_options::ReturnOptionPair<obj::Owned>> {
        self.return_options.borrow().pairs()
    }

    fn take_return_options(&self) -> NativeReturnOptions {
        std::mem::take(&mut *self.return_options.borrow_mut())
    }

    fn restore_return_options(&self, options: NativeReturnOptions) {
        let retired = self.return_options.replace(options);
        drop(retired);
    }

    /// Begin an evaluation/completion boundary with no carried options.
    pub(crate) fn clear_return_options(&self) {
        let retired = self.return_options.replace(NativeReturnOptions::default());
        drop(retired);
    }

    /// Enter a control-command body under the shared completion-option policy.
    pub(crate) fn begin_control_options(
        &self,
        policy: tcl_runtime_api::completion_options::ControlOptionPolicy,
    ) {
        if policy.begins_fresh() {
            self.clear_return_options();
        }
    }

    /// Settle a control command's successful completion under the shared policy.
    pub(crate) fn settle_control_options(
        &self,
        policy: tcl_runtime_api::completion_options::ControlOptionPolicy,
        code: Code,
    ) {
        if code == Code::Ok && policy.settles_success() {
            self.clear_return_options();
        }
    }

    /// The pending `return` `-code`/`-level` (the options a body that completed
    /// via `return` would propagate) — for `catch`/`try`'s options dict and TIP
    /// 329 `-during` chaining.
    pub(crate) fn pending_return_code(&self) -> Code {
        self.return_code.get()
    }

    pub(crate) fn pending_return_level(&self) -> usize {
        self.return_level.get()
    }

    /// Apply a procedure/source **return boundary** to a body completion code
    /// (`TclUpdateReturnInfo`): a `Code::Return` decrements the pending
    /// `-level`; when it reaches 0 the boundary completes with the pending
    /// `-code` (so `return` → Ok, `return -code error` → Error). Other codes
    /// pass through.
    fn settle_return(&mut self, code: Code) -> Code {
        if code != Code::Return {
            return code;
        }
        self.return_level
            .set(self.return_level.get().saturating_sub(1));
        if self.return_level.get() == 0 {
            let c = self.return_code.get();
            self.return_code.set(Code::Ok);
            c
        } else {
            Code::Return
        }
    }

    /// Settle the selected native file boundary independently of procedure return.
    fn settle_source_return(&mut self, code: Code) -> Code {
        use tcl_registry::completion::CompletionCode;
        use tcl_registry::completion_route::{
            InvocationCompletionRoute as Route, ReturnCompletionRoute,
        };
        let route = if code == Code::Return {
            Route::Return(ReturnCompletionRoute {
                eventual_code: CompletionCode::from_int(self.return_code.get().as_int() as i32),
                remaining_level: self.return_level.get() as u64,
            })
        } else {
            Route::Tcl(CompletionCode::from_int(code.as_int() as i32))
        };
        match tcl_registry::source_file::completion_route(self.native_invocation_dialect(), route) {
            Route::Return(pending) => {
                self.return_level.set(pending.remaining_level as usize);
                self.return_code
                    .set(Code::from_int(pending.eventual_code.as_int() as i32));
                Code::Return
            }
            Route::Tcl(code) => {
                if matches!(route, Route::Return(_)) {
                    self.return_level.set(0);
                    self.return_code.set(Code::Ok);
                }
                Code::from_int(code.as_int() as i32)
            }
            _ => self.set_error(b"selected native source completion is unavailable"),
        }
    }

    /// Evaluate one source boundary, projecting its live completion before
    /// outermost error propagation and background-error processing.
    fn eval_sourced_boundary<T>(
        &mut self,
        script: &[u8],
        name: &[u8],
        project: impl FnOnce(&mut Self, Code) -> T,
    ) -> T {
        self.reset_outermost_native_error();
        if let Err(error) = self.record_package_source_file(name) {
            let code = self.report_cmd_error(error.into());
            return project(self, code);
        }
        self.clear_return_options();
        self.script_stack.borrow_mut().push(name.to_vec());
        // A `source`d file is its own `info frame` level: `type source` + the
        // file path, inheriting the enclosing proc/level. Its commands are
        // numbered by the file's own lines (base 0, the file *is* the script).
        let mut frame = self.inherited_cmd_frame();
        frame.kind = FrameKind::Source;
        frame.file = Some(Rc::from(name));
        frame.line_base = 0;
        let code = if self.native_invocation_dialect().native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            self.eval_native_jim_source_unpublished(script, name, frame)
        } else {
            self.eval_framed_unpublished(script, frame)
        };
        self.script_stack.borrow_mut().pop();
        let code = self.settle_source_return(code);
        let projected = project(self, code);
        if self
            .native_invocation_dialect()
            .native_eval_object_protocol()
            .is_some_and(|protocol| protocol.clears_public_source_error_logged())
        {
            self.exc.borrow_mut().already_logged = false;
        }
        self.finish_outermost_eval(code);
        projected
    }

    /// `source`: evaluate `script` as a sourced file named `name`, tracking it on
    /// the script stack (`info script`). A top-level `return` ends the file (the
    /// return boundary maps `return` → Ok); other codes propagate.
    pub fn eval_sourced(&mut self, script: &[u8], name: &[u8]) -> Code {
        self.eval_sourced_boundary(script, name, |_, code| code)
    }

    /// Evaluate a package file in the interpreter-global frame and namespace.
    pub(crate) fn eval_sourced_global(&mut self, script: &[u8], name: &[u8]) -> Code {
        let previous_level = self.frames.borrow_mut().set_active_level(0);
        let previous_namespace = self.current_ns.get();
        self.current_ns.set(self.frames.borrow().frame_ns(0));
        let code = self.eval_sourced(script, name);
        self.frames.borrow_mut().set_active_level(previous_level);
        self.current_ns.set(previous_namespace);
        code
    }

    /// Evaluate a sourced script and return its owned, byte-preserving
    /// completion.
    ///
    /// The source return boundary is settled before the snapshot. An uncaught
    /// error is published to Tcl's globals only after its live options,
    /// including `-during`, have been captured. Host refusals return their typed
    /// original cause without fabricating a Tcl result or options dictionary.
    ///
    /// # Errors
    /// Returns a retained host execution refusal before accessing guest values.
    pub fn eval_sourced_completion(
        &mut self,
        script: &[u8],
        name: &[u8],
    ) -> Result<tcl_runtime_api::ScriptCompletion, tcl_runtime_api::NativeExecutionError> {
        let completion = self.eval_sourced_boundary(script, name, crate::completion::capture_bytes);
        if let Some(error) = self.native_execution_refusal() {
            return Err(error);
        }
        completion
    }

    /// `info script` — the file currently being sourced (empty at top level).
    pub(crate) fn current_script(&self) -> Vec<u8> {
        self.script_stack
            .borrow()
            .last()
            .cloned()
            .unwrap_or_default()
    }

    /// `info script filename` — set the current script name (C's
    /// `iPtr->scriptFile`), replacing the innermost entry (or seeding one at the
    /// top level).
    pub(crate) fn set_current_script(&self, name: &[u8]) {
        let mut s = self.script_stack.borrow_mut();
        match s.last_mut() {
            Some(last) => *last = name.to_vec(),
            None => s.push(name.to_vec()),
        }
    }

    /// The file currently being sourced, as a shared handle (`None` at the top
    /// level) — for stamping a definition/method body's source provenance.
    pub(crate) fn current_source_file(&self) -> Option<Rc<[u8]>> {
        self.script_stack
            .borrow()
            .last()
            .map(|f| Rc::from(f.as_slice()))
    }

    /// Evaluate a TclOO definition body. With `src = Some((file, line_base))`
    /// (the body was defined while sourcing a file) it runs in a `type source`
    /// frame, so its commands — and the method bodies they define — report
    /// file-absolute `info frame` lines (TIP 280). Otherwise it runs inline.
    pub(crate) fn eval_def_body(&mut self, body: &[u8], src: Option<(Rc<[u8]>, u32)>) -> Code {
        self.clear_return_options();
        match src {
            Some((file, line_base)) => {
                let mut frame = self.inherited_cmd_frame();
                frame.kind = FrameKind::Source;
                frame.file = Some(file);
                frame.line_base = line_base;
                frame.oo = None;
                self.eval_framed(body, frame)
            }
            None => self.eval_str(body),
        }
    }

    /// `uplevel`: evaluate `script` in the variable scope **and** namespace of
    /// frame `target_level` (restore caller ns + frame depth
    /// together), then restore. Transparent — the body's completion
    /// code (incl. `return`) propagates unchanged.
    pub(crate) fn eval_uplevel(&mut self, target_level: usize, script: &[u8]) -> Code {
        self.clear_return_options();
        let prev_level = self.frames.borrow_mut().set_active_level(target_level);
        let prev_ns = self.current_ns.get();
        self.current_ns
            .set(self.frames.borrow().frame_ns(target_level));
        // The `uplevel` body is a fresh dynamically-evaluated script: `type
        // eval`, **no** file, body-relative lines (base 0) — but it keeps the
        // invoking proc's name and runs at the target call level (with no
        // `level` key, the redirected scope). Matches tclsh, where `uplevel`'s
        // body is not inlined into the proc bytecode.
        let mut frame = self.inherited_cmd_frame();
        frame.kind = FrameKind::Eval;
        frame.file = None;
        frame.line_base = 0;
        frame.level = target_level;
        frame.omit_level = true;
        let code = self.eval_framed(script, frame);
        self.frames.borrow_mut().set_active_level(prev_level);
        self.current_ns.set(prev_ns);
        code
    }

    /// Shared `namespace eval` core: enter `name`, push a namespace var-scope
    /// frame *and* a `CmdFrame` for the body (C's `namespace eval` is its own
    /// `info frame` level — depth and `info level` both advance — with `proc`
    /// cleared and `level` reported relative to the new scope). `loc` is the
    /// body's TIP 280 location when it is a located literal (`type source`),
    /// else `None` (`type eval`).
    fn ns_eval_framed(
        &mut self,
        name: &[u8],
        body: &[u8],
        loc: Option<(Option<Rc<[u8]>>, u32)>,
        add_eval_frame: bool,
        original_body: Option<(
            tcl_registry::native_eval_object::EvalObjectPurpose,
            *mut TclObj,
        )>,
        original_arguments: Option<&[*mut TclObj]>,
    ) -> Code {
        self.clear_return_options();
        let target = if self
            .name_policy_protocol()
            .is_some_and(|protocol| protocol.recipe().is_jim084())
        {
            let original = crate::obj::Owned::fresh(crate::obj::new_string_bytes(name));
            let namespace = match self.jim_current_namespace_object() {
                Ok(namespace) => namespace,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            let canonical = match self.jim_canonical_namespace_object(&namespace, original.as_ptr())
            {
                Ok(canonical) => canonical,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            let bytes =
                match tcl_syntax::value::ValueOps::native_string_bytes(self, &canonical.as_ptr()) {
                    Ok(bytes) => bytes,
                    Err(error) => return self.report_cmd_error(error.into()),
                };
            self.namespaces_mut().retain_jim_namespace(canonical, bytes)
        } else {
            let dying_name = {
                let namespaces = self.namespaces.borrow();
                namespaces
                    .dying_namespace(self.current_ns.get(), name)
                    .map(|id| namespaces.qualified_name(id))
            };
            if let Some(dying_name) = dying_name {
                let mut message = b"can't create namespace \"".to_vec();
                message.extend_from_slice(&dying_name);
                message.extend_from_slice(b"\": already exists");
                return self.set_error(&message);
            }
            self.namespaces
                .borrow_mut()
                .ensure_namespace(self.current_ns.get(), name)
        };
        self.ns_eval_in_token(
            target,
            body,
            loc,
            add_eval_frame,
            original_body,
            original_arguments,
        )
    }

    pub(crate) fn ns_eval_in_token(
        &mut self,
        target: NsId,
        body: &[u8],
        loc: Option<(Option<Rc<[u8]>>, u32)>,
        add_eval_frame: bool,
        original_body: Option<(
            tcl_registry::native_eval_object::EvalObjectPurpose,
            *mut TclObj,
        )>,
        original_arguments: Option<&[*mut TclObj]>,
    ) -> Code {
        let saved = self.current_ns.get();
        self.current_ns.set(target);
        // A namespace frame: a new scope whose unqualified vars resolve to the
        // namespace (so `set`/`variable`/`upvar 0` inside `namespace eval` —
        // including when nested in a proc — target the namespace, not the
        // enclosing proc's locals).
        self.enter_namespace_activation(target);
        self.frames.borrow_mut().push_namespace(target);
        if let Some(arguments) = original_arguments {
            self.frames
                .borrow_mut()
                .install_original_error_stack_argv(arguments);
        }
        if let Err(error) = self.retain_native_jim_frame_namespace(target) {
            let popped = self.pop_native_call_frame();
            self.current_ns.set(saved);
            self.leave_namespace_activation(popped);
            return self.report_cmd_error(error.into());
        }
        let jim_body = if self.native_invocation_dialect().native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            Some(match original_body {
                Some((_, original)) => obj::Owned::retain(original),
                None => obj::Owned::fresh(new_string(body)),
            })
        } else {
            None
        };
        let jim_body_pointer = jim_body.as_ref().map(obj::Owned::as_ptr);
        if let Some(original) = jim_body {
            self.frames.borrow_mut().retain_jim_frame_body(original);
        }
        let (kind, file, line_base) = match loc {
            Some((file, line)) => (FrameKind::Source, file, line.saturating_sub(1)),
            None => (FrameKind::Eval, None, 0),
        };
        let (ns_level, ns_index) = {
            let f = self.frames.borrow();
            (f.current_level(), f.current_frame_index())
        };
        let frame = CmdFrame {
            kind,
            file,
            proc: None,
            level: ns_level,
            omit_level: false,
            frame_index: ns_index,
            line_base,
            proc_line_base: line_base,
            cmd: Vec::new(),
            original_command: None,
            line: 1,
            oo: None,
            lambda: None,
        };
        let code = match original_body {
            Some((purpose, original)) => self.eval_original_body_framed(purpose, original, frame),
            None => match jim_body_pointer {
                Some(original) => self.eval_original_body_framed(
                    tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceBody,
                    original,
                    frame,
                ),
                None => self.eval_framed(body, frame),
            },
        };
        if code == Code::Error && add_eval_frame {
            // `(in namespace eval "::ns" script line N)` — the body's own frame.
            let fqn = self.namespaces.borrow().qualified_name(target);
            self.append_namespace_eval_frame(&fqn);
        }
        self.clean_current_jim_local_commands();
        let popped = self.pop_native_call_frame();
        self.current_ns.set(saved);
        self.leave_namespace_activation(popped);
        code
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

    // variables (the var resolver; `crate::vars`)
    //
    // Every variable op routes through the one classification + link walk
    // (frame-local vs namespace, qualified vs not), instead of the old flat
    // per-frame table. Root and element inputs stay separate at the storage
    // door; complete names use the selected combined-name projection first.

    /// `set name` — borrowed value (the table keeps its +1), or `None`.
    pub(crate) fn var_get(&self, name: &[u8]) -> Option<*mut TclObj> {
        if self.observed_names.borrow().is_some() {
            let level = self.frames.borrow().current_level();
            return self.observed_variable_get(name, level).ok().flatten();
        }
        self.require_variable_name_protocol().ok()?;
        crate::vars::get(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            self.current_ns.get(),
            name,
        )
    }

    /// Read a complete byte-valued variable name, retaining its literal
    /// element key through trace dispatch, lookup and failure presentation.
    /// The returned value remains borrowed from the selected variable cell.
    pub(crate) fn read_named_variable(&mut self, name: &[u8]) -> Result<*mut TclObj, Code> {
        if self.observed_names.borrow().is_some() {
            let level = self.frames.borrow().current_level();
            return self
                .observed_variable_get(name, level)
                .map_err(|_| Code::Error)?
                .ok_or(Code::Error);
        }
        let input = self
            .combined_variable_input(name)
            .map_err(|_| Code::Error)?;
        let base = input.root().selected();
        let key = input.element().map(|element| element.selected());
        if let Some(code) = self.fire_read_trace(base, key) {
            return Err(code);
        }
        let value = match key {
            Some(key) => self.var_get_elem(base, key),
            None => self.var_get(base),
        };
        value.ok_or_else(|| self.no_such_variable(name, None))
    }

    /// Store through a complete fresh byte-valued name. Root-only storage
    /// callers continue to use `var_set`; this ingress owns element splitting.
    pub(crate) fn var_set_named(
        &mut self,
        name: &[u8],
        value: *mut TclObj,
    ) -> Result<(), VarError> {
        if self.observed_names.borrow().is_some() {
            let level = self.frames.borrow().current_level();
            return self.observed_variable_set(name, level, value);
        }
        let input = self.combined_variable_input(name)?;
        match input.element() {
            Some(element) => self.var_set_elem(input.root().selected(), element.selected(), value),
            None => self.var_set(input.root().selected(), value),
        }
    }

    // frame-addressed access (the `VarStore` `FrameId`-honouring path)
    //
    // Resolve `name` as if `level` were the active frame. Used only for a
    // non-active `FrameId`; the active frame keeps the by-name accessors above.

    /// Frame-addressed [`var_get`](Self::var_get).
    pub(crate) fn var_get_at(&self, name: &[u8], level: usize) -> Option<*mut TclObj> {
        if self.observed_names.borrow().is_some() {
            return self.observed_variable_get(name, level).ok().flatten();
        }
        self.require_variable_name_protocol().ok()?;
        crate::vars::get_at(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            name,
            level,
        )
    }

    /// Frame-addressed `set name(key)`. The base and element stay separate at
    /// the variable-store boundary, avoiding an ambiguous reconstructed name.
    pub(crate) fn var_get_elem_at(
        &self,
        name: &[u8],
        key: &[u8],
        level: usize,
    ) -> Option<*mut TclObj> {
        self.require_variable_name_protocol().ok()?;
        crate::vars::get_elem_at(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            name,
            key,
            level,
        )
    }

    /// Frame-addressed [`var_set`](Self::var_set) — the cell takes a **+1**.
    pub(crate) fn var_set_at(
        &mut self,
        name: &[u8],
        obj: *mut TclObj,
        level: usize,
    ) -> Result<(), VarError> {
        if self.store_escapes_at(name, level) {
            return Err(VarError::Confined);
        }
        if self.observed_names.borrow().is_some() {
            return self.observed_variable_set(name, level, obj);
        }
        self.require_variable_name_protocol()?;
        self.associate_native_jim_variable_value(obj)?;
        crate::vars::set_at(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            name,
            obj,
            level,
        )
    }

    /// Frame-addressed `set name(key) value`. The table takes its +1 on `obj`.
    pub(crate) fn var_set_elem_at(
        &mut self,
        name: &[u8],
        key: &[u8],
        obj: *mut TclObj,
        level: usize,
    ) -> Result<(), VarError> {
        if self.store_escapes_at(name, level) {
            return Err(VarError::Confined);
        }
        self.require_variable_name_protocol()?;
        self.associate_native_jim_variable_value(obj)?;
        crate::vars::set_elem_at(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            name,
            key,
            obj,
            level,
        )
    }

    /// Operational byte removal in a genuine addressed variable frame.
    pub(crate) fn var_unset_at(&mut self, name: &[u8], level: usize) -> bool {
        if self.observed_names.borrow().is_some() {
            return self.observed_variable_unset(name, level).unwrap_or(false);
        }
        if level == self.frames.borrow().current_level() {
            return self.var_unset(name);
        }
        let Ok(input) = self.combined_variable_input(name) else {
            return false;
        };
        let root = input.root().selected().to_vec();
        let element = input.element().map(|element| element.selected().to_vec());
        self.unset_byte_variable_at(name, &root, element.as_deref(), level, false)
    }

    /// Storage-only removal for the quiet bootstrap clear/install boundary.
    pub(crate) fn quiet_var_unset_at(&mut self, name: &[u8], level: usize) -> bool {
        if self.observed_names.borrow().is_some() {
            return self.observed_variable_unset(name, level).unwrap_or(false);
        }
        if self.require_variable_name_protocol().is_err() {
            return false;
        };
        crate::vars::unset_at(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            name,
            level,
        )
    }

    /// Frame-addressed `unset name(key)`.
    pub(crate) fn var_unset_elem_at(&mut self, name: &[u8], key: &[u8], level: usize) -> bool {
        if level == self.frames.borrow().current_level() {
            return self.var_unset_elem(name, key);
        }
        let Ok(input) = self.separate_variable_input(name, Some(key)) else {
            return false;
        };
        let root = input.root().selected().to_vec();
        let key = input
            .element()
            .expect("separate member")
            .selected()
            .to_vec();
        // Presentation only: the receiver retains separate original parts.
        let mut spelling = name.to_vec();
        spelling.push(b'(');
        spelling.extend_from_slice(&key);
        spelling.push(b')');
        self.unset_byte_variable_at(&spelling, &root, Some(&key), level, true)
    }

    /// Frame-addressed [`var_exists`](Self::var_exists).
    pub(crate) fn var_exists_at(&self, name: &[u8], level: usize) -> bool {
        if self.observed_names.borrow().is_some() {
            return self.observed_variable_exists(name, level).unwrap_or(false);
        }
        if self.require_variable_name_protocol().is_err() {
            return false;
        };
        crate::vars::exists_at(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            name,
            level,
        )
    }

    /// `set name(key)` — borrowed.
    pub(crate) fn var_get_elem(&self, name: &[u8], key: &[u8]) -> Option<*mut TclObj> {
        self.require_variable_name_protocol().ok()?;
        crate::vars::get_elem(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            self.current_ns.get(),
            name,
            key,
        )
    }

    /// `set name value` — the cell takes a **+1** on `obj`.
    pub(crate) fn var_set(&mut self, name: &[u8], obj: *mut TclObj) -> Result<(), VarError> {
        if self.store_escapes(name) {
            return Err(VarError::Confined);
        }
        if self.observed_names.borrow().is_some() {
            let level = self.frames.borrow().current_level();
            return self.observed_variable_set(name, level, obj);
        }
        self.require_variable_name_protocol()?;
        self.associate_native_jim_variable_value(obj)?;
        crate::vars::set(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
            obj,
        )?;
        if self.has_variable_traces() {
            let input = self.separate_variable_input(name, None)?;
            if self.fire_var_trace(input.root().selected(), None, b"write") {
                // A write trace errored: the value is set, but the command
                // fails (C's TclObjCallVarTraces). `var_error` wraps the
                // message from `pending_err` as `can't set "name": <msg>`.
                return Err(VarError::TraceError);
            }
        }
        Ok(())
    }

    /// `set name(key) value`.
    pub(crate) fn var_set_elem(
        &mut self,
        name: &[u8],
        key: &[u8],
        obj: *mut TclObj,
    ) -> Result<(), VarError> {
        if self.store_escapes(name) {
            return Err(VarError::Confined);
        }
        self.require_variable_name_protocol()?;
        self.associate_native_jim_variable_value(obj)?;
        crate::vars::set_elem(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
            key,
            obj,
        )?;
        let input = self.separate_variable_input(name, Some(key))?;
        if self.has_variable_traces()
            && self.fire_var_trace(
                input.root().selected(),
                input.element().map(|element| element.selected()),
                b"write",
            )
        {
            return Err(VarError::TraceError);
        }
        Ok(())
    }

    /// Store `obj` into the `(base, elem)` variable and, on success, publish it
    /// as the interp result — while holding a **protective reference** across the
    /// store. `var_set`/`var_set_elem` fire the write trace, and a trace that
    /// `unset`s the variable drops the store's reference; for a *fresh* `obj`
    /// (the value `lappend`/`append` just built) that was the only reference, so
    /// without this bracket the object is freed mid-command and the following
    /// `set_result` reads freed memory — a use-after-free that a write-traced
    /// `lappend`/`append` hits (append-7.x, var-traces). On a store error the
    /// bracket releases the reference (freeing a fresh `obj`, as the old
    /// `drop_fresh` did) and the error is returned for the caller to render.
    pub(crate) fn store_var_result(
        &mut self,
        base: &[u8],
        elem: Option<&[u8]>,
        obj: *mut TclObj,
    ) -> Result<(), VarError> {
        // SAFETY: `obj` is a live object the caller just built or read; the
        // increment/decrement bracket keeps it alive across the trace firing.
        unsafe { obj::incr_ref_count(obj) };
        let stored = match elem {
            Some(k) => self.var_set_elem(base, k, obj),
            None => self.var_set(base, obj),
        };
        if stored.is_ok() {
            // The result is the variable's value *after* the write trace ran, not
            // necessarily the value we stored: a trace may have rewritten the
            // variable (C returns the new value) or unset it (C returns empty).
            // `var_get*` are trace-free store reads, so this fires no read trace.
            let final_val = match elem {
                Some(k) => self.var_get_elem(base, k),
                None => self.var_get(base),
            };
            match final_val {
                Some(v) => self.set_result(v),
                None => self.set_result_bytes(b""),
            }
        }
        // SAFETY: balances the protective increment above (the store retained its
        // own reference on success; `set_result` retained the result's).
        unsafe { obj::decr_ref_count(obj) };
        stored
    }

    /// Flag the scalar `name` `const` (the `const` command, after its value is
    /// stored and its write traces have fired).
    pub(crate) fn mark_constant(&self, name: &[u8]) {
        if self.require_variable_name_protocol().is_err() {
            return;
        }
        crate::vars::mark_constant(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
        );
    }

    /// `Some(error)` — `can't set "name": variable is a constant` — when the
    /// (possibly `arr(idx)`) `name` targets a `const` scalar; `None` otherwise.
    /// The read-modify-write commands (`lappend`/`dict set`/`regsub`/`gets`)
    /// call this before mutating, since their in-place value update would
    /// otherwise bypass the store-time constant check.
    pub(crate) fn const_write_check(&mut self, name: &[u8]) -> Option<Code> {
        let (base, elem) = match self.variable_name_parts(name) {
            Ok(parts) => parts,
            Err(_) => return Some(Code::Error),
        };
        if self.store_escapes(&base) {
            return Some(self.confined_store_error(name));
        }
        if elem.is_none() && self.is_constant(&base) {
            let mut m = b"can't set \"".to_vec();
            m.extend_from_slice(name);
            m.extend_from_slice(b"\": variable is a constant");
            return Some(self.set_error(&m));
        }
        None
    }

    /// Confine every later store to the running procedure's own frame, and
    /// remove the globals the host seeded (`::env`, `::tcl_platform` and the
    /// library paths), so an evaluation reads nothing of the machine it runs
    /// on and leaves nothing behind for the next one to read.
    ///
    /// A store, an array's creation and an unset are refused, as a Tcl error
    /// raised before anything is written or removed, when they would land
    /// anywhere else: at the global level, in a namespace (a qualified name,
    /// `variable`, `namespace eval`), in another frame (`uplevel`), or through
    /// a link a local holds to a variable outside the frame (`global`,
    /// `upvar`). So is a draw from the `rand()` generator,
    /// whose seed every evaluation shares. Reads are unaffected, and an error
    /// is not published to `::errorInfo` and `::errorCode`, which are globals.
    pub fn confine_stores(&mut self) {
        self.stores_confined.set(true);
        for name in tcl_platform::bootstrap::HOST_ARRAYS
            .iter()
            .chain(tcl_platform::bootstrap::HOST_PATH_GLOBALS)
        {
            self.var_unset_at(format!("::{name}").as_bytes(), 0);
        }
    }

    /// Whether stores are confined ([`Self::confine_stores`]).
    #[must_use]
    pub fn stores_confined(&self) -> bool {
        self.stores_confined.get()
    }

    /// Whether a store to `name` from the current frame must be refused:
    /// stores are confined and it would land outside the running procedure's
    /// own frame.
    pub(crate) fn store_escapes(&self, name: &[u8]) -> bool {
        self.stores_confined.get()
            && !crate::vars::lands_in_own_frame(
                &self.frames.borrow(),
                &self.namespaces.borrow(),
                self.current_ns.get(),
                name,
            )
    }

    /// [`Self::store_escapes`] for a store resolved as if `level` were active.
    pub(crate) fn store_escapes_at(&self, name: &[u8], level: usize) -> bool {
        self.stores_confined.get()
            && !crate::vars::lands_in_own_frame_at(
                &self.frames.borrow(),
                &self.namespaces.borrow(),
                name,
                level,
            )
    }

    /// The error a confined store to `name` is refused with — the bytecode
    /// VM's words and code.
    pub(crate) fn confined_store_error(&mut self, name: &[u8]) -> Code {
        let mut message = b"can't set \"".to_vec();
        message.extend_from_slice(name);
        message.extend_from_slice(b"\": stores are confined to the activation");
        self.error_with_code(&message, b"TCL WRITE VARNAME")
    }

    /// The error a confined unset of `name` is refused with — the bytecode
    /// VM's words and code.
    pub(crate) fn confined_unset_error(&mut self, name: &[u8]) -> Code {
        let mut message = b"can't unset \"".to_vec();
        message.extend_from_slice(name);
        message.extend_from_slice(b"\": stores are confined to the activation");
        self.error_with_code(&message, b"TCL UNSET VARNAME")
    }

    /// The error a draw from the `rand()` generator is refused with while
    /// stores are confined, or `None` when they are not. The math functions
    /// need the numeric tower, so without it there is no generator to refuse.
    #[cfg(have_tommath)]
    pub(crate) fn confined_generator_error(&mut self, function: &[u8]) -> Option<Code> {
        if !self.stores_confined.get() {
            return None;
        }
        let mut message = b"can't call \"".to_vec();
        message.extend_from_slice(function);
        message.extend_from_slice(
            b"\": stores are confined to the activation and the generator's seed is not",
        );
        Some(self.error(&message))
    }

    /// Delete every command, in every namespace, whose unrooted name (`set`,
    /// `tcl::mathfunc::abs`) `keep` refuses: the whitelist an engine confines
    /// a body to. A command `keep` names keeps its own binding; nothing is
    /// renamed or replaced.
    pub fn retain_commands(&mut self, keep: &dyn Fn(&str) -> bool) {
        self.retain_command_tokens(&|_, report| core::str::from_utf8(report).is_ok_and(keep));
    }

    /// Keep exact live command generations selected by their original table owners.
    /// Reporting bytes are provided for a caller's byte whitelist; they never
    /// supply the generation used for deletion.
    pub(crate) fn retain_command_tokens(&mut self, keep: &dyn Fn(u64, &[u8]) -> bool) {
        let refused: Vec<_> = {
            let namespaces = self.namespaces.borrow();
            namespaces
                .native_command_generations()
                .into_iter()
                .filter(|&generation| {
                    namespaces
                        .native_command_slot_at_node(generation)
                        .is_some_and(|(ns, simple)| {
                            let report = namespaces.command_fqn_at(ns, &simple);
                            !keep(generation, report.strip_prefix(b"::").unwrap_or(&report))
                        })
                })
                .collect()
        };
        for generation in refused {
            self.delete_command_generation(generation);
        }
    }

    /// Whether `name` resolves to a `const` scalar.
    pub(crate) fn is_constant(&self, name: &[u8]) -> bool {
        if self.require_variable_name_protocol().is_err() {
            return false;
        };
        crate::vars::is_constant(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            self.current_ns.get(),
            name,
        )
    }

    /// Whether `name`, resolved as if `level` were active, is a `const` cell.
    pub(crate) fn is_constant_at(&self, name: &[u8], level: usize) -> bool {
        if self.require_variable_name_protocol().is_err() {
            return false;
        };
        crate::vars::is_constant_at(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            name,
            level,
        )
    }

    /// Ensure `name` is an array (creating an empty one if unset) — backs
    /// `array set name {}` with an empty value list. A scalar `name` errors.
    pub(crate) fn ensure_array(&self, name: &[u8]) -> Result<(), VarError> {
        if self.observed_names.borrow().is_some() {
            let level = self.frames.borrow().current_level();
            return self.observed_ensure_array(name, level);
        }
        self.require_variable_name_protocol()?;
        crate::vars::ensure_array(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
        )
    }

    /// Materialise an unset variable cell for `trace add variable` without
    /// firing write traces or making `info exists` true.
    pub(crate) fn ensure_trace_variable(&self, name: &[u8]) -> Result<(), VarError> {
        self.require_variable_name_protocol()?;
        crate::vars::ensure_undefined(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
        )
    }

    pub(crate) fn ensure_trace_element(&self, name: &[u8], key: &[u8]) -> Result<(), VarError> {
        self.require_variable_name_protocol()?;
        crate::vars::ensure_trace_element(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
            key,
        )
    }

    pub(crate) fn cleanup_trace_shell(&self, home: &crate::vars::TraceHome, key: Option<&[u8]>) {
        if key.is_none()
            && (self.native_error_variable_at(home).is_some()
                || self.native_precision_trace_at(home))
        {
            return;
        }
        crate::vars::cleanup_trace_shell(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            home,
            key,
        );
    }

    /// The call-frame level a variable trace on `base` should be tied to, so it
    /// dies with the frame (C frees a local var's trace list at frame teardown).
    /// `Some(level)` for an unqualified name resolving frame-local in a proc;
    /// `None` (persistent) for qualified / global / `global`-or-`upvar`-linked
    /// names, which outlive the frame.
    /// The home namespace a variable trace on `base` is scoped to — `Some(ns)`
    /// for a trace on a namespace variable (registered at namespace/global scope,
    /// or a qualified name), so it fires only for that namespace's variable and
    /// dies with the namespace; `None` for a proc-local trace, which matches by
    /// raw name. Used at both trace-add and trace-fire time so they agree.
    /// The `(home namespace, home frame level, simple name)` identity a variable
    /// trace on `base` belongs to.
    ///
    /// Registration and firing both key on this, so every spelling that
    /// resolves to the one cell shares one trace list — including an `upvar`
    /// alias, whose level can only be read off the *resolved* place. C gets
    /// this for free: there the alias and its target are the same `Var`, and
    /// the trace list hangs off that `Var`.
    pub(crate) fn trace_identity(&self, base: &[u8]) -> crate::vars::TraceHome {
        let _ = self.require_variable_name_protocol();

        crate::vars::trace_home(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            self.current_ns.get(),
            base,
        )
    }

    /// Shared reentrancy identity of the actual selected root and member allocations.
    pub(super) fn variable_trace_scope(
        &self,
        home: &crate::vars::TraceHome,
        element: Option<&[u8]>,
    ) -> crate::cmd_trace::VarTraceScope {
        let member = element.and_then(|element| {
            crate::vars::trace_element_identity(
                &self.frames.borrow(),
                &self.namespaces.borrow(),
                home,
                element,
            )
        });
        crate::cmd_trace::VarTraceScope::cell(home, element, member)
    }

    /// Whether this release recovers an array element from the resolved `Var`
    /// when the access spelling names none — the release axis itself lives in
    /// `tcl-dialect`, beside `namespace_var_global_fallback`. The two visible
    /// consequences are pinned in `tests/trace_semantics.rs`:
    ///
    /// - `upvar #0 a(k) e; set e 5` fires the array's traces *and* the
    ///   element's with `name2 = k` at 9.0; at 8.6 only the element's own, with
    ///   an empty `name2`.
    /// - `unset a(k)` reports `name1 = a(k)` at 9.0 (the recovered `part2`
    ///   stops `TclCallVarTraces` re-splitting the name) and `name1 = a` at
    ///   8.6.
    fn traces_recover_the_linked_element(&self) -> bool {
        self.runtime_version().traces_recover_linked_array_element()
    }

    /// The unset-trace callbacks a proc frame's locals contribute as the frame
    /// is torn down — C's `TclDeleteVars`, which runs `UnsetVarStruct` (and,
    /// for an array local, `DeleteArray`) over every variable in the frame.
    ///
    /// Read while the frame is still on the stack: after the pop its variables
    /// are gone, and an array local's elements with them. Each variable's own
    /// callbacks fire newest-first and contiguously; *which* variable comes
    /// first follows the frame's retained compiled declarations and native
    /// physical hash entries, including undefined entries carrying traces.
    fn frame_teardown_unset_traces(&self, level: usize) -> Vec<VarTeardownCallback> {
        if self
            .traces
            .borrow()
            .traces
            .iter()
            .all(|t| t.frame_level != Some(level))
        {
            return Vec::new();
        }
        let names: Vec<Vec<u8>> = match self.frames.borrow().table(level) {
            Some(table) => table
                .teardown_names()
                .into_iter()
                .map(<[u8]>::to_vec)
                .collect(),
            None => return Vec::new(),
        };
        let mut victims = Vec::new();
        for name in names {
            let home = crate::vars::TraceHome {
                binding_id: self
                    .frames
                    .borrow()
                    .table(level)
                    .and_then(|table| table.binding_id(&name)),
                selected_member: None,
                ns: None,
                level: Some(level),
                base: name.clone(),
                link_elem: None,
            };
            victims.extend(self.cell_unset_traces(&home, None, &name, b""));
            let elements = self
                .frames
                .borrow()
                .table(level)
                .and_then(|table| table.capture_array_cell(&name))
                .and_then(|array| array.search_keys())
                .unwrap_or_default();
            for elem in elements {
                victims.extend(self.cell_unset_traces(&home, Some(&elem), &name, &elem));
            }
        }
        victims
    }

    /// Drop every variable trace tied to call-frame `level` (the frame is being
    /// popped; its local variables and their traces go away).
    pub(crate) fn clear_frame_var_traces(&self, level: usize) {
        let mut t = self.traces.borrow_mut();
        let removed = t.traces.iter().any(|v| v.frame_level == Some(level));
        if removed {
            t.traces.retain(|v| v.frame_level != Some(level));
        }
        drop(t);
        if removed {
            self.invalidate_guard_domain(GuardDomain::VariableTrace);
        }
    }

    /// Fire a read trace for `name` before a read (the `&mut` chokepoints that
    /// resolve `$var` call this). Returns `Some(Code::Error)` — with the interp
    /// result set to `can't read "name": <msg>` — if a read trace callback
    /// errored (C's `TclObjCallVarTraces` propagation); else `None`.
    pub(crate) fn fire_read_trace(&mut self, name: &[u8], key: Option<&[u8]>) -> Option<Code> {
        if self.observed_name_policy_selected() {
            return self
                .observed_substitution_receiver(name, key)
                .err()
                .map(|_| Code::Error);
        }
        let input = match key {
            Some(key) => self.separate_variable_input(name, Some(key)),
            None => self.combined_variable_input(name),
        };
        let Ok(input) = input else {
            return Some(Code::Error);
        };
        if !self.has_variable_traces() {
            return None;
        }
        let base = input.root().selected().to_vec();
        let key = input.element().map(|element| element.selected());
        if !self.fire_var_trace(&base, key, b"read") {
            return None;
        }
        let msg = self
            .traces
            .borrow_mut()
            .pending_err
            .take()
            .unwrap_or_default();
        // Display name: `base` or `base(key)`.
        let mut display = base.clone();
        if let Some(k) = key {
            display.push(b'(');
            display.extend_from_slice(k);
            display.push(b')');
        }
        Some(self.var_trace_error(&display, b"read", &msg))
    }

    /// Read a variable's current value for a **read-modify-write** command
    /// (`lappend`, `incr`), firing its read trace but **swallowing** any error
    /// the trace raises — the value simply reads as absent.
    ///
    /// This is C's `TclPtrGetVarIdx` seen from a caller that treats a `NULL`
    /// return as "no current value" rather than as a failure:
    /// `Tcl_LappendObjCmd` creates the element instead (bug 3057639,
    /// append-7.2/7.3/9.0) and `TclPtrIncrObjVar` substitutes 0. Both are
    /// oracle-pinned in `tests/trace_semantics.rs`: with an erroring read trace
    /// on `x`, tclsh 8.6.16 and 9.0.4 both leave `incr x` succeeding with 1.
    ///
    /// `set`/`append`-read, by contrast, propagate the error via
    /// [`fire_read_trace`](Self::fire_read_trace).
    pub(crate) fn read_for_update(
        &mut self,
        base: &[u8],
        elem: Option<&[u8]>,
    ) -> Option<*mut TclObj> {
        if self.has_variable_traces() && self.fire_var_trace(base, elem, b"read") {
            // The read trace errored: discard it and treat the value as absent.
            self.traces.borrow_mut().pending_err.take();
            return None;
        }
        match elem {
            Some(k) => self.var_get_elem(base, k),
            None => self.var_get(base),
        }
    }

    /// `lappend`'s name for [`read_for_update`](Self::read_for_update).
    pub(crate) fn lappend_read(&mut self, base: &[u8], elem: Option<&[u8]>) -> Option<*mut TclObj> {
        self.read_for_update(base, elem)
    }

    /// `unset name` — returns whether it existed.
    pub(crate) fn var_unset(&mut self, name: &[u8]) -> bool {
        if self.observed_name_policy_selected() {
            return self
                .observed_variable_unset(name, self.frames.borrow().current_level())
                .unwrap_or(false);
        }
        // Resolve the trace key BEFORE removing the variable. Resolution can
        // depend on the cell still existing — the 8.x namespace-scope fallback
        // only reaches the global when the global cell is present — so
        // re-resolving after the removal would silently pick a different
        // variable and the unset trace would never fire. C resolves the
        // `Var`, fires its traces, and only then frees it.
        let Ok(input) = self.combined_variable_input(name) else {
            return false;
        };
        let base = input.root().selected().to_vec();
        if let Some(element) = input.element() {
            return self.var_unset_elem(&base, element.selected());
        }
        let traced = self.has_variable_traces();
        let key = traced.then(|| self.trace_identity(&base));
        if self.native_invocation_dialect().variable_destruction_protocol(true)
                == Some(tcl_runtime_api::variable_destruction::VariableDestructionProtocol::ArrayLookupThenRootCallbacksThenMembers)
        {
            let selected = crate::vars::begin_array_destruction(
                &mut self.frames.borrow_mut(), &mut self.namespaces.borrow_mut(),
                self.current_ns.get(), &base,
            );
            if let Some(selected) = selected {
                return self.finish_array_unset(name, &base, key, selected);
            }
        }
        let existed = crate::vars::unset(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            &base,
        );
        if let Some(home) = key
            .as_ref()
            .filter(|home| !existed && self.native_error_variable_at(home).is_some())
        {
            let access = self.trace_access(name, &base, None, home, true);
            self.fire_native_error_variable_trace(home, &access, b"unset");
        }
        if let (true, Some(home)) = (existed, key) {
            self.fire_selected_unset_callbacks(&home, &base, None, name, false);
        }
        existed
    }

    fn finish_array_unset(
        &mut self,
        name: &[u8],
        base: &[u8],
        home: Option<crate::vars::TraceHome>,
        selected: crate::frame::RetainedArrayCell,
    ) -> bool {
        let native_error = home
            .as_ref()
            .and_then(|home| self.native_error_variable_at(home));
        let native_precision = home
            .as_ref()
            .is_some_and(|home| self.native_precision_trace_at(home));
        // naming.variable.original-undefined-array-member-retirement
        // docs/design/analysis/name-resolution-proofs/variable-original-undefined-array-member-retirement.md
        // DeleteArray visits original physical entries, including traced
        // undefined members. Defined-value enumeration has a separate purpose.
        let elements = selected.search_keys().unwrap_or_default();
        if let Some(home) = home.as_ref() {
            let access = self.trace_access(name, base, None, home, true);
            let root = self.cell_unset_traces(home, None, &access.reported, b"");
            self.detach_destroyed_trace_group(home, None);
            self.fire_unset_callbacks(root);
            for element in &elements {
                // Preceding callbacks may change this original member's list.
                // Its retained allocation excludes a replacement generation.
                let Some(member_home) = home.for_selected_array_member(&selected, element) else {
                    continue;
                };
                let callbacks =
                    self.cell_unset_traces(&member_home, Some(element), &access.reported, element);
                let registrations = self.destroyed_trace_group_ids(&member_home, Some(element));
                self.detach_destroyed_trace_ids(&registrations);
                let preserve_definition =
                    self.native_c_variable_name_protocol()
                        .is_some_and(|protocol| {
                            protocol.element_unset_preserves_definition_during_trace()
                        });
                let trace = selected.begin_member_retirement(
                    element,
                    preserve_definition,
                    !callbacks.is_empty(),
                );
                selected.retire_member(element);
                self.fire_unset_callbacks(callbacks);
                drop(trace);
                let object_table = self
                    .native_c_variable_name_protocol()
                    .is_some_and(|protocol| protocol.element_table_retains_original());
                selected.finish_member_retirement(element, object_table);
            }
        }
        selected.finish_destruction();
        if native_precision
            && home
                .as_ref()
                .is_some_and(|home| self.native_precision_trace_at(home))
        {
            self.install_native_precision_trace_at(name);
        }
        if let Some(variable) = native_error {
            self.install_native_error_variable_trace(variable);
        }
        true
    }

    fn destroyed_trace_group_ids(
        &self,
        home: &crate::vars::TraceHome,
        element: Option<&[u8]>,
    ) -> Vec<u64> {
        let cell = self.variable_trace_scope(home, element);
        self.traces
            .borrow()
            .traces
            .iter()
            .filter(|trace| cell.owns_registration(trace))
            .map(|trace| trace.id)
            .collect()
    }

    fn detach_destroyed_trace_ids(&mut self, registrations: &[u64]) {
        self.traces
            .borrow_mut()
            .traces
            .retain(|trace| !registrations.contains(&trace.id));
        self.invalidate_guard_domain(GuardDomain::VariableTrace);
    }

    fn detach_destroyed_trace_group(
        &mut self,
        home: &crate::vars::TraceHome,
        element: Option<&[u8]>,
    ) {
        let cell = self.variable_trace_scope(home, element);
        self.traces
            .borrow_mut()
            .traces
            .retain(|trace| !cell.owns_registration(trace));
        self.invalidate_guard_domain(GuardDomain::VariableTrace);
    }

    /// `unset name(key)` — returns whether it existed.
    pub(crate) fn var_unset_elem(&mut self, name: &[u8], key: &[u8]) -> bool {
        if self.require_variable_name_protocol().is_err() {
            return false;
        };
        // Resolved before the removal — see [`Self::var_unset`].
        let trace_key = (self.has_variable_traces()).then(|| {
            let mut home = self.trace_identity(name);
            home.selected_member = crate::vars::trace_element_identity(
                &self.frames.borrow(),
                &self.namespaces.borrow(),
                &home,
                key,
            )
            .map(|identity| (key.to_vec(), identity));
            home
        });
        let existed = crate::vars::unset_elem(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
            key,
        );
        if let (true, Some(home)) = (existed, trace_key) {
            let mut spelling = name.to_vec();
            spelling.push(b'(');
            spelling.extend_from_slice(key);
            spelling.push(b')');
            self.fire_selected_unset_callbacks(&home, name, Some(key), &spelling, false);
        }
        existed
    }

    /// Invoke every variable trace matching `(base, elem, op)`, as
    /// `command base element op`. A running callback suppresses nested traces
    /// on the same resolved variable cell, while traces on unrelated cells
    /// remain active. The interp result is preserved across the callbacks (the
    /// triggering operation owns the result). For `read`/`write` ops a callback
    /// error is **propagated**: the message is stashed in `pending_err` and the
    /// function returns `true` (the access then fails; C's `TclCallVarTraces`).
    /// `unset`/`array` errors are ignored (C does too). Returns whether a
    /// read/write callback errored.
    fn fire_var_trace(&mut self, base: &[u8], elem: Option<&[u8]>, op: &[u8]) -> bool {
        // Resolve the access to the same identity registration used, so a trace
        // matches every spelling of the variable it is on (`::v` vs `v`, an
        // `upvar` alias, and the 8.x namespace-scope fallback).
        let home = self.trace_identity(base);
        let access = self.trace_access(base, base, elem, &home, false);
        self.fire_var_trace_resolved(&home, &access, op)
    }

    /// How one access presents itself to the trace machinery: which cell's
    /// element the traces match, what the callback is told, and whether the
    /// containing array's whole-array traces take part.
    ///
    /// `spelling` is the access as written (`a(k)`), `base`/`elem` its split
    /// halves, and `home` the resolved cell. Three shapes:
    ///
    /// - an explicit element (`set a(k) 2`) matches and reports that element,
    ///   and reports `name1 = a` — C's `TclCallVarTraces` splits `part1` at the
    ///   `(` when the caller left `part2` NULL. The `unset` path is where 9.0
    ///   differs: it recovers `part2` from the `Var` *before* the call, so the
    ///   split never runs and `name1` stays the whole `a(k)`.
    /// - a link into an element (`upvar #0 a(k) e`) matches that element. At
    ///   9.0 it reports it too, and the array's traces fire; at 8.4-8.6
    ///   `part2` stays NULL, so `name2` is empty and only the element's own
    ///   traces run. See [`traces_recover_the_linked_element`](
    ///   Self::traces_recover_the_linked_element).
    /// - anything else is a plain scalar/whole-array access.
    fn trace_access(
        &self,
        spelling: &[u8],
        base: &[u8],
        elem: Option<&[u8]>,
        home: &crate::vars::TraceHome,
        unset: bool,
    ) -> TraceAccess {
        let recover = self.traces_recover_the_linked_element();
        match (elem, home.link_elem.as_deref()) {
            (Some(e), _) => TraceAccess {
                reported: if unset && recover { spelling } else { base }.to_vec(),
                match_elem: Some(e.to_vec()),
                report_elem: Some(e.to_vec()),
                spelling_elem: Some(e.to_vec()),
                whole_array: true,
            },
            (None, Some(k)) => TraceAccess {
                reported: base.to_vec(),
                match_elem: Some(k.to_vec()),
                report_elem: recover.then(|| k.to_vec()),
                spelling_elem: None,
                whole_array: recover,
            },
            (None, None) => TraceAccess {
                reported: base.to_vec(),
                match_elem: None,
                report_elem: None,
                spelling_elem: None,
                whole_array: true,
            },
        }
    }

    /// Native precision is a hidden C variable trace and is not reported by
    /// script-level `trace info`. Its registration follows resolved cell homes.
    fn has_variable_traces(&self) -> bool {
        !self.native_error_cells.borrow().is_empty()
            || !self.traces.borrow().traces.is_empty()
            || self.native_precision_cell.get().is_some()
    }

    /// [`Self::fire_var_trace`] with the identity already resolved — for
    /// `unset`, which must resolve *before* it removes the variable (resolution
    /// can depend on the cell existing).
    fn fire_var_trace_resolved(
        &mut self,
        home: &crate::vars::TraceHome,
        access: &TraceAccess,
        op: &[u8],
    ) -> bool {
        self.fire_var_trace_resolved_with_errors(home, access, op, true)
    }

    fn fire_var_trace_resolved_with_errors(
        &mut self,
        home: &crate::vars::TraceHome,
        access: &TraceAccess,
        op: &[u8],
        leave_error_message: bool,
    ) -> bool {
        if self.host_refusal_pending() {
            return false;
        }
        let elem = access.match_elem.as_deref();
        let reported = access.reported.as_slice();
        // The cell this access reaches, and the array cell containing it — C's
        // `varPtr` and `arrayPtr`, each with its own `VAR_TRACE_ACTIVE`.
        let cell = self.variable_trace_scope(home, elem);
        let traces = self.traces.borrow();
        // "If there are already similar trace functions active for the
        // variable, don't call them again" — C's early return on
        // `TclIsVarTraceActive(varPtr)` (tclTrace.c 9.0.4:2513). Per *cell*: a
        // callback writing a different element of the same array is a different
        // `Var` and fires.
        if self.active_var_trace_scopes.borrow().contains(&cell) {
            return false;
        }
        let array_active = elem.is_some()
            && self
                .active_var_trace_scopes
                .borrow()
                .contains(&cell.array());
        let any = traces.traces.iter().any(|trace| {
            let whole = trace.elem.is_none();
            (!whole || (access.whole_array && !array_active)) && cell.matches(trace, op)
        });
        drop(traces);
        if !any {
            self.fire_native_error_variable_trace(home, access, op);
            return self.native_precision_trace(home, access, op);
        }
        // C aborts the chain on the first callback error for every op *except*
        // unset — "ignore errors in unset traces" (tclTrace.c 9.0.4:2600). An
        // `array` trace's error therefore fails the `array` subcommand.
        let propagate = op != b"unset";
        // Preserve the result object across the callbacks.
        let saved = self.save_native_variable_trace_result(false);
        let global_error_flags = self.capture_native_global_error_flags();

        // The cell is marked active for the whole firing, as C marks `varPtr`
        // once on entry and clears it on the way out — not per callback.
        self.active_var_trace_scopes.borrow_mut().push(cell.clone());

        let mut errored = false;
        let mut callback_failure: Option<obj::Owned> = None;
        let op_name = String::from_utf8_lossy(op).into_owned();
        'groups: for whole_array in [true, false] {
            if whole_array && (!access.whole_array || array_active) {
                continue;
            }
            // Resolve each group only when Tcl reaches it. A whole-array
            // callback can therefore add or remove the selected element's
            // trace before the element group begins.
            let order: Vec<u64> = self
                .traces
                .borrow()
                .traces
                .iter()
                .rev()
                .filter(|trace| trace.elem.is_none() == whole_array)
                .filter(|trace| cell.matches(trace, op))
                .map(|trace| trace.id)
                .collect();
            for id in order {
                // Still registered? A previous callback in this group may
                // have removed it from C's live linked-list walk.
                let Some((cmd, old_style)) = self
                    .traces
                    .borrow()
                    .traces
                    .iter()
                    .find(|trace| trace.id == id)
                    .map(|trace| {
                        (
                            native_variable_observers::Callback::from_trace(trace),
                            trace.old_style,
                        )
                    })
                else {
                    continue;
                };
                let code = match cmd {
                    native_variable_observers::Callback::Native(observer) => self
                        .call_native_variable_observer(
                            observer,
                            reported,
                            access.report_elem.as_deref().unwrap_or(b""),
                            &op_name,
                        ),
                    native_variable_observers::Callback::Script(cmd) => {
                        let op_word = tcl_cmd_core::trace::callback_op_word(&op_name, old_style);
                        let args = self.new_list_object(&[
                            new_string(reported),
                            new_string(access.report_elem.as_deref().unwrap_or(b"")),
                            new_string(op_word.as_bytes()),
                        ]);
                        let mut line = cmd;
                        line.push(b' ');
                        line.extend_from_slice(&obj_bytes(args));
                        drop_fresh(args);
                        self.reset_native_error_objects_before_trace_script();
                        self.clear_return_options();
                        let saved = self.save_native_variable_trace_result(true);
                        let code = match self
                            .native_invocation_dialect()
                            .native_variable_trace_protocol()
                        {
                            Some(protocol) => {
                                self.eval_str(protocol.variable_callback_source(&line))
                            }
                            None => self.report_cmd_error(
                                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                    "variable trace callback source",
                                )
                                .into(),
                            ),
                        };
                        if code != Code::Ok
                            && self
                                .native_invocation_dialect()
                                .native_variable_trace_protocol()
                                .is_some()
                        {
                            callback_failure = Some(obj::Owned::retain(self.result.get()));
                        }
                        if let Some(saved) = saved {
                            self.restore_native_variable_trace_result(saved);
                        }
                        code
                    }
                };
                if propagate && code == Code::Error {
                    // Capture the callback's error message; stop firing (C
                    // aborts the trace chain on the first error).
                    let msg = callback_failure.as_ref().map_or_else(
                        || self.result_bytes(),
                        |failure| obj_bytes(failure.as_ptr()),
                    );
                    self.traces.borrow_mut().pending_err = Some(msg);
                    if leave_error_message {
                        let mut frame = op.to_vec();
                        frame.extend_from_slice(b" trace on \"");
                        frame.extend_from_slice(reported);
                        if let Some(k) = access.spelling_elem.as_deref() {
                            frame.push(b'(');
                            frame.extend_from_slice(k);
                            frame.push(b')');
                        }
                        frame.push(b'"');
                        self.append_frame_noline(&frame);
                    }
                    errored = true;
                    break 'groups;
                }
                drop(callback_failure.take());
            }
            // The intrinsic was installed before user registrations: it is
            // last in the whole-variable group, before any element traces.
            if whole_array {
                self.fire_native_error_variable_trace(home, access, op);
            }
            if whole_array && self.native_precision_trace(home, access, op) {
                errored = true;
                break;
            }
        }
        let popped = self.active_var_trace_scopes.borrow_mut().pop();
        debug_assert_eq!(popped, Some(cell));
        if !errored {
            self.restore_native_global_error_flags(global_error_flags);
        }
        // Restore the saved result (release the trace's, adopt our held +1).
        if !errored || !leave_error_message {
            if let Some(saved) = saved {
                self.restore_native_variable_trace_result(saved);
            }
        }
        errored
    }

    /// C's `TclVarErrMsg` tail after a variable trace aborted an access: the
    /// *result* becomes `can't <verb> "<name>": <reason>` and `-errorcode`
    /// becomes `TCL <READ|WRITE> VARNAME` (`tclVar.c` 9.0.4:1472 / :2073),
    /// while `errorInfo` keeps the chain `TclCallVarTraces` already built — the
    /// callback's own trace plus its `(<type> trace on "…")` frame.
    ///
    /// This is why it is not `set_error`: that starts a *fresh* error and would
    /// throw the callback's trace away, leaving `errorInfo` as the bare
    /// `can't set "x": …` line.
    pub(crate) fn var_trace_error(&mut self, name: &[u8], op: &[u8], reason: &[u8]) -> Code {
        // C's `TclCallVarTraces` verb table (tclTrace.c 9.0.4:2668-2681). The
        // `-errorcode` is *not* set there but by the access that failed, and
        // only `TclPtrGetVarIdx`/`TclPtrSetVarIdx` do so — `TclCheckArrayTraces`
        // has no such tail, so an `array` trace error keeps whatever
        // `-errorcode` the callback left (tclsh: `NONE` for a bare `error`).
        let (verb, word): (&[u8], Option<&[u8]>) = match op {
            b"read" => (b"read", Some(b"READ")),
            b"array" => (b"trace array", None),
            _ => (b"set", Some(b"WRITE")),
        };
        let mut msg = b"can't ".to_vec();
        msg.extend_from_slice(verb);
        msg.extend_from_slice(b" \"");
        msg.extend_from_slice(name);
        msg.extend_from_slice(b"\": ");
        msg.extend_from_slice(reason);
        self.set_result_bytes(&msg);
        if let Some(word) = word {
            let mut code = b"TCL ".to_vec();
            code.extend_from_slice(word);
            code.extend_from_slice(b" VARNAME");
            self.replace_native_error_code(&code);
            let mut exc = self.exc.borrow_mut();
            exc.code = code;
            exc.code_explicit = false;
        }
        Code::Error
    }

    /// Locate one array binding, retain its opaque identity across the array
    /// operation trace, and expose it to the shared command core.
    pub(crate) fn with_array_trace_target(
        &mut self,
        name: &[u8],
        operation: impl FnOnce(&mut Self, &tcl_runtime_api::ArrayTarget) -> Code,
    ) -> Code {
        if self.observed_name_policy_selected() {
            let level = self.frames.borrow().current_level();
            return match self.observed_array_target(name, level) {
                Ok(target) => operation(self, &target),
                Err(error) => self.report_cmd_error(error.into()),
            };
        }
        if self.require_variable_name_protocol().is_err() {
            return Code::Error;
        }

        let frame = tcl_runtime_api::FrameId(self.frames.borrow().current_level());
        let located = crate::vars::array_target_at(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            name,
            frame.0,
        );
        let target = located.as_ref().map_or_else(
            || tcl_runtime_api::ArrayTarget::named_bytes(frame, name),
            |record| tcl_runtime_api::ArrayTarget::cell_bytes(frame, name, record.id()),
        );
        if let Some(record) = located {
            let retained = crate::vars::retain_array_target(
                &mut self.frames.borrow_mut(),
                &mut self.namespaces.borrow_mut(),
                &record,
            );
            debug_assert!(retained);
            self.array_operation_targets
                .borrow_mut()
                .push(ArrayOperationTarget { target: record });
        }
        let result = match self.fire_array_trace(name) {
            Some(code) => code,
            None => operation(self, &target),
        };
        if let Some(id) = target.cell_id() {
            let popped = self.array_operation_targets.borrow_mut().pop();
            debug_assert_eq!(popped.as_ref().map(|record| record.target.id()), Some(id));
            if let Some(record) = popped {
                crate::vars::release_array_target(
                    &mut self.frames.borrow_mut(),
                    &mut self.namespaces.borrow_mut(),
                    &record.target,
                );
            }
        }
        result
    }

    pub(crate) fn array_operation_target(
        &self,
        target: &tcl_runtime_api::ArrayTarget,
    ) -> Option<crate::vars::ArrayCellTarget> {
        let id = target.cell_id()?;
        self.array_operation_targets
            .borrow()
            .iter()
            .rev()
            .find(|record| record.target.id() == id)
            .map(|record| record.target.clone())
    }

    pub(crate) fn array_keys_at_target(
        &self,
        target: &tcl_runtime_api::ArrayTarget,
    ) -> Option<Vec<Vec<u8>>> {
        let record = self.array_operation_target(target)?;
        crate::vars::array_names_at_target(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            &record,
        )
    }

    pub(crate) fn name_policy_protocol(&self) -> Option<tcl_syntax::naming::NamePolicyProtocol> {
        let dialect = self.native_invocation_dialect();
        if let Some(recipe) = dialect.native_name_protocol() {
            let protocol = tcl_syntax::naming::NamePolicyProtocol::for_native_point(
                dialect.execution_point()?,
            )?;
            return (protocol.recipe() == recipe).then_some(protocol);
        }
        dialect.authored_logical_name_simulation(self.logical_name_provider.get()?)
    }

    pub(crate) fn array_search_keys_at_target(
        &self,
        target: &tcl_runtime_api::ArrayTarget,
    ) -> Result<Option<Vec<Vec<u8>>>, tcl_syntax::value::ValueError> {
        if target.cell_id().is_none() {
            // LocateArray observed no original cell; a later same-name creation
            // cannot become this operation's target.
            return Ok(None);
        }
        let record = self.array_operation_target(target).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("retained array search"),
        )?;
        Ok(crate::vars::array_search_keys_at_target(&record))
    }

    pub(crate) fn array_search_element_exists_at_target(
        &self,
        target: &tcl_runtime_api::ArrayTarget,
        key: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        let record = self.array_operation_target(target).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("retained array search"),
        )?;
        Ok(crate::vars::array_search_element_exists_at_target(
            &record, key,
        ))
    }

    pub(crate) fn array_unset_elem_at_target(
        &mut self,
        target: &tcl_runtime_api::ArrayTarget,
        key: &[u8],
    ) -> bool {
        if let Some(record) = self.array_operation_target(target) {
            let Some((removed, home)) = crate::vars::unset_element_at_target(&record, key) else {
                return false;
            };
            if removed && self.has_variable_traces() {
                self.fire_selected_unset_callbacks(
                    &home,
                    target.name_bytes(),
                    Some(key),
                    target.name_bytes(),
                    true,
                );
            }
            removed
        } else {
            false
        }
    }

    /// Tcl's `TclPtrGetVarIdx` read used by `array get`: select the live
    /// element before callbacks, fire containing-array then element traces,
    /// and read that same element afterwards. Trace errors are swallowed but
    /// published to `errorInfo`/`errorCode`; destruction of the array selected
    /// for the enclosing operation remains a hard read error.
    pub(crate) fn array_read_elem_at_target(
        &mut self,
        target: &tcl_runtime_api::ArrayTarget,
        key: &[u8],
    ) -> tcl_runtime_api::ArrayElementRead<*mut TclObj> {
        let live_array = crate::vars::array_target_at(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            target.name_bytes(),
            target.frame().0,
        );
        let live_was_array = live_array.as_ref().is_some_and(|live| {
            crate::vars::array_names_at_target(
                &self.frames.borrow(),
                &self.namespaces.borrow(),
                live,
            )
            .is_some()
        });
        let selected = crate::vars::array_element_target_at(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            target.name_bytes(),
            key,
            target.frame().0,
        );
        let selected_retained = selected.as_ref().is_some_and(|(array, element)| {
            crate::vars::retain_array_element_target(
                &mut self.frames.borrow_mut(),
                &mut self.namespaces.borrow_mut(),
                array,
                key,
                *element,
            )
        });
        let trace_errored = self
            .fire_read_trace(target.name_bytes(), Some(key))
            .is_some();
        let trace_failure = trace_errored.then(|| {
            (
                self.result_bytes(),
                self.error_code(),
                self.error_info(),
                i64::from(self.error_line()),
            )
        });

        let outcome = match self.array_operation_target(target) {
            Some(operation)
                if crate::vars::array_names_at_target(
                    &self.frames.borrow(),
                    &self.namespaces.borrow(),
                    &operation,
                )
                .is_some() =>
            {
                if let Some((_, _, info, line)) = trace_failure {
                    self.publish_and_reset_error();
                    tcl_runtime_api::ArrayElementRead::Missing(
                        tcl_runtime_api::ArrayReadMiss::trace_error(Some(info), line),
                    )
                } else {
                    let value = selected.as_ref().map_or_else(
                        || self.var_get_elem(target.name_bytes(), key),
                        |(array, element)| {
                            crate::vars::get_element_at_target(
                                &self.frames.borrow(),
                                &self.namespaces.borrow(),
                                array,
                                key,
                                *element,
                            )
                        },
                    );
                    value.map_or_else(
                        || {
                            let miss = if live_was_array {
                                tcl_runtime_api::ArrayReadMiss::missing()
                            } else {
                                tcl_runtime_api::ArrayReadMiss::lookup(error_code_list(&[
                                    b"TCL",
                                    b"LOOKUP",
                                    b"VARNAME",
                                    target.name_bytes(),
                                ]))
                            };
                            tcl_runtime_api::ArrayElementRead::Missing(miss)
                        },
                        tcl_runtime_api::ArrayElementRead::Value,
                    )
                }
            }
            Some(operation) => {
                let invalidation = if crate::vars::array_target_is_set(
                    &self.frames.borrow(),
                    &self.namespaces.borrow(),
                    &operation,
                ) {
                    tcl_runtime_api::ArrayInvalidation::Retyped
                } else {
                    tcl_runtime_api::ArrayInvalidation::Unset
                };
                trace_failure.map_or_else(
                    || tcl_runtime_api::ArrayElementRead::ArrayInvalidated(invalidation),
                    |(message, code, info, line)| {
                        tcl_runtime_api::ArrayElementRead::TraceError(
                            tcl_runtime_api::ArrayReadFailure::new_bytes(
                                message,
                                code,
                                Some(info),
                                Some(line),
                            ),
                        )
                    },
                )
            }
            None => trace_failure.map_or_else(
                || {
                    tcl_runtime_api::ArrayElementRead::ArrayInvalidated(
                        tcl_runtime_api::ArrayInvalidation::Unset,
                    )
                },
                |(message, code, info, line)| {
                    tcl_runtime_api::ArrayElementRead::TraceError(
                        tcl_runtime_api::ArrayReadFailure::new_bytes(
                            message,
                            code,
                            Some(info),
                            Some(line),
                        ),
                    )
                },
            ),
        };
        if let tcl_runtime_api::ArrayElementRead::Value(value) = &outcome {
            // The selected element's retained cell is released immediately
            // below. Transfer one hold to the shared array core first so no
            // raw pointer can die between this return and list materialisation.
            tcl_syntax::value::ValueOps::pin_value(self, value);
        }
        if selected_retained {
            if let Some((array, element)) = selected {
                crate::vars::release_array_element_target(
                    &mut self.frames.borrow_mut(),
                    &mut self.namespaces.borrow_mut(),
                    &array,
                    key,
                    element,
                );
            }
        }
        outcome
    }

    /// Fire `name`'s `array` traces — C's `TclCheckArrayTraces`, which every
    /// `array` subcommand reaches through `LocateArray` (tclVar.c:330-350).
    /// `Some(Code::Error)` when a callback errored and the subcommand must
    /// fail with `can't trace array "name": <msg>` (`LocateArray` passes
    /// `leaveErrMsg` 1), else `None`.
    ///
    /// C gates on `TclIsVarArray(varPtr) || TclIsVarUndefined(varPtr)`, so an
    /// `array` trace fires for an array or for a variable that does not exist
    /// yet, and never for a scalar — nor for an array *element*, which is not
    /// an array however it was spelled or aliased. Ordinary element reads and
    /// writes do not fire it at all: it is the `array` command's own hook.
    pub(crate) fn fire_array_trace(&mut self, name: &[u8]) -> Option<Code> {
        if self.traces.borrow().traces.is_empty() {
            return None;
        }
        let (base, elem) = match self.variable_name_parts(name) {
            Ok(parts) => parts,
            Err(_) => return Some(Code::Error),
        };
        if elem.is_some() || (!self.var_is_array(&base) && self.var_exists(&base)) {
            return None;
        }
        if !self.fire_var_trace(&base, None, b"array") {
            return None;
        }
        let msg = self
            .traces
            .borrow_mut()
            .pending_err
            .take()
            .unwrap_or_default();
        Some(self.var_trace_error(&base, b"array", &msg))
    }

    /// Fire `var`'s `op` traces; on a read/write callback error return the
    /// access-aborting message, already wrapped as `can't read/set "var": <msg>`
    /// (`TclCallVarTraces`), else `None`. The `Traces::fire` engine (`state_traits.rs`)
    /// — it keeps the trace internals (the firing guard, `pending_err`, the
    /// per-op wrapping) here; `unset`/`array` callback errors do not abort, so
    /// they yield `None`.
    pub(crate) fn fire_var_traces_for(&mut self, var: &[u8], op: &[u8]) -> Option<Vec<u8>> {
        let Ok((base, elem)) = self.variable_name_parts(var) else {
            return None;
        };
        if !self.fire_var_trace(&base, elem.as_deref(), op) {
            return None;
        }
        let raw = self
            .traces
            .borrow_mut()
            .pending_err
            .take()
            .unwrap_or_default();
        // The user-facing verb: a write trace reports `can't set` (C's wording).
        let verb: &[u8] = if op == b"read" { b"read" } else { b"set" };
        let mut m = b"can't ".to_vec();
        m.extend_from_slice(verb);
        m.extend_from_slice(b" \"");
        m.extend_from_slice(var);
        m.extend_from_slice(b"\": ");
        m.extend_from_slice(&raw);
        Some(m)
    }

    /// Fire matching command traces (`rename`/`delete`) as `command oldName
    /// newName op` (C's `TraceCommandProc`). `new_fqn` is empty for a delete.
    /// Callback errors are **ignored** (C: "We ignore errors in these traced
    /// commands"). Re-entrant firing on *this* command is suppressed (C's
    /// per-`Command` `CMD_TRACE_ACTIVE`/`CMD_DYING`); the interp result is
    /// preserved across the callbacks.
    /// Fire one command token's own trace list.
    /// C walks `cmdPtr->tracePtr`, so a trace registered on a *later* token
    /// bound at the same name — a replacement, or a same-named command in a
    /// namespace that took the retained one's spelling — never fires for the
    /// token being deleted. `dying` is `None` only when an internal caller has
    /// no bound token to discriminate by.
    ///
    /// `key` addresses the trace list and gates re-entry; `old_fqn` only names
    /// the command in the callback's first word. They differ for a rename,
    /// which fires from the *destination's* list (see
    /// [`Self::move_bound_command`]) while still naming the command it is
    /// leaving.
    fn fire_cmd_trace_of_token(
        &mut self,
        key: &[u8],
        old_fqn: &[u8],
        new_fqn: &[u8],
        op_bit: u8,
        dying: Option<u64>,
    ) {
        if self.host_refusal_pending() {
            return;
        }
        if self
            .traces
            .borrow()
            .firing_cmd_traces
            .iter()
            .any(|firing| firing.0 == key && firing.1 == dying)
        {
            return;
        }
        // C prepends each new command trace (`Tcl_TraceCommand`, tclTrace.c
        // 9.0.4:1016-1018) and `CallCommandTraces` walks the list head→tail
        // (tclBasic.c:3972-3974), so the newest fires first. Our Vec pushes
        // newest-last.
        // The callbacks are captured up front, not re-read per step: this walk
        // owns the dying token's list, which a callback's re-creation of the
        // command detaches from the name. See [`Self::cmd_trace_untraced`].
        let entries: Vec<(u64, Vec<u8>)> = self
            .traces
            .borrow()
            .cmd_traces
            .iter()
            .rev()
            // Both mechanisms are needed and neither can do the other's
            // job: the token generation says which registrations this
            // deletion *owns* (a callback's re-creation binds a different
            // token under the same name), while the id lets the walk skip a
            // registration an explicit `trace remove` cancelled mid-walk.
            .filter(|t| {
                t.name == key && (t.ops & op_bit) != 0 && cmd_trace_owned_by(t.token, dying)
            })
            .map(|t| (t.id, t.command.clone()))
            .collect();
        if entries.is_empty() {
            return;
        }
        let op: &[u8] = if op_bit == crate::cmd_trace::ops::RENAME {
            b"rename"
        } else {
            b"delete"
        };
        // Preserve the result object across the callbacks.
        let saved = self.save_native_command_trace_result(false);

        // Only `firing_cmd_traces` is raised, never `exec_firing`: C sets
        // `INTERP_TRACE_IN_PROGRESS` in exactly one place — `TraceExecutionProc`
        // (tclTrace.c 9.0.4:1765), around an *execution* trace's callback —
        // and `CallCommandTraces` sets nothing. So a command dispatched from a
        // `rename`/`delete` callback is traced like any other: its `enter` and
        // `leave` traces fire, and an enclosing `enterstep`/`leavestep` scope
        // steps the callback's own commands. Re-entering *this* command's
        // rename/delete traces is what is suppressed, per command, above.
        // `firing_cmd_traces` is therefore what marks this walk as in flight
        // for `TraceTable::trace_walk_in_flight`.
        self.traces
            .borrow_mut()
            .firing_cmd_traces
            .push((key.to_vec(), dying));
        for (id, cmd) in entries {
            if self.cmd_trace_untraced(id) {
                continue;
            }
            // Append `oldName newName op` as properly-quoted list elements.
            let args =
                self.new_list_object(&[new_string(old_fqn), new_string(new_fqn), new_string(op)]);
            let mut line = cmd;
            line.push(b' ');
            line.extend_from_slice(&obj_bytes(args));
            drop_fresh(args);
            self.reset_native_error_objects_before_trace_script();
            self.clear_return_options();
            let saved_script = self.save_native_command_trace_result(true);
            let _ = self.eval_native_command_trace_script(false, &line);
            if let Some(saved) = saved_script {
                self.restore_native_variable_trace_result(saved);
            }
        }
        {
            let mut traces = self.traces.borrow_mut();
            traces.firing_cmd_traces.pop();
            if !traces.trace_walk_in_flight() {
                traces.untraced_cmd_trace_ids.clear();
            }
        }

        if let Some(saved) = saved {
            self.restore_native_variable_trace_result(saved);
        }
    }

    /// The callback prefix of the live command/execution trace `id`, or `None`
    /// when a callback has since removed it. C walks the trace list through
    /// `nextPtr` and `Tcl_UntraceCommand` unlinks a record at once, so a trace
    /// removed mid-firing never fires in that pass.
    ///
    /// This is the **execution** rule: `TclCheckExecutionTraces` follows the
    /// list of whatever command the name now holds, so a callback that
    /// redefines the traced command stops the rest of the walk — measured on
    /// tclsh 8.6.16 and 9.0.4, where `proc t {}` inside an `enter` callback
    /// keeps the older `enter` callback from running. The delete walk answers
    /// differently; see [`Self::cmd_trace_untraced`].
    fn live_cmd_trace(&self, id: u64) -> Option<Vec<u8>> {
        self.traces
            .borrow()
            .cmd_traces
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.command.clone())
    }

    /// Whether execution trace `id` is currently running its own callback —
    /// C's per-trace `TCL_TRACE_EXEC_IN_PROGRESS` (`tclTrace.c` 9.0.4:1655),
    /// which is what bounds a callback that invokes the command it traces. Per
    /// *trace*: a second trace on the same command still fires for that inner
    /// call, and so does a `leave` trace while an `enter` one is running
    /// (tclsh 8.6/9.0-pinned).
    fn exec_trace_is_firing(&self, id: u64) -> bool {
        self.traces.borrow().firing_exec_traces.contains(&id)
    }

    /// Whether `trace remove` has unlinked command trace `id` since this walk
    /// began. `CallCommandTraces` is handed the dying token's own list
    /// (`tclBasic.c` 9.0.4:3972-3993), so a callback that re-creates the
    /// command under the same name takes the name-keyed table entry over while
    /// the remaining callbacks still run; only an explicit untrace cancels one.
    /// Measured on tclsh 8.6.16 and 9.0.4: with two `delete` traces whose newer
    /// callback runs `proc foo …`, **both** fire.
    fn cmd_trace_untraced(&self, id: u64) -> bool {
        self.traces.borrow().untraced_cmd_trace_ids.contains(&id)
    }

    /// Fire `enter` execution traces on `fqn` (creation order), invoking each as
    /// `<prefix> {cmd args} enter`. Returns `Some(code)` if a callback completed
    /// non-OK — the command is then aborted with that code and the callback's
    /// result (C's `TclEvalObjvInternal`: `traceCode != TCL_OK ⇒ return`).
    fn fire_exec_enter(&mut self, fqn: &[u8], token: Option<u64>, cmd_word: &[u8]) -> Option<Code> {
        use crate::cmd_trace::ops;
        // C fires `enter` newest-first (the trace list is prepended; the loop
        // walks it head→tail). Our Vec pushes newest-last, so iterate reversed.
        let ids: Vec<u64> = self
            .traces
            .borrow()
            .cmd_traces
            .iter()
            .rev()
            .filter(|t| t.name == fqn && t.token == token && (t.ops & ops::ENTER) != 0)
            .map(|t| t.id)
            .collect();
        if ids.is_empty() {
            return None;
        }
        let saved = self.result.get();
        unsafe { obj::incr_ref_count(saved) };
        let saved_options = self.take_return_options();
        self.traces.borrow_mut().exec_firing += 1;
        let mut abort: Option<Code> = None;
        for id in ids {
            // A trace whose own callback is running is skipped, per trace
            // rather than per command (C's `TCL_TRACE_EXEC_IN_PROGRESS`).
            if self.exec_trace_is_firing(id) {
                continue;
            }
            let Some(cmd) = self.live_cmd_trace(id) else {
                continue;
            };
            let args = self.new_list_object(&[new_string(cmd_word), new_string(b"enter")]);
            let mut line = cmd;
            line.push(b' ');
            line.extend_from_slice(&obj_bytes(args));
            drop_fresh(args);
            self.traces.borrow_mut().firing_exec_traces.push(id);
            self.clear_return_options();
            let c = self.eval_native_command_trace_script(true, &line);
            self.traces.borrow_mut().firing_exec_traces.pop();
            if c != Code::Ok {
                // The callback's result becomes the command's result; abort.
                abort = Some(c);
                break;
            }
        }
        self.traces.borrow_mut().exec_firing -= 1;
        if abort.is_some() {
            // Drop the preserved result; the callback's result stands.
            unsafe { obj::decr_ref_count(saved) };
        } else {
            unsafe {
                obj::decr_ref_count(self.result.get());
                self.result.set(saved);
            }
            self.restore_return_options(saved_options);
        }
        abort
    }

    /// Fire `leave` execution traces on `fqn` (reverse creation order), invoking
    /// each as `<prefix> {cmd args} <code> <result> leave`. A leave-trace non-OK
    /// code overrides the command's result/code (C's `TEOV_RunLeaveTraces`).
    fn fire_exec_leave(
        &mut self,
        fqn: &[u8],
        token: Option<u64>,
        cmd_word: &[u8],
        code: Code,
    ) -> Code {
        use crate::cmd_trace::ops;
        // C fires `leave` oldest-first (reverse-scan of the prepended list). Our
        // Vec pushes newest-last, so iterate forward.
        let ids: Vec<u64> = self
            .traces
            .borrow()
            .cmd_traces
            .iter()
            .filter(|t| t.name == fqn && t.token == token && (t.ops & ops::LEAVE) != 0)
            .map(|t| t.id)
            .collect();
        if ids.is_empty() {
            return code;
        }
        // Save the command's result once; restore it after the callbacks (C's
        // single Tcl_SaveInterpState/RestoreInterpState around the loop). The
        // result is NOT restored *between* callbacks: each leave callback's
        // `<result>` element is the live result, so a callback that changes the
        // result is observed by the next one.
        let saved = self.result.get();
        unsafe { obj::incr_ref_count(saved) };
        let saved_options = self.take_return_options();
        let code_str = code.as_int().to_string().into_bytes();

        self.traces.borrow_mut().exec_firing += 1;
        let mut override_code: Option<Code> = None;
        for id in ids {
            // A trace whose own callback is running is skipped, per trace
            // rather than per command (C's `TCL_TRACE_EXEC_IN_PROGRESS`).
            if self.exec_trace_is_firing(id) {
                continue;
            }
            let Some(cmd) = self.live_cmd_trace(id) else {
                continue;
            };
            let result_bytes = obj_bytes(self.result.get());
            let args = self.new_list_object(&[
                new_string(cmd_word),
                new_string(&code_str),
                new_string(&result_bytes),
                new_string(b"leave"),
            ]);
            let mut line = cmd;
            line.push(b' ');
            line.extend_from_slice(&obj_bytes(args));
            drop_fresh(args);
            self.traces.borrow_mut().firing_exec_traces.push(id);
            self.clear_return_options();
            let c = self.eval_native_command_trace_script(true, &line);
            self.traces.borrow_mut().firing_exec_traces.pop();
            if c != Code::Ok {
                override_code = Some(c);
                break;
            }
        }
        self.traces.borrow_mut().exec_firing -= 1;

        match override_code {
            // A leave-trace error/return overrides; the callback's result stands.
            Some(c) => {
                unsafe { obj::decr_ref_count(saved) };
                c
            }
            // Restore the command's own result and code.
            None => {
                unsafe {
                    obj::decr_ref_count(self.result.get());
                    self.result.set(saved);
                }
                self.restore_return_options(saved_options);
                code
            }
        }
    }

    /// The name a command's identity lives under now, following any open
    /// `rename` window (see [`crate::cmd_trace::TraceTable::rename_windows`]) —
    /// `None` outside one. Inside a rename's callbacks the vacating name still
    /// resolves, but it *is* the destination command, so a `trace`, a `rename`
    /// or a delete through it must reach the destination's state.
    pub(crate) fn renamed_cmd_key(&self, fqn: &[u8]) -> Option<Vec<u8>> {
        self.traces
            .borrow()
            .rename_windows
            .iter()
            .find(|(from, _)| from == fqn)
            .map(|(_, to)| to.clone())
    }

    /// A command moved `old_fqn` → `new_fqn`: point every open `rename` window
    /// and every in-flight firing record that named the old key at the new one.
    /// C needs no equivalent — both its hash entries reference one `Command`, so
    /// a nested rename cannot strand them — but our name-keyed state would
    /// otherwise be left on a name the move just vacated: an enclosing window
    /// would stop answering through the vacating name, and the
    /// `firing_cmd_traces` record standing in for `CMD_TRACE_ACTIVE` would stop
    /// suppressing the command's own remaining callbacks.
    fn relocate_rename_state(&self, old_fqn: &[u8], new_fqn: &[u8], generation: u64) {
        let mut traces = self.traces.borrow_mut();
        for (_, destination) in &mut traces.rename_windows {
            if destination == old_fqn {
                *destination = new_fqn.to_vec();
            }
        }
        for (firing, token) in &mut traces.firing_cmd_traces {
            if firing == old_fqn && *token == Some(generation) {
                *firing = new_fqn.to_vec();
            }
        }
    }

    /// Move every command/execution trace on `old_fqn` to `new_fqn` (the trace
    /// follows a renamed command, as C keeps the trace list on the moving
    /// `Command`).
    fn move_cmd_traces(&mut self, old_fqn: &[u8], new_fqn: &[u8], generation: u64) {
        self.relocate_rename_state(old_fqn, new_fqn, generation);
        // C moves one `Command` and its trace list. A visible and hidden token
        // may legally share display spelling, so move only this generation.
        let mut traces = self.traces.borrow_mut();
        let mut moved = false;
        for t in traces.cmd_traces.iter_mut() {
            if t.name == old_fqn && t.token == Some(generation) {
                t.name = new_fqn.to_vec();
                moved = true;
            }
        }
        drop(traces);
        if moved {
            self.invalidate_guard_domain(GuardDomain::CommandTrace);
        }
    }

    /// Whether `name` resolves to an array variable (`set a` array-vs-scalar
    /// diagnostic, `array exists`).
    pub(crate) fn var_is_array(&self, name: &[u8]) -> bool {
        if self.require_variable_name_protocol().is_err() {
            return false;
        };
        crate::vars::is_array(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            self.current_ns.get(),
            name,
        )
    }

    /// Resolve a `global`/`variable`/`upvar` name argument to its
    /// `(target namespace, simple tail)`, in the given `context_ns` (global for
    /// `global`, the current ns for `variable`). `None` if the name is qualified
    /// into a namespace that doesn't exist.
    pub(crate) fn resolve_var_target(
        &self,
        context_ns: NsId,
        name: &[u8],
    ) -> Option<(NsId, Vec<u8>)> {
        let protocol = self.require_variable_name_protocol().ok()?;
        let input = protocol.variable_root_input(name);
        if protocol.is_jim084()
            || input.qualification() != tcl_syntax::naming::NativeNameQualification::Unqualified
        {
            self.namespaces.borrow().var_home(context_ns, name)
        } else {
            Some((context_ns, input.selected().to_vec()))
        }
    }

    /// Qualification selected from the original variable operand.
    pub(crate) fn variable_is_qualified(&self, name: &[u8]) -> bool {
        self.require_variable_name_protocol().is_ok_and(|protocol| {
            protocol.variable_root_input(name).qualification()
                != tcl_syntax::naming::NativeNameQualification::Unqualified
        })
    }

    /// The current call-frame level (`upvar` relative-level arithmetic).
    pub(crate) fn current_level(&self) -> usize {
        self.frames.borrow().current_level()
    }

    /// `variable tail` / `global tail` — link `tail` in the current frame to
    /// `target_ns::tail` (a no-op when the current context already is that var).
    pub(crate) fn make_variable(&mut self, target_ns: NsId, tail: &[u8]) {
        if self.require_variable_name_protocol().is_err() {
            return;
        }

        crate::vars::make_variable(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            target_ns,
            tail,
        );
    }

    /// Link local name `local` to `target_ns::target` (TIP 500 private instance
    /// variables, whose storage name is mangled per declaring class).
    pub(crate) fn make_variable_mapped(&mut self, target_ns: NsId, local: &[u8], target: &[u8]) {
        if self.require_variable_name_protocol().is_err() {
            return;
        }

        crate::vars::make_variable_mapped(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            target_ns,
            local,
            target,
        );
    }

    /// The fully-qualified name of an existing namespace `name` (absolute or
    /// relative to the current namespace), or `None` if it does not exist —
    /// for `definitionnamespace`, which requires the namespace to exist.
    pub(crate) fn resolve_namespace_name(&self, name: &[u8]) -> Option<Vec<u8>> {
        let ns = self.namespaces.borrow();
        let id = ns.find_namespace(self.current_ns.get(), name)?;
        Some(ns.qualified_name(id))
    }

    /// Resolve (creating if needed) a namespace by name, relative to the current
    /// namespace — for `apply`'s optional namespace term.
    pub(crate) fn ensure_namespace(&mut self, name: &[u8]) -> NsId {
        self.invalidate_command_environment();
        self.namespaces
            .borrow_mut()
            .ensure_namespace(self.current_ns.get(), name)
    }

    /// Create an OO command's default instance namespace under the exact
    /// namespace token that will receive the command. This differs from public
    /// `namespace eval` creation only during a synchronous delete callback,
    /// where the command and its owned namespace must both join the dying
    /// generation.
    pub(crate) fn ensure_command_owned_namespace(&mut self, name: &[u8]) -> NsId {
        self.namespaces
            .borrow_mut()
            .ensure_command_owned_namespace(self.current_ns.get(), name)
    }

    /// Resolve (creating if needed) a namespace by name, anchored at the
    /// **global** namespace regardless of the current one — C's
    /// `TclGetNamespaceForQualName(..., TCL_GLOBAL_ONLY |
    /// TCL_CREATE_NS_IF_UNKNOWN)`. Used by `interp invokehidden -namespace`:
    /// `-namespace bar` names `::bar` even when called from inside another
    /// namespace (tclsh 9.0.4-pinned).
    pub(crate) fn ensure_global_namespace(&mut self, name: &[u8]) -> NsId {
        self.invalidate_command_environment();
        self.namespaces.borrow_mut().ensure_namespace(GLOBAL, name)
    }

    /// Delete the namespace `ns` (by id), e.g. an OO object's instance namespace
    /// when the object is destroyed.
    pub(crate) fn delete_namespace_by_id(&mut self, ns: NsId) {
        self.invalidate_command_environment();
        if self.defer_namespace_teardown(ns) {
            return;
        }
        let teardown_ids = self.namespaces.borrow().descendant_ids(ns);
        // TclOO state follows the exact namespace token. This must happen only
        // once deletion is no longer deferred: an active old token can coexist
        // with fresh objects at the same Tcl-facing name. Descendants receive
        // the same check when recursion reaches their own token.
        self.oo_namespace_deleted(ns);
        let native_error_reset = ns == GLOBAL && !self.native_error_cells.borrow().is_empty();
        if ns == GLOBAL {
            self.native_error_cells.borrow_mut().clear();
        }
        self.delete_namespace_token(ns);
        self.sweep_dying_namespace(ns, &teardown_ids);
        self.namespaces
            .borrow_mut()
            .finish_namespace_teardown(&teardown_ids);
        if native_error_reset {
            self.install_native_error_variable_traces();
        }
    }

    /// C's `activationCount > (nsPtr == globalNsPtr)` branch of
    /// `Tcl_DeleteNamespace`: a token a call frame is still running in retires
    /// its owned ensembles and drops its public name now, but keeps its command
    /// table, variables and children for those frames. `Tcl_PopCallFrame` calls
    /// the deletion again once the last of them goes away. Reports whether the
    /// teardown was deferred.
    fn defer_namespace_teardown(&mut self, ns: NsId) -> bool {
        if !self.namespaces.borrow().namespace_is_active(ns) {
            return false;
        }
        self.retire_namespace_owned_ensembles(ns);
        self.retire_namespace_unknown_root(ns);
        let retired = self.namespaces.borrow_mut().defer_namespace(ns);
        drop(retired);
        true
    }

    /// The same token teardown, entered the way C's
    /// `TclDeleteNamespaceChildren` enters it — through `Tcl_DeleteNamespace`,
    /// so a child with its own live frame defers in turn.
    fn delete_namespace_token_checked(&mut self, ns: NsId) {
        if self.defer_namespace_teardown(ns) {
            return;
        }
        self.oo_namespace_deleted(ns);
        self.delete_namespace_token(ns);
    }

    /// Retire the ensemble tokens configured against `ns`, and the imports of
    /// the commands they were bound to. C pops `nsPtr->ensembles` before it
    /// looks at the activation count, so an owned ensemble dies immediately
    /// even when the namespace itself is retained.
    fn retire_namespace_owned_ensembles(&mut self, ns: NsId) {
        let ids = std::collections::HashSet::from([ns]);
        let mut deleted_origins = std::collections::HashSet::<u64>::new();
        let mut ensemble_victims = self.namespaces.borrow().ensembles_for(&ids);
        let hidden_tokens: Vec<(Vec<u8>, Rc<crate::ensemble::EnsembleToken>)> = self
            .hidden
            .borrow()
            .iter()
            .filter_map(|(name, binding)| match &binding.command {
                Command::Ensemble(token) if ids.contains(&token.config().ns) => {
                    let mut fqn = b"::".to_vec();
                    fqn.extend_from_slice(name);
                    Some((fqn, Rc::clone(token)))
                }
                _ => None,
            })
            .collect();
        ensemble_victims.extend(hidden_tokens);
        let mut deleted_tokens = Vec::with_capacity(ensemble_victims.len());
        for (_fqn, token) in ensemble_victims {
            if deleted_tokens.iter().any(|seen| Rc::ptr_eq(seen, &token)) {
                continue;
            }
            if let Some((_, generation)) = self.ensemble_identity_location(&token) {
                deleted_origins.insert(generation);
            }
            if let Some((_, generation)) = self.retire_ensemble_identity(&token) {
                deleted_origins.insert(generation);
            }
            deleted_tokens.push(token);
        }
        self.remove_imports_for_deleted_origins(deleted_origins, &deleted_tokens);
    }

    /// Tear down one exact namespace token in Tcl's recursive order. Its owned
    /// ensembles retire while this token and all children are live; then only
    /// this token becomes dying and loses its ordinary command table. Children
    /// receive the same lifecycle recursively after the parent's callbacks.
    fn delete_namespace_token(&mut self, ns: NsId) {
        if self
            .require_variable_name_protocol()
            .is_ok_and(|protocol| protocol.is_jim084())
        {
            let qualified = self.namespaces.borrow().qualified_name(ns);
            let key = tcl_syntax::naming::jim_global_variable_key_bytes(b"", &qualified);
            let prefix = (!key.is_empty()).then(|| [key.as_slice(), b"::"].concat());
            let names = self.namespaces.borrow().var_names(GLOBAL);
            for name in names.into_iter().filter(|name| {
                prefix
                    .as_ref()
                    .is_none_or(|prefix| name.starts_with(prefix))
            }) {
                let absolute = [b"::".as_slice(), &name].concat();
                self.var_unset(&absolute);
            }
        }
        self.retire_namespace_owned_ensembles(ns);
        self.retire_namespace_unknown_root(ns);
        // Keep this token's original variable path during its callbacks. Its
        // public namespace identity is dying; the parent edge retires afterwards.
        self.namespaces
            .borrow_mut()
            .begin_namespace_variable_teardown(ns);
        self.retire_namespace_variables(ns);
        self.namespaces.borrow_mut().begin_namespace_teardown(ns);
        self.namespaces.borrow_mut().clear_namespace_token(ns);
        self.tear_down_command_table(ns);
        self.namespaces
            .borrow_mut()
            .finish_native_namespace_command_path(ns);

        // Tcl snapshots and recursively deletes children only after this
        // token's ordinary command callbacks have completed. A callback may
        // already have deleted one; skip any token no longer publicly live.
        let children = self.namespaces.borrow().children_hash_order(ns);
        for child in children {
            if self.namespaces.borrow().namespace_is_live(child) {
                self.delete_namespace_token_checked(child);
            }
        }
        self.namespaces
            .borrow_mut()
            .namespace_name_finish_deletion(ns);
    }

    /// Delete one dying namespace's command table in its selected C traversal:
    /// Tcl 8.4/8.5 select the first entry again after each command; Tcl 8.6/9
    /// retain each complete table frontier until that deletion pass finishes.
    ///
    /// Each token's `delete` traces fire while its entry is still in the table
    /// (`Tcl_DeleteCommandFromToken` calls `CallCommandTraces` before
    /// `Tcl_DeleteHashEntry`), and its imports retire depth-first straight
    /// after — not in a bulk pass over the whole namespace. A callback that
    /// deletes or redefines a snapshotted entry changes its generation; C's
    /// `CMD_DYING` early return then leaves the replacement to the next
    /// snapshot.
    fn tear_down_command_table(&mut self, ns: NsId) {
        // Native proof: naming.namespace.original-command-holder-routing
        // docs/design/analysis/name-resolution-proofs/namespace-original-command-holder-routing.md
        // The public callback rows and pinned C teardown traversal stay separate.
        let traversal = self
            .name_policy_protocol()
            .and_then(|policy| policy.recipe().namespace_command_teardown());
        loop {
            let mut snapshot = self.namespaces.borrow().command_hash_order(ns);
            if let Some(traversal) = traversal {
                snapshot.truncate(traversal.frontier_len(snapshot.len()));
            }
            if snapshot.is_empty() {
                break;
            }
            let mut retired_any = false;
            for (tail, generation) in snapshot {
                if self.namespaces.borrow().command_generation(ns, &tail) != Some(generation) {
                    continue;
                }
                self.on_bound_command_replaced(ns, &tail);
                if self.namespaces.borrow().command_generation(ns, &tail) != Some(generation) {
                    // The callback deleted this token, or redefined the name:
                    // its own deletion already unlinked the entry, and any
                    // replacement is a distinct token for the next snapshot.
                    retired_any = true;
                    continue;
                }
                let command = self.namespaces.borrow().command_in(ns, &tail);
                let ensemble_tokens = match &command {
                    Some(Command::Ensemble(token)) => vec![Rc::clone(token)],
                    _ => Vec::new(),
                };
                let oo_owner = command.as_ref().and_then(Command::oo_object);
                self.namespaces.borrow_mut().remove_in(ns, &tail);
                self.retire_pending_native_ensemble_roles();
                self.remove_imports_for_deleted_origins([generation], &ensemble_tokens);
                if let Some(owner) = oo_owner {
                    self.oo_command_renamed(owner, None);
                }
                retired_any = true;
            }
            if !retired_any {
                break;
            }
        }
    }

    /// `root`'s subtree with every retained token's subtree stepped over — the
    /// nodes this teardown still owns.
    fn retained_teardown_ids(&self, root: NsId) -> Vec<NsId> {
        let namespaces = self.namespaces.borrow();
        namespaces
            .descendant_ids(root)
            .into_iter()
            .filter(|id| !namespaces.under_deferred_token(*id))
            .collect()
    }

    /// Finish commands created re-entrantly while a namespace's original
    /// delete callbacks ran. The detached namespace remains command-addressable
    /// during this sweep, but never reappears in the visible namespace tree.
    fn sweep_dying_namespace(&mut self, root: NsId, retained_ids: &[NsId]) {
        // A child whose own frame was still running deferred its teardown; it
        // and its subtree keep every binding they had until that frame pops.
        let mut ids: Vec<NsId> = {
            let namespaces = self.namespaces.borrow();
            retained_ids
                .iter()
                .copied()
                .filter(|id| !namespaces.under_deferred_token(*id))
                .collect()
        };
        let mut traced = std::collections::HashSet::<(NsId, Vec<u8>)>::new();
        let mut origins = std::collections::HashSet::<u64>::new();
        let mut tokens = Vec::<Rc<crate::ensemble::EnsembleToken>>::new();

        loop {
            for id in self.retained_teardown_ids(root) {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            let id_set: std::collections::HashSet<NsId> = ids.iter().copied().collect();

            let mut ensemble_victims = self.namespaces.borrow().ensembles_for(&id_set);
            ensemble_victims.extend(self.hidden.borrow().iter().filter_map(|(name, binding)| {
                let Command::Ensemble(token) = &binding.command else {
                    return None;
                };
                if !id_set.contains(&token.config().ns)
                    || tokens.iter().any(|seen| Rc::ptr_eq(seen, token))
                {
                    return None;
                }
                let mut fqn = b"::".to_vec();
                fqn.extend_from_slice(name);
                Some((fqn, Rc::clone(token)))
            }));
            let mut found_new_token = false;
            for (_fqn, token) in ensemble_victims {
                if tokens.iter().any(|seen| Rc::ptr_eq(seen, &token)) {
                    continue;
                }
                found_new_token = true;
                if let Some((_, generation)) = self.ensemble_identity_location(&token) {
                    origins.insert(generation);
                }
                if let Some((_, generation)) = self.retire_ensemble_identity(&token) {
                    origins.insert(generation);
                }
                tokens.push(token);
            }

            let slots = self.namespaces.borrow().command_slots_in_ids(&ids);
            let new_slots: Vec<(NsId, Vec<u8>)> = slots
                .into_iter()
                .filter(|slot| !traced.contains(slot))
                .collect();
            for (id, tail) in &new_slots {
                // Callback-created command traces fire while the command is
                // still addressable through the dying namespace token.
                self.on_bound_command_replaced(*id, tail);
                traced.insert((*id, tail.clone()));
            }

            for id in self.retained_teardown_ids(root) {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            origins.extend(
                self.namespaces
                    .borrow()
                    .command_locations_in_ids(&ids)
                    .into_iter()
                    .map(|(_, generation)| generation),
            );
            self.remove_imports_for_deleted_origins(origins.iter().copied(), &tokens);

            let remaining = self.namespaces.borrow().command_slots_in_ids(&ids);
            if !found_new_token
                && new_slots.is_empty()
                && remaining.iter().all(|slot| traced.contains(slot))
            {
                break;
            }
        }

        // Import delete callbacks may have added a new child under the detached
        // root during the last fixed-point pass.
        for id in self.retained_teardown_ids(root) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        origins.extend(
            self.namespaces
                .borrow()
                .command_locations_in_ids(&ids)
                .into_iter()
                .map(|(_, generation)| generation),
        );
        self.remove_imports_for_deleted_origins(origins.iter().copied(), &tokens);
        let cleared = self.namespaces.borrow().command_locations_in_ids(&ids);
        let retired = self.namespaces.borrow_mut().clear_namespace_ids(&ids);
        self.retire_pending_native_ensemble_roles();
        drop(retired);
        for (fqn, generation) in cleared {
            self.remove_cmd_traces_of_token(&fqn, Some(generation));
        }
    }

    /// The unset-trace callbacks a **cell** contributes when it is destroyed,
    /// newest-first — C walks the `Var`'s prepended trace list head to tail.
    /// Non-destructive: the caller's own sweep drops the traces.
    fn cell_unset_traces(
        &self,
        home: &crate::vars::TraceHome,
        elem: Option<&[u8]>,
        report_name: &[u8],
        report_elem: &[u8],
    ) -> Vec<VarTeardownCallback> {
        let cell = self.variable_trace_scope(home, elem);
        self.traces
            .borrow()
            .traces
            .iter()
            .rev()
            .filter(|trace| cell.owns_registration(trace) && cell.matches(trace, b"unset"))
            .map(|t| {
                (
                    report_name.to_vec(),
                    report_elem.to_vec(),
                    native_variable_observers::Callback::from_trace(t),
                    t.old_style,
                )
            })
            .collect()
    }

    /// Fire collected unset-trace callbacks as `command name {} unset`. Errors
    /// are ignored (an unset trace's result is discarded, as in C).
    fn fire_unset_callbacks(&mut self, victims: Vec<VarTeardownCallback>) {
        if self.host_refusal_pending() || victims.is_empty() {
            return;
        }
        let saved = self.save_native_variable_trace_result(false);
        for (name, elem, cmd, old_style) in victims {
            if let native_variable_observers::Callback::Native(observer) = &cmd {
                self.call_native_variable_observer(observer.clone(), &name, &elem, "unset");
                continue;
            }
            let native_variable_observers::Callback::Script(cmd) = cmd else {
                unreachable!()
            };
            // A trace registered the deprecated way is called with the `rwua`
            // letter, not the operation name — the teardown path must honour
            // that exactly as the explicit-unset path does (`TraceVarProc`,
            // tclTrace.c 8.6.16:2002-2011).
            let op = tcl_cmd_core::trace::callback_op_word("unset", old_style);
            let args = self.new_list_object(&[
                new_string(&name),
                new_string(&elem),
                new_string(op.as_bytes()),
            ]);
            let mut line = cmd;
            line.push(b' ');
            line.extend_from_slice(&obj_bytes(args));
            drop_fresh(args);
            self.clear_return_options();
            let saved_script = self.save_native_variable_trace_result(true);
            let _ = self.eval_str(&line);
            if let Some(saved_script) = saved_script {
                self.restore_native_variable_trace_result(saved_script);
            }
        }
        if let Some(saved) = saved {
            self.restore_native_variable_trace_result(saved);
        }
    }

    /// Resolve a (relative/absolute) namespace name to its id, or `None`.
    pub(crate) fn find_namespace_id(&self, name: &[u8]) -> Option<NsId> {
        self.namespaces
            .borrow()
            .find_namespace(self.current_ns.get(), name)
    }

    // introspection (`info` / `array`)

    /// `info exists name` — whether a scalar/array/element variable is set
    /// (splitting `arr(key)`).
    pub(crate) fn var_exists(&self, name: &[u8]) -> bool {
        if self.observed_names.borrow().is_some() {
            let level = self.frames.borrow().current_level();
            return self.observed_variable_exists(name, level).unwrap_or(false);
        }
        let Ok(input) = self.combined_variable_input(name) else {
            return false;
        };
        let base = input.root().selected();
        match input.element() {
            Some(element) => crate::vars::exists_elem(
                &self.frames.borrow(),
                &self.namespaces.borrow(),
                self.current_ns.get(),
                base,
                element.selected(),
            ),
            None => crate::vars::exists(
                &self.frames.borrow(),
                &self.namespaces.borrow(),
                self.current_ns.get(),
                base,
            ),
        }
    }

    /// The element names of array `name` (`array names`/`get`), or `None`.
    pub(crate) fn array_names(&self, name: &[u8]) -> Option<Vec<Vec<u8>>> {
        self.require_variable_name_protocol().ok()?;
        crate::vars::array_names(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            self.current_ns.get(),
            name,
        )
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

    /// The directly-bound command names that the selected runtime surface
    /// permits callers to enumerate. This is the common filter behind the
    /// `info commands` adapter and internal namespace listings.
    pub(crate) fn visible_command_names_in(&self, id: NsId) -> Vec<Vec<u8>> {
        let ns = self.namespaces.borrow();
        let mut prefix = ns.qualified_name(id);
        if prefix != b"::" {
            prefix.extend_from_slice(b"::");
        }
        ns.command_names(id)
            .iter()
            .filter_map(|name| {
                // Mirror `resolve_dispatchable`'s gate exactly: a command the
                // release does not have must not be *listed* either, or
                // `info commands ::oo::*` on an 8.4 surface advertises names
                // that then fail to dispatch. The TclOO roots the engine
                // installs on the registry's behalf are gated alongside
                // builtins; every script-created object stays invariant. Use
                // the exact generation too, so a renamed builtin is gated by
                // the same stable registry identity as dispatch.
                let command = ns.command_in(id, name)?;
                let mut fqn = prefix.clone();
                fqn.extend_from_slice(name);
                self.command_visible_for_surface_at(&command, &fqn, ns.command_generation(id, name))
                    .then(|| name.to_vec())
            })
            .collect()
    }

    /// Actual original Jim table reports, filtered using their independent comparison slots.
    pub(crate) fn visible_command_report_names_in(&self, id: NsId) -> Vec<Vec<u8>> {
        let names = self.visible_command_names_in(id);
        let namespaces = self.namespaces();
        names
            .iter()
            .map(|name| namespaces.command_table_report_name(id, name).to_vec())
            .collect()
    }

    /// Report original table keys only for actual visible Jim alias bindings.
    pub(crate) fn visible_alias_report_names_in(
        &self,
        id: NsId,
    ) -> Result<Vec<Vec<u8>>, tcl_syntax::value::ValueError> {
        if self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_none()
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim alias inventory issuer",
            ));
        }
        let names = self.visible_command_names_in(id);
        let namespaces = self.namespaces();
        Ok(names
            .iter()
            .filter_map(|name| {
                matches!(namespaces.command_in(id, name), Some(Command::Alias { .. }))
                    .then(|| namespaces.command_table_report_name(id, name).to_vec())
            })
            .collect())
    }

    /// The unqualified names of `id`'s commands that `namespace import`
    /// created — C's `NamespaceImportCmd` introspection form (`objc == 1`),
    /// which walks the namespace's `cmdTable` for entries whose `deleteProc`
    /// is `DeleteImportedCmd`. Sorted: C yields hash order, which is not
    /// reproducible, and the VM sorts for the same reason.
    pub(crate) fn imported_command_tails(&self, id: NsId) -> Vec<Vec<u8>> {
        let ns = self.namespaces.borrow();
        let mut names: Vec<Vec<u8>> = ns
            .command_names(id)
            .iter()
            .filter(|name| matches!(ns.resolve(id, name), Some(Command::Imported { .. })))
            .map(|name| name.to_vec())
            .collect();
        names.sort();
        names
    }

    /// The proc definition bound to `name` (for `info body`/`args`/`default`).
    pub(crate) fn proc_def(&self, name: &[u8]) -> Option<Rc<ProcDef>> {
        let mut cmd = self
            .namespaces
            .borrow()
            .resolve(self.current_ns.get(), name)?;
        // Follow `namespace import` redirects to the underlying proc, so
        // `info args`/`body`/`default` work on an imported proc (info-1.7/2.4).
        for _ in 0..64 {
            match cmd {
                Command::Proc(def) => return Some(def.declaration()),
                Command::Imported {
                    source,
                    source_generation,
                    ensemble,
                    ..
                } if ensemble.as_ref().is_none_or(|token| token.is_deleted()) => {
                    cmd = self.resolve_import_source(&source, source_generation)?;
                }
                _ => return None,
            }
        }
        None
    }

    /// Capture the selected target cell before installing an element alias.
    pub(crate) fn prepare_upvar_target(&mut self, target: &mut Link) -> Result<(), VarError> {
        if self.observed_names.borrow().is_some() {
            return self.observed_refusal();
        }
        crate::vars::prepare_upvar_target(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            target,
        )
    }

    /// `upvar` — link `local` in the current frame to the resolved `target`.
    pub(crate) fn make_upvar(&mut self, target: Link, local: &[u8]) {
        if self.require_variable_name_protocol().is_err() {
            return;
        }

        crate::vars::make_upvar(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            target,
            local,
        );
    }

    /// The storage resolver's lifetime check for an `upvar` alias.
    pub(crate) fn upvar_would_invert(&self, target: &Link, local: &[u8]) -> bool {
        crate::vars::upvar_would_invert(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            self.current_ns.get(),
            target,
            local,
        )
    }

    /// `upvar … target ns::tail` — install the link as namespace variable
    /// `home_ns::tail` (a qualified local name).
    pub(crate) fn make_upvar_in(&mut self, home_ns: NsId, tail: &[u8], target: Link) {
        if self.require_variable_name_protocol().is_err() {
            return;
        }

        crate::vars::make_upvar_in(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            home_ns,
            tail,
            target,
        );
    }

    // result

    /// Construct a List retaining the producer's selected string updater.
    /// An unavailable dialect leaves the backing unselected; construction
    /// grants no native conversion or execution authority.
    pub(crate) fn new_list_object(&self, elements: &[*mut TclObj]) -> *mut TclObj {
        let recipe = self
            .native_invocation_dialect()
            .native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        ));
        match recipe {
            Some(recipe) => crate::list::new_list_obj_native(elements, recipe.protocol()),
            None => crate::list::new_list_obj(elements),
        }
    }

    /// `Tcl_SetObjResult`: retain `obj` into the result slot, release the prior.
    ///
    /// # Safety
    /// `obj` must be a live `TclObj`.
    pub unsafe fn set_obj_result(&mut self, obj: *mut TclObj) {
        if let Some(recipe) = self
            .native_invocation_dialect()
            .native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        )) {
            // A fresh compound producer retains the selected updater without
            // materialising members or changing their native references.
            if crate::list::native_string_protocol(obj).is_none() {
                let _ = crate::list::seal_string_protocol(obj, recipe.protocol());
            }
            if crate::dict::native_string_protocol(obj).is_none() {
                let _ = crate::dict::seal_string_protocol(obj, recipe.protocol());
            }
        }
        // A different result cannot inherit a primitive getter projection.
        let mut exc = self.exc.borrow_mut();
        exc.primitive_getter = None;
        exc.expression_error_stage = None;
        drop(exc);
        let old = self.result.get();
        // SAFETY: `obj` live (caller); `old` is the interp's owned result.
        unsafe {
            obj::incr_ref_count(obj);
            self.result.set(obj);
            obj::decr_ref_count(old);
        }
    }

    /// Set the result to `obj` (the safe wrapper builtins use on objects they
    /// already hold — argv elements, or fresh objects they just minted).
    ///
    /// Not marked `unsafe`: in the runtime every `TclObj *` in flight is a live
    /// object from our single allocator (the Tcl C model), and builtins handle
    /// these pointers ubiquitously — threading `unsafe` through every call site
    /// would add noise without adding safety. The invariant is upheld by the
    /// eval loop's retain/release discipline.
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn set_result(&mut self, obj: *mut TclObj) {
        // SAFETY: builtins only pass live objects (argv elements / fresh objs).
        unsafe { self.set_obj_result(obj) }
    }

    /// Set the result to a fresh string object with `bytes`.
    pub fn set_result_bytes(&mut self, bytes: &[u8]) {
        let obj = new_string(bytes);
        // SAFETY: fresh obj; set_obj_result retains it, then we drop our 0-ref
        // (the obj was rc 0; set_obj_result took it to rc 1, interp-owned).
        unsafe { self.set_obj_result(obj) };
    }

    /// Set the result to a byte-array object with `bytes` as its exact raw
    /// payload. Its normal string representation is generated only when a
    /// string consumer requests it.
    pub(crate) fn byte_array_string_recipe(
        &mut self,
    ) -> Result<tcl_registry::native_string_materialization::ByteArrayStringRecipe, Code> {
        self.native_invocation_dialect().byte_array_string_recipe(
            Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation),
        ).ok_or_else(|| self.refuse_native_access(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("byte-array string materialization")))
    }

    /// Create byte-array storage only after its complete updater recipe is selected.
    pub(crate) fn new_native_byte_array(&mut self, bytes: &[u8]) -> Result<*mut TclObj, Code> {
        let recipe = self.byte_array_string_recipe()?;
        Ok(crate::bytearray::new_byte_array(bytes, recipe))
    }

    pub(crate) fn set_result_byte_array(&mut self, bytes: &[u8]) -> Code {
        let obj = match self.new_native_byte_array(bytes) {
            Ok(value) => value,
            Err(code) => return code,
        };
        // SAFETY: fresh byte-array object; the interpreter takes its owning
        // reference exactly as it does for an ordinary fresh string object.
        unsafe { self.set_obj_result(obj) };
        Code::Ok
    }

    /// `Tcl_GetObjResult` — borrowed (interp keeps its +1).
    pub fn get_obj_result(&self) -> *mut TclObj {
        self.result.get()
    }

    /// `Tcl_ResetResult`: an empty result, no error in flight (the next one
    /// starts its own `errorInfo`, `-errorcode` and error stack) and no pending
    /// `return`, so its level and code are the defaults again.
    pub(crate) fn reset_result(&mut self) {
        self.set_result_bytes(b"");
        *self.exc.borrow_mut() = ExceptionState::default();
        self.mark_error_stack_reset();
        self.set_return_state(1, Code::Ok);
        self.clear_return_options();
    }

    /// An error a C API call reports: `message` as the result and, when given,
    /// `code` as its `-errorcode` (`NONE` otherwise), starting a new error.
    pub(crate) fn c_api_error(&mut self, message: &[u8], code: Option<&[u8]>) {
        self.set_result_bytes(message);
        *self.exc.borrow_mut() = ExceptionState {
            info: None,
            code: code.map(<[u8]>::to_vec).unwrap_or_default(),
            code_explicit: code.is_some(),
            already_logged: false,
        };
        self.c_api_errors.set(self.c_api_errors.get() + 1);
    }

    /// `Tcl_SetObjErrorCode`: `code` is the `-errorcode` of the error the running
    /// C command is about to return, whatever its result.
    pub(crate) fn set_c_error_code(&mut self, code: &[u8]) {
        *self.exc.borrow_mut() = ExceptionState {
            info: None,
            code: code.to_vec(),
            code_explicit: true,
            already_logged: false,
        };
        self.c_api_errors.set(self.c_api_errors.get() + 1);
    }

    /// Note that a C API call left an error the interpreter already holds (a
    /// command's own, such as `package provide`'s), as [`Self::c_api_error`]
    /// would have.
    pub(crate) fn note_c_api_error(&self) {
        self.c_api_errors.set(self.c_api_errors.get() + 1);
    }

    /// `Tcl_AppendResult`'s one piece: the result grows by `piece`, in place when
    /// it is a plain string nothing else holds.
    pub(crate) fn append_result_bytes(&mut self, piece: &[u8]) {
        let current = self.result.get();
        if obj::is_plain_string(current) && !obj::is_shared(current) {
            obj::string_append_inplace(current, piece);
            return;
        }
        let mut bytes = obj_bytes(current);
        bytes.extend_from_slice(piece);
        self.set_result_bytes(&bytes);
    }

    /// The current result's string bytes (copied).
    pub fn result_bytes(&self) -> Vec<u8> {
        obj_bytes(self.result.get())
    }

    /// Run an implicit callback while preserving the caller's result and
    /// carried completion options. The callback may inspect and replace the
    /// live result internally; its final value is discarded when `f` returns.
    pub(crate) fn with_preserved_result<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let saved = self.result.get();
        // SAFETY: the interpreter owns `saved`; hold an independent reference
        // while callback evaluation replaces the result slot.
        unsafe { obj::incr_ref_count(saved) };
        let saved_options = self.take_return_options();
        let output = f(self);
        // SAFETY: discard the callback's owned result and transfer the saved
        // reference back into the interpreter result slot.
        unsafe { obj::decr_ref_count(self.result.get()) };
        self.result.set(saved);
        self.restore_return_options(saved_options);
        output
    }

    /// The current result **object** — a borrowed pointer (the interp keeps its
    /// reference; the caller must not release it without first taking its own
    /// `+1`). Backs `Commands::dispatch`'s completion capture in `state_traits.rs`.
    pub(crate) fn result_obj(&self) -> *mut TclObj {
        self.result.get()
    }

    /// Intern an exact command token to a stable, dense raw `CommandId`, minting
    /// one on first sight. Backs `Namespaces::find_command`.
    fn intern_cmd(&self, fqn: &[u8], generation: u64) -> u32 {
        let mut a = self.cmd_arena.borrow_mut();
        let key = (fqn.to_vec(), generation);
        if let Some(&id) = a.ids.get(&key) {
            return id;
        }
        let id = u32::try_from(a.commands.len()).expect("command count fits u32");
        a.commands.push(key.clone());
        a.ids.insert(key, id);
        id
    }

    /// Resolve `name` from namespace context `cxt` (through the namespace tree to
    /// the root) to its command's FQN, then intern that to a stable raw
    /// `CommandId`. `None` if it resolves to no command. The `Namespaces::find_command`
    /// engine (`state_traits.rs`), keeping the namespace-table access here.
    pub(crate) fn find_command_id(&self, cxt: NsId, name: &[u8]) -> Option<u32> {
        let (fqn, generation) = {
            let namespaces = self.namespaces.borrow();
            (
                namespaces.resolve_fqn(cxt, name)?,
                namespaces.resolve_generation(cxt, name)?,
            )
        };
        Some(self.intern_cmd(&fqn, generation))
    }

    /// The exact `(FQN, generation)` an interned raw `CommandId` names.
    pub(crate) fn command_identity(&self, id: u32) -> Option<(Vec<u8>, u64)> {
        self.cmd_arena.borrow().commands.get(id as usize).cloned()
    }

    /// The FQN projection of an interned raw `CommandId`.
    pub(crate) fn command_fqn(&self, id: u32) -> Option<Vec<u8>> {
        let (_, generation) = self.command_identity(id)?;
        self.raw_command_location_by_generation(generation)
            .map(|(fqn, _)| fqn)
    }

    /// The command an interned command was ultimately imported from — C's
    /// `TclGetOriginalCommand` (following an imported command's retained
    /// ensemble token or by-name source to a fixed point), interned in its turn.
    /// `None` when it is not an imported command. Backs
    /// `Namespaces::command_origin`. Bounded against a cycle a retargeting bug
    /// could leave behind; a well-formed chain is acyclic.
    pub(crate) fn imported_source_id(&self, id: u32) -> Option<u32> {
        let (_, mut generation) = self.command_identity(id)?;
        let (mut fqn, mut command) = self.raw_command_location_by_generation(generation)?;
        let mut hops = 0;
        while let Command::Imported {
            source,
            source_generation,
            ensemble,
            ..
        } = command
        {
            let source = ensemble
                .filter(|token| !token.is_deleted())
                .map_or(source, |token| token.name());
            if let Some(next) = self.raw_command_by_generation(source_generation) {
                fqn = source;
                generation = source_generation;
                command = next;
            } else {
                let namespaces = self.namespaces.borrow();
                fqn = namespaces.resolve_fqn(GLOBAL, &source)?;
                generation = namespaces.resolve_generation(GLOBAL, &source)?;
                command = namespaces.resolve(GLOBAL, &source)?;
            }
            hops += 1;
            if hops >= 64 {
                break;
            }
        }
        (hops > 0).then(|| self.intern_cmd(&fqn, generation))
    }

    /// Immediate source binding and optional real-ensemble identity for a new
    /// `namespace import`. Import-of-import chains deliberately keep every hop:
    /// replacing or deleting the intermediate command must affect its own
    /// importers before `namespace origin` walks any farther toward the root.
    /// Only a *direct* ensemble source contributes an ensemble token; an
    /// imported ensemble is reached through its intermediate command binding.
    pub(crate) fn import_metadata_in(
        &self,
        source_ns: NsId,
        simple: &[u8],
    ) -> Option<(Vec<u8>, u64, Option<Rc<crate::ensemble::EnsembleToken>>)> {
        let namespaces = self.namespaces.borrow();
        let fqn = namespaces.command_fqn_at(source_ns, simple);
        let source_generation = namespaces.command_generation(source_ns, simple)?;
        let ensemble = match namespaces.command_in(source_ns, simple)? {
            Command::Ensemble(token) => Some(token),
            _ => None,
        };
        Some((fqn, source_generation, ensemble))
    }

    /// The fully-qualified name of namespace `ns` (`"::"` for the root). Backs
    /// `Namespaces::name` (`state_traits.rs`), keeping the namespace-table access here.
    pub(crate) fn ns_qualified_name(&self, ns: NsId) -> Vec<u8> {
        self.namespaces.borrow().qualified_name(ns)
    }

    /// Raise an error with result `msg` and return [`Code::Error`] — the generic
    /// throw. Resets the [`ExceptionState`] to a fresh error: the source trace
    /// (`::errorInfo`) is then built up **as the error unwinds**
    /// ([`log_command_info`](Self::log_command_info) /
    /// [`make_proc_error`](Self::make_proc_error)) and published to the globals at
    /// the catch / outermost-eval boundary — not stamped here.
    ///
    /// Arbitrary result bytes carry the neutral `NONE` error-code identity.
    pub(crate) fn error(&mut self, msg: &[u8]) -> Code {
        if self.host_refusal_pending() {
            return Code::Error;
        }
        self.set_result_bytes(msg);
        let code = b"NONE";
        *self.exc.borrow_mut() = ExceptionState {
            native: Default::default(),
            info: None,
            code: code.to_vec(),
            code_explicit: false,
            already_logged: false,
            primitive_getter: None,
            expression_error_stage: None,
        };
        self.capture_native_error_objects();
        Code::Error
    }

    /// Like [`error`](Self::error) but with an explicit `-errorcode` (the trace
    /// still builds up as the error unwinds). For commands that mirror C's
    /// richer error codes (`TCL LOOKUP INDEX …`, `TCL OO …`).
    pub(crate) fn error_with_code(&mut self, msg: &[u8], code: &[u8]) -> Code {
        if self.host_refusal_pending() {
            return Code::Error;
        }
        self.set_result_bytes(msg);
        *self.exc.borrow_mut() = ExceptionState {
            native: Default::default(),
            info: None,
            code: code.to_vec(),
            code_explicit: false,
            already_logged: false,
            primitive_getter: None,
            expression_error_stage: None,
        };
        self.capture_native_error_objects();
        Code::Error
    }

    /// Select result representation from retained command-error metadata.
    /// A lookalike message cannot select the native arity append producer.
    fn command_error_string_result(
        &self,
        expected: Option<tcl_syntax::native_string::NativeStringProtocol>,
        wrong_arguments: bool,
    ) -> Result<
        Option<tcl_registry::native_string_materialization::NativeStringMaterialization>,
        tcl_syntax::value::ValueError,
    > {
        let dialect = self.native_invocation_dialect();
        let expected = expected.or_else(|| {
            wrong_arguments.then(|| dialect.native_wrong_arguments_protocol())
                .flatten()
                .and_then(tcl_registry::native_wrong_arguments::NativeWrongArgumentsProtocol::string_result)
        });
        let Some(expected) = expected else {
            return Ok(None);
        };
        dialect
            .native_string_materialization(None)
            .filter(|actual| actual.protocol() == expected)
            .map(Some)
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "selected error String producer",
            ))
    }

    /// Publish a portable command-layer error through this runtime's result
    /// and exception-state ABI without losing a structured error code.
    pub(crate) fn report_cmd_error(&mut self, error: tcl_cmd_core::CmdError) -> Code {
        if self.host_refusal_pending() {
            return Code::Error;
        }
        if let Some(error) = error.native_access_refusal() {
            return self.refuse_native_access(error);
        }
        let details = error.into_byte_details();
        let explicit_code_store =
            matches!(details.error_code, tcl_cmd_core::CmdErrorCodeUpdate::Set(_));
        let wrong_arguments = matches!(
            details.error_code,
            tcl_cmd_core::CmdErrorCodeUpdate::WrongArguments
        );
        let update = match details.error_code.resolve(|| {
            self.native_invocation_dialect()
                .wrong_arguments_protocol(Some(tcl_registry::native_wrong_arguments::LogicalWrongArgumentsProvider::Tcl84CoreSimulation))
                .map(|protocol| protocol.error_code().as_bytes().to_vec())
                .ok_or(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("wrong arguments"))
        }) {
            Ok(update) => update,
            Err(refusal) => return self.refuse_native_access(refusal),
        };
        let string_result =
            match self.command_error_string_result(details.string_result, wrong_arguments) {
                Ok(result) => result,
                Err(error) => return self.report_cmd_error(error.into()),
            };
        self.set_result_bytes(&details.message);
        if let Some(materialization) = string_result {
            if let Err(error) =
                obj::retain_native_string_representation(self.get_obj_result(), materialization)
            {
                return self.report_cmd_error(error.into());
            }
        }
        if let tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(code) = &update {
            if !self.uses_c84_global_error_info() || explicit_code_store {
                self.replace_native_error_code(code);
            } else {
                self.exc.borrow_mut().native.global_code_set = false;
            }
        }
        let mut exc = self.exc.borrow_mut();
        match update {
            tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(code) => {
                exc.code = code;
                exc.code_explicit = false;
            }
            tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Unchanged => {}
        }
        if self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_some()
        {
            exc.native.info_len = details.error_info.as_ref().map_or(0, Vec::len);
            exc.native.info = details
                .error_info
                .as_deref()
                .map(|bytes| obj::Owned::fresh(new_string(bytes)));
        }
        exc.info = details.error_info;
        exc.already_logged = false;
        exc.primitive_getter = details.primitive_getter;
        exc.expression_error_stage = None;
        drop(exc);
        if let Some(line) = details
            .error_line
            .and_then(|value| u32::try_from(value).ok())
        {
            self.error_line.set(line);
        }
        Code::Error
    }

    /// Publish a list parse failure using the shared list owner's message and
    /// structured code.
    pub(crate) fn report_list_error(&mut self, source: &[u8], error: parse::ListError) -> Code {
        self.error_with_code(
            &parse::list_error_message(source, error),
            error.error_code(),
        )
    }

    /// Set an authored background-handler prefix in a fixture.
    #[cfg(test)]
    pub(crate) fn set_bgerror_handler(&self, prefix: &[u8]) {
        self.invalidate_interpreter_policy();
        let replaced = self
            .bgerror
            .borrow_mut()
            .replace(obj::Owned::fresh(new_string(prefix)));
        drop(replaced);
    }

    /// Store or return the original background handler, retaining its object identity.
    pub(crate) fn bgerror_apply(
        &mut self,
        prefix: Option<*mut TclObj>,
    ) -> Result<obj::Owned, tcl_cmd_core::CmdError> {
        if let Some(prefix) = prefix {
            let valid = match tcl_syntax::value::ValueOps::list_len(self, &prefix) {
                Ok(length) => length > 0,
                Err(error) if error.native_access_refusal().is_some() => return Err(error.into()),
                Err(_) => false,
            };
            if !valid {
                let mut detail = tcl_cmd_core::CmdError::new_bytes(
                    b"cmdPrefix must be list of length >= 1".to_vec(),
                )
                .into_byte_details();
                if let Some(code) = self
                    .native_invocation_dialect()
                    .native_event_protocol()
                    .and_then(|p| p.background_prefix_format_code())
                {
                    detail.error_code = tcl_cmd_core::CmdErrorCodeUpdate::Set(code.to_vec());
                }
                return Err(tcl_cmd_core::CmdError::from_byte_details(detail));
            }
            self.invalidate_interpreter_policy();
            let replaced = self
                .bgerror
                .borrow_mut()
                .replace(obj::Owned::retain(prefix));
            drop(replaced);
        }
        Ok(self
            .bgerror
            .borrow()
            .as_ref()
            .cloned()
            .unwrap_or_else(|| obj::Owned::fresh(new_string(b"::tcl::Bgerror"))))
    }

    /// Queue a background error (a destructor failing during implicit teardown,
    /// etc.) for later processing by `update` — Tcl defers it to the event loop
    /// rather than firing at the error site.
    pub(crate) fn report_bg_error(&mut self, msg: &[u8], options: &[u8]) {
        self.bg_queue.borrow_mut().push((
            obj::Owned::fresh(new_string(msg)),
            obj::Owned::fresh(new_string(options)),
        ));
    }

    pub(crate) fn report_bg_error_original(&mut self, message: obj::Owned, options: obj::Owned) {
        self.bg_queue.borrow_mut().push((message, options));
    }

    /// Mutable access to the event loop's pending-event queue (`after`/`vwait`/
    /// `update`, in `cmd_event`). Callers must drop the borrow before evaluating
    /// an event script.
    pub(crate) fn events_mut(&self) -> std::cell::RefMut<'_, crate::cmd_event::EventQueue> {
        self.events.borrow_mut()
    }

    /// Mutable access to the live-coroutine registry (`cmd_coro`). Callers must
    /// drop the borrow before blocking on a coroutine handoff (the worker thread
    /// reaches the same registry to swap its context).
    pub(crate) fn coros_mut(
        &self,
    ) -> std::cell::RefMut<'_, std::collections::BTreeMap<u64, crate::cmd_coro::CoroEntry>> {
        self.coros.borrow_mut()
    }

    /// Publish a probe's original objects before acknowledging its handoff.
    /// Only the running worker accesses the interpreter until that acknowledgement.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn publish_coro_probe_error(&self, snapshot: ErrorSnapshot) {
        let previous = self.coro_probe_error.replace(Some(snapshot));
        debug_assert!(
            previous.is_none(),
            "previous probe receipt was not consumed"
        );
    }

    /// Consume the parked worker's receipt before any further coroutine handoff.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn take_coro_probe_error(&self) -> Option<ErrorSnapshot> {
        self.coro_probe_error.borrow_mut().take()
    }

    /// Swap the interp's per-flow *execution context* (call frames, the
    /// `info frame` stack, current namespace, recursion depth, return/error
    /// state, the TclOO call/define stacks, …) with `ctx`. This is how a
    /// coroutine handoff installs the resuming side's context: a single swap is
    /// its own inverse, so resume (caller→coro) and yield (coro→caller) each
    /// call it once. The shared *definitions* (namespaces, commands, classes,
    /// channels, the result object) are not swapped — coroutines share them.
    pub(crate) fn swap_coro_ctx(&self, ctx: &mut CoroContext) {
        self.native_compilation
            .borrow_mut()
            .swap_execution(&mut ctx.native_compilation);
        {
            let mut f = self.frames.borrow_mut();
            std::mem::swap(&mut *f, &mut ctx.frames);
        }
        std::mem::swap(&mut *self.cmd_frames.borrow_mut(), &mut ctx.cmd_frames);
        std::mem::swap(&mut *self.script_stack.borrow_mut(), &mut ctx.script_stack);
        std::mem::swap(&mut *self.arg_lines.borrow_mut(), &mut ctx.arg_lines);
        std::mem::swap(&mut *self.exc.borrow_mut(), &mut ctx.exc);
        std::mem::swap(&mut *self.error_stack.borrow_mut(), &mut ctx.error_stack);
        self.error_stack.borrow_mut().configure(
            self.native_invocation_dialect()
                .native_error_objects_protocol(),
        );
        std::mem::swap(
            &mut *self.jim_error_stack.borrow_mut(),
            &mut ctx.jim_error_stack,
        );
        std::mem::swap(
            &mut *self.jim_evaluation_frames.borrow_mut(),
            &mut ctx.jim_evaluation_frames,
        );
        std::mem::swap(
            &mut *self.jim_invocation_borrows.borrow_mut(),
            &mut ctx.jim_invocation_borrows,
        );
        ctx.jim_procedure_level = self.jim_procedure_level.replace(ctx.jim_procedure_level);
        ctx.native_dispatch_depth = self
            .native_dispatch_depth
            .replace(ctx.native_dispatch_depth);
        std::mem::swap(
            &mut *self.deferred_tailcalls.borrow_mut(),
            &mut ctx.deferred_tailcalls,
        );
        self.oo.borrow_mut().swap_exec(&mut ctx.oo);
        let ns = self.current_ns.replace(ctx.current_ns);
        ctx.current_ns = ns;
        let rd = self.recursion_depth.replace(ctx.recursion_depth);
        ctx.recursion_depth = rd;
        let rc = self.return_code.replace(ctx.return_code);
        ctx.return_code = rc;
        let rl = self.return_level.replace(ctx.return_level);
        ctx.return_level = rl;
        std::mem::swap(
            &mut *self.return_options.borrow_mut(),
            &mut ctx.return_options,
        );
        std::mem::swap(
            &mut *self.array_operation_targets.borrow_mut(),
            &mut ctx.array_operation_targets,
        );
        std::mem::swap(
            &mut *self.active_var_trace_scopes.borrow_mut(),
            &mut ctx.active_var_trace_scopes,
        );
        let ed = self.eval_depth.replace(ctx.eval_depth);
        ctx.eval_depth = ed;
        let el = self.error_line.replace(ctx.error_line);
        ctx.error_line = el;
    }

    /// Swap the execution context of the actual coroutine generation with the live
    /// interpreter context (the resume/yield handoff in `cmd_coro`). A no-op if
    /// the coroutine is gone. The registry borrow is released before any
    /// blocking handoff.
    pub(crate) fn coro_swap_generation(&self, generation: u64) {
        // Take the context out (releasing the registry borrow), swap, put back —
        // so `swap_coro_ctx`'s RefCell touches never overlap the registry borrow.
        let taken = self
            .coros
            .borrow_mut()
            .get_mut(&generation)
            .map(|e| std::mem::replace(&mut e.context, CoroContext::placeholder()));
        if let Some(mut ctx) = taken {
            self.swap_coro_ctx(&mut ctx);
            if let Some(e) = self.coros.borrow_mut().get_mut(&generation) {
                e.context = ctx;
            }
        }
    }

    /// A second owning handle to the same interpreter state (an `Rc` clone) —
    /// handed to a coroutine worker thread (`cmd_coro`).
    pub(crate) fn clone_handle(&self) -> Interp {
        Interp(Rc::clone(&self.0))
    }

    /// Whether `name` resolves to a command in the current namespace.
    pub(crate) fn command_exists(&self, name: &[u8]) -> bool {
        self.namespaces
            .borrow()
            .resolve(self.current_ns.get(), name)
            .is_some()
    }

    /// Dispatch retained background-error prefixes and preserve the caller's result.
    pub(crate) fn process_bg_errors(&mut self) {
        while !self.bg_queue.borrow().is_empty() {
            let batch = {
                let mut pending = self.bg_queue.borrow_mut();
                std::mem::take(&mut *pending)
            };
            for (message, options) in batch {
                let jim = self
                    .native_invocation_dialect()
                    .native_event_protocol()
                    .is_some_and(|p| p.after_options_exact());
                let configured = if jim {
                    None
                } else {
                    self.bgerror.borrow().clone()
                };
                let has_configured = configured.is_some();
                let handler =
                    configured.unwrap_or_else(|| obj::Owned::fresh(new_string(b"bgerror")));
                let saved = obj::Owned::retain(self.get_obj_result());
                let mut argv =
                    match tcl_syntax::value::ValueOps::list_elements(self, &handler.as_ptr()) {
                        Ok(words) => words
                            .into_iter()
                            .map(obj::Owned::retain)
                            .collect::<Vec<_>>(),
                        Err(error) => {
                            self.report_cmd_error(error.into());
                            continue;
                        }
                    };
                if let Some(head) = argv.first() {
                    let namespace = if jim {
                        self.current_ns.get()
                    } else {
                        crate::namespace::GLOBAL
                    };
                    match self.resolve_original_command_at(namespace, head.as_ptr()) {
                        Ok(Some(_)) => {
                            argv.push(message);
                            if has_configured {
                                argv.push(options);
                            }
                            let words = argv.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                            if jim {
                                let _ = self.dispatch(&words);
                            } else {
                                self.with_event_global_frame(|interp| {
                                    let _ = interp.dispatch(&words);
                                });
                            }
                        }
                        Ok(None) => {}
                        Err(error) => {
                            self.report_cmd_error(error.into());
                            continue;
                        }
                    }
                }
                unsafe {
                    self.set_obj_result(saved.as_ptr());
                }
            }
        }
    }

    /// Pre-seed the error trace for `error msg info ?code?` / `throw`: the result
    /// is `msg`, the trace starts at `info` (so the throwing command is **not**
    /// re-logged — `ERR_ALREADY_LOGGED`), and `-errorcode` is `code`. Returns
    /// [`Code::Error`].
    pub(crate) fn raise_with_info(&mut self, msg: &[u8], info: &[u8], code: &[u8]) -> Code {
        self.set_result_bytes(msg);
        if self.uses_jim_error_stack() {
            self.jim_error_stack
                .borrow_mut()
                .adopt_explicit(obj::Owned::fresh(new_string(info)));
        }
        *self.exc.borrow_mut() = ExceptionState {
            native: Default::default(),
            info: Some(info.to_vec()),
            code: code.to_vec(),
            code_explicit: false,
            already_logged: true,
            primitive_getter: None,
            expression_error_stage: None,
        };
        self.capture_native_error_objects();
        self.exc.borrow_mut().native.legacy_copy = true;
        Code::Error
    }

    /// Begin a fresh error whose result the caller has already set, with
    /// `-errorcode` `code` and an empty trace (it accumulates as the error
    /// unwinds). Used by `error msg`/`throw`. Returns [`Code::Error`].
    pub(crate) fn set_error_state(&mut self, code: &[u8]) -> Code {
        *self.exc.borrow_mut() = ExceptionState {
            native: Default::default(),
            info: None,
            code: code.to_vec(),
            code_explicit: false,
            already_logged: false,
            primitive_getter: None,
            expression_error_stage: None,
        };
        self.capture_native_error_objects();
        Code::Error
    }

    /// Apply the error-related options of a `return -code error` to the live
    /// exception state (`TclProcessReturn`'s `TCL_ERROR` arm). This populates the
    /// interp's errorInfo/errorCode/errorStack — *not* the `::errorInfo`/
    /// `::errorCode` globals, which are written only when the error is actually
    /// reported (caught as code 1 / reaching the top level). An explicit non-empty
    /// `-errorinfo` is taken verbatim and marks the error already-logged (so the
    /// unwind does not append a `while executing` frame); otherwise the trace
    /// accumulates normally. `-errorcode` defaults to `NONE`; a valid `-errorstack`
    /// replaces the built stack.
    pub(crate) fn process_return_error(
        &mut self,
        errorinfo: Option<&[u8]>,
        errorcode: Option<&[u8]>,
        errorstack: Option<&[u8]>,
    ) {
        if self.uses_jim_error_stack() {
            if let Some(info) = errorinfo {
                self.jim_error_stack
                    .borrow_mut()
                    .adopt_explicit(obj::Owned::fresh(new_string(info)));
            }
        }
        let (info, already_logged) = match errorinfo {
            Some(i) if !i.is_empty() => (Some(i.to_vec()), true),
            _ => (None, false),
        };
        *self.exc.borrow_mut() = ExceptionState {
            native: Default::default(),
            info,
            code: errorcode.unwrap_or(b"NONE").to_vec(),
            code_explicit: errorcode.is_some(),
            already_logged,
            primitive_getter: None,
            expression_error_stage: None,
        };
        self.capture_native_error_objects();
        self.exc.borrow_mut().native.legacy_copy = true;
        if let Some(es) = errorstack.filter(|_| self.runtime_version().has_error_stack()) {
            if let Ok(parts) = validate_error_stack(crate::parse::split_list(es)) {
                let _ = self.error_stack.borrow_mut().adopt(parts);
            }
        }
    }

    /// Append one `while executing` / `invoked from within` frame for the command
    /// `src[cmd.start..cmd.end]` as an error unwinds through it — the
    /// `TclLogCommandInfo` mirror (`tclNamesp.c`). A no-op (consuming the flag)
    /// when the command was already logged deeper in the same script; otherwise
    /// it computes the 1-based source line, seeds `errorInfo` from the result on
    /// the first frame, truncates the command to 150 bytes (`...` on overflow),
    /// and sets `already_logged`.
    fn log_command_info(&mut self, src: &[u8], cmd: &parse::Command) {
        // The same guard [`log_command_bytes`] applies. Repeated here so the
        // already-logged path — the common one on a deep unwind — skips the
        // newline scan `line_of` costs.
        if self.exc.borrow().already_logged {
            return;
        }
        self.log_evaluated_command_bytes(line_of(src, cmd.start), &src[cmd.start..cmd.end]);
    }

    /// [`log_command_info`](Self::log_command_info) over the command's bytes
    /// and its 1-based line within the enclosing script.
    ///
    /// The one implementation of C's `TclLogCommandInfo`, shared by the eval
    /// loop and by `tcl_codegen_log_command` — so `error_line` keeps its
    /// single writer, and a compiled statement gets the identical
    /// `already_logged` protocol, error-stack entry, and 150-byte truncation
    /// rather than a second, drifting copy of them.
    pub(crate) fn log_evaluated_command_bytes(&mut self, raw_line: u32, cmd_bytes: &[u8]) {
        self.log_command_bytes(raw_line, cmd_bytes);
        if self
            .native_invocation_dialect()
            .native_error_log_protocol()
            .is_some_and(
                tcl_registry::native_error_log::NativeErrorLogProtocol::evaluation_owns_logged_flag,
            )
        {
            self.exc.borrow_mut().already_logged = true;
        }
    }

    pub(crate) fn log_command_bytes(&mut self, raw_line: u32, cmd_bytes: &[u8]) {
        self.propagate_error_stage();
        if self.uses_jim_error_stack() {
            self.capture_jim_error_stack();
            return;
        }
        // Already logged deeper in the same script (e.g. an inner `[cmd]` subst,
        // or an inline `if`/`while` body): the enclosing command is the same C
        // bytecode frame, so it is *not* re-logged. The flag stays set and is
        // cleared only at a real frame boundary (`make_proc_error` /
        // `append_body_frame`), which is what lets the proc-*call* command log.
        if self.exc.borrow().already_logged {
            return;
        }
        // TIP 348: record this frame's inner context / uplevel boundary into the
        // error stack (the errorStack half of C's `Tcl_LogCommandInfo`).
        self.error_stack_log(cmd_bytes);
        // errorLine, measured against the enclosing `codePtr->source` (C's
        // `TclLogCommandInfo`): the command's file-absolute line
        // (`line_base + 1 + count('\n')`) minus the proc/eval body's base, so an
        // inline `catch`/`if` body's commands still report their proc-body line.
        // This is the *only* writer of `error_line`; it persists across `catch`
        // and a subsequent `error msg info`.
        let line = self.cmd_frames.borrow().last().map_or(raw_line, |f| {
            (f.line_base + raw_line)
                .saturating_sub(f.proc_line_base)
                .max(1)
        });
        self.error_line.set(line);
        let started = self.exc.borrow().info.is_some();
        if !started {
            // First frame: errorInfo is seeded from the error message (the result).
            let msg = self.result_bytes();
            if self
                .native_invocation_dialect()
                .native_error_variable_protocol()
                .is_some()
            {
                let mut exc = self.exc.borrow_mut();
                exc.native.info = Some(obj::Owned::retain(self.result.get()));
                exc.native.info_len = msg.len();
            }
            self.exc.borrow_mut().info = Some(msg);
        }
        let verb: &[u8] = if started {
            b"invoked from within"
        } else {
            b"while executing"
        };
        let overflow = cmd_bytes.len() > 150;
        let slice = if overflow {
            &cmd_bytes[..150]
        } else {
            cmd_bytes
        };
        let mut exc = self.exc.borrow_mut();
        let buf = exc.info.as_mut().expect("seeded above");
        buf.extend_from_slice(b"\n    ");
        buf.extend_from_slice(verb);
        buf.extend_from_slice(b"\n\"");
        buf.extend_from_slice(slice);
        if overflow {
            buf.extend_from_slice(b"...");
        }
        buf.push(b'"');
        exc.already_logged = true;
        drop(exc);
        self.update_native_error_info();
        // Pre-8.5 compatibility (C's `TclLogCommandInfo`, tclNamesp.c): if user
        // code traces `::errorInfo` for writes, push the value out to the variable
        // *now*, mid-unwind — firing the write trace while the failing command's
        // call frame is still live (so the handler's `info level` sees it). Skipped
        // (no var write) when nothing traces `::errorInfo`, so the normal path
        // still publishes once, at the `catch`/top level.
        if !self.uses_c84_global_error_info() && self.errorinfo_has_write_trace() {
            if self
                .native_invocation_dialect()
                .native_error_variable_protocol()
                .is_some()
            {
                self.publish_traced_native_error_info();
            } else {
                let info = self.error_info();
                let ei = new_string(&info);
                if self.var_set(b"::errorInfo", ei).is_err() {
                    drop_fresh(ei);
                }
            }
        }
        if self
            .native_invocation_dialect()
            .native_error_log_protocol()
            .is_some_and(
                tcl_registry::native_error_log::NativeErrorLogProtocol::evaluation_owns_logged_flag,
            )
        {
            // C8.4 Tcl_LogCommandInfo clears this after AddObjErrorInfo and
            // its setters' trace chains. The evaluating command owns the set.
            self.exc.borrow_mut().already_logged = false;
        }
    }

    /// Whether `::errorInfo` carries a user write-trace — C's `TclIsVarTraced`
    /// gate in `TclLogCommandInfo`. Hidden core read/unset registrations
    /// do not qualify as user write observers.
    fn errorinfo_has_write_trace(&self) -> bool {
        if self.traces.borrow().traces.is_empty() {
            return false;
        }
        // Same `(home, simple name)` key registration and firing use — a
        // literal `::errorInfo` here would miss every trace now that traces
        // are keyed by the resolved variable rather than the spelling.
        let home = self.trace_identity(b"::errorInfo");
        let cell = self.variable_trace_scope(&home, None);
        self.traces
            .borrow()
            .traces
            .iter()
            .any(|trace| cell.matches(trace, b"write"))
    }

    /// Append the `(procedure "NAME" line N)` / `(lambda term "..." line N)`
    /// frame when a proc/lambda body unwinds with an error (`MakeProcError` /
    /// `MakeLambdaError`, `tclProc.c`), then clear `already_logged` so the
    /// proc-call command itself is logged by its enclosing eval. The line is the
    /// body-relative `error_line` the innermost body command recorded.
    fn make_proc_error(&mut self, frame: ProcFrame) {
        if self.uses_jim_error_stack() {
            return;
        }
        // `(procedure "NAME" line N)` / `(lambda term "NAME" line N)` — the name
        // quoted, truncated to 60 bytes (`...` on overflow).
        // Append a name quoted and truncated to 60 bytes (`...` on overflow),
        // C's ELLIPSIFY.
        fn push_ellipsified(out: &mut Vec<u8>, name: &[u8]) {
            let overflow = name.len() > 60;
            out.push(b'"');
            out.extend_from_slice(if overflow { &name[..60] } else { name });
            if overflow {
                out.extend_from_slice(b"...");
            }
            out.push(b'"');
        }
        let inner = match frame {
            ProcFrame::Proc(n) | ProcFrame::Lambda(n) => {
                let kind: &[u8] = if matches!(frame, ProcFrame::Lambda(_)) {
                    b"lambda term"
                } else {
                    b"procedure"
                };
                let mut inner = Vec::with_capacity(kind.len() + n.len() + 8);
                inner.extend_from_slice(kind);
                inner.push(b' ');
                push_ellipsified(&mut inner, n);
                inner
            }
            ProcFrame::Method { kind, owner, what } => {
                // `KIND "OWNER" method "NAME"` / `KIND "OWNER" constructor`.
                let mut inner = Vec::new();
                inner.extend_from_slice(kind);
                inner.push(b' ');
                push_ellipsified(&mut inner, owner);
                match what {
                    MethodFrameWhat::Named(name) => {
                        inner.extend_from_slice(b" method ");
                        push_ellipsified(&mut inner, name);
                    }
                    MethodFrameWhat::Constructor => inner.extend_from_slice(b" constructor"),
                    MethodFrameWhat::Destructor => inner.extend_from_slice(b" destructor"),
                }
                inner
            }
        };
        self.append_frame_line(&inner);
        self.exc.borrow_mut().already_logged = false;
    }

    /// Append a `("LABEL" body line N)` frame (the `eval`/`uplevel`/`foreach`
    /// body trace, e.g. `("eval" body line 1)`), then clear `already_logged` so
    /// the enclosing command logs. C emits these for script bodies that evaluate
    /// through a fresh `CmdFrame` (`eval`/`uplevel`/`foreach`), unlike the
    /// inline-compiled `if`/`while`/`for`/`switch`.
    pub(crate) fn append_body_frame(&mut self, label: &[u8]) {
        if self.uses_jim_error_stack() {
            return;
        }
        // The `"label" body` shape: `("eval" body line N)`.
        let mut inner = Vec::with_capacity(label.len() + 8);
        inner.push(b'"');
        inner.extend_from_slice(label);
        inner.extend_from_slice(b"\" body");
        self.append_frame_line(&inner);
        self.exc.borrow_mut().already_logged = false;
    }

    /// Append the `(in namespace eval "<fqn>" script line N)` errorInfo frame
    /// when a `namespace eval` body unwinds with an error (C's `NamespaceEvalCmd`),
    /// then clear `already_logged` so the `namespace eval` command itself logs.
    pub(crate) fn append_namespace_eval_frame(&mut self, fqn: &[u8]) {
        if self.uses_jim_error_stack() {
            return;
        }
        let mut inner = b"in namespace eval \"".to_vec();
        inner.extend_from_slice(fqn);
        inner.extend_from_slice(b"\" script");
        self.append_frame_line(&inner);
        self.exc.borrow_mut().already_logged = false;
    }

    /// Shared tail of the `(... line N)` frames: append `"\n    (<inner> line
    /// <N>)"` to `errorInfo` (seeding it from the result message if no frame has
    /// been logged yet), where `inner` is the caller-built body — e.g.
    /// `procedure "p"`, `lambda term "..."`, or `"eval" body`.
    /// Clear `already_logged` after adding a frame at a real frame boundary
    /// (e.g. an OO definition script), so the enclosing command logs its own
    /// `invoked from within` frame.
    pub(crate) fn clear_error_logged(&self) {
        self.exc.borrow_mut().already_logged = false;
    }

    /// Append the independently selected lambda-parse frame to errorInfo.
    /// This carries no line suffix and clears already_logged so the enclosing
    /// original invocation can still log its own frame.
    pub(crate) fn append_lambda_parse_frame(&mut self, frame: &[u8]) {
        if self.exc.borrow().info.is_none() {
            let msg = self.result_bytes();
            if self
                .native_invocation_dialect()
                .native_error_variable_protocol()
                .is_some()
            {
                let mut exc = self.exc.borrow_mut();
                exc.native.info = Some(obj::Owned::retain(self.result.get()));
                exc.native.info_len = msg.len();
            }
            self.exc.borrow_mut().info = Some(msg);
        }
        {
            let mut exc = self.exc.borrow_mut();
            let buf = exc.info.as_mut().expect("seeded above");
            buf.extend_from_slice(frame);
        }
        self.update_native_error_info();
        self.exc.borrow_mut().already_logged = false;
    }

    pub(crate) fn append_frame_line(&mut self, inner: &[u8]) {
        let line = self.error_line.get();
        if self.exc.borrow().info.is_none() {
            let msg = self.result_bytes();
            if self
                .native_invocation_dialect()
                .native_error_variable_protocol()
                .is_some()
            {
                let mut exc = self.exc.borrow_mut();
                exc.native.info = Some(obj::Owned::retain(self.result.get()));
                exc.native.info_len = msg.len();
            }
            self.exc.borrow_mut().info = Some(msg);
        }
        let mut exc = self.exc.borrow_mut();
        let buf = exc.info.as_mut().expect("seeded above");
        buf.extend_from_slice(b"\n    (");
        buf.extend_from_slice(inner);
        buf.extend_from_slice(b" line ");
        buf.extend_from_slice(line.to_string().as_bytes());
        buf.push(b')');
        drop(exc);
        self.update_native_error_info();
    }

    /// Append a frame with no `line N` suffix — `"\n    (<text>)"`, e.g.
    /// `("for" initial command)` / `("for" loop-end command)` (C's
    /// `Tcl_AddErrorInfo` for the `for` init/next scripts) — seeding errorInfo
    /// from the result if needed, then clear `already_logged` so the enclosing
    /// command logs its own `invoked from within` frame.
    pub(crate) fn append_frame_noline(&mut self, text: &[u8]) {
        if self.exc.borrow().info.is_none() {
            let msg = self.result_bytes();
            if self
                .native_invocation_dialect()
                .native_error_variable_protocol()
                .is_some()
            {
                let mut exc = self.exc.borrow_mut();
                exc.native.info = Some(obj::Owned::retain(self.result.get()));
                exc.native.info_len = msg.len();
            }
            self.exc.borrow_mut().info = Some(msg);
        }
        {
            let mut exc = self.exc.borrow_mut();
            let buf = exc.info.as_mut().expect("seeded above");
            buf.extend_from_slice(b"\n    (");
            buf.extend_from_slice(text);
            buf.push(b')');
        }
        self.update_native_error_info();
        self.exc.borrow_mut().already_logged = false;
    }

    /// Append a literal `"\n    <text>"` errorInfo context line. Ensemble
    /// unknown-handler result diagnostics use this Tcl_AddErrorInfo shape (no
    /// parentheses and no line suffix), then allow the enclosing command to log
    /// its ordinary `invoked from within` frame.
    pub(crate) fn append_error_info_context(&mut self, text: &[u8]) {
        if self.exc.borrow().info.is_none() {
            let msg = self.result_bytes();
            if self
                .native_invocation_dialect()
                .native_error_variable_protocol()
                .is_some()
            {
                let mut exc = self.exc.borrow_mut();
                exc.native.info = Some(obj::Owned::retain(self.result.get()));
                exc.native.info_len = msg.len();
            }
            self.exc.borrow_mut().info = Some(msg);
        }
        {
            let mut exc = self.exc.borrow_mut();
            let buf = exc.info.as_mut().expect("seeded above");
            buf.extend_from_slice(b"\n    ");
            buf.extend_from_slice(text);
        }
        self.update_native_error_info();
        self.exc.borrow_mut().already_logged = false;
    }

    pub(crate) fn uses_jim_error_stack(&self) -> bool {
        self.native_invocation_dialect().error_stack_protocol()
            == Some(NativeErrorStackProtocol::Jim084)
    }

    /// Project borrowed invocation objects only at actual error capture, while
    /// every participating caller still owns its argv. Formatting diagnostics
    /// must not change normal execution's sharing or bytearray purity.
    fn materialized_jim_evaluation_frames(&self) -> Vec<JimEvaluationFrame<obj::Owned>> {
        // naming.source.jim-original-source-entry
        // docs/design/analysis/name-resolution-proofs/jim-original-source-entry.md
        let borrows = self.jim_invocation_borrows.borrow();
        self.jim_evaluation_frames
            .borrow()
            .iter()
            .enumerate()
            .rev()
            .map(|(index, frame)| {
                let invocation = borrows
                    .iter()
                    .rev()
                    .find(|view| view.frame_index == index && !view.argv.is_empty())
                    .map_or_else(
                        || obj::Owned::fresh(new_string(&frame.invocation)),
                        |view| obj::Owned::fresh(self.new_list_object(&view.argv)),
                    );
                JimEvaluationFrame {
                    procedure_level: frame.procedure_level,
                    command_name: frame
                        .command_name
                        .as_ref()
                        .map(|name| obj::Owned::fresh(new_string(name))),
                    is_procedure: frame.is_procedure,
                    script: frame.script.as_ref().map(|script| JimScriptLocation {
                        file: borrows
                            .iter()
                            .rev()
                            .find_map(|view| {
                                (view.frame_index == index)
                                    .then_some(view.script_filename)
                                    .flatten()
                            })
                            .map_or_else(
                                || obj::Owned::fresh(new_string(&script.file)),
                                obj::Owned::retain,
                            ),
                        line: script.line,
                    }),
                    invocation,
                }
            })
            .collect()
    }

    fn jim_trace_object(
        &self,
        frames: Vec<tcl_runtime_api::jim_error_stack::JimErrorFrame<obj::Owned>>,
    ) -> obj::Owned {
        let values: Vec<_> = frames
            .into_iter()
            .flat_map(|frame| {
                [
                    frame.procedure,
                    frame.file,
                    obj::Owned::fresh(obj::new_wide_int_obj(i64::from(frame.line))),
                    frame.invocation,
                ]
            })
            .collect();
        let pointers: Vec<_> = values.iter().map(obj::Owned::as_ptr).collect();
        obj::Owned::fresh(self.new_list_object(&pointers))
    }

    fn capture_jim_error_stack(&self) {
        if self.host_refusal_pending() {
            return;
        }
        self.jim_error_stack.borrow_mut().capture_object(|| {
            let frames = self.materialized_jim_evaluation_frames();
            let empty = obj::Owned::fresh(new_string(b""));
            capture_jim_error_frames(self.jim_procedure_level.get(), &frames, None, &empty)
                .map(|frames| self.jim_trace_object(frames))
        });
    }

    fn capture_jim_script_parse_failure(&self) {
        let script = self
            .cmd_frames
            .borrow()
            .last()
            .map(|frame| JimScriptLocation {
                file: obj::Owned::fresh(new_string(frame.file.as_deref().unwrap_or(b""))),
                line: frame.line_base + 1,
            })
            .unwrap_or_else(|| JimScriptLocation {
                file: obj::Owned::fresh(new_string(b"")),
                line: 1,
            });
        self.jim_error_stack.borrow_mut().capture_object(|| {
            let frames = self.materialized_jim_evaluation_frames();
            let empty = obj::Owned::fresh(new_string(b""));
            capture_jim_error_frames(
                self.jim_procedure_level.get(),
                &frames,
                Some(&script),
                &empty,
            )
            .map(|frames| self.jim_trace_object(frames))
        });
    }

    pub(crate) fn jim_stacktrace_object(&self) -> obj::Owned {
        match self.jim_error_stack.borrow().trace() {
            Some(JimErrorTrace::Explicit(value)) => value.clone(),
            Some(JimErrorTrace::Automatic(frames)) => self.jim_trace_object(frames.clone()),
            None => obj::Owned::fresh(self.new_list_object(&[])),
        }
    }

    pub(crate) fn jim_stacktrace(&self) -> Vec<u8> {
        obj_bytes(self.jim_stacktrace_object().as_ptr())
    }

    pub(crate) fn reset_jim_error_capture(&self) {
        self.jim_error_stack.borrow_mut().mark_reset();
    }

    pub(crate) fn adopt_jim_stacktrace(&self, original: obj::Owned) {
        self.jim_error_stack.borrow_mut().adopt_explicit(original);
    }

    pub(crate) fn jim_return_receipt(
        &self,
    ) -> tcl_runtime_api::jim_return_state::JimReturnReceipt<obj::Owned> {
        tcl_runtime_api::jim_return_state::JimReturnReceipt {
            pending: tcl_runtime_api::jim_return_state::JimReturnState {
                code: self.return_code.get().as_int() as i32,
                level: i64::try_from(self.return_level.get()).unwrap_or(i64::MAX),
            },
            error_code: self.var_get_at(b"::errorCode", 0).map(obj::Owned::retain),
            stack_trace: self.jim_stacktrace_object(),
        }
    }

    /// The current accumulated `errorInfo` (for `catch`'s `-errorinfo`): the
    /// trace if any frame was logged, else the bare error message.
    pub(crate) fn error_info(&self) -> Vec<u8> {
        if self.uses_jim_error_stack() {
            return self.jim_stacktrace();
        }
        let info = self.exc.borrow().info.clone();
        info.unwrap_or_else(|| self.result_bytes())
    }

    /// Capture the error trace state (`errorInfo`/`errorCode` accumulation), so a
    /// command run in a *different* flow can transplant it into this one. Used by
    /// `coroprobe`: the probe runs in the coroutine's (swapped-in) exception
    /// state, but its error must surface in the *caller's* trace once that state
    /// is swapped back out.
    pub(crate) fn snapshot_error(&self) -> ErrorSnapshot {
        let exc = self.exc.borrow();
        ErrorSnapshot {
            native: exc.native.clone(),
            jim_error_stack: self.jim_error_stack.borrow().clone(),
            info: exc.info.clone(),
            code: exc.code.clone(),
            code_explicit: exc.code_explicit,
            primitive_getter: exc.primitive_getter.clone(),
            expression_error_stage: exc.expression_error_stage.clone(),
        }
    }

    /// Restore an [`ErrorSnapshot`] captured by [`snapshot_error`] into this
    /// flow's exception state (the trace continues from there — e.g. `coroprobe`
    /// then appends its own `(injected coroutine probe command)` frame).
    pub(crate) fn restore_error(&self, snap: ErrorSnapshot) {
        *self.jim_error_stack.borrow_mut() = snap.jim_error_stack;
        let mut exc = self.exc.borrow_mut();
        exc.native = snap.native;
        exc.info = snap.info;
        exc.code = snap.code;
        exc.code_explicit = snap.code_explicit;
        exc.primitive_getter = snap.primitive_getter;
        exc.expression_error_stage = snap.expression_error_stage;
    }

    /// `info frame` (no arg): the depth of the source-location stack.
    pub(crate) fn cmd_frame_depth(&self) -> usize {
        self.cmd_frames.borrow().len()
    }

    /// The `info frame N` description for stack position `n` (C's level
    /// arithmetic: `n > 0` is absolute, 1-based from the root; `n <= 0` is
    /// relative to the current top, `0` = current). Returns the dict's
    /// (key, value) pairs in C's key order (`type line [file] cmd [proc]
    /// level`), or `None` if out of range.
    pub(crate) fn cmd_frame_info(
        &mut self,
        n: i64,
    ) -> Result<Option<NativeCommandFrameDescription>, tcl_syntax::value::ValueError> {
        let original = {
            let frames = self.cmd_frames.borrow();
            let depth = frames.len() as i64;
            let pos = if n > 0 { n } else { depth + n };
            if pos < 1 || pos > depth {
                return Ok(None);
            }
            frames[(pos - 1) as usize].original_command.clone()
        };
        let original_bytes = match original {
            Some(value) => Some(self.eval_object_bytes(value.as_ptr())?),
            None => None,
        };
        let cmd_frames = self.cmd_frames.borrow();
        let depth = cmd_frames.len() as i64;
        let pos = if n > 0 { n } else { depth + n };
        if pos < 1 || pos > depth {
            return Ok(None);
        }
        let f = &cmd_frames[(pos - 1) as usize];
        let mut pairs = vec![
            (b"type".to_vec(), f.kind.as_bytes().to_vec()),
            (b"line".to_vec(), f.line.to_string().into_bytes()),
        ];
        if let Some(file) = &f.file {
            pairs.push((b"file".to_vec(), file.to_vec()));
        }
        pairs.push((
            b"cmd".to_vec(),
            original_bytes
                .as_ref()
                .map_or_else(|| f.cmd.clone(), |bytes| bytes.to_vec()),
        ));
        // A TclOO method frame reports `method`/`class`|`object` (the declarer),
        // and an `apply` lambda reports `lambda <expr>`, in place of `proc`
        // (C's `TclInfoFrame`).
        if let Some((method, kind, owner)) = &f.oo {
            if !method.is_empty() {
                pairs.push((b"method".to_vec(), method.clone()));
            }
            pairs.push((kind.clone(), owner.clone()));
        } else if let Some(l) = &f.lambda {
            pairs.push((b"lambda".to_vec(), l.clone()));
        } else if let Some(p) = &f.proc {
            pairs.push((b"proc".to_vec(), p.clone()));
        }
        // `level` is the distance from the current call level — but C only adds
        // it when the frame's CallFrame is *reachable* from the current var frame
        // by walking the caller chain (`TclInfoFrame`). A frame bypassed by an
        // `uplevel` redirection (e.g. the proc that called `uplevel`, when viewed
        // from the uplevel'd callee) is off the chain and omits `level`, even
        // though it shares a level with a chain frame. `omit_level` short-circuits
        // the explicit `uplevel`-body case (C's `framePtr->framePtr == NULL`).
        if !f.omit_level {
            let frames = self.frames.borrow();
            if frames.caller_chain_indices().contains(&f.frame_index) {
                let level = frames.current_level().saturating_sub(f.level);
                pairs.push((b"level".to_vec(), level.to_string().into_bytes()));
            }
        }
        Ok(Some(pairs))
    }

    /// Snapshot the current error/result state (the result bytes + the
    /// `errorInfo`/`errorCode` accumulator) so a side-effecting cleanup (e.g.
    /// running a destructor after a failed constructor) can run and then have
    /// the original error restored.
    pub(crate) fn error_snapshot(&self) -> (Vec<u8>, ExceptionState, JimErrorStack<obj::Owned>) {
        (
            self.result_bytes(),
            self.exc.borrow().clone(),
            self.jim_error_stack.borrow().clone(),
        )
    }

    /// Restore a previously taken [`error_snapshot`](Self::error_snapshot).
    pub(crate) fn error_restore(
        &mut self,
        snap: (Vec<u8>, ExceptionState, JimErrorStack<obj::Owned>),
    ) {
        *self.jim_error_stack.borrow_mut() = snap.2;
        self.set_result_bytes(&snap.0);
        *self.exc.borrow_mut() = snap.1;
    }

    /// The current `errorCode` (for `catch`'s `-errorcode`): the stamped value,
    /// or `NONE`.
    pub(crate) fn error_code(&self) -> Vec<u8> {
        let projected = {
            let exc = self.exc.borrow();
            if let Some(original) = &exc.native.code {
                self.native_invocation_dialect()
                    .native_string_protocol()
                    .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "private error-code string",
                    ))
                    .and_then(|protocol| {
                        crate::dict::native_object_bytes(original.as_ptr(), protocol)
                    })
            } else if exc.code.is_empty() && !exc.code_explicit {
                Ok(b"NONE".to_vec())
            } else {
                Ok(exc.code.clone())
            }
        };
        match projected {
            Ok(bytes) => bytes,
            Err(error) => {
                self.clone().report_cmd_error(error.into());
                Vec::new()
            }
        }
    }

    /// Mark the live error's `-errorcode` as explicitly supplied (so an explicit
    /// empty code is preserved instead of defaulting to `NONE`; error-4.5).
    pub(crate) fn mark_error_code_explicit(&mut self) {
        self.exc.borrow_mut().code_explicit = true;
    }

    /// Record the inner context and actual call-frame role at the reached
    /// command-log operation. The caller owns the already-logged guard.
    pub(crate) fn error_stack_log(&self, command: &[u8]) {
        if self.host_refusal_pending() || !self.runtime_version().has_error_stack() {
            return;
        }
        use tcl_runtime_api::error_stack::ErrorStackFrame;
        let mut es = self.error_stack.borrow_mut();
        let _ = es.begin_inner(b"INNER".to_vec(), command.to_vec());
        let frames = self.frames.borrow();
        let strings = self
            .native_invocation_dialect()
            .native_error_objects_protocol()
            .expect("actual C error stack")
            .strings();
        let frame = match frames.error_stack_frame() {
            Some((Some(delta), _)) => ErrorStackFrame::Redirect(obj::Owned::fresh(
                obj::new_wide_int_obj(i64::try_from(delta).unwrap_or(i64::MAX)),
            )),
            Some((None, words)) => {
                let original = frames.original_error_stack_argv();
                // Native TclOO teardown frames have objc==0. Their original
                // empty vector is an actual unreported frame, not CALL {}.
                if original.as_ref().is_some_and(Vec::is_empty) {
                    return;
                }
                let replacements;
                let words = if let Some(original) = &original {
                    original.as_slice()
                } else {
                    replacements = words
                        .iter()
                        .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
                        .collect::<Vec<_>>();
                    // Legacy host frames have no original C argv capability.
                    // Their byte-authored invocation words are separate producers.
                    let pointers = replacements
                        .iter()
                        .map(obj::Owned::as_ptr)
                        .collect::<Vec<_>>();
                    return self.log_byte_authored_error_frame(&mut es, &pointers, strings);
                };
                ErrorStackFrame::Call(obj::Owned::fresh(crate::list::new_list_obj_native(
                    words, strings,
                )))
            }
            None => ErrorStackFrame::Unreported,
        };
        let _ = es.log_original_frame(frame);
    }

    fn log_byte_authored_error_frame(
        &self,
        stack: &mut NativeErrorStack,
        words: &[*mut TclObj],
        strings: tcl_syntax::native_string::NativeStringProtocol,
    ) {
        let frame = tcl_runtime_api::error_stack::ErrorStackFrame::Call(obj::Owned::fresh(
            crate::list::new_list_obj_native(words, strings),
        ));
        stack.log_original_frame(frame);
    }

    /// Retain the actual private List header without reconstructing its members.
    pub(crate) fn original_error_stack_value(&self) -> obj::Owned {
        self.error_stack.borrow().value()
    }

    /// Copy supplied original members into the selected private List header.
    pub(crate) fn seed_original_error_stack(
        &self,
        original: *mut TclObj,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.error_stack.borrow_mut().adopt_original(original)
    }

    /// The innermost error command's 1-based source line.
    pub(crate) fn error_line(&self) -> u32 {
        self.error_line.get()
    }

    /// Mark the start of a new error episode (C's `iPtr->resetErrorStack = 1`,
    /// set by `Tcl_ResetResult`): the *next* logged command rebuilds the stack.
    /// The current contents are kept until then (so `info errorstack` after a
    /// `catch` still reports the last error).
    pub(crate) fn mark_error_stack_reset(&self) {
        self.error_stack.borrow_mut().mark_reset();
    }

    /// Preserve the reached expression failure record until script propagation.
    pub(crate) fn retain_expression_error_stage(
        &self,
        stage: Box<tcl_registry::native_expression_error::NativeExpressionErrorStage>,
    ) {
        self.exc.borrow_mut().expression_error_stage = Some(stage);
    }

    /// Primitive callers retain their result and state. Script propagation owns
    /// the separate message projection and selected expression error-code update.
    fn propagate_error_stage(&mut self) {
        if self.host_refusal_pending() {
            return;
        }
        let stage = self.exc.borrow_mut().expression_error_stage.take();
        if let Some(stage) = stage {
            if let tcl_registry::native_numeric_error::NativeExpressionErrorCodeUpdate::Set(code) =
                stage.eval_update()
            {
                self.replace_native_error_code(code);
                let mut exc = self.exc.borrow_mut();
                exc.code.clone_from(code);
                exc.code_explicit = false;
            }
        }
        let primitive = self.exc.borrow_mut().primitive_getter.take();
        if let Some(primitive) = primitive {
            let current = self.result_bytes();
            let projected = primitive.eval_result_bytes(&current);
            // An unchanged byte projection preserves the command's original
            // result header and its producer-owned primary representation.
            if projected.len() != current.len() {
                self.set_result_bytes(projected);
            }
        }
    }

    /// Publish and consume the accumulated exception at a native result reset.
    fn publish_error(&mut self) {
        self.propagate_error_stage();
        if self
            .native_invocation_dialect()
            .error_arguments()
            .is_some_and(|protocol| protocol.publishes_tcl_error_globals())
        {
            if self
                .native_invocation_dialect()
                .native_error_variable_protocol()
                .is_some()
            {
                self.publish_native_error_objects();
            } else if !self.uses_c84_global_error_info() || !self.exc.borrow().native.legacy_copy {
                let info = self.error_info();
                let code = self.error_code();
                let ei = new_string(&info);
                if self.var_set(b"::errorInfo", ei).is_err() {
                    drop_fresh(ei);
                }
                let ec = new_string(&code);
                if self.var_set(b"::errorCode", ec).is_err() {
                    drop_fresh(ec);
                }
            }
        }
        *self.exc.borrow_mut() = ExceptionState::default();
        // The exception is consumed: drop any `-during` chain link with it.
        self.clear_during();
        // A new error episode starts fresh: the next logged command rebuilds the
        // TIP 348 error stack (C's `Tcl_ResetResult` sets `resetErrorStack`).
        self.mark_error_stack_reset();
    }

    /// A new public native evaluation resets the active exception episode.
    /// Modern C publishes its retained private originals before releasing them;
    /// C8.4 leaves the existing global headers in their cells.
    fn reset_outermost_native_error(&mut self) {
        if self.eval_depth.get() != 0 || self.host_refusal_pending() {
            return;
        }
        if self.uses_c84_global_error_info() {
            self.reset_native_global_error_episode();
        } else if self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_some()
        {
            self.publish_error();
        }
    }

    /// Publish + reset, for `catch`/`try` once they have captured the options.
    pub(crate) fn publish_and_reset_error(&mut self) {
        if !self.host_refusal_pending() {
            self.publish_error();
        }
    }

    /// Stash the `-during` chain link for the next error-options build (TIP 329
    /// exception chaining): `opts` is the options dict of the exception the
    /// just-thrown handler/`finally` exception supersedes. Takes its own owning
    /// reference (releasing any prior link); the caller keeps its own.
    pub(crate) fn set_during(&self, opts: *mut TclObj) {
        // SAFETY: `opts` is a live object; retain it for the field's own ref.
        unsafe { obj::incr_ref_count(opts) };
        if let Some(old) = self.during.replace(Some(opts)) {
            // SAFETY: drop the previously-held link's owning reference.
            unsafe { obj::decr_ref_count(old) };
        }
    }

    /// Drop the pending `-during` chain link, if any (no error to chain).
    pub(crate) fn clear_during(&self) {
        if let Some(old) = self.during.take() {
            // SAFETY: release the field's owning reference.
            unsafe { obj::decr_ref_count(old) };
        }
    }

    /// The pending `-during` chain link (borrowed), for
    /// [`completion_options`](crate::cmd_error::completion_options) to splice
    /// into an error's options dict. `None` when no chaining is active.
    pub(crate) fn during_opts(&self) -> Option<*mut TclObj> {
        let d = self.during.take();
        self.during.set(d);
        d
    }

    // eval

    /// Evaluate through the common outer boundary, projecting the live
    /// completion before applying its publication tail.
    fn eval_str_boundary<T>(
        &mut self,
        src: &[u8],
        project: impl FnOnce(&mut Self, Code) -> T,
    ) -> T {
        self.reset_outermost_native_error();
        if self.eval_depth.get() == 0 {
            self.clear_return_options();
        }
        let owned = self.cmd_frames.borrow().is_empty().then(CmdFrame::root);
        let code = if self.native_invocation_dialect().native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            // Jim's source entry evaluates an original String through the Script
            // owner. Real token objects retain Source on a returned literal.
            let original = obj::Owned::fresh(obj::new_string_bytes(src));
            let frame = owned.unwrap_or_else(|| self.inherited_cmd_frame());
            if self.codegen_activation_enter() {
                let code = self.eval_native_jim_script(original, frame);
                // The public projector captures the live completion before
                // this boundary publishes its outermost exception state.
                self.eval_depth.set(self.eval_depth.get().saturating_sub(1));
                code
            } else {
                Code::Error
            }
        } else {
            self.eval_script_mode_unpublished(src, owned, false)
        };
        let projected = project(self, code);
        if self
            .native_invocation_dialect()
            .native_eval_object_protocol()
            .is_some_and(|protocol| protocol.clears_public_source_error_logged())
        {
            self.exc.borrow_mut().already_logged = false;
        }
        self.finish_outermost_eval(code);
        projected
    }

    /// Evaluate a whole script; the result is left in the interp result. Returns
    /// the completion code of the last command (or `Ok` for an empty script).
    ///
    /// At the true top level (no `info frame` stack yet) this owns the root
    /// `CmdFrame`; nested calls (command substitution `[cmd]`) run in the
    /// enclosing frame and add none — matching C, where `[cmd]` is the same
    /// `cmdFramePtr` level. A proc body / `eval` / `source` body gets its own
    /// frame via [`eval_framed`](Self::eval_framed).
    pub fn eval_str(&mut self, src: &[u8]) -> Code {
        self.eval_str_boundary(src, |_, code| code)
    }

    /// Evaluate a script and return an owned, byte-preserving completion.
    ///
    /// This is the public host/embedding boundary. The result and live return
    /// options are captured from the live exception. Actual C private error
    /// objects remain owned after this method returns, until a native result
    /// reset or a hidden error-variable read. No text encoding is applied.
    /// Host-only failures return their original typed cause independently of
    /// every guest completion code, including Tcl errors captured by `catch`.
    ///
    /// # Errors
    /// Returns a retained host execution refusal before accessing guest values.
    pub fn eval_completion(
        &mut self,
        script: &[u8],
    ) -> Result<tcl_runtime_api::ScriptCompletion, tcl_runtime_api::NativeExecutionError> {
        let completion = self.eval_str_boundary(script, crate::completion::capture_bytes);
        if let Some(error) = self.native_execution_refusal() {
            return Err(error);
        }
        completion
    }

    /// Evaluate `src` as the body of its own `info frame` level (`frame` is
    /// pushed for the duration). Used by proc calls, `eval`/`uplevel`, and
    /// `source`.
    fn eval_framed(&mut self, src: &[u8], mut frame: CmdFrame) -> Code {
        frame.proc_line_base = frame.line_base;
        self.eval_script(src, Some(frame))
    }

    /// [`Self::eval_framed`] before its outermost publication tail. This is
    /// reserved for a source boundary, which must settle `return` first and may
    /// snapshot the resulting completion before publication.
    fn eval_framed_unpublished(&mut self, src: &[u8], mut frame: CmdFrame) -> Code {
        // A freshly pushed frame is its own `codePtr->source`, so `errorLine` is
        // measured from this body's base (an inline `catch`/`if` body, by
        // contrast, shares the enclosing frame via `eval_shared_located_body` and
        // keeps the proc's `proc_line_base`).
        frame.proc_line_base = frame.line_base;
        self.eval_script_mode_unpublished(src, Some(frame), false)
    }

    /// The shared command loop. If `owned` is `Some`, it is pushed as this
    /// script's `CmdFrame` and updated to each command as the loop steps through
    /// it (so `info frame` sees the live command/line); an unframed call (`None`,
    /// i.e. command substitution) leaves the enclosing frame untouched.
    fn eval_script(&mut self, src: &[u8], owned: Option<CmdFrame>) -> Code {
        self.eval_script_mode(src, owned, false)
    }

    /// [`eval_script`](Self::eval_script) with an explicit `advance_shared` mode:
    /// when `owned` is `None` but `advance_shared` is set, the enclosing frame is
    /// shared (not pushed — no new level) yet its `line`/`cmd` still advance with
    /// each command. This is the command-substitution case (see
    /// [`eval_command_subst`](Self::eval_command_subst)); the caller sets up and
    /// restores the shared frame's `line_base`.
    fn eval_script_mode(
        &mut self,
        src: &[u8],
        owned: Option<CmdFrame>,
        advance_shared: bool,
    ) -> Code {
        let code = self.eval_script_mode_unpublished(src, owned, advance_shared);
        self.finish_outermost_eval(code);
        code
    }

    /// The shared command loop through depth/frame unwind, stopping before the
    /// outermost publication tail. Public result adapters use this seam to
    /// snapshot live return options; ordinary evaluators immediately pass the
    /// code to [`Self::finish_outermost_eval`].
    fn eval_script_mode_unpublished(
        &mut self,
        src: &[u8],
        owned: Option<CmdFrame>,
        advance_shared: bool,
    ) -> Code {
        if !self.evaluation_is_live() {
            return Code::Error;
        }
        // The C object callback has no interpreter argument. Activate the
        // selected engine policy before evaluating; its precision remains
        // shared by interpreters of that engine on this thread.
        if let Some(policy) = self.native_invocation_dialect().double_string_policy() {
            obj::install_double_string_policy(policy);
        }
        let dialect = self.native_invocation_dialect();
        parse::install_native_list_policy(
            dialect.lexer_grammar.list_parse,
            dialect.lexer_grammar.escapes,
        );
        // Native-stack safety net — see `NATIVE_EVAL_DEPTH_LIMIT`'s doc
        // comment. Checked before incrementing / doing any other setup, so
        // bailing out here needs no unwind: `owned` (not yet pushed) simply
        // drops normally.
        if NATIVE_EVAL_DEPTH_LIMIT.exceeded(self.eval_depth.get() + 1) {
            return self.error(b"too many nested evaluations (infinite loop?)");
        }
        if self.eval_depth.get() == 0 {
            self.reset_native_compilation_admission();
        }
        if let Err(failure) = self.enter_native_script(src) {
            return self.admit_native_compilation_error(&failure, None);
        }
        if let Err(code) = self.reset_native_ensemble_rewrite(
            tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent::BytecodeEntry,
        ) {
            self.leave_native_script();
            return code;
        }
        if self.uses_jim_error_stack() {
            self.jim_error_stack.borrow_mut().mark_reset();
        }
        self.eval_depth.set(self.eval_depth.get() + 1);
        let pushed = owned.is_some();
        let owns_frame = pushed || advance_shared;
        if let Some(f) = owned {
            self.cmd_frames.borrow_mut().push(f);
        }
        let mut last = Code::Ok;
        let commands = parse::parse_script_with_config(src, self.lexer_config());
        let failure = (self.native_invocation_dialect().script_parse_timing()
            == Some(tcl_registry::invocation_words::NativeScriptParseTiming::BeforeScript))
        .then(|| {
            commands
                .iter()
                .find_map(|command| parse::first_parse_error(&command.words, self.lexer_config()))
        })
        .flatten();
        if let Some(message) = failure {
            last = self.error(message.as_bytes());
            self.capture_jim_script_parse_failure();
        }
        if commands.is_empty() {
            // A script with no commands (empty / whitespace / comments only)
            // evaluates to the empty result — `Tcl_EvalEx` resets the result at
            // entry, and with nothing to set it the result is empty. Without this
            // a stale prior result leaks through (e.g. an empty proc body, `eval
            // {}`, or an `lmap`/`foreach` body that produces nothing).
            self.set_result_bytes(b"");
        }
        for cmd in &commands {
            if last != Code::Ok {
                break;
            }
            last = self.eval_command(src, cmd, owns_frame);
            if self.host_refusal_pending() {
                last = Code::Error;
            }
            if last != Code::Ok {
                break; // error/return/break/continue propagate up
            }
        }
        if pushed {
            self.cmd_frames.borrow_mut().pop();
        }
        self.eval_depth.set(self.eval_depth.get() - 1);
        self.leave_native_script();
        last
    }

    /// Apply the one outermost-evaluation tail after any result/options
    /// projection has observed the live completion state.
    fn finish_outermost_eval(&mut self, code: Code) {
        if self.host_refusal_pending() || self.eval_depth.get() != 0 {
            return;
        }
        if code == Code::Error {
            if self
                .native_invocation_dialect()
                .native_error_variable_protocol()
                .is_some()
            {
                self.propagate_error_stage();
            } else {
                self.publish_error();
            }
        }
        // Between top-level commands, drain any queued background errors with the
        // current handler — the event loop's behaviour, so errors from one
        // command don't leak into a later command's intercepted handler.
        if !self.bg_queue.borrow().is_empty() {
            self.process_bg_errors();
        }
    }

    /// A `CmdFrame` for an `eval` body, inheriting the current frame's
    /// kind/proc/level/file context (the body runs in the enclosing CallFrame).
    /// `line_base` is the line of the `eval` command minus one — the body opens
    /// on that line, so in a sourced context its commands stay file-absolute
    /// (e.g. `eval` at file line 5 → its body at line 5). `uplevel`/`source`
    /// override fields ([`eval_uplevel`](Self::eval_uplevel)/
    /// [`eval_sourced`](Self::eval_sourced)).
    fn inherited_cmd_frame(&self) -> CmdFrame {
        let line_base = self.current_cmd_line().saturating_sub(1);
        let cmd_frames = self.cmd_frames.borrow();
        let top = cmd_frames.last();
        CmdFrame {
            kind: top.map_or(FrameKind::Eval, |f| f.kind),
            file: top.and_then(|f| f.file.clone()),
            proc: top.and_then(|f| f.proc.clone()),
            level: top.map_or(0, |f| f.level),
            // Inherit the enclosing frame's level-reachability (an inline body
            // of an `uplevel`-redirected script also omits `level`).
            omit_level: top.is_some_and(|f| f.omit_level),
            // An `eval`/body frame runs in the enclosing call frame — inherit its
            // identity so the `info frame` `level` reachability test sees it as
            // the same CallFrame.
            frame_index: top.map_or(0, |f| f.frame_index),
            line_base,
            proc_line_base: line_base,
            cmd: Vec::new(),
            original_command: None,
            line: 1,
            oo: top.and_then(|f| f.oo.clone()),
            lambda: top.and_then(|f| f.lambda.clone()),
        }
    }

    /// Evaluate an inline **control-command body** (`if`/`while`/`for`/`foreach`/
    /// …). A body that is a located literal (TIP 280) runs as its own
    /// line-advancing source frame at the body's file line, so `info frame`
    /// reports the executing command's true line; a pure list dispatches by
    /// element identity; a dynamic body (a script in a variable) runs as its own
    /// `type eval` level with body-relative lines (C's `TclEvalObjEx` always
    /// pushes a cmdframe — `info frame` reports `line 3` for the body's 3rd line,
    /// not the enclosing command's line).
    pub(crate) fn eval_control_body(&mut self, body: *mut TclObj) -> Code {
        let snapshot = match obj::native_object_snapshot(body) {
            Ok(snapshot) => snapshot,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let Some(protocol) = self.eval_object_protocol() else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native control-body object dispatch",
                )
                .into(),
            );
        };
        if protocol.dispatches_list(
            tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
            &snapshot,
        ) {
            let frame = self.unlocated_frame();
            return self.dispatch_list_obj(body, frame);
        }
        // Jim's control workers evaluate the same original object through
        // Script; Source primary fields are independent of C TIP280 locations.
        if self.native_invocation_dialect().native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return self.eval_generic_control_body(body);
        }
        // naming.mathop-procedure-source-controls
        // docs/design/analysis/name-resolution-proofs/mathop-procedure-source-controls.md
        if !self.in_proc()
            && protocol
                .compiles_source(tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody)
        {
            return self.eval_generic_control_body(body);
        }
        if let Some((file, line)) = self.arg_loc(body) {
            // Inside a proc the enclosing command (`while`/`for`/`if`/`foreach`/
            // `try`) is compiled inline, so its literal body runs in the **same**
            // `info frame` level — no new frame, the shared frame's line just
            // advances to each body command (C's bytecode inlining). At the top
            // level (uncompiled command form) the body is evaluated as its own
            // frame (`TclEvalObjEx`), matching tclsh's `info frame` depth.
            if self.in_proc() {
                return self.eval_shared_located_body(body, line);
            }
            let mut frame = self.inherited_cmd_frame();
            frame.kind = FrameKind::Source;
            frame.file = file;
            frame.line_base = line.saturating_sub(1);
            let bytes = obj_bytes(body);
            return self.eval_framed(&bytes, frame);
        }
        self.eval_unlocated_body(&obj_bytes(body))
    }

    /// Evaluate a located-literal control body that is **inlined** into the
    /// enclosing frame (the in-proc case of [`eval_control_body`]). The enclosing
    /// frame is shared (no new `info frame` level); its `line_base` is shifted so
    /// the body's commands report their own file-absolute lines, then restored.
    /// The body literal lives in the same source as the enclosing frame, so the
    /// frame's `kind`/`file` already match — only the line mapping changes.
    fn eval_shared_located_body(&mut self, body: *mut TclObj, line: u32) -> Code {
        let saved = {
            let mut frames = self.cmd_frames.borrow_mut();
            frames.last_mut().map(|top| {
                let saved = (
                    top.line_base,
                    top.line,
                    std::mem::take(&mut top.cmd),
                    top.original_command.take(),
                );
                top.line_base = line.saturating_sub(1);
                saved
            })
        };
        let Some((line_base, line, cmd, original_command)) = saved else {
            // No enclosing frame to share (shouldn't happen under `in_proc`) —
            // fall back to a body-relative eval rather than panic.
            return self.eval_unlocated_body(&obj_bytes(body));
        };
        let bytes = obj_bytes(body);
        let code = self.eval_script_mode(&bytes, None, true);
        if let Some(top) = self.cmd_frames.borrow_mut().last_mut() {
            top.line_base = line_base;
            top.line = line;
            top.cmd = cmd;
            top.original_command = original_command;
        }
        code
    }

    /// The TIP 280 source location recorded for `obj` (the literal-argument
    /// location table), or `None` for a dynamic value. Lets a command that
    /// re-splits a literal (e.g. `switch`'s single-list-arg form) recover the
    /// enclosing file + the list word's line, to derive its sub-bodies' lines.
    pub(crate) fn arg_location(&self, obj: *mut TclObj) -> Option<(Option<Rc<[u8]>>, u32)> {
        self.arg_loc(obj)
    }

    /// Point the current shared frame's `line_base` at `line` (a condition word's
    /// source line) so command substitutions inside an `if`/`while`/`for`
    /// expression report their file-absolute line (TIP 280). Returns the previous
    /// `line_base` to hand back to [`restore_line_base`](Self::restore_line_base).
    #[cfg(have_tommath)]
    pub(crate) fn push_cond_line_base(&self, line: u32) -> Option<u32> {
        self.cmd_frames.borrow_mut().last_mut().map(|top| {
            let old = top.line_base;
            top.line_base = line.saturating_sub(1);
            old
        })
    }

    /// Restore a `line_base` saved by [`push_cond_line_base`](Self::push_cond_line_base).
    #[cfg(have_tommath)]
    pub(crate) fn restore_line_base(&self, old: u32) {
        if let Some(top) = self.cmd_frames.borrow_mut().last_mut() {
            top.line_base = old;
        }
    }

    /// The `(file, line)` of list element `index` within the literal `obj` — for
    /// a body that is a sub-element of a located list literal (an `apply` lambda's
    /// body, C's `TclListLines`). The element's line is the list word's line plus
    /// the newlines preceding the element. `None` when `obj` is a dynamic value
    /// or lacks a file (then the body is body-relative).
    pub(crate) fn list_element_location(
        &self,
        obj: *mut TclObj,
        index: usize,
    ) -> Option<(Rc<[u8]>, u32)> {
        let (Some(file), line) = self.arg_loc(obj)? else {
            return None;
        };
        let nl = scan_list_offsets(&obj_bytes(obj))?.get(index)?.0;
        Some((file, line + nl))
    }

    /// A `type eval`, no-file, body-relative `CmdFrame` (inheriting the enclosing
    /// proc/level) — the frame for a body with no source location (a dynamic
    /// script, or a canonical-list body).
    pub(crate) fn unlocated_frame(&self) -> CmdFrame {
        let mut frame = self.inherited_cmd_frame();
        frame.kind = FrameKind::Eval;
        frame.file = None;
        frame.line_base = 0;
        frame
    }

    /// Evaluate a body whose source location is unknown (C's switch `line = -1`
    /// case: a dynamically-built list body). It runs as `type eval` with **no
    /// file**, so a command defined inside (e.g. `proc`) is body-relative
    /// (`type proc`, line 1) rather than inheriting the enclosing file's lines.
    pub(crate) fn eval_unlocated_body(&mut self, body: &[u8]) -> Code {
        let frame = self.unlocated_frame();
        self.eval_framed(body, frame)
    }

    /// Evaluate a multi-arg `eval`/`uplevel` body (the args were space-joined
    /// into a fresh dynamic script) as its own `info frame` level. Such a body has
    /// no source location, so it is `type eval` with body-relative lines (C's
    /// `TclEvalObjEx` of a non-literal). The errorInfo `("eval" body line N)`
    /// frame is appended separately by the caller.
    pub(crate) fn eval_body(&mut self, script: &[u8]) -> Code {
        self.clear_return_options();
        self.eval_unlocated_body(script)
    }

    /// Evaluate a `[...]` command substitution's inner `script`. A `[cmd]` is
    /// **not** a new `info frame` level (C compiles it into the enclosing
    /// command's bytecode — `info frame` depth is unchanged), but it *does*
    /// advance the reported `line`: a command inside the brackets reports the
    /// line it actually appears on, even when the bracket spans lines or follows
    /// a `\`-newline continuation. `script` is a sub-slice of `src` (it borrows
    /// from the parsed buffer), so its start offset — and thus its file-absolute
    /// line — comes straight from the original source: bs+nl continuations are
    /// real newlines there, so plain newline counting matches C's TIP 280 result
    /// without C's separate continuation-line `adjust` bookkeeping.
    ///
    /// The enclosing frame's location (`line_base`/`line`/`cmd`) is saved and
    /// restored around the inner eval so the rest of the enclosing command keeps
    /// reporting its own line once the substitution returns.
    fn eval_command_subst(&mut self, src: &[u8], script: &[u8]) -> Code {
        let offset = (script.as_ptr() as usize).wrapping_sub(src.as_ptr() as usize);
        let saved = {
            let mut frames = self.cmd_frames.borrow_mut();
            match frames.last_mut() {
                Some(top) if offset <= src.len() => {
                    let saved = (
                        top.line_base,
                        top.line,
                        std::mem::take(&mut top.cmd),
                        top.original_command.take(),
                    );
                    // Shift the body-relative base so the inner script's line 1
                    // maps to the bracket's file-absolute line.
                    top.line_base = (top.line_base + line_of(src, offset)).saturating_sub(1);
                    Some(saved)
                }
                _ => None,
            }
        };
        // Share the enclosing frame but advance its line/cmd through the inner
        // commands (the third eval mode: no new frame, but line tracking on).
        let code = self.eval_script_mode(script, None, saved.is_some());
        if let Some((line_base, line, cmd, original_command)) = saved {
            if let Some(top) = self.cmd_frames.borrow_mut().last_mut() {
                top.line_base = line_base;
                top.line = line;
                top.cmd = cmd;
                top.original_command = original_command;
            }
        }
        code
    }

    /// `eval` of a single body **object** — like [`eval_body`](Self::eval_body),
    /// but a literal obj with a recorded source location (TIP 280 LABC) runs as
    /// `type source` at its original file+line (the test-body case) rather than
    /// `type eval`.
    /// Generic command bodies always enter their original object activation.
    /// Enter an original control-body object at its retained optional source location.
    pub(crate) fn eval_original_control_body_location(
        &mut self,
        original: *mut TclObj,
        location: Option<(Option<Rc<[u8]>>, u32)>,
    ) -> Code {
        let mut frame = self.inherited_cmd_frame();
        match location {
            Some((file, line)) => {
                frame.kind = FrameKind::Source;
                frame.file = file;
                frame.line_base = line.saturating_sub(1);
            }
            None => {
                frame.kind = FrameKind::Eval;
                frame.file = None;
                frame.line_base = 0;
            }
        }
        self.eval_original_body_framed(
            tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
            original,
            frame,
        )
    }

    pub(crate) fn eval_generic_control_body(&mut self, original: *mut TclObj) -> Code {
        let mut frame = self.inherited_cmd_frame();
        match self.arg_loc(original) {
            Some((file, line)) => {
                frame.kind = FrameKind::Source;
                frame.file = file;
                frame.line_base = line.saturating_sub(1);
            }
            None => {
                frame.kind = FrameKind::Eval;
                frame.file = None;
                frame.line_base = 0;
            }
        }
        self.eval_original_body_framed(
            tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
            original,
            frame,
        )
    }

    pub(crate) fn eval_body_obj(&mut self, obj: *mut TclObj) -> Code {
        self.clear_return_options();
        let mut frame = self.inherited_cmd_frame();
        match self.arg_loc(obj) {
            // A located literal body keeps its file+line (`type source`).
            Some((file, line)) => {
                frame.kind = FrameKind::Source;
                frame.file = file;
                frame.line_base = line.saturating_sub(1);
            }
            // A dynamic body (`eval $script`) is `type eval`, body-relative — it
            // does not inherit the enclosing file's lines (C's `TclEvalObjEx`).
            None => {
                frame.kind = FrameKind::Eval;
                frame.file = None;
                frame.line_base = 0;
            }
        }
        self.eval_original_body_framed(
            tcl_registry::native_eval_object::EvalObjectPurpose::Eval,
            obj,
            frame,
        )
    }

    /// Evaluate an original object using its selected native dispatch recipe.
    pub(crate) fn eval_original_body_framed(
        &mut self,
        purpose: tcl_registry::native_eval_object::EvalObjectPurpose,
        original: *mut TclObj,
        frame: CmdFrame,
    ) -> Code {
        let owner = obj::Owned::retain(original);
        let Some(protocol) = self.eval_object_protocol() else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native script-object dispatch",
                )
                .into(),
            );
        };
        let snapshot = match obj::native_object_snapshot(owner.as_ptr()) {
            Ok(snapshot) => snapshot,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let list_dispatch = protocol.dispatches_list(purpose, &snapshot);
        let jim_script = self.native_invocation_dialect().native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084);
        if list_dispatch || jim_script {
            // Original-object drivers enter the same evaluation activation as
            // generated bodies. Their dispatch can recursively evaluate another
            // original object without entering the parsed-source command loop.
            if !self.codegen_activation_enter() {
                return Code::Error;
            }
            let code = if list_dispatch {
                self.dispatch_list_obj(owner.as_ptr(), frame)
            } else {
                self.eval_native_jim_script(owner, frame)
            };
            self.codegen_activation_leave(code);
            return code;
        }
        if protocol.compiles_source(purpose)
            && self.native_invocation_dialect().tcl_version.is_some()
        {
            match self.prepare_original_c_body(owner.as_ptr(), self.current_ns.get(), None) {
                Ok(Some(artifact)) => return self.execute_original_c_body(&artifact, frame),
                Ok(None) => {}
                Err(code) => return code,
            }
        }
        match self.eval_object_bytes(owner.as_ptr()) {
            Ok(bytes) => self.eval_framed(&bytes, frame),
            Err(error) => self.report_cmd_error(error.into()),
        }
    }

    /// Dispatch a pure-list script object as a single command, using its element
    /// objects directly (no stringify/re-parse) — this preserves each element's
    /// `Tcl_Obj` identity, so a nested `eval`/`uplevel $bodyVar` still finds the
    /// body's TIP 280 source location.
    fn dispatch_list_obj(&mut self, original: *mut TclObj, mut frame: CmdFrame) -> Code {
        let owner = obj::Owned::retain(original);
        let Some(backing) = crate::list::native_list_backing(owner.as_ptr()) else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "retained native List backing",
                )
                .into(),
            );
        };
        let elements = match backing.elements() {
            Ok(elements) => elements,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        if elements.is_empty() {
            self.set_result_bytes(b"");
            return Code::Ok;
        }
        frame.original_command = Some(owner);
        frame.cmd.clear();
        frame.line = frame.line_base + 1;
        self.cmd_frames.borrow_mut().push(frame);
        let code = self.dispatch(&elements);
        self.cmd_frames.borrow_mut().pop();
        code
    }

    /// `uplevel` of a single body **object** — the redirected-scope variant of
    /// [`eval_body_obj`](Self::eval_body_obj). A located literal keeps its source
    /// provenance; a dynamic body is `type eval`, no file.
    pub(crate) fn eval_uplevel_obj(&mut self, target_level: usize, obj: *mut TclObj) -> Code {
        self.clear_return_options();
        let loc = self.arg_loc(obj);
        let prev_level = self.frames.borrow_mut().set_active_level(target_level);
        let prev_ns = self.current_ns.get();
        self.current_ns
            .set(self.frames.borrow().frame_ns(target_level));
        let mut frame = self.inherited_cmd_frame();
        frame.level = target_level;
        frame.omit_level = true;
        match loc {
            Some((file, line)) => {
                frame.kind = FrameKind::Source;
                frame.file = file;
                frame.line_base = line.saturating_sub(1);
            }
            None => {
                frame.kind = FrameKind::Eval;
                frame.file = None;
                frame.line_base = 0;
            }
        }
        let code = self.eval_original_body_framed(
            tcl_registry::native_eval_object::EvalObjectPurpose::UpLevel,
            obj,
            frame,
        );
        self.frames.borrow_mut().set_active_level(prev_level);
        self.current_ns.set(prev_ns);
        code
    }

    /// Evaluate one parsed command, then — if it errored — append its
    /// `while executing` / `invoked from within` frame to the error trace
    /// (`TclLogCommandInfo`), using the command's source slice and line.
    fn eval_command(&mut self, src: &[u8], cmd: &parse::Command, owns_frame: bool) -> Code {
        // Update the current `info frame` level to this command — the innermost
        // command executing at this level. A `[cmd]` substitution (and an inline
        // control body) shares the enclosing frame and adds no level, so it
        // updates the `cmd` but keeps the **enclosing** command's `line` (the
        // line the substitution appears on); only the frame-owning script
        // advances the line as it steps through its own commands.
        if let Some(top) = self.cmd_frames.borrow_mut().last_mut() {
            if owns_frame {
                // `line_base` shifts a source-defined proc's body lines to be
                // file-absolute; it is 0 elsewhere (body-relative).
                top.line = top.line_base + line_of(src, cmd.start);
            }
            top.cmd = src[cmd.start..cmd.end].to_vec();
            top.original_command = None;
        }
        if self.uses_jim_error_stack() {
            let script = self
                .cmd_frames
                .borrow()
                .last()
                .map(|frame| JimScriptLocation {
                    file: frame.file.as_deref().unwrap_or(b"").to_vec(),
                    line: frame.line_base + line_of(src, cmd.start),
                });
            self.jim_evaluation_frames
                .borrow_mut()
                .push(JimEvaluationFrame {
                    procedure_level: self.jim_procedure_level.get(),
                    command_name: None,
                    is_procedure: false,
                    script,
                    invocation: Vec::new(),
                });
        }
        let selected = self.native_command_selection(src, cmd);
        let code = self.eval_words(src, &cmd.words, selected);
        if code == Code::Error {
            self.log_command_info(src, cmd);
        }
        if self.uses_jim_error_stack() {
            self.jim_evaluation_frames.borrow_mut().pop();
        }
        code
    }

    /// Substitute each word of a command (with `{*}` expansion), then dispatch.
    fn eval_words(
        &mut self,
        src: &[u8],
        words: &[parse::Word],
        selected: Option<Box<native_compilation::SelectedInvocation>>,
    ) -> Code {
        // C parses a command WHOLE before it substitutes any of it
        // (`Tcl_EvalEx` → `Tcl_ParseCommand` → `TclEvalObjvInternal`), so a
        // parse failure in a later word — or inside a later word's `[…]` —
        // stops an earlier word's command substitution from ever running.
        // Measured on 8.6.16 and 9.0.4: `list [sfx inner] {a}b` raises `extra
        // characters after close-brace` with `sfx` never called. Walking the
        // words in order and raising at the first `WordPart::ParseError` (this
        // engine's carrier for those failures — the scanner stays infallible so
        // the LSP can keep tokenising) matches that: substituting word by word
        // without this check would run `sfx` before the later parse error is
        // seen.
        if let Some(msg) = parse::first_parse_error(words, self.lexer_config()) {
            return self.error(msg.as_bytes());
        }
        // The command's reported line + source file (for TIP 280 argument-line
        // tracking and the LABC literal-location table), and the first word's
        // offset (to measure each word's line within the command).
        let cmd_line = self.cmd_frames.borrow().last().map_or(0, |f| f.line);
        let file = self.cmd_frames.borrow().last().and_then(|f| f.file.clone());
        let w0 = words.first().map_or(0, |w| w.start);

        let mut argv: Vec<*mut TclObj> = Vec::new();
        // Per-**argv-element** file-absolute line (aligned with `argv`, so `{*}`
        // expansion stays correct), and the argv indices of literal arguments to
        // record in the LABC table (`eval`/`uplevel`/proc body source locations).
        let mut arg_lines: Vec<u32> = Vec::new();
        let mut labc: Vec<usize> = Vec::new();

        for w in words {
            let word_line = cmd_line + count_newlines(&src[w0..w.start.min(src.len())]);
            let is_literal = matches!(w.body, parse::WordBody::Literal(_));
            self.begin_native_arguments();
            let substituted = self.substitute_word(src, &w.body);
            self.end_native_arguments();
            let obj = match substituted {
                Ok(o) => o, // owned (+1)
                Err(code) => {
                    release_all(&argv);
                    return code;
                }
            };
            let obj = if w.kind == parse::WordKind::Quoted
                && self
                    .native_invocation_dialect()
                    .quoted_substitution_preserves_object()
                    == Some(false)
                && matches!(&w.body, WordBody::Parts(parts) if parts.len() == 1
                    && matches!(&parts[0], WordPart::Variable(_) | WordPart::Command(_) | WordPart::Expression(_)))
            {
                let copy = new_string(&obj_bytes(obj));
                // SAFETY: the substituted object is owned; the new string has
                // independent bytes and starts its own native count recipe.
                unsafe {
                    obj::incr_ref_count(copy);
                    obj::decr_ref_count(obj);
                }
                copy
            } else {
                obj
            };
            if w.expand {
                // Expand the live list objects directly. Stringifying and
                // reparsing would lose numeric/byte representations and reject
                // raw Jim strings already held in valid list elements.
                let elems = match crate::list::list_elements(obj) {
                    Ok(e) => e,
                    Err(e) => {
                        unsafe { obj::decr_ref_count(obj) };
                        release_all(&argv);
                        return self.report_cmd_error(e.into());
                    }
                };
                // For a `{*}` of a *literal* word, each element keeps its source
                // line (its offset within the literal — C's `TclListLines`), so a
                // body-defining element (`namespace {*}{eval ns {proc …}}`) is
                // `type source`. A dynamic `{*}$v` has no per-element location.
                let offsets = is_literal
                    .then(|| scan_list_offsets(&obj_bytes(obj)))
                    .flatten();
                for (k, &eo) in elems.iter().enumerate() {
                    unsafe { obj::incr_ref_count(eo) };
                    let idx = argv.len();
                    argv.push(eo);
                    let (nl, lit) = offsets
                        .as_ref()
                        .and_then(|o| o.get(k))
                        .copied()
                        .unwrap_or((0, false));
                    arg_lines.push(word_line + nl);
                    if file.is_some() && lit {
                        labc.push(idx);
                    }
                }
                unsafe { obj::decr_ref_count(obj) };
            } else {
                let idx = argv.len();
                argv.push(obj); // already owned (+1)
                arg_lines.push(word_line);
                if file.is_some() && is_literal {
                    labc.push(idx);
                }
            }
        }

        if argv.is_empty() {
            // TclEvalObjvInternal's empty expanded argv is a successful empty
            // command. It still resets the prior interpreter result, just as
            // the ordinary dispatch boundary below does.
            self.set_result_bytes(b"");
            return Code::Ok;
        }

        // TIP 280 LABC: record each literal argument's obj → (file, line) so a
        // later `eval`/`uplevel`/`proc`-body of that obj reports `type source`.
        // Popped after dispatch (dynamic scope; the objs live until then).
        let pushed = labc.len();
        if pushed > 0 {
            let mut locs = self.arg_locs.borrow_mut();
            for &idx in &labc {
                locs.push((argv[idx], file.clone(), arg_lines[idx]));
                if self.uses_jim_error_stack() {
                    obj::retain_script_location(argv[idx], file.clone(), arg_lines[idx]);
                }
            }
        }
        *self.arg_lines.borrow_mut() = arg_lines;

        let code = self.dispatch_native_selection(selected, &argv);
        if pushed > 0 {
            let mut locs = self.arg_locs.borrow_mut();
            let keep = locs.len() - pushed;
            locs.truncate(keep);
        }
        // Safe to release argv now: a command that made an argv element its
        // result did so via set_obj_result, which holds an independent +1.
        release_all(&argv);
        code
    }

    /// The recorded TIP 280 source location of a script obj (C's `lineLABCPtr`
    /// lookup), or `None` for a dynamic/computed script. Scans newest-first.
    fn arg_loc(&self, obj: *mut TclObj) -> Option<(Option<Rc<[u8]>>, u32)> {
        if let Some(location) = self
            .uses_jim_error_stack()
            .then(|| obj::script_location(obj))
            .flatten()
        {
            return Some(location);
        }
        self.arg_locs
            .borrow()
            .iter()
            .rev()
            .find(|(o, _, _)| *o == obj)
            .map(|(_, f, l)| (f.clone(), *l))
    }

    /// Look up `argv[0]` and invoke it; on a miss, fall to the `unknown` handler
    /// (auto-load / `package` / friendly errors — the pure-Tcl `unknown` proc),
    /// matching C's `TclEvalObjvInternal`.
    pub(crate) fn dispatch(&mut self, argv: &[*mut TclObj]) -> Code {
        self.dispatch_prebound_with_entry(argv, None, true)
    }

    /// Internal forwarding retains the current ensemble rewrite until its
    /// target evaluates an ordinary command or bytecode body.
    pub(crate) fn dispatch_invoke(&mut self, argv: &[*mut TclObj]) -> Code {
        self.dispatch_prebound(argv, None)
    }

    /// Forward original words through an explicit command lookup namespace,
    /// retaining the caller's variable frame and the real dispatch boundary.
    pub(crate) fn dispatch_in_lookup_namespace(
        &mut self,
        lookup: NsId,
        argv: &[*mut TclObj],
    ) -> Code {
        self.dispatch_selection_with_entry(
            argv,
            CommandDispatchSelection::LookupAt(
                lookup,
                tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
            ),
            false,
        )
    }

    /// Invoke the exact command token named by a shared-runtime `CommandId`.
    /// IDs retain a generation as well as an FQN, so a recreated same-named
    /// namespace command cannot capture a previously resolved handle.
    pub(crate) fn dispatch_command_id(&mut self, id: u32, argv: &[*mut TclObj]) -> Option<Code> {
        let (_, generation) = self.command_identity(id)?;
        let (fqn, command) = match self.command_by_generation(generation) {
            CommandGenerationLookup::Found { fqn, command } => (fqn, command),
            CommandGenerationLookup::Missing | CommandGenerationLookup::Unavailable => {
                return None;
            }
        };
        let mut full = Vec::with_capacity(argv.len() + 1);
        full.push(new_string(&fqn));
        full.extend_from_slice(argv);
        for &word in &full {
            // SAFETY: the head is fresh and every remaining word is borrowed
            // live from the caller for this dispatch.
            unsafe { obj::incr_ref_count(word) };
        }
        let code = self.dispatch_prebound(
            &full,
            Some((
                fqn,
                CommandBinding {
                    generation,
                    command,
                    jim_table_key: None,
                },
            )),
        );
        release_all(&full);
        Some(code)
    }

    /// The central dispatch boundary, optionally with a command binding whose
    /// table is not visible to normal namespace lookup (`interp invokehidden`).
    /// Pre-resolved bindings still pass through command counting and execution
    /// traces; their stored generation selects the exact trace sidecar.
    fn dispatch_prebound(
        &mut self,
        argv: &[*mut TclObj],
        prebound: Option<(Vec<u8>, CommandBinding)>,
    ) -> Code {
        self.dispatch_prebound_with_entry(argv, prebound, false)
    }

    fn dispatch_prebound_with_entry(
        &mut self,
        argv: &[*mut TclObj],
        prebound: Option<(Vec<u8>, CommandBinding)>,
        ordinary: bool,
    ) -> Code {
        self.dispatch_selection_with_entry(argv, prebound.into(), ordinary)
    }

    fn dispatch_selection_with_entry(
        &mut self,
        argv: &[*mut TclObj],
        prebound: CommandDispatchSelection,
        ordinary: bool,
    ) -> Code {
        let previous = self.native_dispatch_depth.get();
        let depth = previous.saturating_add(1);
        // Unknown handlers and other forwarding entries add real recursive
        // dispatch frames between script-body activations. Bound this shared
        // entry before lookup/rewrite effects, using the existing native-stack
        // budget independently of the body evaluator's depth.
        if NATIVE_EVAL_DEPTH_LIMIT.exceeded(depth) {
            return self.error(b"too many nested evaluations (infinite loop?)");
        }
        if ordinary {
            if let Err(code) = self.reset_native_ensemble_rewrite(
                tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent::BeforeOrdinaryLookup,
            ) {
                return code;
            }
        }
        self.native_dispatch_depth.set(depth);
        let code = self.dispatch_prebound_inner(argv, prebound, ordinary);
        let code = self.drain_tailcalls(depth, code);
        self.native_dispatch_depth.set(previous);
        code
    }

    fn dispatch_prebound_inner(
        &mut self,
        argv: &[*mut TclObj],
        prebound: CommandDispatchSelection,
        ordinary: bool,
    ) -> Code {
        if !self.evaluation_is_live() {
            return Code::Error;
        }
        // TclEvalObjvInternal resets the interpreter result before execution
        // traces and command dispatch. This is the central entry used by parsed
        // commands, canonical-list eval, aliases/ensembles, callbacks and the
        // native ABI. `argv` owns/borrows its objects independently, so dropping
        // the prior result here cannot invalidate an argument.
        self.set_result_bytes(b"");
        self.reset_native_global_error_episode();
        self.cmd_count.set(self.cmd_count.get() + 1);
        // An embedder's limits are charged here, where every command passes:
        // past one, the command fails instead of running.
        if let Some(code) = self.charge_dispatch() {
            return code;
        }
        // Fast path: nothing is registered, so nothing can fire. Being inside a
        // trace callback is *not* a reason to skip: C's
        // `TclCheckExecutionTraces` never consults `INTERP_TRACE_IN_PROGRESS`,
        // so a command dispatched from a callback still fires its own
        // `enter`/`leave` traces. Only the step machinery is gated, in
        // `dispatch_traced`.
        let traced = !self.traces.borrow().cmd_traces.is_empty();
        if !traced {
            return match prebound {
                CommandDispatchSelection::Bound(_, binding) => {
                    if ordinary {
                        if let Err(code) = self.reset_native_ensemble_rewrite(
                            tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent::AfterSuccessfulOrdinaryLookup,
                        ) { return code; }
                    }
                    self.invoke_bound(binding.command, Some(binding.generation), argv)
                }
                CommandDispatchSelection::Unselected => self.dispatch_inner(argv, ordinary),
                CommandDispatchSelection::LookupAt(lookup, origin) => {
                    self.dispatch_inner_at(argv, ordinary, lookup, origin)
                }
                CommandDispatchSelection::Missing {
                    lookup,
                    caller,
                    origin,
                } => self.dispatch_missing_command(argv, lookup, caller, origin),
            };
        }
        self.dispatch_traced(argv, prebound, ordinary)
    }

    /// Slow path: the command may carry execution (enter/leave/step) traces, or
    /// a step trace is active. Mirrors C's `TclEvalObjvInternal` order: interp
    /// (step) enter traces fire before per-command enter; per-command leave
    /// fires before interp (step) leave.
    fn dispatch_traced(
        &mut self,
        argv: &[*mut TclObj],
        prebound: CommandDispatchSelection,
        ordinary: bool,
    ) -> Code {
        use crate::cmd_trace::ops;
        // Inside a rename's callbacks the vacating name still resolves, but the
        // one command's trace list has already moved to the destination key —
        // so look the traces up there, as C reaches them through the shared
        // `Command` from either hash entry. Only the *key* is canonicalised:
        // the callback's own words stay the spelling the caller invoked
        // (`cmd_word` below), which is what tclsh passes.
        let lookup_context = match &prebound {
            CommandDispatchSelection::LookupAt(lookup, origin) => Some((*lookup, *origin)),
            _ => None,
        };
        let mut missing = match &prebound {
            CommandDispatchSelection::Missing {
                lookup,
                caller,
                origin,
            } => Some((*lookup, *caller, *origin)),
            _ => None,
        };
        let (fqn, token, prebound_command) = match prebound {
            CommandDispatchSelection::Bound(fqn, binding) => {
                if ordinary {
                    if let Err(code) = self.reset_native_ensemble_rewrite(
                        tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent::AfterSuccessfulOrdinaryLookup,
                    ) { return code; }
                }
                (Some(fqn), Some(binding.generation), Some(binding.command))
            }
            CommandDispatchSelection::Unselected | CommandDispatchSelection::LookupAt(..) => {
                let lookup = lookup_context.map_or(self.current_ns.get(), |(lookup, _)| lookup);
                let selected = match self.resolve_original_command_at(lookup, argv[0]) {
                    Ok(selected) => selected,
                    Err(error) => return self.report_cmd_error(error.into()),
                };
                if selected.is_none() {
                    if let Some((lookup, origin)) = lookup_context {
                        missing = Some((lookup, self.current_ns.get(), origin));
                    }
                }
                if ordinary && selected.is_some() {
                    if let Err(code) = self.reset_native_ensemble_rewrite(
                        tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent::AfterSuccessfulOrdinaryLookup,
                    ) { return code; }
                }
                let token = selected.and_then(|(_, generation)| generation);
                let fqn = token
                    .and_then(|generation| {
                        self.namespaces
                            .borrow()
                            .native_command_at_node(generation)
                            .map(|(_, fqn)| fqn)
                    })
                    .map(|fqn| self.renamed_cmd_key(&fqn).unwrap_or(fqn));
                (fqn, token, None)
            }
            CommandDispatchSelection::Missing { .. } => (None, None, None),
        };
        let (has_enter, has_leave, has_step) = match &fqn {
            Some(f) => {
                let t = self.traces.borrow();
                let (mut he, mut hl, mut hs) = (false, false, false);
                for tr in t
                    .cmd_traces
                    .iter()
                    .filter(|tr| tr.name == *f && tr.token == token)
                {
                    he |= (tr.ops & ops::ENTER) != 0;
                    hl |= (tr.ops & ops::LEAVE) != 0;
                    hs |= (tr.ops & ops::STEP_ANY) != 0;
                }
                (he, hl, hs)
            }
            None => (false, false, false),
        };
        // C's one read of `INTERP_TRACE_IN_PROGRESS`: `TclCheckInterpTraces`
        // returns immediately while an execution callback is running, so the
        // callback's own commands are never step-observed.
        let stepping = {
            let t = self.traces.borrow();
            !t.step_active.is_empty() && t.exec_firing == 0
        };
        if !has_enter && !has_leave && !has_step && !stepping {
            return match prebound_command {
                Some(command) => self.invoke_bound(command, token, argv),
                None => match missing {
                    Some((lookup, caller, origin)) => {
                        self.dispatch_missing_command(argv, lookup, caller, origin)
                    }
                    None => match lookup_context {
                        Some((lookup, origin)) => {
                            self.dispatch_inner_at(argv, ordinary, lookup, origin)
                        }
                        None => self.dispatch_inner(argv, ordinary),
                    },
                },
            };
        }
        // The `{cmd arg ...}` word: argv rendered as a single list element (C's
        // `TraceExecutionProc` builds it via per-arg `DStringAppendElement`).
        let cmd_word = {
            let lst = self.new_list_object(argv);
            let bytes = obj_bytes(lst);
            drop_fresh(lst);
            bytes
        };
        // (A) enterstep for active step traces (interp traces fire on enter
        // before per-command enter); a non-OK enterstep aborts the command.
        if stepping {
            if let Some(c) = self.fire_step(&cmd_word, ops::ENTERSTEP, None) {
                return c;
            }
        }
        // (B) per-command enter; a non-OK enter aborts with the callback result.
        if has_enter {
            if let Some(c) = self.fire_exec_enter(fqn.as_deref().unwrap(), token, &cmd_word) {
                return c;
            }
        }
        // (C) install this command's step traces (deduped against recursion).
        let installed = if has_step {
            self.install_step_traces(fqn.as_deref().unwrap(), token)
        } else {
            0
        };
        let mut code = match prebound_command {
            Some(command) => self.invoke_bound(command, token, argv),
            None => match missing {
                Some((lookup, caller, origin)) => {
                    self.dispatch_missing_command(argv, lookup, caller, origin)
                }
                None => self.dispatch_inner(argv, ordinary),
            },
        };
        // (D) remove the step traces installed above (they are the last pushed).
        if installed > 0 {
            self.remove_installed_step_traces(installed);
        }
        if self.host_refusal_pending() {
            return Code::Error;
        }
        // (E) per-command leave (before interp/step leave), then (F) leavestep.
        if has_leave {
            code = self.fire_exec_leave(fqn.as_deref().unwrap(), token, &cmd_word, code);
        }
        if stepping {
            if let Some(c) = self.fire_step(&cmd_word, ops::LEAVESTEP, Some(code)) {
                code = c;
            }
        }
        code
    }

    /// Push a `StepActive` for each step trace on `fqn` not already live (dedup
    /// by owner+prefix handles recursion: only the outermost installs). Returns
    /// how many were pushed (the last `n` of `step_active`, popped on exit).
    fn install_step_traces(&mut self, fqn: &[u8], token: Option<u64>) -> usize {
        use crate::cmd_trace::{StepActive, ops};
        let to_install: Vec<(u8, Vec<u8>)> = {
            let t = self.traces.borrow();
            t.cmd_traces
                .iter()
                .filter(|c| c.name == fqn && c.token == token && (c.ops & ops::STEP_ANY) != 0)
                .filter(|c| {
                    !t.step_active
                        .iter()
                        .any(|s| s.owner == fqn && s.token == token && s.command == c.command)
                })
                .map(|c| (c.ops & ops::STEP_ANY, c.command.clone()))
                .collect()
        };
        let n = to_install.len();
        let mut tt = self.traces.borrow_mut();
        for (ops_bits, command) in to_install {
            tt.step_active.push(StepActive {
                owner: fqn.to_vec(),
                token,
                ops: ops_bits,
                command,
            });
        }
        n
    }

    /// Pop the `n` step traces this command installed (balanced nesting keeps
    /// them at the end: any nested step-traced command popped its own first).
    fn remove_installed_step_traces(&mut self, n: usize) {
        let mut tt = self.traces.borrow_mut();
        let keep = tt.step_active.len() - n;
        tt.step_active.truncate(keep);
    }

    /// Fire active step traces for the current command. `ENTERSTEP` fires in
    /// reverse install order with `<prefix> {cmd args} enterstep` (a non-OK code
    /// aborts); `LEAVESTEP` fires in install order with `<prefix> {cmd args}
    /// <code> <result> leavestep` (a non-OK code overrides). The result is saved
    /// once and restored after, but live between callbacks (C's interp-trace
    /// `SaveInterpState`/`RestoreInterpState`).
    fn fire_step(&mut self, cmd_word: &[u8], op_bit: u8, code: Option<Code>) -> Option<Code> {
        use crate::cmd_trace::ops;
        let is_enter = op_bit == ops::ENTERSTEP;
        let mut cmds: Vec<Vec<u8>> = self
            .traces
            .borrow()
            .step_active
            .iter()
            .filter(|s| (s.ops & op_bit) != 0)
            .map(|s| s.command.clone())
            .collect();
        if is_enter {
            cmds.reverse();
        }
        if cmds.is_empty() {
            return None;
        }
        let saved = self.result.get();
        unsafe { obj::incr_ref_count(saved) };
        let saved_options = self.take_return_options();
        let code_str = code.map(|c| c.as_int().to_string().into_bytes());
        let op_label: &[u8] = if is_enter { b"enterstep" } else { b"leavestep" };

        self.traces.borrow_mut().exec_firing += 1;
        let mut outcome: Option<Code> = None;
        for cmd in cmds {
            let args = if is_enter {
                self.new_list_object(&[new_string(cmd_word), new_string(op_label)])
            } else {
                let result_bytes = obj_bytes(self.result.get());
                self.new_list_object(&[
                    new_string(cmd_word),
                    new_string(code_str.as_deref().unwrap_or(b"0")),
                    new_string(&result_bytes),
                    new_string(op_label),
                ])
            };
            let mut line = cmd;
            line.push(b' ');
            line.extend_from_slice(&obj_bytes(args));
            drop_fresh(args);
            self.clear_return_options();
            let c = self.eval_native_command_trace_script(true, &line);
            if c != Code::Ok {
                outcome = Some(c);
                break;
            }
        }
        self.traces.borrow_mut().exec_firing -= 1;

        match outcome {
            Some(c) => {
                unsafe { obj::decr_ref_count(saved) };
                Some(c)
            }
            None => {
                unsafe {
                    obj::decr_ref_count(self.result.get());
                    self.result.set(saved);
                }
                self.restore_return_options(saved_options);
                None
            }
        }
    }

    /// The original resolve→invoke→unknown dispatch (trace-free).
    fn dispatch_inner(&mut self, argv: &[*mut TclObj], ordinary: bool) -> Code {
        self.dispatch_inner_at(
            argv,
            ordinary,
            self.current_ns.get(),
            tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
        )
    }

    fn dispatch_inner_at(
        &mut self,
        argv: &[*mut TclObj],
        ordinary: bool,
        lookup: NsId,
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    ) -> Code {
        // The availability gate lives in `resolve_dispatchable`, so a builtin
        // the emulated release does not carry misses here and falls through to
        // the `unknown` machinery below like any other unresolved name.
        match self.resolve_original_command_at(lookup, argv[0]) {
            Ok(Some((cmd, generation))) => {
                if ordinary {
                    if let Err(code) = self.reset_native_ensemble_rewrite(
                        tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent::AfterSuccessfulOrdinaryLookup,
                    ) { return code; }
                }
                return self.invoke_bound(cmd, generation, argv);
            }
            Ok(None) => {}
            Err(error) => return self.report_cmd_error(error.into()),
        }
        self.dispatch_missing_command(
            argv,
            lookup,
            self.current_ns.get(),
            if origin != tcl_registry::command_lookup::CommandLookupOrigin::Ordinary {
                origin
            } else if ordinary {
                tcl_registry::command_lookup::CommandLookupOrigin::Ordinary
            } else {
                tcl_registry::command_lookup::CommandLookupOrigin::AliasInvocation
            },
        )
    }

    /// Return the original stored root, installing the native global default
    /// lazily. Namespace storage owns the root independently from the result.
    pub(crate) fn namespace_unknown_root(
        &mut self,
        namespace: NsId,
        default: bool,
    ) -> Option<obj::Owned> {
        let stored = self.namespaces.borrow().unknown_handler(namespace);
        if let Some(stored) = stored {
            return Some(obj::Owned::retain(stored));
        }
        if !default {
            return None;
        }
        let root = obj::Owned::fresh(new_string(b"::unknown"));
        let retired = self
            .namespaces
            .borrow_mut()
            .set_unknown_handler(namespace, Some(root.clone()));
        drop(retired);
        Some(root)
    }

    fn retire_namespace_unknown_root(&mut self, namespace: NsId) {
        let retired = self
            .namespaces
            .borrow_mut()
            .set_unknown_handler(namespace, None);
        drop(retired);
    }

    fn prepare_missing_command(
        &mut self,
        argv: &[*mut TclObj],
        lookup: NsId,
        caller: NsId,
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    ) -> Result<Box<PreparedMissingCommand>, Code> {
        use tcl_registry::command_lookup::{
            UnknownHandlerNamespace, native_lookup_fallback_policy,
        };
        use tcl_syntax::value::ValueOps;
        let Some(policy) = native_lookup_fallback_policy(self.native_invocation_dialect(), origin)
        else {
            return Err(self.invalid_original_command(argv[0]));
        };
        let selected = match policy.namespace_handler {
            Some(UnknownHandlerNamespace::Caller) => caller,
            Some(UnknownHandlerNamespace::Lookup) => lookup,
            None => GLOBAL,
        };
        let root = if policy.namespace_handler.is_some() {
            self.namespace_unknown_root(selected, selected == GLOBAL)
                .or_else(|| self.namespace_unknown_root(GLOBAL, true))
                .expect("the native global default is installed")
        } else {
            obj::Owned::fresh(new_string(policy.default_handler.as_bytes()))
        };
        let prefix = match self.list_elements(&root.as_ptr()) {
            Ok(prefix) => prefix,
            Err(error) => return Err(self.report_cmd_error(error.into())),
        };
        let owned: Vec<_> = prefix
            .iter()
            .chain(argv)
            .map(|word| obj::Owned::retain(*word))
            .collect();
        drop(root);
        let Some(head) = owned.first() else {
            return Err(self.invalid_original_command(argv[0]));
        };
        let selected = match self.resolve_original_command_at(lookup, head.as_ptr()) {
            Ok(Some(selected)) => selected,
            Ok(None) => return Err(self.invalid_original_command(argv[0])),
            Err(error) => return Err(self.report_cmd_error(error.into())),
        };
        self.reset_native_ensemble_rewrite(
            tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent::AfterSuccessfulOrdinaryLookup,
        )?;
        let fqn = selected.1.and_then(|generation| {
            self.namespaces
                .borrow()
                .native_command_at_node(generation)
                .map(|(_, fqn)| fqn)
        });
        let words = owned.iter().map(obj::Owned::as_ptr).collect();
        Ok(Box::new(PreparedMissingCommand {
            owned,
            words,
            selected: Some(selected),
            fqn,
        }))
    }

    fn dispatch_missing_command(
        &mut self,
        argv: &[*mut TclObj],
        lookup: NsId,
        caller: NsId,
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    ) -> Code {
        let mut prepared = match self.prepare_missing_command(argv, lookup, caller, origin) {
            Ok(prepared) => prepared,
            Err(code) => return code,
        };
        let previous_namespace = self.current_ns.replace(lookup);
        let previous_frame_namespace = self.frames.borrow_mut().replace_active_namespace(lookup);
        let (command, generation) = prepared.selected.take().expect("prepared handler");
        let code = match (prepared.fqn.take(), generation) {
            (Some(fqn), Some(generation)) => self.dispatch_prebound(
                &prepared.words,
                Some((
                    fqn,
                    CommandBinding {
                        generation,
                        command,
                        jim_table_key: None,
                    },
                )),
            ),
            _ => self.invoke_bound(command, generation, &prepared.words),
        };
        self.frames
            .borrow_mut()
            .replace_active_namespace(previous_frame_namespace);
        self.current_ns.set(previous_namespace);
        // Keep actual prefix members through the reached invocation, then free
        // them after callback replacement/deletion has released the stored root.
        prepared.owned.clear();
        code
    }

    fn invoke_bound(
        &mut self,
        cmd: Command,
        generation: Option<u64>,
        argv: &[*mut TclObj],
    ) -> Code {
        if self.host_refusal_pending() {
            return Code::Error;
        }
        if let Err(error) = self.associate_native_jim_arguments(argv) {
            return self.report_cmd_error(error.into());
        }
        if self.uses_jim_error_stack() {
            let name = generation
                .and_then(|id| {
                    self.raw_command_location_by_generation(id)
                        .map(|(name, _)| name)
                })
                .or_else(|| {
                    argv.first()
                        .and_then(|word| self.resolve_cmd_fqn(&obj_bytes(*word)))
                });
            let mut frames = self.jim_evaluation_frames.borrow_mut();
            if let Some(frame) = frames.last_mut() {
                frame.command_name =
                    name.map(|name| name.strip_prefix(b"::").unwrap_or(&name).to_vec());
                frame.is_procedure = matches!(cmd, Command::Proc(_));
            }
        }
        let _jim_invocation = self.borrow_jim_invocation(argv);
        if self.native_invocation_dialect().native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            let context = match self.native_jim_object_context() {
                Ok(context) => context,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            self.set_result(context.empty_object().as_ptr());
        }
        let _jim_command = self.retain_active_jim_command(&cmd, generation);
        let code = self.invoke_bound_body(cmd, generation, argv);
        self.retire_pending_native_ensemble_roles();
        if code == Code::Error && self.uses_jim_error_stack() {
            self.capture_jim_error_stack();
        }
        code
    }

    fn enter_original_jim_evaluation(
        &self,
        line: u32,
        filename: *mut TclObj,
    ) -> JimEvaluationScope {
        // naming.source.jim-original-source-entry
        // docs/design/analysis/name-resolution-proofs/jim-original-source-entry.md
        let script = self
            .cmd_frames
            .borrow()
            .last()
            .map(|frame| JimScriptLocation {
                file: frame.file.as_deref().unwrap_or(b"").to_vec(),
                line,
            });
        let mut frames = self.jim_evaluation_frames.borrow_mut();
        let previous_len = frames.len();
        frames.push(JimEvaluationFrame {
            procedure_level: self.jim_procedure_level.get(),
            command_name: None,
            is_procedure: false,
            script,
            invocation: Vec::new(),
        });
        let mut borrows = self.jim_invocation_borrows.borrow_mut();
        let previous_borrows_len = borrows.len();
        borrows.push(JimBorrowedInvocation {
            frame_index: previous_len,
            argv: Vec::new(),
            script_filename: Some(filename),
        });
        JimEvaluationScope {
            interp: self.clone(),
            previous_len,
            previous_borrows_len,
        }
    }

    fn borrow_jim_invocation(&self, argv: &[*mut TclObj]) -> Option<JimInvocationScope> {
        if !self.uses_jim_error_stack() {
            return None;
        }
        let frame_index = self.jim_evaluation_frames.borrow().len().checked_sub(1)?;
        let mut borrows = self.jim_invocation_borrows.borrow_mut();
        let previous_len = borrows.len();
        borrows.push(JimBorrowedInvocation {
            frame_index,
            // Copy pointer slots, not object references: the caller owns the
            // objects through invoke_bound's synchronous return or unwind.
            argv: argv.to_vec(),
            script_filename: None,
        });
        Some(JimInvocationScope {
            interp: self.clone(),
            previous_len,
        })
    }

    fn invoke_bound_body(
        &mut self,
        cmd: Command,
        generation: Option<u64>,
        argv: &[*mut TclObj],
    ) -> Code {
        match cmd {
            Command::Builtin(f) => {
                let _native_admission = self.enter_native_builtin(generation, argv);
                f(self, argv)
            }
            Command::Alias {
                target,
                prefix,
                jim_prefix,
                ..
            } => self.dispatch_alias(&target, &prefix, jim_prefix.as_ref(), argv),
            Command::ObjCmd(command) => self.invoke_obj_cmd(&command, argv),
            Command::Imported {
                source,
                source_generation,
                ensemble,
                ..
            } => {
                if let Some(token) = ensemble {
                    if !token.is_deleted() {
                        return self.dispatch_ensemble(&token, argv);
                    }
                }
                match self.resolve_import_source(&source, source_generation) {
                    // Transparent redirect: forward argv unchanged to the source.
                    Some(cmd) => self.invoke_bound(cmd, Some(source_generation), argv),
                    None => self.invalid_command(&source),
                }
            }
            Command::Ensemble(token) => self.dispatch_ensemble(&token, argv),
            Command::Proc(def) => self.call_bound_proc(&def, argv),
            Command::ChildInterp(command) => self.dispatch_original_child(&command, argv),
            Command::OoObject(id) => self.oo_dispatch(id, argv),
            Command::OoMy(id) => crate::cmd_oo::my_cmd(self, id, argv),
            Command::OoMyClass(id) => crate::cmd_oo::myclass_cmd(self, id, argv),
            Command::ParentAlias {
                target,
                prefix,
                jim_prefix,
                ..
            } => self.dispatch_parent_alias(&target, &prefix, jim_prefix.as_ref(), argv),
        }
    }

    /// Call an extension's `Tcl_ObjCmdProc` with `argv` as its `objv`, and
    /// take the completion code it answers. The result is whatever the
    /// procedure left in the interpreter through `Tcl_SetObjResult`, which
    /// dispatch emptied before the call.
    fn invoke_obj_cmd(&mut self, command: &ObjCommand, argv: &[*mut TclObj]) -> Code {
        let Ok(objc) = c_int::try_from(argv.len()) else {
            self.set_result_bytes(b"too many arguments for a C command");
            return Code::Error;
        };
        let stated = self.c_api_errors.get();
        // SAFETY: the extension registered this procedure and client data
        // together; `self` stays borrowed for the call, so the pointer is live
        // for as long as the procedure may use it, and `argv` holds `objc` live
        // objects.
        let code = unsafe {
            (command.proc_)(
                command.client_data,
                std::ptr::from_mut(self),
                objc,
                argv.as_ptr(),
            )
        };
        let code = Code::from_int(code);
        // C Tcl starts every command with no error in flight, so an error the
        // procedure returned without stating a code through the C API is
        // `NONE`, whatever error came before it.
        if code == Code::Error && self.c_api_errors.get() == stated {
            *self.exc.borrow_mut() = ExceptionState::default();
        }
        code
    }

    /// `Tcl_CreateObjCommand`: select the original C publication address.
    /// Unqualified names bind globally; qualified relative names use the actual
    /// current namespace. Answers the installed token without relooking up a name.
    pub(crate) fn create_obj_command(&mut self, name: &[u8], command: ObjCommand) -> Option<u64> {
        let selected = self
            .namespaces_mut()
            .command_c_api_publication_at(self.current_ns(), name);
        let Some((namespace, simple)) = selected else {
            self.refuse_host_command("C command publication has no selected original namespace");
            return None;
        };
        self.bind_command_replacement(namespace, &simple, Command::ObjCmd(Rc::new(command)));
        if self.host_refusal_pending() {
            return None;
        }
        self.namespaces().command_generation(namespace, &simple)
    }

    /// Register `cmd` under the (possibly qualified) name `name` — for the OO
    /// object/class commands.
    pub(crate) fn ns_register(&mut self, name: &[u8], cmd: Command) {
        let ns = self
            .namespaces
            .borrow_mut()
            .command_home_ns(self.current_ns.get(), name);
        let tail = tcl_syntax::naming::written_command_tail(name).to_vec();
        self.bind_command_replacement(ns, &tail, cmd);
    }

    /// The fully-qualified name a (relative or absolute) command/object name
    /// resolves to, relative to the current namespace — used to name OO
    /// objects/classes consistently.
    pub(crate) fn fqn_for(&self, name: &[u8]) -> Vec<u8> {
        if name.starts_with(b"::") {
            return normalize_colons(name);
        }
        let qn = self
            .namespaces
            .borrow()
            .qualified_name(self.current_ns.get());
        let mut fqn = qn.clone();
        if qn != b"::" {
            fqn.extend_from_slice(b"::");
        }
        fqn.extend_from_slice(name);
        normalize_colons(&fqn)
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

    /// The `info cmdtype` classification of `name`, or `None` if no such command.
    pub(crate) fn cmdtype(&self, name: &[u8]) -> Option<&'static [u8]> {
        let cmd = self
            .namespaces
            .borrow()
            .resolve(self.current_ns.get(), name)?;
        Some(match cmd {
            // A coroutine resume command registers as a builtin but reports
            // its own cmdType (C's per-command registration).
            Command::Builtin(_) => match self.resolve_cmd_token(name) {
                Some(generation) if self.coros.borrow().contains_key(&generation) => b"coroutine",
                _ => b"native",
            },
            Command::ObjCmd(_) => b"native",
            Command::Proc(_) => b"proc",
            Command::Alias { .. } | Command::ParentAlias { .. } => b"alias",
            Command::Imported { .. } => b"import",
            Command::Ensemble(_) => b"ensemble",
            Command::OoObject(_) => b"object",
            Command::OoMy(_) => b"privateObject",
            Command::OoMyClass(_) => b"privateClass",
            Command::ChildInterp(_) => b"interp",
        })
    }

    /// The `::tcl::mathfunc::*` function names (`info functions`).
    pub(crate) fn mathfunc_names(&self) -> Vec<Vec<u8>> {
        let surface =
            tcl_registry::expr_surface::RuntimeExprSurface::for_tcl_version(self.runtime_version());
        if !surface.has_math_function_command_table() {
            return surface
                .builtin_math_function_names()
                .into_iter()
                .map(|name| name.as_bytes().to_vec())
                .collect();
        }
        let id = self
            .namespaces
            .borrow()
            .find_namespace(GLOBAL, b"::tcl::mathfunc");
        id.map_or_else(Vec::new, |id| self.visible_command_names_in(id))
    }

    /// The canonical FQN `name` resolves to (full resolution order), or `None`
    /// if no such command — for `trace add|remove|info command|execution`, which
    /// must address the same binding `dispatch` hits and error
    /// `invalid command name` on a miss.
    pub(crate) fn resolve_cmd_fqn(&self, name: &[u8]) -> Option<Vec<u8>> {
        self.namespaces
            .borrow()
            .resolve_fqn(self.current_ns.get(), name)
    }

    /// The child-as-command subcommand words, in C table order (`options[]` in
    /// `NRChildCmd`, `tclInterp.c`), resolved with
    /// `Tcl_GetIndexFromObj(…, "option", 0)` — so `ev` abbreviates `eval` and
    /// the empty word is `ambiguous option ""`. The child command object
    /// advertises a *shorter* list than `interp` does (no `children`, `create`,
    /// `delete`, or `exists`: those are only ever spelled `interp <op> path`),
    /// and this runtime dispatches all thirteen.
    /// The `wrong # args: should be "<child><tail>"` message for the `$child
    /// <sub>` shorthand. C's `NRChildCmd` builds every one of its arity errors
    /// with `Tcl_WrongNumArgs(interp, 1, objv, …)`, so the noun is **`objv[0]`
    /// — the word this call was written with**, never the `interp` ensemble and
    /// never the child's table key.
    ///
    /// The two spellings diverge whenever the command is not reached under the
    /// name it was created with: after `interp create kid; rename kid foo`,
    /// `foo hidden extra` reports `"foo hidden"`, and the qualified `::foo
    /// hidden extra` reports `"::foo hidden"` (tclsh 8.6.16 / 9.0.4-pinned).
    /// Only the child *lookup* keeps using the table key.
    ///
    /// One spelling this does not yet recover: reached through an `interp
    /// alias`, C reports the *alias*' name, because `AliasObjCmd` installs an
    /// ensemble rewrite (`TclInitRewriteEnsemble`, tclInterp.c) that
    /// `Tcl_WrongNumArgs` reads back. This runtime's alias trampoline records no
    /// such rewrite, so `bar hidden extra` reports the target's name rather than
    /// `bar` — a separate gap in alias dispatch, not in this seam.
    ///
    /// The *subcommand* half of the noun goes the other way: every `tail` here
    /// spells the table word in full, because `Tcl_WrongNumArgs` expands an
    /// index-typed argument back to its table entry (`tclIndexObj.c`) and
    /// `Tcl_GetIndexFromObj` has already retyped `objv[1]` by the time the
    /// arity check runs. So an abbreviated call reports the canonical word:
    /// `kid hidd extra` is `"kid hidden"`, not `"kid hidd"` (tclsh
    /// 8.6.16 / 9.0.4-pinned). Written word for the command, canonical word
    /// for the subcommand.
    fn child_wrong_args(&mut self, argv: &[*mut TclObj], tail: &[u8]) -> Code {
        let mut message = b"wrong # args: should be \"".to_vec();
        message.extend_from_slice(&invoked_word(argv));
        message.extend_from_slice(tail);
        message.push(b'"');
        self.wrong_arguments_message(&message)
    }

    /// Dispatch a child-interpreter command (`$child subcommand ?arg ...?`): the
    /// child is addressable like the `interp` ensemble restricted to it.
    ///
    /// The `hide` / `expose` / `invokehidden` arms hand off to
    /// [`crate::cmd_alias::hidectl_in`] / [`crate::cmd_alias::invokehidden_in`],
    /// the same owners `interp hide|expose|invokehidden path …` calls — C's
    /// `NRChildCmd` and `NRInterpCmd` share `ChildHide` / `ChildExpose` /
    /// `ChildInvokeHidden` the same way. Only the arity check and its noun
    /// differ between the two entry points.
    fn dispatch_child(&mut self, name: &[u8], argv: &[*mut TclObj]) -> Code {
        if argv.len() < 2 {
            return self.child_wrong_args(argv, b" cmd ?arg ...?");
        }
        if self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_some()
        {
            const CHOICES: &[&[u8]] = &[b"eval", b"delete", b"alias"];
            let word = obj_bytes(argv[1]);
            let sub = match crate::cmd_alias::resolve_interp_option(CHOICES, CHOICES, &word) {
                Ok(sub) => sub,
                Err(message) => return self.error(&message),
            };
            return match sub {
                b"eval" if argv.len() >= 3 => self.child_eval_original(name, &argv[2..]),
                b"delete" if argv.len() == 2 => {
                    self.delete_child(name);
                    self.set_result_bytes(b"");
                    Code::Ok
                }
                b"alias" if argv.len() >= 4 => {
                    let alias = obj_bytes(argv[2]);
                    match self.install_original_jim_parent_alias(name, &alias, &argv[3..]) {
                        Ok(true) => {
                            self.set_result_bytes(b"");
                            Code::Ok
                        }
                        Ok(false) => self.error(b"could not find interpreter"),
                        Err(error) => self.report_cmd_error(error.into()),
                    }
                }
                _ => self.child_wrong_args(argv, b" subcommand ?arg ...?"),
            };
        }
        let sub = match self.native_interpreter_option_from_original(argv[1], true) {
            Ok(name) => name.as_bytes(),
            Err(error) => return self.report_cmd_error(error),
        };
        match sub {
            b"eval" => {
                if argv.len() < 3 {
                    return self.child_wrong_args(argv, b" eval arg ?arg ...?");
                }
                self.child_eval_original(name, &argv[2..])
            }
            b"issafe" => {
                if argv.len() != 2 {
                    return self.child_wrong_args(argv, b" issafe");
                }
                let safe = self.with_child(name, |c| c.is_safe()).unwrap_or(false);
                self.set_result_bytes(if safe { b"1" } else { b"0" });
                Code::Ok
            }
            b"delete" => {
                self.delete_child(name);
                self.set_result_bytes(b"");
                Code::Ok
            }
            // Both forms are `$child hide|expose name ?other?`. The one-word
            // form spells source and destination the same, which is what makes
            // C's asymmetric checks visible here: `kid hide ::foo::bar` is a
            // qualified *token* and `kid expose ::foo::bar` a qualified
            // *destination*, so the two report different errors.
            b"hide" | b"expose" => {
                // The *resolved* word, so `kid ex foo` is an expose too.
                let hide = sub == b"hide";
                let op = if hide {
                    CommandVisibilityOp::Hide
                } else {
                    CommandVisibilityOp::Expose
                };
                if argv.len() != 3 && argv.len() != 4 {
                    return self.child_wrong_args(
                        argv,
                        if hide {
                            b" hide cmdName ?hiddenCmdName?"
                        } else {
                            b" expose hiddenCmdName ?cmdName?"
                        },
                    );
                }
                // A safe interpreter may not touch any hidden-command table
                // (checked on the executing interp).
                if self.is_safe() {
                    return self.error(if hide {
                        b"permission denied: safe interpreter cannot hide commands"
                    } else {
                        b"permission denied: safe interpreter cannot expose commands"
                    });
                }
                crate::cmd_alias::hidectl_in(self, &[name.to_vec()], op, &argv[2..])
            }
            b"invokehidden" => {
                let mut usage = invoked_word(argv);
                usage.extend_from_slice(
                    b" invokehidden ?-namespace ns? ?-global? ?--? cmd ?arg ..?",
                );
                crate::cmd_alias::invokehidden_in(self, &[name.to_vec()], &argv[2..], &usage)
            }
            b"hidden" => {
                if argv.len() != 2 {
                    return self.child_wrong_args(argv, b" hidden");
                }
                let names = self
                    .with_child(name, |c| c.hidden_names())
                    .unwrap_or_default();
                let elems: Vec<*mut TclObj> =
                    names.iter().map(|n| obj::new_string_bytes(n)).collect();
                self.set_result(self.new_list_object(&elems));
                Code::Ok
            }
            b"aliases" => {
                if argv.len() != 2 {
                    return self.child_wrong_args(argv, b" aliases");
                }
                let names = self
                    .with_child(name, |c| c.alias_names())
                    .unwrap_or_default();
                let elems: Vec<*mut TclObj> =
                    names.iter().map(|n| obj::new_string_bytes(n)).collect();
                self.set_result(self.new_list_object(&elems));
                Code::Ok
            }
            // `$child alias srcCmd targetCmd ?arg ...?` — a cross-interp alias in
            // the child delegating to `targetCmd` in this (parent) interp. (The
            // target is implicitly the parent, so there is no target-path arg.)
            b"alias" => {
                if argv.len() < 4 {
                    return self.child_wrong_args(argv, b" alias aliasName ?targetName? ?arg ...?");
                }
                let alias = obj_bytes(argv[2]);
                let target = obj_bytes(argv[3]);
                let prefix: Vec<Vec<u8>> = argv[4..].iter().map(|&a| obj_bytes(a)).collect();
                if !self.install_parent_alias(name, &alias, target, prefix) {
                    if self.host_refusal_pending() {
                        return Code::Error;
                    }
                    return self.error(b"could not find interpreter");
                }
                self.set_result(obj::new_string_bytes(&alias));
                Code::Ok
            }
            // `$child recursionlimit ?newlimit?` — the path-less child form.
            b"recursionlimit" => {
                if argv.len() > 3 {
                    return self.child_wrong_args(argv, b" recursionlimit ?newlimit?");
                }
                let newlimit = argv.get(2).map(|&a| obj_bytes(a));
                match self.with_child(name, |c| c.recursion_limit_apply(newlimit.as_deref())) {
                    Some(Ok(n)) => {
                        self.set_result_bytes(n.to_string().as_bytes());
                        Code::Ok
                    }
                    Some(Err(m)) => self.error(&m),
                    None => self.error(b"could not find interpreter"),
                }
            }
            // `$child bgerror ?cmdPrefix?` — get/set the child's background-error
            // handler.
            b"bgerror" => {
                if argv.len() > 3 {
                    return self.child_wrong_args(argv, b" bgerror ?cmdPrefix?");
                }
                let prefix = argv.get(2).copied();
                match self.with_child(name, |c| c.bgerror_apply(prefix)) {
                    Some(Ok(h)) => {
                        unsafe {
                            self.set_obj_result(h.as_ptr());
                        }
                        Code::Ok
                    }
                    Some(Err(error)) => self.report_cmd_error(error),
                    None => self.error(b"could not find interpreter"),
                }
            }
            // `$child marktrusted` — clear the child's safe flag (denied from a
            // safe interpreter).
            b"marktrusted" => {
                if self.is_safe() {
                    return self.error(b"permission denied: safe interpreter cannot mark trusted");
                }
                self.with_child(name, |c| c.mark_trusted());
                self.set_result_bytes(b"");
                Code::Ok
            }
            // `$child debug ?-frame ?bool??` — the per-interp frame-debug switch.
            b"debug" => {
                if argv.len() > 4 {
                    return self.child_wrong_args(argv, b" debug ?-frame ?bool??");
                }
                let opts: Vec<*mut TclObj> = argv[2..].to_vec();
                match self.with_child(name, |c| c.debug_apply(&opts)) {
                    Some(Ok(o)) => {
                        self.set_result(o);
                        Code::Ok
                    }
                    Some(Err(error)) => self.report_cmd_error(error),
                    None => self.error(b"could not find interpreter"),
                }
            }
            // `$child limit limitType ?-option value …?` — query/configure the
            // child's commands/time limit.
            b"limit" => {
                if argv.len() < 3 {
                    return self.child_wrong_args(argv, b" limit limitType ?-option value ...?");
                }
                let ltype = obj_bytes(argv[2]);
                let opts: Vec<*mut TclObj> = argv[3..].to_vec();
                match self.with_child(name, |c| c.limit_apply(&ltype, &opts)) {
                    Some(Ok(o)) => {
                        self.set_result(o);
                        Code::Ok
                    }
                    Some(Err(error)) => self.report_cmd_error(error),
                    None => self.error(b"could not find interpreter"),
                }
            }
            _ => self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native child interpreter worker",
                )
                .into(),
            ),
        }
    }

    /// Jim's zero-argument factory creates a global interpreter handle.
    pub(crate) fn create_jim_child(&mut self) -> Vec<u8> {
        let name = format!("::interp.handle{}", self.interp_counter.get()).into_bytes();
        self.interp_counter.set(self.interp_counter.get() + 1);
        self.create_child(Some(name))
    }

    /// Create a child interpreter named `name` (auto-generated when empty),
    /// registering it as a command in this interp. Returns the name.
    pub(crate) fn create_child(&mut self, name: Option<Vec<u8>>) -> Vec<u8> {
        self.invalidate_interpreter_policy();
        self.invalidate_command_environment();
        let name = name.unwrap_or_else(|| {
            let n = format!("interp{}", self.interp_counter.get());
            self.interp_counter.set(self.interp_counter.get() + 1);
            n.into_bytes()
        });
        // Physical constructor inventory requires the same explicit profile
        // recipe as with_native_core; a logical runtime version is not one.
        let bootstrap = tcl_registry::InvocationDialect::of_profile(self.dialect_profile())
            .native_bootstrap_protocol();
        let mut child = if bootstrap.is_some() {
            Interp::with_native_core(
                self.host(),
                self.dialect_profile(),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .expect("selected native child core bootstrap")
        } else {
            Interp::with_host(self.host())
        };
        child.native_child_interpreter.set(true);
        // A child interpreter is another interpreter of the *same* Tcl build,
        // not a different release — C compiles one library in, so every child
        // reports and behaves as its parent's release. Inherited before the
        // globals are written, so the child's `tcl_version`/`tcl_patchLevel`
        // and its namespace-scope variable resolution both agree with the
        // parent. Resolution still runs against the child's *own* global
        // namespace: the rule is shared, the variables are not. The whole
        // profile is inherited, not just the release, so a child's
        // command-surface availability gate agrees too.
        child.set_dialect_profile(self.dialect_profile());
        let initialisation = bootstrap
            .map(|bootstrap| bootstrap.child_initialisation())
            .filter(|initialisation| initialisation.initialises_jim_static_extensions());
        let initialisation = initialisation.map_or(Ok(()), |initialisation| {
            self.initialise_original_jim_child(&mut child, initialisation)
        });
        if self.host_refusal_pending() {
            return name;
        }
        if child.host_refusal_pending() {
            self.transport_host_refusal_from(&child);
            return name;
        }
        if let Err(error) = initialisation {
            self.report_cmd_error(error.into());
            return name;
        }
        child
            .logical_expression_parse_provider
            .set(self.logical_expression_parse_provider.get());
        child
            .logical_source_word_provider
            .set(self.logical_source_word_provider.get());
        child
            .logical_eval_object_provider
            .set(self.logical_eval_object_provider.get());
        let pin = self.0.pin.borrow().clone();
        child.install_pin(pin);
        child
            .channels
            .borrow_mut()
            .share_process_state_from(&self.channels.borrow());
        // The child owns its selected core root allocations. Jim's child API
        // additionally initializes its supported static distribution and copies
        // its designated parent globals; C library initialisation is separate.
        // `interp debug -frame` is seeded from the creating interp's
        // `env(TCL_INTERP_DEBUG_FRAME)` (C's `Tcl_CreateChild`).
        if self
            .var_get_elem(b"env", b"TCL_INTERP_DEBUG_FRAME")
            .map(|value| {
                match crate::typed_value::native_boolean(value, self.native_invocation_dialect()) {
                    Ok(boolean) => boolean,
                    Err(error) => {
                        if let Some(refusal) = error.native_access_refusal() {
                            self.refuse_native_access(refusal);
                        }
                        false
                    }
                }
            })
            .unwrap_or(false)
        {
            child.0.debug_frame.set(true);
            child.invalidate_interpreter_policy();
        }
        if self.host_refusal_pending() {
            return name;
        }
        let command = ChildInterpreterCommand::new(name.clone(), child.clone());
        self.children
            .borrow_mut()
            .insert(name.clone(), child.clone());
        let (namespace, tail) = if self
            .native_invocation_dialect()
            .native_string_protocol()
            .is_some_and(|protocol| {
                !matches!(
                    protocol,
                    tcl_syntax::native_string::NativeStringProtocol::Jim084
                )
            }) {
            self.namespaces
                .borrow_mut()
                .command_c_api_publication_at(self.current_ns.get(), &name)
                .expect("selected native child C command registration")
        } else {
            let namespace = self.namespaces.borrow_mut().command_home_ns(GLOBAL, &name);
            (
                namespace,
                tcl_syntax::naming::written_command_tail(&name).to_vec(),
            )
        };
        self.bind_command_replacement(namespace, &tail, Command::ChildInterp(command));
        child.native_child_command_generation.set(
            self.namespaces
                .borrow()
                .command_generation(namespace, &tail),
        );
        name
    }

    /// Whether a child interpreter `name` exists.
    pub(crate) fn child_exists(&self, name: &[u8]) -> bool {
        self.children
            .borrow()
            .get(name)
            .is_some_and(|child| !child.pending_delete.get())
    }

    /// Run `f` on the child interpreter `name` (or `None` if it doesn't exist) —
    /// the mutable-access path for `interp <sub> childPath …`.
    ///
    /// The child's handle is **cloned out** of the table (an `Rc` bump) so the
    /// `children` borrow is released before `f` runs. `f` may therefore re-enter
    /// `self` — a child's aliased `source`/`invokehidden` calling back to the
    /// parent — and even re-enter the same child through a fresh handle: the
    /// shared `InterpState` is reached via the `Rc` plus per-field interior
    /// mutability, never an aliased `&mut`. The child's `parent` `Weak` is set for
    /// the call so re-entrancy can reach up, and restored after.
    ///
    /// The child is marked active for the call ([`eval_active`](InterpState)), so
    /// a deleted child remains alive through this retained activation while its
    /// former parent-table entry and command token are retired independently.
    pub(crate) fn with_child<R>(
        &mut self,
        name: &[u8],
        f: impl FnOnce(&mut Interp) -> R,
    ) -> Option<R> {
        let mut child = self
            .children
            .borrow()
            .get(name)
            .filter(|child| !child.pending_delete.get())?
            .clone();
        // Parent association and active/deferred-delete lifecycle are part of
        // child-interpreter policy. A token minted before this entry must not
        // survive into a differently-associated child evaluation.
        child.invalidate_interpreter_policy();
        let saved_parent = child.parent.replace(Rc::downgrade(&self.0));
        child.eval_active.set(child.eval_active.get() + 1);
        let r = f(&mut child);
        child.eval_active.set(child.eval_active.get() - 1);
        *child.parent.borrow_mut() = saved_parent;
        child.invalidate_interpreter_policy();
        self.retire_pending_children();
        Some(r)
    }

    /// Run `f` against the interpreter addressed by a (possibly multi-level)
    /// path — a list of child names descending from this interp. An empty path
    /// is this interp itself; otherwise each name is resolved through
    /// [`with_child`] in turn. Returns `None` if any name in the chain is not a
    /// child of its predecessor (`interp create {a b}`, `interp eval {a b} …`).
    pub(crate) fn with_child_path<R>(
        &mut self,
        path: &[Vec<u8>],
        f: impl FnOnce(&mut Interp) -> R,
    ) -> Option<R> {
        match path {
            [] => Some(f(self)),
            [name] => self.with_child(name, f),
            [name, rest @ ..] => self
                .with_child(name, |c| c.with_child_path(rest, f))
                .flatten(),
        }
    }

    /// `interp hide name`: move command `name` out of the command table into
    /// the hidden table under `hidden_name`. `Missing` when `name` does not
    /// resolve, `NonGlobal` when it resolves outside the global namespace,
    /// `Collision` when `hidden_name` is already hidden, `Moved` on success —
    /// never a bare success flag, because the caller has to word four
    /// different diagnostics, and C's order between them is observable.
    pub(crate) fn hide_command(
        &mut self,
        name: &[u8],
        hidden_name: &[u8],
    ) -> CommandVisibilityOutcome {
        // Gated: a builtin the emulated release does not carry is not there to
        // be hidden, so `interp hide` cannot park it in the hidden table where
        // `interp invokehidden` would reach it past the surface check.
        let resolved = self.resolve_dispatchable(GLOBAL, name);
        match resolved {
            Some(_) => {
                // C's order, and it is observable: `Tcl_HideCommand` resolves
                // the source before it rejects a non-global one, and rejects a
                // non-global one before it refuses an occupied token
                // (tclBasic.c:2325, :2339, :2365). `interp hide kid nosuch
                // taken` is therefore `unknown command "nosuch"`, not the
                // collision.
                //
                // The global test goes through the shared namespace owner, so a
                // run of colons collapses the way `TclGetNamespaceForQualName`
                // collapses it: `::::foo` names the global `foo` (tclsh-pinned).
                if !tcl_cmd_core::namespace::qualifiers(name).is_empty() {
                    return CommandVisibilityOutcome::NonGlobal;
                }
                if self.hidden.borrow().contains_key(hidden_name) {
                    return CommandVisibilityOutcome::Collision;
                }
                self.invalidate_interpreter_policy();
                let old_fqn = self.namespaces.borrow().resolve_fqn(GLOBAL, name);
                let Some(binding) = self.namespaces.borrow_mut().take(GLOBAL, name) else {
                    return CommandVisibilityOutcome::Missing;
                };
                let mut hidden_fqn = b"::".to_vec();
                hidden_fqn.extend_from_slice(hidden_name);
                if let Some(old_fqn) = &old_fqn {
                    self.move_cmd_traces(old_fqn, &hidden_fqn, binding.generation);
                    self.retarget_import_sources(binding.generation, &hidden_fqn);
                }
                if let Command::Ensemble(token) = &binding.command {
                    token.rename(hidden_fqn);
                }
                self.namespaces
                    .borrow_mut()
                    .note_native_command_hidden(binding.generation);
                self.hidden
                    .borrow_mut()
                    .insert(hidden_name.to_vec(), binding);
                CommandVisibilityOutcome::Moved
            }
            None => CommandVisibilityOutcome::Missing,
        }
    }

    /// `interp expose name`: move a hidden command back into the command table.
    pub(crate) fn expose_command(
        &mut self,
        hidden_name: &[u8],
        name: &[u8],
    ) -> CommandVisibilityOutcome {
        // C's order: `Tcl_ExposeCommand` looks the token up (tclBasic.c:2486)
        // before it examines the destination (:2525), so `interp expose kid
        // nosuchtok taken` is `unknown hidden command "nosuchtok"`, not the
        // collision.
        if !self.hidden.borrow().contains_key(hidden_name) {
            return CommandVisibilityOutcome::Missing;
        }
        if self.namespaces.borrow().resolve_fqn(GLOBAL, name).is_some() {
            return CommandVisibilityOutcome::Collision;
        }
        // Invalidate before removing from the hidden table. A missing entry may
        // over-invalidate, which is preferable to a re-entrant visibility gap.
        self.invalidate_interpreter_policy();
        let binding = self.hidden.borrow_mut().remove(hidden_name);
        match binding {
            Some(binding) => {
                let mut old_hidden_fqn = b"::".to_vec();
                old_hidden_fqn.extend_from_slice(hidden_name);
                let generation = binding.generation;
                let ensemble = match &binding.command {
                    Command::Ensemble(token) => Some(Rc::clone(token)),
                    _ => None,
                };
                self.namespaces.borrow_mut().restore(name, binding);
                self.retire_pending_native_ensemble_roles();
                let new_fqn = self
                    .namespaces
                    .borrow()
                    .resolve_fqn(GLOBAL, name)
                    .unwrap_or_else(|| self.fqn_for(name));
                self.move_cmd_traces(&old_hidden_fqn, &new_fqn, generation);
                self.retarget_import_sources(generation, &new_fqn);
                if let Some(token) = ensemble {
                    token.rename(new_fqn);
                }
                CommandVisibilityOutcome::Moved
            }
            None => CommandVisibilityOutcome::Missing,
        }
    }

    /// Convert the typed result of a hidden-table move into Tcl's public
    /// diagnostic. Both `interp hide/expose` and the child command shorthand
    /// use this seam so missing-source and occupied-destination cases cannot
    /// drift in message or structured error code.
    pub(crate) fn finish_command_visibility(
        &mut self,
        op: CommandVisibilityOp,
        source: &[u8],
        destination: &[u8],
        outcome: CommandVisibilityOutcome,
    ) -> Code {
        let (message, error_code) = match (op, outcome) {
            (_, CommandVisibilityOutcome::Moved) => {
                self.set_result_bytes(b"");
                return Code::Ok;
            }
            (CommandVisibilityOp::Hide, CommandVisibilityOutcome::Missing) => {
                let mut message = b"unknown command \"".to_vec();
                message.extend_from_slice(source);
                message.push(b'"');
                (
                    message,
                    error_code_list(&[b"TCL", b"LOOKUP", b"COMMAND", source]),
                )
            }
            (CommandVisibilityOp::Expose, CommandVisibilityOutcome::Missing) => {
                let mut message = b"unknown hidden command \"".to_vec();
                message.extend_from_slice(source);
                message.push(b'"');
                (
                    message,
                    error_code_list(&[b"TCL", b"LOOKUP", b"HIDDENTOKEN", source]),
                )
            }
            (CommandVisibilityOp::Hide, CommandVisibilityOutcome::NonGlobal) => (
                b"can only hide global namespace commands (use rename then hide)".to_vec(),
                b"TCL HIDE NON_GLOBAL".to_vec(),
            ),
            // C keeps this branch behind its own "theoretically impossible"
            // comment (tclBasic.c:2500): only `Tcl_HideCommand` fills the
            // hidden table, and it already refused a non-global source, so
            // `expose_command` never reports it either.
            (CommandVisibilityOp::Expose, CommandVisibilityOutcome::NonGlobal) => (
                b"trying to expose a non-global command namespace command".to_vec(),
                b"NONE".to_vec(),
            ),
            (CommandVisibilityOp::Hide, CommandVisibilityOutcome::Collision) => {
                let mut message = b"hidden command named \"".to_vec();
                message.extend_from_slice(destination);
                message.extend_from_slice(b"\" already exists");
                (message, b"TCL HIDE ALREADY_HIDDEN".to_vec())
            }
            (CommandVisibilityOp::Expose, CommandVisibilityOutcome::Collision) => {
                let mut message = b"exposed command \"".to_vec();
                message.extend_from_slice(destination);
                message.extend_from_slice(b"\" already exists");
                (message, b"TCL EXPOSE COMMAND_EXISTS".to_vec())
            }
        };
        self.error_with_code(&message, &error_code)
    }

    /// `interp invokehidden name ?arg ...?` — invoke a hidden command.
    pub(crate) fn invoke_hidden(&mut self, name: &[u8], argv: &[*mut TclObj]) -> Code {
        let binding = self.hidden.borrow().get(name).cloned();
        match binding {
            Some(binding) => {
                let mut fqn = b"::".to_vec();
                fqn.extend_from_slice(name);
                self.dispatch_prebound(argv, Some((fqn, binding)))
            }
            None => {
                let mut m = b"invalid hidden command name \"".to_vec();
                m.extend_from_slice(name);
                m.push(b'"');
                self.error(&m)
            }
        }
    }

    /// Sorted names of the hidden commands (`interp hidden`).
    pub(crate) fn hidden_names(&self) -> Vec<Vec<u8>> {
        self.hidden.borrow().keys().cloned().collect()
    }

    /// Move every import retaining one exact source command generation. Hidden
    /// imports carry the same metadata as visible ones and must move too.
    fn retarget_import_sources(&mut self, source_generation: u64, new_fqn: &[u8]) {
        self.namespaces
            .borrow_mut()
            .retarget_imports(source_generation, new_fqn);
        for binding in self.hidden.borrow_mut().values_mut() {
            if let Command::Imported {
                source,
                source_generation: candidate,
                ..
            } = &mut binding.command
            {
                if *candidate == source_generation {
                    *source = new_fqn.to_vec();
                }
            }
        }
    }

    /// Bind surviving by-name replacement redirects to a replacement's fresh
    /// generation. An import whose old generation still exists elsewhere was
    /// moved by a callback and remains attached to that exact token.
    fn reattach_missing_import_sources(&mut self, source_fqn: &[u8], new_generation: u64) {
        let mut live_generations = self.namespaces.borrow().command_generations();
        live_generations.extend(
            self.hidden
                .borrow()
                .values()
                .map(|binding| binding.generation),
        );
        self.namespaces.borrow_mut().reattach_missing_imports(
            source_fqn,
            new_generation,
            &live_generations,
        );
        for binding in self.hidden.borrow_mut().values_mut() {
            let Command::Imported {
                source,
                source_generation,
                ..
            } = &mut binding.command
            else {
                continue;
            };
            if source == source_fqn && !live_generations.contains(source_generation) {
                *source_generation = new_generation;
            }
        }
    }

    /// Retarget imports of one exact source generation to a newly-created
    /// ensemble token, across visible and hidden import bindings.
    fn retarget_imports_to_ensemble(
        &mut self,
        source_generation: u64,
        new: &Rc<crate::ensemble::EnsembleToken>,
    ) {
        self.namespaces
            .borrow_mut()
            .retarget_imports_to_ensemble(source_generation, new);
        for binding in self.hidden.borrow_mut().values_mut() {
            let Command::Imported {
                source_generation: candidate,
                ensemble,
                ..
            } = &mut binding.command
            else {
                continue;
            };
            if *candidate == source_generation {
                *ensemble = Some(Rc::clone(new));
            }
        }
    }

    /// Remove one imported command by stable identity wherever a delete-trace
    /// callback may have renamed or hidden it. A callback replacement/re-import
    /// has a new identity and is deliberately left alone.
    fn import_identity_location(&self, identity: &Rc<ImportToken>) -> Option<(Vec<u8>, u64)> {
        if let Some(location) = self.namespaces.borrow().import_identity_location(identity) {
            return Some(location);
        }
        self.hidden.borrow().iter().find_map(|(name, binding)| {
            matches!(
                &binding.command,
                Command::Imported { identity: current, .. }
                    if Rc::ptr_eq(current, identity)
            )
            .then(|| {
                let mut fqn = b"::".to_vec();
                fqn.extend_from_slice(name);
                (fqn, binding.generation)
            })
        })
    }

    fn remove_import_identity(&mut self, identity: &Rc<ImportToken>) -> Option<(Vec<u8>, u64)> {
        if let Some(fqn) = self
            .namespaces
            .borrow_mut()
            .remove_import_identity(identity)
        {
            return Some(fqn);
        }
        let hidden_name = self.hidden.borrow().iter().find_map(|(name, binding)| {
            matches!(
                &binding.command,
                Command::Imported { identity: current, .. }
                    if Rc::ptr_eq(current, identity)
            )
            .then(|| name.clone())
        })?;
        let binding = self.hidden.borrow_mut().remove(&hidden_name)?;
        self.namespaces
            .borrow_mut()
            .retire_native_command_node(binding.generation);
        let mut fqn = b"::".to_vec();
        fqn.extend_from_slice(&hidden_name);
        Some((fqn, binding.generation))
    }

    fn ensemble_identity_location(
        &self,
        identity: &Rc<crate::ensemble::EnsembleToken>,
    ) -> Option<(Vec<u8>, u64)> {
        if let Some(location) = self
            .namespaces
            .borrow()
            .ensemble_identity_location(identity)
        {
            return Some(location);
        }
        self.hidden.borrow().iter().find_map(|(name, binding)| {
            matches!(
                &binding.command,
                Command::Ensemble(current) if Rc::ptr_eq(current, identity)
            )
            .then(|| {
                let mut fqn = b"::".to_vec();
                fqn.extend_from_slice(name);
                (fqn, binding.generation)
            })
        })
    }

    /// Remove one ensemble by stable token identity after running the delete
    /// trace at the name where deletion began. The callback may rename, hide,
    /// expose, or replace the command; only the captured token is retired, and
    /// any trace sidecar moved with it is dropped without firing twice.
    fn retire_ensemble_identity(
        &mut self,
        identity: &Rc<crate::ensemble::EnsembleToken>,
    ) -> Option<(Vec<u8>, u64)> {
        if let Some((trace_fqn, generation)) = self.ensemble_identity_location(identity) {
            self.fire_delete_traces_of_token(&trace_fqn, Some(generation));
        }
        let visible_removed = self
            .namespaces
            .borrow_mut()
            .remove_ensemble_identity(identity);
        let removed_fqn = visible_removed.or_else(|| {
            let hidden_name = self.hidden.borrow().iter().find_map(|(name, binding)| {
                matches!(
                    &binding.command,
                    Command::Ensemble(current) if Rc::ptr_eq(current, identity)
                )
                .then(|| name.clone())
            })?;
            let binding = self.hidden.borrow_mut().remove(&hidden_name)?;
            self.namespaces
                .borrow_mut()
                .retire_native_command_node(binding.generation);
            let mut fqn = b"::".to_vec();
            fqn.extend_from_slice(&hidden_name);
            Some((fqn, binding.generation))
        });
        if let Some((live_fqn, generation)) = removed_fqn.as_ref() {
            self.remove_cmd_traces_of_token(live_fqn, Some(*generation));
        }
        identity.mark_deleted();
        removed_fqn
    }

    /// Remove every visible or hidden import whose immediate origin was truly
    /// deleted. Delete traces fire while each visible imported command is still
    /// in its table; the stable import identity then prevents a callback's
    /// replacement command from being removed as if it were the old import.
    /// Removed aliases are fed back into the set until transitive chains reach a
    /// fixed point. A replacement never calls this seam, so recreating an old
    /// source name cannot resurrect aliases deleted here.
    fn remove_imports_for_deleted_origins(
        &mut self,
        origins: impl IntoIterator<Item = u64>,
        tokens: &[Rc<crate::ensemble::EnsembleToken>],
    ) {
        let mut origins: std::collections::HashSet<u64> = origins.into_iter().collect();
        loop {
            let visible = self
                .namespaces
                .borrow()
                .imports_for_origins(&origins, tokens);
            let hidden: Vec<(Vec<u8>, Rc<ImportToken>)> = self
                .hidden
                .borrow()
                .iter()
                .filter_map(|(name, binding)| {
                    let Command::Imported {
                        source_generation,
                        ensemble,
                        identity,
                        ..
                    } = &binding.command
                    else {
                        return None;
                    };
                    let retains_token = ensemble.as_ref().is_some_and(|imported| {
                        tokens.iter().any(|victim| Rc::ptr_eq(imported, victim))
                    });
                    if !origins.contains(source_generation) && !retains_token {
                        return None;
                    }
                    let mut fqn = b"::".to_vec();
                    fqn.extend_from_slice(name);
                    Some((fqn, Rc::clone(identity)))
                })
                .collect();

            let mut removed_any = false;
            for (_, identity) in visible {
                let Some((fqn, generation)) = self.import_identity_location(&identity) else {
                    continue;
                };
                self.fire_delete_traces_of_token(&fqn, Some(generation));
                if let Some((removed_fqn, removed_generation)) =
                    self.remove_import_identity(&identity)
                {
                    self.remove_cmd_traces_of_token(&removed_fqn, Some(removed_generation));
                    origins.insert(removed_generation);
                    removed_any = true;
                }
            }
            for (_, identity) in hidden {
                let Some((fqn, generation)) = self.import_identity_location(&identity) else {
                    continue;
                };
                self.fire_delete_traces_of_token(&fqn, Some(generation));
                if let Some((removed_fqn, removed_generation)) =
                    self.remove_import_identity(&identity)
                {
                    self.remove_cmd_traces_of_token(&removed_fqn, Some(removed_generation));
                    origins.insert(removed_generation);
                    removed_any = true;
                }
            }
            if !removed_any {
                break;
            }
        }
    }

    /// Make this interp "safe": hide the commands that touch the host
    /// (filesystem, processes, sockets, the interpreter itself) — the core of
    /// `interp create -safe`. This does not re-alias `source`/`load`/`file`
    /// through the Safe Base, which would need cross-interp aliases.
    pub(crate) fn make_safe(&mut self) {
        // Variable unsets below can fire callbacks. Stale existing tokens before
        // the first visibility/policy write, not after re-entrant code can run.
        self.invalidate_interpreter_policy();
        // The hide list is the registry's `Traits::SAFE_INTERP_HIDDEN` query,
        // not a name list this engine keeps: C's own set is the `CmdInfo`
        // rows lacking `CMD_IS_SAFE` plus the whole-command rows of
        // `unsafeEnsembleCommands`, and that is what the trait records.
        // `hide_command` returns `false` for a name this interpreter does not
        // carry, which is the per-release narrowing: `unload` (8.5+) and
        // `zipfs` (9.0+) are release-gated and simply are not there under an
        // older pin, so no second availability rule is needed.
        //
        // `after` / `vwait` are correctly absent from the trait — confirmed
        // present and callable inside a real safe child on tclsh 8.6.14
        // (`s eval {info commands after}` returns `after`). A hand-typed hide
        // list here would risk hiding them by mistake, breaking legitimate
        // safe-interp code that uses `after idle` / `after cancel`.
        for name in tcl_registry::safe_interp_hidden_commands() {
            self.hide_command(name.as_bytes(), name.as_bytes());
        }
        self.scrub_host_globals_for_safe();
        // A safe interp's `clock` is aliased to the parent's, so date/time
        // formatting works without the child reaching the timezone files.
        self.ns_register(
            b"clock",
            Command::ParentAlias {
                target: b"clock".to_vec(),
                prefix: Vec::new(),
                publication_name: b"clock".to_vec(),
                jim_prefix: None,
                identity: Rc::new(()),
            },
        );
        self.is_safe.set(true);
    }

    fn scrub_host_globals_for_safe(&mut self) {
        // The shared schema owns the portable/host-revealing distinction too,
        // so installation and safe scrubbing cannot drift independently.
        for key in tcl_platform::bootstrap::safe_scrub_keys() {
            self.var_unset_elem(b"::tcl_platform", key.as_bytes());
        }
        self.var_unset(b"::env");
        // A safe interp has no real library/package paths. The Safe Base may
        // re-virtualise `auto_path` after this scrub.
        for name in tcl_platform::bootstrap::HOST_PATH_GLOBALS {
            self.var_unset(format!("::{name}").as_bytes());
        }
    }

    /// Whether this interp is safe (`interp issafe`).
    pub(crate) fn is_safe(&self) -> bool {
        self.is_safe.get()
    }

    /// `interp marktrusted` — clear this interp's safe flag (a parent demoting a
    /// child from safe to trusted). Future children it creates are trusted too.
    pub(crate) fn mark_trusted(&self) {
        self.invalidate_interpreter_policy();
        self.is_safe.set(false);
    }

    /// `interp debug ?-frame ?bool??` on this interp. Returns the fresh result
    /// object (the `-frame N` dict, or the bool), or the error-message bytes.
    /// `-frame` is a one-way latch: setting it to false once true keeps it true.
    pub(crate) fn debug_apply(
        &mut self,
        opts: &[*mut TclObj],
    ) -> Result<*mut TclObj, tcl_cmd_core::CmdError> {
        let frame_byte: &[u8] = if self.debug_frame.get() { b"1" } else { b"0" };
        match opts.len() {
            0 => Ok(dict_obj(self, &[(b"-frame", frame_byte.to_vec())])),
            1 => {
                self.native_static_option_index(
                    opts[0],
                    DEBUG_OPTIONS.names(),
                    false,
                    "debug option",
                )?;
                Ok(obj::new_string_bytes(frame_byte))
            }
            _ => {
                self.native_static_option_index(
                    opts[0],
                    DEBUG_OPTIONS.names(),
                    false,
                    "debug option",
                )?;
                if crate::typed_value::native_boolean(opts[1], self.native_invocation_dialect())? {
                    self.invalidate_interpreter_policy();
                    self.debug_frame.set(true);
                }
                let frame_byte: &[u8] = if self.debug_frame.get() { b"1" } else { b"0" };
                Ok(obj::new_string_bytes(frame_byte))
            }
        }
    }

    /// `interp recursionlimit` get / set on this interp. `newlimit` is the
    /// optional new-limit bytes; returns the resulting limit, or the error
    /// message the caller should raise (`expected integer …` / `… too large …`
    /// / `recursion limit must be > 0`). Each interp keeps its own limit, so a
    /// child raising its limit leaves the parent's untouched.
    pub(crate) fn recursion_limit_apply(&self, newlimit: Option<&[u8]>) -> Result<i64, Vec<u8>> {
        match newlimit {
            None => Ok(self.recursion_limit.get() as i64),
            Some(bytes) => {
                let n = parse_recursion_limit(bytes)?;
                if n <= 0 {
                    return Err(b"recursion limit must be > 0".to_vec());
                }
                self.invalidate_interpreter_policy();
                self.recursion_limit.set(n as usize);
                Ok(n)
            }
        }
    }

    /// `interp limit limitType ?-option value …?` on this interp. Returns the
    /// fresh result object on success, or the error-message bytes the caller
    /// should raise.
    pub(crate) fn limit_apply(
        &self,
        ltype: &[u8],
        opts: &[*mut TclObj],
    ) -> Result<*mut TclObj, tcl_cmd_core::CmdError> {
        match LIMIT_TYPES.index_of_cmd(ltype)? {
            0 => self.limit_commands(opts),
            _ => self.limit_time(opts),
        }
    }

    fn limit_commands(&self, opts: &[*mut TclObj]) -> Result<*mut TclObj, tcl_cmd_core::CmdError> {
        const OPTS: &[&[u8]] = &[b"-command", b"-granularity", b"-value"];
        if opts.is_empty() {
            let l = self.limits.borrow();
            return Ok(dict_obj(
                self,
                &[
                    (b"-command", l.cmd_command.clone()),
                    (b"-granularity", l.cmd_granularity.to_string().into_bytes()),
                    (b"-value", opt_int(l.cmd_value)),
                ],
            ));
        }
        if opts.len() == 1 {
            let opt = resolve_limit_opt(&obj_bytes(opts[0]), OPTS)
                .map_err(tcl_cmd_core::CmdError::new_bytes)?;
            let l = self.limits.borrow();
            let val = match opt.as_slice() {
                b"-command" => l.cmd_command.clone(),
                b"-granularity" => l.cmd_granularity.to_string().into_bytes(),
                _ => opt_int(l.cmd_value),
            };
            return Ok(obj::new_string_bytes(&val));
        }
        // A trailing option with no value is a catchable error, not a silent
        // drop (`interp limit c commands -value 1 -granularity`).
        if opts.len() % 2 != 0 {
            return Err(tcl_cmd_core::CmdError::wrong_arguments_message_bytes(
                b"wrong # args: should be \"interp limit path commands ?-option value ...?\""
                    .to_vec(),
            ));
        }
        // The option loop may commit an earlier pair before a later pair is
        // rejected. Invalidate before its first possible policy write so that
        // partial-on-error mutation cannot leave an old token live.
        self.invalidate_interpreter_policy();
        let mut i = 0;
        while i + 1 < opts.len() {
            let opt = resolve_limit_opt(&obj_bytes(opts[i]), OPTS)
                .map_err(tcl_cmd_core::CmdError::new_bytes)?;
            let val = obj_bytes(opts[i + 1]);
            match opt.as_slice() {
                b"-command" => self.limits.borrow_mut().cmd_command = val,
                b"-granularity" => {
                    let n = parse_limit_int(&val).map_err(tcl_cmd_core::CmdError::new_bytes)?;
                    if n < 1 {
                        return Err(tcl_cmd_core::CmdError::new_bytes(
                            b"granularity must be at least 1".to_vec(),
                        ));
                    }
                    self.limits.borrow_mut().cmd_granularity = n;
                }
                _ => {
                    let n = parse_limit_int(&val).map_err(tcl_cmd_core::CmdError::new_bytes)?;
                    if n < 0 {
                        return Err(tcl_cmd_core::CmdError::new_bytes(
                            b"command limit value must be at least 0".to_vec(),
                        ));
                    }
                    self.limits.borrow_mut().cmd_value = Some(n);
                }
            }
            i += 2;
        }
        Ok(obj::new_string_bytes(b""))
    }

    fn limit_time(&self, opts: &[*mut TclObj]) -> Result<*mut TclObj, tcl_cmd_core::CmdError> {
        const OPTS: &[&[u8]] = &[b"-command", b"-granularity", b"-milliseconds", b"-seconds"];
        if opts.is_empty() {
            let l = self.limits.borrow();
            let (secs, millis) = match l.time_value {
                Some((s, m)) => (s.to_string().into_bytes(), m.to_string().into_bytes()),
                None => (Vec::new(), Vec::new()),
            };
            return Ok(dict_obj(
                self,
                &[
                    (b"-command", l.time_command.clone()),
                    (b"-granularity", l.time_granularity.to_string().into_bytes()),
                    (b"-milliseconds", millis),
                    (b"-seconds", secs),
                ],
            ));
        }
        if opts.len() == 1 {
            let opt = resolve_limit_opt(&obj_bytes(opts[0]), OPTS)
                .map_err(tcl_cmd_core::CmdError::new_bytes)?;
            let l = self.limits.borrow();
            let val = match opt.as_slice() {
                b"-command" => l.time_command.clone(),
                b"-granularity" => l.time_granularity.to_string().into_bytes(),
                b"-seconds" => opt_int(l.time_value.map(|(s, _)| s)),
                _ => opt_int(l.time_value.map(|(_, m)| m)),
            };
            return Ok(obj::new_string_bytes(&val));
        }
        if opts.len() % 2 != 0 {
            return Err(tcl_cmd_core::CmdError::wrong_arguments_message_bytes(
                b"wrong # args: should be \"interp limit path time ?-option value ...?\"".to_vec(),
            ));
        }
        // As with command limits, parsing is incremental and can partially
        // mutate before returning an error on a later option.
        self.invalidate_interpreter_policy();
        let (mut sec, mut ms) = self.limits.borrow().time_value.unwrap_or((0, 0));
        let mut touched = self.limits.borrow().time_value.is_some();
        let mut i = 0;
        while i + 1 < opts.len() {
            let opt = resolve_limit_opt(&obj_bytes(opts[i]), OPTS)
                .map_err(tcl_cmd_core::CmdError::new_bytes)?;
            let val = obj_bytes(opts[i + 1]);
            match opt.as_slice() {
                b"-command" => self.limits.borrow_mut().time_command = val,
                b"-granularity" => {
                    let n = parse_limit_int(&val).map_err(tcl_cmd_core::CmdError::new_bytes)?;
                    if n < 1 {
                        return Err(tcl_cmd_core::CmdError::new_bytes(
                            b"granularity must be at least 1".to_vec(),
                        ));
                    }
                    self.limits.borrow_mut().time_granularity = n;
                }
                b"-seconds" => {
                    let n = parse_limit_int(&val).map_err(tcl_cmd_core::CmdError::new_bytes)?;
                    if n < 0 {
                        return Err(tcl_cmd_core::CmdError::new_bytes(
                            b"seconds must be non-negative".to_vec(),
                        ));
                    }
                    sec = n;
                    touched = true;
                }
                _ => {
                    let n = parse_limit_int(&val).map_err(tcl_cmd_core::CmdError::new_bytes)?;
                    if n < 0 {
                        return Err(tcl_cmd_core::CmdError::new_bytes(
                            b"milliseconds must be non-negative".to_vec(),
                        ));
                    }
                    ms = n;
                    touched = true;
                }
            }
            i += 2;
        }
        if touched {
            // Normalise excess milliseconds into seconds.
            sec += ms.div_euclid(1000);
            ms = ms.rem_euclid(1000);
            self.limits.borrow_mut().time_value = Some((sec, ms));
        }
        Ok(obj::new_string_bytes(b""))
    }

    /// Whether this interp's `time` limit has elapsed (an absolute wall-clock
    /// deadline). `false` when no time limit is set.
    #[cfg(have_tommath)]
    pub(crate) fn time_limit_exceeded(&self) -> bool {
        match self.limits.borrow().time_value {
            Some((secs, millis)) => {
                let deadline = i128::from(secs) * 1000 + i128::from(millis);
                self.host().clock().now_millis() >= deadline
            }
            None => false,
        }
    }

    /// Whether a `time` limit is configured at all — the cheap guard the loop
    /// commands check before paying for a wall-clock read.
    #[cfg(have_tommath)]
    pub(crate) fn has_time_limit(&self) -> bool {
        self.limits.borrow().time_value.is_some()
    }

    /// Advance the limit-poll counter and, when a `time` limit is armed and the
    /// throttle window elapses, set the `time limit exceeded` error and return
    /// its `Code`. Called from the loop commands (`while`/`for`) each iteration
    /// so an unbounded loop under `interp limit $i time` terminates. A guarded
    /// no-op when no time limit is set.
    #[cfg(have_tommath)]
    pub(crate) fn limit_check_tick(&mut self) -> Option<Code> {
        if let Some(code) = self.charge_tick() {
            return Some(code);
        }
        if !self.has_time_limit() {
            return None;
        }
        let t = self.limit_tick.get().wrapping_add(1);
        self.limit_tick.set(t);
        if t & 0x0FFF == 0 && self.time_limit_exceeded() {
            return Some(self.error(b"time limit exceeded"));
        }
        None
    }

    /// The names of this interp's direct child interpreters (sorted).
    pub(crate) fn child_names(&self) -> Vec<Vec<u8>> {
        self.children.borrow().keys().cloned().collect()
    }

    /// Delete a child interpreter (and its command). Returns whether it existed.
    ///
    /// Its exact command token is removed and its parent-table entry retired now.
    /// An executing child is retained by the actual `with_child` activation,
    /// so the interpreter allocation survives until that eval unwinds
    /// ([`with_child`]/[`eval_in_child`]) — never while a re-entrant eval of it is
    /// still on the stack. Mirrors C's deferred `Tcl_DeleteInterp`.
    ///
    /// [`with_child`]: Interp::with_child
    /// [`eval_in_child`]: Interp::eval_in_child
    pub(crate) fn delete_child(&mut self, name: &[u8]) -> bool {
        let child = self.children.borrow().get(name).cloned();
        let Some(child) = child else {
            return false;
        };
        self.invalidate_interpreter_policy();
        let generation = child.native_child_command_generation.get();
        let placed = generation.and_then(|generation| {
            self.namespaces
                .borrow()
                .native_command_at_node(generation)
                .map(|(_, full_name)| full_name)
        });
        if let Some(full_name) = placed {
            self.delete_bound_command(&full_name);
        } else if let Some(generation) = generation {
            self.hidden
                .borrow_mut()
                .retain(|_, binding| binding.generation != generation);
        }
        child.pending_delete.set(true);
        child.invalidate_interpreter_policy();
        self.retire_pending_children();
        self.invalidate_command_environment();
        true
    }

    /// Run a cross-interp alias (`ParentAlias`): invoke `target` (+ `prefix` +
    /// the call args) in this interp's *parent*, copying the parent's result
    /// back. The parent is reached by upgrading the `parent` `Weak` to an owned
    /// `Interp` handle and dispatching through it — re-entrancy (the parent
    /// calling `interp invokehidden $child …` back into this child) works via the
    /// shared interior-mutable state and is only **bounded** by
    /// `MAX_CROSS_INTERP_DEPTH` to cap native-stack growth.
    fn dispatch_parent_alias(
        &mut self,
        target: &[u8],
        prefix: &[Vec<u8>],
        jim_prefix: Option<&obj::Owned>,
        argv: &[*mut TclObj],
    ) -> Code {
        let policy = tcl_runtime_api::completion_options::ControlOptionPolicy::FRESH_FORWARDED;
        self.begin_control_options(policy);
        let Some(parent_state) = self.parent.borrow().upgrade() else {
            return self.error(b"cannot invoke a parent alias from the root interpreter");
        };
        if CROSS_INTERP_DEPTH.with(|d| d.get()) >= MAX_CROSS_INTERP_DEPTH {
            return self.error(b"too many nested cross-interpreter calls");
        }
        let mut parent = Interp(parent_state);
        if self.native_invocation_dialect().native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            let Some(original_prefix) = jim_prefix else {
                return self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "Jim original parent alias prefix",
                    )
                    .into(),
                );
            };
            return self.dispatch_original_jim_parent_alias(&mut parent, original_prefix, argv);
        }
        // Build [target, *prefix, *argv[1..]] — each element owned (+1).
        let mut new_argv: Vec<*mut TclObj> = Vec::with_capacity(prefix.len() + argv.len());
        let push_owned = |v: &mut Vec<*mut TclObj>, o: *mut TclObj| {
            unsafe { obj::incr_ref_count(o) };
            v.push(o);
        };
        push_owned(&mut new_argv, new_string(target));
        for p in prefix {
            push_owned(&mut new_argv, new_string(p));
        }
        for &a in &argv[1..] {
            push_owned(&mut new_argv, a);
        }
        CROSS_INTERP_DEPTH.with(|d| d.set(d.get() + 1));
        // `parent` is an owned handle sharing the parent's `InterpState`; the
        // dispatch mutates it through interior mutability (no aliased `&mut`).
        parent.begin_control_options(policy);
        let code = parent.dispatch_invoke(&new_argv);
        let res = parent.result_bytes();
        let options = parent.pending_return_option_objects();
        CROSS_INTERP_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
        release_all(&new_argv);
        self.set_result_bytes(&res);
        self.set_return_option_objects(options);
        code
    }

    fn call_bound_proc(&mut self, binding: &NativeProcedureCommand, argv: &[*mut TclObj]) -> Code {
        use tcl_registry::native_procedure::{
            NativeProcedureCompilationPurpose, NativeProcedureRecompileAction,
        };
        use tcl_runtime_api::native_procedure_roles::NativeProcedureRoleOwner;
        let mut declaration = binding.declaration();
        if let Err(error) = declaration.check_native_liveness() {
            return self.report_cmd_error(error.into());
        }
        if declaration.native.is_none()
            && !self.original_procedure_artifact_is_current(&declaration)
        {
            if let Some(protocol) = tcl_registry::native_procedure::procedure_activation_protocol(
                self.native_invocation_dialect(),
            ) {
                match protocol.recompilation_action(
                    NativeProcedureCompilationPurpose::CommandBody,
                    declaration.native_procedure_role_ledger().references(),
                ) {
                    Some(NativeProcedureRecompileAction::ReplaceSharedDeclaration) => {
                        declaration = match binding.replace_for_recompilation() {
                            Ok(declaration) => declaration,
                            Err(error) => return self.report_cmd_error(error.into()),
                        };
                    }
                    Some(NativeProcedureRecompileAction::RetainDeclaration) | None => {}
                }
            }
        }
        self.call_proc(&declaration, argv)
    }

    /// Call a user proc (`TclObjInterpProc`): a thin wrapper over [`run_proc`]
    /// with the proc's `(params, body, ns)` and the call args (`argv[1..]`).
    ///
    /// [`run_proc`]: Interp::run_proc
    fn call_proc(&mut self, def: &Rc<ProcDef>, argv: &[*mut TclObj]) -> Code {
        let original_body = match def
            .check_native_liveness()
            .and_then(|()| def.body.checked_ptr())
        {
            Ok(original) => original,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let jim_parameters = match def
            .jim_parameters
            .as_ref()
            .map(obj::ProcedureObject::checked_ptr)
            .transpose()
        {
            Ok(original) => original,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let name = obj_bytes(argv[0]);
        let body = match tcl_syntax::value::ValueOps::native_string_bytes(self, &original_body) {
            Ok(body) => body,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let location = def.location();
        self.run_proc(
            &def.params,
            &body,
            location.namespace,
            &argv[1..],
            &name,
            CallMeta {
                original_argv: Some(argv),
                err: ProcFrame::Proc(&name),
                fqn: Some(&location.qualified_name),
                source: def.source.clone(),
                body_line_base: def.body_line_base,
                link_vars: &[],
                oo_variable_resolver: None,
                keep_loop_codes: false,
                same_level: false,
                usage_prefix: None,
                level_words: None,
                quote_name: true,
                native: def.native,
                statics: def.native_statics(),
                c_procedure: Some(def),
                c_method_client_data: None,
                jim_parameters,
                jim_body: self.uses_jim_error_stack().then_some(original_body),
                jim_namespace: location.jim_namespace.as_ref(),
            },
        )
    }

    pub(crate) fn schedule_tailcall(&mut self, words: &[*mut TclObj]) -> Code {
        if !self.in_proc() {
            return self.error(b"tailcall can only be called from a proc, lambda or method");
        }
        let Some(protocol) = self.native_invocation_dialect().tailcall_protocol() else {
            return self.error(b"native tailcall protocol is not selected");
        };
        if protocol == tcl_registry::invocation_words::NativeTailcallProtocol::Tcl {
            // The generic handler retires the previous request before producing
            // a namespace operand; an empty invocation creates no replacement.
            if !self.replace_pending_tailcall(None) {
                return self.error(b"tailcall can only be called from a proc, lambda or method");
            }
            if words.is_empty() {
                self.set_result_bytes(b"");
                return Code::Return;
            }
            let namespace = match tcl_cmd_core::namespace::current_original(self) {
                Ok(original) => crate::obj::Owned::fresh(original),
                Err(error) => return self.report_cmd_error(error),
            };
            let Some(strings) = self.native_invocation_dialect().native_string_protocol() else {
                return self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original tailcall List issuer",
                    )
                    .into(),
                );
            };
            let mut members = Vec::with_capacity(words.len() + 1);
            members.push(namespace.as_ptr());
            members.extend_from_slice(words);
            let original =
                crate::obj::Owned::fresh(crate::list::new_list_obj_native(&members, strings));
            return self.schedule_original_tailcall_list(original);
        }
        self.set_result_bytes(b"");
        if words.is_empty() && !protocol.schedules_empty() {
            return Code::Ok;
        }
        let request = crate::frame::PendingTailcall {
            namespace: self.current_ns.get(),
            namespace_name: self
                .namespaces
                .borrow()
                .qualified_name(self.current_ns.get()),
            original_list: None,
            words: words
                .iter()
                .map(|word| crate::obj::Owned::retain(*word))
                .collect(),
        };
        if !self.replace_pending_tailcall(Some(request)) {
            return self.error(b"tailcall can only be called from a proc, lambda or method");
        }
        Code::from_int(protocol.pending_code())
    }

    pub(crate) fn schedule_original_tailcall_list(&mut self, original: crate::obj::Owned) -> Code {
        if !self.in_proc() {
            return self.error(b"tailcall can only be called from a proc, lambda or method");
        }
        // Compiled operands already exist. Retire the old List before examining
        // the new one, and release a namespace-only cancellation at this opcode.
        if !self.replace_pending_tailcall(None) {
            return self.error(b"tailcall can only be called from a proc, lambda or method");
        }
        let Some(protocol) = self.native_invocation_dialect().native_string_protocol() else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original tailcall List issuer",
                )
                .into(),
            );
        };
        let members = match crate::list::list_elements_native_checked(original.as_ptr(), protocol) {
            Ok(members) => members,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        if members.len() <= 1 {
            drop(original);
            self.set_result_bytes(b"");
            return Code::Return;
        }
        let request = crate::frame::PendingTailcall {
            namespace: self.current_ns.get(),
            namespace_name: Vec::new(),
            words: Vec::new(),
            original_list: Some(original),
        };
        if !self.replace_pending_tailcall(Some(request)) {
            return self.error(b"tailcall can only be called from a proc, lambda or method");
        }
        self.set_result_bytes(b"");
        Code::Return
    }

    fn replace_pending_tailcall(&mut self, request: Option<crate::frame::PendingTailcall>) -> bool {
        let retired = { self.frames.borrow_mut().replace_tailcall(request) };
        match retired {
            Ok(previous) => {
                drop(previous);
                true
            }
            Err(rejected) => {
                drop(rejected);
                false
            }
        }
    }

    fn dispatch_tailcall(&mut self, request: crate::frame::PendingTailcall) -> Code {
        if let Some(original) = request.original_list.as_ref() {
            return self.invoke_original_namespace_list(original.as_ptr());
        }
        let words: Vec<_> = request
            .words
            .iter()
            .map(crate::obj::Owned::as_ptr)
            .collect();
        let lookup = {
            let namespaces = self.namespaces.borrow();
            if namespaces.namespace_is_live(request.namespace) {
                Some(request.namespace)
            } else {
                namespaces.find_namespace(GLOBAL, &request.namespace_name)
            }
        };
        let Some(lookup) = lookup else {
            let message = [
                b"namespace \"".as_slice(),
                &request.namespace_name,
                b"\" not found",
            ]
            .concat();
            let error_code =
                error_code_list(&[b"TCL", b"LOOKUP", b"NAMESPACE", &request.namespace_name]);
            return self.error_with_code(&message, &error_code);
        };
        self.dispatch_original_words_at(&words, lookup)
    }

    pub(crate) fn invoke_original_namespace_list(&mut self, original: *mut TclObj) -> Code {
        let Some(protocol) = self.native_invocation_dialect().native_string_protocol() else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original coroutine List issuer",
                )
                .into(),
            );
        };
        let members = match crate::list::list_elements_native_checked(original, protocol) {
            Ok(members) => members,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let Some((namespace, words)) = members.split_first() else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original coroutine namespace operand",
                )
                .into(),
            );
        };
        if words.is_empty() {
            self.set_result_bytes(b"");
            return Code::Ok;
        }
        let lookup = match self.native_namespace_object_lookup(*namespace) {
            Ok(Some(namespace)) => namespace,
            Ok(None) => {
                return crate::cmd_namespace::ns_operation_not_found(
                    self,
                    *namespace,
                    tcl_syntax::naming::NativeNamespaceLookupOperation::ObjectLookup,
                );
            }
            Err(error) => return self.report_cmd_error(error.into()),
        };
        self.dispatch_original_words_at(words, lookup)
    }

    fn dispatch_original_words_at(&mut self, words: &[*mut TclObj], lookup: NsId) -> Code {
        let Some(head) = words.first() else {
            self.set_result_bytes(b"");
            return Code::Ok;
        };
        if let Err(code) = self.reset_native_ensemble_rewrite(
            tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent::BeforeOrdinaryLookup,
        ) {
            return code;
        }
        let selected = match self.resolve_original_command_at(lookup, *head) {
            Ok(selected) => selected,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let selection = match selected {
            Some((command, Some(generation))) => {
                let fqn = self
                    .namespaces
                    .borrow()
                    .native_command_at_node(generation)
                    .map(|(_, fqn)| fqn);
                let Some(fqn) = fqn else {
                    return self.report_cmd_error(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "tailcall command allocation",
                        )
                        .into(),
                    );
                };
                CommandDispatchSelection::Bound(
                    fqn,
                    CommandBinding {
                        generation,
                        command,
                        jim_table_key: None,
                    },
                )
            }
            Some((command, None)) => return self.invoke_bound(command, None, words),
            None => CommandDispatchSelection::Missing {
                lookup,
                caller: self.current_ns.get(),
                origin: tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
            },
        };
        self.dispatch_prebound_inner(words, selection, true)
    }

    fn drain_tailcalls(&mut self, depth: u32, mut code: Code) -> Code {
        loop {
            let request = {
                let mut requests = self.deferred_tailcalls.borrow_mut();
                if requests.last().is_some_and(|(owner, _)| *owner == depth) {
                    requests.pop().map(|(_, request)| request)
                } else {
                    None
                }
            };
            let Some(request) = request else {
                return code;
            };
            if code == Code::Ok {
                code = self.dispatch_tailcall(request);
            }
        }
    }

    /// The shared proc-call protocol (`TclObjInterpProc`), used by both `proc`
    /// dispatch and `apply`: arity-check `call_args` against `params`, push a call
    /// frame in namespace `ns`, bind the params (defaults; an `args` catch-all
    /// collects the rest), run `body`, then pop. A body-level `return` becomes
    /// `Ok`; an escaping `break`/`continue` is an error. `usage_called` is the
    /// prefix of the `wrong # args` message (`name` for a proc, `apply
    /// lambdaExpr` for `apply`). Conservative-first per
    /// `proc-call-and-stack-traces.md` PC-2 (the CmdFrame/stack-trace +
    /// `info level`/`info frame` bookkeeping land with PC-1/PC-4/PC-5).
    pub(crate) fn run_proc<O: obj::ObjectPointer>(
        &mut self,
        params: &[Param<O>],
        body: &[u8],
        ns: NsId,
        call_args: &[*mut TclObj],
        usage_called: &[u8],
        mut meta: CallMeta,
    ) -> Code {
        let issuer_dispatch = self.native_dispatch_depth.get();
        let original_artifact = if let Some(procedure) = meta.c_procedure {
            let original = match procedure
                .check_native_liveness()
                .and_then(|()| procedure.body.checked_ptr())
            {
                Ok(original) => original,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            match self.prepare_original_c_body(original, ns, Some(procedure)) {
                Ok(artifact) => artifact,
                Err(code) => return code,
            }
        } else {
            None
        };
        let mut native_admission = match self.prepare_native_procedure(body, ns) {
            Ok(admission) => admission,
            Err(failure) => {
                return self.admit_native_compilation_error(&failure, Some(usage_called));
            }
        };
        // InvokeProcedureMethod retains its ProcedureMethod after compiling the
        // body, before the proc core binds formals. Replacement withdraws the
        // table's clientData reference while this entered owner stays live.
        let _method_execution = match meta
            .c_method_client_data
            .map(|method| method.enter_method())
            .transpose()
        {
            Ok(execution) => execution,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let usage = meta.usage_prefix.as_deref().unwrap_or(usage_called);
        let supplied = call_args.len();
        let Some(grammar) = self.native_invocation_dialect().parameter_grammar() else {
            return self.error(b"formal parameter dialect is not selected");
        };
        let Ok(bindings) = native_compilation::native_parameter_plan(params, supplied, grammar)
        else {
            return match self.proc_wrong_args(usage, params, supplied, meta.quote_name) {
                Ok(message) => self.wrong_arguments_message(&message),
                Err(error) => self.report_cmd_error(error.into()),
            };
        };
        if grammar.skips_empty_body_activation() && body.is_empty() {
            self.set_result_bytes(b"");
            return Code::Ok;
        }
        let jim_body = meta.jim_body;
        let (saved_ns, proc_frame) = match self.enter_native_procedure_frame(
            params,
            call_args,
            usage_called,
            (ns, &mut meta),
            bindings,
            original_artifact
                .as_ref()
                .and_then(|artifact| artifact.compiled_local_layout()),
        ) {
            Ok(frame) => frame,
            Err(code) => return code,
        };
        if matches!(meta.err, ProcFrame::Method { .. }) {
            self.oo_bind_method_activation();
        }
        // Run the compiled body when there is one, else the source body. The
        // two are interchangeable here precisely because everything a body can
        // observe about its call — the variable frame, its namespace, `info
        // level`'s words, the bound formals, `wrong # args` — was decided
        // above, and everything about its completion is decided below.
        //
        // Two things force the source body even when a compiled one exists:
        //
        // - a live step trace. `dispatch_traced` installs this proc's own
        //   `enterstep`/`leavestep` into `step_active` *before* dispatching, so
        //   one read covers both "this proc is step-traced" and "an outer step
        //   trace is running". A natively lowered `set`/`incr`/`expr` never
        //   reaches `dispatch`, so it would fire no step trace and the
        //   transcript would silently lose lines. `enter`/`leave` traces need
        //   nothing: they fire around the whole call either way.
        // - the entry declining, which it may only do before any observable
        //   effect, so falling through to the source body is not a partial
        //   re-run.
        let native = meta
            .native
            .filter(|_| self.traces.borrow().step_active.is_empty());
        self.clear_return_options();
        native_admission.activate();
        let jim_level = self.jim_procedure_level.get();
        if self.uses_jim_error_stack() {
            self.jim_procedure_level.set(jim_level + 1);
        }
        let execution = self.acquire_native_procedure_execution(meta.c_procedure);
        let code = match execution {
            Err(error) => self.report_cmd_error(error.into()),
            Ok(()) => match native {
                Some(entry) => match self.run_native_body(entry, call_args, *proc_frame) {
                    Ok(code) => code,
                    Err(frame) => self.run_selected_procedure_body(
                        body,
                        original_artifact.as_ref(),
                        jim_body,
                        *frame,
                    ),
                },
                None => self.run_selected_procedure_body(
                    body,
                    original_artifact.as_ref(),
                    jim_body,
                    *proc_frame,
                ),
            },
        };
        self.release_native_procedure_execution();
        self.jim_procedure_level.set(jim_level);
        // The frame's local variables (and any traces on them) die with it.
        let proc_level = self.frames.borrow().current_level();
        // Capture `[info level 0]` (the invocation words) before the frame is
        // popped — the TIP 348 `CALL` entry if this body unwinds with an error.
        // The locals' unset traces are collected while the frame — and its
        // arrays' elements — still exist, and fire once it is gone, as C's
        // `TclDeleteVars` runs over a frame that is on its way out.
        let teardown = if self.traces.borrow().traces.is_empty() {
            Vec::new()
        } else {
            self.frame_teardown_unset_traces(proc_level)
        };
        let tailcall = self.frames.borrow_mut().take_tailcall();
        self.clean_current_jim_local_commands();
        let popped = self.pop_native_call_frame();
        if self.has_variable_traces() {
            self.clear_frame_var_traces(proc_level);
        }
        self.current_ns.set(saved_ns);
        self.fire_unset_callbacks(teardown);
        self.leave_namespace_activation(popped);
        self.recursion_depth.set(self.recursion_depth.get() - 1);
        // Apply the return boundary (`return`/`return -code -level`), then a
        // bare `break`/`continue` that escaped the body (no enclosing loop) is an
        // error (C Tcl: `invoked "break" outside of a loop`).
        // A *bare* `break`/`continue` command escaping the body (no enclosing
        // loop) is the `invoked "break" outside of a loop` error; but `return
        // -code break` (the body completed with `Code::Return`) propagates the
        // raw completion code unchanged — C distinguishes these, e.g. an
        // ensemble `-unknown` handler that does `return -code break` yields code
        // 3, not the loop error (namespace-47.4).
        let tailcall_code = self
            .native_invocation_dialect()
            .tailcall_protocol()
            .map(|protocol| i64::from(protocol.pending_code()));
        if let Some(tailcall) = tailcall.filter(|_| {
            code == Code::Ok || code == Code::Return || Some(code.as_int()) == tailcall_code
        }) {
            self.clear_return_options();
            self.deferred_tailcalls
                .borrow_mut()
                .push((issuer_dispatch, tailcall));
            self.set_result_bytes(b"");
            return Code::Ok;
        }
        let from_return = code == Code::Return;
        let settled = match self.settle_return(code) {
            Code::Break if !meta.keep_loop_codes && !from_return => {
                self.error(b"invoked \"break\" outside of a loop")
            }
            Code::Continue if !meta.keep_loop_codes && !from_return => {
                self.error(b"invoked \"continue\" outside of a loop")
            }
            other => other,
        };
        // On error, append the `(procedure "name" line N)` / `(lambda term ...)`
        // frame and clear `already_logged` so the proc-call command logs next.
        // Skipped when the error was produced by the return boundary itself
        // (`return -code error`, i.e. the body completed with `Code::Return`):
        // C only adds the procedure frame when the error unwinds *through* the
        // body, not when `return` synthesises it (error-6.7, result-6.2).
        if settled == Code::Error {
            if code != Code::Return {
                self.make_proc_error(meta.err);
            } else {
                // `return -code error`: no procedure frame, but the *caller* still
                // logs its own `invoked from within "<call>"` frame, so release
                // the already-logged flag that `process_return_error` may have set
                // for an explicit `-errorinfo` (error-6.7).
                self.clear_error_logged();
            }
        }
        settled
    }

    /// Execute the body selected before activation and formal binding. A native
    /// entry that declines uses this same retained selection and untouched frame.
    fn run_selected_procedure_body(
        &mut self,
        body: &[u8],
        original_artifact: Option<&Rc<native_body_artifact::NativeBodyArtifact>>,
        jim_body: Option<*mut TclObj>,
        frame: CmdFrame,
    ) -> Code {
        if let Some(artifact) = original_artifact {
            self.execute_original_c_body(artifact, frame)
        } else if let Some(original) = jim_body {
            self.eval_original_body_framed(
                tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
                original,
                frame,
            )
        } else {
            self.eval_framed(body, frame)
        }
    }

    /// Run a proc's **compiled** body in the call frame
    /// [`run_proc`](Self::run_proc) has already prepared — the native-tier
    /// mirror of [`eval_framed`](Self::eval_framed) +
    /// [`eval_script_mode`](Self::eval_script_mode), minus the parse and the
    /// command loop.
    ///
    /// Ownership, stated here because the emitted side has to match it
    /// exactly: `run_proc` owns the Tcl variable frame, `info level`'s words
    /// and the formal binding; this method owns the compiled activation and
    /// the body's `CmdFrame`; the entry owns **neither** (see
    /// [`NativeProcEntry`]).
    ///
    /// `Ok(code)` is the body's raw completion — `2` for a body-level
    /// `return`, exactly as the interpreted body reports it, so `run_proc`'s
    /// `settle_return` cannot tell the two apart. `Err(frame)` is a decline:
    /// nothing observable happened, and the untouched frame comes back so the
    /// source body can run in it. Handing the frame back rather than cloning
    /// it up front keeps the common path allocation-free; the box is what
    /// keeps a rarely-taken `Err` from widening every return of this function.
    fn run_native_body(
        &mut self,
        entry: NativeProcEntry,
        call_args: &[*mut TclObj],
        mut frame: CmdFrame,
    ) -> Result<Code, Box<CmdFrame>> {
        // A freshly pushed frame is its own `codePtr->source`, so `errorLine`
        // is measured from this body's base (`eval_framed`).
        frame.proc_line_base = frame.line_base;
        // The activation the eval loop would hold for this body. A refusal is
        // `eval_script_mode`'s: the error is already set, no activation is
        // held, and nothing has been pushed, so `run_proc`'s ordinary error
        // tail is exactly right — including the `(procedure "p" line N)` frame.
        if !self.codegen_activation_enter() {
            return Ok(Code::Error);
        }
        self.cmd_frames.borrow_mut().push(frame);
        // `Tcl_EvalEx` resets the result at entry; `eval_script_mode` can skip
        // that because its first command always sets one. A compiled body
        // cannot promise the same — `set`/`incr`/`puts` write no interpreter
        // result — so a completion that comes back with a null result means
        // "the body left the result alone", and that has to be the empty
        // string the eval loop would have produced, never the caller's.
        self.set_result_bytes(b"");
        let mut out = crate::codegen_abi::TclCompletionAbi {
            code: 0,
            result: core::ptr::null_mut(),
            options: core::ptr::null_mut(),
        };
        let argc = i32::try_from(call_args.len()).unwrap_or(i32::MAX);
        // SAFETY: `entry` was supplied by a generated module through
        // `tcl_codegen_proc_define_native`; `argv[..argc]` are the live bound
        // call arguments it borrows; `out` is live, zeroed completion storage.
        let status = unsafe { entry(call_args.as_ptr(), argc, core::ptr::from_mut(&mut out)) };
        let popped = self.cmd_frames.borrow_mut().pop();
        if status == tcl_runtime_api::codegen_abi::NATIVE_PROC_STATUS_HOST_REFUSED {
            self.codegen_activation_leave(Code::Error);
            return Ok(Code::Error);
        }
        if status == NATIVE_PROC_STATUS_DECLINED {
            self.codegen_activation_leave(Code::Ok);
            return Err(Box::new(
                popped.expect("the CmdFrame this method just pushed"),
            ));
        }
        // Counted here, not before the call: a decline is not a native run.
        self.native_proc_dispatches
            .set(self.native_proc_dispatches.get().saturating_add(1));
        let code = Code::from_int(out.code);
        if !out.result.is_null() {
            // `set_result` takes the interpreter's own reference, so the
            // entry's is released after the store rather than transferred.
            self.set_result(out.result);
            // SAFETY: the entry transferred one owned reference on `result`.
            unsafe { obj::decr_ref_count(out.result) };
        }
        if !out.options.is_null() {
            // The options dict is a snapshot of state the runtime already
            // holds: `return`'s `-code`/`-level` were recorded by the `return`
            // command when the body invoked it, and `settle_return` below
            // reads those, not this dict. So it is released, never consulted.
            // SAFETY: the entry transferred one owned reference on `options`.
            unsafe { obj::decr_ref_count(out.options) };
        }
        self.codegen_activation_leave(code);
        Ok(code)
    }

    /// The number of proc bodies dispatched through a [`NativeProcEntry`].
    pub(crate) fn native_proc_dispatches(&self) -> u64 {
        self.native_proc_dispatches.get()
    }

    fn ensemble_wrong_args_for_invocation(
        &mut self,
        argv: &[*mut TclObj],
        parameters: &[Vec<u8>],
    ) -> Code {
        let Some(configuration) = self
            .native_invocation_dialect()
            .native_ensemble_configuration_protocol()
        else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "ensemble argument usage",
                )
                .into(),
            );
        };
        let mut message = b"wrong # args: should be \"".to_vec();
        message.extend_from_slice(&obj_bytes(argv[0]));
        for parameter in parameters {
            message.push(b' ');
            crate::list::append_list_element(&mut message, parameter, false);
        }
        message.push(b' ');
        message.extend_from_slice(configuration.missing_selector_usage());
        message.push(b'"');
        self.wrong_arguments_message(&message)
    }

    /// The ensemble trampoline: resolve `argv[1]` against the subcommand set
    /// (exact, then unambiguous prefix unless `-prefixes 0`), map it to a target
    /// command prefix (`-map`, else `<ns>::<sub>`), and re-dispatch
    /// `[target… , argv[2..]…]`. Mirrors C Tcl's `tclEnsemble.c` (the A3 contract).
    fn dispatch_ensemble(
        &mut self,
        token: &crate::ensemble::EnsembleToken,
        argv: &[*mut TclObj],
    ) -> Code {
        let mut current = token.config();
        let mut reparsed = false;
        loop {
            // Re-read every structural field after an empty `-unknown` result:
            // the callback can reconfigure the ensemble before asking Tcl to
            // parse the same invocation again.
            let cfg = &current;
            let Some(layout) =
                tcl_cmd_core::ensemble::invocation_layout(argv.len(), 1, cfg.parameters.len())
            else {
                return self.ensemble_wrong_args_for_invocation(argv, &cfg.parameters);
            };
            let Some(configuration) = self
                .native_invocation_dialect()
                .native_ensemble_configuration_protocol()
            else {
                return self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "ensemble member names",
                    )
                    .into(),
                );
            };
            let original_sub = obj_bytes(argv[layout.subcommand]);
            let sub = configuration.member_name(&original_sub);
            let subs = self.ensemble_subcommands(cfg);
            if let Some(idx) = configuration.resolve_member(&subs, &original_sub, cfg.prefixes) {
                let resolved = &subs[idx];
                if let Err(error) = self.build_original_ensemble_table(cfg) {
                    return self.report_cmd_error(error.into());
                }
                let Some((prefix, mapped)) = cfg.originals.table.prefix(resolved) else {
                    return self.report_cmd_error(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "ensemble original selected prefix",
                        )
                        .into(),
                    );
                };
                let prefix = obj::Owned::retain(prefix);
                let default_target = !mapped;
                // Spell-fix the subcommand to its resolved name in the recorded
                // source, so an abbreviated `ev` is reported as `event` (C's
                // `TclSpellFix`).
                let mut source: Vec<_> = argv
                    .iter()
                    .map(|&word| {
                        EnsembleRewriteWord::Borrowed(crate::obj::NativeObjectLifetime::retain(
                            word,
                        ))
                    })
                    .collect();
                source[layout.subcommand] =
                    EnsembleRewriteWord::Owned(crate::obj::Owned::fresh(new_string(resolved)));
                return self.dispatch_ensemble_target(
                    prefix,
                    cfg.ns,
                    argv,
                    &layout,
                    source,
                    default_target.then_some(resolved.as_slice()),
                );
            }
            // Miss: try the `-unknown` handler once.
            if !cfg.unknown.is_empty() && !reparsed {
                reparsed = true;
                match self.ensemble_unknown(token, cfg, argv) {
                    EnsembleUnknown::Prefix(prefix) => {
                        let live = token.config();
                        let Some(live_layout) = tcl_cmd_core::ensemble::invocation_layout(
                            argv.len(),
                            1,
                            live.parameters.len(),
                        ) else {
                            return self.ensemble_wrong_args_for_invocation(argv, &live.parameters);
                        };
                        let source: Vec<_> = argv
                            .iter()
                            .map(|&word| {
                                EnsembleRewriteWord::Borrowed(
                                    crate::obj::NativeObjectLifetime::retain(word),
                                )
                            })
                            .collect();
                        return self.dispatch_ensemble_target(
                            prefix,
                            live.ns,
                            argv,
                            &live_layout,
                            source,
                            None,
                        );
                    }
                    EnsembleUnknown::Reparse => {
                        current = token.config();
                        continue;
                    }
                    EnsembleUnknown::Failed(code) => return code,
                }
            }
            // A namespace ensemble with no subcommands at all gets a distinct
            // message; otherwise "unknown or ambiguous" (prefixes on) / "unknown"
            // (prefixes off) followed by the candidate list (C's
            // `NsEnsembleImplementationCmdNR`).
            let ecode = error_code_list(&[b"TCL", b"LOOKUP", b"SUBCOMMAND", sub]);
            let ns_fqn = self.namespaces.borrow().qualified_name(cfg.ns);
            let m = tcl_cmd_core::ensemble::unknown_subcommand_message(
                &subs,
                sub,
                cfg.prefixes,
                &ns_fqn,
            );
            return self.error_with_code(&m, &ecode);
        }
    }

    /// The ensemble's valid subcommand set (sorted, deduped): explicit
    /// `-subcommands`, else the `-map` keys, else the namespace's exports.
    fn ensemble_subcommands(&self, cfg: &crate::ensemble::EnsembleConfig) -> Vec<Vec<u8>> {
        let Some(protocol) = self
            .native_invocation_dialect()
            .native_ensemble_configuration_protocol()
        else {
            return Vec::new();
        };
        let keys: Vec<_> = cfg
            .map
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|(key, _)| key.as_slice())
            .collect();
        let explicit = cfg
            .subcommands
            .as_ref()
            .map(|names| names.iter().map(Vec::as_slice).collect::<Vec<_>>());
        let exports = self.namespaces.borrow().exported_commands(cfg.ns);
        let exports: Vec<_> = exports.iter().map(Vec::as_slice).collect();
        let map = crate::ensemble::NativeEnsembleObjects::pointer(&cfg.originals.map);
        let same = map.is_some()
            && map == crate::ensemble::NativeEnsembleObjects::pointer(&cfg.originals.subcommands);
        protocol
            .table_plan(explicit.as_deref(), &keys, &exports, same)
            .entries
            .into_iter()
            .map(|entry| entry.member)
            .collect()
    }

    /// Dispatch a resolved ensemble subcommand: `[prefix…, params…, rest…]` (the
    /// `-parameters` values thread in right after the target prefix; the
    /// subcommand's own args follow). `layout` is always computed by the shared
    /// ensemble owner from the live token configuration.
    fn dispatch_ensemble_target(
        &mut self,
        prefix: obj::Owned,
        namespace: NsId,
        argv: &[*mut TclObj],
        layout: &tcl_cmd_core::ensemble::InvocationLayout,
        source: Vec<EnsembleRewriteWord>,
        default_name: Option<&[u8]>,
    ) -> Code {
        let Some(dispatch) = self
            .native_invocation_dialect()
            .native_ensemble_dispatch_protocol()
        else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native ensemble forwarding",
                )
                .into(),
            );
        };
        let Some(string) = self
            .eval_frame_dialect()
            .native_string_materialization(None)
        else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native ensemble invocation List",
                )
                .into(),
            );
        };
        let protocol = string.protocol();
        let members = match crate::list::list_elements_native_checked(prefix.as_ptr(), protocol) {
            Ok(members) => members,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let prefix_len = members.len();
        let invocation = if dispatch.always_copy_prefix || argv.len() == 2 {
            match crate::list::native_list_copy(prefix.as_ptr(), protocol) {
                Ok(copy) => copy,
                Err(error) => return self.report_cmd_error(error.into()),
            }
        } else {
            let mut words = members;
            words.extend_from_slice(&argv[layout.parameters.clone()]);
            words.extend_from_slice(&argv[layout.arguments..]);
            obj::Owned::fresh(crate::list::new_list_obj_native(&words, protocol))
        };
        let mut words =
            match crate::list::list_elements_native_checked(invocation.as_ptr(), protocol) {
                Ok(words) => words,
                Err(error) => return self.report_cmd_error(error.into()),
            };
        if dispatch.always_copy_prefix {
            words.extend_from_slice(&argv[layout.parameters.clone()]);
            words.extend_from_slice(&argv[layout.arguments..]);
        }
        let prefix = if dispatch.always_copy_prefix {
            Some(prefix)
        } else {
            drop(prefix);
            None
        };
        let lookup = if dispatch.use_ensemble_namespace {
            namespace
        } else {
            GLOBAL
        };
        let default_target_was_missing = default_name.is_some()
            && self
                .resolve_dispatchable_with_generation(lookup, &obj_bytes(words[0]))
                .is_none();
        let root = self.begin_original_ensemble_rewrite(
            source,
            layout.arguments,
            prefix_len + layout.parameters.len(),
        );
        let code = self.dispatch_selection_with_entry(
            &words,
            CommandDispatchSelection::LookupAt(
                lookup,
                tcl_registry::command_lookup::CommandLookupOrigin::EnsembleInvocation,
            ),
            false,
        );
        if code == Code::Error && default_target_was_missing {
            if let Some(name) = default_name {
                let expected = [
                    b"invalid command name \"".as_slice(),
                    obj_bytes(words[0]).as_slice(),
                    b"\"",
                ]
                .concat();
                if obj_bytes(self.get_obj_result()) == expected {
                    let message = [b"invalid command name \"".as_slice(), name, b"\""].concat();
                    self.set_result_bytes(&message);
                }
            }
        }
        if root {
            self.clear_ensemble_rewrite();
        }
        drop(invocation);
        drop(prefix);
        code
    }

    /// Invoke an ensemble's `-unknown` handler on a subcommand miss
    /// (`EnsembleUnknownCallback`): `handler… ensembleFQN argv[1..]…`. An empty
    /// `TCL_OK` result asks for a reparse (the handler defined the subcommand); a
    /// non-empty one is the replacement command prefix; anything else fails.
    fn ensemble_unknown(
        &mut self,
        token: &crate::ensemble::EnsembleToken,
        cfg: &crate::ensemble::EnsembleConfig,
        argv: &[*mut TclObj],
    ) -> EnsembleUnknown {
        let ens_fqn = token.name();
        let Some(protocol) = self
            .eval_frame_dialect()
            .native_string_materialization(None)
            .map(|recipe| recipe.protocol())
        else {
            return EnsembleUnknown::Failed(
                self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "ensemble unknown original prefix",
                    )
                    .into(),
                ),
            );
        };
        let Some(handler) = crate::ensemble::NativeEnsembleObjects::pointer(&cfg.originals.unknown)
        else {
            return EnsembleUnknown::Failed(
                self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "ensemble unknown configured root",
                    )
                    .into(),
                ),
            );
        };
        let call = obj::Owned::fresh(obj::duplicate(handler));
        let fqn = obj::Owned::fresh(new_string(&ens_fqn));
        let mut additions = vec![fqn.as_ptr()];
        additions.extend_from_slice(&argv[1..]);
        if let Err(error) =
            crate::list::append_prepared_native_elements(call.as_ptr(), &additions, protocol)
        {
            return EnsembleUnknown::Failed(self.report_cmd_error(error.into()));
        }
        drop(fqn);
        let hv = match crate::list::list_elements_native_checked(call.as_ptr(), protocol) {
            Ok(words) => words,
            Err(error) => return EnsembleUnknown::Failed(self.report_cmd_error(error.into())),
        };
        let mut handler_call = Vec::new();
        for (index, &word) in hv.iter().enumerate() {
            if index != 0 {
                handler_call.push(b' ');
            }
            crate::list::append_list_element(&mut handler_call, &obj_bytes(word), false);
        }
        let code = self.dispatch(&hv);
        match code {
            Code::Ok => {
                if token.is_deleted() {
                    let code = self.error_with_code(
                        tcl_cmd_core::ensemble::UNKNOWN_DELETED_MESSAGE.as_bytes(),
                        tcl_cmd_core::ensemble::UNKNOWN_DELETED_ERROR_CODE.as_bytes(),
                    );
                    self.append_frame_noline(b"ensemble unknown subcommand handler");
                    return EnsembleUnknown::Failed(code);
                }
                let result = obj::Owned::retain(self.get_obj_result());
                self.set_result_bytes(b"");
                drop(call);
                match crate::list::list_elements_native_checked(result.as_ptr(), protocol) {
                    Ok(words) if !words.is_empty() => EnsembleUnknown::Prefix(result),
                    Ok(_) => EnsembleUnknown::Reparse,
                    Err(error) => {
                        let code = self.report_cmd_error(error.into());
                        self.append_error_info_context(
                            b"while parsing result of ensemble unknown subcommand handler",
                        );
                        EnsembleUnknown::Failed(code)
                    }
                }
            }
            Code::Error => {
                if let Some(command) =
                    parse::parse_script_with_config(&handler_call, self.lexer_config()).first()
                {
                    self.log_command_info(&handler_call, command);
                }
                self.append_frame_noline(b"ensemble unknown subcommand handler");
                EnsembleUnknown::Failed(Code::Error)
            }
            other => {
                let mut m = b"unknown subcommand handler returned bad code: ".to_vec();
                match other {
                    Code::Return => m.extend_from_slice(b"return"),
                    Code::Break => m.extend_from_slice(b"break"),
                    Code::Continue => m.extend_from_slice(b"continue"),
                    Code::Other(value) => m.extend_from_slice(value.to_string().as_bytes()),
                    Code::Ok | Code::Error => unreachable!("handled above"),
                }
                let code = self.error_with_code(&m, b"TCL ENSEMBLE UNKNOWN_RESULT");
                let mut context = b"result of ensemble unknown subcommand handler: ".to_vec();
                context.extend_from_slice(&handler_call);
                self.append_error_info_context(&context);
                EnsembleUnknown::Failed(code)
            }
        }
    }

    /// Invoke `::tcl::mathfunc::<fname>` (resolved **absolutely**, so it works
    /// from any current namespace) with `args` as `objv` — the hook `expr`'s
    /// function-call path uses so a user-defined / overridden / renamed
    /// `::tcl::mathfunc::NAME` wins (the A3 contract). Leaves the result in the
    /// interp result; a missing command reports C's `invalid command name
    /// "tcl::mathfunc::NAME"` (no leading `::`, matching tclsh).
    #[cfg(have_tommath)]
    pub(crate) fn eval_math_call(&mut self, fname: &[u8], args: &[*mut TclObj]) -> Code {
        // C's expr emits the *relative* name `tcl::mathfunc::NAME`, resolved
        // from the CURRENT namespace by the ordinary command rule — so
        // `::ns::tcl::mathfunc::NAME` shadows the global function inside
        // `::ns` (tclsh 8.6.16/9.0.4-pinned; see the mathfunc rows in the
        // shared conformance vector table). The global `::tcl::mathfunc`
        // is simply the final fall-through base.
        let mut rel = b"tcl::mathfunc::".to_vec();
        rel.extend_from_slice(fname);
        let cmd = self.resolve_dispatchable_with_generation(self.current_ns.get(), &rel);
        let Some((cmd, generation)) = cmd else {
            let mut m = b"invalid command name \"tcl::mathfunc::".to_vec();
            m.extend_from_slice(fname);
            m.push(b'"');
            let error_code = error_code_list(&[b"TCL", b"LOOKUP", b"COMMAND", &rel]);
            return self.error_with_code(&m, &error_code);
        };
        // Build [name, args…], each owned (+1), and invoke the resolved command
        // directly (already resolved above; no second resolution).
        let mut argv: Vec<*mut TclObj> = Vec::with_capacity(args.len() + 1);
        let name_obj = new_string(&rel);
        // SAFETY: name_obj is fresh; args are live; take the owning +1 the argv holds.
        unsafe { obj::incr_ref_count(name_obj) };
        argv.push(name_obj);
        for &a in args {
            unsafe { obj::incr_ref_count(a) };
            argv.push(a);
        }
        let code = self.invoke_bound(cmd, generation, &argv);
        release_all(&argv);
        code
    }

    /// The alias trampoline (`docs/design/runtime/rename-alias.md` §4.2): resolve
    /// the stored `target` by name **anchored at the global namespace** (so a
    /// target deleted after the alias was created surfaces lazily here, but a
    /// *renamed* target is not followed), synthesise
    /// `[target, *prefix, *caller_tail]`, and invoke. Alias-of-alias chains fall
    /// out naturally (the resolved target may itself be an `Alias`) and are
    /// bounded by [`MAX_ALIAS_DISPATCH_DEPTH`], so a cycle that escaped the
    /// definition-time gate errors instead of exhausting the native stack. The
    /// wrapper begins a fresh completion-option scope before invoking its
    /// target, including across an interpreter boundary.
    fn dispatch_alias(
        &mut self,
        target: &[u8],
        prefix: &[Vec<u8>],
        jim_prefix: Option<&obj::Owned>,
        argv: &[*mut TclObj],
    ) -> Code {
        self.begin_control_options(
            tcl_runtime_api::completion_options::ControlOptionPolicy::FRESH_FORWARDED,
        );
        if ALIAS_DISPATCH_DEPTH.with(|d| d.get()) >= MAX_ALIAS_DISPATCH_DEPTH {
            return self.error(b"too many nested alias invocations (infinite loop?)");
        }
        // Build [target, *prefix, *argv[1..]] — each element owned (+1).
        let mut new_argv: Vec<*mut TclObj> = Vec::with_capacity(prefix.len() + argv.len());
        let push_owned = |v: &mut Vec<*mut TclObj>, o: *mut TclObj| {
            // SAFETY: `o` is a live object; take the owning +1 the new argv holds.
            unsafe { obj::incr_ref_count(o) };
            v.push(o);
        };
        if let Some(original) = jim_prefix {
            let words = match tcl_syntax::value::ValueOps::list_elements(self, &original.as_ptr()) {
                Ok(words) => words,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            for word in words {
                push_owned(&mut new_argv, word);
            }
        } else {
            push_owned(&mut new_argv, new_string(target));
            for p in prefix {
                push_owned(&mut new_argv, new_string(p));
            }
        }
        for &a in &argv[1..] {
            push_owned(&mut new_argv, a);
        }
        ALIAS_DISPATCH_DEPTH.with(|d| d.set(d.get() + 1));
        let lookup = if self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_some()
        {
            self.current_ns.get()
        } else {
            GLOBAL
        };
        let code = match self.resolve_original_command_at(lookup, new_argv[0]) {
            Ok(Some((command, generation))) => self.invoke_bound(command, generation, &new_argv),
            Ok(None) => self.dispatch_missing_command(
                &new_argv,
                lookup,
                self.current_ns.get(),
                tcl_registry::command_lookup::CommandLookupOrigin::AliasInvocation,
            ),
            Err(error) => self.report_cmd_error(error.into()),
        };
        ALIAS_DISPATCH_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
        release_all(&new_argv);
        code
    }

    /// The `invalid command name "X"` error (the resolver miss; `unknown` later).
    pub(crate) fn invalid_command(&mut self, name: &[u8]) -> Code {
        self.invalid_command_result(name)
    }

    /// C's `Tcl_FindCommand` + `TCL_LEAVE_ERR_MSG` miss (`unknown command "X"`,
    /// `tclNamesp.c`) — distinct from `invalid_command`'s `invalid command
    /// name`. Used by `trace add|remove|info command|execution`.
    pub(crate) fn unknown_command(&mut self, name: &[u8]) -> Code {
        let mut msg = b"unknown command \"".to_vec();
        msg.extend_from_slice(name);
        msg.push(b'"');
        self.error(&msg)
    }

    /// Jim sugar enters the expression evaluator without command lookup or a
    /// fabricated script frame. Its result is owned by the caller.
    fn eval_word_expression(&mut self, source: &[u8]) -> Result<*mut TclObj, Code> {
        #[cfg(have_tommath)]
        {
            if let Err(error) = core::str::from_utf8(source) {
                return Err(self.refuse_unicode_access(
                    tcl_syntax::raw_string::UnicodeAccessError {
                        valid_up_to: error.valid_up_to(),
                        error_len: error.error_len(),
                    },
                ));
            }
            crate::builtins::eval_expr_obj(self, source)
        }
        #[cfg(not(have_tommath))]
        {
            let _ = source;
            Err(self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::ExpressionEngineUnavailable,
            ))
        }
    }

    fn word_expression_bytes(&mut self, source: &[u8]) -> Result<Vec<u8>, Code> {
        let value = self.eval_word_expression(source)?;
        let bytes = obj_bytes(value);
        // SAFETY: expression evaluation transfers one owning reference.
        unsafe { obj::decr_ref_count(value) };
        Ok(bytes)
    }

    /// Substitute one word's body into an **owned** (`+1`) object.
    /// A `Variable` reference to an unset variable, or a `[cmd]` that errors,
    /// returns `Err(code)` with the interp result already set.
    fn substitute_word(&mut self, src: &[u8], body: &WordBody) -> Result<*mut TclObj, Code> {
        match body {
            WordBody::Literal(bytes) => {
                let obj = new_string(bytes);
                unsafe { obj::incr_ref_count(obj) };
                Ok(obj)
            }
            WordBody::Parts(parts) => {
                // Object-passthrough fast path: a word that is
                // *exactly one* substitution returns that value's **object**
                // (preserving its internal rep), not a stringified copy. This is
                // what keeps `$list`→`lindex`/`llength` etc. O(1) instead of
                // re-shimmering the string each access (the hidden-O(N²) seam).
                if parts.len() == 1 {
                    match &parts[0] {
                        WordPart::Variable(v) => {
                            let index = match &v.index {
                                Some(p) => Some(self.subst_index_value(p)?),
                                None => None,
                            };
                            if let Some(c) = self.fire_read_trace(v.name, index.as_deref()) {
                                return Err(c);
                            }
                            let obj = match index.as_deref() {
                                Some(key) => self.var_get_elem(v.name, key),
                                None => self.var_get(v.name),
                            };
                            return match obj {
                                Some(o) => {
                                    // SAFETY: `o` is a live store-owned object;
                                    // we take an owning +1 to hand to the caller.
                                    unsafe { obj::incr_ref_count(o) };
                                    Ok(o)
                                }
                                None => Err(self.no_such_variable(v.name, index.as_deref())),
                            };
                        }
                        WordPart::Expression(expression) => {
                            return self.eval_word_expression(expression);
                        }
                        WordPart::Command(script) => {
                            // Command substitution propagates *any* non-OK
                            // completion code (`return`/`break`/`continue`, not
                            // just error) out of `[...]`, matching C Tcl.
                            let code = self.eval_command_subst(src, script);
                            if code != Code::Ok {
                                return Err(code);
                            }
                            let r = self.result.get();
                            // SAFETY: the interp result is live; take an owning +1.
                            unsafe { obj::incr_ref_count(r) };
                            return Ok(r);
                        }
                        // single Text/Backslash → fall through to the buffer path
                        _ => {}
                    }
                }
                let mut buf: Vec<u8> = Vec::new();
                for part in parts {
                    match part {
                        WordPart::Text(b) => buf.extend_from_slice(b),
                        WordPart::Expression(expression) => {
                            buf.extend_from_slice(&self.word_expression_bytes(expression)?);
                        }
                        WordPart::Variable(v) => {
                            let index = match &v.index {
                                Some(parts) => Some(self.subst_index_value(parts)?),
                                None => None,
                            };
                            if let Some(c) = self.fire_read_trace(v.name, index.as_deref()) {
                                return Err(c);
                            }
                            match self.read_var(v.name, index.as_deref()) {
                                Some(bytes) => buf.extend_from_slice(&bytes),
                                None => return Err(self.no_such_variable(v.name, index.as_deref())),
                            }
                        }
                        WordPart::Command(script) => {
                            let code = self.eval_command_subst(src, script);
                            if code != Code::Ok {
                                return Err(code);
                            }
                            buf.extend_from_slice(&self.result_bytes());
                        }
                        WordPart::ParseError(msg) => return Err(self.error(msg.as_bytes())),
                    }
                }
                let obj = new_string(&buf);
                unsafe { obj::incr_ref_count(obj) };
                Ok(obj)
            }
        }
    }

    /// Evaluate a quoted expression or raw index through its original arena.
    /// Command completions follow this purpose, independently of `subst`.
    #[cfg(have_tommath)]
    pub(crate) fn do_expression_subst(
        &mut self,
        source: &[u8],
        quoted: bool,
    ) -> Result<Vec<u8>, Code> {
        use tcl_lexer::{ExecutablePart, ExecutablePartArena, SourceImage, Span};
        use tcl_registry::invocation_words::{
            ExpressionQuoteControl, LogicalExpressionQuoteProvider,
        };
        let dialect = self.native_invocation_dialect();
        let policy = if quoted {
            Some(dialect.expression_quote_control().or_else(|| dialect.logical_expression_quote_control(LogicalExpressionQuoteProvider::Tcl84CoreSimulation)).ok_or_else(|| self.refuse_native_access(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("native expression quote settlement")))?)
        } else {
            None
        };
        let end = u32::try_from(source.len()).map_err(|_| {
            self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native expression operand extent",
                ),
            )
        })?;
        let arena = ExecutablePartArena::decompose(
            SourceImage::native(source),
            Span::new(0, end),
            Default::default(),
            self.lexer_config(),
        )
        .map_err(|_| {
            self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native expression operand geometry",
                ),
            )
        })?;
        let mut stack = vec![(arena.root(), 0usize, Vec::<u8>::new(), None::<Vec<u8>>)];
        loop {
            let frame = stack.last_mut().expect("operand root frame");
            let Some(component) = arena.list(frame.0).get(frame.1) else {
                let (_, _, out, _) = stack.pop().expect("completed operand");
                let Some(parent) = stack.last_mut() else {
                    return Ok(out);
                };
                let name = parent.3.take().expect("index receiver");
                if let Some(code) = self.fire_read_trace(&name, Some(&out)) {
                    return Err(code);
                }
                let value = self
                    .read_var(&name, Some(&out))
                    .ok_or_else(|| self.no_such_variable(&name, Some(&out)))?;
                parent.2.extend_from_slice(&value);
                continue;
            };
            frame.1 += 1;
            match &component.part {
                ExecutablePart::Text(_) => {
                    let protocol = self.source_string_protocol().ok_or_else(|| {
                        self.refuse_native_access(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("native executable text recipe"))
                    })?;
                    let text = tcl_syntax::backslash::native_arena_text(
                        &arena, component, self.lexer_config().escapes, protocol,
                    ).map_err(|_| self.refuse_native_access(
                        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                            "native executable text decoding",
                        ),
                    ))?;
                    frame.2.extend_from_slice(&text);
                }
                ExecutablePart::Variable {
                    name,
                    index: Some(index),
                } => {
                    frame.3 = Some(arena.bytes(*name).expect("arena name geometry").to_vec());
                    stack.push((*index, 0, Vec::new(), None));
                }
                ExecutablePart::Variable { name, index: None } => {
                    let name = arena.bytes(*name).expect("arena name geometry");
                    if let Some(code) = self.fire_read_trace(name, None) {
                        return Err(code);
                    }
                    let value = self
                        .read_var(name, None)
                        .ok_or_else(|| self.no_such_variable(name, None))?;
                    frame.2.extend_from_slice(&value);
                }
                ExecutablePart::Command { body } => {
                    let code = self.eval_command_subst(
                        arena.image().bytes(),
                        arena.bytes(*body).expect("arena command geometry"),
                    );
                    let code = if policy == Some(ExpressionQuoteControl::Jim084) {
                        match code {
                            Code::Return => {
                                self.return_code.set(Code::Ok);
                                Code::Ok
                            }
                            Code::Break => self.error(b"invoked \"break\" outside of a loop"),
                            Code::Continue => self.error(b"invoked \"continue\" outside of a loop"),
                            Code::Other(_) => Code::Error,
                            other => other,
                        }
                    } else {
                        code
                    };
                    if code != Code::Ok {
                        return Err(code);
                    }
                    frame.2.extend_from_slice(&self.result_bytes());
                }
                ExecutablePart::Expression { expression } => {
                    frame.2.extend_from_slice(&self.word_expression_bytes(
                        arena.bytes(*expression).expect("arena expression geometry"),
                    )?)
                }
                ExecutablePart::ParseError(message) => return Err(self.error(message.as_bytes())),
            }
        }
    }

    /// Substitute with the input string's TIP 280 location
    /// (the `subst` command's argument word): a `[...]` inside the substituted
    /// string then reports the line it appears on (C compiles `subst` with the
    /// argument's line table). `loc` is `None` for internal callers that do not
    /// track lines (`dict map` key substitution), where the `[...]` is line-
    /// tracked relative to the enclosing frame as usual.
    pub(crate) fn do_subst_located(
        &mut self,
        src: &[u8],
        flags: crate::subst::SubstFlags,
        loc: Option<(Option<Rc<[u8]>>, u32)>,
    ) -> Result<Vec<u8>, Code> {
        let body = crate::subst::scan(src, flags, self.lexer_config());
        match &body {
            WordBody::Literal(b) => Ok(b.to_vec()),
            WordBody::Parts(parts) => {
                // Align the enclosing frame to the argument word's file+line, so
                // each `[...]`'s line (computed by `eval_command_subst` against
                // `src`) is file-absolute. Saved/restored around the resolution.
                let saved = loc.and_then(|(file, line)| {
                    let mut frames = self.cmd_frames.borrow_mut();
                    frames.last_mut().map(|top| {
                        let prev = (top.line_base, top.file.clone());
                        top.line_base = line.saturating_sub(1);
                        if file.is_some() {
                            top.file = file;
                        }
                        prev
                    })
                });
                let result = self.resolve_subst_parts(src, parts);
                if let Some((line_base, file)) = saved {
                    if let Some(top) = self.cmd_frames.borrow_mut().last_mut() {
                        top.line_base = line_base;
                        top.file = file;
                    }
                }
                result
            }
        }
    }

    /// Resolve substitution `parts` (scanned from `src`) to bytes, propagating
    /// any non-OK code (the `subst`-command path; cf. `substitute_word`, which
    /// builds an object). `src` lets a `[...]` part advance the `info frame`
    /// line to its own position via [`eval_command_subst`](Self::eval_command_subst).
    fn resolve_subst_parts(&mut self, src: &[u8], parts: &[WordPart]) -> Result<Vec<u8>, Code> {
        let mut out = Vec::new();
        for part in parts {
            match part {
                WordPart::Text(b) => out.extend_from_slice(b),
                WordPart::Expression(expression) => {
                    out.extend_from_slice(&self.word_expression_bytes(expression)?);
                }
                WordPart::Variable(v) => {
                    // A `break`/`continue`/`return` from a `[...]` in the array
                    // index diverts the whole variable substitution (C's
                    // `TCL_TOKEN_VARIABLE` arm of `TclSubstTokens`): on `break`
                    // the subst ends, on `continue` this variable contributes
                    // nothing, on `return` the result is substituted in place of
                    // the variable's value (which is not looked up).
                    let (index, idx_code) = match &v.index {
                        Some(p) => {
                            let (val, code) = self.subst_index(p)?;
                            (Some(val), code)
                        }
                        None => (None, Code::Ok),
                    };
                    match idx_code {
                        Code::Ok => {
                            if let Some(c) = self.fire_read_trace(v.name, index.as_deref()) {
                                return Err(c);
                            }
                            match self.read_var(v.name, index.as_deref()) {
                                Some(bytes) => out.extend_from_slice(&bytes),
                                None => return Err(self.no_such_variable(v.name, index.as_deref())),
                            }
                        }
                        Code::Break => break,
                        Code::Continue => {}
                        _ => out.extend_from_slice(&self.result_bytes()),
                    }
                }
                WordPart::Command(script) => {
                    // `subst`'s per-`[...]` completion-code rule (C's compiled
                    // subst / `TclSubstTokens`): `break` ends the whole
                    // substitution (returning what's accumulated), `continue`
                    // contributes nothing for this bracket, and any other
                    // non-error code (`return`, custom) substitutes its result.
                    // Only a genuine error propagates.
                    match self.eval_command_subst(src, script) {
                        Code::Ok | Code::Return | Code::Other(_) => {
                            out.extend_from_slice(&self.result_bytes())
                        }
                        Code::Break => break,
                        Code::Continue => {}
                        Code::Error => return Err(Code::Error),
                    }
                }
                // Reached in evaluation order, so `[...]` parts before it have
                // already run and kept their side effects — C's behaviour.
                WordPart::ParseError(msg) => return Err(self.error(msg.as_bytes())),
            }
        }
        Ok(out)
    }

    /// [`subst_index`](Self::subst_index) for the word-substitution path, where a
    /// non-OK completion code in the index propagates as an error/code (rather
    /// than diverting an enclosing `subst`).
    fn subst_index_value(&mut self, parts: &[WordPart]) -> Result<Vec<u8>, Code> {
        match self.subst_index(parts)? {
            (val, Code::Ok) => Ok(val),
            (_, code) => Err(code),
        }
    }

    /// Resolve a `$arr(index)` index (itself substituted) to its bytes plus the
    /// completion code of the last `[...]` in it (`Ok` normally; `Break`/
    /// `Continue`/`Return` divert the enclosing variable substitution per C's
    /// `TclSubstTokens`). An error propagates as `Err`.
    fn subst_index(&mut self, parts: &[WordPart]) -> Result<(Vec<u8>, Code), Code> {
        let mut buf = Vec::new();
        for part in parts {
            match part {
                WordPart::Text(b) => buf.extend_from_slice(b),
                WordPart::Expression(expression) => {
                    buf.extend_from_slice(&self.word_expression_bytes(expression)?);
                }
                WordPart::Variable(v) => {
                    let (idx, idx_code) = match &v.index {
                        Some(p) => {
                            let (val, code) = self.subst_index(p)?;
                            (Some(val), code)
                        }
                        None => (None, Code::Ok),
                    };
                    match idx_code {
                        Code::Ok => match self.read_var(v.name, idx.as_deref()) {
                            Some(bytes) => buf.extend_from_slice(&bytes),
                            None => return Err(self.no_such_variable(v.name, idx.as_deref())),
                        },
                        other => return Ok((buf, other)),
                    }
                }
                WordPart::Command(script) => match self.eval_str(script) {
                    Code::Ok => buf.extend_from_slice(&self.result_bytes()),
                    Code::Error => return Err(Code::Error),
                    Code::Return => {
                        buf.extend_from_slice(&self.result_bytes());
                        return Ok((buf, Code::Return));
                    }
                    other => return Ok((buf, other)),
                },
                WordPart::ParseError(msg) => return Err(self.error(msg.as_bytes())),
            }
        }
        Ok((buf, Code::Ok))
    }

    /// Read a variable's value bytes via the variable resolver.
    fn read_var(&mut self, name: &[u8], index: Option<&[u8]>) -> Option<Vec<u8>> {
        if self.observed_name_policy_selected() {
            let value = self.observed_substitution_receiver(name, index).ok()??;
            return tcl_syntax::value::ValueOps::native_string_bytes(self, &value)
                .ok()
                .map(|bytes| bytes.to_vec());
        }
        self.require_variable_name_protocol().ok()?;
        let input = match index {
            Some(index) => self.separate_variable_input(name, Some(index)),
            None => self.combined_variable_input(name),
        }
        .ok()?;
        let name = input.root().selected();
        let index = input.element().map(|element| element.selected());
        crate::vars::resolve_var_bytes(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            self.current_ns.get(),
            name,
            index,
        )
    }

    fn no_such_variable(&mut self, name: &[u8], index: Option<&[u8]>) -> Code {
        if self.host_refusal_pending() {
            return Code::Error;
        }
        let msg = self.read_miss_msg(name, index);
        self.error(&msg)
    }

    /// Build the C-faithful `can't read "NAME": …` message for a failed variable
    /// read, distinguishing the three cases `tclVar.c` reports: a scalar read of
    /// an array (`variable is array`), a missing element of an *existing* array
    /// (`no such element in array`), and a wholly missing variable (`no such
    /// variable`). `base`/`index` are the split reference (`base(index)`).
    pub(crate) fn read_miss_msg(&self, original: &[u8], index: Option<&[u8]>) -> Vec<u8> {
        use tcl_syntax::naming::{
            NativeVariableDiagnosticOperation as Operation,
            NativeVariableDiagnosticReason as Reason, NativeVariableFailureSite as Site,
            NativeVariableInputForm,
        };
        let input = match index {
            Some(element) => self.separate_variable_input(original, Some(element)),
            None => self.combined_variable_input(original),
        };
        let Ok(input) = input else {
            return Vec::new();
        };
        let root = input.root().selected();
        let element = input.element().map(|element| element.selected());
        let reason = if self.var_is_array(root) {
            if element.is_some() {
                Reason::NoSuchElement
            } else {
                Reason::IsArray
            }
        } else if element.is_some() && self.var_get(root).is_some() {
            Reason::NotArray
        } else {
            Reason::NoSuchVariable
        };
        let form = match index {
            Some(element) => NativeVariableInputForm::Separate {
                root: original,
                element: Some(element),
            },
            None => NativeVariableInputForm::Combined(original),
        };
        let protocol = input.root().protocol();
        let diagnostic = tcl_syntax::naming::report_native_variable_diagnostic_at(
            protocol,
            Operation::Read,
            reason,
            if reason == Reason::IsArray {
                Site::ValueRead
            } else {
                Site::NameLookup
            },
            form,
        );
        let Ok(diagnostic) = diagnostic else {
            self.clone().refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "variable read diagnostic",
                ),
            );
            return Vec::new();
        };
        let mut message = b"can't read \"".to_vec();
        message.extend_from_slice(&diagnostic.name);
        message.extend_from_slice(b"\": ");
        message.extend_from_slice(diagnostic.reason.message().as_bytes());
        message
    }

    /// Set an error result and return [`Code::Error`] — for builtins.
    pub(crate) fn set_error(&mut self, msg: &[u8]) -> Code {
        self.error(msg)
    }

    /// Set the canonical `wrong # args: should be "usage"` arity error and
    /// return [`Code::Error`] (`Tcl_WrongNumArgs` with a literal usage) — the
    /// one home for the builtins' arity message, formerly a per-`cmd_*.rs`
    /// copy.
    /// Render a runtime handler's usage with the actual invoked command word.
    pub(crate) fn wrong_args_for_invocation(
        &mut self,
        arguments: &[*mut TclObj],
        suffix: &[u8],
    ) -> Code {
        self.wrong_args_for_prefix(arguments, 1, suffix)
    }

    /// Render actual leading argument values through the retained dispatch
    /// rewrite and the selected native usage quoting policy, preserving bytes.
    pub(crate) fn wrong_args_for_prefix(
        &mut self,
        arguments: &[*mut TclObj],
        prefix_words: usize,
        suffix: &[u8],
    ) -> Code {
        let mut usage = match self.argument_usage_prefix(arguments, prefix_words) {
            Ok(usage) => usage,
            Err(code) => return code,
        };
        if !suffix.is_empty() {
            if !usage.is_empty() {
                usage.push(b' ');
            }
            usage.extend_from_slice(suffix);
        }
        self.wrong_args(&usage)
    }

    /// Render only the retained actual handler header, without touching operands.
    pub(crate) fn argument_usage_prefix(
        &mut self,
        arguments: &[*mut TclObj],
        prefix_words: usize,
    ) -> Result<Vec<u8>, Code> {
        let Some(protocol) = self.native_invocation_dialect().usage_protocol(Some(
            tcl_registry::native_usage::LogicalUsageProvider::Tcl84CoreSimulation,
        )) else {
            return Err(self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "argument usage",
                ),
            ));
        };
        // The typed tuples preserve canonical Index expansion through prefix
        // rewrites. Their byte allocations confer no object ownership roles.
        let mut header = Vec::new();
        for &original in arguments.iter().take(prefix_words) {
            let (bytes, index) = self
                .native_index_usage_bytes(original)
                .map_err(|error| self.report_cmd_error(error.into()))?;
            header.push((bytes, index, false));
        }
        let mut rewrites = Vec::new();
        if let Some(rewrite) = self.ensemble_rewrite() {
            let mut original_prefix = Vec::new();
            for word in rewrite.source.iter().take(rewrite.removed) {
                let (bytes, index) = self
                    .native_index_usage_bytes(word.as_ptr())
                    .map_err(|error| self.report_cmd_error(error.into()))?;
                original_prefix.push((bytes, index, true));
            }
            rewrites.push(tcl_cmd_core::ensemble::ArgumentUsageRewrite {
                original_prefix,
                removed_words: rewrite.inserted,
            });
        }
        // Lifetime-only pointers borrow the installed handler's original words;
        // native string/canonical updaters invoke no guest callback here.
        let adapter = self
            .handler_usage_adapters
            .borrow()
            .iter()
            .rev()
            .find(|adapter| arguments.starts_with(&adapter.parse_prefix))
            .map(|adapter| {
                (
                    adapter.parse_prefix.len(),
                    adapter
                        .original_prefix
                        .iter()
                        .map(|word| word.as_ptr())
                        .collect::<Vec<_>>(),
                )
            });
        if let Some((removed_words, originals)) = adapter {
            if prefix_words < removed_words {
                rewrites.clear();
            } else {
                let mut original_prefix = Vec::new();
                for original in originals {
                    let (bytes, index) = self
                        .native_index_usage_bytes(original)
                        .map_err(|error| self.report_cmd_error(error.into()))?;
                    original_prefix.push((bytes, index, true));
                }
                rewrites.push(tcl_cmd_core::ensemble::ArgumentUsageRewrite {
                    original_prefix,
                    removed_words,
                });
            }
        }
        let header = tcl_cmd_core::ensemble::rewrite_argument_usage(&header, &rewrites);
        let operands = header
            .iter()
            .map(|(bytes, index, rewritten)| {
                if !*index {
                    tcl_registry::native_usage::NativeUsageWord::Original(bytes)
                } else if *rewritten {
                    tcl_registry::native_usage::NativeUsageWord::RewrittenIndex(bytes)
                } else {
                    tcl_registry::native_usage::NativeUsageWord::CanonicalIndex(bytes)
                }
            })
            .collect::<Vec<_>>();
        protocol.render_header(&operands).ok_or_else(|| {
            self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "argument usage",
                ),
            )
        })
    }

    pub(crate) fn wrong_args(&mut self, usage: &[u8]) -> Code {
        self.report_cmd_error(tcl_cmd_core::CmdError::wrong_args_bytes(usage))
    }

    pub(crate) fn wrong_arguments_message(&mut self, message: &[u8]) -> Code {
        self.report_cmd_error(tcl_cmd_core::CmdError::wrong_arguments_message_bytes(
            message,
        ))
    }
}

/// The `wrong # args: should be "name p1 ?p2? ?arg ...?"` message for a proc
/// call — required params bare, defaulted params `?p?`, the `args` catch-all
/// `?arg ...?` (mirrors C's `Tcl_WrongNumArgs` for procs).
impl Interp {
    /// The `wrong # args` message for a proc/method, applying any active
    /// ensemble-rewrite so the call is reported as the user wrote it (C's
    /// `Tcl_WrongNumArgs` rewrite path). When a rewrite is active and all the
    /// inserted words are accounted for, the leading `removed` words of the
    /// original `source` replace the rewritten prefix, and the formal parameters
    /// already satisfied by the inserted arguments are dropped.
    pub(crate) fn proc_wrong_args<O: obj::ObjectPointer>(
        &self,
        called: &[u8],
        params: &[Param<O>],
        supplied: usize,
        quote_name: bool,
    ) -> Result<Vec<u8>, tcl_syntax::value::ValueError> {
        let protocol = self
            .native_invocation_dialect()
            .usage_protocol(Some(
                tcl_registry::native_usage::LogicalUsageProvider::Tcl84CoreSimulation,
            ))
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "procedure usage",
            ))?;
        if let Some(rw) = self.ensemble_rewrite() {
            // Print the first `removed` words of `source` (the chained
            // `numRemovedObjs`) in place of the target prefix, then the formal
            // parameters not already satisfied by the supplied arguments. Using
            // the runtime `supplied` count (rather than the static `inserted`)
            // keeps the dropped-parameter count right across an OO forward, where
            // the inserted prefix words (`my method …`) are dispatch verbs that do
            // not themselves fill the target's parameters.
            let user_args = rw.source.len().saturating_sub(rw.removed);
            let drop = supplied.saturating_sub(user_args);
            // Only rewrite when the dropped parameters are actually present (C's
            // `objc < toSkip` guard); otherwise fall back to the plain message.
            if drop <= params.len() {
                let values: Vec<_> = rw
                    .source
                    .iter()
                    .take(rw.removed)
                    .map(|word| obj_bytes(word.as_ptr()))
                    .collect();
                let prefix: Vec<_> = values.iter().map(Vec::as_slice).collect();
                return Ok(proc_usage_words(
                    protocol,
                    &prefix,
                    &params[drop..],
                    self.native_invocation_dialect().parameter_grammar(),
                ));
            }
        }
        Ok(proc_usage(
            protocol,
            called,
            params,
            quote_name,
            self.native_invocation_dialect().parameter_grammar(),
        ))
    }
}

/// Build a `wrong # args` message from explicit leading `words` followed by the
/// formal `shown` parameters (`all` is the full parameter list, for the `args`
/// catch-all test). Shared by the plain and ensemble-rewritten forms.
fn proc_usage_words<O: obj::ObjectPointer>(
    protocol: tcl_registry::native_usage::NativeUsageProtocol,
    words: &[&[u8]],
    shown: &[Param<O>],
    grammar: Option<tcl_dialect::ParameterGrammar>,
) -> Vec<u8> {
    let mut header = Vec::new();
    for (index, word) in words.iter().enumerate() {
        if index > 0 {
            header.push(b' ');
        }
        header.extend_from_slice(word);
    }
    protocol.render_procedure_message(&header, &formal_usage(shown, grammar))
}

fn formal_usage<O: obj::ObjectPointer>(
    params: &[Param<O>],
    grammar: Option<tcl_dialect::ParameterGrammar>,
) -> Vec<u8> {
    let Some(grammar) = grammar else {
        return Vec::new();
    };
    let parameters = native_compilation::formal_parameters(params, grammar);
    tcl_syntax::formal_params::formal_parameter_usage_bytes(&parameters, grammar)
}

fn proc_usage<O: obj::ObjectPointer>(
    protocol: tcl_registry::native_usage::NativeUsageProtocol,
    called: &[u8],
    params: &[Param<O>],
    quote_name: bool,
    grammar: Option<tcl_dialect::ParameterGrammar>,
) -> Vec<u8> {
    let header = protocol.render_procedure_name(called, quote_name);
    protocol.render_procedure_message(&header, &formal_usage(params, grammar))
}

/// The word a call was written with — `argv[0]`, C's `objv[0]`.
///
/// This is what `Tcl_WrongNumArgs` prints, and it is deliberately *not* the
/// name a command is filed under: the two differ after a `rename`, under a
/// qualified spelling, and through an alias. Empty for an argv with no words,
/// which no dispatch path produces.
fn invoked_word(argv: &[*mut TclObj]) -> Vec<u8> {
    argv.first().map_or_else(Vec::new, |&word| obj_bytes(word))
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

/// Collapse every run of two-or-more `:` to a single `::` separator, matching
/// Tcl's namespace-name normalisation (empty namespace components are ignored):
/// `::::classinstance` → `::classinstance`, `::a:::b` → `::a::b`. A lone `:` is
/// a legal identifier character and is left untouched.
fn normalize_colons(name: &[u8]) -> Vec<u8> {
    if !name.windows(3).any(|w| w == b":::") {
        return name.to_vec();
    }
    let mut out = Vec::with_capacity(name.len());
    let mut i = 0;
    while i < name.len() {
        if name[i] == b':' {
            let mut j = i;
            while j < name.len() && name[j] == b':' {
                j += 1;
            }
            let run = j - i;
            if run >= 2 {
                out.extend_from_slice(b"::");
            } else {
                out.push(b':');
            }
            i = j;
        } else {
            out.push(name[i]);
            i += 1;
        }
    }
    out
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
mod tests {
    use super::*;
    use crate::counters;

    #[test]
    fn primitive_getter_preserves_seeded_code_until_script_propagation() {
        use tcl_syntax::scalar_getter::{
            NativeScalarGetterFailure, NativeScalarGetterKind, NativeScalarGetterProtocol,
        };
        leak_free(|interp| {
            interp.set_error_state(b"PROBE BEFORE");
            interp.mark_error_code_explicit();
            let protocol =
                NativeScalarGetterProtocol::for_tcl_version(tcl_dialect::TclVersion::V8_5);
            let record = protocol
                .failure_presentation(
                    NativeScalarGetterKind::Wide,
                    NativeScalarGetterFailure::CachedNonInteger,
                    b"1.5\0X",
                )
                .expect("audited cached C8.5 getter failure");
            let primitive_message = record.message_bytes().to_vec();
            let propagated_message = record.eval_message_bytes().to_vec();
            assert_ne!(primitive_message, propagated_message);
            let error = tcl_syntax::value::ValueError::NativeScalarGetter(Box::new(record));
            assert_eq!(interp.report_cmd_error(error.into()), Code::Error);
            assert_eq!(interp.result_bytes(), primitive_message);
            assert_eq!(interp.error_code(), b"PROBE BEFORE");
            assert!(interp.exc.borrow().code_explicit);

            let snapshot = interp.snapshot_error();
            interp.error_with_code(b"other", b"OTHER");
            interp.set_result_bytes(&primitive_message);
            interp.restore_error(snapshot);
            interp.log_command_bytes(1, b"getter");
            assert_eq!(interp.result_bytes(), propagated_message);
            assert_eq!(interp.error_code(), b"PROBE BEFORE");
            assert!(interp.exc.borrow().primitive_getter.is_none());

            let record = protocol
                .failure_presentation(
                    NativeScalarGetterKind::Wide,
                    NativeScalarGetterFailure::CachedNonInteger,
                    b"1.5\0X",
                )
                .expect("same primitive stage inside contextual command error");
            let error = tcl_cmd_core::CmdError::from(
                tcl_syntax::value::ValueError::NativeScalarGetter(Box::new(record)),
            );
            let mut details = error.into_byte_details();
            details.message.splice(0..0, b"context: ".iter().copied());
            interp.report_cmd_error(tcl_cmd_core::CmdError::from_byte_details(details));
            interp.log_command_bytes(1, b"getter");
            let mut wrapped = b"context: ".to_vec();
            wrapped.extend_from_slice(&propagated_message);
            assert_eq!(interp.result_bytes(), wrapped);
        });
    }

    #[test]
    #[cfg(have_tommath)]
    fn c84_direct_invalid_expression_preserves_code_until_eval() {
        // Native proof: naming.expression.original-direct-arithmetic-error-stage
        // docs/design/analysis/name-resolution-proofs/expression-original-direct-arithmetic-error-stage.md

        leak_free(|interp| {
            interp.set_runtime_version(tcl_dialect::TclVersion::V8_4);
            for source in [b"\"bad\" + 1".as_slice(), b"\"bad\" ? 1 : 0".as_slice()] {
                interp.set_error_state(b"PROBE BEFORE");
                assert_eq!(
                    crate::builtins::eval_expr_obj(interp, source),
                    Err(Code::Error)
                );
                assert_eq!(interp.error_code(), b"PROBE BEFORE");
                assert!(interp.exc.borrow().expression_error_stage.is_some());
                interp.log_command_bytes(1, b"expr");
                assert_eq!(interp.error_code(), b"NONE");
            }
            interp.set_error_state(b"PROBE BEFORE");
            assert_eq!(
                crate::builtins::eval_expr_obj(interp, b"NaN + 1"),
                Err(Code::Error)
            );
            assert_ne!(interp.error_code(), b"PROBE BEFORE");
            assert!(interp.exc.borrow().expression_error_stage.is_none());
            let domain = interp.error_code();
            interp.log_command_bytes(1, b"expr");
            assert_eq!(interp.error_code(), domain);
        });
    }

    #[test]
    #[cfg(have_tommath)]
    fn counted_syntax_failures_keep_direct_code_and_apply_eval_update() {
        for &(engine, case, source, message, direct, propagated) in
            tcl_test_support::expressions::NATIVE_EXPRESSION_SYNTAX_STATES
        {
            leak_free(|interp| {
                interp.set_dialect_profile(crate::environment::profile_for_dialect(engine));
                interp.set_error_state(b"PROBE BEFORE");
                assert_eq!(
                    crate::builtins::eval_expr_obj(interp, source),
                    Err(Code::Error),
                    "{engine}/{case}"
                );
                assert_eq!(interp.result_bytes(), message, "{engine}/{case}");
                assert_eq!(interp.error_code(), direct, "{engine}/{case} Direct");
                let snapshot = interp.snapshot_error();
                interp.error_with_code(b"other failure", b"OTHER");
                interp.set_result_bytes(message);
                interp.restore_error(snapshot);
                interp.log_command_bytes(1, b"expr $probeSource");
                assert_eq!(interp.error_code(), propagated, "{engine}/{case} Eval");
                assert_eq!(interp.result_bytes(), message, "{engine}/{case}");
                assert!(interp.exc.borrow().expression_error_stage.is_none());
            });
        }
    }

    #[test]
    fn native_wrong_arguments_string_result_requires_the_structured_presenter() {
        // naming.variable.original-upvar-and-exists-completion-and-name-windows
        // docs/design/analysis/name-resolution-proofs/variable.original-upvar-and-exists-completion-and-name-windows.md
        // naming.list.original-assign-objects-and-instructions
        // docs/design/analysis/name-resolution-proofs/list.original-assign-objects-and-instructions.md
        // The original tables independently observe C84 upvar case11 and
        // C85 lassign case28 String results. This tests the shared presenter.
        use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let error =
                tcl_cmd_core::CmdError::wrong_args_bytes(b"upvar ?level? otherVar localVar");
            let message = error.message_bytes().to_vec();
            assert_eq!(interp.report_cmd_error(error), Code::Error);
            assert!(!interp.host_refusal_pending(), "{engine}");
            assert!(
                matches!(
                    obj::native_object_snapshot(interp.result_obj())
                        .unwrap()
                        .cache,
                    Cache::String { .. }
                ),
                "{engine}"
            );
            assert_eq!(interp.result_bytes(), message, "{engine}");
            assert_eq!(
                interp.report_cmd_error(tcl_cmd_core::CmdError::new_bytes(message.as_slice())),
                Code::Error
            );
            assert!(
                matches!(
                    obj::native_object_snapshot(interp.result_obj())
                        .unwrap()
                        .cache,
                    Cache::None
                ),
                "{engine}: unrelated message"
            );
        }
    }

    #[test]
    fn unchanged_primitive_projection_preserves_the_original_string_result() {
        // naming.compiler.introspection-source-and-effect-frontiers
        // docs/design/analysis/name-resolution-proofs/compiler-introspection-source-and-effect-frontiers.md
        // Native case13 observes the selected integer-getter String result.
        // This separate owner control checks unchanged propagation identity.
        use tcl_syntax::scalar_getter::{
            NativeScalarGetterFailure, NativeScalarGetterKind, NativeScalarGetterProtocol,
        };
        for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let version = interp.native_invocation_dialect().tcl_version.unwrap();
            let record = NativeScalarGetterProtocol::for_tcl_version(version)
                .failure_presentation(
                    NativeScalarGetterKind::Int,
                    NativeScalarGetterFailure::Invalid,
                    b"invalid",
                )
                .expect("retained native integer failure");
            let error = tcl_cmd_core::CmdError::from(
                tcl_syntax::value::ValueError::NativeScalarGetter(Box::new(record)),
            )
            .with_native_string_result(tcl_syntax::native_string::NativeStringProtocol::C(version));
            assert_eq!(interp.report_cmd_error(error), Code::Error);
            let original = obj::Owned::retain(interp.result_obj());
            let before = obj::native_object_snapshot(original.as_ptr()).unwrap();
            assert!(matches!(
                before.cache,
                tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
            ));
            interp.propagate_error_stage();
            assert_eq!(interp.result_obj(), original.as_ptr(), "{engine}");
            assert_eq!(
                obj::native_object_snapshot(interp.result_obj())
                    .unwrap()
                    .cache,
                before.cache,
                "{engine}"
            );
            assert!(interp.exc.borrow().primitive_getter.is_none());
        }
    }

    #[test]
    fn command_error_default_and_explicit_updates_replace_seeded_code() {
        for (environment, expected) in [
            ("tcl8.4", b"NONE".as_slice()),
            ("tcl8.5", b"NONE".as_slice()),
            ("tcl8.6", b"TCL WRONGARGS".as_slice()),
            ("tcl9.0", b"TCL WRONGARGS".as_slice()),
            ("tcl9.1", b"TCL WRONGARGS".as_slice()),
            ("jim", b"NONE".as_slice()),
        ] {
            leak_free(|interp| {
                interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
                interp.set_error_state(b"PROBE BEFORE");
                interp.mark_error_code_explicit();
                let authentic = tcl_cmd_core::CmdError::wrong_args_bytes(b"n\0\xc0\x80\xff arg");
                let original = authentic.message_bytes().to_vec();
                interp.report_cmd_error(tcl_cmd_core::CmdError::new_bytes(original.as_slice()));
                assert_eq!(interp.result_bytes(), original);
                assert_eq!(interp.error_code(), b"NONE", "{environment}");
                assert!(!interp.exc.borrow().code_explicit);
                interp.report_cmd_error(authentic);
                assert_eq!(interp.result_bytes(), original);
                assert_eq!(interp.error_code(), expected, "{environment}");
                interp
                    .report_cmd_error(tcl_cmd_core::CmdError::with_error_code("bad", "EXACT CODE"));
                assert_eq!(interp.error_code(), b"EXACT CODE");
            });
        }
    }

    #[test]
    fn missing_arity_protocol_refuses_before_guest_state_changes() {
        leak_free(|interp| {
            let unknown = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
                "jim",
                &[],
                "Jim",
                tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
            )));
            interp.set_dialect_profile(unknown);
            // Bootstrap release globals use their owned slots; installing an
            // unsupported profile does not issue an unrelated variable lookup.
            assert_eq!(interp.native_access_refusal(), None);
            assert!(interp
                .native_invocation_dialect()
                .wrong_arguments_protocol(Some(tcl_registry::native_wrong_arguments::LogicalWrongArgumentsProvider::Tcl84CoreSimulation))
                .is_none());
            interp.set_result_bytes(b"RESULT BEFORE");
            interp.set_error_state(b"CODE BEFORE");
            assert_eq!(
                interp.report_cmd_error(tcl_cmd_core::CmdError::wrong_args("native arg")),
                Code::Error
            );
            assert_eq!(interp.result_bytes(), b"RESULT BEFORE");
            assert_eq!(interp.error_code(), b"CODE BEFORE");
            assert_eq!(
                interp.native_access_refusal(),
                Some(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "wrong arguments",
                    ),
                )
            );
        });
    }

    #[test]
    fn jim_diagnostic_argv_borrows_preserve_sharing_and_lazy_string_representation() {
        fn probe(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
            // SAFETY: invocation owns the argument through this callback.
            assert_eq!(unsafe { (*argv[1]).ref_count }, 1);
            assert!(unsafe { (*argv[1]).bytes.is_null() });
            // SAFETY: argv is live; set_obj_result takes its own result hold.
            unsafe { interp.set_obj_result(argv[1]) };
            Code::Ok
        }
        fn fail(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
            assert_eq!(unsafe { (*argv[1]).ref_count }, 1);
            assert!(unsafe { (*argv[1]).bytes.is_null() });
            interp.error(b"BOOM")
        }
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        interp
            .jim_evaluation_frames
            .borrow_mut()
            .push(JimEvaluationFrame {
                procedure_level: 0,
                command_name: None,
                is_procedure: false,
                script: None,
                invocation: Vec::new(),
            });
        for (callback, expected) in [
            (probe as BuiltinFn, Code::Ok),
            (fail as BuiltinFn, Code::Error),
        ] {
            let argv = [new_string(b"probe"), obj::new_wide_int_obj(42)];
            for word in argv {
                unsafe { obj::incr_ref_count(word) };
            }
            assert_eq!(
                interp.invoke_bound(Command::Builtin(callback), None, &argv),
                expected
            );
            assert!(interp.jim_invocation_borrows.borrow().is_empty());
            if expected == Code::Ok {
                assert!(unsafe { (*argv[1]).bytes.is_null() });
            } else {
                assert_eq!(interp.jim_stacktrace(), b"{} {} 1 {probe 42}");
            }
            release_all(&argv);
            interp.set_result_bytes(b"");
        }
        interp.jim_evaluation_frames.borrow_mut().pop();
    }

    struct SyntheticHost {
        clock: SyntheticClock,
        stdio: SyntheticStdIo,
        env: SyntheticEnv,
    }

    impl SyntheticHost {
        fn new(entries: &[(&str, &str)]) -> Self {
            Self {
                clock: SyntheticClock,
                stdio: SyntheticStdIo,
                env: SyntheticEnv(RefCell::new(
                    entries
                        .iter()
                        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                        .collect(),
                )),
            }
        }
    }

    struct SyntheticClock;

    impl tcl_platform::Clock for SyntheticClock {
        fn now_secs(&self) -> i64 {
            0
        }

        fn now_millis(&self) -> i128 {
            0
        }
    }

    struct SyntheticStdIo;

    impl tcl_platform::StdIo for SyntheticStdIo {
        fn write_stdout(&self, _bytes: &[u8]) {}

        fn write_stderr(&self, _bytes: &[u8]) {}
    }

    struct SyntheticEnv(RefCell<std::collections::BTreeMap<String, String>>);

    impl tcl_platform::Env for SyntheticEnv {
        fn get(&self, key: &str) -> Option<String> {
            self.0.borrow().get(key).cloned()
        }

        fn set(&self, key: &str, value: &str) {
            self.0
                .borrow_mut()
                .insert(key.to_string(), value.to_string());
        }

        fn vars(&self) -> Vec<(String, String)> {
            self.0
                .borrow()
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        }

        fn cwd(&self) -> Result<String, tcl_platform::HostError> {
            Ok("/synthetic".to_string())
        }

        fn chdir(&self, _path: &str) -> Result<(), tcl_platform::HostError> {
            Ok(())
        }
    }

    impl tcl_platform::Host for SyntheticHost {
        fn capabilities(&self) -> tcl_platform::Capabilities {
            tcl_platform::Capabilities::empty()
        }

        fn clock(&self) -> &dyn tcl_platform::Clock {
            &self.clock
        }

        fn stdio(&self) -> &dyn tcl_platform::StdIo {
            &self.stdio
        }

        fn env(&self) -> &dyn tcl_platform::Env {
            &self.env
        }
    }

    const GUARDED_IDENTITY: GuardIdentity = GuardIdentity::new(1, 71);

    fn guarded_builtin(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        interp.set_result_bytes(b"");
        Code::Ok
    }

    fn resultless_builtin(_interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        Code::Ok
    }

    fn prepare_interpreter_guard(interp: &mut Interp) -> GuardToken {
        interp.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
        interp
            .prepare_command_guard(
                b"guarded",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::Interpreter),
            )
            .expect("interpreter domain should be guardable")
    }

    fn assert_interpreter_guard_stale(interp: &mut Interp, token: GuardToken) {
        // The command stays attested through every mutation below, so a failed
        // check proves the Interpreter epoch changed.
        assert!(interp.attested_identities(b"guarded").is_some());
        assert!(!interp.check_command_guard(token, b"guarded"));
    }

    #[test]
    fn native_jim_core_keeps_only_constructor_empty_roles() {
        let interp = Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let context = interp.native_jim_object_context().unwrap();
        let empty = context.empty_object();
        assert_eq!(interp.result_obj(), empty.as_ptr());
        // These are the actual empty, result, errorProc and top-frame namespace
        // roles in Jim_CreateInterp, before a distribution extension is loaded.
        assert_eq!(unsafe { (*empty.as_ptr()).ref_count }, 4);
        assert!(interp.find_command_id(GLOBAL, b"binary").is_none());
        assert!(interp.find_command_id(GLOBAL, b"set").is_some());
    }

    #[test]
    fn native_core_root_cells_match_six_original_constructor_inventories() {
        use tcl_registry::special_vars::NativeBootstrapInputs;
        let fixture =
            include_str!("../../../rust/tcl-registry/tests/data/native_bootstrap/core-roots.tsv");
        for (profile_name, version) in [
            ("tcl8.4", "8.4"),
            ("tcl8.5", "8.5"),
            ("tcl8.6", "8.6"),
            ("tcl9.0", "9.0"),
            ("tcl9.1", "9.1"),
            ("jim", "jim"),
        ] {
            counters::reset();
            {
                let interp = Interp::with_native_core(
                    default_host(),
                    crate::environment::profile_for_dialect(profile_name),
                    NativeBootstrapInputs {
                        package_path: Vec::new(),
                        default_library: Some(b"/native/build/library".to_vec()),
                    },
                )
                .unwrap();
                assert_eq!(
                    interp.find_command_id(GLOBAL, b"binary").is_some(),
                    version != "jim",
                    "{profile_name}"
                );
                assert_eq!(
                    interp.find_command_id(GLOBAL, b"oo::class").is_some(),
                    matches!(version, "8.6" | "9.0" | "9.1"),
                    "{profile_name}"
                );
                assert_eq!(
                    interp.find_command_id(GLOBAL, b"try").is_some(),
                    matches!(version, "8.6" | "9.0" | "9.1" | "jim"),
                    "{profile_name}"
                );
                assert_eq!(
                    interp.find_command_id(GLOBAL, b"throw").is_some(),
                    matches!(version, "8.6" | "9.0" | "9.1"),
                    "{profile_name}"
                );
                assert!(
                    interp.find_command_id(GLOBAL, b"auto_load").is_none(),
                    "{profile_name}"
                );
                assert!(
                    interp.find_command_id(GLOBAL, b"auto_import").is_none(),
                    "{profile_name}"
                );
                let expected: Vec<_> = fixture
                    .lines()
                    .filter_map(|line| {
                        let fields: Vec<_> = line.split('\t').collect();
                        (fields[0] == version && fields[2] == "1")
                            .then_some(fields[1].as_bytes().to_vec())
                    })
                    .collect();
                assert!(!expected.is_empty());
                let namespaces = interp.namespaces.borrow();
                let table = namespaces.var_table(GLOBAL);
                assert_eq!(
                    table
                        .names()
                        .into_iter()
                        .map(<[u8]>::to_vec)
                        .collect::<Vec<_>>(),
                    expected,
                    "{profile_name}"
                );
                for line in fixture
                    .lines()
                    .filter(|line| line.starts_with(&format!("{version}\t")))
                {
                    let fields: Vec<_> = line.split('\t').collect();
                    assert!(
                        table.has_native_namespace_cell(fields[1].as_bytes()),
                        "{profile_name}: {}",
                        fields[1]
                    );
                }
                for name in [b"argv".as_slice(), b"argc", b"argv0", b"tcl_library"] {
                    assert!(
                        !table.has_native_namespace_cell(name),
                        "{profile_name}: {name:?}"
                    );
                }
            }
            assert_eq!(
                counters::finalize(),
                0,
                "{profile_name}: {} objects, {} buffers",
                counters::live_objs(),
                counters::live_bufs()
            );
        }
    }

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
        }
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
    }

    /// Evaluate `src`, requiring success, and return the result bytes.
    fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?} -> {:?}",
            String::from_utf8_lossy(src),
            String::from_utf8_lossy(&i.result_bytes())
        );
        i.result_bytes()
    }

    #[test]
    fn fresh_interp_installs_the_shared_platform_schema_before_init() {
        leak_free(|i| {
            let mut expected = tcl_platform::bootstrap::entries()
                .iter()
                .map(|entry| entry.name())
                .collect::<Vec<_>>();
            expected.sort_unstable();
            assert_eq!(
                ok(i, b"lsort [array names ::tcl_platform]"),
                expected.join(" ").as_bytes()
            );
            assert_eq!(ok(i, b"set ::tcl_platform(osVersion)"), b"");
            assert!(!ok(i, b"set ::tcl_platform(machine)").is_empty());
            assert_eq!(
                ok(
                    i,
                    b"list [info exists ::env] [info exists ::argv] \
                      [info exists ::argv0] [info exists ::argc] \
                      [info exists ::auto_path] [info exists ::tcl_library]"
                ),
                b"1 1 1 1 1 1"
            );
        });
    }

    #[test]
    fn selected_host_bootstrap_and_rebind_replace_all_host_globals() {
        counters::reset();
        {
            let first = Rc::new(SyntheticHost::new(&[
                ("USER", "first-user"),
                ("TCL_LIBRARY", "/first/lib"),
                ("TCL_WASM_SPEC", "first-wasm"),
                ("FIRST_ONLY", "stale"),
            ]));
            let mut interp = Interp::with_host(first);
            assert_eq!(
                interp.host().capabilities(),
                tcl_platform::Capabilities::empty()
            );
            assert_eq!(
                ok(
                    &mut interp,
                    b"list $::tcl_platform(user) $::tcl_platform(wasm) \
                      $::tcl_library $::env(FIRST_ONLY)"
                ),
                b"first-user first-wasm /first/lib stale"
            );
            ok(
                &mut interp,
                b"set ::env(EMBEDDER_STALE) old; \
                  set ::tcl_platform(user) old; \
                  set ::auto_path /old/auto; \
                  set ::tclDefaultLibrary /old/default; \
                  set ::tcl_pkgPath /old/pkg",
            );

            interp.set_host(Rc::new(SyntheticHost::new(&[
                ("USER", "second-user"),
                ("TCL_LIBRARY", "/second/lib"),
                ("TCL_WASM_SPEC", "second-wasm"),
                ("SECOND_ONLY", "fresh"),
            ])));
            assert_eq!(
                ok(
                    &mut interp,
                    b"list $::tcl_platform(user) $::tcl_platform(wasm) \
                      $::tcl_library $::env(SECOND_ONLY) \
                      [info exists ::env(FIRST_ONLY)] \
                      [info exists ::env(EMBEDDER_STALE)] \
                      [info exists ::tclDefaultLibrary] \
                      [info exists ::tcl_pkgPath] [llength $::auto_path]"
                ),
                b"second-user second-wasm /second/lib fresh 0 0 0 0 0"
            );
            assert_eq!(
                ok(
                    &mut interp,
                    b"interp create child; child eval {list $::tcl_platform(user) \
                      $::tcl_library $::env(SECOND_ONLY)}"
                ),
                b"second-user /second/lib fresh"
            );
        }
        assert_eq!(counters::finalize(), 0);
        assert_eq!(counters::double_free_count(), 0);
    }

    #[test]
    fn child_and_safe_platform_schemas_come_from_the_shared_owner() {
        leak_free(|i| {
            let mut child_keys = tcl_platform::bootstrap::entries()
                .iter()
                .map(|entry| entry.name())
                .collect::<Vec<_>>();
            child_keys.sort_unstable();
            assert_eq!(
                ok(
                    i,
                    b"interp create child; child eval {lsort [array names ::tcl_platform]}"
                ),
                child_keys.join(" ").as_bytes()
            );

            let scrubbed = tcl_platform::bootstrap::safe_scrub_keys().collect::<Vec<_>>();
            let mut safe_keys = tcl_platform::bootstrap::entries()
                .iter()
                .map(|entry| entry.name())
                .filter(|name| !scrubbed.contains(name))
                .collect::<Vec<_>>();
            safe_keys.sort_unstable();
            assert_eq!(
                ok(
                    i,
                    b"interp create -safe safe; safe eval {lsort [array names ::tcl_platform]}"
                ),
                safe_keys.join(" ").as_bytes()
            );
            assert!(safe_keys.contains(&"threaded"));
            assert_eq!(
                ok(
                    i,
                    b"safe eval {list [info exists ::env] \
                      [info exists ::tcl_library] [info exists ::auto_path] \
                      [info exists ::tclDefaultLibrary] [info exists ::tcl_pkgPath]}"
                ),
                b"0 0 0 0 0"
            );
        });
    }

    #[test]
    fn command_guard_checks_identity_and_exact_lifecycle() {
        leak_free(|i| {
            i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
            let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
            assert_eq!(
                i.prepare_command_guard(b"guarded", GuardIdentity::new(1, 72), domains),
                Err(GuardError::IdentityMismatch)
            );
            let token = i
                .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
                .unwrap();
            assert!(i.check_command_guard(token, b"guarded"));
            assert!(i.release_command_guard(token));
            assert!(!i.release_command_guard(token));
            assert!(!i.check_command_guard(token, b"guarded"));
        });
    }

    /// A request for a Family-B intrinsic covers the variable-trace domain or
    /// is refused, whatever the caller asked for, and a Value member needs no
    /// such domain. Covered, the guard is refused while a variable trace
    /// exists and goes stale when one is added.
    #[test]
    fn a_family_b_guard_request_must_cover_the_variable_trace_domain() {
        use tcl_registry::IntrinsicId;
        leak_free(|i| {
            let version = i.runtime_version();
            let identity = |member: IntrinsicId| {
                GuardIdentity::registry_intrinsic_with_semantics(
                    member.stable_id(),
                    member.guard_semantics_key(version),
                )
            };
            let (stores, value) = (
                identity(IntrinsicId::DictSet),
                identity(IntrinsicId::ListLength),
            );
            i.register_guarded_builtin(b"stores", guarded_builtin, stores);
            i.register_guarded_builtin(b"pure", guarded_builtin, value);
            let command = GuardDomains::one(GuardDomain::CommandEnvironment);
            let traced = command.with(GuardDomain::VariableTrace);

            assert_eq!(
                i.prepare_command_guard(b"stores", stores, command),
                Err(GuardError::DomainsInsufficient)
            );
            let token = i
                .prepare_command_guard(b"pure", value, command)
                .expect("a Value member requires no variable-trace domain");
            assert!(i.release_command_guard(token));

            let token = i
                .prepare_command_guard(b"stores", stores, traced)
                .expect("a request covering the family's domain");
            assert!(i.check_command_guard_identity(token, b"stores", stores));
            assert_eq!(
                i.eval_str(b"trace add variable watched write callback"),
                Code::Ok
            );
            assert!(!i.check_command_guard_identity(token, b"stores", stores));
            assert_eq!(
                i.prepare_command_guard(b"stores", stores, traced),
                Err(GuardError::PrerequisiteUnsatisfied)
            );
            assert!(i.prepare_command_guard(b"pure", value, command).is_ok());
        });
    }

    /// A guard is bound to its command's token. A definition, rename or alias
    /// of another command leaves it and its attestation alone; replacing,
    /// renaming away or hiding the command drops it, and restoring the same
    /// token brings it back; a profile pin keeps the attestation.
    #[test]
    fn an_unrelated_mutation_keeps_the_guard_and_a_rebinding_drops_it() {
        leak_free(|i| {
            i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
            let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
            let token = i
                .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
                .unwrap();

            i.register_builtin(b"unrelated", guarded_builtin);
            ok(i, b"proc foo {} {return 1}");
            ok(i, b"rename foo bar");
            ok(i, b"interp alias {} baz {} bar");
            ok(i, b"rename baz {}");
            assert!(i.check_command_guard(token, b"guarded"));
            assert!(
                i.prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
                    .is_ok()
            );

            // A change to the lookup environment itself stales the token over
            // it, and the attestation stays.
            ok(i, b"namespace eval other {namespace path ::}");
            assert!(!i.check_command_guard(token, b"guarded"));
            let token = i
                .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
                .expect("still attested");

            // The profile pin keeps the attestation and a token that does not
            // depend on interpreter policy.
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            assert!(i.check_command_guard(token, b"guarded"));
            assert!(
                i.prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
                    .is_ok()
            );

            // Renaming the command away drops the guard at its name, and the
            // attestation goes with the command; restoring the name restores it.
            ok(i, b"rename guarded moved");
            assert!(!i.check_command_guard(token, b"guarded"));
            assert_eq!(
                i.prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains),
                Err(GuardError::IdentityUnavailable)
            );
            assert!(
                i.prepare_command_guard(b"moved", GUARDED_IDENTITY, domains)
                    .is_ok()
            );
            ok(i, b"rename moved guarded");
            assert!(i.check_command_guard(token, b"guarded"));

            // Hiding it drops the guard, exposing it restores it.
            assert_eq!(
                i.hide_command(b"guarded", b"guarded"),
                CommandVisibilityOutcome::Moved
            );
            assert!(!i.check_command_guard(token, b"guarded"));
            assert_eq!(
                i.expose_command(b"guarded", b"guarded"),
                CommandVisibilityOutcome::Moved
            );
            assert!(i.check_command_guard(token, b"guarded"));

            // A different command at the name is never attested.
            ok(i, b"proc guarded {} {return 1}");
            assert!(!i.check_command_guard(token, b"guarded"));
            assert_eq!(
                i.prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains),
                Err(GuardError::IdentityUnavailable)
            );
        });
    }

    /// The attestation belongs to the token the name reaches, so a definition
    /// in a namespace that shadows the command drops the guard for calls made
    /// from that namespace and only from it.
    #[test]
    fn a_shadowing_definition_drops_the_guard_only_for_calls_from_its_namespace() {
        leak_free(|i| {
            i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
            let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
            let token = i
                .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
                .unwrap();

            ok(i, b"namespace eval ns {proc guarded {} {return shadow}}");
            let ns = i
                .namespaces
                .borrow()
                .find_namespace(GLOBAL, b"ns")
                .expect("the namespace exists");
            assert!(i.check_command_guard(token, b"guarded"));
            i.current_ns.set(ns);
            assert!(!i.check_command_guard(token, b"guarded"));
            assert_eq!(
                i.prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains),
                Err(GuardError::IdentityUnavailable)
            );
            i.current_ns.set(GLOBAL);
            assert!(i.check_command_guard(token, b"guarded"));
        });
    }

    /// The three lookup domains move together on a hidden invocation into a
    /// namespace, whether the namespace is named, is the global one, or does not
    /// exist yet, and on `oo::copy`: events the guard domain's docs list beside
    /// `namespace path` and the rest, which a command-table mutation is not.
    #[test]
    fn the_lookup_domains_move_on_a_hidden_invocation_into_a_namespace_and_on_oo_copy() {
        leak_free(|i| {
            i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
            let domains = GuardDomains::one(GuardDomain::CommandEnvironment)
                .with(GuardDomain::Namespace)
                .with(GuardDomain::UnknownHandling);
            let moves = |i: &mut Interp, script: &[u8]| {
                let token = i
                    .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
                    .expect("the lookup domains are guardable");
                ok(i, script);
                assert!(i.attested_identities(b"guarded").is_some());
                !i.check_command_guard(token, b"guarded")
            };

            ok(i, b"interp hide {} lindex");
            assert!(
                moves(i, b"interp invokehidden {} -namespace fresh lindex {a b} 0"),
                "a namespace it creates"
            );
            assert!(
                moves(i, b"interp invokehidden {} -namespace fresh lindex {a b} 0"),
                "a namespace that exists"
            );
            assert!(
                moves(i, b"interp invokehidden {} -global lindex {a b} 0"),
                "the global namespace"
            );
            assert!(
                !moves(i, b"interp invokehidden {} lindex {a b} 0"),
                "a hidden invocation that names no namespace moves nothing"
            );

            ok(
                i,
                b"oo::class create C {variable v; constructor {} {set v 1}}",
            );
            ok(i, b"set o [C new]");
            assert!(moves(i, b"oo::copy $o"), "oo::copy");
        });
    }

    /// The sweep reads the generation the interpreter is pinned to and nothing
    /// an overlay installed: a pack's command is no runtime implementation, and
    /// only the runtime may attest.
    #[test]
    fn identities_come_from_the_pinned_generation_never_an_overlay() {
        use tcl_registry::{CommandSpec, IntrinsicId, SemanticOperationId};
        leak_free(|i| {
            // A generation an overlay installed under this interpreter's own
            // profile, holding a spec that declares an intrinsic, and a builtin
            // of that name in the interpreter.
            let overlaid = tcl_registry::registry_for_profile_with_overlay(
                i.dialect_profile(),
                0xC0DE,
                |registry| {
                    registry.insert(CommandSpec {
                        name: "overlay_length",
                        semantic_operation: Some(SemanticOperationId::Intrinsic(
                            IntrinsicId::StringLength,
                        )),
                        ..CommandSpec::DEFAULT
                    });
                },
            );
            i.register_builtin(b"overlay_length", guarded_builtin);

            i.attach_identities();
            assert!(
                i.attested_identities(b"overlay_length").is_none(),
                "the sweep of the pinned generation attests nothing an overlay declares"
            );
            assert!(
                i.attested_identities(b"string").is_some(),
                "and still attests the shipped command"
            );

            // Control: the fixture is sound, a sweep over the overlay's own
            // store would attest the builtin.
            i.attach_identities_from(&overlaid);
            assert!(i.attested_identities(b"overlay_length").is_some());
        });
    }

    /// A name is attested only while the command bound at it is the builtin
    /// the runtime registered there: a procedure defined over it, or another
    /// builtin renamed into it, gains nothing, and the original, put back,
    /// has what it had.
    #[test]
    fn the_sweep_attests_only_the_builtin_it_registered() {
        leak_free(|i| {
            use tcl_registry::IntrinsicId;
            let length = GuardIdentity::registry_intrinsic_with_semantics(
                IntrinsicId::StringLength.stable_id(),
                IntrinsicId::StringLength.guard_semantics_key(i.runtime_version()),
            );
            let attested = |i: &Interp| {
                i.attested_identities(b"string")
                    .is_some_and(|identities| identities.contains(&length))
            };
            assert!(attested(i));

            ok(i, b"rename string original");
            assert!(!attested(i), "moved away, the name reaches nothing");
            ok(i, b"proc string args {return x}");
            i.attach_identities();
            assert!(!attested(i), "a procedure at the name gains no attestation");

            ok(i, b"rename string {}");
            ok(i, b"rename puts string");
            i.attach_identities();
            assert!(
                !attested(i),
                "another builtin renamed into the name gains none"
            );

            ok(i, b"rename string puts");
            ok(i, b"rename original string");
            assert!(attested(i), "the original, restored, is attested as it was");
        });
    }

    /// Every builtin this runtime registers at the name of a spec that
    /// declares an intrinsic is attested for every identity the spec's
    /// intrinsics have, and no other command is attested at all.
    #[test]
    fn every_registered_builtin_with_an_intrinsic_is_attested_for_all_of_them() {
        leak_free(|i| {
            let registry = crate::environment::store_for_profile(i.dialect_profile());
            let report = tcl_runtime_api::BackingReport::from_entries(i.backing_report());
            let mut attested_names = 0;
            for name in registry.command_names() {
                let spec = registry.get_exact(name).expect("a named spec");
                let expected: std::collections::BTreeSet<GuardIdentity> = spec
                    .intrinsic_ids()
                    .into_iter()
                    .flat_map(|id| {
                        id.guard_semantics_variants().iter().map(move |semantics| {
                            GuardIdentity::registry_intrinsic_with_semantics(
                                id.stable_id(),
                                *semantics,
                            )
                        })
                    })
                    .collect();
                let actual = i.attested_identities(name.as_bytes());
                if expected.is_empty() || report.of(name) != RegisteredBacking::Builtin {
                    assert!(actual.is_none(), "{name} has nothing to attest");
                } else {
                    assert_eq!(actual.as_ref(), Some(&expected), "{name}");
                    attested_names += 1;
                }
            }
            assert!(attested_names >= 10, "{attested_names}");
        });
    }

    /// The backing report says what the handler table holds: handlers, the
    /// engine's `TclOO` roots and the commands the object system binds in every
    /// object, handlers that only refuse, and absences; and nothing a script
    /// defines.
    #[test]
    fn the_backing_report_says_what_the_handler_table_holds() {
        leak_free(|i| {
            let report =
                |i: &Interp| tcl_runtime_api::BackingReport::from_entries(i.backing_report());
            let before = report(i);
            assert_eq!(before.of("set"), RegisteredBacking::Builtin);
            assert_eq!(
                before.of("::tcl::string::insert"),
                RegisteredBacking::Builtin
            );
            assert_eq!(before.of("exec"), RegisteredBacking::Unsupported);
            assert_eq!(before.of("oo::class"), RegisteredBacking::Object);
            assert_eq!(before.of("oo::object"), RegisteredBacking::Object);
            assert_eq!(before.of("my"), RegisteredBacking::Object);
            assert_eq!(before.of("zipfs"), RegisteredBacking::Absent);
            assert_eq!(before.of("tcl::dict::get"), RegisteredBacking::Absent);
            #[cfg(have_tommath)]
            assert_eq!(before.of("expr"), RegisteredBacking::Builtin);
            #[cfg(not(have_tommath))]
            assert_eq!(before.of("expr"), RegisteredBacking::NeedsNumericTower);

            ok(i, b"proc mine {} {}");
            ok(i, b"oo::class create Mine");
            ok(i, b"interp alias {} aliased {} set");
            let after = report(i);
            for name in ["mine", "Mine", "aliased"] {
                assert_eq!(after.of(name), RegisteredBacking::Absent, "{name}");
            }
            assert_eq!(after, before, "a script adds nothing to the report");
        });
    }

    /// The commands a build without the numeric tower reports as needing it
    /// are the ones a build with it registers as handlers.
    #[cfg(have_tommath)]
    #[test]
    fn the_tower_commands_a_build_without_it_names_are_registered_with_it() {
        leak_free(|i| {
            let report = tcl_runtime_api::BackingReport::from_entries(i.backing_report());
            let names = crate::builtins::tower_command_names();
            assert!(names.len() > 60, "{}", names.len());
            for name in names {
                assert_eq!(report.of(&name), RegisteredBacking::Builtin, "{name}");
            }
        });
    }

    /// A build without the tower reports each of those commands as needing it,
    /// and registers none of them.
    #[cfg(not(have_tommath))]
    #[test]
    fn a_build_without_the_tower_reports_the_commands_it_lacks_as_needing_it() {
        leak_free(|i| {
            let report = tcl_runtime_api::BackingReport::from_entries(i.backing_report());
            for name in crate::builtins::tower_command_names() {
                assert_eq!(
                    report.of(&name),
                    RegisteredBacking::NeedsNumericTower,
                    "{name}"
                );
            }
            assert_eq!(ok(i, b"info commands expr"), b"");
        });
    }

    #[test]
    fn trace_registration_invalidates_matching_guard_domains() {
        leak_free(|i| {
            i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
            let command_token = i
                .prepare_command_guard(
                    b"guarded",
                    GUARDED_IDENTITY,
                    GuardDomains::one(GuardDomain::CommandTrace),
                )
                .unwrap();
            assert_eq!(
                i.eval_str(b"trace add command guarded rename callback"),
                Code::Ok
            );
            assert!(!i.check_command_guard(command_token, b"guarded"));

            let variable_token = i
                .prepare_command_guard(
                    b"guarded",
                    GUARDED_IDENTITY,
                    GuardDomains::one(GuardDomain::VariableTrace),
                )
                .unwrap();
            assert_eq!(
                i.eval_str(b"trace add variable watched write callback"),
                Code::Ok
            );
            assert!(!i.check_command_guard(variable_token, b"guarded"));
        });
    }

    #[test]
    fn object_dispatch_remains_poisoned_but_interpreter_is_guardable() {
        leak_free(|i| {
            i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
            assert_eq!(
                i.prepare_command_guard(
                    b"guarded",
                    GUARDED_IDENTITY,
                    GuardDomains::one(GuardDomain::ObjectDispatch)
                ),
                Err(GuardError::Poisoned)
            );
            let token = prepare_interpreter_guard(i);
            assert!(i.check_command_guard(token, b"guarded"));
        });
    }

    #[test]
    fn command_identity_survives_unrelated_mutation_and_follows_its_actual_generation() {
        leak_free(|i| {
            let token = prepare_interpreter_guard(i);
            i.register_builtin(b"unrelated", resultless_builtin);
            assert!(i.check_command_guard(token, b"guarded"));
            let fresh = i
                .prepare_command_guard(
                    b"guarded",
                    GUARDED_IDENTITY,
                    GuardDomains::one(GuardDomain::CommandEnvironment),
                )
                .unwrap();
            assert_eq!(
                i.rename_command(b"guarded", b"moved"),
                RenameOutcome::Renamed
            );
            assert!(!i.check_command_guard(fresh, b"moved"));
            assert!(i.check_command_guard(token, b"moved"));
            let moved = i
                .prepare_command_guard(
                    b"moved",
                    GUARDED_IDENTITY,
                    GuardDomains::one(GuardDomain::Interpreter),
                )
                .unwrap();
            i.register_guarded_builtin(b"moved", guarded_builtin, GUARDED_IDENTITY);
            assert!(!i.check_command_guard(token, b"moved"));
            assert!(!i.check_command_guard(moved, b"moved"));
            let replacement = i
                .prepare_command_guard(
                    b"moved",
                    GUARDED_IDENTITY,
                    GuardDomains::one(GuardDomain::Interpreter),
                )
                .unwrap();
            assert!(i.check_command_guard(replacement, b"moved"));
            assert!(i.release_command_guard(token));
            assert!(i.release_command_guard(fresh));
            assert!(i.release_command_guard(moved));
            assert!(i.release_command_guard(replacement));
        });
    }

    #[test]
    fn string_length_guard_accepts_irreducible_base_domains() {
        leak_free(|i| {
            let identity = GuardIdentity::registry_intrinsic_with_semantics(
                tcl_registry::IntrinsicId::StringLength.stable_id(),
                tcl_registry::IntrinsicId::StringLength.guard_semantics_key(i.runtime_version()),
            );
            let domains = GuardDomains::one(GuardDomain::CommandEnvironment)
                .with(GuardDomain::Namespace)
                .with(GuardDomain::CommandTrace)
                .with(GuardDomain::Interpreter);
            let token = i
                .prepare_command_guard(b"string", identity, domains)
                .expect("the registry BASE domains should be live");
            assert!(i.check_command_guard_identity(token, b"string", identity));
            assert!(i.release_command_guard(token));
        });
    }

    #[test]
    fn visibility_safety_and_topology_mutations_stale_interpreter_guards() {
        leak_free(|i| {
            let token = prepare_interpreter_guard(i);
            assert_eq!(
                i.hide_command(b"set", b"set"),
                CommandVisibilityOutcome::Moved
            );
            assert_interpreter_guard_stale(i, token);

            let token = prepare_interpreter_guard(i);
            assert_eq!(
                i.expose_command(b"set", b"set"),
                CommandVisibilityOutcome::Moved
            );
            assert_interpreter_guard_stale(i, token);

            let token = prepare_interpreter_guard(i);
            assert_eq!(i.create_child(Some(b"child".to_vec())), b"child");
            assert_interpreter_guard_stale(i, token);

            let token = prepare_interpreter_guard(i);
            assert!(i.delete_child(b"child"));
            assert_interpreter_guard_stale(i, token);

            let token = prepare_interpreter_guard(i);
            i.make_safe();
            assert_interpreter_guard_stale(i, token);

            let token = prepare_interpreter_guard(i);
            i.mark_trusted();
            assert_interpreter_guard_stale(i, token);
        });
    }

    /// A child is another interpreter of the same build, so it states the
    /// world its parent is pinned to, the overlay included.
    #[test]
    fn a_child_states_the_context_its_parent_is_pinned_to() {
        const OVERLAY: u64 = 0x0C0_1705;
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").expect("catalogue profile");
        tcl_registry::registry_for_profile_with_overlay(profile, OVERLAY, |_| {});
        leak_free(|parent| {
            let mut context = tcl_registry::model::runtime_context_for_profile(profile);
            context.overlay_generation = OVERLAY;
            context.packages = vec![("vendor".to_owned(), "2.1".to_owned())];
            parent.pin_context(&context).expect("installed, so it pins");
            parent.create_child(Some(b"child".to_vec()));
            let child = parent
                .children
                .borrow()
                .get(b"child".as_slice())
                .expect("child")
                .clone();
            assert_eq!(child.runtime_context(), context);
            assert_eq!(child.held_identity(), parent.held_identity());
        });
    }

    #[test]
    fn child_parent_association_stales_interpreter_guard() {
        leak_free(|parent| {
            parent.create_child(Some(b"child".to_vec()));
            let mut child = parent
                .children
                .borrow()
                .get(b"child".as_slice())
                .expect("child")
                .clone();
            let token = prepare_interpreter_guard(&mut child);
            assert!(child.check_command_guard(token, b"guarded"));

            parent.with_child(b"child", |_| ());
            assert_interpreter_guard_stale(&mut child, token);
        });
    }

    #[test]
    fn execution_policy_and_runtime_version_mutations_stale_interpreter_guards() {
        leak_free(|i| {
            let token = prepare_interpreter_guard(i);
            i.set_host(i.host());
            assert_interpreter_guard_stale(i, token);

            let token = prepare_interpreter_guard(i);
            i.set_bgerror_handler(b"handler");
            assert_interpreter_guard_stale(i, token);

            let token = prepare_interpreter_guard(i);
            let frame = new_string(b"-frame");
            let enabled = new_string(b"1");
            let result = i.debug_apply(&[frame, enabled]).expect("debug policy");
            drop_fresh(frame);
            drop_fresh(enabled);
            drop_fresh(result);
            assert_interpreter_guard_stale(i, token);

            let token = prepare_interpreter_guard(i);
            assert_eq!(i.recursion_limit_apply(Some(b"250")), Ok(250));
            assert_interpreter_guard_stale(i, token);

            let token = prepare_interpreter_guard(i);
            let option = new_string(b"-value");
            let value = new_string(b"100");
            let result = i
                .limit_apply(b"commands", &[option, value])
                .expect("command limit policy");
            drop_fresh(option);
            drop_fresh(value);
            drop_fresh(result);
            assert_interpreter_guard_stale(i, token);

            let token = prepare_interpreter_guard(i);
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            assert_interpreter_guard_stale(i, token);
        });
    }

    #[test]
    fn expression_policy_keeps_native_dispatch_separate_from_authored_parser() {
        use tcl_registry::invocation_words::LogicalExpressionParseProvider;
        use tcl_runtime_api::expression_policy::ExpressionEvaluationOrigin;
        leak_free(|interp| {
            interp.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            let native = interp.expression_evaluation_policy().unwrap();
            assert!(matches!(
                native.origin,
                ExpressionEvaluationOrigin::Native(_)
            ));
            assert_eq!(
                tcl_registry::native_expression_program::expression_function_dispatch(
                    Some(&native),
                    interp.native_invocation_dialect()
                ),
                Some(tcl_registry::mathfunc::NativeMathFunctionDispatch::CommandTable)
            );
            interp.set_dialect_profile(tcl_dialect::DialectProfile::irules());
            assert!(interp.expression_evaluation_policy().is_none());
            let host = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
            assert!(interp.set_logical_expression_parse_provider(
                LogicalExpressionParseProvider::Tcl84CoreSimulation,
                host
            ));
            let authored = interp.expression_evaluation_policy().unwrap();
            assert_eq!(
                authored.origin,
                ExpressionEvaluationOrigin::AuthoredTcl84Parser
            );
            assert!(authored.numeric_simulation.is_none());
            assert_eq!(
                tcl_registry::native_expression_program::expression_function_dispatch(
                    Some(&authored),
                    tcl_registry::InvocationDialect::of_profile(host)
                ),
                None
            );
        });
    }

    // These helpers exercise epoch policy against an original live registration.
    // They do not restore a named lookup, native instruction or hosted engine.
    fn retained_fixture_identity(interp: &Interp, generation: u64) -> Option<GuardIdentity> {
        interp.raw_command_location_by_generation(generation)?;
        interp
            .guarded_commands
            .borrow()
            .get(&generation)?
            .contains(&GUARDED_IDENTITY)
            .then_some(GUARDED_IDENTITY)
    }

    fn prepare_retained_fixture_guard(
        interp: &Interp,
        generation: u64,
        domain: GuardDomain,
    ) -> GuardToken {
        let observed = retained_fixture_identity(interp, generation);
        interp
            .guards
            .borrow_mut()
            .prepare(
                GUARDED_IDENTITY,
                observed,
                GuardDomains::one(domain),
                generation,
            )
            .unwrap()
    }

    fn check_retained_fixture_guard(interp: &Interp, token: GuardToken, generation: u64) -> bool {
        interp.guards.borrow().check(
            token,
            retained_fixture_identity(interp, generation),
            &generation,
        )
    }

    #[test]
    fn logical_provider_changes_stale_only_interpreter_policy_guards() {
        fn install(
            interp: &mut Interp,
            host: &'static tcl_dialect::DialectProfile,
            provider: u8,
        ) -> bool {
            match provider {
                0 => interp.set_logical_eval_object_provider(
                    tcl_registry::native_eval_object::LogicalEvalObjectProvider::Tcl84CoreSimulation,
                    host,
                ),
                1 => interp.set_logical_source_word_provider(
                    tcl_registry::invocation_words::LogicalSourceWordProvider::Tcl84CoreSimulation,
                    host,
                ),
                2 => interp.set_logical_expression_parse_provider(
                    tcl_registry::invocation_words::LogicalExpressionParseProvider::Tcl84CoreSimulation,
                    host,
                ),
                _ => unreachable!(),
            }
        }
        for provider in [0, 1, 2] {
            leak_free(|interp| {
                let source = tcl_dialect::DialectProfile::irules();
                let host = crate::environment::profile_for_dialect("tcl9.0");
                let replacement = crate::environment::profile_for_dialect("tcl9.1");
                // Retain an actual registration before selecting a hosted
                // source profile that has no command naming capability.
                interp.set_dialect_profile(host);
                interp.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
                let generation = interp
                    .namespaces
                    .borrow()
                    .resolve_generation(GLOBAL, b"guarded")
                    .unwrap();
                interp.set_dialect_profile(source);
                assert!(matches!(
                    interp.prepare_command_guard(
                        b"guarded",
                        GUARDED_IDENTITY,
                        GuardDomains::one(GuardDomain::Interpreter)
                    ),
                    Err(GuardError::IdentityUnavailable)
                ));
                let command = prepare_retained_fixture_guard(
                    interp,
                    generation,
                    GuardDomain::CommandEnvironment,
                );
                let policy =
                    prepare_retained_fixture_guard(interp, generation, GuardDomain::Interpreter);
                assert!(interp.native_compiler_cache_epochs(GLOBAL).is_none());
                assert!(!install(interp, source, provider));
                assert!(check_retained_fixture_guard(interp, policy, generation));
                assert!(install(interp, host, provider));
                assert!(!check_retained_fixture_guard(interp, policy, generation));
                assert!(interp.release_command_guard(policy));

                let policy =
                    prepare_retained_fixture_guard(interp, generation, GuardDomain::Interpreter);
                assert!(install(interp, host, provider));
                assert!(check_retained_fixture_guard(interp, policy, generation));
                assert!(!install(interp, source, provider));
                assert!(check_retained_fixture_guard(interp, policy, generation));
                assert!(install(interp, replacement, provider));
                assert!(!check_retained_fixture_guard(interp, policy, generation));
                assert!(check_retained_fixture_guard(interp, command, generation));
                assert!(interp.native_compiler_cache_epochs(GLOBAL).is_none());
                assert!(
                    interp
                        .native_invocation_dialect()
                        .execution_point()
                        .is_none()
                );
                assert!(interp.release_command_guard(policy));
                assert!(interp.release_command_guard(command));
            });
        }
    }

    #[test]
    fn string_length_intrinsic_uses_the_selected_runtime_character_model() {
        leak_free(|i| {
            let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
            for intrinsic in [
                tcl_registry::IntrinsicId::StringLength,
                tcl_registry::IntrinsicId::StringIndex,
            ] {
                let identity = GuardIdentity::registry_intrinsic_with_semantics(
                    intrinsic.stable_id(),
                    intrinsic.guard_semantics_key(i.runtime_version()),
                );
                let token = i
                    .prepare_command_guard(b"string", identity, domains)
                    .unwrap();
                assert!(i.check_command_guard(token, b"string"));
            }

            let value = new_string("é🙂".as_bytes());
            assert_eq!(
                i.execute_intrinsic(tcl_registry::IntrinsicId::StringLength, &[value]),
                Some(Code::Ok)
            );
            assert_eq!(i.result_bytes(), b"2");

            let identity = GuardIdentity::registry_intrinsic_with_semantics(
                tcl_registry::IntrinsicId::StringLength.stable_id(),
                tcl_registry::IntrinsicId::StringLength.guard_semantics_key(i.runtime_version()),
            );
            let live_domains = GuardDomains::one(GuardDomain::CommandEnvironment)
                .with(GuardDomain::Namespace)
                .with(GuardDomain::CommandTrace)
                .with(GuardDomain::Interpreter);
            let token = i
                .prepare_command_guard(b"string", identity, live_domains)
                .expect("registry BASE domains are guardable");
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            assert!(!i.check_command_guard_identity(token, b"string", identity));
            assert!(i.release_command_guard(token));
            assert_eq!(
                i.execute_intrinsic(tcl_registry::IntrinsicId::StringLength, &[value]),
                Some(Code::Error)
            );
            assert_eq!(
                i.native_access_refusal(),
                Some(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "native string count cache origin",
                    ),
                )
            );
            i.reset_native_compilation_admission();
            let c86_value = new_string("é🙂".as_bytes());
            assert_eq!(
                i.execute_intrinsic(tcl_registry::IntrinsicId::StringLength, &[c86_value]),
                Some(Code::Ok)
            );
            assert_eq!(i.result_bytes(), b"3");
            assert_eq!(i.eval_str("string length é🙂".as_bytes()), Code::Ok);
            assert_eq!(i.result_bytes(), b"3");

            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            assert_eq!(i.eval_str("string length é🙂".as_bytes()), Code::Ok);
            assert_eq!(i.result_bytes(), b"2");
            drop_fresh(value);
            drop_fresh(c86_value);
            assert_eq!(
                i.execute_intrinsic(tcl_registry::IntrinsicId::ListLength, &[]),
                None
            );
        });
    }

    #[test]
    fn renaming_spec_registered_string_stales_its_intrinsic_guard() {
        leak_free(|i| {
            let identity = GuardIdentity::registry_intrinsic_with_semantics(
                tcl_registry::IntrinsicId::StringLength.stable_id(),
                tcl_registry::IntrinsicId::StringLength.guard_semantics_key(i.runtime_version()),
            );
            let token = i
                .prepare_command_guard(
                    b"string",
                    identity,
                    GuardDomains::one(GuardDomain::CommandEnvironment),
                )
                .unwrap();
            assert_eq!(i.eval_str(b"rename string moved"), Code::Ok);
            assert!(!i.check_command_guard(token, b"string"));
            assert_eq!(
                i.prepare_command_guard(
                    b"string",
                    identity,
                    GuardDomains::one(GuardDomain::CommandEnvironment)
                ),
                Err(GuardError::IdentityUnavailable)
            );
        });
    }

    /// The real runtime's `string length` guard, over the registry's base
    /// domains, survives the definition, rename and alias of unrelated commands
    /// and falls back once `string` itself is rebound.
    #[test]
    fn an_unrelated_mutation_keeps_the_string_length_guard() {
        leak_free(|i| {
            let identity = GuardIdentity::registry_intrinsic_with_semantics(
                tcl_registry::IntrinsicId::StringLength.stable_id(),
                tcl_registry::IntrinsicId::StringLength.guard_semantics_key(i.runtime_version()),
            );
            let domains = GuardDomains::one(GuardDomain::CommandEnvironment)
                .with(GuardDomain::Namespace)
                .with(GuardDomain::CommandTrace)
                .with(GuardDomain::Interpreter);
            let token = i
                .prepare_command_guard(b"string", identity, domains)
                .expect("the registry BASE domains are guardable");

            ok(i, b"proc foo {x} {return $x}");
            ok(i, b"rename foo bar");
            ok(i, b"interp alias {} baz {} bar");
            assert!(i.check_command_guard_identity(token, b"string", identity));

            ok(i, b"rename string moved");
            assert!(!i.check_command_guard_identity(token, b"string", identity));
            ok(i, b"rename moved string");
            assert!(i.check_command_guard_identity(token, b"string", identity));
            ok(i, b"proc string args {return shadow}");
            assert!(!i.check_command_guard_identity(token, b"string", identity));
        });
    }

    /// The pin keeps every attestation and the pinned surface decides which of
    /// them a guard reaches: a command the release lacks has none, and answers
    /// again under a release that has it.
    #[test]
    fn a_pin_to_a_release_without_the_command_leaves_it_unattested() {
        leak_free(|i| {
            // `lassign` is a command Tcl 8.4 does not have.
            i.register_guarded_builtin(b"lassign", guarded_builtin, GUARDED_IDENTITY);
            let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
            let token = i
                .prepare_command_guard(b"lassign", GUARDED_IDENTITY, domains)
                .expect("attested in the default release");
            assert!(i.check_command_guard(token, b"lassign"));

            i.set_runtime_version(tcl_dialect::TclVersion::V8_4);
            assert!(!i.check_command_guard(token, b"lassign"));
            assert_eq!(
                i.prepare_command_guard(b"lassign", GUARDED_IDENTITY, domains),
                Err(GuardError::IdentityUnavailable)
            );

            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            assert!(
                i.prepare_command_guard(b"lassign", GUARDED_IDENTITY, domains)
                    .is_ok()
            );
        });
    }

    #[test]
    fn command_trace_stales_the_string_intrinsic_guard() {
        leak_free(|i| {
            let identity = GuardIdentity::registry_intrinsic_with_semantics(
                tcl_registry::IntrinsicId::StringLength.stable_id(),
                tcl_registry::IntrinsicId::StringLength.guard_semantics_key(i.runtime_version()),
            );
            let token = i
                .prepare_command_guard(
                    b"string",
                    identity,
                    GuardDomains::one(GuardDomain::CommandTrace),
                )
                .unwrap();
            assert_eq!(
                i.eval_str(b"trace add command string rename callback"),
                Code::Ok
            );
            assert!(!i.check_command_guard(token, b"string"));
        });
    }

    /// The native Runtime returns its explicit host evaluation-limit error
    /// before synchronously nested generic callbacks exhaust the host stack.
    /// This host boundary is separate from the original native completions.
    #[test]
    fn deeply_nested_foreach_reaches_the_explicit_host_evaluation_limit() {
        // Native proof: naming.runtime.original-nested-foreach-source-completion
        // docs/design/analysis/name-resolution-proofs/runtime-original-nested-foreach-source-completion.md
        // The identical 300-body source completes in all six native providers.
        // These assertions exercise this Runtime's separately bounded host path.
        leak_free(|i| {
            let mut src = String::new();
            for _ in 0..300 {
                src.push_str("foreach x {1} {\n");
            }
            src.push_str("set done 1\n");
            for _ in 0..300 {
                src.push_str("}\n");
            }
            assert_eq!(i.eval_str(src.as_bytes()), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"too many nested evaluations (infinite loop?)"
            );
        });
    }

    /// Defence in depth for the alias trampoline. `interp alias`
    /// and `rename` both refuse a cycle at definition time, so plant one
    /// straight into the command table — bypassing that gate the way only a
    /// bug could — and confirm the dispatch bound turns what would otherwise be
    /// unbounded native recursion (`invoke` → `dispatch_alias` → `invoke`, i.e.
    /// stack exhaustion / a WASM trap) into a catchable Tcl error.
    #[test]
    fn a_planted_alias_cycle_errors_instead_of_exhausting_the_stack() {
        leak_free(|i| {
            let alias = |target: &[u8]| Command::Alias {
                target: target.to_vec(),
                prefix: Vec::new(),
                publication_name: target.to_vec(),
                jim_prefix: None,
                identity: Rc::new(()),
            };
            {
                let mut namespaces = i.namespaces.borrow_mut();
                namespaces.register(b"loop_a", alias(b"loop_b"));
                namespaces.register(b"loop_b", alias(b"loop_a"));
            }
            assert_eq!(i.eval_str(b"loop_a"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"too many nested alias invocations (infinite loop?)"
            );
            // The counter unwinds with the failed call, so a later alias call
            // is unaffected.
            assert_eq!(i.eval_str(b"interp alias {} = {} set"), Code::Ok);
            assert_eq!(i.eval_str(b"= v 1"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
        });
    }

    /// A moderately nested `foreach` (well under `NATIVE_EVAL_DEPTH_LIMIT`)
    /// still runs to completion — the safety net must not fire on realistic
    /// nesting depths.
    #[test]
    fn moderately_nested_foreach_still_runs() {
        leak_free(|i| {
            let mut src = String::new();
            for _ in 0..50 {
                src.push_str("foreach x {1} {\n");
            }
            src.push_str("set done 1\n");
            for _ in 0..50 {
                src.push_str("}\n");
            }
            assert_eq!(i.eval_str(src.as_bytes()), Code::Ok);
            assert_eq!(i.eval_str(b"set done"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
        });
    }

    /// A recursive proc with no base case relies purely on a recursion cap
    /// to terminate. Before `NATIVE_EVAL_DEPTH_LIMIT`, this overflowed the
    /// native stack well before the pre-existing `RECURSION_LIMIT` (1000)
    /// was ever reached — a real, unguarded crash on ordinary recursive Tcl
    /// code, not just pathological control-flow nesting.
    #[test]
    fn unbounded_proc_recursion_errors_instead_of_crashing() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"proc r {} { r }"), Code::Ok);
            assert_eq!(i.eval_str(b"r"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"too many nested evaluations (infinite loop?)"
            );
        });
    }

    #[test]
    fn set_and_read_back() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"set x 5"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
            assert_eq!(i.eval_str(b"set y $x"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
        });
    }

    #[test]
    fn command_substitution_closes_the_seam() {
        // [set y 42] evaluates the inner command; x becomes its result.
        leak_free(|i| {
            assert_eq!(i.eval_str(b"set x [set y 42]"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            assert_eq!(i.eval_str(b"set x"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
        });
    }

    #[test]
    fn incr_arithmetic() {
        leak_free(|i| {
            i.eval_str(b"set n 10");
            assert_eq!(i.eval_str(b"incr n"), Code::Ok);
            assert_eq!(i.result_bytes(), b"11");
            assert_eq!(i.eval_str(b"incr n 5"), Code::Ok);
            assert_eq!(i.result_bytes(), b"16");
            // incr of an unset var starts from 0
            assert_eq!(i.eval_str(b"incr fresh"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
        });
    }

    #[test]
    fn undefined_variable_is_an_error() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"set y $nope"), Code::Error);
            assert_eq!(i.result_bytes(), b"can't read \"nope\": no such variable");
        });
    }

    #[test]
    fn unknown_command_is_an_error() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"frobnicate a b"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"frobnicate\"");
        });
    }

    #[test]
    fn missing_alias_target_uses_custom_unknown_with_target_and_args() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(b"proc unknown {cmd args} {list unknown $cmd $args}"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"interp alias {} la {} nosuch"), Code::Ok);
            assert_eq!(i.eval_str(b"la a b"), Code::Ok);
            assert_eq!(i.result_bytes(), b"unknown nosuch {a b}");
        });
    }

    #[test]
    fn missing_alias_target_without_unknown_preserves_target_error_identity() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"interp alias {} la {} nosuch"), Code::Ok);
            assert_eq!(i.eval_str(b"la a b"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"nosuch\"");
        });
    }

    #[test]
    fn missing_alias_target_preserves_prefix_arguments_for_unknown() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(b"proc unknown {cmd args} {list $cmd $args}"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"interp alias {} la {} nosuch prefix"), Code::Ok);
            assert_eq!(i.eval_str(b"la one two"), Code::Ok);
            assert_eq!(i.result_bytes(), b"nosuch {prefix one two}");
        });
    }

    #[test]
    fn missing_alias_target_selects_the_native_handler_context() {
        for version in tcl_dialect::TclVersion::ALL
            .into_iter()
            .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
        {
            leak_free(|i| {
                i.set_runtime_version(version);
                assert_eq!(
                    i.eval_str(b"proc unknown {cmd args} {list global $cmd $args}"),
                    Code::Ok
                );
                assert_eq!(i.eval_str(b"namespace eval n {proc u {cmd args} {list ns $cmd $args}; namespace unknown ::n::u; interp alias {} la {} nosuch; la x}"), Code::Ok);
                let expected: &[u8] = if version == tcl_dialect::TclVersion::V8_5 {
                    b"global nosuch x"
                } else {
                    b"ns nosuch x"
                };
                assert_eq!(i.result_bytes(), expected);
            });
        }
    }

    #[test]
    fn original_command_getter_uses_the_retained_lookup_namespace() {
        for version in tcl_dialect::TclVersion::ALL {
            leak_free(|i| {
                i.set_runtime_version(version);
                assert_eq!(i.eval_str(b"namespace eval A {proc p {} {return A}}; namespace eval B {proc p {} {return B}}"), Code::Ok);
                let head = crate::obj::Owned::fresh(new_string(b"p"));
                for name in [b"::A".as_slice(), b"::B", b"::A"] {
                    let namespace = i.namespaces.borrow().find_namespace(GLOBAL, name).unwrap();
                    let (command, generation) = i
                        .resolve_original_command_at(namespace, head.as_ptr())
                        .unwrap()
                        .unwrap();
                    assert_eq!(
                        i.invoke_bound(command, generation, &[head.as_ptr()]),
                        Code::Ok
                    );
                    assert_eq!(i.result_bytes(), &name[2..]);
                }
            });
        }
    }

    #[test]
    fn tailcall_retired_namespace_relookup_observes_deletion_and_recreation() {
        for version in tcl_dialect::TclVersion::ALL
            .into_iter()
            .filter(|version| *version >= tcl_dialect::TclVersion::V8_6)
        {
            for recreate in [false, true] {
                leak_free(|i| {
                    i.set_runtime_version(version);
                    let source: &[u8] = if recreate {
                        b"namespace eval N {proc issue {} {tailcall target}; proc target {} {return OLD}}; proc leave args {namespace delete N; namespace eval N {proc target {} {return NEW}}}; trace add execution N::issue leave leave; N::issue"
                    } else {
                        b"namespace eval N {proc issue {} {tailcall target}; proc target {} {return OLD}}; proc leave args {namespace delete N}; trace add execution N::issue leave leave; N::issue"
                    };
                    assert_eq!(
                        i.eval_str(source),
                        if recreate { Code::Ok } else { Code::Error }
                    );
                    assert_eq!(
                        i.result_bytes(),
                        if recreate {
                            b"NEW".as_slice()
                        } else {
                            b"namespace \"::N\" not found"
                        }
                    );
                });
            }
        }
    }

    #[test]
    fn tailcall_selected_miss_does_not_relookup_the_callers_command() {
        for version in tcl_dialect::TclVersion::ALL
            .into_iter()
            .filter(|version| *version >= tcl_dialect::TclVersion::V8_6)
        {
            leak_free(|i| {
                i.set_runtime_version(version);
                assert_eq!(i.eval_str(b"namespace eval Iss {proc p {} {tailcall missing}}; namespace eval Caller {proc missing {} {return WRONG}; proc u {cmd args} {return HANDLER}; namespace unknown ::Caller::u; ::Iss::p}"), Code::Ok);
                assert_eq!(i.result_bytes(), b"HANDLER");
            });
        }
    }

    #[test]
    fn missing_alias_unknown_cycle_is_bounded() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"interp alias {} la {} nosuch"), Code::Ok);
            assert_eq!(i.eval_str(b"proc unknown {cmd args} {la}"), Code::Ok);
            assert_eq!(i.eval_str(b"la"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"too many nested evaluations (infinite loop?)"
            );
        });
    }

    #[test]
    fn release_hidden_alias_target_uses_unknown_under_tcl84() {
        use tcl_dialect::TclVersion;

        leak_free(|i| {
            i.set_runtime_version(TclVersion::V8_4);
            assert_eq!(
                i.eval_str(b"proc unknown {cmd args} {list hidden $cmd $args}"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"interp alias {} la {} lassign"), Code::Ok);
            assert_eq!(i.eval_str(b"la {a b} x y"), Code::Ok);
            assert_eq!(i.result_bytes(), b"hidden lassign {{a b} x y}");
        });
    }

    #[test]
    fn absolute_qualified_command_resolves() {
        leak_free(|i| {
            // `::set` is the global `set`, reached via the namespace resolver.
            assert_eq!(i.eval_str(b"::set x 5"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
            assert_eq!(i.eval_str(b"set y $x"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
            // an unknown namespace qualifier is an error
            assert_eq!(i.eval_str(b"::nosuch::cmd a"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"::nosuch::cmd\"");
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn expr_command_end_to_end() {
        leak_free(|i| {
            // braced arithmetic with precedence
            assert_eq!(i.eval_str(b"expr {2 + 3 * 4}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"14");
            // a variable resolved through the frame store (object-preserving)
            assert_eq!(i.eval_str(b"set x 20"), Code::Ok);
            assert_eq!(i.eval_str(b"expr {$x * 2 + 2}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            // overflow promotes to a bignum, then a command substitution feeds back in
            assert_eq!(i.eval_str(b"expr {2 ** 64}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"18446744073709551616");
            assert_eq!(i.eval_str(b"expr {[set x] < 100}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            // divide by zero surfaces the verbatim error
            assert_eq!(i.eval_str(b"expr {1 / 0}"), Code::Error);
            assert_eq!(i.result_bytes(), b"divide by zero");
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn incr_promotes_to_bignum() {
        leak_free(|i| {
            // incr starts at 0 for an unset var
            assert_eq!(i.eval_str(b"incr n"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            // incr past a wide promotes to a bignum (never wraps)
            assert_eq!(i.eval_str(b"set big 9223372036854775807"), Code::Ok); // i64::MAX
            assert_eq!(i.eval_str(b"incr big"), Code::Ok);
            assert_eq!(i.result_bytes(), b"9223372036854775808");
            // incrementing a bignum cell keeps working, and demotes when it fits
            assert_eq!(i.eval_str(b"incr big -1"), Code::Ok);
            assert_eq!(i.result_bytes(), b"9223372036854775807");
            // a non-integer value is rejected verbatim
            assert_eq!(i.eval_str(b"set f 1.5"), Code::Ok);
            assert_eq!(i.eval_str(b"incr f"), Code::Error);
            assert_eq!(i.result_bytes(), b"expected integer but got \"1.5\"");
        });
    }

    /// The `incr` adapter's element + const paths, exercised through the shared
    /// `ValueOps::int_add` seam: array elements increment (and widen) correctly,
    /// an unset element starts at 0, and a `const` scalar is rejected before the
    /// read-modify-write (the const check stays runtime-side).
    #[cfg(have_tommath)]
    #[test]
    fn incr_element_and_const_via_seam() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"set a(k) 10"), Code::Ok);
            assert_eq!(i.eval_str(b"incr a(k) 5"), Code::Ok);
            assert_eq!(i.result_bytes(), b"15");
            // an unset element starts at 0
            assert_eq!(i.eval_str(b"incr a(fresh)"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            // an element past a wide promotes to a bignum (never wraps)
            assert_eq!(i.eval_str(b"set a(big) 9223372036854775807"), Code::Ok);
            assert_eq!(i.eval_str(b"incr a(big)"), Code::Ok);
            assert_eq!(i.result_bytes(), b"9223372036854775808");
            // a const scalar cannot be incremented
            assert_eq!(i.eval_str(b"const c 7"), Code::Ok);
            assert_eq!(i.eval_str(b"incr c"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't incr \"c\": variable is a constant"
            );
        });
    }

    #[test]
    fn qualified_global_aliases_plain_at_top_level() {
        // `::pinged` and `pinged` resolve to the SAME global.
        leak_free(|i| {
            assert_eq!(i.eval_str(b"set ::pinged 1"), Code::Ok);
            assert_eq!(i.eval_str(b"set pinged"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(i.eval_str(b"set pinged 2"), Code::Ok);
            assert_eq!(i.eval_str(b"set ::pinged"), Code::Ok);
            assert_eq!(i.result_bytes(), b"2");
            assert_eq!(i.eval_str(b"unset ::pinged"), Code::Ok);
        });
    }

    #[test]
    fn qualified_var_resolves_through_namespace_table() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"namespace eval a {}"), Code::Ok);
            // qualified write lands in ::a's var table …
            assert_eq!(i.eval_str(b"set ::a::x 5"), Code::Ok);
            // … visible as the unqualified `x` from inside ::a …
            assert_eq!(i.eval_str(b"namespace eval a { set x }"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
            // … and through `$::a::x` substitution.
            assert_eq!(i.eval_str(b"set y $::a::x"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
            assert_eq!(i.eval_str(b"unset ::a::x"), Code::Ok);
        });
    }

    #[test]
    fn qualified_set_into_missing_namespace_errors() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"set ::nosuch::x 1"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't set \"::nosuch::x\": parent namespace doesn't exist"
            );
            // a read of the same name reports the ordinary no-such-variable.
            assert_eq!(i.eval_str(b"set ::nosuch::x"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't read \"::nosuch::x\": no such variable"
            );
        });
    }

    #[test]
    fn namespace_eval_body_set_is_a_namespace_var() {
        // `set x` inside `namespace eval` (a non-proc context) creates a ns var,
        // not a global.
        leak_free(|i| {
            assert_eq!(i.eval_str(b"namespace eval b { set v 10 }"), Code::Ok);
            assert_eq!(i.eval_str(b"set ::b::v"), Code::Ok);
            assert_eq!(i.result_bytes(), b"10");
            assert_eq!(i.eval_str(b"set v"), Code::Error); // not a global
            assert_eq!(i.eval_str(b"unset ::b::v"), Code::Ok);
        });
    }

    #[test]
    fn array_element_via_set_and_subst() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"set a(k) hello"), Code::Ok);
            assert_eq!(i.eval_str(b"set out $a(k)"), Code::Ok);
            assert_eq!(i.result_bytes(), b"hello");
        });
    }

    #[test]
    fn literal_element_references_match_the_selected_native_interpreter() {
        let mut engines: Vec<_> = tcl_test_support::available_tclshs()
            .into_iter()
            .map(|engine| (engine.path, Some(engine.version)))
            .collect();
        if let Some(engine) = tcl_test_support::locate_jimsh().expect("Jim oracle discovery") {
            engines.push((engine.path, None));
        }
        let scripts = [
            "set arr(k) ELEMENT; list ${arr(k)} $arr(k)",
            "set i k; set arr(k) ELEMENT; set {arr($i)} LITERAL; list ${arr($i)} $arr($i)",
            "set arr(k) ELEMENT; list prefix${arr(k)}suffix [subst {${arr(k)}}]",
            "set arr(k) ELEMENT; catch {set result ${arr(missing)}} result; set result",
            "proc probe {} {set arr(k) ELEMENT; set {arr($i)} LITERAL; set i k; list ${arr(k)} ${arr($i)} $arr($i)}; probe",
            "set code [catch {proc probe {{arr(k)}} {list [info exists arr] ${arr(k)} $arr(k) [set arr]}; probe FORMAL} result]; list $code $result",
        ];
        for (path, version) in engines {
            for script in scripts {
                let expected = tcl_test_support::run_script(
                    &path,
                    format!("set c [catch {{{script}}} r]; puts [list $c $r]\n").as_bytes(),
                )
                .expect("native variable reference execution");
                assert!(
                    expected.success() && expected.stderr.is_empty(),
                    "{expected:?}"
                );
                counters::reset();
                {
                    let profile =
                        version.map_or("jim", tcl_dialect::TclVersion::dialect_profile_name);
                    let mut interp = Interp::with_native_core(
                        default_host(),
                        crate::environment::profile_for_dialect(profile),
                        tcl_registry::special_vars::NativeBootstrapInputs::default(),
                    )
                    .expect("selected original native interpreter constructor");
                    let code = interp.eval_str(script.as_bytes());
                    assert!(
                        !interp.host_refusal_pending(),
                        "{path:?}: {script}: {:?}",
                        interp.native_access_refusal(),
                    );
                    let observed = jim_list_bytes([
                        code.as_int().to_string().into_bytes(),
                        interp.result_bytes(),
                    ]);
                    assert_eq!(
                        observed,
                        expected
                            .stdout
                            .strip_suffix(b"\n")
                            .unwrap_or(&expected.stdout),
                        "{path:?}: {script}"
                    );
                    assert!(!interp.host_refusal_pending(), "{path:?}: {script}");
                }
                assert_eq!(
                    counters::finalize(),
                    0,
                    "{path:?}: native source owner leaks"
                );
                assert_eq!(counters::double_free_count(), 0);
            }
        }
    }

    #[test]
    #[cfg(not(have_tommath))]
    fn an_unavailable_sugar_engine_cannot_be_caught_as_a_guest_error() {
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
            let code = interp
                .eval_str(b"set prior 1; catch {set destination $(1+2)} captured; set after 1");
            assert_eq!(code, Code::Error);
            assert_eq!(
                interp.native_access_refusal(),
                Some(tcl_syntax::raw_string::NativeValueAccessRefusal::ExpressionEngineUnavailable)
            );
            assert_eq!(interp.var_get(b"prior").map(obj_bytes), Some(b"1".to_vec()));
            assert!(interp.var_get(b"captured").is_none());
            assert!(interp.var_get(b"destination").is_none());
            assert!(interp.var_get(b"after").is_none());
        });
    }

    #[test]
    #[cfg(not(have_tommath))]
    fn unavailable_safe_integer_engine_stops_after_the_reached_operand() {
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
            let code = interp
                .eval_str(b"set prior 1; catch {string repeat x {1+1}} captured; set after 1");
            assert_eq!(code, Code::Error);
            assert_eq!(
                interp.native_access_refusal(),
                Some(tcl_syntax::raw_string::NativeValueAccessRefusal::ExpressionEngineUnavailable)
            );
            assert_eq!(interp.var_get(b"prior").map(obj_bytes), Some(b"1".to_vec()));
            assert!(interp.var_get(b"captured").is_none());
            assert!(interp.var_get(b"after").is_none());
        });
    }

    #[test]
    #[cfg(have_tommath)]
    fn jim_expression_sugar_bypasses_command_lookup_and_keeps_original_expression() {
        let jim = tcl_test_support::require_jimsh().expect("Jim expression sugar oracle");
        let scripts = [
            "rename expr oldexpr; proc expr args {return WRONG}; set x 3; list $(1+2) \"pre$($x+1)post\" [subst {$(1+2)}] [subst -novariables {$(1+2)}] [subst -nocommands {$(1+2)}]",
            "set seen 0; proc once {} {incr ::seen; return 4}; list $([once]+1) $seen",
            "set prior 1; set code [catch {set destination $(k)} result]; list $code $result $prior [info exists destination]",
            "set value $(1+(2*3)); list $value $()",
        ];
        for script in scripts {
            let expected = tcl_test_support::run_script(
                &jim.path,
                format!("set c [catch {{{script}}} r]; puts [list $c $r]\n").as_bytes(),
            )
            .expect("native sugar execution");
            assert!(
                expected.success() && expected.stderr.is_empty(),
                "{expected:?}"
            );
            leak_free(|interp| {
                interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
                let code = interp.eval_str(script.as_bytes());
                let observed = jim_list_bytes([
                    code.as_int().to_string().into_bytes(),
                    interp.result_bytes(),
                ]);
                assert_eq!(
                    observed,
                    expected
                        .stdout
                        .strip_suffix(b"\n")
                        .unwrap_or(&expected.stdout),
                    "{script}"
                );
                assert!(!interp.host_refusal_pending(), "{script}");
            });
        }
    }

    #[test]
    #[cfg(have_tommath)]
    fn jim_raw_expression_comparisons_errors_and_numeric_nul_match_native() {
        use tcl_test_support::expressions::{
            JIM_RAW_EXPRESSION_ERROR_SCRIPTS, JIM_RAW_EXPRESSION_VALUE_SCRIPTS,
            NUMERIC_NUL_EXPRESSION_OBSERVATION_SCRIPTS,
        };
        let jim = tcl_test_support::require_jimsh().expect("required Jim expression oracle");
        for script in JIM_RAW_EXPRESSION_VALUE_SCRIPTS
            .iter()
            .chain(JIM_RAW_EXPRESSION_ERROR_SCRIPTS)
            .chain(NUMERIC_NUL_EXPRESSION_OBSERVATION_SCRIPTS)
        {
            let observation = format!(
                "set ::errorCode NONE; set c [catch {{{script}}} r]; binary scan $r H* hx; list $c $hx $::errorCode"
            );
            let expected = tcl_test_support::run_script(
                &jim.path,
                format!("puts [{observation}]\n").as_bytes(),
            )
            .expect("native raw expression observation");
            assert!(
                expected.success() && expected.stderr.is_empty(),
                "{script}: {expected:?}"
            );
            leak_free(|interp| {
                interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
                assert_eq!(
                    interp.eval_str(observation.as_bytes()),
                    Code::Ok,
                    "{script}"
                );
                assert!(!interp.host_refusal_pending(), "{script}");
                assert_eq!(
                    interp.result_bytes(),
                    expected
                        .stdout
                        .strip_suffix(b"\n")
                        .unwrap_or(&expected.stdout),
                    "{script}"
                );
            });
        }
    }

    #[test]
    #[cfg(have_tommath)]
    fn numeric_nul_expression_observations_match_every_selected_native_engine() {
        let scripts = tcl_test_support::expressions::NUMERIC_NUL_EXPRESSION_OBSERVATION_SCRIPTS;
        assert_eq!(scripts.len(), 18);
        let mut engines: Vec<_> = tcl_test_support::available_tclshs()
            .into_iter()
            .map(|engine| (engine.path, Some(engine.version)))
            .collect();
        let jim = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
            Some(tcl_test_support::require_jimsh().expect("required Jim numeric oracle"))
        } else {
            tcl_test_support::locate_jimsh().expect("Jim numeric oracle discovery")
        };
        if let Some(jim) = jim {
            engines.push((jim.path, None));
        }
        let mut failures = Vec::new();
        for (path, version) in engines {
            for (index, script) in scripts.iter().enumerate() {
                let expected =
                    tcl_test_support::run_script(&path, format!("puts [{script}]\n").as_bytes())
                        .expect("native numeric NUL observation");
                assert!(
                    expected.success() && expected.stderr.is_empty(),
                    "{script}: {expected:?}"
                );
                leak_free(|interp| {
                    match version {
                        Some(version) => interp.set_runtime_version(version),
                        None => interp
                            .set_dialect_profile(crate::environment::profile_for_dialect("jim")),
                    }
                    let code = interp.eval_str(script.as_bytes());
                    let observed = interp.result_bytes();
                    let wanted = expected
                        .stdout
                        .strip_suffix(b"\n")
                        .unwrap_or(&expected.stdout);
                    if code != Code::Ok || interp.host_refusal_pending() || observed != wanted {
                        failures.push(format!("{version:?} case {index}: {script}\ncode {code:?}, observed {observed:?}, expected {wanted:?}"));
                    }
                });
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn original_variable_word_vectors_match_the_selected_native_interpreter() {
        use tcl_syntax::execution_conformance::{ExecutionDomain, vectors};
        let cases: Vec<_> = vectors(ExecutionDomain::CommandBinding)
            .into_iter()
            .filter(|case| case.id.starts_with("variable_word_"))
            .collect();
        assert_eq!(
            cases.len(),
            12,
            "retain every shared original-variable control"
        );
        let mut engines: Vec<_> = tcl_test_support::available_tclshs()
            .into_iter()
            .map(|engine| (engine.path, Some(engine.version)))
            .collect();
        let jim = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
            Some(tcl_test_support::require_jimsh().expect("required Jim variable-word oracle"))
        } else {
            tcl_test_support::locate_jimsh().expect("Jim variable-word oracle discovery")
        };
        if let Some(engine) = jim {
            engines.push((engine.path, None));
        }
        let mut failures = Vec::new();
        for (path, version) in engines {
            for case in &cases {
                let expected = tcl_test_support::run_script(&path, case.script().as_bytes())
                    .expect("native original-variable execution");
                assert!(
                    expected.success() && expected.stderr.is_empty(),
                    "{expected:?}"
                );
                leak_free(|interp| {
                    match version {
                        Some(version) => interp.set_runtime_version(version),
                        None => interp.set_dialect_profile(
                            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
                        ),
                    }
                    let code = interp.eval_str(case.source.as_bytes());
                    let observed = jim_list_bytes([
                        code.as_int().to_string().into_bytes(),
                        interp.result_bytes(),
                    ]);
                    let wanted = expected
                        .stdout
                        .strip_suffix(b"\n")
                        .unwrap_or(&expected.stdout);
                    if observed != wanted || interp.host_refusal_pending() {
                        failures.push(format!(
                            "{path:?} {}: observed {observed:?}, expected {wanted:?}, host refusal {}",
                            case.id, interp.host_refusal_pending(),
                        ));
                    }
                });
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn expand_marker_splits_arguments() {
        // A recording builtin proves {*} produced the right argv.
        leak_free(|i| {
            fn record(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
                let joined: Vec<Vec<u8>> = argv[1..].iter().map(|&o| obj_bytes(o)).collect();
                interp.set_result_bytes(&joined.join(&b'|'));
                Code::Ok
            }
            i.register_builtin(b"record", record);
            i.eval_str(b"set lst {a b c}");
            assert_eq!(i.eval_str(b"record {*}$lst tail"), Code::Ok);
            assert_eq!(i.result_bytes(), b"a|b|c|tail");
        });
    }

    #[test]
    fn return_sets_code() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"return done"), Code::Return);
            assert_eq!(i.result_bytes(), b"done");
        });
    }

    #[test]
    fn multi_command_script() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"set a 1; set b 2\nset c $a$b"), Code::Ok);
            assert_eq!(i.result_bytes(), b"12");
        });
    }

    #[test]
    fn direct_dispatch_resets_the_prior_result_at_the_central_boundary() {
        leak_free(|i| {
            i.register_builtin(b"resultless", resultless_builtin);
            assert_eq!(i.eval_str(b"set stale prior"), Code::Ok);
            assert_eq!(i.result_bytes(), b"prior");

            let command = new_string(b"resultless");
            // `dispatch` borrows argv whose owners retain each object.
            unsafe { obj::incr_ref_count(command) };
            assert_eq!(i.dispatch(&[command]), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
            unsafe { obj::decr_ref_count(command) };
        });
    }

    #[test]
    fn command_substitution_propagates_non_error_codes() {
        // A non-OK code other than `Error` (here `[return]`) propagates out of
        // `[...]` rather than being treated as an ordinary value (C Tcl). The
        // path is uniform (`code != Ok → Err(code)`), so this also covers
        // `break`/`continue` once those commands land.
        leak_free(|i| {
            assert_eq!(i.eval_str(b"set x [return foo]"), Code::Return);
            assert_eq!(i.result_bytes(), b"foo");
        });
    }

    #[test]
    fn scalar_read_of_array_reports_variable_is_array() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"set a(k) v"), Code::Ok);
            assert_eq!(i.eval_str(b"set a"), Code::Error);
            assert_eq!(i.result_bytes(), b"can't read \"a\": variable is array");
        });
    }

    #[test]
    fn expand_split_error_names_the_right_failure() {
        // `{*}` over a value whose list form has an unmatched quote reports the
        // quote failure, not a hardcoded brace message.
        leak_free(|i| {
            assert_eq!(i.eval_str(b"set s {\"abc}"), Code::Ok); // s = "abc
            assert_eq!(i.eval_str(b"list {*}$s"), Code::Error);
            assert_eq!(i.result_bytes(), b"unmatched open quote in list");
        });
    }

    /// Selecting the release selects the *numeral grammar* the whole runtime
    /// reads with, because every numeral goes through one facility
    /// (`tcl_syntax::number`) whose ambient dialect `set_runtime_version`
    /// installs. Tcl 9.0 defines `KILL_OCTAL` in `tclStrToD.c`, so a bare
    /// leading zero is decimal there (`0755` == 755) while 8.4/8.6 read it as
    /// octal (`0755` == 493); `0b`/`0o` only exist from 8.5 and `0d` from 9.0.
    /// Exercised through `expr`, which reads operands with
    /// `ParseFlags::default()` — i.e. exactly the ambient this installs.
    #[cfg(have_tommath)]
    #[test]
    fn expr_numerals_follow_the_release_selected_number_grammar() {
        use tcl_dialect::TclVersion;

        // TP: octal-by-leading-zero holds up to 8.6 …
        for version in [TclVersion::V8_4, TclVersion::V8_6] {
            leak_free(|i| {
                i.set_runtime_version(version);
                assert_eq!(ok(i, b"expr {0755}"), b"493", "{version:?}");
                assert_eq!(ok(i, b"expr {010 + 1}"), b"9", "{version:?}");
                // TN: without a leading zero the release cannot matter.
                assert_eq!(ok(i, b"expr {755}"), b"755", "{version:?}");
                // A leading-zero run before `.`/`e` stays a decimal float in
                // every release (C backtracks out of its octal state).
                assert_eq!(ok(i, b"expr {07.5}"), b"7.5", "{version:?}");
            });
        }

        // FN: 9.0 must not keep reading it as octal — the direction of this
        // change is easy to invert, so pin both sides.
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V9_0);
            assert_eq!(ok(i, b"expr {0755}"), b"755");
            assert_eq!(ok(i, b"expr {010 + 1}"), b"11");
            assert_eq!(ok(i, b"expr {755}"), b"755");
            assert_eq!(ok(i, b"expr {07.5}"), b"7.5");
        });

        // FP: a prefix its release does not have is not a numeral at all, so
        // the word is a bareword rather than a silently-different number.
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V8_4);
            assert_eq!(i.eval_str(b"expr {0o17}"), Code::Error);
        });
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V8_6);
            assert_eq!(ok(i, b"expr {0o17}"), b"15");
            assert_eq!(i.eval_str(b"expr {1_0}"), Code::Error);
        });
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V9_0);
            assert_eq!(ok(i, b"expr {0o17}"), b"15");
            assert_eq!(ok(i, b"expr {1_0}"), b"10");
        });
    }

    /// Selecting the release selects the *lexing grammar* scripts parse
    /// under and the builtin command surface: under an 8.4 pin `{*}` does
    /// not expand and the first-close `${…}`
    /// rule applies, and `lassign` (8.5+) resolves to `invalid command
    /// name` — while a user-defined proc of the same name stays callable
    /// (the compat-polyfill pattern).
    #[test]
    fn grammar_and_command_surface_follow_the_selected_release() {
        use tcl_dialect::TclVersion;

        // The `{*}` expansion grammar (TIP 157, 8.5+).
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V9_0);
            assert_eq!(ok(i, b"llength [list {*}{a b}]"), b"2");
        });
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V8_4);
            // No expansion under 8.4 (TIP 157 is 8.5+), so `{*}{a b}` is a
            // braced word `{*}` with `{a b}` welded onto its close-brace —
            // which C rejects. Measured on tclsh8.4.20: `llength [list
            // {*}{a b}]` reports `extra characters after close-brace`, where
            // 8.5.19/8.6.16/9.0.4/9.1b0 all answer `2`. The boundary owner's
            // `welded_after_close` enforces this: a braced word directly
            // followed by another brace-delimited word is rejected rather
            // than merged into one word.
            assert_eq!(i.eval_str(b"llength [list {*}{a b}]"), Code::Error);
            assert_eq!(i.result_bytes(), b"extra characters after close-brace");
        });

        // The `${…}` delimiting rule: 8.x stops at the first `}`
        // (its `Tcl_ParseVarName` counts no braces), 9.x nests. Verified
        // against tclsh8.4.20 / tclsh9.0.4.
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V8_4);
            assert_eq!(i.eval_str(b"set \"a{b\" 5"), Code::Ok);
            assert_eq!(i.eval_str(b"set \"a{b}c\" 9"), Code::Ok);
            assert_eq!(ok(i, b"set r ${a{b}c}"), b"5c}");
        });
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V9_0);
            assert_eq!(i.eval_str(b"set \"a{b\" 5"), Code::Ok);
            assert_eq!(i.eval_str(b"set \"a{b}c\" 9"), Code::Ok);
            assert_eq!(ok(i, b"set r ${a{b}c}"), b"9");
        });

        // The builtin surface: lassign is 8.5+, lpop is 9.0+.
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V8_4);
            assert_eq!(i.eval_str(b"lassign {a b} x"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"lassign\"");
            assert_eq!(ok(i, b"llength [info commands lassign]"), b"0");
            // The polyfill pattern: a user proc wins over the hidden builtin.
            assert_eq!(
                i.eval_str(b"proc lassign {l args} { return polyfill }"),
                Code::Ok
            );
            assert_eq!(ok(i, b"lassign {a b} x"), b"polyfill");
        });
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V8_6);
            assert_eq!(ok(i, b"lassign {a b} x"), b"b");
            assert_eq!(i.eval_str(b"set l {a b c}"), Code::Ok);
            assert_eq!(i.eval_str(b"lpop l"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"lpop\"");
        });
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V9_0);
            assert_eq!(i.eval_str(b"set l {a b c}"), Code::Ok);
            assert_eq!(ok(i, b"lpop l"), b"c");
        });
    }

    /// The availability gate is a property of the **final resolved
    /// builtin**, not of the spelling the caller wrote. Without that, the
    /// two resolve-then-`invoke` shapes — the alias trampoline and the
    /// `namespace import` redirect — could reach the builtin through a
    /// second, ungated resolution and make an 8.4-hidden `lassign` callable
    /// as `interp alias {} la {} lassign; la {a b} x y`.
    ///
    /// Error identity is oracled against real tclsh 8.6.16 / 9.0.4:
    /// `interp alias {} la {} nosuchcmd; la a b` reports `invalid command
    /// name "nosuchcmd"` — the *target* name, not the alias — so a
    /// release-hidden target reports the same way a deleted one does.
    #[test]
    fn the_release_command_surface_gates_alias_and_import_dispatch() {
        use tcl_dialect::TclVersion;

        // The reviewer's reproducer: an 8.4 alias to the 8.5+ `lassign`.
        // (Creating the alias still succeeds — real Tcl binds alias targets
        // lazily, so an alias to a nonexistent command is legal.)
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V8_4);
            assert_eq!(i.eval_str(b"interp alias {} la {} lassign"), Code::Ok);
            assert_eq!(i.eval_str(b"la {a b} x y"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"lassign\"");
            // …and nothing was assigned through the alias.
            assert_eq!(i.eval_str(b"set x"), Code::Error);
            // An alias *chain* is gated at the final builtin too, not merely
            // at the first hop.
            assert_eq!(i.eval_str(b"interp alias {} lb {} la"), Code::Ok);
            assert_eq!(i.eval_str(b"lb {a b} x y"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"lassign\"");
        });
        // The same alias is a working `lassign` on a release that carries it.
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V9_0);
            assert_eq!(i.eval_str(b"interp alias {} la {} lassign"), Code::Ok);
            assert_eq!(ok(i, b"la {a b} x y; list $x $y"), b"a b");
        });

        // The `namespace import` redirect.  A leading `::` with no separator
        // before the command (`::lassign`) must retain its absolute-global
        // meaning while resolving the import source (the public `namespace
        // qualifiers` result intentionally returns an empty string here).
        // First pin the exact issue reproducer, then explicitly export the
        // builtin so the redirect and release-hidden dispatch halves remain
        // observable in the same test.
        const IMPORT: &[u8] = b"namespace eval n {namespace import ::lassign}";
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V9_0);
            assert_eq!(i.eval_str(IMPORT), Code::Ok);
            // This reproducer succeeds without an explicit export.  Global
            // builtins are not exported by default, so this
            // records no alias; the important contract is that it does not
            // resolve the absolute root as the destination namespace.
            assert_eq!(i.eval_str(b"namespace export lassign"), Code::Ok);
            assert_eq!(i.eval_str(IMPORT), Code::Ok);
            assert_eq!(ok(i, b"n::lassign {a b} x y; list $x $y"), b"a b");
            i.set_runtime_version(TclVersion::V8_4);
            assert_eq!(i.eval_str(b"n::lassign {a b} x y"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"::lassign\"");
        });

        // Direct dispatch is unchanged by moving the gate into the shared
        // resolver — both the hidden and the carried release.
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V8_4);
            assert_eq!(i.eval_str(b"lassign {a b} x y"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"lassign\"");
        });
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V9_0);
            assert_eq!(ok(i, b"lassign {a b} x y; list $x $y"), b"a b");
        });
    }

    /// An import keeps the final builtin's registry identity when that source
    /// moves into the hidden table. Repinning the runtime surface must gate the
    /// visible import, while an explicit `invokehidden` remains the privileged
    /// path that deliberately bypasses ordinary command visibility.
    #[test]
    fn an_imported_hidden_builtin_keeps_its_release_gate() {
        use tcl_dialect::TclVersion;

        leak_free(|i| {
            i.set_runtime_version(TclVersion::V9_0);
            assert_eq!(
                i.eval_str(
                    b"namespace export lpop
                      namespace eval n {namespace import ::lpop}
                      set l {a b c}
                      interp hide {} lpop hp
                      n::lpop l",
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"c");

            i.set_runtime_version(TclVersion::V8_6);
            assert_eq!(i.eval_str(b"n::lpop l"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"::hp\"");
            assert_eq!(ok(i, b"set l"), b"a b");

            assert_eq!(i.eval_str(b"interp invokehidden {} hp l"), Code::Ok);
            assert_eq!(i.result_bytes(), b"b");

            i.set_runtime_version(TclVersion::V9_0);
            assert_eq!(i.eval_str(b"interp expose {} hp lz"), Code::Ok);
            i.set_runtime_version(TclVersion::V8_6);
            assert_eq!(i.eval_str(b"lz l"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"lz\"");
            assert_eq!(ok(i, b"info commands lz"), b"");
            i.set_runtime_version(TclVersion::V9_0);
        });
    }

    /// The same gate on the two *non*-dispatch shapes that would hand a
    /// release-hidden builtin back ungated: `rename`, which would rebind it
    /// under a name the registry has no spec for, and `interp hide`, which
    /// would park it where `interp invokehidden` reaches it.
    ///
    /// `rename`'s refusal is oracled against tclsh 8.6.16 / 9.0.4:
    /// `rename nosuchcmd zz` → `can't rename "nosuchcmd": command doesn't
    /// exist`; an empty destination uses `can't delete`. Hiding a command that
    /// does not exist raises `unknown command`, matching Tcl_HideCommand and
    /// avoiding a swallowed typo in a security-sensitive path.
    #[test]
    fn the_release_command_surface_gates_rename_and_hide() {
        use tcl_dialect::TclVersion;

        leak_free(|i| {
            i.set_runtime_version(TclVersion::V8_4);
            assert_eq!(i.eval_str(b"rename lassign lz"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't rename \"lassign\": command doesn't exist"
            );
            assert_eq!(i.eval_str(b"lz {a b} x y"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"lz\"");

            assert_eq!(i.eval_str(b"interp hide {} lassign"), Code::Error);
            assert_eq!(i.result_bytes(), b"unknown command \"lassign\"");
            assert_eq!(
                i.eval_str(b"interp invokehidden {} lassign {a b} x y"),
                Code::Error
            );
            assert_eq!(i.result_bytes(), b"invalid hidden command name \"lassign\"");
            assert_eq!(i.eval_str(b"interp hide {} lassign"), Code::Error);
            assert_eq!(i.result_bytes(), b"unknown command \"lassign\"");
        });
        // Both stay ordinary operations on a release that carries `lassign`.
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V9_0);
            assert_eq!(i.eval_str(b"rename lassign lz"), Code::Ok);
            assert_eq!(ok(i, b"lz {a b} x y; list $x $y"), b"a b");
        });
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V9_0);
            assert_eq!(i.eval_str(b"interp hide {} lassign"), Code::Ok);
            assert_eq!(
                ok(i, b"interp invokehidden {} lassign {a b} x y; list $x $y"),
                b"a b"
            );
        });
    }

    /// A fresh interpreter installs *its own* grammar rather than inheriting
    /// whatever an earlier interpreter left in the thread-ambient slot — the
    /// unchanged-version short-circuit in `set_runtime_version` would otherwise
    /// leave a default-release interp reading numerals as 8.6.
    #[cfg(have_tommath)]
    #[test]
    fn a_fresh_interp_reinstalls_the_number_grammar_after_an_8_6_interp() {
        use tcl_dialect::TclVersion;

        leak_free(|i| {
            i.set_runtime_version(TclVersion::V8_6);
            assert_eq!(ok(i, b"expr {0755}"), b"493");
        });
        // Same thread, ambient left at 8.6 by the interp above.
        leak_free(|i| {
            assert_eq!(i.runtime_version(), DEFAULT_RUNTIME_VERSION);
            assert_eq!(ok(i, b"expr {0755}"), b"755");
            // …and re-pinning the release it already reports still installs it.
            i.set_runtime_version(DEFAULT_RUNTIME_VERSION);
            assert_eq!(ok(i, b"expr {0755}"), b"755");
        });
    }

    /// `return -code` / `try on` read their integer through the same one
    /// facility, so the spellings they accept track the emulated release
    /// (C reads it with `Tcl_GetIntFromObj`, whose grammar is the release's).
    /// The ambient is thread-local, so each dialect is installed explicitly
    /// here rather than inherited from another test.
    #[test]
    fn completion_code_integers_follow_the_release_selected_number_grammar() {
        use tcl_syntax::number::{NumberSyntax, set_runtime_syntax};

        // Release-independent: decimal, hex, the full signed *and* unsigned
        // 32-bit window, and its reduction to an `int`.
        for syntax in [
            NumberSyntax::Tcl84,
            NumberSyntax::Tcl85,
            NumberSyntax::Tcl90,
        ] {
            set_runtime_syntax(syntax);
            assert_eq!(parse_completion_int(b"42"), Some(42), "{syntax:?}");
            assert_eq!(parse_completion_int(b"+42"), Some(42), "{syntax:?}");
            assert_eq!(parse_completion_int(b"-7"), Some(-7), "{syntax:?}");
            assert_eq!(parse_completion_int(b" 7 "), Some(7), "{syntax:?}");
            assert_eq!(parse_completion_int(b"0x1f"), Some(31), "{syntax:?}");
            assert_eq!(
                parse_completion_int(b"-2147483648"),
                Some(i32::MIN),
                "{syntax:?}"
            );
            // The unsigned half is reachable and wraps to an `int`.
            assert_eq!(parse_completion_int(b"0xFFFFFFFF"), Some(-1), "{syntax:?}");
            assert_eq!(
                parse_completion_int(b"2147483648"),
                Some(i32::MIN),
                "{syntax:?}"
            );
            // Outside the window, a bare prefix, junk, a float, and a magnitude
            // past a wide are all rejected.
            assert_eq!(parse_completion_int(b"4294967296"), None, "{syntax:?}");
            assert_eq!(parse_completion_int(b"-2147483649"), None, "{syntax:?}");
            assert_eq!(parse_completion_int(b"0x"), None, "{syntax:?}");
            assert_eq!(parse_completion_int(b""), None, "{syntax:?}");
            assert_eq!(parse_completion_int(b"abc"), None, "{syntax:?}");
            assert_eq!(parse_completion_int(b"1.5"), None, "{syntax:?}");
            assert_eq!(parse_completion_int(b"12x"), None, "{syntax:?}");
            assert_eq!(
                parse_completion_int(b"99999999999999999999"),
                None,
                "{syntax:?}"
            );
            // One sign only — the facility consumes it, so `--5` is not 5.
            assert_eq!(parse_completion_int(b"--5"), None, "{syntax:?}");
        }

        // Octal-by-leading-zero up to 8.6, decimal from 9.0.
        for syntax in [NumberSyntax::Tcl84, NumberSyntax::Tcl85] {
            set_runtime_syntax(syntax);
            assert_eq!(parse_completion_int(b"010"), Some(8), "{syntax:?}");
            assert_eq!(parse_completion_int(b"-010"), Some(-8), "{syntax:?}");
            // An invalid octal digit stops the scan, so the whole word is not a
            // number (C's "bad octal" report).
            assert_eq!(parse_completion_int(b"08"), None, "{syntax:?}");
        }
        set_runtime_syntax(NumberSyntax::Tcl90);
        assert_eq!(parse_completion_int(b"010"), Some(10));
        assert_eq!(parse_completion_int(b"-010"), Some(-10));
        assert_eq!(parse_completion_int(b"08"), Some(8));

        // `0o`/`0b` arrive in 8.5, `0d` and `_` separators in 9.0 — an
        // unavailable prefix is not a prefix, so the word is not an integer.
        set_runtime_syntax(NumberSyntax::Tcl84);
        assert_eq!(parse_completion_int(b"0o17"), None);
        assert_eq!(parse_completion_int(b"0b101"), None);
        assert_eq!(parse_completion_int(b"0d99"), None);
        assert_eq!(parse_completion_int(b"1_0"), None);
        set_runtime_syntax(NumberSyntax::Tcl85);
        assert_eq!(parse_completion_int(b"0o17"), Some(15));
        assert_eq!(parse_completion_int(b"0b101"), Some(5));
        assert_eq!(parse_completion_int(b"0d99"), None);
        assert_eq!(parse_completion_int(b"1_0"), None);
        set_runtime_syntax(NumberSyntax::Tcl90);
        assert_eq!(parse_completion_int(b"0o17"), Some(15));
        assert_eq!(parse_completion_int(b"0b101"), Some(5));
        assert_eq!(parse_completion_int(b"0d99"), Some(99));
        assert_eq!(parse_completion_int(b"1_0"), Some(10));
    }

    /// The same release gate reached through the script surface: `return -level
    /// 0 -code 010` completes with code 8 up to 8.6 and 10 from 9.0.
    #[test]
    fn return_code_word_reads_its_integer_in_the_emulated_release() {
        use tcl_dialect::TclVersion;

        leak_free(|i| {
            i.set_runtime_version(TclVersion::V8_6);
            assert_eq!(ok(i, b"catch {return -level 0 -code 010}"), b"8");
        });
        leak_free(|i| {
            i.set_runtime_version(TclVersion::V9_0);
            assert_eq!(ok(i, b"catch {return -level 0 -code 010}"), b"10");
        });
    }
    /// An empty operand (and a whitespace-only or sign-only one) must report a
    /// Tcl error on **every** release rather than abort the process. Regression:
    /// the shared parser's octal-by-leading-zero branch read its first byte
    /// unguarded, so with a pre-9.0 grammar installed — which is exactly what
    /// `set_runtime_version` now does — `expr {$empty + 1}` panicked in
    /// `tcl_syntax::number::parse` instead of failing the expression.
    #[cfg(have_tommath)]
    #[test]
    fn an_empty_numeral_errors_instead_of_panicking_on_every_release() {
        use tcl_dialect::TclVersion;

        for version in [TclVersion::V8_4, TclVersion::V8_6, TclVersion::V9_0] {
            leak_free(|i| {
                i.set_runtime_version(version);
                assert_eq!(
                    i.eval_str(b"set x {}; expr {$x + 1}"),
                    Code::Error,
                    "{version:?}"
                );
                assert_eq!(
                    i.eval_str(b"set x { }; expr {$x + 1}"),
                    Code::Error,
                    "{version:?}"
                );
                assert_eq!(
                    i.eval_str(b"set x -; expr {$x + 1}"),
                    Code::Error,
                    "{version:?}"
                );
            });
        }
    }

    /// Every command `interp create -safe` hides, per release, measured on
    /// the reference interpreters with
    /// `interp create -safe s; lsort [interp hidden s]` — top-level command
    /// names only. The `tcl:file:*` / `tcl:zipfs:*` / `tcl:clock:*` entries a
    /// real 8.6+ interpreter also lists are C's internal rewrite names for
    /// the *unsafe subcommands* of an ensemble, not commands a script can
    /// name; this runtime does not model ensembles that way.
    ///
    /// Patch levels measured: 8.4.20, 8.5.19, 8.6.14, 9.0.4, 9.1b0.
    #[cfg(all(test, have_tommath))]
    const MEASURED_SAFE_HIDDEN: &[(tcl_dialect::TclVersion, &[&str])] = &[
        (
            tcl_dialect::TclVersion::V8_4,
            &[
                "cd",
                "encoding",
                "exec",
                "exit",
                "fconfigure",
                "file",
                "glob",
                "load",
                "open",
                "pwd",
                "socket",
                "source",
            ],
        ),
        // 8.5 adds `unload` (TIP 100); 8.6 is the same set.
        (
            tcl_dialect::TclVersion::V8_5,
            &[
                "cd",
                "encoding",
                "exec",
                "exit",
                "fconfigure",
                "file",
                "glob",
                "load",
                "open",
                "pwd",
                "socket",
                "source",
                "unload",
            ],
        ),
        (
            tcl_dialect::TclVersion::V8_6,
            &[
                "cd",
                "encoding",
                "exec",
                "exit",
                "fconfigure",
                "file",
                "glob",
                "load",
                "open",
                "pwd",
                "socket",
                "source",
                "unload",
            ],
        ),
        // 9.0 adds `zipfs`.
        (
            tcl_dialect::TclVersion::V9_0,
            &[
                "cd",
                "encoding",
                "exec",
                "exit",
                "fconfigure",
                "file",
                "glob",
                "load",
                "open",
                "pwd",
                "socket",
                "source",
                "unload",
                "zipfs",
            ],
        ),
        // 9.1 additionally lists `clock` — an artefact of the safe base
        // hiding the C `clock` and immediately re-providing a safe one, not
        // an unsafety fact, so it is deliberately absent from the registry
        // trait (`clock format 0 -gmt 1` works inside a 9.1 safe child).
        (
            tcl_dialect::TclVersion::V9_1,
            &[
                "cd",
                "clock",
                "encoding",
                "exec",
                "exit",
                "fconfigure",
                "file",
                "glob",
                "load",
                "open",
                "pwd",
                "socket",
                "source",
                "unload",
                "zipfs",
            ],
        ),
    ];

    /// The hidden set under each pinned release is exactly the measured
    /// tclsh set, narrowed to the commands this runtime carries — `make_safe`
    /// reads that set from the registry's `Traits::SAFE_INTERP_HIDDEN` query
    /// rather than a hand-kept name list.
    ///
    /// The narrowing *is* the per-release mechanism, not a fudge: `unload`
    /// (8.5+) and `zipfs` (9.0+) are release-gated commands, so "hide what
    /// the trait names, if this interpreter carries it" reproduces the
    /// differences between the rows with no second availability rule. Any
    /// residue is asserted to be genuinely absent from `info commands`, so
    /// implementing one of them forces this test to be revisited.
    #[cfg(have_tommath)]
    #[test]
    fn safe_interp_hidden_set_matches_the_measured_tclsh_sets() {
        for &(version, measured) in MEASURED_SAFE_HIDDEN {
            leak_free(|i| {
                i.set_runtime_version(version);
                // `clock` is implemented and stays *visible*, which is what a
                // real 9.1 safe child does behaviourally; only its appearance
                // in `interp hidden` differs.
                let carried: Vec<&str> = measured
                    .iter()
                    .copied()
                    .filter(|name| {
                        *name != "clock" && {
                            let script = format!("llength [info commands {name}]");
                            ok(i, script.as_bytes()) == b"1"
                        }
                    })
                    .collect();
                assert_eq!(i.eval_str(b"set s [interp create -safe]"), Code::Ok);
                let hidden = ok(i, b"lsort [$s hidden]");
                let hidden = String::from_utf8_lossy(&hidden);
                let actual: Vec<&str> = hidden.split_whitespace().collect();
                assert_eq!(actual, carried, "{version:?} hidden set");
                i.eval_str(b"interp delete $s");
            });
        }
    }

    /// TP: `clock` stays callable inside a safe child on every release —
    /// measured on tclsh 8.6.14, 9.0.4 and 9.1b0, where
    /// `s eval {clock format 0 -gmt 1}` succeeds even though 9.1 lists
    /// `clock` in `interp hidden`.
    #[cfg(have_tommath)]
    #[test]
    fn clock_remains_callable_in_a_safe_child() {
        for &(version, _) in MEASURED_SAFE_HIDDEN {
            leak_free(|i| {
                i.set_runtime_version(version);
                assert_eq!(i.eval_str(b"set s [interp create -safe]"), Code::Ok);
                assert_eq!(
                    ok(i, b"$s eval {clock format 0 -gmt 1 -format %Y}"),
                    b"1970",
                    "{version:?}"
                );
                i.eval_str(b"interp delete $s");
            });
        }
    }

    /// The core packages a bare interpreter pre-provides follow the pinned
    /// release, so `package require Tcl 8.5` fails under a 9.x pin exactly as
    /// `tclsh9.0` fails it, rather than succeeding the way it would if both
    /// engines hardcoded `9.0.4`/`Tcl`+`tcl` regardless of the pin.
    ///
    /// Measured (`package provide <name>` in a fresh `tclsh`):
    /// 8.4.20 → `Tcl` = `8.4`, no `tcl`; 8.5.19 → `Tcl` = `8.5.19`;
    /// 8.6.14 → `Tcl` = `8.6.14`, `TclOO` = `1.1.0`; 9.0.4 and 9.1b0 → all
    /// four names, at the patch level / `1.3.1`.
    #[cfg(have_tommath)]
    #[test]
    fn core_package_provides_follow_the_pinned_release() {
        use tcl_dialect::TclVersion;

        for version in TclVersion::ALL {
            leak_free(|i| {
                i.set_runtime_version(version);
                for core in version.core_provided_packages() {
                    let script = format!("package provide {}", core.name);
                    assert_eq!(
                        ok(i, script.as_bytes()),
                        core.version.as_bytes(),
                        "{version:?} provides {}",
                        core.name
                    );
                }
                // FN guard: a name this release does not pre-provide answers
                // with the empty string, not a stale earlier pin's version.
                if version < TclVersion::V9_0 {
                    assert_eq!(ok(i, b"package provide tcl"), b"", "{version:?}");
                }
                if version < TclVersion::V8_6 {
                    assert_eq!(ok(i, b"package provide TclOO"), b"", "{version:?}");
                }
                // `package require Tcl 8.5` means [8.5, 9) — satisfied on
                // 8.5/8.6, a version conflict on 8.4 and on every 9.x.
                let wanted = matches!(version, TclVersion::V8_5 | TclVersion::V8_6);
                assert_eq!(
                    i.eval_str(b"package require Tcl 8.5") == Code::Ok,
                    wanted,
                    "{version:?}: package require Tcl 8.5"
                );
            });
        }
    }

    /// `::tcl::build-info` reports the pinned release's build identity, and
    /// splits its fields the way C does — `patchlevel` up to the `+`,
    /// `version` up to the second `.`. Measured: `tclsh9.0` answers
    /// `9.0.4` / `9.0`, and `tclsh9.1` answers `9.1.0` / `9.1`.
    #[cfg(have_tommath)]
    #[test]
    fn build_info_follows_the_pinned_release() {
        use tcl_dialect::TclVersion;

        for (version, patchlevel, short) in [
            (TclVersion::V9_0, &b"9.0.4"[..], &b"9.0"[..]),
            (TclVersion::V9_1, &b"9.1.0"[..], &b"9.1"[..]),
        ] {
            leak_free(|i| {
                i.set_runtime_version(version);
                assert_eq!(ok(i, b"::tcl::build-info patchlevel"), patchlevel);
                assert_eq!(ok(i, b"::tcl::build-info version"), short);
                // An unset build flag reports 0 rather than erroring.
                assert_eq!(ok(i, b"::tcl::build-info memdebug"), b"0");
            });
        }
    }

    /// `set_dialect_profile` is the profile form of `pin_context`: the context a
    /// profile names, resolved through the ingress, pins the same profile, the
    /// same release and the same identity.
    #[test]
    fn the_profile_form_of_a_pin_is_the_context_the_profile_names() {
        for profile in tcl_dialect::DialectProfile::all().iter().chain([
            tcl_dialect::DialectProfile::plain_tcl(),
            tcl_dialect::DialectProfile::tk(),
        ]) {
            let mut by_profile = Interp::new();
            by_profile.set_dialect_profile(profile);
            let mut by_context = Interp::new();
            by_context
                .pin_context(&tcl_registry::model::runtime_context_for_profile(profile))
                .unwrap_or_else(|error| panic!("{}: {error}", profile.name));

            assert!(
                std::ptr::eq(by_context.dialect_profile(), by_profile.dialect_profile()),
                "{}",
                profile.name
            );
            assert_eq!(by_context.runtime_version(), by_profile.runtime_version());
            assert_eq!(by_context.runtime_context(), by_profile.runtime_context());
            assert_eq!(by_context.held_identity(), by_profile.held_identity());
        }
    }

    /// The runtime states the identity of the world it is pinned to: the
    /// context's fields, this build's ABI, intrinsic table and embedded library,
    /// and no pack facts.
    #[test]
    fn an_interp_states_the_identity_of_the_world_it_is_pinned_to() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").expect("catalogue profile");
        let mut interp = Interp::new();
        interp.set_dialect_profile(profile);
        let expected = tcl_registry::model::runtime_context_for_profile(profile)
            .identity(&[], tcl_registry::intrinsic_table_hash());
        assert_eq!(interp.held_identity(), expected);
        assert_eq!(expected.environment, "tcl8.6");
        assert_eq!(expected.release, "8.6");
        assert_eq!(
            expected.abi_version,
            tcl_runtime_api::codegen_abi::CODEGEN_ABI_VERSION
        );
        assert!(expected.packs.is_empty());

        interp.set_dialect_profile(tcl_dialect::DialectProfile::plain_tcl());
        assert_eq!(interp.held_identity().environment, "tcl");
    }

    /// A context the ingress does not agree with is an error and leaves the pin
    /// as it was; an overlay nothing has installed is one of them, and is never
    /// the un-overlaid generation under another name.
    #[test]
    fn a_context_the_ingress_refuses_leaves_the_pin_unchanged() {
        use tcl_registry::model::PinError;

        const OVERLAY: u64 = 0x0C0_1704;
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").expect("catalogue profile");
        let context = tcl_registry::model::runtime_context_for_profile(profile);
        let mut interp = Interp::new();
        interp
            .pin_context(&context)
            .expect("the profile's own context");

        let mut unknown = context.clone();
        unknown.environment = "no-such-environment".to_owned();
        let mut wrong_build = context.clone();
        wrong_build.build = tcl_dialect::model::BuildProfileId::JimFull;
        let mut missing_overlay = context.clone();
        missing_overlay.overlay_generation = OVERLAY;
        for (what, refused) in [
            ("unknown", unknown),
            ("build", wrong_build),
            ("overlay", missing_overlay.clone()),
        ] {
            assert!(interp.pin_context(&refused).is_err(), "{what}");
            assert!(std::ptr::eq(interp.dialect_profile(), profile), "{what}");
            assert_eq!(interp.runtime_context(), context, "{what}");
        }
        assert!(matches!(
            interp.pin_context(&missing_overlay),
            Err(PinError::OverlayMiss(miss)) if miss.overlay == OVERLAY
        ));

        tcl_registry::registry_for_profile_with_overlay(profile, OVERLAY, |_| {});
        interp
            .pin_context(&missing_overlay)
            .expect("installed, so it pins");
        assert_eq!(interp.runtime_context().overlay_generation, OVERLAY);
        assert!(std::ptr::eq(interp.dialect_profile(), profile));
    }
}

#[cfg(test)]
mod native_error_log_tests;

mod native_command_traces;
mod native_error_headers;
mod native_return_instruction;
mod native_trace_result;

#[cfg(test)]
mod native_child_alias_publication_tests;
