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

//! The interpreter: `Tcl_Interp` + the eval loop + command dispatch (T1.4).
//!
//! Builds on the value model (T1.1), parse/subst (T1.2), and the frame/var
//! store (T1.3). This is the **interpreter-fallback** path — the AOT compiler
//! is the primary route (north star); this runs what isn't (yet) AOT-compiled
//! and what genuinely needs runtime interpretation (`eval $dynamic`, etc.).
//!
//! Closes the **command** half of T1.2's subst seam: a `[cmd]` substitution
//! recursively evaluates its inner script through this loop.
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

use core::ffi::c_char;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use tcl_core_types::RecursionLimit;
use tcl_runtime_api::guard::{
    GuardDomain, GuardDomains, GuardError, GuardIdentity, GuardManager, GuardToken,
};

use crate::builtins;
use crate::frame::{FrameStack, Link, VarError};
use crate::namespace::{Namespaces, NsId, RenameOutcome, GLOBAL};
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
}

/// One entry of the source-location stack (`cmdFramePtr`; PC-5) — the runtime
/// state `info frame` reports. One is pushed per script-evaluation level Tcl
/// tracks: the top-level script, a proc call, an `eval`/`uplevel` body, and a
/// `source`d file — but **not** a `[cmd]` substitution or an inline
/// `if`/`while`/`for`/`foreach` body (those run in the enclosing frame). The
/// `cmd`/`line` are updated to the currently-executing command of the
/// frame-owning script as the eval loop steps through it.
struct CmdFrame {
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
type VarTeardownCallback = (Vec<u8>, Vec<u8>, Vec<u8>, bool);

/// The outcome of an ensemble `-unknown` handler (`EnsembleUnknownCallback`).
enum EnsembleUnknown {
    /// A non-empty result: the replacement command prefix to dispatch.
    Prefix(Vec<Vec<u8>>),
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
            line: 1,
            oo: None,
            lambda: None,
        }
    }
}

/// Join argument objects with single spaces (the `eval`/`interp eval` body form).
fn join_words(args: &[*mut TclObj]) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, &a) in args.iter().enumerate() {
        if i > 0 {
            out.push(b' ');
        }
        out.extend_from_slice(&obj_bytes(a));
    }
    out
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
/// published to the `::errorInfo`/`::errorCode` globals when the error is caught
/// or reaches the outermost eval.
#[derive(Default, Clone)]
pub(crate) struct ExceptionState {
    /// The accumulating `errorInfo`. `None` until the first frame is appended
    /// (C's `errorInfo == NULL`) — which selects `while executing` over `invoked
    /// from within` and seeds the buffer from the result message.
    info: Option<Vec<u8>>,
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
    info: Option<Vec<u8>>,
    code: Vec<u8>,
    code_explicit: bool,
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
    exc: ExceptionState,
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
            exc: ExceptionState::default(),
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

/// A registered command. `External { table_index, client_data }` — extension
/// commands registered through `Tcl_CreateObjCommand` — is the one variant the
/// C-extension ABI still wants; see `docs/design/runtime/c-extension-abi.md`
/// §13.
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

#[derive(Clone)]
pub enum Command {
    /// A native Rust handler.
    Builtin(BuiltinFn),
    /// An `interp alias`: dispatch re-resolves `target` **by name, anchored at
    /// the global namespace, on every call** (so it lazily observes the target's
    /// *deletion* but does NOT follow its *rename* — the stored name simply stops
    /// resolving), then prepends the frozen `prefix` words to the caller's args.
    /// See `docs/design/runtime/rename-alias.md` §4.
    Alias {
        target: Vec<u8>,
        prefix: Vec<Vec<u8>>,
        /// Per-instance identity — see [`Command::is_same_binding`].
        identity: Rc<()>,
    },
    /// A `namespace import` redirect. Ordinary imports re-resolve `source` by
    /// name; an ensemble import additionally retains the source's stable command
    /// token. The latter is what lets an import follow a source ensemble through
    /// hide/expose without accidentally switching to a replacement installed at
    /// the vacated name.
    Imported {
        source: Vec<u8>,
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
    Proc(Rc<ProcDef>),
    /// A child interpreter, addressable as a command (`$child eval …`). The
    /// `Vec<u8>` is the child's name; dispatch routes the subcommand to the child
    /// `Interp` stored in [`Interp::children`].
    ChildInterp(Vec<u8>),
    /// A TclOO object or class, addressable as a command (`$obj method …`,
    /// `Class new`). The `Vec<u8>` is the FQN; dispatch routes to
    /// [`crate::cmd_oo`] via the [`OoState`](crate::cmd_oo::OoState) registry.
    OoObject(Vec<u8>),
    /// A cross-interp alias installed in a *child* interp that delegates to a
    /// command in the *parent* (`interp alias child name {} parentCmd …`). When
    /// invoked, it runs `target` (+ `prefix` + the call args) in the parent.
    ParentAlias {
        target: Vec<u8>,
        prefix: Vec<Vec<u8>>,
        /// Per-instance identity — see [`Command::is_same_binding`].
        identity: Rc<()>,
    },
}

impl Command {
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
            (Self::Proc(a), Self::Proc(b)) => Rc::ptr_eq(a, b),
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
            (Self::ChildInterp(a), Self::ChildInterp(b)) => a == b,
            (Self::OoObject(a), Self::OoObject(b)) => a == b,
            _ => false,
        }
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
pub struct Param {
    pub name: Vec<u8>,
    pub default: Option<Vec<u8>>,
}

/// A compiled `proc` definition: its parameters, body script, and the namespace
/// it was defined in (which becomes the current namespace while it runs).
#[derive(Clone)]
pub struct ProcDef {
    pub params: Vec<Param>,
    pub body: Vec<u8>,
    pub ns: NsId,
    /// The proc's fully-qualified name (`::ns::name`) — the `info frame` `proc`
    /// key, fixed at definition time.
    pub fqn: Vec<u8>,
    /// The file the proc was defined in (`source`d), if any — makes its body
    /// frame `type source` with this `file` (`info frame`).
    pub source: Option<Rc<[u8]>>,
    /// The line base for the body's `info frame` lines: `0` (body-relative) for
    /// an eval-defined proc, or the defining file line minus one for one defined
    /// in a `source`d file (so its commands report file-absolute lines).
    pub body_line_base: u32,
}

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
/// `Commands::dispatch_id`: a bijection between a command's FQN and a dense raw
/// `CommandId` (the index into `fqns`). Minted on first `find_command`.
#[derive(Default)]
struct CmdArena {
    ids: std::collections::HashMap<Vec<u8>, u32>,
    fqns: Vec<Vec<u8>>,
}

pub struct InterpState {
    pub(crate) frames: RefCell<FrameStack>,
    /// The command-table-as-core-service: the namespace tree + the one
    /// `resolve(currentNs, name)` resolver (T1.5).
    namespaces: RefCell<Namespaces>,
    /// Runtime-issued speculative guards and explicitly attested builtin IDs.
    guards: RefCell<GuardManager>,
    guarded_commands:
        RefCell<std::collections::BTreeMap<Vec<u8>, std::collections::BTreeSet<GuardIdentity>>>,
    /// The current namespace for command resolution (the eval context; a proc
    /// runs in its *defining* namespace — wired with procs). Global at top level.
    current_ns: Cell<NsId>,
    /// Active proc-call nesting depth — C Tcl's `interp recursionlimit`. Bounds
    /// recursion so an infinite proc loop raises a catchable error instead of
    /// overflowing the (wasm) stack (the tracked PR #557 follow-up).
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
    /// Variable-trace registry (`trace add|remove|info variable`).
    pub(crate) traces: RefCell<crate::cmd_trace::TraceTable>,
    /// The error stack-trace accumulator (PC-4).
    exc: RefCell<ExceptionState>,
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
    hidden: RefCell<std::collections::BTreeMap<Vec<u8>, Command>>,
    /// While this interp runs as a child (`$parent eval`/`$child eval`), a `Weak`
    /// handle to its parent — for cross-interp aliases that delegate to a parent
    /// command. `Weak` (not `Rc`) so the parent→child ownership has no cycle;
    /// upgraded for the call's duration. Set on entry to a child eval
    /// ([`eval_in_child`]/[`with_child`]) and restored after.
    ///
    /// [`eval_in_child`]: Interp::eval_in_child
    /// [`with_child`]: Interp::with_child
    parent: RefCell<Weak<InterpState>>,
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
    /// Set when a delete was requested while [`eval_active`](Self::eval_active)
    /// was non-zero; the actual removal from the parent's table is deferred until
    /// the last eval unwinds (C's deferred `Tcl_DeleteInterp`).
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
    pub(crate) oo: RefCell<crate::cmd_oo::OoState>,
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
    bgerror: RefCell<Vec<u8>>,
    /// Queued background errors `(message, options)`, drained by `update` (Tcl
    /// defers them to the event loop rather than firing at the error site).
    bg_queue: RefCell<Vec<(Vec<u8>, Vec<u8>)>>,
    /// The event loop's pending timer + idle events (`after`/`vwait`/`update`).
    events: RefCell<crate::cmd_event::EventQueue>,
    /// Live coroutines (`coroutine`/`yield`), keyed by command name. Each holds
    /// the coroutine's saved execution context (swapped in/out on resume/yield)
    /// and the handoff channels to its worker thread (`cmd_coro`).
    coros: RefCell<std::collections::BTreeMap<Vec<u8>, crate::cmd_coro::CoroEntry>>,
    /// The active ensemble-rewrite, if any (C's `iPtr->ensembleRewrite`): the
    /// original command words a forward / ensemble / constructor dispatch
    /// replaced, so a downstream `wrong # args` can report the call as the user
    /// wrote it. `removed` is how many leading words of `source` map to the
    /// rewritten prefix. Set at the root dispatch, cleared when it returns.
    ensemble_rewrite: RefCell<Option<EnsembleRewrite>>,
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
    error_stack: RefCell<Vec<Vec<u8>>>,
    /// C's `iPtr->resetErrorStack`: set when the result is reset (a new error
    /// episode is starting), so the next command logged clears the stack and
    /// records its inner context. Starts `true`.
    reset_error_stack: Cell<bool>,
    /// The `try` exception-chaining link (TIP 329 `-during`): when a `try`
    /// handler or `finally` script throws, the options dict of the *prior*
    /// exception it superseded is stashed here so the next error-options build
    /// ([`build_options`](crate::cmd_error)) splices it in as `-during`. Holds an
    /// owning reference (released when overwritten, cleared, or the interp drops).
    /// Cleared when an error is published/caught ([`publish_error`](Self::publish_error)),
    /// since the chain is then consumed.
    during: Cell<Option<*mut TclObj>>,
    result: Cell<*mut TclObj>,
    /// Command-FQN ⇆ dense raw `CommandId` arena for `Namespaces::find_command`
    /// and `Commands::dispatch_id`. Interior-mutable because `find_command` is
    /// `&self` but mints a handle on first sight; `state_traits.rs` wraps the raw id
    /// in the contract's `CommandId`. Bidirectional: `find_command` interns an
    /// FQN, `dispatch_id` reverses the id back to its FQN to invoke it.
    cmd_arena: RefCell<CmdArena>,
    /// `interp limit` configuration. The `time` limit is enforced by the loop
    /// commands; `commands` is stored for query/set only.
    limits: RefCell<LimitSet>,
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
    /// The dialect profile this interpreter validates its builtin command
    /// surface against (issue #1463) and derives its lexing grammar from
    /// (issue #1462). Defaults to the permissive fallback profile, which
    /// hides nothing and lexes with the modern grammar;
    /// [`Interp::set_runtime_version`] pins the matching plain-Tcl profile.
    dialect_profile: Cell<&'static tcl_dialect::DialectProfile>,
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
    /// The `Command::OoObject` entries the engine installs on the registry's
    /// behalf (the TclOO roots `::oo::object`, `::oo::class`,
    /// `::oo::configurable`, `::oo::abstract`, `::oo::singleton`) rather than
    /// a script creating them. They carry the registry's release gate the way
    /// a builtin does; every other object command is user-created and
    /// release-invariant. Filled at bootstrap (`cmd_oo::install`), read by
    /// [`Interp::resolve_dispatchable`].
    registry_object_roots: RefCell<std::collections::HashSet<Vec<u8>>>,
}

/// An ensemble-rewrite record (C's `iPtr->ensembleRewrite`, see
/// `InterpState::ensemble_rewrite`). `Tcl_WrongNumArgs` prints the first
/// `removed` words of `source` in place of the `inserted` leading words of the
/// actual (rewritten) call.
#[derive(Clone)]
pub(crate) struct EnsembleRewrite {
    /// The original command words as the user wrote them (e.g. `foo test 1 2 3`),
    /// with the subcommand spell-fixed to its resolved name.
    pub source: Vec<Vec<u8>>,
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

/// A native-stack safety net over **every** script-body evaluation —
/// control-flow bodies (`if`/`while`/`for`/`foreach`/…), proc bodies,
/// `eval`/`uplevel`/`source`, and command substitution — checked against
/// [`Interp::eval_depth`] in [`Interp::eval_script_mode`], independently of
/// [`RECURSION_LIMIT`]/[`Interp::recursion_limit`] (issue #996).
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
/// class of ambient stack budget that made issue #996 reproducible for the
/// analyser): unguarded nested `foreach` bodies overflow the stack (SIGABRT)
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
fn dict_obj(pairs: &[(&[u8], Vec<u8>)]) -> *mut TclObj {
    let mut elems: Vec<*mut TclObj> = Vec::with_capacity(pairs.len() * 2);
    for (k, v) in pairs {
        elems.push(obj::new_string_bytes(k));
        elems.push(obj::new_string_bytes(v));
    }
    let list = crate::list::new_list_obj(&elems);
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
/// This carried #1443's bug verbatim, in both halves: the hand-rolled
/// `starts_with` filter could only ever say `bad option`, so the empty word —
/// a prefix of *every* option — reported `bad option ""` where C reports
/// `ambiguous option ""`; and the `", or"` enumeration was hand-built beside
/// `prefix::choice_list_bytes`, which owns it. `OptionTable::abbreviating`
/// now supplies both.
fn resolve_limit_opt(arg: &[u8], opts: &[&[u8]]) -> Result<Vec<u8>, Vec<u8>> {
    let table = tcl_cmd_core::prefix::OptionTable::abbreviating("option", opts);
    match table.index_of(arg) {
        Ok(i) => Ok(opts[i].to_vec()),
        Err(m) => Err(m),
    }
}

/// Validate an `interp debug` option: it must be a non-empty prefix of `-frame`.
fn check_debug_opt(opt: *mut TclObj) -> Result<(), Vec<u8>> {
    let o = obj_bytes(opt);
    if !o.is_empty() && b"-frame".starts_with(o.as_slice()) {
        return Ok(());
    }
    let mut m = b"bad debug option \"".to_vec();
    m.extend_from_slice(&o);
    m.extend_from_slice(b"\": must be -frame");
    Err(m)
}

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

/// Install `version`'s numeric-literal grammar as this thread's ambient one, so
/// every numeral this runtime reads (`expr`, `format`, `dict`, `incr`, the
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
fn install_number_syntax(version: tcl_dialect::TclVersion) {
    tcl_syntax::number::set_runtime_syntax(version.number_syntax());
}

fn default_host() -> Rc<dyn tcl_platform::Host> {
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
        let result = obj::new_obj();
        // SAFETY: `result` is freshly created; the interp takes the owning ref.
        unsafe { obj::incr_ref_count(result) };
        let mut guards = GuardManager::default();
        // Object dispatch still has mutation sites spread through the TclOO
        // engine. Fail closed until those sites have one mutation owner. The
        // interpreter-policy surface is centralised on
        // `invalidate_interpreter_policy` below and can issue live guards.
        guards.poison(GuardDomain::ObjectDispatch);
        let mut interp = Interp(Rc::new(InterpState {
            frames: RefCell::new(FrameStack::new()),
            namespaces: RefCell::new(Namespaces::new()),
            guards: RefCell::new(guards),
            guarded_commands: RefCell::new(std::collections::BTreeMap::new()),
            current_ns: Cell::new(GLOBAL),
            recursion_depth: Cell::new(0),
            recursion_limit: Cell::new(RECURSION_LIMIT),
            packages: RefCell::new(crate::cmd_package::PackageState::with_core(
                DEFAULT_RUNTIME_VERSION,
            )),
            script_stack: RefCell::new(Vec::new()),
            channels: RefCell::new(crate::cmd_chan::ChannelTable::default()),
            return_code: Cell::new(Code::Ok),
            return_level: Cell::new(1),
            traces: RefCell::new(crate::cmd_trace::TraceTable::default()),
            exc: RefCell::new(ExceptionState::default()),
            error_line: Cell::new(1),
            children: RefCell::new(std::collections::BTreeMap::new()),
            interp_counter: Cell::new(0),
            hidden: RefCell::new(std::collections::BTreeMap::new()),
            parent: RefCell::new(Weak::new()),
            is_safe: Cell::new(false),
            eval_active: Cell::new(0),
            pending_delete: Cell::new(false),
            host: RefCell::new(host),
            oo: RefCell::new(crate::cmd_oo::OoState::default()),
            cmd_frames: RefCell::new(Vec::new()),
            arg_lines: RefCell::new(Vec::new()),
            arg_locs: RefCell::new(Vec::new()),
            eval_depth: Cell::new(0),
            var_trace_epoch: Cell::new(1),
            cmd_count: Cell::new(0),
            exit_code: Cell::new(None),
            measure_overhead: Cell::new(0.0),
            bgerror: RefCell::new(Vec::new()),
            bg_queue: RefCell::new(Vec::new()),
            events: RefCell::new(crate::cmd_event::EventQueue::default()),
            coros: RefCell::new(std::collections::BTreeMap::new()),
            ensemble_rewrite: RefCell::new(None),
            #[cfg(have_tommath)]
            rand_seed: Cell::new(None),
            error_stack: RefCell::new(Vec::new()),
            reset_error_stack: Cell::new(true),
            during: Cell::new(None),
            result: Cell::new(result),
            cmd_arena: RefCell::new(CmdArena::default()),
            limits: RefCell::new(LimitSet::default()),
            #[cfg(have_tommath)]
            limit_tick: Cell::new(0),
            debug_frame: Cell::new(false),
            runtime_version: Cell::new(DEFAULT_RUNTIME_VERSION),
            // The "no dialect pinned" ingress: the lenient environment,
            // whose unit profile is the permissive fallback that hides
            // nothing. `set_dialect_profile` replaces all three together.
            dialect_profile: Cell::new(crate::environment::profile_for_dialect("")),
            profile_registry: Cell::new(None),
            dialect_point: Cell::new(Some(crate::environment::surface_point(
                crate::environment::profile_for_dialect(""),
            ))),
            registry_object_roots: RefCell::new(std::collections::HashSet::new()),
        }));
        // The numeric grammar is thread-ambient and may have been left on
        // another release by an interpreter built earlier on this thread, so a
        // fresh interp installs its own rather than inheriting whatever is
        // there (`set_runtime_version` re-installs when an embedder repins).
        install_number_syntax(DEFAULT_RUNTIME_VERSION);
        builtins::install(&mut interp);
        // C sets `tcl_version`/`tcl_patchLevel` in `Tcl_CreateInterp`
        // (9.0.4 `generic/tclBasic.c:1346-1347`), **not** in `Tcl_Init` — so
        // they exist in an interpreter that never sources `init.tcl`.
        // Mirroring that placement is what lets `info patchlevel` answer
        // without `--init`. `set_startup_globals` installs that pair together
        // with the rest of Tcl_CreateInterp's predefined surface.
        interp.set_startup_globals();
        interp
    }

    // -- capability host ------------------------------------------------------

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
        *self.0.host.borrow_mut() = host;
        let mut interp = self.clone();
        interp.rebootstrap_host_globals();
        if interp.is_safe.get() {
            interp.scrub_host_globals_for_safe();
        }
    }

    // -- emulated Tcl release -------------------------------------------------

    /// Pin the Tcl release this interpreter emulates.
    ///
    /// Mirrors [`tcl_vm::Vm::set_runtime_version`]'s contract for the
    /// tree-walking runtime: every release-dependent *semantic* is derived
    /// from this one value rather than being set independently, so the two
    /// engines cannot drift apart by having one of them updated and not the
    /// other (issue #1328). Today that is the numeric-literal grammar (see
    /// [`install_number_syntax`]), the namespace-scope variable fallback
    /// (TIP 278), and the release-reporting globals.
    ///
    /// The fallback is a property of the **namespace table**, which every
    /// interpreter owns privately, so a child (`interp create`) and a safe
    /// interpreter each resolve against their own global namespace — setting
    /// it here never reaches across an interpreter boundary.
    pub fn set_runtime_version(&mut self, version: tcl_dialect::TclVersion) {
        // A bare release pin is the matching plain-Tcl profile: the emulated
        // release is one fact carrying the runtime semantics, the lexing
        // grammar (issue #1462), and the command-surface availability mask
        // (issue #1463). The release name is a dialect *name*, so it
        // resolves through the one ingress seam (`crate::environment`)
        // rather than through `by_name`.
        self.set_dialect_profile(crate::environment::profile_for_dialect(
            version.dialect_name(),
        ));
    }

    /// Pin the dialect profile this interpreter emulates — the profile form
    /// of [`Self::set_runtime_version`], for hosts whose dialect is a vendor
    /// profile rather than a plain Tcl release. The runtime version follows
    /// the profile's pinned `vm_runtime_version`, scripts are lexed with the
    /// profile's grammar, and the profile's availability mask becomes the
    /// builtin command-surface filter.
    pub fn set_dialect_profile(&mut self, profile: &'static tcl_dialect::DialectProfile) {
        let version = profile.vm_runtime_version;
        // Ahead of the unchanged-profile short-circuit: the numeric grammar is
        // *thread*-ambient, not per-interp, so "this interp already emulates
        // `version`" does not imply the thread's grammar is this interp's. A
        // second interpreter constructed on a thread where an earlier one
        // installed 8.4 must re-install its own release even when its version
        // field needs no change.
        install_number_syntax(version);
        if std::ptr::eq(self.dialect_profile(), profile) {
            return;
        }
        self.invalidate_interpreter_policy();
        self.invalidate_command_environment();
        self.0.dialect_profile.set(profile);
        self.0
            .profile_registry
            .set((!profile.is_fallback()).then(|| crate::environment::store_for_profile(profile)));
        self.0
            .dialect_point
            .set(Some(crate::environment::surface_point(profile)));
        self.0.runtime_version.set(version);
        self.namespaces.borrow_mut().ns_var_global_fallback =
            version.namespace_var_global_fallback();
        self.write_release_globals();
        // `package provide Tcl` is a release fact, not a runtime constant, and
        // the pre-provided entries were written against the *previous* pin —
        // re-derive them (ledger row B4).
        self.packages.borrow_mut().provide_core(version);
    }

    /// The Tcl release this interpreter emulates (see
    /// [`Self::set_runtime_version`]).
    #[must_use]
    pub fn runtime_version(&self) -> tcl_dialect::TclVersion {
        self.0.runtime_version.get()
    }

    /// The dialect profile this interpreter validates its command surface
    /// against (see [`Self::set_dialect_profile`]).
    #[must_use]
    pub fn dialect_profile(&self) -> &'static tcl_dialect::DialectProfile {
        self.0.dialect_profile.get()
    }

    /// The lexer configuration scripts evaluate under: the pinned profile's
    /// grammar (issue #1462) — `{*}` expansion off and the first-close `${…}`
    /// rule on when the interpreter emulates Tcl 8.4.
    pub(crate) fn lexer_config(&self) -> tcl_lexer::LexerConfig {
        tcl_lexer::LexerConfig::from_grammar(self.dialect_profile().grammar)
    }

    /// Whether a builtin command is exposed on this interpreter's selected
    /// runtime surface. The registry recognises versioned builtin entries;
    /// unrecognised names remain available for user-defined commands.
    ///
    /// Two registry-backed cases (issue #1463): the math-function surface
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

    /// Record `fqn` as an engine-installed TclOO root object command, so the
    /// release-availability gate treats it like a builtin (see
    /// [`InterpState::registry_object_roots`]).
    pub(crate) fn declare_registry_object_root(&self, fqn: &[u8]) {
        self.0
            .registry_object_roots
            .borrow_mut()
            .insert(fqn.to_vec());
    }

    /// Drop `fqn`'s engine-installed root marking. The marking is an identity,
    /// not a reservation on the *name*: once a script creates its own object
    /// under that name the entry is release-invariant like any proc, so the
    /// marking must not outlive the entry it described.
    pub(crate) fn forget_registry_object_root(&self, fqn: &[u8]) {
        self.0.registry_object_roots.borrow_mut().remove(fqn);
    }

    /// Whether `fqn` is an engine-installed TclOO root that this release does
    /// **not** have (e.g. `::oo::configurable` on an 8.6 surface). Such a root
    /// is invisible to every dispatch and enumeration path, so for anything
    /// that asks "is this name taken?" it must read as free — real tclsh 8.6
    /// has no `::oo::configurable`, and a script may define one.
    pub(crate) fn is_gate_hidden_object_root(&self, fqn: &[u8]) -> bool {
        self.0.registry_object_roots.borrow().contains(fqn)
            && !self.builtin_command_visible_for_surface(fqn)
    }

    /// The availability half of [`Self::builtin_command_visible_for_surface`]:
    /// whether the pinned profile admits registry builtin `name`. Names the
    /// registry does not know are always admitted — they may be engine
    /// extensions the registry has no spec for.
    fn profile_admits_registry_builtin(&self, name: &str) -> bool {
        let Some(registry) = self.0.profile_registry.get() else {
            return true; // the permissive fallback profile gates nothing
        };
        registry.get(name).is_none()
            || registry
                .get_for_surface(name, self.0.dialect_point.get())
                .is_some()
    }

    /// Resolve `name` (from namespace `origin`) to a command handle, applying
    /// the availability gate of [`Self::builtin_command_visible_for_surface`]
    /// to the **final** resolved builtin identity: a builtin the emulated
    /// release does not carry resolves to `None`, exactly as if no command of
    /// that name existed.
    ///
    /// This is the single owner of the gate on the dispatch side (PR #1481
    /// review of issues #1462/#1463). The gate was originally spelled out at
    /// the direct-dispatch call site only, so the two other resolve-then-
    /// [`Self::invoke`] shapes — the alias trampoline
    /// ([`Self::dispatch_alias`]) and the `namespace import` redirect
    /// ([`Command::Imported`]) — reached the builtin behind an ungated second
    /// resolution, making a release-hidden builtin callable through an alias
    /// or an imported spelling. Every name→`Command` step that feeds `invoke`
    /// now goes through here, so a new dispatch path cannot silently reopen
    /// the hole.
    ///
    /// A gated miss is deliberately indistinguishable from a deleted command:
    /// each caller then reports the miss the way it already reports a target
    /// that genuinely does not exist (`invalid command name "<target>"`,
    /// naming the resolved target rather than the alias — matching real tclsh
    /// 8.6/9.0 for `interp alias {} la {} nosuchcmd; la`), which is precisely
    /// the "this release does not have that command" contract of #1462.
    pub(crate) fn resolve_dispatchable(&self, origin: NsId, name: &[u8]) -> Option<Command> {
        let (cmd, fqn) = {
            let ns = self.namespaces.borrow();
            let cmd = ns.resolve(origin, name)?;
            // Only a *directly* bound builtin carries a release identity. Procs,
            // aliases and ensembles are script-created, and a nested redirect
            // (import/alias) is gated when it resolves in its turn. The one
            // object exception is the TclOO **roots** the engine installs on
            // the registry's behalf (`::oo::class` and friends, dated
            // TCL86_PLUS / TCL90_PLUS): those must vanish with the release the
            // way a builtin does, while every script-created object command
            // stays release-invariant.
            let gated = match &cmd {
                Command::Builtin(_) => true,
                Command::OoObject(fqn) => self.0.registry_object_roots.borrow().contains(fqn),
                _ => false,
            };
            if !gated {
                return Some(cmd);
            }
            let fqn = ns.resolve_fqn(origin, name)?;
            (cmd, fqn)
        };
        self.builtin_command_visible_for_surface(&fqn)
            .then_some(cmd)
    }

    /// Convert `obj` to the byte view consumed by Tcl's `binary` command.
    ///
    /// Byte-array objects retain their own raw payload. Ordinary string objects
    /// are converted through the release profile instead: Tcl 8 truncates a
    /// wide code point to one byte, while Tcl 9 rejects it. Keeping this at the
    /// interpreter boundary makes every `binary` subcommand use the same
    /// dual-representation and version rule.
    pub(crate) fn binary_bytes(&mut self, obj: *mut TclObj) -> Result<Vec<u8>, Code> {
        crate::bytearray::binary_bytes(obj, self.runtime_version().byte_string_encoding()).map_err(
            |err| {
                let message = format!(
                    "expected code point values below 0xff but value at byte offset {} was 0x{:x}",
                    err.byte_offset, err.code_point
                );
                self.error_with_code(message.as_bytes(), b"TCL VALUE BYTES")
            },
        )
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
        let code = crate::cmd_mathfunc::mathfunc(self, &argv);
        release_all(&argv);
        code
    }

    /// Write the release-reporting globals (`tcl_version` / `tcl_patchLevel`)
    /// for the currently-emulated release.  Called at interpreter creation
    /// (C does this in `Tcl_CreateInterp`) and again whenever
    /// [`Self::set_runtime_version`] changes the answer.
    pub(crate) fn write_release_globals(&mut self) {
        let version = self.runtime_version();
        for (name, val) in [
            (&b"::tcl_version"[..], version.version_string()),
            (b"::tcl_patchLevel", version.patchlevel()),
        ] {
            let o = new_string(val.as_bytes());
            if self.var_set(name, o).is_err() {
                drop_fresh(o);
            }
        }
    }

    // -- command registry -----------------------------------------------------

    /// Register a built-in command (a possibly-qualified `name`, creating
    /// intermediate namespaces; overwrites any existing command of `name`).
    pub fn register_builtin(&mut self, name: &[u8], f: BuiltinFn) {
        self.namespaces
            .borrow_mut()
            .register(name, Command::Builtin(f));
        self.invalidate_command_environment();
    }

    /// Register a builtin with a stable semantic identity understood by
    /// offline-generated code. Ordinary registration never infers identity
    /// from a spelling or function address.
    pub fn register_guarded_builtin(&mut self, name: &[u8], f: BuiltinFn, identity: GuardIdentity) {
        self.register_builtin(name, f);
        if let Some(fqn) = self.namespaces.borrow().resolve_fqn(GLOBAL, name) {
            self.guarded_commands
                .borrow_mut()
                .entry(fqn)
                .or_default()
                .insert(identity);
        }
    }

    /// Register a builtin and derive every semantic identity from its registry
    /// command, subcommand, and invocation-form descriptors.
    pub fn register_spec_builtin(&mut self, spec: &tcl_registry::CommandSpec, f: BuiltinFn) {
        self.register_builtin(spec.name.as_bytes(), f);
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
            if let Some(fqn) = self
                .namespaces
                .borrow()
                .resolve_fqn(GLOBAL, spec.name.as_bytes())
            {
                self.guarded_commands.borrow_mut().insert(fqn, identities);
            }
        }
    }

    /// Verify live command identity and issue a guard over `domains`.
    pub fn prepare_command_guard(
        &self,
        name: &[u8],
        expected: GuardIdentity,
        domains: GuardDomains,
    ) -> Result<GuardToken, GuardError> {
        let traces = self.traces.borrow();
        if (domains.contains(GuardDomain::CommandTrace) && !traces.cmd_traces.is_empty())
            || (domains.contains(GuardDomain::VariableTrace) && !traces.traces.is_empty())
        {
            return Err(GuardError::PrerequisiteUnsatisfied);
        }
        drop(traces);
        let observed = self
            .namespaces
            .borrow()
            .resolve_fqn(self.current_ns.get(), name)
            .and_then(|fqn| {
                let identities = self.guarded_commands.borrow();
                let identities = identities.get(&fqn)?;
                Some(if identities.contains(&expected) {
                    expected
                } else {
                    *identities.first()?
                })
            });
        self.guards
            .borrow_mut()
            .prepare(expected, observed, domains)
    }

    /// Re-check a guard against the current resolved implementation identity.
    #[must_use]
    pub fn check_command_guard(&self, token: GuardToken, name: &[u8]) -> bool {
        let Some(fqn) = self
            .namespaces
            .borrow()
            .resolve_fqn(self.current_ns.get(), name)
        else {
            return false;
        };
        let identities = self.guarded_commands.borrow();
        let Some(identities) = identities.get(&fqn) else {
            return false;
        };
        identities
            .iter()
            .any(|identity| self.guards.borrow().check(token, Some(*identity)))
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
        let Some(fqn) = self
            .namespaces
            .borrow()
            .resolve_fqn(self.current_ns.get(), name)
        else {
            return false;
        };
        let identities = self.guarded_commands.borrow();
        if !identities
            .get(&fqn)
            .is_some_and(|identities| identities.contains(&expected))
        {
            return false;
        }
        self.guards
            .borrow()
            .check_expected(token, expected, Some(expected))
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
                let result = tcl_cmd_core::string::length(self, value);
                self.set_result(result);
                Some(Code::Ok)
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

    fn invalidate_command_environment(&self) {
        self.guarded_commands.borrow_mut().clear();
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
    /// namespace. Drives the one command table; see [`Namespaces::rename`].
    pub(crate) fn rename_command(&mut self, old: &[u8], new: &[u8]) -> RenameOutcome {
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
        // observable happens, same as the alias-loop guard above (issue
        // #1412 item 1). A release-gated TclOO root this build hides reads
        // as free here too (`is_gate_hidden_object_root`), same as every
        // other "is this name taken?" check.
        if let Some(occupant_fqn) = self
            .namespaces
            .borrow_mut()
            .destination_occupant_fqn(self.current_ns.get(), new)
        {
            if !self.is_gate_hidden_object_root(&occupant_fqn) {
                return RenameOutcome::TargetExists;
            }
        }
        // A builtin the emulated release does not carry is not there to be
        // renamed or deleted (#1462/#1463): rebinding it under a name the
        // registry has no spec for would hand it back ungated, defeating the
        // availability mask outright. Checked before any observable effect,
        // like the alias-loop refusal above.
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
        let (ensemble_token, import_token) =
            match self.namespaces.borrow().resolve(self.current_ns.get(), old) {
                Some(Command::Ensemble(token)) => (Some(token), None),
                Some(Command::Imported { identity, .. }) => (None, Some(identity)),
                _ => (None, None),
            };
        self.invalidate_command_environment();
        // Command traces fire *before* the table mutation (C's TclRenameCommand:
        // the command still exists under its old name during the callback), with
        // the fully-qualified old and new names. C deletes the command *token*
        // it captured here, not whatever the name holds when the callback
        // returns — so the binding is captured alongside the name.
        let bound_before = self.namespaces.borrow().resolve(self.current_ns.get(), old);
        // The token whose trace list this deletion frees, captured before the
        // callbacks can bind a replacement at the same name.
        let dying_token = self.resolve_cmd_token(old);
        let old_fqn = self.resolve_cmd_fqn(old);
        if let Some(of) = &old_fqn {
            if !self.traces.borrow().cmd_traces.is_empty() {
                if new.is_empty() {
                    self.fire_cmd_trace(of, b"", crate::cmd_trace::ops::DELETE);
                } else {
                    let nf = self.fqn_for(new);
                    self.fire_cmd_trace(of, &nf, crate::cmd_trace::ops::RENAME);
                }
            }
        }
        let existed = old_fqn.is_some();
        // Deleting a suspended coroutine's command tears down its worker first.
        if new.is_empty() {
            if let Some(of) = &old_fqn {
                crate::cmd_coro::on_command_deleted(self, of);
            }
        }
        // `Namespaces::rename` moves the table entry directly rather than going
        // through `ns_register`, so clear the destination's TclOO root marking
        // here too. Today an OO rename is also re-registered through the funnel
        // by `oo_command_renamed`, which clears it; doing it here as well makes
        // the invariant — "a root marking never outlives the entry it
        // described" — hold for the rename path on its own, rather than by way
        // of a follow-up call that a future refactor could reorder or drop.
        if !new.is_empty() {
            let dest = self.fqn_for(new);
            self.forget_registry_object_root(&dest);
        }
        // An imported command carries a stable identity. Its delete trace may
        // force-reimport or otherwise replace the same binding; delete the
        // captured old identity wherever it moved, never the callback's fresh
        // command at the old name.
        let removed_import_fqn = if new.is_empty() {
            import_token
                .as_ref()
                .and_then(|identity| self.remove_import_identity(identity))
        } else {
            None
        };
        // A delete-trace callback that re-creates the command (`proc foo {} …`)
        // has bound a *new* command at the old name. C's captured token is
        // `CMD_DYING` and no longer owns the hash entry, so its deletion leaves
        // the fresh command standing (`Tcl_DeleteCommandFromToken`,
        // tclBasic.c) — `foo` still exists, and calls the new body. Deleting
        // "whatever is at the name now" would remove the callback's work
        // instead. This is the command half of the rule the import branch just
        // above already applies to its own identity. Issue #1633.
        let recreated = new.is_empty()
            && match (
                &bound_before,
                self.namespaces.borrow().resolve(self.current_ns.get(), old),
            ) {
                (Some(before), Some(now)) => !before.is_same_binding(&now),
                _ => false,
            };
        let raw = if recreated {
            RenameOutcome::Deleted
        } else if new.is_empty() && import_token.is_some() {
            if removed_import_fqn.is_some() {
                RenameOutcome::Deleted
            } else {
                RenameOutcome::NoSuchCommand
            }
        } else {
            self.namespaces
                .borrow_mut()
                .rename(self.current_ns.get(), old, new)
        };
        // A delete-trace callback may itself delete the command (e.g. by
        // deleting the object's namespace). C captured the command token before
        // the callback, so the deletion still succeeds — treat "existed at
        // entry, gone now, `new` empty" as a normal delete (cleanup is
        // idempotent) rather than reporting "command doesn't exist".
        let outcome = if existed && matches!(raw, RenameOutcome::NoSuchCommand) && new.is_empty() {
            RenameOutcome::Deleted
        } else {
            raw
        };
        if let Some(of) = old_fqn {
            let oo_live = !self.oo_is_empty();
            match outcome {
                // The trace list (and any OO object) follows to the new name,
                // and so does every `namespace import` redirect of the old
                // name — C's imports hold the source's command token, so they
                // survive a source rename (tclsh-pinned; see
                // `Namespaces::retarget_imports`).
                RenameOutcome::Renamed => {
                    let nf = self.fqn_for(new);
                    self.move_cmd_traces(&of, &nf);
                    self.retarget_import_sources(&of, &nf);
                    if oo_live {
                        self.oo_command_renamed(&of, Some(&nf));
                    }
                }
                // The command is gone; the dying token's traces and OO
                // registry entry go with it. A replacement the delete callback
                // bound at the same name keeps its own traces (C frees only
                // `cmdPtr->tracePtr`).
                RenameOutcome::Deleted => {
                    self.remove_cmd_traces_of_token(&of, dying_token);
                    let tokens: Vec<_> = ensemble_token.into_iter().collect();
                    let mut origins = vec![of.clone()];
                    if let Some(removed_fqn) = removed_import_fqn {
                        if removed_fqn != of {
                            self.remove_cmd_traces(&removed_fqn);
                            origins.push(removed_fqn);
                        }
                    }
                    self.remove_imports_for_deleted_origins(origins, &tokens);
                    if oo_live {
                        self.oo_command_renamed(&of, None);
                    }
                }
                // `AliasLoop` and `TargetExists` both returned above, before
                // the table was touched.
                RenameOutcome::NoSuchCommand
                | RenameOutcome::AliasLoop
                | RenameOutcome::TargetExists => {}
            }
        }
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
        self.invalidate_command_environment();
        let mut namespaces = self.namespaces.borrow_mut();
        let Some((ns, simple)) = namespaces.register_at(
            name,
            Command::Alias {
                target,
                prefix,
                identity: Rc::new(()),
            },
        ) else {
            return Ok(()); // no tail to bind — nothing was registered
        };
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
        self.with_child(child, |c| {
            c.ns_register(
                name,
                Command::ParentAlias {
                    target,
                    prefix,
                    identity: Rc::new(()),
                },
            );
        })
        .is_some()
    }

    /// The `(target, prefix)` of the alias bound to `name` (the query form), or
    /// `None` if `name` resolves to something that isn't an alias.
    pub(crate) fn alias_info(&self, name: &[u8]) -> Option<(Vec<u8>, Vec<Vec<u8>>)> {
        match self
            .namespaces
            .borrow()
            .resolve(self.current_ns.get(), name)
        {
            Some(Command::Alias { target, prefix, .. }) => Some((target, prefix)),
            _ => None,
        }
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
        self.invalidate_command_environment();
        // If `name` is a suspended coroutine, terminate its worker first.
        crate::cmd_coro::on_command_deleted(self, name);
        let source_fqn = self.resolve_cmd_fqn(name);
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
        if deleted {
            if let Some(source_fqn) = source_fqn {
                let tokens: Vec<_> = ensemble_token.into_iter().collect();
                self.remove_imports_for_deleted_origins([source_fqn], &tokens);
            }
        }
        deleted
    }

    /// Register an ensemble command (`namespace ensemble create`); `name` is the
    /// ensemble command (possibly qualified — rooted at global like any builtin).
    pub(crate) fn create_ensemble(&mut self, name: &[u8], cfg: crate::ensemble::EnsembleConfig) {
        self.invalidate_command_environment();
        let fqn = self.fqn_for(name);
        // The `-command` name resolves relative to the current namespace, like a
        // proc name (C's `TclGetNamespaceForQualName(name, cxtPtr=nsPtr, ...)` in
        // `NamespaceEnsembleCmd`). `namespace ensemble create -command path`
        // inside `namespace eval ::tcl::tm` therefore binds `::tcl::tm::path`,
        // not a bare `::path` at global scope.
        let ns = self
            .namespaces
            .borrow_mut()
            .command_home_ns(self.current_ns.get(), name);
        // Written-name tail: empty for a trailing separator run (the `{}`
        // command, #934) — must match `home_of`'s resolution split.
        let tail = tcl_syntax::naming::written_command_tail(name).to_vec();
        let old_token = match self.namespaces.borrow().command_in(ns, &tail) {
            Some(Command::Ensemble(token)) => Some(token),
            _ => None,
        };
        let new_token = Rc::new(crate::ensemble::EnsembleToken::new(cfg, fqn.clone()));

        // Creating an ensemble at an occupied name is command replacement, not
        // in-place token mutation. The old token must become dead (an active
        // unknown callback observes UNKNOWN_DELETED), while imports of the
        // occupied source binding are explicitly reattached to the new token.
        if let Some(old_token) = old_token.as_ref() {
            self.retire_ensemble_identity(&fqn, old_token);
        } else {
            self.on_command_replaced(&fqn);
        }
        self.namespaces
            .borrow_mut()
            .bind(ns, &tail, Command::Ensemble(Rc::clone(&new_token)));
        self.retarget_imports_to_ensemble(&fqn, old_token.as_ref(), &new_token);
    }

    /// Define a user proc (`proc name params body`). The proc's defining
    /// namespace (where its body runs, and where it is bound) is the namespace
    /// `name` lands in — **relative to the current namespace** (so `proc next`
    /// inside `namespace eval counter` binds `::counter::next`, not a global).
    pub(crate) fn define_proc(&mut self, name: &[u8], params: Vec<Param>, body_obj: *mut TclObj) {
        self.invalidate_command_environment();
        let body = obj_bytes(body_obj);
        let ns = self
            .namespaces
            .borrow_mut()
            .command_home_ns(self.current_ns.get(), name);
        // Written-name tail: empty for a trailing separator run (`proc x::`
        // defines `::x::`, the `{}` command in `::x` — tclsh-pinned, #934);
        // must match `home_of`'s resolution split or the proc just defined
        // could not be invoked.
        let tail = tcl_syntax::naming::written_command_tail(name).to_vec();
        // The proc's FQN (`info frame` `proc` key): `<ns>::<tail>`, the global ns
        // contributing just the leading `::`.
        let qn = self.namespaces.borrow().qualified_name(ns);
        let mut fqn = qn.clone();
        if qn != b"::" {
            fqn.extend_from_slice(b"::");
        }
        fqn.extend_from_slice(&tail);
        // The proc's body frame reports `type source` with file-absolute lines
        // when its body argument is a located literal (TIP 280 LABC) — the body
        // word, not the `proc` command, carries the location, so a `proc` whose
        // body opens on a later line than the command is still file-accurate, and
        // a *dynamic* body (`proc p {} $bodyVar`, or a body from a dynamically
        // built list) has no location and stays body-relative (`type proc`),
        // matching C's literal line table rather than a whole-file "am I
        // sourcing" flag.
        let (source, body_line_base) = match self.arg_loc(body_obj) {
            Some((file @ Some(_), line)) => (file, line.saturating_sub(1)),
            _ => (None, 0),
        };
        let def = Rc::new(ProcDef {
            params,
            body,
            ns,
            fqn: fqn.clone(),
            source,
            body_line_base,
        });
        self.bind_command_replacement(ns, &tail, Command::Proc(def));
    }

    /// Install a fresh command token at an exact namespace binding, applying
    /// Tcl's command-replacement lifecycle first. The displaced command's
    /// delete trace runs while it is still visible, and all command/execution
    /// trace sidecars belonging to that old token are discarded before the new
    /// command is bound. Fresh bindings use the same funnel (the lifecycle step
    /// is then a no-op).
    pub(crate) fn bind_command_replacement(&mut self, ns: NsId, tail: &[u8], command: Command) {
        self.invalidate_command_environment();
        let qn = self.namespaces.borrow().qualified_name(ns);
        let mut fqn = qn.clone();
        if qn != b"::" {
            fqn.extend_from_slice(b"::");
        }
        fqn.extend_from_slice(tail);
        self.on_command_replaced(&fqn);
        self.namespaces.borrow_mut().bind(ns, tail, command);
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
        if self
            .traces
            .borrow()
            .cmd_traces
            .iter()
            .all(|t| t.name != fqn)
        {
            return;
        }
        let dying = self.resolve_cmd_token(fqn);
        self.fire_cmd_trace(fqn, b"", crate::cmd_trace::ops::DELETE);
        self.remove_cmd_traces_of_token(fqn, dying);
    }

    /// The generation of the command token `name` resolves to — the identity a
    /// command trace hangs off.
    pub(crate) fn resolve_cmd_token(&self, name: &[u8]) -> Option<u64> {
        self.namespaces
            .borrow()
            .resolve_generation(self.current_ns.get(), name)
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
        traces.cmd_traces.retain(|t| {
            t.name != fqn
                || match (t.token, dying) {
                    (Some(token), Some(dying)) => token > dying,
                    _ => false,
                }
        });
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
        let ns = self.namespaces.borrow();
        let mut cur = ns.resolve(self.current_ns.get(), name)?;
        // Bounded walk: an import chain cannot outlive the table, and a
        // malformed cycle terminates instead of spinning.
        for _ in 0..64 {
            match cur {
                Command::Ensemble(token) => return Some(token),
                Command::Imported {
                    source, ensemble, ..
                } => {
                    if let Some(token) = ensemble {
                        if !token.is_deleted() {
                            return Some(token);
                        }
                    }
                    cur = ns.resolve(GLOBAL, &source)?;
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
    /// invokehidden`'s `-global`/`-namespace` evaluation-context switch
    /// (issue #1412 item 5), which invokes one command rather than evaluating
    /// a script body, so it needs no `namespace eval`-style frame.
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
        if NATIVE_EVAL_DEPTH_LIMIT.exceeded(self.eval_depth.get() + 1) {
            self.error(b"too many nested evaluations (infinite loop?)");
            return false;
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
        self.eval_depth.set(self.eval_depth.get().saturating_sub(1));
        if self.eval_depth.get() != 0 {
            return;
        }
        if code == Code::Error {
            self.publish_error();
        }
        if !self.bg_queue.borrow().is_empty() {
            self.process_bg_errors();
        }
    }

    /// Enter a generated procedure body using the ordinary Tcl variable frame.
    pub(crate) fn codegen_frame_push(&mut self) {
        self.frames.borrow_mut().push(self.current_ns.get());
    }

    /// Leave a generated procedure body and restore its caller's namespace.
    pub(crate) fn codegen_frame_pop(&mut self) {
        let mut frames = self.frames.borrow_mut();
        frames.pop();
        self.current_ns.set(frames.frame_ns(frames.current_level()));
    }

    /// Associate an indexed generated local with its name-addressable Tcl cell.
    pub(crate) fn codegen_bind_slot(&self, slot: usize, name: &[u8]) {
        self.frames.borrow_mut().bind_compiled_slot(slot, name);
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
        let (base, _) = crate::frame::split_array_ref(name);
        let home = self.trace_identity(&base);
        self.traces
            .borrow()
            .traces
            .iter()
            .any(|t| crate::cmd_trace::same_variable(t, &home.base, home.ns, home.level))
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
        if !self.traces.borrow().traces.is_empty() {
            return None;
        }
        match self.frames.borrow().compiled_slot_var(slot)? {
            crate::frame::Var::Scalar(value) => Some(*value),
            _ => None,
        }
    }

    /// Begin an ensemble-rewrite (a forward / ensemble / constructor replacing
    /// the original command words). Returns `true` if this is the *root* rewrite
    /// (no rewrite was active) — the caller must `clear_ensemble_rewrite` when
    /// its dispatch returns. A nested rewrite is ignored (the root's `source` is
    /// what `wrong # args` reports), matching the common case of C's
    /// `TclInitRewriteEnsemble` chaining.
    pub(crate) fn begin_ensemble_rewrite(
        &self,
        source: Vec<Vec<u8>>,
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
        self.invalidate_command_environment();
        self.namespaces.borrow_mut()
    }

    /// Bootstrap the standard library like C's `Tcl_Init`: source
    /// `$tcl_library/init.tcl`. After this the
    /// pure-Tcl `unknown`/auto-load/`package` machinery is live, so
    /// `package require` works through `pkgIndex.tcl`/`tclIndex`.
    /// Set the predefined variables (`tcl_version`/`tcl_platform`/`env`/
    /// `argv`/…) that C installs in `Tcl_CreateInterp`, before `Tcl_Init`.
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
            self.var_unset_at(format!("::{name}").as_bytes(), 0);
        }
        for name in tcl_platform::bootstrap::HOST_PATH_GLOBALS {
            self.var_unset_at(format!("::{name}").as_bytes(), 0);
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
            Some(bytes) => self.eval_sourced(&bytes, init_path.as_bytes()),
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

    /// `source`: evaluate `script` as a sourced file named `name`, tracking it on
    /// the script stack (`info script`). A top-level `return` ends the file (the
    /// return boundary maps `return` → Ok); other codes propagate.
    pub fn eval_sourced(&mut self, script: &[u8], name: &[u8]) -> Code {
        self.script_stack.borrow_mut().push(name.to_vec());
        // A `source`d file is its own `info frame` level: `type source` + the
        // file path, inheriting the enclosing proc/level. Its commands are
        // numbered by the file's own lines (base 0, the file *is* the script).
        let mut frame = self.inherited_cmd_frame();
        frame.kind = FrameKind::Source;
        frame.file = Some(Rc::from(name));
        frame.line_base = 0;
        let code = self.eval_framed(script, frame);
        self.script_stack.borrow_mut().pop();
        self.settle_return(code)
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

    /// `namespace eval name body`: switch the current namespace to `name`
    /// (creating it, relative to the current ns unless `::`-anchored), evaluate
    /// `body` there, then restore. The current-ns switch is what makes commands
    /// defined in `body` land in the right table.
    pub(crate) fn ns_eval(&mut self, name: &[u8], body: &[u8]) -> Code {
        self.ns_eval_framed(name, body, None, true)
    }

    /// `namespace eval`-style body evaluation in `name` for callers that supply
    /// their *own* errorInfo frame (the TclOO `eval`/`my eval` method, which logs
    /// `(in "my eval" script line N)` instead of the `namespace eval` frame).
    pub(crate) fn ns_eval_no_frame(&mut self, name: &[u8], body: &[u8]) -> Code {
        self.ns_eval_framed(name, body, None, false)
    }

    /// `namespace eval` of a single body **object** — like
    /// [`ns_eval`](Self::ns_eval), but a literal obj with a recorded TIP 280
    /// source location runs as `type source` at its file+line (so a `proc`/
    /// command defined inside `namespace eval { … }` reports file-absolute
    /// `info frame` lines), rather than the dynamic `type eval`.
    pub(crate) fn ns_eval_obj(&mut self, name: &[u8], obj: *mut TclObj) -> Code {
        let loc = self.arg_loc(obj);
        let bytes = obj_bytes(obj);
        self.ns_eval_framed(name, &bytes, loc, true)
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
    ) -> Code {
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
        let target = self
            .namespaces
            .borrow_mut()
            .ensure_namespace(self.current_ns.get(), name);
        let saved = self.current_ns.get();
        self.current_ns.set(target);
        // A namespace frame: a new scope whose unqualified vars resolve to the
        // namespace (so `set`/`variable`/`upvar 0` inside `namespace eval` —
        // including when nested in a proc — target the namespace, not the
        // enclosing proc's locals).
        self.frames.borrow_mut().push_namespace(target);
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
            line: 1,
            oo: None,
            lambda: None,
        };
        let code = self.eval_framed(body, frame);
        if code == Code::Error && add_eval_frame {
            // `(in namespace eval "::ns" script line N)` — the body's own frame.
            let fqn = self.namespaces.borrow().qualified_name(target);
            self.append_namespace_eval_frame(&fqn);
        }
        self.frames.borrow_mut().pop();
        self.current_ns.set(saved);
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

    // -- variables (the var resolver; `crate::vars`) --------------------------
    //
    // Every variable op routes through the one classification + link walk
    // (frame-local vs namespace, qualified vs not), instead of the old flat
    // per-frame table. The `name` here is the array *base* (callers split
    // `a(k)` via `split_array_ref` first), so `::ns::base` qualifies correctly.

    /// `set name` — borrowed value (the table keeps its +1), or `None`.
    pub(crate) fn var_get(&self, name: &[u8]) -> Option<*mut TclObj> {
        crate::vars::get(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            self.current_ns.get(),
            name,
        )
    }

    // -- frame-addressed access (the `VarStore` `FrameId`-honouring path) -----
    //
    // Resolve `name` as if `level` were the active frame. Used only for a
    // non-active `FrameId`; the active frame keeps the by-name accessors above.

    /// Frame-addressed [`var_get`](Self::var_get).
    pub(crate) fn var_get_at(&self, name: &[u8], level: usize) -> Option<*mut TclObj> {
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
        crate::vars::set_elem_at(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            name,
            key,
            obj,
            level,
        )
    }

    /// Frame-addressed [`var_unset`](Self::var_unset).
    pub(crate) fn var_unset_at(&mut self, name: &[u8], level: usize) -> bool {
        crate::vars::unset_at(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            name,
            level,
        )
    }

    /// Frame-addressed `unset name(key)`.
    pub(crate) fn var_unset_elem_at(&mut self, name: &[u8], key: &[u8], level: usize) -> bool {
        crate::vars::unset_elem_at(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            name,
            key,
            level,
        )
    }

    /// Frame-addressed [`var_exists`](Self::var_exists).
    pub(crate) fn var_exists_at(&self, name: &[u8], level: usize) -> bool {
        crate::vars::exists_at(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            name,
            level,
        )
    }

    /// `set name(key)` — borrowed.
    pub(crate) fn var_get_elem(&self, name: &[u8], key: &[u8]) -> Option<*mut TclObj> {
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
        crate::vars::set(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
            obj,
        )?;
        if !self.traces.borrow().traces.is_empty() {
            let (base, elem) = crate::frame::split_array_ref(name);
            if self.fire_var_trace(&base, elem.as_deref(), b"write") {
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
        crate::vars::set_elem(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
            key,
            obj,
        )?;
        if !self.traces.borrow().traces.is_empty() && self.fire_var_trace(name, Some(key), b"write")
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
        let (base, elem) = crate::frame::split_array_ref(name);
        if elem.is_none() && self.is_constant(&base) {
            let mut m = b"can't set \"".to_vec();
            m.extend_from_slice(name);
            m.extend_from_slice(b"\": variable is a constant");
            return Some(self.set_error(&m));
        }
        None
    }

    /// Whether `name` resolves to a `const` scalar.
    pub(crate) fn is_constant(&self, name: &[u8]) -> bool {
        crate::vars::is_constant(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            self.current_ns.get(),
            name,
        )
    }

    /// `array default set arrayName value` — set the array's TIP 508 default
    /// (creating an empty array if needed). `Err` if the name is a scalar / its
    /// namespace is missing.
    pub(crate) fn set_array_default(
        &mut self,
        name: &[u8],
        obj: *mut TclObj,
    ) -> Result<(), VarError> {
        crate::vars::set_array_default(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
            obj,
        )
    }

    /// Ensure `name` is an array (creating an empty one if unset) — backs
    /// `array set name {}` with an empty value list. A scalar `name` errors.
    pub(crate) fn ensure_array(&self, name: &[u8]) -> Result<(), VarError> {
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
        crate::vars::ensure_undefined(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
        )
    }

    /// The array's TIP 508 default value (borrowed), or `None`.
    pub(crate) fn array_default(&self, name: &[u8]) -> Option<*mut TclObj> {
        crate::vars::array_default(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            self.current_ns.get(),
            name,
        )
    }

    /// `array default unset arrayName` — drop the array's default value.
    pub(crate) fn unset_array_default(&mut self, name: &[u8]) {
        crate::vars::unset_array_default(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
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
    /// The fully-qualified name `name` (resolved from namespace `base_ns`,
    /// following links) ultimately points at — for the `varname` object method.
    pub(crate) fn resolved_var_full_name(&self, base_ns: NsId, name: &[u8]) -> Option<Vec<u8>> {
        crate::vars::resolved_full_name(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            base_ns,
            name,
        )
    }

    /// The `(home namespace, home frame level, simple name)` identity a variable
    /// trace on `base` belongs to.
    ///
    /// Registration and firing both key on this, so every spelling that
    /// resolves to the one cell shares one trace list — including an `upvar`
    /// alias, whose level can only be read off the *resolved* place (issue
    /// #1633's `upvar` row). C gets this for free: there the alias and its
    /// target are the same `Var`, and the trace list hangs off that `Var`.
    pub(crate) fn trace_identity(&self, base: &[u8]) -> crate::vars::TraceHome {
        crate::vars::trace_home(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            self.current_ns.get(),
            base,
        )
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
    /// first is C's local-slot / hash walk and is not a pinned property, so the
    /// frame's own (sorted) name order stands in for it (issue #1575 row 1).
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
            Some(table) => table.names().into_iter().map(<[u8]>::to_vec).collect(),
            None => return Vec::new(),
        };
        let mut victims = Vec::new();
        for name in names {
            let home = crate::vars::TraceHome {
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
                .and_then(|t| t.array_names(&name))
                .map(|keys| keys.into_iter().map(<[u8]>::to_vec).collect::<Vec<_>>())
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
        if self.traces.borrow().traces.is_empty() {
            return None;
        }
        let (base, elem) = crate::frame::split_array_ref(name);
        let key = key.or(elem.as_deref());
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
        if !self.traces.borrow().traces.is_empty() && self.fire_var_trace(base, elem, b"read") {
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
        // Resolve the trace key BEFORE removing the variable. Resolution can
        // depend on the cell still existing — the 8.x namespace-scope fallback
        // only reaches the global when the global cell is present — so
        // re-resolving after the removal would silently pick a different
        // variable and the unset trace would never fire (issue #1328).
        // C resolves the `Var`, fires its traces, and only then frees it.
        let (base, elem) = crate::frame::split_array_ref(name);
        let traced = !self.traces.borrow().traces.is_empty();
        let key = traced.then(|| self.trace_identity(&base));
        // Unsetting a whole array destroys each element cell too, and C's
        // `DeleteArray` fires each element's own traces — with `arrayPtr` NULL,
        // so only that element's list runs — after the array's own firing
        // (tclVar.c). The elements have to be read while the array is still
        // there (issue #1575 row 3).
        let elements = if traced && elem.is_none() {
            self.array_names(&base).unwrap_or_default()
        } else {
            Vec::new()
        };
        let existed = crate::vars::unset(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
        );
        if let (true, Some(home)) = (existed, key) {
            let access = self.trace_access(name, &base, elem.as_deref(), &home, true);
            self.fire_var_trace_resolved(&home, &access, b"unset");
            // Then, for a whole-array unset, each element cell in turn.
            let per_element: Vec<VarTeardownCallback> = elements
                .iter()
                .flat_map(|e| self.cell_unset_traces(&home, Some(e), &access.reported, e))
                .collect();
            self.fire_unset_callbacks(per_element);
            // The variable (and its traces) go away — drop every trace on it
            // (C frees the Var's trace list on unset). Element unset drops only
            // that element's traces (whole-variable traces survive) — and an
            // alias for an element (`upvar #0 a(k) e; unset e`) is an element
            // unset, so the element comes from the access shape, not the
            // spelling.
            let mut t = self.traces.borrow_mut();
            match access.match_elem.as_deref() {
                Some(e) => t.traces.retain(|v| {
                    !(crate::cmd_trace::same_variable(v, &home.base, home.ns, home.level)
                        && v.elem.as_deref() == Some(e))
                }),
                None => t.traces.retain(|v| {
                    !crate::cmd_trace::same_variable(v, &home.base, home.ns, home.level)
                }),
            }
            drop(t);
            self.invalidate_guard_domain(GuardDomain::VariableTrace);
        }
        existed
    }

    /// `unset name(key)` — returns whether it existed.
    pub(crate) fn var_unset_elem(&mut self, name: &[u8], key: &[u8]) -> bool {
        // Resolved before the removal — see [`Self::var_unset`].
        let trace_key =
            (!self.traces.borrow().traces.is_empty()).then(|| self.trace_identity(name));
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
            let access = self.trace_access(&spelling, name, Some(key), &home, true);
            self.fire_var_trace_resolved(&home, &access, b"unset");
            // Drop this element's traces (whole-array traces survive).
            self.traces.borrow_mut().traces.retain(|v| {
                !(crate::cmd_trace::same_variable(v, &home.base, home.ns, home.level)
                    && v.elem.as_deref() == Some(key))
            });
            self.invalidate_guard_domain(GuardDomain::VariableTrace);
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
        // `upvar` alias, and the 8.x namespace-scope fallback) — issues #1328,
        // #1633.
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

    /// [`Self::fire_var_trace`] with the identity already resolved — for
    /// `unset`, which must resolve *before* it removes the variable (resolution
    /// can depend on the cell existing).
    fn fire_var_trace_resolved(
        &mut self,
        home: &crate::vars::TraceHome,
        access: &TraceAccess,
        op: &[u8],
    ) -> bool {
        let (access_ns, access_frame_level, base) = (home.ns, home.level, home.base.as_slice());
        let elem = access.match_elem.as_deref();
        let reported = access.reported.as_slice();
        // The cell this access reaches, and the array cell containing it — C's
        // `varPtr` and `arrayPtr`, each with its own `VAR_TRACE_ACTIVE`.
        let cell = crate::cmd_trace::VarTraceScope::cell(base, elem, access_ns, access_frame_level);
        let traces = self.traces.borrow();
        // "If there are already similar trace functions active for the
        // variable, don't call them again" — C's early return on
        // `TclIsVarTraceActive(varPtr)` (tclTrace.c 9.0.4:2513). Per *cell*: a
        // callback writing a different element of the same array is a different
        // `Var` and fires (issue #1574).
        if traces.active_var_scopes.contains(&cell) {
            return false;
        }
        let array_active = elem.is_some() && traces.active_var_scopes.contains(&cell.array());
        // C fires the containing array's traces before the element's own
        // (`TclCallVarTraces`, tclTrace.c 9.0.4: the `arrayPtr` loop at :2581
        // precedes the `varPtr` loop at :2623), and walks each list head→tail
        // — newest-first, since `TraceVarEx` prepends (:3090-3092). Our Vec
        // pushes newest-last, so each group is reversed. Issue #1440.
        let selected = |whole_array: bool| {
            traces
                .traces
                .iter()
                .rev()
                .filter(move |t| t.elem.is_none() == whole_array)
                .filter(|t| {
                    crate::cmd_trace::matches(t, base, elem, op, access_ns, access_frame_level)
                })
        };
        let array_group = (access.whole_array && !array_active)
            .then(|| selected(true))
            .into_iter()
            .flatten();
        // The walk order is fixed here, but *what runs* is re-read at each step:
        // C follows `active.nextTracePtr`, which `Tcl_UntraceVar2` rewrites when
        // a callback removes a trace mid-walk, so a trace removed during the
        // firing does not fire in that same pass. Snapshotting the callbacks
        // themselves would run one that is already gone (issue #1633). Ids are
        // enough to re-find each registration, and are never reused.
        let order: Vec<u64> = array_group.chain(selected(false)).map(|t| t.id).collect();
        drop(traces);
        if order.is_empty() {
            return false;
        }
        // C aborts the chain on the first callback error for every op *except*
        // unset — "ignore errors in unset traces" (tclTrace.c 9.0.4:2600). An
        // `array` trace's error therefore fails the `array` subcommand.
        let propagate = op != b"unset";
        // Preserve the result object across the callbacks.
        let saved = self.result.get();
        unsafe { obj::incr_ref_count(saved) };

        // The cell is marked active for the whole firing, as C marks `varPtr`
        // once on entry and clears it on the way out — not per callback.
        self.traces
            .borrow_mut()
            .active_var_scopes
            .push(cell.clone());

        let mut errored = false;
        let op_name = String::from_utf8_lossy(op).into_owned();
        for id in order {
            // Still registered? A previous callback in this same firing may have
            // removed it (C's `nextTracePtr` rewrite).
            let Some((cmd, old_style)) = self
                .traces
                .borrow()
                .traces
                .iter()
                .find(|t| t.id == id)
                .map(|t| (t.command.clone(), t.old_style))
            else {
                continue;
            };
            // Append `base element op` as properly-quoted trailing words. A
            // trace installed by the deprecated `trace variable` form is
            // called with the single `rwua` letter (C's `TCL_TRACE_OLD_STYLE`).
            let op_word = tcl_cmd_core::trace::callback_op_word(&op_name, old_style);
            let args = crate::list::new_list_obj(&[
                new_string(reported),
                new_string(access.report_elem.as_deref().unwrap_or(b"")),
                new_string(op_word.as_bytes()),
            ]);
            let mut line = cmd;
            line.push(b' ');
            line.extend_from_slice(&obj_bytes(args));
            drop_fresh(args);
            let code = self.eval_str(&line);
            if propagate && code == Code::Error {
                // Capture the callback's error message; stop firing (C aborts
                // the trace chain on the first error).
                let msg = self.result_bytes();
                self.traces.borrow_mut().pending_err = Some(msg);
                // C's `TclCallVarTraces` error tail (tclTrace.c 9.0.4:2662-2696)
                // keeps the callback's own errorInfo chain and appends a
                // `(<type> trace on "name")` frame to it; only the *result* is
                // replaced afterwards, by `TclVarErrMsg`. The element named
                // here is the one from the access **spelling** — C snapshots
                // `part2` before recovering one from a linked element — so
                // `set a(k) 2` reports `(write trace on "a(k)")` while
                // `set e 5` through `upvar #0 a(k) e` reports
                // `(write trace on "e")`.
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
                errored = true;
                break;
            }
        }
        let popped = self.traces.borrow_mut().active_var_scopes.pop();
        debug_assert_eq!(popped, Some(cell));
        // Restore the saved result (release the trace's, adopt our held +1).
        unsafe {
            obj::decr_ref_count(self.result.get());
            self.result.set(saved);
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
    /// `can't set "x": …` line (issue #1633's errorInfo row).
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
            let mut exc = self.exc.borrow_mut();
            exc.code = code;
            exc.code_explicit = false;
        }
        Code::Error
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
        let (base, elem) = crate::frame::split_array_ref(name);
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
        let (base, elem) = crate::frame::split_array_ref(var);
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
    fn fire_cmd_trace(&mut self, old_fqn: &[u8], new_fqn: &[u8], op_bit: u8) {
        if self
            .traces
            .borrow()
            .firing_cmd_traces
            .iter()
            .any(|firing| firing == old_fqn)
        {
            return;
        }
        // C prepends each new command trace (`Tcl_TraceCommand`, tclTrace.c
        // 9.0.4:1016-1018) and `CallCommandTraces` walks the list head→tail
        // (tclBasic.c:3972-3974), so the newest fires first. Our Vec pushes
        // newest-last. Issue #1440.
        // The callbacks are captured up front, not re-read per step: this walk
        // owns the dying token's list, which a callback's re-creation of the
        // command detaches from the name. See [`Self::cmd_trace_untraced`].
        let entries: Vec<(u64, Vec<u8>)> = self
            .traces
            .borrow()
            .cmd_traces
            .iter()
            .rev()
            .filter(|t| t.name == old_fqn && (t.ops & op_bit) != 0)
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
        let saved = self.result.get();
        unsafe { obj::incr_ref_count(saved) };

        {
            let mut traces = self.traces.borrow_mut();
            traces.exec_firing += 1;
            traces.firing_cmd_traces.push(old_fqn.to_vec());
        }
        for (id, cmd) in entries {
            if self.cmd_trace_untraced(id) {
                continue;
            }
            // Append `oldName newName op` as properly-quoted list elements.
            let args = crate::list::new_list_obj(&[
                new_string(old_fqn),
                new_string(new_fqn),
                new_string(op),
            ]);
            let mut line = cmd;
            line.push(b' ');
            line.extend_from_slice(&obj_bytes(args));
            drop_fresh(args);
            let _ = self.eval_str(&line);
        }
        {
            let mut traces = self.traces.borrow_mut();
            traces.exec_firing -= 1;
            traces.firing_cmd_traces.pop();
            if traces.exec_firing == 0 {
                traces.untraced_cmd_trace_ids.clear();
            }
        }

        unsafe {
            obj::decr_ref_count(self.result.get());
            self.result.set(saved);
        }
    }

    /// The callback prefix of the live command/execution trace `id`, or `None`
    /// when a callback has since removed it. C walks the trace list through
    /// `nextPtr` and `Tcl_UntraceCommand` unlinks a record at once, so a trace
    /// removed mid-firing never fires in that pass. Issue #1633 row 8.
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
    fn fire_exec_enter(&mut self, fqn: &[u8], cmd_word: &[u8]) -> Option<Code> {
        use crate::cmd_trace::ops;
        // C fires `enter` newest-first (the trace list is prepended; the loop
        // walks it head→tail). Our Vec pushes newest-last, so iterate reversed.
        let ids: Vec<u64> = self
            .traces
            .borrow()
            .cmd_traces
            .iter()
            .rev()
            .filter(|t| t.name == fqn && (t.ops & ops::ENTER) != 0)
            .map(|t| t.id)
            .collect();
        if ids.is_empty() {
            return None;
        }
        let saved = self.result.get();
        unsafe { obj::incr_ref_count(saved) };
        self.traces.borrow_mut().exec_firing += 1;
        let mut abort: Option<Code> = None;
        for id in ids {
            let Some(cmd) = self.live_cmd_trace(id) else {
                continue;
            };
            let args = crate::list::new_list_obj(&[new_string(cmd_word), new_string(b"enter")]);
            let mut line = cmd;
            line.push(b' ');
            line.extend_from_slice(&obj_bytes(args));
            drop_fresh(args);
            let c = self.eval_str(&line);
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
        }
        abort
    }

    /// Fire `leave` execution traces on `fqn` (reverse creation order), invoking
    /// each as `<prefix> {cmd args} <code> <result> leave`. A leave-trace non-OK
    /// code overrides the command's result/code (C's `TEOV_RunLeaveTraces`).
    fn fire_exec_leave(&mut self, fqn: &[u8], cmd_word: &[u8], code: Code) -> Code {
        use crate::cmd_trace::ops;
        // C fires `leave` oldest-first (reverse-scan of the prepended list). Our
        // Vec pushes newest-last, so iterate forward.
        let ids: Vec<u64> = self
            .traces
            .borrow()
            .cmd_traces
            .iter()
            .filter(|t| t.name == fqn && (t.ops & ops::LEAVE) != 0)
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
        let code_str = code.as_int().to_string().into_bytes();

        self.traces.borrow_mut().exec_firing += 1;
        let mut override_code: Option<Code> = None;
        for id in ids {
            let Some(cmd) = self.live_cmd_trace(id) else {
                continue;
            };
            let result_bytes = obj_bytes(self.result.get());
            let args = crate::list::new_list_obj(&[
                new_string(cmd_word),
                new_string(&code_str),
                new_string(&result_bytes),
                new_string(b"leave"),
            ]);
            let mut line = cmd;
            line.push(b' ');
            line.extend_from_slice(&obj_bytes(args));
            drop_fresh(args);
            let c = self.eval_str(&line);
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
                code
            }
        }
    }

    /// Move every command/execution trace on `old_fqn` to `new_fqn` (the trace
    /// follows a renamed command, as C keeps the trace list on the moving
    /// `Command`).
    fn move_cmd_traces(&mut self, old_fqn: &[u8], new_fqn: &[u8]) {
        // C moves the `Command` itself, so its trace list travels with the
        // token. Re-stamp to whatever token now stands at the destination, so
        // a later deletion there still tells this list from a replacement's.
        let token = self.resolve_cmd_token(new_fqn);
        let mut traces = self.traces.borrow_mut();
        let mut moved = false;
        for t in traces.cmd_traces.iter_mut() {
            if t.name == old_fqn {
                t.name = new_fqn.to_vec();
                t.token = token;
                moved = true;
            }
        }
        drop(traces);
        if moved {
            self.invalidate_guard_domain(GuardDomain::CommandTrace);
        }
    }

    /// Drop every command/execution trace on `fqn` (the command is gone).
    fn remove_cmd_traces(&mut self, fqn: &[u8]) {
        let mut traces = self.traces.borrow_mut();
        let old_len = traces.cmd_traces.len();
        traces.cmd_traces.retain(|t| t.name != fqn);
        let removed = traces.cmd_traces.len() != old_len;
        drop(traces);
        if removed {
            self.invalidate_guard_domain(GuardDomain::CommandTrace);
        }
    }

    /// Whether `name` resolves to an array variable (`set a` array-vs-scalar
    /// diagnostic, `array exists`).
    pub(crate) fn var_is_array(&self, name: &[u8]) -> bool {
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
        if tcl_syntax::naming::is_qualified(name) {
            self.namespaces.borrow().var_home(context_ns, name)
        } else {
            Some((context_ns, name.to_vec()))
        }
    }

    /// The current call-frame level (`upvar` relative-level arithmetic).
    pub(crate) fn current_level(&self) -> usize {
        self.frames.borrow().current_level()
    }

    /// `variable tail` / `global tail` — link `tail` in the current frame to
    /// `target_ns::tail` (a no-op when the current context already is that var).
    pub(crate) fn make_variable(&mut self, target_ns: NsId, tail: &[u8]) {
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

    /// Resolve (creating if needed) a namespace by name, anchored at the
    /// **global** namespace regardless of the current one — C's
    /// `TclGetNamespaceForQualName(..., TCL_GLOBAL_ONLY |
    /// TCL_CREATE_NS_IF_UNKNOWN)`. Used by `interp invokehidden -namespace`
    /// (issue #1412 item 5): `-namespace bar` names `::bar` even when called
    /// from inside another namespace (tclsh 9.0.4-pinned).
    pub(crate) fn ensure_global_namespace(&mut self, name: &[u8]) -> NsId {
        self.invalidate_command_environment();
        self.namespaces.borrow_mut().ensure_namespace(GLOBAL, name)
    }

    /// Delete the namespace `ns` (by id), e.g. an OO object's instance namespace
    /// when the object is destroyed.
    pub(crate) fn delete_namespace_by_id(&mut self, ns: NsId) {
        self.invalidate_command_environment();
        let teardown_ids = self.namespaces.borrow().descendant_ids(ns);
        self.delete_namespace_token(ns);
        self.sweep_dying_namespace(ns, &teardown_ids);
        self.namespaces
            .borrow_mut()
            .finish_namespace_teardown(&teardown_ids);
    }

    /// Tear down one exact namespace token in Tcl's recursive order. Its owned
    /// ensembles retire while this token and all children are live; then only
    /// this token becomes dying and loses its ordinary command table. Children
    /// receive the same lifecycle recursively after the parent's callbacks.
    fn delete_namespace_token(&mut self, ns: NsId) {
        // Variables in this namespace are about to be unset. Names are captured
        // while the token is live, then callbacks run after this exact token is
        // marked dying (C's order; oo-11.8).
        let victims = self.take_ns_unset_traces(ns);
        // Ensemble commands are tied to their configured namespace, even when
        // their binding lives elsewhere (for example the default global `::ns`
        // command), so they retire before this token is marked dying. Every
        // other command in the table retires one token at a time below.
        {
            let ids = std::collections::HashSet::from([ns]);
            let mut deleted_origins: std::collections::HashSet<Vec<u8>> =
                std::collections::HashSet::new();
            let mut ensemble_victims = self.namespaces.borrow().ensembles_for(&ids);
            let hidden_tokens: Vec<(Vec<u8>, Rc<crate::ensemble::EnsembleToken>)> = self
                .hidden
                .borrow()
                .iter()
                .filter_map(|(name, command)| match command {
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
            for (fqn, token) in ensemble_victims {
                if deleted_tokens.iter().any(|seen| Rc::ptr_eq(seen, &token)) {
                    continue;
                }
                deleted_origins.insert(fqn.clone());
                if let Some(live_fqn) = self.retire_ensemble_identity(&fqn, &token) {
                    deleted_origins.insert(live_fqn);
                }
                deleted_tokens.push(token);
            }
            self.remove_imports_for_deleted_origins(deleted_origins, &deleted_tokens);
        }
        self.namespaces.borrow_mut().begin_namespace_teardown(ns);
        self.namespaces.borrow_mut().clear_namespace_token(ns);
        self.fire_unset_callbacks(victims);
        self.tear_down_command_table(ns);

        // Tcl snapshots and recursively deletes children only after this
        // token's ordinary command callbacks have completed. A callback may
        // already have deleted one; skip any token no longer publicly live.
        let children = self.namespaces.borrow().children_hash_order(ns);
        for child in children {
            if self.namespaces.borrow().namespace_is_live(child) {
                self.delete_namespace_token(child);
            }
        }
    }

    /// Delete one dying namespace's command table the way `TclTeardownNamespace`
    /// does: snapshot `cmdTable` in `Tcl_FirstHashEntry` order, then delete each
    /// snapshotted token in turn, repeating while the table is non-empty.
    ///
    /// Each token's `delete` traces fire while its entry is still in the table
    /// (`Tcl_DeleteCommandFromToken` calls `CallCommandTraces` before
    /// `Tcl_DeleteHashEntry`), and its imports retire depth-first straight
    /// after — not in a bulk pass over the whole namespace. A callback that
    /// deletes or redefines a snapshotted entry changes its generation; C's
    /// `CMD_DYING` early return then leaves the replacement to the next
    /// snapshot.
    fn tear_down_command_table(&mut self, ns: NsId) {
        loop {
            let snapshot = self.namespaces.borrow().command_hash_order(ns);
            if snapshot.is_empty() {
                break;
            }
            let mut retired_any = false;
            for (tail, generation) in snapshot {
                if self.namespaces.borrow().command_generation(ns, &tail) != Some(generation) {
                    continue;
                }
                let fqn = self.namespaces.borrow().command_fqn_at(ns, &tail);
                self.on_command_replaced(&fqn);
                if self.namespaces.borrow().command_generation(ns, &tail) != Some(generation) {
                    // The callback deleted this token, or redefined the name:
                    // its own deletion already unlinked the entry, and any
                    // replacement is a distinct token for the next snapshot.
                    retired_any = true;
                    continue;
                }
                let ensemble_tokens = match self.namespaces.borrow().command_in(ns, &tail) {
                    Some(Command::Ensemble(token)) => vec![token],
                    _ => Vec::new(),
                };
                self.namespaces.borrow_mut().remove_in(ns, &tail);
                self.remove_imports_for_deleted_origins([fqn], &ensemble_tokens);
                retired_any = true;
            }
            if !retired_any {
                break;
            }
        }
    }

    /// Finish commands created re-entrantly while a namespace's original
    /// delete callbacks ran. The detached namespace remains command-addressable
    /// during this sweep, but never reappears in the visible namespace tree.
    fn sweep_dying_namespace(&mut self, root: NsId, retained_ids: &[NsId]) {
        let mut ids = retained_ids.to_vec();
        let mut traced = std::collections::HashSet::<Vec<u8>>::new();
        let mut origins = std::collections::HashSet::<Vec<u8>>::new();
        let mut tokens = Vec::<Rc<crate::ensemble::EnsembleToken>>::new();

        loop {
            for id in self.namespaces.borrow().descendant_ids(root) {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            let id_set: std::collections::HashSet<NsId> = ids.iter().copied().collect();

            let mut ensemble_victims = self.namespaces.borrow().ensembles_for(&id_set);
            ensemble_victims.extend(self.hidden.borrow().iter().filter_map(|(name, command)| {
                let Command::Ensemble(token) = command else {
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
            for (fqn, token) in ensemble_victims {
                if tokens.iter().any(|seen| Rc::ptr_eq(seen, &token)) {
                    continue;
                }
                found_new_token = true;
                origins.insert(fqn.clone());
                if let Some(live_fqn) = self.retire_ensemble_identity(&fqn, &token) {
                    origins.insert(live_fqn);
                }
                tokens.push(token);
            }

            let fqns = self.namespaces.borrow().command_fqns_in_ids(&ids);
            let new_fqns: Vec<Vec<u8>> = fqns
                .iter()
                .filter(|fqn| !traced.contains(*fqn))
                .cloned()
                .collect();
            for fqn in &new_fqns {
                // Callback-created command traces fire while the command is
                // still addressable through the dying namespace token.
                self.on_command_replaced(fqn);
                traced.insert(fqn.clone());
            }

            for id in self.namespaces.borrow().descendant_ids(root) {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            origins.extend(self.namespaces.borrow().command_fqns_in_ids(&ids));
            self.remove_imports_for_deleted_origins(origins.iter().cloned(), &tokens);

            let remaining = self.namespaces.borrow().command_fqns_in_ids(&ids);
            if !found_new_token
                && new_fqns.is_empty()
                && remaining.iter().all(|f| traced.contains(f))
            {
                break;
            }
        }

        // Import delete callbacks may have added a new child under the detached
        // root during the last fixed-point pass.
        for id in self.namespaces.borrow().descendant_ids(root) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        origins.extend(self.namespaces.borrow().command_fqns_in_ids(&ids));
        self.remove_imports_for_deleted_origins(origins.iter().cloned(), &tokens);
        self.namespaces.borrow_mut().clear_namespace_ids(&ids);
        for fqn in origins {
            self.remove_cmd_traces(&fqn);
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
        self.traces
            .borrow()
            .traces
            .iter()
            .rev()
            .filter(|t| {
                crate::cmd_trace::same_variable(t, &home.base, home.ns, home.level)
                    && t.elem.as_deref() == elem
                    && t.ops.iter().any(|o| o == b"unset")
            })
            .map(|t| {
                (
                    report_name.to_vec(),
                    report_elem.to_vec(),
                    t.command.clone(),
                    t.old_style,
                )
            })
            .collect()
    }

    /// Remove and return the `(fullName, command)` of every *unset* variable
    /// trace registered directly on a namespace variable in `ns`. Descendants
    /// retain their traces until recursive teardown reaches their own token.
    fn take_ns_unset_traces(&self, ns: NsId) -> Vec<VarTeardownCallback> {
        if self.traces.borrow().traces.iter().all(|t| t.ns.is_none()) {
            return Vec::new();
        }
        let mut victims = Vec::new();
        let mut traces = self.traces.borrow_mut();
        let old_len = traces.traces.len();
        let ns_ref = self.namespaces.borrow();
        traces.traces.retain(|t| {
            let hit =
                t.ns.is_some_and(|n| n == ns && t.ops.iter().any(|o| o == b"unset"));
            if hit {
                let home = t.ns.unwrap();
                let mut fqn = ns_ref.qualified_name(home);
                if fqn != b"::" {
                    fqn.extend_from_slice(b"::");
                }
                // `base` may be qualified (registered as `::n::x`); use its
                // simple tail under the home namespace.
                let simple = match t.base.windows(2).rposition(|w| w == b"::") {
                    Some(i) => &t.base[i + 2..],
                    None => &t.base[..],
                };
                fqn.extend_from_slice(simple);
                if let Some(e) = &t.elem {
                    fqn.push(b'(');
                    fqn.extend_from_slice(e);
                    fqn.push(b')');
                }
                victims.push((fqn, Vec::new(), t.command.clone(), t.old_style));
            }
            !hit
        });
        let removed = traces.traces.len() != old_len;
        drop(ns_ref);
        drop(traces);
        if removed {
            self.invalidate_guard_domain(GuardDomain::VariableTrace);
        }
        // Grouped per variable, newest-first inside each group — see
        // [`Self::group_newest_first_per_entity`]. Issue #1440.
        Self::group_newest_first_per_entity(victims, |victim| victim.0.clone())
    }

    /// Order a namespace's collected teardown callbacks the way C fires them.
    ///
    /// C tears a namespace down one entity at a time — a per-`Var` loop for
    /// variables, a per-`Command` loop for commands — and each of those
    /// completes that entity's whole trace list before the next entity starts.
    /// So an entity's callbacks are **contiguous**, and within an entity the
    /// newest registration fires first (the list is prepended and walked head
    /// to tail).
    ///
    /// We collect with `retain` over one flat, registration-ordered Vec, which
    /// interleaves entities. Regroup it: entities keep the order they were
    /// first seen, each group runs newest-first. *Which* entity comes first is
    /// C's hash-table walk and is deliberately not pinned — but that a group is
    /// contiguous is pinned regardless of hash order, which a flat reverse got
    /// wrong (`A1 B1 A2 B2` fired `B2 A2 B1 A1`, not C's `A2 A1 B2 B1`).
    fn group_newest_first_per_entity<T>(victims: Vec<T>, key: impl Fn(&T) -> Vec<u8>) -> Vec<T> {
        let mut order: Vec<Vec<u8>> = Vec::new();
        let mut groups: std::collections::HashMap<Vec<u8>, Vec<T>> =
            std::collections::HashMap::new();
        for victim in victims {
            let entity = key(&victim);
            groups
                .entry(entity.clone())
                .or_insert_with(|| {
                    order.push(entity);
                    Vec::new()
                })
                .push(victim);
        }
        order
            .into_iter()
            .flat_map(|key| {
                let mut group = groups.remove(&key).unwrap_or_default();
                group.reverse();
                group
            })
            .collect()
    }

    /// Fire collected unset-trace callbacks as `command name {} unset`. Errors
    /// are ignored (an unset trace's result is discarded, as in C).
    fn fire_unset_callbacks(&mut self, victims: Vec<VarTeardownCallback>) {
        if victims.is_empty() {
            return;
        }
        let saved = self.result.get();
        unsafe { obj::incr_ref_count(saved) };
        for (name, elem, cmd, old_style) in victims {
            // A trace registered the deprecated way is called with the `rwua`
            // letter, not the operation name — the teardown path must honour
            // that exactly as the explicit-unset path does (`TraceVarProc`,
            // tclTrace.c 8.6.16:2002-2011).
            let op = tcl_cmd_core::trace::callback_op_word("unset", old_style);
            let args = crate::list::new_list_obj(&[
                new_string(&name),
                new_string(&elem),
                new_string(op.as_bytes()),
            ]);
            let mut line = cmd;
            line.push(b' ');
            line.extend_from_slice(&obj_bytes(args));
            drop_fresh(args);
            let _ = self.eval_str(&line);
        }
        unsafe {
            obj::decr_ref_count(self.result.get());
            self.result.set(saved);
        }
    }

    /// Resolve a (relative/absolute) namespace name to its id, or `None`.
    pub(crate) fn find_namespace_id(&self, name: &[u8]) -> Option<NsId> {
        self.namespaces
            .borrow()
            .find_namespace(self.current_ns.get(), name)
    }

    /// Namespace lookup for defining a qualified command. A detached dying
    /// namespace is intentionally invisible to namespace introspection but its
    /// retained command table remains a valid definition target until the
    /// teardown callback sweep completes.
    pub(crate) fn find_command_namespace_id(&self, name: &[u8]) -> Option<NsId> {
        let namespaces = self.namespaces.borrow();
        namespaces
            .find_namespace(self.current_ns.get(), name)
            .or_else(|| namespaces.dying_namespace(self.current_ns.get(), name))
    }

    // -- introspection (`info` / `array`) -------------------------------------

    /// `info exists name` — whether a scalar/array/element variable is set
    /// (splitting `arr(key)`).
    pub(crate) fn var_exists(&self, name: &[u8]) -> bool {
        let (base, elem) = crate::frame::split_array_ref(name);
        match elem {
            Some(k) => crate::vars::exists_elem(
                &self.frames.borrow(),
                &self.namespaces.borrow(),
                self.current_ns.get(),
                &base,
                &k,
            ),
            None => crate::vars::exists(
                &self.frames.borrow(),
                &self.namespaces.borrow(),
                self.current_ns.get(),
                &base,
            ),
        }
    }

    /// The element names of array `name` (`array names`/`get`), or `None`.
    pub(crate) fn array_names(&self, name: &[u8]) -> Option<Vec<Vec<u8>>> {
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

    /// Variable names visible in the current scope (`info vars`): the active
    /// frame's locals in a proc, else the current namespace's variables.
    pub(crate) fn visible_var_names(&self) -> Vec<Vec<u8>> {
        if self.frames.borrow().in_proc() {
            self.frames.borrow().local_names()
        } else {
            self.namespaces.borrow().var_names(self.current_ns.get())
        }
    }

    /// `info consts` — the `const` scalar names visible in the current scope.
    /// Filters the visible variables by constness (following links), so an OO
    /// instance variable linked to a `const` namespace variable is included.
    pub(crate) fn visible_const_names(&self) -> Vec<Vec<u8>> {
        self.visible_var_names()
            .into_iter()
            .filter(|n| self.is_constant(n))
            .collect()
    }

    /// `info consts ns::pat` — the `const` scalar names in the namespace named
    /// `qualifier` (absolute or relative to the current namespace).
    pub(crate) fn consts_in_namespace(&self, qualifier: &[u8]) -> Vec<Vec<u8>> {
        let ns = self.namespaces.borrow();
        let target = if qualifier.is_empty() {
            Some(GLOBAL)
        } else {
            ns.find_namespace(self.current_ns.get(), qualifier)
        };
        match target {
            Some(id) => {
                let mut v = ns.const_names(id);
                v.sort();
                v
            }
            None => Vec::new(),
        }
    }

    /// The canonical fully-qualified prefix (ending in `::`) of the namespace a
    /// pattern qualifier addresses (`info vars ns::pat`): `::` for the global
    /// namespace, `::a::b::` otherwise. Resolves a *relative* qualifier against
    /// the current namespace, so results are always absolute (matching C, where
    /// names are re-qualified through the namespace's `fullName`). `None` if the
    /// namespace doesn't exist. (Used by [`set_list_qualified`] for `info
    /// vars`/`consts`; command/proc re-qualification moved to the shared core.)
    pub(crate) fn canonical_ns_prefix(&self, qualifier: &[u8]) -> Option<Vec<u8>> {
        let ns = self.namespaces.borrow();
        let id = if qualifier.is_empty() {
            GLOBAL
        } else {
            ns.find_namespace(self.current_ns.get(), qualifier)?
        };
        let mut p = ns.qualified_name(id);
        if id != GLOBAL {
            p.extend_from_slice(b"::"); // global's qualified_name is already `::`
        }
        Some(p)
    }

    /// Simple command names in the namespace named `qualifier` (absolute or
    /// relative to the current namespace), or empty if it does not exist. Used by
    /// `cmd_oo` for ensemble/method enumeration. (`info commands`/`procs` listing
    /// is the shared `tcl_cmd_core::info::command_list` core.)
    pub(crate) fn commands_in_namespace(&self, qualifier: &[u8]) -> Vec<Vec<u8>> {
        let target = {
            let ns = self.namespaces.borrow();
            // An empty qualifier (a leading `::pattern`) addresses the global ns.
            if qualifier.is_empty() {
                Some(GLOBAL)
            } else {
                ns.find_namespace(self.current_ns.get(), qualifier)
            }
        };
        target.map_or_else(Vec::new, |id| self.visible_command_names_in(id))
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
                // builtins; every script-created object stays invariant.
                let gated = match ns.resolve(id, name) {
                    Some(Command::Builtin(_)) => true,
                    Some(Command::OoObject(fqn)) => {
                        self.0.registry_object_roots.borrow().contains(&fqn)
                    }
                    _ => false,
                };
                let mut full_name = prefix.clone();
                full_name.extend_from_slice(name);
                (!gated || self.builtin_command_visible_for_surface(&full_name))
                    .then(|| name.to_vec())
            })
            .collect()
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
        let ns = self.namespaces.borrow();
        let mut cmd = ns.resolve(self.current_ns.get(), name)?;
        // Follow `namespace import` redirects to the underlying proc, so
        // `info args`/`body`/`default` work on an imported proc (info-1.7/2.4).
        for _ in 0..64 {
            match cmd {
                Command::Proc(def) => return Some(def),
                Command::Imported {
                    source, ensemble, ..
                } if ensemble.as_ref().is_none_or(|token| token.is_deleted()) => {
                    cmd = ns.resolve(GLOBAL, &source)?;
                }
                _ => return None,
            }
        }
        None
    }

    /// `upvar` — link `local` in the current frame to the resolved `target`.
    pub(crate) fn make_upvar(&mut self, target: Link, local: &[u8]) {
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
        crate::vars::make_upvar_in(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            home_ns,
            tail,
            target,
        );
    }

    // -- result ---------------------------------------------------------------

    /// `Tcl_SetObjResult`: retain `obj` into the result slot, release the prior.
    ///
    /// # Safety
    /// `obj` must be a live `TclObj`.
    pub unsafe fn set_obj_result(&mut self, obj: *mut TclObj) {
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
    pub(crate) fn set_result_byte_array(&mut self, bytes: &[u8]) {
        let obj = crate::bytearray::new_byte_array(bytes);
        // SAFETY: fresh byte-array object; the interpreter takes its owning
        // reference exactly as it does for an ordinary fresh string object.
        unsafe { self.set_obj_result(obj) };
    }

    /// `Tcl_GetObjResult` — borrowed (interp keeps its +1).
    pub fn get_obj_result(&self) -> *mut TclObj {
        self.result.get()
    }

    /// The current result's string bytes (copied).
    pub fn result_bytes(&self) -> Vec<u8> {
        obj_bytes(self.result.get())
    }

    /// The current result **object** — a borrowed pointer (the interp keeps its
    /// reference; the caller must not release it without first taking its own
    /// `+1`). Backs `Commands::dispatch`'s completion capture in `state_traits.rs`.
    pub(crate) fn result_obj(&self) -> *mut TclObj {
        self.result.get()
    }

    /// Intern a command's fully-qualified name to a stable, dense raw
    /// `CommandId`, minting one on first sight. Backs `Namespaces::find_command`.
    fn intern_cmd(&self, fqn: &[u8]) -> u32 {
        let mut a = self.cmd_arena.borrow_mut();
        if let Some(&id) = a.ids.get(fqn) {
            return id;
        }
        let id = u32::try_from(a.fqns.len()).expect("command count fits u32");
        a.fqns.push(fqn.to_vec());
        a.ids.insert(fqn.to_vec(), id);
        id
    }

    /// Resolve `name` from namespace context `cxt` (through the namespace tree to
    /// the root) to its command's FQN, then intern that to a stable raw
    /// `CommandId`. `None` if it resolves to no command. The `Namespaces::find_command`
    /// engine (`state_traits.rs`), keeping the namespace-table access here.
    pub(crate) fn find_command_id(&self, cxt: NsId, name: &[u8]) -> Option<u32> {
        let fqn = self.namespaces.borrow().resolve_fqn(cxt, name)?;
        Some(self.intern_cmd(&fqn))
    }

    /// The FQN an interned raw `CommandId` was minted from, or `None` for a
    /// fabricated/out-of-range id. Backs `Commands::dispatch_id`'s reverse step.
    pub(crate) fn command_fqn(&self, id: u32) -> Option<Vec<u8>> {
        self.cmd_arena.borrow().fqns.get(id as usize).cloned()
    }

    /// The command an interned command was ultimately imported from — C's
    /// `TclGetOriginalCommand` (following an imported command's retained
    /// ensemble token or by-name source to a fixed point), interned in its turn.
    /// `None` when it is not an imported command. Backs
    /// `Namespaces::command_origin`. Bounded against a cycle a retargeting bug
    /// could leave behind; a well-formed chain is acyclic.
    pub(crate) fn imported_source_id(&self, id: u32) -> Option<u32> {
        let mut fqn = self.command_fqn(id)?;
        let mut hops = 0;
        loop {
            let next = match self.namespaces.borrow().resolve(GLOBAL, &fqn) {
                Some(Command::Imported {
                    source, ensemble, ..
                }) => ensemble
                    .filter(|token| !token.is_deleted())
                    .map_or(source, |token| token.name()),
                _ => break,
            };
            fqn = next;
            hops += 1;
            if hops >= 64 {
                break;
            }
        }
        (hops > 0).then(|| self.intern_cmd(&fqn))
    }

    /// Immediate source binding and optional real-ensemble identity for a new
    /// `namespace import`. Import-of-import chains deliberately keep every hop:
    /// replacing or deleting the intermediate command must affect its own
    /// importers before `namespace origin` walks any farther toward the root.
    /// Only a *direct* ensemble source contributes an ensemble token; an
    /// imported ensemble is reached through its intermediate command binding.
    pub(crate) fn import_metadata_at(
        &self,
        name: &[u8],
    ) -> Option<(Vec<u8>, Option<Rc<crate::ensemble::EnsembleToken>>)> {
        let namespaces = self.namespaces.borrow();
        let fqn = namespaces.resolve_fqn(self.current_ns.get(), name)?;
        let ensemble = match namespaces.resolve(GLOBAL, &fqn)? {
            Command::Ensemble(token) => Some(token),
            _ => None,
        };
        Some((fqn, ensemble))
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
    /// The `-errorcode` taxonomy is conservative for now: `wrong # args` ⇒ `TCL
    /// WRONGARGS`, else `NONE` (the full taxonomy is a follow-up). `error`/`throw`
    /// set a richer code on their own paths.
    pub(crate) fn error(&mut self, msg: &[u8]) -> Code {
        self.set_result_bytes(msg);
        let code: &[u8] = if msg.starts_with(b"wrong # args:") {
            b"TCL WRONGARGS"
        } else {
            b"NONE"
        };
        *self.exc.borrow_mut() = ExceptionState {
            info: None,
            code: code.to_vec(),
            code_explicit: false,
            already_logged: false,
        };
        Code::Error
    }

    /// Like [`error`](Self::error) but with an explicit `-errorcode` (the trace
    /// still builds up as the error unwinds). For commands that mirror C's
    /// richer error codes (`TCL LOOKUP INDEX …`, `TCL OO …`).
    pub(crate) fn error_with_code(&mut self, msg: &[u8], code: &[u8]) -> Code {
        self.set_result_bytes(msg);
        *self.exc.borrow_mut() = ExceptionState {
            info: None,
            code: code.to_vec(),
            code_explicit: false,
            already_logged: false,
        };
        Code::Error
    }

    /// The `interp bgerror` handler command prefix (empty ⇒ default).
    pub(crate) fn bgerror_handler(&self) -> Vec<u8> {
        self.bgerror.borrow().clone()
    }

    /// Set the `interp bgerror` handler command prefix.
    pub(crate) fn set_bgerror_handler(&self, prefix: &[u8]) {
        self.invalidate_interpreter_policy();
        *self.bgerror.borrow_mut() = prefix.to_vec();
    }

    /// `interp bgerror` get / set on this interp. With no `prefix` it returns
    /// the current handler; with one it must be a list of length ≥ 1 (set, then
    /// returned). Returns the handler bytes, or the error message.
    pub(crate) fn bgerror_apply(&self, prefix: Option<*mut TclObj>) -> Result<Vec<u8>, Vec<u8>> {
        match prefix {
            None => Ok(self.bgerror_handler()),
            Some(p) => {
                if crate::list::list_length(p).unwrap_or(0) < 1 {
                    return Err(b"cmdPrefix must be list of length >= 1".to_vec());
                }
                let bytes = obj_bytes(p);
                self.set_bgerror_handler(&bytes);
                Ok(bytes)
            }
        }
    }

    /// Queue a background error (a destructor failing during implicit teardown,
    /// etc.) for later processing by `update` — Tcl defers it to the event loop
    /// rather than firing at the error site.
    pub(crate) fn report_bg_error(&mut self, msg: &[u8], options: &[u8]) {
        self.bg_queue
            .borrow_mut()
            .push((msg.to_vec(), options.to_vec()));
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
    ) -> std::cell::RefMut<'_, std::collections::BTreeMap<Vec<u8>, crate::cmd_coro::CoroEntry>>
    {
        self.coros.borrow_mut()
    }

    /// Swap the interp's per-flow *execution context* (call frames, the
    /// `info frame` stack, current namespace, recursion depth, return/error
    /// state, the TclOO call/define stacks, …) with `ctx`. This is how a
    /// coroutine handoff installs the resuming side's context: a single swap is
    /// its own inverse, so resume (caller→coro) and yield (coro→caller) each
    /// call it once. The shared *definitions* (namespaces, commands, classes,
    /// channels, the result object) are not swapped — coroutines share them.
    pub(crate) fn swap_coro_ctx(&self, ctx: &mut CoroContext) {
        {
            let mut f = self.frames.borrow_mut();
            std::mem::swap(&mut *f, &mut ctx.frames);
        }
        std::mem::swap(&mut *self.cmd_frames.borrow_mut(), &mut ctx.cmd_frames);
        std::mem::swap(&mut *self.script_stack.borrow_mut(), &mut ctx.script_stack);
        std::mem::swap(&mut *self.arg_lines.borrow_mut(), &mut ctx.arg_lines);
        std::mem::swap(&mut *self.exc.borrow_mut(), &mut ctx.exc);
        self.oo.borrow_mut().swap_exec(&mut ctx.oo);
        let ns = self.current_ns.replace(ctx.current_ns);
        ctx.current_ns = ns;
        let rd = self.recursion_depth.replace(ctx.recursion_depth);
        ctx.recursion_depth = rd;
        let rc = self.return_code.replace(ctx.return_code);
        ctx.return_code = rc;
        let rl = self.return_level.replace(ctx.return_level);
        ctx.return_level = rl;
        let ed = self.eval_depth.replace(ctx.eval_depth);
        ctx.eval_depth = ed;
        let el = self.error_line.replace(ctx.error_line);
        ctx.error_line = el;
    }

    /// Swap the execution context of the coroutine named `name` with the live
    /// interpreter context (the resume/yield handoff in `cmd_coro`). A no-op if
    /// the coroutine is gone. The registry borrow is released before any
    /// blocking handoff.
    pub(crate) fn coro_swap_named(&self, name: &[u8]) {
        // Take the context out (releasing the registry borrow), swap, put back —
        // so `swap_coro_ctx`'s RefCell touches never overlap the registry borrow.
        let taken = self
            .coros
            .borrow_mut()
            .get_mut(name)
            .map(|e| std::mem::replace(&mut e.context, CoroContext::placeholder()));
        if let Some(mut ctx) = taken {
            self.swap_coro_ctx(&mut ctx);
            if let Some(e) = self.coros.borrow_mut().get_mut(name) {
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

    /// Register coroutine command `name` (its invocation resumes it).
    pub(crate) fn register_coroutine_command(&mut self, name: &[u8]) {
        self.ns_register(name, Command::Builtin(crate::cmd_coro::coro_resume_command));
    }

    /// Process queued background errors (called by `update`): invoke the
    /// `interp bgerror` handler as `{*}$handler $msg $options` for each. With no
    /// handler set they are dropped. The caller's result is preserved.
    pub(crate) fn process_bg_errors(&mut self) {
        // Drain in FIFO order; processing one may queue more.
        while !self.bg_queue.borrow().is_empty() {
            let batch: Vec<(Vec<u8>, Vec<u8>)> = std::mem::take(&mut self.bg_queue.borrow_mut());
            for (msg, options) in batch {
                let handler = self.bgerror_handler();
                if handler.is_empty() {
                    continue;
                }
                let saved = self.result_bytes();
                let hobj = obj::new_string_bytes(&handler);
                unsafe { obj::incr_ref_count(hobj) };
                let words = crate::list::list_elements(hobj).unwrap_or_default();
                let mut argv: Vec<*mut TclObj> = Vec::with_capacity(words.len() + 2);
                for w in words {
                    unsafe { obj::incr_ref_count(w) };
                    argv.push(w);
                }
                for s in [&msg, &options] {
                    let o = obj::new_string_bytes(s);
                    unsafe { obj::incr_ref_count(o) };
                    argv.push(o);
                }
                if !argv.is_empty() {
                    let _ = self.dispatch(&argv);
                }
                for a in &argv {
                    unsafe { obj::decr_ref_count(*a) };
                }
                unsafe { obj::decr_ref_count(hobj) };
                self.set_result_bytes(&saved);
            }
        }
    }

    /// Pre-seed the error trace for `error msg info ?code?` / `throw`: the result
    /// is `msg`, the trace starts at `info` (so the throwing command is **not**
    /// re-logged — `ERR_ALREADY_LOGGED`), and `-errorcode` is `code`. Returns
    /// [`Code::Error`].
    pub(crate) fn raise_with_info(&mut self, msg: &[u8], info: &[u8], code: &[u8]) -> Code {
        self.set_result_bytes(msg);
        *self.exc.borrow_mut() = ExceptionState {
            info: Some(info.to_vec()),
            code: code.to_vec(),
            code_explicit: false,
            already_logged: true,
        };
        Code::Error
    }

    /// Begin a fresh error whose result the caller has already set, with
    /// `-errorcode` `code` and an empty trace (it accumulates as the error
    /// unwinds). Used by `error msg`/`throw`. Returns [`Code::Error`].
    pub(crate) fn set_error_state(&mut self, code: &[u8]) -> Code {
        *self.exc.borrow_mut() = ExceptionState {
            info: None,
            code: code.to_vec(),
            code_explicit: false,
            already_logged: false,
        };
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
        let (info, already_logged) = match errorinfo {
            Some(i) if !i.is_empty() => (Some(i.to_vec()), true),
            _ => (None, false),
        };
        *self.exc.borrow_mut() = ExceptionState {
            info,
            code: errorcode.unwrap_or(b"NONE").to_vec(),
            code_explicit: errorcode.is_some(),
            already_logged,
        };
        if let Some(es) = errorstack {
            if let Ok(parts) = crate::parse::split_list(es) {
                *self.error_stack.borrow_mut() = parts;
                self.reset_error_stack.set(false);
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
        self.error_stack_log(&src[cmd.start..cmd.end]);
        // errorLine, measured against the enclosing `codePtr->source` (C's
        // `TclLogCommandInfo`): the command's file-absolute line
        // (`line_base + 1 + count('\n')`) minus the proc/eval body's base, so an
        // inline `catch`/`if` body's commands still report their proc-body line.
        // This is the *only* writer of `error_line`; it persists across `catch`
        // and a subsequent `error msg info`.
        let raw = line_of(src, cmd.start);
        let line = self.cmd_frames.borrow().last().map_or(raw, |f| {
            (f.line_base + raw).saturating_sub(f.proc_line_base).max(1)
        });
        self.error_line.set(line);
        let started = self.exc.borrow().info.is_some();
        if !started {
            // First frame: errorInfo is seeded from the error message (the result).
            let msg = self.result_bytes();
            self.exc.borrow_mut().info = Some(msg);
        }
        let verb: &[u8] = if started {
            b"invoked from within"
        } else {
            b"while executing"
        };
        let cmd_bytes = &src[cmd.start..cmd.end];
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
        // Pre-8.5 compatibility (C's `TclLogCommandInfo`, tclNamesp.c): if user
        // code traces `::errorInfo` for writes, push the value out to the variable
        // *now*, mid-unwind — firing the write trace while the failing command's
        // call frame is still live (so the handler's `info level` sees it). Skipped
        // (no var write) when nothing traces `::errorInfo`, so the normal path
        // still publishes once, at the `catch`/top level.
        if self.errorinfo_has_write_trace() {
            let info = self.error_info();
            let ei = new_string(&info);
            if self.var_set(b"::errorInfo", ei).is_err() {
                drop_fresh(ei);
            }
        }
    }

    /// Whether `::errorInfo` carries a user write-trace — C's `TclIsVarTraced`
    /// gate in `TclLogCommandInfo` (we install no core `EstablishErrorInfoTraces`,
    /// so any matching write trace qualifies).
    fn errorinfo_has_write_trace(&self) -> bool {
        if self.traces.borrow().traces.is_empty() {
            return false;
        }
        // Same `(home, simple name)` key registration and firing use — a
        // literal `::errorInfo` here would miss every trace now that traces
        // are keyed by the resolved variable rather than the spelling.
        let home = self.trace_identity(b"::errorInfo");
        self.traces
            .borrow()
            .traces
            .iter()
            .any(|t| crate::cmd_trace::matches(t, &home.base, None, b"write", home.ns, home.level))
    }

    /// Append the `(procedure "NAME" line N)` / `(lambda term "..." line N)`
    /// frame when a proc/lambda body unwinds with an error (`MakeProcError` /
    /// `MakeLambdaError`, `tclProc.c`), then clear `already_logged` so the
    /// proc-call command itself is logged by its enclosing eval. The line is the
    /// body-relative `error_line` the innermost body command recorded.
    fn make_proc_error(&mut self, frame: ProcFrame) {
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

    /// Append `"\n    (parsing lambda expression \"<name>\")"` to errorInfo — the
    /// `apply` lambda-parse failure frame (C's `Tcl_AppendObjToErrorInfo` in
    /// `Tcl_ApplyObjCmd`). Unlike [`append_frame_line`](Self::append_frame_line)
    /// it carries no `line N` suffix. Clears `already_logged` so the enclosing
    /// `apply` command still logs its own `invoked from within` frame.
    pub(crate) fn append_lambda_parse_frame(&mut self, name: &[u8]) {
        if self.exc.borrow().info.is_none() {
            let msg = self.result_bytes();
            self.exc.borrow_mut().info = Some(msg);
        }
        {
            let mut exc = self.exc.borrow_mut();
            let buf = exc.info.as_mut().expect("seeded above");
            buf.extend_from_slice(b"\n    (parsing lambda expression \"");
            buf.extend_from_slice(name);
            buf.extend_from_slice(b"\")");
        }
        self.exc.borrow_mut().already_logged = false;
    }

    pub(crate) fn append_frame_line(&mut self, inner: &[u8]) {
        let line = self.error_line.get();
        if self.exc.borrow().info.is_none() {
            let msg = self.result_bytes();
            self.exc.borrow_mut().info = Some(msg);
        }
        let mut exc = self.exc.borrow_mut();
        let buf = exc.info.as_mut().expect("seeded above");
        buf.extend_from_slice(b"\n    (");
        buf.extend_from_slice(inner);
        buf.extend_from_slice(b" line ");
        buf.extend_from_slice(line.to_string().as_bytes());
        buf.push(b')');
    }

    /// Append a frame with no `line N` suffix — `"\n    (<text>)"`, e.g.
    /// `("for" initial command)` / `("for" loop-end command)` (C's
    /// `Tcl_AddErrorInfo` for the `for` init/next scripts) — seeding errorInfo
    /// from the result if needed, then clear `already_logged` so the enclosing
    /// command logs its own `invoked from within` frame.
    pub(crate) fn append_frame_noline(&mut self, text: &[u8]) {
        if self.exc.borrow().info.is_none() {
            let msg = self.result_bytes();
            self.exc.borrow_mut().info = Some(msg);
        }
        {
            let mut exc = self.exc.borrow_mut();
            let buf = exc.info.as_mut().expect("seeded above");
            buf.extend_from_slice(b"\n    (");
            buf.extend_from_slice(text);
            buf.push(b')');
        }
        self.exc.borrow_mut().already_logged = false;
    }

    /// Append a literal `"\n    <text>"` errorInfo context line. Ensemble
    /// unknown-handler result diagnostics use this Tcl_AddErrorInfo shape (no
    /// parentheses and no line suffix), then allow the enclosing command to log
    /// its ordinary `invoked from within` frame.
    fn append_error_info_context(&mut self, text: &[u8]) {
        if self.exc.borrow().info.is_none() {
            let msg = self.result_bytes();
            self.exc.borrow_mut().info = Some(msg);
        }
        {
            let mut exc = self.exc.borrow_mut();
            let buf = exc.info.as_mut().expect("seeded above");
            buf.extend_from_slice(b"\n    ");
            buf.extend_from_slice(text);
        }
        self.exc.borrow_mut().already_logged = false;
    }

    /// The current accumulated `errorInfo` (for `catch`'s `-errorinfo`): the
    /// trace if any frame was logged, else the bare error message.
    pub(crate) fn error_info(&self) -> Vec<u8> {
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
            info: exc.info.clone(),
            code: exc.code.clone(),
            code_explicit: exc.code_explicit,
        }
    }

    /// Restore an [`ErrorSnapshot`] captured by [`snapshot_error`] into this
    /// flow's exception state (the trace continues from there — e.g. `coroprobe`
    /// then appends its own `(injected coroutine probe command)` frame).
    pub(crate) fn restore_error(&self, snap: ErrorSnapshot) {
        let mut exc = self.exc.borrow_mut();
        exc.info = snap.info;
        exc.code = snap.code;
        exc.code_explicit = snap.code_explicit;
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
    pub(crate) fn cmd_frame_info(&self, n: i64) -> Option<Vec<(Vec<u8>, Vec<u8>)>> {
        let cmd_frames = self.cmd_frames.borrow();
        let depth = cmd_frames.len() as i64;
        let pos = if n > 0 { n } else { depth + n };
        if pos < 1 || pos > depth {
            return None;
        }
        let f = &cmd_frames[(pos - 1) as usize];
        let mut pairs = vec![
            (b"type".to_vec(), f.kind.as_bytes().to_vec()),
            (b"line".to_vec(), f.line.to_string().into_bytes()),
        ];
        if let Some(file) = &f.file {
            pairs.push((b"file".to_vec(), file.to_vec()));
        }
        pairs.push((b"cmd".to_vec(), f.cmd.clone()));
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
        Some(pairs)
    }

    /// Snapshot the current error/result state (the result bytes + the
    /// `errorInfo`/`errorCode` accumulator) so a side-effecting cleanup (e.g.
    /// running a destructor after a failed constructor) can run and then have
    /// the original error restored.
    pub(crate) fn error_snapshot(&self) -> (Vec<u8>, ExceptionState) {
        (self.result_bytes(), self.exc.borrow().clone())
    }

    /// Restore a previously taken [`error_snapshot`](Self::error_snapshot).
    pub(crate) fn error_restore(&mut self, snap: (Vec<u8>, ExceptionState)) {
        self.set_result_bytes(&snap.0);
        *self.exc.borrow_mut() = snap.1;
    }

    /// The current `errorCode` (for `catch`'s `-errorcode`): the stamped value,
    /// or `NONE`.
    pub(crate) fn error_code(&self) -> Vec<u8> {
        let exc = self.exc.borrow();
        if exc.code.is_empty() && !exc.code_explicit {
            b"NONE".to_vec()
        } else {
            exc.code.clone()
        }
    }

    /// Mark the live error's `-errorcode` as explicitly supplied (so an explicit
    /// empty code is preserved instead of defaulting to `NONE`; error-4.5).
    pub(crate) fn mark_error_code_explicit(&mut self) {
        self.exc.borrow_mut().code_explicit = true;
    }

    /// Record one command frame into the TIP 348 error stack as an error unwinds
    /// (`Tcl_LogCommandInfo`'s errorStack half). On the first log of a new error
    /// episode (`reset_error_stack`), the stack is cleared and seeded with `INNER
    /// <command>`. Then, if the active frame is an `uplevel`-redirected one
    /// (`framePtr != varFramePtr`), an `UP <delta>` entry is appended. `CALL`
    /// entries are added separately at proc-frame boundaries
    /// ([`error_stack_push_call`](Self::error_stack_push_call)).
    pub(crate) fn error_stack_log(&self, command: &[u8]) {
        if self.reset_error_stack.get() {
            self.reset_error_stack.set(false);
            let mut es = self.error_stack.borrow_mut();
            es.clear();
            es.push(b"INNER".to_vec());
            es.push(command.to_vec());
        }
        let (top, active) = {
            let f = self.frames.borrow();
            (f.top_level(), f.current_level())
        };
        if top > active {
            let mut es = self.error_stack.borrow_mut();
            es.push(b"UP".to_vec());
            es.push((top - active).to_string().into_bytes());
        }
    }

    /// Append a TIP 348 `CALL <info level 0>` entry — the invocation words of a
    /// proc/lambda/method frame that an error is unwinding out of. The words are
    /// joined into a single Tcl-list element (so `g 1212` renders as `{g 1212}`).
    pub(crate) fn error_stack_push_call(&self, words: &[Vec<u8>]) {
        if self.reset_error_stack.get() {
            // No inner context recorded yet (the error started at this boundary);
            // nothing to chain a CALL onto until a command is logged.
            return;
        }
        let mut value = Vec::new();
        for (i, w) in words.iter().enumerate() {
            if i > 0 {
                value.push(b' ');
            }
            crate::list::append_list_element(&mut value, w, i == 0);
        }
        let mut es = self.error_stack.borrow_mut();
        es.push(b"CALL".to_vec());
        es.push(value);
    }

    /// Render the TIP 348 error stack as a Tcl list (`info errorstack` / the
    /// options-dict `-errorstack` value).
    pub(crate) fn error_stack_value(&self) -> Vec<u8> {
        let es = self.error_stack.borrow();
        let mut buf = Vec::new();
        for (i, e) in es.iter().enumerate() {
            if i > 0 {
                buf.push(b' ');
            }
            crate::list::append_list_element(&mut buf, e, false);
        }
        buf
    }

    /// Mark the start of a new error episode (C's `iPtr->resetErrorStack = 1`,
    /// set by `Tcl_ResetResult`): the *next* logged command rebuilds the stack.
    /// The current contents are kept until then (so `info errorstack` after a
    /// `catch` still reports the last error).
    pub(crate) fn mark_error_stack_reset(&self) {
        self.reset_error_stack.set(true);
    }

    /// Publish the accumulated trace to the `::errorInfo`/`::errorCode` globals
    /// and reset the accumulator for the next error. Called when the error is
    /// caught (`catch`) or reaches the outermost eval.
    fn publish_error(&mut self) {
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
        *self.exc.borrow_mut() = ExceptionState::default();
        // The exception is consumed: drop any `-during` chain link with it.
        self.clear_during();
        // A new error episode starts fresh: the next logged command rebuilds the
        // TIP 348 error stack (C's `Tcl_ResetResult` sets `resetErrorStack`).
        self.mark_error_stack_reset();
    }

    /// Publish + reset, for `catch`/`try` once they have captured the options.
    pub(crate) fn publish_and_reset_error(&mut self) {
        self.publish_error();
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

    /// The pending `-during` chain link (borrowed), for `build_options` to splice
    /// into an error's options dict. `None` when no chaining is active.
    pub(crate) fn during_opts(&self) -> Option<*mut TclObj> {
        let d = self.during.take();
        self.during.set(d);
        d
    }

    // -- eval -----------------------------------------------------------------

    /// Evaluate a whole script; the result is left in the interp result. Returns
    /// the completion code of the last command (or `Ok` for an empty script).
    ///
    /// At the true top level (no `info frame` stack yet) this owns the root
    /// `CmdFrame`; nested calls (command substitution `[cmd]`) run in the
    /// enclosing frame and add none — matching C, where `[cmd]` is the same
    /// `cmdFramePtr` level. A proc body / `eval` / `source` body gets its own
    /// frame via [`eval_framed`](Self::eval_framed).
    pub fn eval_str(&mut self, src: &[u8]) -> Code {
        let owned = self.cmd_frames.borrow().is_empty().then(CmdFrame::root);
        self.eval_script(src, owned)
    }

    /// Evaluate `src` as the body of its own `info frame` level (`frame` is
    /// pushed for the duration). Used by proc calls, `eval`/`uplevel`, and
    /// `source`.
    fn eval_framed(&mut self, src: &[u8], mut frame: CmdFrame) -> Code {
        // A freshly pushed frame is its own `codePtr->source`, so `errorLine` is
        // measured from this body's base (an inline `catch`/`if` body, by
        // contrast, shares the enclosing frame via `eval_shared_located_body` and
        // keeps the proc's `proc_line_base`).
        frame.proc_line_base = frame.line_base;
        self.eval_script(src, Some(frame))
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
        // Native-stack safety net — see `NATIVE_EVAL_DEPTH_LIMIT`'s doc
        // comment (issue #996). Checked before incrementing / doing any
        // other setup, so bailing out here needs no unwind: `owned` (not
        // yet pushed) simply drops normally.
        if NATIVE_EVAL_DEPTH_LIMIT.exceeded(self.eval_depth.get() + 1) {
            return self.error(b"too many nested evaluations (infinite loop?)");
        }
        self.eval_depth.set(self.eval_depth.get() + 1);
        let pushed = owned.is_some();
        let owns_frame = pushed || advance_shared;
        if let Some(f) = owned {
            self.cmd_frames.borrow_mut().push(f);
        }
        let mut last = Code::Ok;
        let commands = parse::parse_script_with_config(src, self.lexer_config());
        if commands.is_empty() {
            // A script with no commands (empty / whitespace / comments only)
            // evaluates to the empty result — `Tcl_EvalEx` resets the result at
            // entry, and with nothing to set it the result is empty. Without this
            // a stale prior result leaks through (e.g. an empty proc body, `eval
            // {}`, or an `lmap`/`foreach` body that produces nothing).
            self.set_result_bytes(b"");
        }
        for cmd in &commands {
            last = self.eval_command(src, cmd, owns_frame);
            if last != Code::Ok {
                break; // error/return/break/continue propagate up
            }
        }
        if pushed {
            self.cmd_frames.borrow_mut().pop();
        }
        self.eval_depth.set(self.eval_depth.get() - 1);
        // The outermost eval publishes the accumulated trace to the globals so
        // an uncaught error leaves `::errorInfo`/`::errorCode` set, exactly as a
        // `catch` would (`catch` publishes earlier, at depth > 0).
        if self.eval_depth.get() == 0 && last == Code::Error {
            self.publish_error();
        }
        // Between top-level commands, drain any queued background errors with the
        // current handler — the event loop's behaviour, so errors from one
        // command don't leak into a later command's intercepted handler.
        if self.eval_depth.get() == 0 && !self.bg_queue.borrow().is_empty() {
            self.process_bg_errors();
        }
        last
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
        if crate::list::is_pure_list(body) {
            let frame = self.unlocated_frame();
            return self.dispatch_list_obj(body, frame);
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
                let saved = (top.line_base, top.line, std::mem::take(&mut top.cmd));
                top.line_base = line.saturating_sub(1);
                saved
            })
        };
        let Some((line_base, line, cmd)) = saved else {
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

    /// Evaluate a body whose file-absolute first `line` and `file` were computed
    /// by the caller (C's `switch`/list-element TIP 280 path, where the body is a
    /// sub-element of a list literal, so it has no `Tcl_Obj` of its own to carry
    /// a location). Runs as a line-advancing `type source` frame.
    pub(crate) fn eval_located_body(
        &mut self,
        file: Option<Rc<[u8]>>,
        line: u32,
        body: &[u8],
    ) -> Code {
        let mut frame = self.inherited_cmd_frame();
        frame.kind = FrameKind::Source;
        frame.file = file;
        frame.line_base = line.saturating_sub(1);
        self.eval_framed(body, frame)
    }

    /// A `type eval`, no-file, body-relative `CmdFrame` (inheriting the enclosing
    /// proc/level) — the frame for a body with no source location (a dynamic
    /// script, or a canonical-list body).
    fn unlocated_frame(&self) -> CmdFrame {
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
                    let saved = (top.line_base, top.line, std::mem::take(&mut top.cmd));
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
        if let Some((line_base, line, cmd)) = saved {
            if let Some(top) = self.cmd_frames.borrow_mut().last_mut() {
                top.line_base = line_base;
                top.line = line;
                top.cmd = cmd;
            }
        }
        code
    }

    /// `eval` of a single body **object** — like [`eval_body`](Self::eval_body),
    /// but a literal obj with a recorded source location (TIP 280 LABC) runs as
    /// `type source` at its original file+line (the test-body case) rather than
    /// `type eval`.
    pub(crate) fn eval_body_obj(&mut self, obj: *mut TclObj) -> Code {
        // A pure list is one command, dispatched by element identity (so a
        // contained literal keeps its source location — C's list-eval path),
        // inside its own `type eval` frame.
        if crate::list::is_pure_list(obj) {
            let frame = self.unlocated_frame();
            return self.dispatch_list_obj(obj, frame);
        }
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
        let bytes = obj_bytes(obj);
        self.eval_framed(&bytes, frame)
    }

    /// Dispatch a pure-list script object as a single command, using its element
    /// objects directly (no stringify/re-parse) — this preserves each element's
    /// `Tcl_Obj` identity, so a nested `eval`/`uplevel $bodyVar` still finds the
    /// body's TIP 280 source location.
    fn dispatch_list_obj(&mut self, obj: *mut TclObj, frame: CmdFrame) -> Code {
        let elems = match crate::list::list_elements(obj) {
            Ok(e) => e,
            Err(e) => return self.error(e.message()),
        };
        if elems.is_empty() {
            self.set_result_bytes(b"");
            return Code::Ok;
        }
        // The pure list is exactly one command; push its `info frame` level (a
        // canonical-list body has no source location, so the frame is the
        // `type eval`, body-relative one the caller supplies) and report the list
        // string as the executing command, before dispatching by element identity.
        let mut owned = frame;
        owned.cmd = obj_bytes(obj);
        owned.line = owned.line_base + 1;
        self.cmd_frames.borrow_mut().push(owned);
        for &e in &elems {
            // SAFETY: live element; take an owning +1 for the call.
            unsafe { obj::incr_ref_count(e) };
        }
        let code = self.dispatch(&elems);
        release_all(&elems);
        self.cmd_frames.borrow_mut().pop();
        code
    }

    /// `uplevel` of a single body **object** — the redirected-scope variant of
    /// [`eval_body_obj`](Self::eval_body_obj). A located literal keeps its source
    /// provenance; a dynamic body is `type eval`, no file.
    pub(crate) fn eval_uplevel_obj(&mut self, target_level: usize, obj: *mut TclObj) -> Code {
        let loc = self.arg_loc(obj);
        let prev_level = self.frames.borrow_mut().set_active_level(target_level);
        let prev_ns = self.current_ns.get();
        self.current_ns
            .set(self.frames.borrow().frame_ns(target_level));
        let code = if crate::list::is_pure_list(obj) {
            // Pure list → one command by element identity (see `dispatch_list_obj`),
            // in a `type eval` frame redirected to the target level.
            let mut frame = self.unlocated_frame();
            frame.level = target_level;
            frame.omit_level = true;
            self.dispatch_list_obj(obj, frame)
        } else {
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
            let bytes = obj_bytes(obj);
            self.eval_framed(&bytes, frame)
        };
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
        }
        let code = self.eval_words(src, &cmd.words);
        if code == Code::Error {
            self.log_command_info(src, cmd);
        }
        code
    }

    /// Substitute each word of a command (with `{*}` expansion), then dispatch.
    fn eval_words(&mut self, src: &[u8], words: &[parse::Word]) -> Code {
        // C parses a command WHOLE before it substitutes any of it
        // (`Tcl_EvalEx` → `Tcl_ParseCommand` → `TclEvalObjvInternal`), so a
        // parse failure in a later word — or inside a later word's `[…]` —
        // stops an earlier word's command substitution from ever running.
        // Measured on 8.6.16 and 9.0.4: `list [sfx inner] {a}b` raises `extra
        // characters after close-brace` with `sfx` never called. Walking the
        // words in order and raising at the first `WordPart::ParseError` (this
        // engine's carrier for those failures — the scanner stays infallible so
        // the LSP can keep tokenizing) substituted word by word instead, which
        // ran `sfx`. Issue #1787; the gap #1818's header recorded as pending.
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
            let obj = match self.substitute_word(src, &w.body) {
                Ok(o) => o, // owned (+1)
                Err(code) => {
                    release_all(&argv);
                    return code;
                }
            };
            if w.expand {
                // Split the substituted value as a list; each element is an arg.
                let bytes = obj_bytes(obj);
                unsafe { obj::decr_ref_count(obj) }; // done with the word obj
                let elems = match parse::split_list(&bytes) {
                    Ok(e) => e,
                    Err(e) => {
                        release_all(&argv);
                        return self.error(e.message());
                    }
                };
                // For a `{*}` of a *literal* word, each element keeps its source
                // line (its offset within the literal — C's `TclListLines`), so a
                // body-defining element (`namespace {*}{eval ns {proc …}}`) is
                // `type source`. A dynamic `{*}$v` has no per-element location.
                let offsets = is_literal.then(|| scan_list_offsets(&bytes)).flatten();
                for (k, e) in elems.iter().enumerate() {
                    let eo = new_string(e);
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
            }
        }
        *self.arg_lines.borrow_mut() = arg_lines;

        let code = self.dispatch(&argv);
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
        self.cmd_count.set(self.cmd_count.get() + 1);
        // Fast path: no command/execution traces, or we're already inside a
        // trace callback (C's INTERP_TRACE_IN_PROGRESS) — original dispatch.
        let traced = {
            let t = self.traces.borrow();
            !t.cmd_traces.is_empty() && t.exec_firing == 0
        };
        if !traced {
            return self.dispatch_inner(argv);
        }
        self.dispatch_traced(argv)
    }

    /// Slow path: the command may carry execution (enter/leave/step) traces, or
    /// a step trace is active. Mirrors C's `TclEvalObjvInternal` order: interp
    /// (step) enter traces fire before per-command enter; per-command leave
    /// fires before interp (step) leave.
    fn dispatch_traced(&mut self, argv: &[*mut TclObj]) -> Code {
        use crate::cmd_trace::ops;
        let name = obj_bytes(argv[0]);
        let fqn = self.resolve_cmd_fqn(&name);
        let (has_enter, has_leave, has_step) = match &fqn {
            Some(f) => {
                let t = self.traces.borrow();
                let (mut he, mut hl, mut hs) = (false, false, false);
                for tr in t.cmd_traces.iter().filter(|tr| tr.name == *f) {
                    he |= (tr.ops & ops::ENTER) != 0;
                    hl |= (tr.ops & ops::LEAVE) != 0;
                    hs |= (tr.ops & ops::STEP_ANY) != 0;
                }
                (he, hl, hs)
            }
            None => (false, false, false),
        };
        let stepping = !self.traces.borrow().step_active.is_empty();
        if !has_enter && !has_leave && !has_step && !stepping {
            return self.dispatch_inner(argv);
        }
        // The `{cmd arg ...}` word: argv rendered as a single list element (C's
        // `TraceExecutionProc` builds it via per-arg `DStringAppendElement`).
        let cmd_word = {
            let lst = crate::list::new_list_obj(argv);
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
            if let Some(c) = self.fire_exec_enter(fqn.as_deref().unwrap(), &cmd_word) {
                return c;
            }
        }
        // (C) install this command's step traces (deduped against recursion).
        let installed = if has_step {
            self.install_step_traces(fqn.as_deref().unwrap())
        } else {
            0
        };
        let mut code = self.dispatch_inner(argv);
        // (D) remove the step traces installed above (they are the last pushed).
        if installed > 0 {
            self.remove_installed_step_traces(installed);
        }
        // (E) per-command leave (before interp/step leave), then (F) leavestep.
        if has_leave {
            code = self.fire_exec_leave(fqn.as_deref().unwrap(), &cmd_word, code);
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
    fn install_step_traces(&mut self, fqn: &[u8]) -> usize {
        use crate::cmd_trace::{ops, StepActive};
        let to_install: Vec<(u8, Vec<u8>)> = {
            let t = self.traces.borrow();
            t.cmd_traces
                .iter()
                .filter(|c| c.name == fqn && (c.ops & ops::STEP_ANY) != 0)
                .filter(|c| {
                    !t.step_active
                        .iter()
                        .any(|s| s.owner == fqn && s.command == c.command)
                })
                .map(|c| (c.ops & ops::STEP_ANY, c.command.clone()))
                .collect()
        };
        let n = to_install.len();
        let mut tt = self.traces.borrow_mut();
        for (ops_bits, command) in to_install {
            tt.step_active.push(StepActive {
                owner: fqn.to_vec(),
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
        let code_str = code.map(|c| c.as_int().to_string().into_bytes());
        let op_label: &[u8] = if is_enter { b"enterstep" } else { b"leavestep" };

        self.traces.borrow_mut().exec_firing += 1;
        let mut outcome: Option<Code> = None;
        for cmd in cmds {
            let args = if is_enter {
                crate::list::new_list_obj(&[new_string(cmd_word), new_string(op_label)])
            } else {
                let result_bytes = obj_bytes(self.result.get());
                crate::list::new_list_obj(&[
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
            let c = self.eval_str(&line);
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
                None
            }
        }
    }

    /// The original resolve→invoke→unknown dispatch (trace-free).
    fn dispatch_inner(&mut self, argv: &[*mut TclObj]) -> Code {
        let name = obj_bytes(argv[0]);
        // The availability gate lives in `resolve_dispatchable`, so a builtin
        // the emulated release does not carry misses here and falls through to
        // the `unknown` machinery below like any other unresolved name.
        if let Some(cmd) = self.resolve_dispatchable(self.current_ns.get(), &name) {
            return self.invoke(cmd, argv);
        }
        // Inside an `oo::define`/`oo::objdefine` body, an unresolved leading word
        // may be a definition subcommand (an abbreviation, or one without a
        // global builtin); resolve it as C's define ensemble would.
        if self.in_oo_define() {
            if let Some(code) = self.oo_define_command(&name, argv) {
                return code;
            }
        }
        // Command miss: dispatch through the current namespace's `namespace
        // unknown` handler if it has a custom one, else the global `unknown`
        // command (and only if we're not already resolving `unknown` itself).
        if name != b"unknown" {
            // A custom unknown handler is a command *prefix* (a list), invoked as
            // `handler… name args…`. The current namespace's handler wins; a
            // namespace with none falls back to the global namespace's (which a
            // script can set to override `::unknown`), else the `unknown` command.
            let ns_handler = {
                let ns = self.namespaces.borrow();
                let cur = self.current_ns.get();
                ns.unknown_handler(cur).map(<[u8]>::to_vec).or_else(|| {
                    (cur != GLOBAL)
                        .then(|| ns.unknown_handler(GLOBAL).map(<[u8]>::to_vec))
                        .flatten()
                })
            };
            if let Some(handler) = ns_handler {
                if let Ok(prefix) = crate::parse::split_list(&handler) {
                    let mut new_argv: Vec<*mut TclObj> =
                        Vec::with_capacity(prefix.len() + argv.len());
                    for w in &prefix {
                        let o = new_string(w);
                        unsafe { obj::incr_ref_count(o) };
                        new_argv.push(o);
                    }
                    for &a in argv {
                        unsafe { obj::incr_ref_count(a) };
                        new_argv.push(a);
                    }
                    let code = self.dispatch(&new_argv);
                    release_all(&new_argv);
                    return code;
                }
            }
            let unk = self.resolve_dispatchable(GLOBAL, b"unknown");
            if let Some(unk) = unk {
                let mut new_argv: Vec<*mut TclObj> = Vec::with_capacity(argv.len() + 1);
                let head = new_string(b"unknown");
                // SAFETY: fresh + live argv elements; take the owning +1.
                unsafe { obj::incr_ref_count(head) };
                new_argv.push(head);
                for &a in argv {
                    unsafe { obj::incr_ref_count(a) };
                    new_argv.push(a);
                }
                let code = self.invoke(unk, &new_argv);
                release_all(&new_argv);
                return code;
            }
        }
        self.invalid_command(&name)
    }

    /// Invoke an already-resolved command handle with `argv`.
    fn invoke(&mut self, cmd: Command, argv: &[*mut TclObj]) -> Code {
        match cmd {
            Command::Builtin(f) => f(self, argv),
            Command::Alias { target, prefix, .. } => self.dispatch_alias(&target, &prefix, argv),
            Command::Imported {
                source, ensemble, ..
            } => {
                if let Some(token) = ensemble {
                    if !token.is_deleted() {
                        return self.dispatch_ensemble(&token, argv);
                    }
                }
                // Gated resolution: a source the emulated release does not carry
                // is a miss, so an imported spelling cannot smuggle a hidden
                // builtin past the surface check (PR #1481 review).
                match self.resolve_dispatchable(GLOBAL, &source) {
                    // Transparent redirect: forward argv unchanged to the source.
                    Some(cmd) => self.invoke(cmd, argv),
                    None => self.invalid_command(&source),
                }
            }
            Command::Ensemble(token) => self.dispatch_ensemble(&token, argv),
            Command::Proc(def) => self.call_proc(&def, argv),
            Command::ChildInterp(name) => self.dispatch_child(&name, argv),
            Command::OoObject(fqn) => self.oo_dispatch(&fqn, argv),
            Command::ParentAlias { target, prefix, .. } => {
                self.dispatch_parent_alias(&target, &prefix, argv)
            }
        }
    }

    /// Register `cmd` under the (possibly qualified) name `name` — for the OO
    /// object/class commands.
    pub(crate) fn ns_register(&mut self, name: &[u8], cmd: Command) {
        // The TclOO root marking is an identity, not a reservation on the
        // *name*: it says "the engine installed this entry on the registry's
        // behalf, so date it by the registry". Registering over that name
        // replaces the identity with a script-created one, which is
        // release-invariant like any proc, so the marking must not outlive the
        // entry it described — otherwise the availability gate hides the new
        // command forever.
        //
        // This lives in the single registration funnel rather than at the
        // individual creation verbs so that every path is covered: `create`,
        // `new`, `oo::copy`, `rename` onto the name, and any funnel added
        // later. (The VM is immune for the same structural reason — its clear
        // lives in `register_command`.) Safe against the engine's own installs
        // because each declares its root *after* registering it.
        self.forget_registry_object_root(name);
        self.namespaces.borrow_mut().register(name, cmd);
        self.invalidate_command_environment();
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
        // engine and the VM cannot drift on a seeded stream (#1432). What
        // stays here is the seed *storage* and the nondeterministic
        // first-seed policy.
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
            // A coroutine resume command and the per-object `my`/`myclass`
            // commands all register as builtins but report their own cmdType
            // (C's per-command registrations).
            Command::Builtin(_) => {
                let fqn = self
                    .namespaces
                    .borrow()
                    .resolve_fqn(self.current_ns.get(), name);
                match fqn {
                    Some(fqn) if self.coros.borrow().contains_key(&fqn) => b"coroutine",
                    Some(fqn) => self.oo_private_cmd_kind(&fqn).unwrap_or(b"native"),
                    None => b"native",
                }
            }
            Command::Proc(_) => b"proc",
            Command::Alias { .. } | Command::ParentAlias { .. } => b"alias",
            Command::Imported { .. } => b"import",
            Command::Ensemble(_) => b"ensemble",
            Command::OoObject(_) => b"object",
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
    fn child_wrong_args(&mut self, argv: &[*mut TclObj], tail: &[u8]) -> Code {
        let mut message = b"wrong # args: should be \"".to_vec();
        message.extend_from_slice(&invoked_word(argv));
        message.extend_from_slice(tail);
        message.push(b'"');
        self.error(&message)
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
        match obj_bytes(argv[1]).as_slice() {
            b"eval" => {
                if argv.len() < 3 {
                    return self.child_wrong_args(argv, b" eval arg ?arg ...?");
                }
                let script = join_words(&argv[2..]);
                self.eval_in_child(name, &script)
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
                let hide = obj_bytes(argv[1]) == b"hide";
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
                self.set_result(crate::list::new_list_obj(&elems));
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
                self.set_result(crate::list::new_list_obj(&elems));
                Code::Ok
            }
            // `$child alias srcCmd targetCmd ?arg ...?` — a cross-interp alias in
            // the child delegating to `targetCmd` in this (parent) interp. (The
            // target is implicitly the parent, so there is no target-path arg.)
            b"alias" if argv.len() >= 4 => {
                let alias = obj_bytes(argv[2]);
                let target = obj_bytes(argv[3]);
                let prefix: Vec<Vec<u8>> = argv[4..].iter().map(|&a| obj_bytes(a)).collect();
                self.install_parent_alias(name, &alias, target, prefix);
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
                        self.set_result_bytes(&h);
                        Code::Ok
                    }
                    Some(Err(m)) => self.error(&m),
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
                    Some(Err(m)) => self.error(&m),
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
                    Some(Err(m)) => self.error(&m),
                    None => self.error(b"could not find interpreter"),
                }
            }
            other => {
                // Same `bad option` shape as the `interp` ensemble's own
                // fallthrough (`cmd_alias.rs::interp_cmd`) — the child
                // command object (`NRChildCmd`, tclInterp.c) advertises a
                // *shorter* list than `interp` does (no `children`,
                // `create`, `delete`, or `exists`: those are only ever
                // spelled `interp <op> path`, never `$child <op>`), but the
                // shape is the same tclsh `bad option` error, not a
                // runtime-specific message (issue #1412 item 7).
                let mut m = b"bad option \"".to_vec();
                m.extend_from_slice(other);
                m.extend_from_slice(
                    b"\": must be alias, aliases, bgerror, debug, eval, expose, \
                      hide, hidden, issafe, invokehidden, limit, marktrusted, \
                      or recursionlimit",
                );
                self.error(&m)
            }
        }
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
        let mut child = Interp::with_host(self.host());
        // A child interpreter is another interpreter of the *same* Tcl build,
        // not a different release — C compiles one library in, so every child
        // reports and behaves as its parent's release. Inherited before the
        // globals are written, so the child's `tcl_version`/`tcl_patchLevel`
        // and its namespace-scope variable resolution both agree with the
        // parent (issue #1328). Resolution still runs against the child's
        // *own* global namespace: the rule is shared, the variables are not.
        // The whole profile is inherited, not just the release, so a child's
        // command-surface availability gate agrees too (issue #1463).
        child.set_dialect_profile(self.dialect_profile());
        // `Interp::new` already gave the child its own predefined globals
        // (`tcl_platform`, `env`, argv, …). The full `init.tcl`
        // (package/auto-load) remains deferred.
        // `interp debug -frame` is seeded from the creating interp's
        // `env(TCL_INTERP_DEBUG_FRAME)` (C's `Tcl_CreateChild`).
        if self
            .var_get_elem(b"env", b"TCL_INTERP_DEBUG_FRAME")
            .map(|o| crate::typed_value::boolean(o).unwrap_or(false))
            .unwrap_or(false)
        {
            child.0.debug_frame.set(true);
            child.invalidate_interpreter_policy();
        }
        self.children.borrow_mut().insert(name.clone(), child);
        self.namespaces
            .borrow_mut()
            .register(&name, Command::ChildInterp(name.clone()));
        name
    }

    /// Whether a child interpreter `name` exists.
    pub(crate) fn child_exists(&self, name: &[u8]) -> bool {
        self.children.borrow().contains_key(name)
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
    /// a delete requested *during* `f` (the self-deleting `exit` alias) is
    /// deferred to here and applied once no eval of it remains on the stack.
    pub(crate) fn with_child<R>(
        &mut self,
        name: &[u8],
        f: impl FnOnce(&mut Interp) -> R,
    ) -> Option<R> {
        let mut child = self.children.borrow().get(name)?.clone();
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
        let teardown = child.pending_delete.get() && child.eval_active.get() == 0;
        drop(child); // release our handle clone before freeing the table's
        if teardown {
            self.invalidate_interpreter_policy();
            self.children.borrow_mut().remove(name);
            self.namespaces.borrow_mut().delete(GLOBAL, name);
            self.invalidate_command_environment();
        }
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
            Some(cmd) => {
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
                self.namespaces.borrow_mut().take(GLOBAL, name);
                let mut hidden_fqn = b"::".to_vec();
                hidden_fqn.extend_from_slice(hidden_name);
                if let Some(old_fqn) = &old_fqn {
                    self.move_cmd_traces(old_fqn, &hidden_fqn);
                }
                if let Command::Ensemble(token) = &cmd {
                    if let Some(old_fqn) = old_fqn {
                        token.rename(hidden_fqn.clone());
                        self.retarget_import_sources(&old_fqn, &hidden_fqn);
                    } else {
                        token.rename(hidden_fqn);
                    }
                }
                self.hidden.borrow_mut().insert(hidden_name.to_vec(), cmd);
                self.invalidate_command_environment();
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
        let cmd = self.hidden.borrow_mut().remove(hidden_name);
        match cmd {
            Some(cmd) => {
                let mut old_hidden_fqn = b"::".to_vec();
                old_hidden_fqn.extend_from_slice(hidden_name);
                let old_fqn = match &cmd {
                    Command::Ensemble(token) => Some(token.name()),
                    _ => None,
                };
                self.namespaces.borrow_mut().register(name, cmd);
                let new_fqn = self
                    .namespaces
                    .borrow()
                    .resolve_fqn(GLOBAL, name)
                    .unwrap_or_else(|| self.fqn_for(name));
                self.move_cmd_traces(&old_hidden_fqn, &new_fqn);
                if let Some(old_fqn) = old_fqn {
                    self.retarget_import_sources(&old_fqn, &new_fqn);
                }
                self.invalidate_command_environment();
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
        let cmd = self.hidden.borrow().get(name).cloned();
        match cmd {
            Some(cmd) => self.invoke(cmd, argv),
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

    /// Move every import's by-name source shadow across a source rename. Hidden
    /// imports carry the same metadata as visible ones and must move too.
    fn retarget_import_sources(&mut self, old_fqn: &[u8], new_fqn: &[u8]) {
        self.namespaces
            .borrow_mut()
            .retarget_imports(old_fqn, new_fqn);
        for command in self.hidden.borrow_mut().values_mut() {
            if let Command::Imported { source, .. } = command {
                if source.as_slice() == old_fqn {
                    *source = new_fqn.to_vec();
                }
            }
        }
    }

    /// Retarget imports at an occupied source binding to a newly-created
    /// ensemble token. This covers visible and hidden imports, and deliberately
    /// does not mutate the retired token.
    fn retarget_imports_to_ensemble(
        &mut self,
        source_fqn: &[u8],
        old: Option<&Rc<crate::ensemble::EnsembleToken>>,
        new: &Rc<crate::ensemble::EnsembleToken>,
    ) {
        self.namespaces
            .borrow_mut()
            .retarget_imports_to_ensemble(source_fqn, old, new);
        for command in self.hidden.borrow_mut().values_mut() {
            let Command::Imported {
                source, ensemble, ..
            } = command
            else {
                continue;
            };
            let retains_old = old.is_some_and(|old| {
                ensemble
                    .as_ref()
                    .is_some_and(|token| Rc::ptr_eq(token, old))
            });
            if source.as_slice() == source_fqn || retains_old {
                *source = source_fqn.to_vec();
                *ensemble = Some(Rc::clone(new));
            }
        }
    }

    /// Remove one imported command by stable identity wherever a delete-trace
    /// callback may have renamed or hidden it. A callback replacement/re-import
    /// has a new identity and is deliberately left alone.
    fn remove_import_identity(&mut self, identity: &Rc<ImportToken>) -> Option<Vec<u8>> {
        if let Some(fqn) = self
            .namespaces
            .borrow_mut()
            .remove_import_identity(identity)
        {
            return Some(fqn);
        }
        let hidden_name = self.hidden.borrow().iter().find_map(|(name, command)| {
            matches!(
                command,
                Command::Imported { identity: current, .. }
                    if Rc::ptr_eq(current, identity)
            )
            .then(|| name.clone())
        })?;
        self.hidden.borrow_mut().remove(&hidden_name);
        let mut fqn = b"::".to_vec();
        fqn.extend_from_slice(&hidden_name);
        Some(fqn)
    }

    /// Remove one ensemble by stable token identity after running the delete
    /// trace at the name where deletion began. The callback may rename, hide,
    /// expose, or replace the command; only the captured token is retired, and
    /// any trace sidecar moved with it is dropped without firing twice.
    fn retire_ensemble_identity(
        &mut self,
        trace_fqn: &[u8],
        identity: &Rc<crate::ensemble::EnsembleToken>,
    ) -> Option<Vec<u8>> {
        self.on_command_replaced(trace_fqn);
        let removed_fqn = self
            .namespaces
            .borrow_mut()
            .remove_ensemble_identity(identity)
            .or_else(|| {
                let hidden_name = self.hidden.borrow().iter().find_map(|(name, command)| {
                    matches!(
                        command,
                        Command::Ensemble(current) if Rc::ptr_eq(current, identity)
                    )
                    .then(|| name.clone())
                })?;
                self.hidden.borrow_mut().remove(&hidden_name);
                let mut fqn = b"::".to_vec();
                fqn.extend_from_slice(&hidden_name);
                Some(fqn)
            });
        if let Some(live_fqn) = removed_fqn.as_deref() {
            if live_fqn != trace_fqn {
                self.remove_cmd_traces(live_fqn);
            }
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
        origins: impl IntoIterator<Item = Vec<u8>>,
        tokens: &[Rc<crate::ensemble::EnsembleToken>],
    ) {
        let mut origins: std::collections::HashSet<Vec<u8>> = origins.into_iter().collect();
        loop {
            let visible = self
                .namespaces
                .borrow()
                .imports_for_origins(&origins, tokens);
            let hidden: Vec<(Vec<u8>, Rc<ImportToken>)> = self
                .hidden
                .borrow()
                .iter()
                .filter_map(|(name, command)| {
                    let Command::Imported {
                        source,
                        ensemble,
                        identity,
                    } = command
                    else {
                        return None;
                    };
                    let retains_token = ensemble.as_ref().is_some_and(|imported| {
                        tokens.iter().any(|victim| Rc::ptr_eq(imported, victim))
                    });
                    if !origins.contains(source) && !retains_token {
                        return None;
                    }
                    let mut fqn = b"::".to_vec();
                    fqn.extend_from_slice(name);
                    Some((fqn, Rc::clone(identity)))
                })
                .collect();

            let mut removed_any = false;
            for (fqn, identity) in visible {
                self.on_command_replaced(&fqn);
                if let Some(removed_fqn) = self.remove_import_identity(&identity) {
                    if removed_fqn != fqn {
                        self.remove_cmd_traces(&removed_fqn);
                    }
                    origins.insert(removed_fqn);
                    removed_any = true;
                }
            }
            for (fqn, identity) in hidden {
                self.on_command_replaced(&fqn);
                if let Some(removed_fqn) = self.remove_import_identity(&identity) {
                    if removed_fqn != fqn {
                        self.remove_cmd_traces(&removed_fqn);
                    }
                    origins.insert(removed_fqn);
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
    /// `interp create -safe`. The Safe Base's re-aliasing of `source`/`load`/
    /// `file` is a follow-up (needs cross-interp aliases).
    pub(crate) fn make_safe(&mut self) {
        // Variable unsets below can fire callbacks. Stale existing tokens before
        // the first visibility/policy write, not after re-entrant code can run.
        self.invalidate_interpreter_policy();
        // The hide list is the registry's `Traits::SAFE_INTERP_HIDDEN` query,
        // not a name list this engine keeps (ledger row B2): C's own set is
        // the `CmdInfo` rows lacking `CMD_IS_SAFE` plus the whole-command rows
        // of `unsafeEnsembleCommands`, and that is what the trait records.
        // `hide_command` returns `false` for a name this interpreter does not
        // carry, which is the per-release narrowing: `unload` (8.5+) and
        // `zipfs` (9.0+) are release-gated and simply are not there under an
        // older pin, so no second availability rule is needed.
        //
        // `after` / `vwait` are correctly absent from the trait — confirmed
        // present and callable inside a real safe child on tclsh 8.6.14
        // (`s eval {info commands after}` returns `after`); an earlier
        // hand-typed list here once hid them, breaking legitimate safe-interp
        // code using `after idle` / `after cancel`.
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
    pub(crate) fn debug_apply(&self, opts: &[*mut TclObj]) -> Result<*mut TclObj, Vec<u8>> {
        let frame_byte: &[u8] = if self.debug_frame.get() { b"1" } else { b"0" };
        match opts.len() {
            0 => Ok(dict_obj(&[(b"-frame", frame_byte.to_vec())])),
            1 => {
                check_debug_opt(opts[0])?;
                Ok(obj::new_string_bytes(frame_byte))
            }
            _ => {
                check_debug_opt(opts[0])?;
                if crate::typed_value::boolean(opts[1]).map_err(|e| e.message)? {
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
    ) -> Result<*mut TclObj, Vec<u8>> {
        match ltype {
            b"commands" => self.limit_commands(opts),
            b"time" => self.limit_time(opts),
            other => {
                let mut m = b"bad limit type \"".to_vec();
                m.extend_from_slice(other);
                m.extend_from_slice(b"\": must be commands or time");
                Err(m)
            }
        }
    }

    fn limit_commands(&self, opts: &[*mut TclObj]) -> Result<*mut TclObj, Vec<u8>> {
        const OPTS: &[&[u8]] = &[b"-command", b"-granularity", b"-value"];
        if opts.is_empty() {
            let l = self.limits.borrow();
            return Ok(dict_obj(&[
                (b"-command", l.cmd_command.clone()),
                (b"-granularity", l.cmd_granularity.to_string().into_bytes()),
                (b"-value", opt_int(l.cmd_value)),
            ]));
        }
        if opts.len() == 1 {
            let opt = resolve_limit_opt(&obj_bytes(opts[0]), OPTS)?;
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
            return Err(
                b"wrong # args: should be \"interp limit path commands ?-option value ...?\""
                    .to_vec(),
            );
        }
        // The option loop may commit an earlier pair before a later pair is
        // rejected. Invalidate before its first possible policy write so that
        // partial-on-error mutation cannot leave an old token live.
        self.invalidate_interpreter_policy();
        let mut i = 0;
        while i + 1 < opts.len() {
            let opt = resolve_limit_opt(&obj_bytes(opts[i]), OPTS)?;
            let val = obj_bytes(opts[i + 1]);
            match opt.as_slice() {
                b"-command" => self.limits.borrow_mut().cmd_command = val,
                b"-granularity" => {
                    let n = parse_limit_int(&val)?;
                    if n < 1 {
                        return Err(b"granularity must be at least 1".to_vec());
                    }
                    self.limits.borrow_mut().cmd_granularity = n;
                }
                _ => {
                    let n = parse_limit_int(&val)?;
                    if n < 0 {
                        return Err(b"command limit value must be at least 0".to_vec());
                    }
                    self.limits.borrow_mut().cmd_value = Some(n);
                }
            }
            i += 2;
        }
        Ok(obj::new_string_bytes(b""))
    }

    fn limit_time(&self, opts: &[*mut TclObj]) -> Result<*mut TclObj, Vec<u8>> {
        const OPTS: &[&[u8]] = &[b"-command", b"-granularity", b"-milliseconds", b"-seconds"];
        if opts.is_empty() {
            let l = self.limits.borrow();
            let (secs, millis) = match l.time_value {
                Some((s, m)) => (s.to_string().into_bytes(), m.to_string().into_bytes()),
                None => (Vec::new(), Vec::new()),
            };
            return Ok(dict_obj(&[
                (b"-command", l.time_command.clone()),
                (b"-granularity", l.time_granularity.to_string().into_bytes()),
                (b"-milliseconds", millis),
                (b"-seconds", secs),
            ]));
        }
        if opts.len() == 1 {
            let opt = resolve_limit_opt(&obj_bytes(opts[0]), OPTS)?;
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
            return Err(
                b"wrong # args: should be \"interp limit path time ?-option value ...?\"".to_vec(),
            );
        }
        // As with command limits, parsing is incremental and can partially
        // mutate before returning an error on a later option.
        self.invalidate_interpreter_policy();
        let (mut sec, mut ms) = self.limits.borrow().time_value.unwrap_or((0, 0));
        let mut touched = self.limits.borrow().time_value.is_some();
        let mut i = 0;
        while i + 1 < opts.len() {
            let opt = resolve_limit_opt(&obj_bytes(opts[i]), OPTS)?;
            let val = obj_bytes(opts[i + 1]);
            match opt.as_slice() {
                b"-command" => self.limits.borrow_mut().time_command = val,
                b"-granularity" => {
                    let n = parse_limit_int(&val)?;
                    if n < 1 {
                        return Err(b"granularity must be at least 1".to_vec());
                    }
                    self.limits.borrow_mut().time_granularity = n;
                }
                b"-seconds" => {
                    let n = parse_limit_int(&val)?;
                    if n < 0 {
                        return Err(b"seconds must be non-negative".to_vec());
                    }
                    sec = n;
                    touched = true;
                }
                _ => {
                    let n = parse_limit_int(&val)?;
                    if n < 0 {
                        return Err(b"milliseconds must be non-negative".to_vec());
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
    /// If the child is currently executing (a self-deleting `exit`/`interp
    /// delete` from inside its own eval), the actual teardown is **deferred**:
    /// the command binding is removed now so the name stops dispatching, but the
    /// interp handle is freed only when its last eval unwinds
    /// ([`with_child`]/[`eval_in_child`]) — never while a re-entrant eval of it is
    /// still on the stack. Mirrors C's deferred `Tcl_DeleteInterp`.
    ///
    /// [`with_child`]: Interp::with_child
    /// [`eval_in_child`]: Interp::eval_in_child
    pub(crate) fn delete_child(&mut self, name: &[u8]) -> bool {
        // Classify under a short `children` borrow, then act (re-borrowing) so we
        // never hold `children` across the `namespaces` mutation.
        let disposition = {
            let children = self.children.borrow();
            match children.get(name) {
                Some(child) if child.eval_active.get() > 0 => {
                    child.invalidate_interpreter_policy();
                    child.pending_delete.set(true);
                    Some(false) // busy: defer the removal
                }
                Some(_) => Some(true), // idle: remove now
                None => None,
            }
        };
        match disposition {
            None => false,
            Some(remove_now) => {
                self.invalidate_interpreter_policy();
                if remove_now {
                    self.children.borrow_mut().remove(name);
                }
                self.namespaces.borrow_mut().delete(GLOBAL, name);
                self.invalidate_command_environment();
                true
            }
        }
    }

    /// Evaluate `script` in child interpreter `name`, copying its result/code
    /// back. `None`-returning if the child doesn't exist (caller raises).
    ///
    /// Runs through [`with_child`](Self::with_child), so a parent command invoked
    /// *during* this eval — via a child→parent alias — can re-enter the same child
    /// (`interp invokehidden $child …`), which the Safe Base relies on.
    pub(crate) fn eval_in_child(&mut self, name: &[u8], script: &[u8]) -> Code {
        match self.with_child(name, |c| (c.eval_str(script), c.result_bytes())) {
            Some((code, result)) => {
                self.set_result_bytes(&result);
                code
            }
            None => {
                let mut m = b"could not find interpreter \"".to_vec();
                m.extend_from_slice(name);
                m.push(b'"');
                self.error(&m)
            }
        }
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
        argv: &[*mut TclObj],
    ) -> Code {
        let Some(parent_state) = self.parent.borrow().upgrade() else {
            return self.error(b"cannot invoke a parent alias from the root interpreter");
        };
        if CROSS_INTERP_DEPTH.with(|d| d.get()) >= MAX_CROSS_INTERP_DEPTH {
            return self.error(b"too many nested cross-interpreter calls");
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
        let mut parent = Interp(parent_state);
        let code = parent.dispatch(&new_argv);
        let res = parent.result_bytes();
        CROSS_INTERP_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
        release_all(&new_argv);
        self.set_result_bytes(&res);
        code
    }

    /// Call a user proc (`TclObjInterpProc`): a thin wrapper over [`run_proc`]
    /// with the proc's `(params, body, ns)` and the call args (`argv[1..]`).
    ///
    /// [`run_proc`]: Interp::run_proc
    fn call_proc(&mut self, def: &ProcDef, argv: &[*mut TclObj]) -> Code {
        let name = obj_bytes(argv[0]);
        self.run_proc(
            &def.params,
            &def.body,
            def.ns,
            &argv[1..],
            &name,
            CallMeta {
                err: ProcFrame::Proc(&name),
                fqn: Some(&def.fqn),
                source: def.source.clone(),
                body_line_base: def.body_line_base,
                link_vars: &[],
                keep_loop_codes: false,
                same_level: false,
                usage_prefix: None,
                level_words: None,
                quote_name: true,
            },
        )
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
    pub(crate) fn run_proc(
        &mut self,
        params: &[Param],
        body: &[u8],
        ns: NsId,
        call_args: &[*mut TclObj],
        usage_called: &[u8],
        meta: CallMeta,
    ) -> Code {
        let has_args = params.last().is_some_and(|p| p.name == b"args");
        let positional = if has_args {
            &params[..params.len() - 1]
        } else {
            params
        };
        // The `wrong # args` command prefix (an OO method passes the invoking
        // `obj method`; everything else uses the invoked name).
        let usage = meta.usage_prefix.as_deref().unwrap_or(usage_called);
        // Arity (defaults assumed trailing — the common shape): supplied must
        // cover the no-default params, and not exceed the positionals unless an
        // `args` catch-all soaks up the rest.
        let supplied = call_args.len();
        let required = positional.iter().filter(|p| p.default.is_none()).count();
        if supplied < required || (!has_args && supplied > positional.len()) {
            return self.error(&self.proc_wrong_args(usage, params, supplied, meta.quote_name));
        }
        // Recursion bound (catchable, not a stack overflow).
        if self.recursion_depth.get() >= self.recursion_limit.get() {
            return self.error(b"too many nested evaluations (infinite loop?)");
        }
        self.recursion_depth.set(self.recursion_depth.get() + 1);

        if meta.same_level {
            self.frames.borrow_mut().push_same_level(ns);
        } else {
            self.frames.borrow_mut().push(ns);
        }
        // Record the invocation words for `info level N`: an OO constructor
        // supplies the `create`/`new` invocation words verbatim; otherwise the
        // invoked name plus the supplied arguments.
        let words = meta.level_words.unwrap_or_else(|| {
            let mut w = Vec::with_capacity(call_args.len() + 1);
            w.push(usage_called.to_vec());
            w.extend(call_args.iter().map(|&a| obj_bytes(a)));
            w
        });
        self.frames.borrow_mut().set_words(words);
        let saved_ns = self.current_ns.get();
        self.current_ns.set(ns);

        // Pre-link a TclOO method's declared instance variables: each name in
        // the frame becomes a link to the object's namespace variable (`ns`), so
        // the method sees instance state without an explicit `variable`.
        for (local, target) in meta.link_vars {
            self.make_variable_mapped(ns, local, target);
        }

        // Bind positionals left-to-right: the supplied arg, else the default.
        // Binding is purely positional — a defaulted parameter does *not* yield
        // its slot to a later one, so a required parameter reached with no
        // supplied arg (a non-trailing default, e.g. `proc p {a {b 2} c}` called
        // with 2 args) is a `wrong # args` error, matching tclsh 9.0.
        for (i, p) in positional.iter().enumerate() {
            let stored = if i < call_args.len() {
                self.var_set(&p.name, call_args[i])
            } else if let Some(def) = &p.default {
                // A fresh rc-0 default; `var_set` retains it, so on the error
                // path it must be dropped.
                let o = new_string(def);
                match self.var_set(&p.name, o) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        drop_fresh(o);
                        Err(e)
                    }
                }
            } else {
                self.frames.borrow_mut().pop();
                self.current_ns.set(saved_ns);
                self.recursion_depth.set(self.recursion_depth.get() - 1);
                return self.error(&self.proc_wrong_args(usage, params, supplied, meta.quote_name));
            };
            if stored.is_err() {
                self.frames.borrow_mut().pop();
                self.current_ns.set(saved_ns);
                self.recursion_depth.set(self.recursion_depth.get() - 1);
                return self.error(b"proc parameter binding failed");
            }
        }
        // The `args` catch-all: a list of the remaining args. Clamp the split
        // point — when trailing positionals took their defaults, fewer args were
        // supplied than there are positionals, so `args` is simply empty.
        if has_args {
            let rest = &call_args[positional.len().min(call_args.len())..];
            let list = crate::list::new_list_obj(rest); // rc 0; var_set retains
            if self.var_set(b"args", list).is_err() {
                drop_fresh(list);
            }
        }

        // The proc body runs as its own `info frame` level: `type proc` (or
        // `source` if defined in a sourced file), the proc FQN, and the new call
        // level (set after `frames.push`, so `current_level` is the proc's).
        // A TclOO method body carries its method context for `info frame`
        // (`method`/`class`|`object`), which displaces the `proc` key.
        let oo = match &meta.err {
            ProcFrame::Method { kind, owner, what } => {
                let method = match what {
                    MethodFrameWhat::Named(n) => n.to_vec(),
                    MethodFrameWhat::Constructor | MethodFrameWhat::Destructor => Vec::new(),
                };
                Some((method, kind.to_vec(), owner.to_vec()))
            }
            _ => None,
        };
        // An `apply` lambda reports `lambda <expr>` (not `proc`) in `info frame`.
        let lambda = match &meta.err {
            ProcFrame::Lambda(expr) => Some(expr.to_vec()),
            _ => None,
        };
        let (proc_lvl, proc_idx) = {
            let f = self.frames.borrow();
            (f.current_level(), f.current_frame_index())
        };
        let proc_frame = CmdFrame {
            kind: if meta.source.is_some() {
                FrameKind::Source
            } else {
                FrameKind::Proc
            },
            file: meta.source,
            proc: meta.fqn.map(<[u8]>::to_vec),
            level: proc_lvl,
            omit_level: false,
            frame_index: proc_idx,
            line_base: meta.body_line_base,
            proc_line_base: meta.body_line_base,
            cmd: Vec::new(),
            line: 1,
            oo,
            lambda,
        };
        let code = self.eval_framed(body, proc_frame);
        // The frame's local variables (and any traces on them) die with it.
        let proc_level = self.frames.borrow().current_level();
        // Capture `[info level 0]` (the invocation words) before the frame is
        // popped — the TIP 348 `CALL` entry if this body unwinds with an error.
        let call_words = self.level_words(proc_level);
        // The locals' unset traces are collected while the frame — and its
        // arrays' elements — still exist, and fire once it is gone, as C's
        // `TclDeleteVars` runs over a frame that is on its way out.
        let teardown = if self.traces.borrow().traces.is_empty() {
            Vec::new()
        } else {
            self.frame_teardown_unset_traces(proc_level)
        };
        self.frames.borrow_mut().pop();
        if !self.traces.borrow().traces.is_empty() {
            self.clear_frame_var_traces(proc_level);
        }
        self.current_ns.set(saved_ns);
        self.fire_unset_callbacks(teardown);
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
                // TIP 348: record this proc/lambda/method frame as a `CALL` entry.
                if let Some(words) = call_words {
                    self.error_stack_push_call(&words);
                }
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
                let mut m = b"wrong # args: should be \"".to_vec();
                m.extend_from_slice(&obj_bytes(argv[0]));
                for p in &cfg.parameters {
                    m.push(b' ');
                    crate::list::append_list_element(&mut m, p, false);
                }
                m.extend_from_slice(b" subcommand ?arg ...?\"");
                return self.error(&m);
            };
            let sub = obj_bytes(argv[layout.subcommand]);
            let subs = self.ensemble_subcommands(cfg);
            if let Some(idx) = tcl_cmd_core::ensemble::resolve_subcommand(&subs, &sub, cfg.prefixes)
            {
                let resolved = &subs[idx];
                // The target command prefix: a `-map` entry, else `<ns>::<sub>`.
                let mapped = cfg.map.as_ref().and_then(|m| {
                    m.iter()
                        .find(|(k, _)| k == resolved)
                        .map(|(_, p)| p.clone())
                });
                let default_target = mapped.is_none();
                let prefix: Vec<Vec<u8>> = mapped.unwrap_or_else(|| {
                    let mut t = self.namespaces.borrow().qualified_name(cfg.ns);
                    if cfg.ns != GLOBAL {
                        t.extend_from_slice(b"::");
                    }
                    t.extend_from_slice(resolved);
                    vec![t]
                });
                // Spell-fix the subcommand to its resolved name in the recorded
                // source, so an abbreviated `ev` is reported as `event` (C's
                // `TclSpellFix`).
                let mut source: Vec<Vec<u8>> = argv.iter().map(|&a| obj_bytes(a)).collect();
                source[layout.subcommand] = resolved.clone();
                return self.dispatch_ensemble_target(
                    &prefix,
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
                            let mut m = b"wrong # args: should be \"".to_vec();
                            m.extend_from_slice(&obj_bytes(argv[0]));
                            for parameter in &live.parameters {
                                m.push(b' ');
                                crate::list::append_list_element(&mut m, parameter, false);
                            }
                            m.extend_from_slice(b" subcommand ?arg ...?\"");
                            return self.error(&m);
                        };
                        let source: Vec<Vec<u8>> = argv.iter().map(|&a| obj_bytes(a)).collect();
                        return self.dispatch_ensemble_target(
                            &prefix,
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
            let ecode = error_code_list(&[b"TCL", b"LOOKUP", b"SUBCOMMAND", &sub]);
            let ns_fqn = self.namespaces.borrow().qualified_name(cfg.ns);
            let m = tcl_cmd_core::ensemble::unknown_subcommand_message(
                &subs,
                &sub,
                cfg.prefixes,
                &ns_fqn,
            );
            return self.error_with_code(&m, &ecode);
        }
    }

    /// The ensemble's valid subcommand set (sorted, deduped): explicit
    /// `-subcommands`, else the `-map` keys, else the namespace's exports.
    fn ensemble_subcommands(&self, cfg: &crate::ensemble::EnsembleConfig) -> Vec<Vec<u8>> {
        let mut subs: Vec<Vec<u8>> = match (&cfg.subcommands, &cfg.map) {
            (Some(list), _) => list.clone(),
            (None, Some(map)) => map.iter().map(|(k, _)| k.clone()).collect(),
            (None, None) => self.namespaces.borrow().exported_commands(cfg.ns),
        };
        subs.sort();
        subs.dedup();
        subs
    }

    /// Dispatch a resolved ensemble subcommand: `[prefix…, params…, rest…]` (the
    /// `-parameters` values thread in right after the target prefix; the
    /// subcommand's own args follow). `layout` is always computed by the shared
    /// ensemble owner from the live token configuration.
    fn dispatch_ensemble_target(
        &mut self,
        prefix: &[Vec<u8>],
        argv: &[*mut TclObj],
        layout: &tcl_cmd_core::ensemble::InvocationLayout,
        source: Vec<Vec<u8>>,
        default_name: Option<&[u8]>,
    ) -> Code {
        let default_target_was_missing = default_name.is_some()
            && self
                .resolve_cmd_fqn(prefix.first().map_or(b"", Vec::as_slice))
                .is_none();
        let mut new_argv: Vec<*mut TclObj> = Vec::with_capacity(prefix.len() + argv.len() - 1);
        for w in prefix {
            let o = new_string(w);
            // SAFETY: fresh obj; take the owning +1 the new argv holds.
            unsafe { obj::incr_ref_count(o) };
            new_argv.push(o);
        }
        for &a in &argv[layout.parameters.clone()] {
            // SAFETY: live arg; take an owning +1.
            unsafe { obj::incr_ref_count(a) };
            new_argv.push(a);
        }
        for &a in &argv[layout.arguments..] {
            // SAFETY: live arg; take an owning +1.
            unsafe { obj::incr_ref_count(a) };
            new_argv.push(a);
        }
        // Record the call as the user wrote it so a `wrong # args` from the target
        // is reported in ensemble terms (C's `TclInitRewriteEnsemble`): the
        // ensemble command, its `-parameters`, and the subcommand word (`2 +
        // nparams`) are removed; the target prefix + `-parameters` are inserted.
        let nparams = layout.parameters.len();
        let is_root = self.begin_ensemble_rewrite(source, layout.arguments, prefix.len() + nparams);
        let code = self.dispatch(&new_argv);
        if code == Code::Error && default_target_was_missing {
            if let Some(name) = default_name {
                let mut qualified = b"invalid command name \"".to_vec();
                qualified.extend_from_slice(&prefix[0]);
                qualified.push(b'"');
                if obj_bytes(self.get_obj_result()) == qualified {
                    let mut message = b"invalid command name \"".to_vec();
                    message.extend_from_slice(name);
                    message.push(b'"');
                    // This is a presentation rewrite only. `::unknown` may
                    // have supplied custom -errorcode/-errorinfo/-errorstack;
                    // replacing ExceptionState here would discard all three.
                    // The target miss was proven before dispatch, and the
                    // exact default-target message proves this is the miss we
                    // are allowed to spell in ensemble terms.
                    self.set_result_bytes(&message);
                }
            }
        }
        if is_root {
            self.clear_ensemble_rewrite();
        }
        release_all(&new_argv);
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
        let mut hv: Vec<*mut TclObj> = Vec::with_capacity(cfg.unknown.len() + argv.len());
        for w in &cfg.unknown {
            let o = new_string(w);
            unsafe { obj::incr_ref_count(o) };
            hv.push(o);
        }
        let fqo = new_string(&ens_fqn);
        unsafe { obj::incr_ref_count(fqo) };
        hv.push(fqo);
        for &a in &argv[1..] {
            unsafe { obj::incr_ref_count(a) };
            hv.push(a);
        }
        let mut handler_call = Vec::new();
        for (index, &word) in hv.iter().enumerate() {
            if index != 0 {
                handler_call.push(b' ');
            }
            crate::list::append_list_element(&mut handler_call, &obj_bytes(word), false);
        }
        let code = self.dispatch(&hv);
        release_all(&hv);
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
                let res = obj_bytes(self.get_obj_result());
                match crate::parse::split_list(&res) {
                    Ok(prefix) if !prefix.is_empty() => EnsembleUnknown::Prefix(prefix),
                    Ok(_) => EnsembleUnknown::Reparse,
                    Err(e) => {
                        let error_code: &[u8] = match e {
                            crate::parse::ListError::UnmatchedBrace => b"TCL VALUE LIST BRACE",
                            crate::parse::ListError::UnmatchedQuote => b"TCL VALUE LIST QUOTE",
                            crate::parse::ListError::BraceFollowedByJunk
                            | crate::parse::ListError::QuoteFollowedByJunk => {
                                b"TCL VALUE LIST JUNK"
                            }
                            crate::parse::ListError::NotUtf8 => b"TCL VALUE LIST",
                        };
                        let message = crate::parse::list_error_message(&res, e);
                        let code = self.error_with_code(&message, error_code);
                        self.append_error_info_context(
                            b"while parsing result of ensemble unknown subcommand handler",
                        );
                        EnsembleUnknown::Failed(code)
                    }
                }
            }
            Code::Error => {
                if let Some(command) = parse::parse_script(&handler_call).first() {
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
        let cmd = self.resolve_dispatchable(self.current_ns.get(), &rel);
        let Some(cmd) = cmd else {
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
        let code = self.invoke(cmd, &argv);
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
    /// definition-time gate errors instead of exhausting the native stack.
    fn dispatch_alias(&mut self, target: &[u8], prefix: &[Vec<u8>], argv: &[*mut TclObj]) -> Code {
        if ALIAS_DISPATCH_DEPTH.with(|d| d.get()) >= MAX_ALIAS_DISPATCH_DEPTH {
            return self.error(b"too many nested alias invocations (infinite loop?)");
        }
        let target_cmd = self.resolve_dispatchable(GLOBAL, target);
        // Build [target, *prefix, *argv[1..]] — each element owned (+1).
        let mut new_argv: Vec<*mut TclObj> = Vec::with_capacity(prefix.len() + argv.len());
        let push_owned = |v: &mut Vec<*mut TclObj>, o: *mut TclObj| {
            // SAFETY: `o` is a live object; take the owning +1 the new argv holds.
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
        ALIAS_DISPATCH_DEPTH.with(|d| d.set(d.get() + 1));
        let code = match target_cmd {
            Some(target_cmd) => self.invoke(target_cmd, &new_argv),
            None => {
                // Lazily bound: the target was deleted, never existed, or is
                // a builtin the emulated release does not carry. Feed the
                // synthesized target call back through ordinary dispatch so
                // the global `unknown` handler sees the target name, prefix,
                // and original arguments exactly as C's `TclInvokeAlias`
                // does (tclBasic.c / tclNamesp.c). Alias targets are resolved
                // in the global namespace, so do the fallback dispatch there
                // too; in particular, do not let a caller namespace's
                // `namespace unknown` intercept this path.
                let saved_ns = self.current_ns.replace(GLOBAL);
                let code = self.dispatch(&new_argv);
                self.current_ns.set(saved_ns);
                code
            }
        };
        ALIAS_DISPATCH_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
        release_all(&new_argv);
        code
    }

    /// The `invalid command name "X"` error (the resolver miss; `unknown` later).
    pub(crate) fn invalid_command(&mut self, name: &[u8]) -> Code {
        let mut msg = b"invalid command name \"".to_vec();
        msg.extend_from_slice(name);
        msg.push(b'"');
        self.error(&msg)
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

    /// `subst` — substitute variables / commands / backslashes in `src` per
    /// `flags`, propagating errors (an unset variable or a failing `[...]`).
    #[cfg(have_tommath)]
    pub(crate) fn do_subst(
        &mut self,
        src: &[u8],
        flags: crate::subst::SubstFlags,
    ) -> Result<Vec<u8>, Code> {
        self.do_subst_located(src, flags, None)
    }

    /// [`do_subst`](Self::do_subst) with the input string's TIP 280 location
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
    fn read_var(&self, name: &[u8], index: Option<&[u8]>) -> Option<Vec<u8>> {
        crate::vars::resolve_var_bytes(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            self.current_ns.get(),
            name,
            index,
        )
    }

    fn no_such_variable(&mut self, name: &[u8], index: Option<&[u8]>) -> Code {
        let msg = self.read_miss_msg(name, index);
        self.error(&msg)
    }

    /// Build the C-faithful `can't read "NAME": …` message for a failed variable
    /// read, distinguishing the three cases `tclVar.c` reports: a scalar read of
    /// an array (`variable is array`), a missing element of an *existing* array
    /// (`no such element in array`), and a wholly missing variable (`no such
    /// variable`). `base`/`index` are the split reference (`base(index)`).
    pub(crate) fn read_miss_msg(&self, base: &[u8], index: Option<&[u8]>) -> Vec<u8> {
        let mut msg = b"can't read \"".to_vec();
        msg.extend_from_slice(base);
        if let Some(i) = index {
            msg.push(b'(');
            msg.extend_from_slice(i);
            msg.push(b')');
        }
        msg.extend_from_slice(b"\": ");
        if self.var_is_array(base) {
            if index.is_some() {
                msg.extend_from_slice(b"no such element in array");
            } else {
                msg.extend_from_slice(b"variable is array");
            }
        } else if index.is_some() && self.var_exists(base) {
            // An existing scalar accessed with an index (`set b(123)` where `b`
            // is a scalar): C's `tclVar.c` reports `variable isn't array`.
            msg.extend_from_slice(b"variable isn't array");
        } else {
            msg.extend_from_slice(b"no such variable");
        }
        msg
    }

    /// Set an error result and return [`Code::Error`] — for builtins.
    pub(crate) fn set_error(&mut self, msg: &[u8]) -> Code {
        self.error(msg)
    }

    /// Set the canonical `wrong # args: should be "usage"` arity error and
    /// return [`Code::Error`] (`Tcl_WrongNumArgs` with a literal usage) — the
    /// one home for the builtins' arity message, formerly a per-`cmd_*.rs`
    /// copy.
    pub(crate) fn wrong_args(&mut self, usage: &[u8]) -> Code {
        let mut m = b"wrong # args: should be \"".to_vec();
        m.extend_from_slice(usage);
        m.push(b'"');
        self.set_error(&m)
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
    pub(crate) fn proc_wrong_args(
        &self,
        called: &[u8],
        params: &[Param],
        supplied: usize,
        quote_name: bool,
    ) -> Vec<u8> {
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
                let prefix: Vec<&[u8]> = rw
                    .source
                    .iter()
                    .take(rw.removed)
                    .map(Vec::as_slice)
                    .collect();
                return proc_usage_words(&prefix, &params[drop..], params);
            }
        }
        proc_usage(called, params, quote_name)
    }
}

/// Build a `wrong # args` message from explicit leading `words` followed by the
/// formal `shown` parameters (`all` is the full parameter list, for the `args`
/// catch-all test). Shared by the plain and ensemble-rewritten forms.
fn proc_usage_words(words: &[&[u8]], shown: &[Param], all: &[Param]) -> Vec<u8> {
    let mut m = b"wrong # args: should be \"".to_vec();
    for (i, w) in words.iter().enumerate() {
        if i > 0 {
            m.push(b' ');
        }
        m.extend_from_slice(w);
    }
    let last_is_args = all.last().is_some_and(|p| p.name == b"args");
    for p in shown {
        m.push(b' ');
        if last_is_args && std::ptr::eq(p, all.last().unwrap()) {
            m.extend_from_slice(b"?arg ...?");
        } else if p.default.is_some() {
            m.push(b'?');
            m.extend_from_slice(&p.name);
            m.push(b'?');
        } else {
            m.extend_from_slice(&p.name);
        }
    }
    m.push(b'"');
    m
}

fn proc_usage(called: &[u8], params: &[Param], quote_name: bool) -> Vec<u8> {
    let mut m = b"wrong # args: should be \"".to_vec();
    // A single-word proc name is list-quoted if it needs it — `a b  c` → `{a b  c}`,
    // `` → `{}` (C's `Tcl_WrongNumArgs` via `TclScanElement`, Bug 942757). `apply`
    // and TclOO pass a pre-joined multi-word usage prefix that must stay raw.
    if quote_name {
        crate::list::append_list_element(&mut m, called, false);
    } else {
        m.extend_from_slice(called);
    }
    let n = params.len();
    for (i, p) in params.iter().enumerate() {
        m.push(b' ');
        if i + 1 == n && p.name == b"args" {
            m.extend_from_slice(b"?arg ...?");
        } else if p.default.is_some() {
            m.push(b'?');
            m.extend_from_slice(&p.name);
            m.push(b'?');
        } else {
            m.extend_from_slice(&p.name);
        }
    }
    m.push(b'"');
    m
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
        // Runs when the last `Interp` handle to this state is dropped. Release
        // the result; the `FrameStack` field drops afterwards, releasing all
        // variable refs. The command table holds no object refs. Children are
        // `Interp` handles (their own `Rc`s); the parent link is `Weak`, so the
        // tree has no reference cycle to leak.
        // SAFETY: `result` is the interp's owned reference, dropped once.
        unsafe { obj::decr_ref_count(self.result.get()) };
        self.result.set(core::ptr::null_mut());
        // Release any pending `-during` chain link the interp still owns.
        if let Some(d) = self.during.take() {
            // SAFETY: `during` held an owning reference; drop it once.
            unsafe { obj::decr_ref_count(d) };
        }
    }
}

// ---------------------------------------------------------------------------
// Object byte helpers.
// ---------------------------------------------------------------------------

/// A fresh (`rc 0`) string object holding `bytes`.
pub(crate) fn new_string(bytes: &[u8]) -> *mut TclObj {
    // SAFETY: `bytes` is a valid readable slice.
    unsafe { obj::new_string_obj(bytes.as_ptr() as *const c_char, bytes.len() as obj::TclSize) }
}

/// Collapse every run of two-or-more `:` to a single `::` separator, matching
/// Tcl's namespace-name normalization (empty namespace components are ignored):
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
mod tests {
    use super::*;
    use crate::counters;

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
        // Command-table mutation deliberately clears all identity attestations.
        // Restore this explicit identity so a failed check proves the
        // Interpreter epoch changed rather than merely observing a missing ID.
        interp.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
        assert!(!interp.check_command_guard(token, b"guarded"));
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

    #[test]
    fn command_mutation_invalidates_guard_and_identity_attestation() {
        leak_free(|i| {
            i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
            let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
            let token = i
                .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
                .unwrap();
            i.register_builtin(b"unrelated", guarded_builtin);
            assert!(!i.check_command_guard(token, b"guarded"));
            assert_eq!(
                i.prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains),
                Err(GuardError::IdentityUnavailable)
            );
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
                Some(Code::Ok)
            );
            assert_eq!(i.result_bytes(), b"3");
            assert_eq!(i.eval_str("string length é🙂".as_bytes()), Code::Ok);
            assert_eq!(i.result_bytes(), b"3");

            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            assert_eq!(i.eval_str("string length é🙂".as_bytes()), Code::Ok);
            assert_eq!(i.result_bytes(), b"2");
            drop_fresh(value);
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
            assert!(!i.check_command_guard(token, b"moved"));
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

    /// Regression coverage for issue #996 in this runtime specifically: a
    /// tree-walking interpreter recurses natively (`eval_command` → command
    /// dispatch → `eval_control_body`/`run_proc` → `eval_script_mode`,
    /// recursively) for every nested control-flow body or proc call, unlike
    /// C Tcl's bytecode-compiled control structures. Empirically, on this
    /// crate's native build under `cargo test`'s ~2 MiB per-test stack:
    /// unguarded nested `foreach` bodies overflowed (SIGABRT) between depth
    /// 200 and 250, and unbounded recursive proc calls overflowed *before
    /// ever reaching* the pre-existing `RECURSION_LIMIT` of 1000 — i.e. that
    /// cap was never actually a safe backstop against a native crash. See
    /// `NATIVE_EVAL_DEPTH_LIMIT`'s doc comment for the full rationale.
    #[test]
    fn deeply_nested_foreach_errors_instead_of_crashing() {
        leak_free(|i| {
            // 300 is comfortably past the measured 200-250 crash range and
            // past NATIVE_EVAL_DEPTH_LIMIT (128); the assertion is that this
            // *returns* a catchable error rather than aborting the process.
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

    /// Defence in depth for the alias trampoline (issue #1447). `interp alias`
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
    fn missing_alias_target_uses_global_unknown_not_caller_namespace_unknown() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(b"proc unknown {cmd args} {list global $cmd $args}"),
                Code::Ok
            );
            assert_eq!(
                i.eval_str(
                    b"namespace eval n {proc u {cmd args} {list ns $cmd $args}; namespace unknown u; interp alias {} la {} nosuch; la x}"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"global nosuch x");
        });
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
        // The headline T1.5 fix: `::pinged` and `pinged` are the SAME global
        // (before, `::pinged` was a literal frame key distinct from `pinged`).
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
    /// under (issue #1462) and the builtin command surface (issue #1463):
    /// under an 8.4 pin `{*}` does not expand and the first-close `${…}`
    /// rule applies, and `lassign` (8.5+) resolves to `invalid command
    /// name` — while a user-defined proc of the same name stays callable
    /// (the compat-polyfill pattern).
    #[test]
    fn grammar_and_command_surface_follow_the_selected_release() {
        use tcl_dialect::TclVersion;

        // #1462 — the `{*}` expansion grammar (TIP 157, 8.5+).
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
            // 8.5.19/8.6.16/9.0.4/9.1b0 all answer `2`. This engine used to
            // recover it as one word and answer `1`; the boundary owner's
            // `welded_after_close` (#1786) closed that residual gap.
            assert_eq!(i.eval_str(b"llength [list {*}{a b}]"), Code::Error);
            assert_eq!(i.result_bytes(), b"extra characters after close-brace");
        });

        // #1462 — the `${…}` delimiting rule: 8.x stops at the first `}`
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

        // #1463 — the builtin surface: lassign is 8.5+, lpop is 9.0+.
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

    /// The #1463 availability gate is a property of the **final resolved
    /// builtin**, not of the spelling the caller wrote. PR #1481's review
    /// found the gate spelled out at direct dispatch only, so the two
    /// resolve-then-`invoke` shapes — the alias trampoline and the `namespace
    /// import` redirect — reached the builtin through a second, ungated
    /// resolution and made an 8.4-hidden `lassign` callable as
    /// `interp alias {} la {} lassign; la {a b} x y`.
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
            // The exact #1493 reproducer is successful without an explicit
            // export.  Global builtins are not exported by default, so this
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

    /// The same gate on the two *non*-dispatch shapes that would hand a
    /// release-hidden builtin back ungated (PR #1481 review): `rename`, which
    /// would rebind it under a name the registry has no spec for, and `interp
    /// hide`, which would park it where `interp invokehidden` reaches it.
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
        use tcl_syntax::number::{set_runtime_syntax, NumberSyntax};

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
    /// holds no name list any more, only the registry's
    /// `Traits::SAFE_INTERP_HIDDEN` query (ledger row B2).
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
    /// `tclsh9.0` fails it — it used to succeed here, because both engines
    /// hardcoded `9.0.4`/`Tcl`+`tcl` regardless of the pin (ledger row B4).
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
    /// `9.0.4` / `9.0`, and `tclsh9.1` answers `9.1b0` / `9.1b0` (its patch
    /// level has no second `.`, so `version` runs to the `+`).
    #[cfg(have_tommath)]
    #[test]
    fn build_info_follows_the_pinned_release() {
        use tcl_dialect::TclVersion;

        for (version, patchlevel, short) in [
            (TclVersion::V9_0, &b"9.0.4"[..], &b"9.0"[..]),
            (TclVersion::V9_1, &b"9.1b0"[..], &b"9.1b0"[..]),
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
}
