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
    },
    /// A `namespace import` redirect: dispatch re-resolves `source` (the source
    /// command's FQN) anchored at global and forwards the caller's argv
    /// unchanged. The importing-ns binding is transparent to callers; `namespace
    /// forget` removes redirects by matching `source`.
    Imported { source: Vec<u8> },
    /// A `namespace ensemble`: dispatch maps `argv[1]` (a subcommand) to a target
    /// command prefix (`-map`, else `<ns>::<sub>`) and forwards `argv[2..]` — the
    /// generalised `dict for`→`::tcl::dict::for` redirect. See [`crate::ensemble`].
    Ensemble(crate::ensemble::EnsembleConfig),
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
    },
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
}

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
/// (mirroring C's `Tcl_GetIndexFromObj`). Returns the canonical spelling or a
/// `bad option "X": must be …` error.
fn resolve_limit_opt(arg: &[u8], opts: &[&[u8]]) -> Result<Vec<u8>, Vec<u8>> {
    let matches: Vec<&[u8]> = opts
        .iter()
        .copied()
        .filter(|o| o.starts_with(arg))
        .collect();
    if matches.len() == 1 {
        return Ok(matches[0].to_vec());
    }
    if opts.contains(&arg) {
        return Ok(arg.to_vec());
    }
    let mut m = b"bad option \"".to_vec();
    m.extend_from_slice(arg);
    m.extend_from_slice(b"\": must be ");
    for (i, o) in opts.iter().enumerate() {
        if i == opts.len() - 1 && i > 0 {
            m.extend_from_slice(b", or ");
        } else if i > 0 {
            m.extend_from_slice(b", ");
        }
        m.extend_from_slice(o);
    }
    Err(m)
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

/// Whether `bytes` is a truthy boolean literal (for the `-frame` latch).
fn parse_truth(bytes: &[u8]) -> bool {
    matches!(
        bytes.to_ascii_lowercase().as_slice(),
        b"1" | b"true" | b"yes" | b"on"
    )
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

impl Interp {
    /// Create an interp: global frame, the built-in command set, an empty
    /// result, and the predefined variables that C installs in
    /// `Tcl_CreateInterp`.
    pub fn new() -> Interp {
        let result = obj::new_obj();
        // SAFETY: `result` is freshly created; the interp takes the owning ref.
        unsafe { obj::incr_ref_count(result) };
        // The default capability host. Native builds get the full-capability
        // std-backed `NativeHost`. The `wasm32-wasip1` build gets `WasiHost`
        // (stdout/stderr reach WASI `fd_write`, so `puts` is visible — the
        // AOT-script target). The `wasm32-unknown-unknown` build gets the
        // placeholder `BrowserHost` (mandatory caps stubbed, no fs/sockets/process)
        // so the runtime links — a real browser host plugs into the same trait.
        #[cfg(not(target_arch = "wasm32"))]
        let host: Rc<dyn tcl_platform::Host> = Rc::new(tcl_host_native::NativeHost::new());
        #[cfg(all(target_arch = "wasm32", target_os = "wasi"))]
        let host: Rc<dyn tcl_platform::Host> = Rc::new(crate::host_wasm::WasiHost::new());
        #[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
        let host: Rc<dyn tcl_platform::Host> = Rc::new(crate::host_wasm::BrowserHost::new());
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
            packages: RefCell::new(crate::cmd_package::PackageState::with_core()),
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
        // without `--init`; `set_startup_globals` re-sets the same pair on the
        // `Tcl_Init` path, which is idempotent.
        interp.write_release_globals();
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
        // Ahead of the unchanged-version short-circuit: the numeric grammar is
        // *thread*-ambient, not per-interp, so "this interp already emulates
        // `version`" does not imply the thread's grammar is this interp's. A
        // second interpreter constructed on a thread where an earlier one
        // installed 8.4 must re-install its own release even when its version
        // field needs no change.
        install_number_syntax(version);
        if self.runtime_version() == version {
            return;
        }
        self.invalidate_interpreter_policy();
        self.invalidate_command_environment();
        self.0.runtime_version.set(version);
        self.namespaces.borrow_mut().ns_var_global_fallback =
            version.namespace_var_global_fallback();
        self.write_release_globals();
    }

    /// The Tcl release this interpreter emulates (see
    /// [`Self::set_runtime_version`]).
    #[must_use]
    pub fn runtime_version(&self) -> tcl_dialect::TclVersion {
        self.0.runtime_version.get()
    }

    /// Whether a builtin command is exposed on this interpreter's selected
    /// runtime surface. The registry recognises versioned builtin entries;
    /// unrecognised names remain available for user-defined commands.
    pub(crate) fn builtin_command_visible_for_surface(&self, name: &[u8]) -> bool {
        core::str::from_utf8(name).map_or(true, |name| {
            tcl_registry::expr_surface::RuntimeExprSurface::for_tcl_version(self.runtime_version())
                .permits_builtin_math_function_command(name)
        })
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
        self.invalidate_command_environment();
        // Command traces fire *before* the table mutation (C's TclRenameCommand:
        // the command still exists under its old name during the callback), with
        // the fully-qualified old and new names.
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
        let raw = self
            .namespaces
            .borrow_mut()
            .rename(self.current_ns.get(), old, new);
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
                    self.namespaces.borrow_mut().retarget_imports(&of, &nf);
                    if oo_live {
                        self.oo_command_renamed(&of, Some(&nf));
                    }
                }
                // The command is gone; its traces and OO registry entry go too.
                RenameOutcome::Deleted => {
                    self.remove_cmd_traces(&of);
                    if oo_live {
                        self.oo_command_renamed(&of, None);
                    }
                }
                RenameOutcome::NoSuchCommand => {}
            }
        }
        outcome
    }

    /// Install an `interp alias` redirect named `name` → `target ?prefix...?`.
    pub(crate) fn install_alias(&mut self, name: &[u8], target: Vec<u8>, prefix: Vec<Vec<u8>>) {
        self.invalidate_command_environment();
        self.namespaces
            .borrow_mut()
            .register(name, Command::Alias { target, prefix });
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
            c.ns_register(name, Command::ParentAlias { target, prefix });
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
            Some(Command::Alias { target, prefix }) => Some((target, prefix)),
            _ => None,
        }
    }

    /// Delete the command bound to `name` (the alias-clear form); returns whether
    /// it existed.
    pub(crate) fn delete_command(&mut self, name: &[u8]) -> bool {
        self.invalidate_command_environment();
        // If `name` is a suspended coroutine, terminate its worker first.
        crate::cmd_coro::on_command_deleted(self, name);
        self.namespaces
            .borrow_mut()
            .delete(self.current_ns.get(), name)
    }

    /// Register an ensemble command (`namespace ensemble create`); `name` is the
    /// ensemble command (possibly qualified — rooted at global like any builtin).
    pub(crate) fn create_ensemble(&mut self, name: &[u8], cfg: crate::ensemble::EnsembleConfig) {
        self.invalidate_command_environment();
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
        self.namespaces
            .borrow_mut()
            .bind(ns, &tail, Command::Ensemble(cfg));
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
        // Redefining a command deletes the old one: fire its delete command
        // traces and drop all its traces (C's Tcl_CreateObjCommand replacing an
        // existing command). Keyed by the FQN we are about to bind.
        self.on_command_replaced(&fqn);
        self.namespaces
            .borrow_mut()
            .bind(ns, &tail, Command::Proc(def));
    }

    /// A command at `fqn` is being replaced or deleted: fire its `delete`
    /// command traces, then drop every command/execution trace on it (the
    /// command — and its trace list — go away). No-op when it has no traces.
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
        self.fire_cmd_trace(fqn, b"", crate::cmd_trace::ops::DELETE);
        self.remove_cmd_traces(fqn);
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
        matches!(
            self.namespaces
                .borrow()
                .resolve(self.current_ns.get(), name),
            Some(Command::Ensemble(_))
        )
    }

    /// The configuration of the ensemble command `name` resolves to, or `None`
    /// if `name` is not an ensemble (`namespace ensemble configure`/cget).
    pub(crate) fn ensemble_config(&self, name: &[u8]) -> Option<crate::ensemble::EnsembleConfig> {
        match self
            .namespaces
            .borrow()
            .resolve(self.current_ns.get(), name)
        {
            Some(Command::Ensemble(cfg)) => Some(cfg),
            _ => None,
        }
    }

    /// Rebind the ensemble command `name` with an updated configuration
    /// (`namespace ensemble configure` set form). `name` already resolves to an
    /// ensemble (the caller read it via [`ensemble_config`](Self::ensemble_config)),
    /// so the update lands at the namespace where it *resolves* — which may be
    /// reached via `namespace path`, not the current namespace — rather than
    /// creating a shadowing copy in the current namespace.
    pub(crate) fn set_ensemble_config(
        &mut self,
        name: &[u8],
        cfg: crate::ensemble::EnsembleConfig,
    ) {
        self.invalidate_command_environment();
        if !self.namespaces.borrow_mut().rebind_resolved(
            self.current_ns.get(),
            name,
            Command::Ensemble(cfg.clone()),
        ) {
            // Should not happen (the caller verified it resolves); create at the
            // current-namespace location as a fallback.
            self.create_ensemble(name, cfg);
        }
    }

    /// Every alias command's name across the whole tree (`interp aliases`).
    pub(crate) fn alias_names(&self) -> Vec<Vec<u8>> {
        self.namespaces.borrow().alias_names()
    }

    /// The current namespace (the eval context) — for the `namespace` builtin.
    pub(crate) fn current_ns(&self) -> NsId {
        self.current_ns.get()
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
        let lib = self.host().env().get("TCL_LIBRARY").unwrap_or_default();
        let set = |i: &mut Interp, name: &[u8], val: &[u8]| {
            let o = new_string(val);
            if i.var_set(name, o).is_err() {
                drop_fresh(o);
            }
        };
        set(self, b"::tcl_library", lib.as_bytes());
        // `tcl_version`/`tcl_patchLevel` are NOT set here: C sets them in
        // `Tcl_CreateInterp`, and so does this runtime (see `Interp::new`).
        // Re-derived rather than re-literalled so a non-9.0
        // `set_runtime_version` is not silently overwritten by `Tcl_Init`.
        self.write_release_globals();
        set(self, b"::tcl_interactive", b"0");
        set(self, b"::argv", b"");
        set(self, b"::argv0", b"");
        set(self, b"::argc", b"0");
        set(self, b"::auto_path", b"");
        // tcl_platform array (the fields init.tcl + tcltest read).
        for (k, v) in [
            (&b"platform"[..], &b"unix"[..]),
            (b"os", b"Linux"),
            (b"osVersion", b"0"),
            (b"byteOrder", b"littleEndian"),
            (b"wordSize", b"8"),
            (b"pointerSize", b"8"),
            (b"engine", b"Tcl"),
            (b"threaded", b"0"),
            (b"pathSeparator", b":"),
        ] {
            let o = new_string(v);
            if self.var_set_elem(b"tcl_platform", k, o).is_err() {
                drop_fresh(o);
            }
        }
        // Backend-introspection keys (the test-suite constraint overlay reads
        // these). The tree-walk runtime targets native or wasm32-wasip*, so the
        // wasm / WASI facts come from the build's `cfg`; an environment override
        // (read through the host seam) lets a native binary evaluate another
        // backend's skip lists.
        {
            use tcl_platform::backend::{self, key};
            let detected = |k: &str, compiled: &str| -> String {
                backend::override_env_var(k)
                    .and_then(|var| self.host().env().get(var))
                    .filter(|v| !v.is_empty())
                    .unwrap_or_else(|| compiled.to_string())
            };
            let backend_keys = [
                (key::RUNTIME, "treewalk".to_string()),
                (key::RUNTIME_VERSION, env!("CARGO_PKG_VERSION").to_string()),
                (
                    key::WASM,
                    detected(key::WASM, backend::compiled_wasm_spec()),
                ),
                (
                    key::WASI,
                    detected(key::WASI, backend::compiled_wasi_spec()),
                ),
                (
                    key::WASI_VERSION,
                    detected(key::WASI_VERSION, backend::compiled_wasi_host()),
                ),
                (
                    key::EBPF,
                    detected(key::EBPF, backend::compiled_ebpf_spec()),
                ),
            ];
            for (k, v) in backend_keys {
                let o = new_string(v.as_bytes());
                if self.var_set_elem(b"tcl_platform", k.as_bytes(), o).is_err() {
                    drop_fresh(o);
                }
            }
        }
        // env array from the host environment (no quoting hazards via var_set_elem).
        let vars = self.host().env().vars();
        for (k, v) in vars {
            let o = new_string(v.as_bytes());
            if self.var_set_elem(b"env", k.as_bytes(), o).is_err() {
                drop_fresh(o);
            }
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

    /// Frame-addressed [`var_unset`](Self::var_unset).
    pub(crate) fn var_unset_at(&mut self, name: &[u8], level: usize) -> bool {
        crate::vars::unset_at(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            name,
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

    /// The `(home namespace, simple base name)` a variable trace on `base`
    /// should be keyed by — see
    /// [`crate::vars::home_namespace_and_base`]. Registration uses this so a
    /// trace matches every spelling that resolves to the same variable.
    pub(crate) fn trace_var_key(&self, base: &[u8]) -> (Option<NsId>, Vec<u8>) {
        crate::vars::home_namespace_and_base(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            self.current_ns.get(),
            base,
        )
    }

    pub(crate) fn local_trace_level(&self, base: &[u8]) -> Option<usize> {
        if tcl_syntax::naming::is_qualified(base) {
            return None;
        }
        let frames = self.frames.borrow();
        let level = frames.current_level();
        if level == 0 || !frames.in_proc() || frames.current_is_link(base) {
            return None;
        }
        Some(level)
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
        let mut m = b"can't read \"".to_vec();
        m.extend_from_slice(&display);
        m.extend_from_slice(b"\": ");
        m.extend_from_slice(&msg);
        Some(self.set_error(&m))
    }

    /// Read a variable's current value for `lappend`, firing its read trace but
    /// **swallowing** any error the trace raises (yielding `None`, as if the
    /// variable were absent). This is C's `Tcl_ObjGetVar2` *without*
    /// `TCL_LEAVE_ERR_MSG`: `lappend` fires the read trace (its side effects run
    /// — append-7.2/7.3) yet a trace that errors on a missing element must not
    /// fail the append, which instead creates the element (bug 3057639,
    /// append-9.0). `set`/`append`-read, by contrast, propagate the error via
    /// [`fire_read_trace`](Self::fire_read_trace).
    pub(crate) fn lappend_read(&mut self, base: &[u8], elem: Option<&[u8]>) -> Option<*mut TclObj> {
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
        let key = traced.then(|| (self.trace_var_key(&base), self.local_trace_level(&base)));
        let existed = crate::vars::unset(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
        );
        if let (true, Some(((access_ns, resolved_base), access_frame_level))) = (existed, key) {
            self.fire_var_trace_resolved(
                access_ns,
                access_frame_level,
                &resolved_base,
                elem.as_deref(),
                b"unset",
            );
            // The variable (and its traces) go away — drop every trace on it
            // (C frees the Var's trace list on unset). Element unset drops only
            // that element's traces (whole-variable traces survive).
            let mut t = self.traces.borrow_mut();
            match elem {
                Some(e) => t.traces.retain(|v| {
                    !(crate::cmd_trace::same_variable(
                        v,
                        &resolved_base,
                        access_ns,
                        access_frame_level,
                    ) && v.elem.as_deref() == Some(e.as_slice()))
                }),
                None => t.traces.retain(|v| {
                    !crate::cmd_trace::same_variable(
                        v,
                        &resolved_base,
                        access_ns,
                        access_frame_level,
                    )
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
        let trace_key = (!self.traces.borrow().traces.is_empty())
            .then(|| (self.trace_var_key(name), self.local_trace_level(name)));
        let existed = crate::vars::unset_elem(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
            key,
        );
        if let (true, Some(((access_ns, resolved_base), access_frame_level))) = (existed, trace_key)
        {
            self.fire_var_trace_resolved(
                access_ns,
                access_frame_level,
                &resolved_base,
                Some(key),
                b"unset",
            );
            // Drop this element's traces (whole-array traces survive).
            self.traces.borrow_mut().traces.retain(|v| {
                !(crate::cmd_trace::same_variable(v, &resolved_base, access_ns, access_frame_level)
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
        // Resolve the access to the same `(home, simple name)` key registration
        // used, so a trace matches every spelling of the variable it is on
        // (`::v` vs `v`, and the 8.x namespace-scope fallback) — issue #1328.
        let access_frame_level = self.local_trace_level(base);
        let (access_ns, base) = self.trace_var_key(base);
        self.fire_var_trace_resolved(access_ns, access_frame_level, &base, elem, op)
    }

    /// [`Self::fire_var_trace`] with the `(home, simple name)` key already
    /// resolved — for `unset`, which must resolve *before* it removes the
    /// variable (resolution can depend on the cell existing).
    fn fire_var_trace_resolved(
        &mut self,
        access_ns: Option<NsId>,
        access_frame_level: Option<usize>,
        base: &[u8],
        elem: Option<&[u8]>,
        op: &[u8],
    ) -> bool {
        let traces = self.traces.borrow();
        let cmds: Vec<(crate::cmd_trace::VarTraceScope, Vec<u8>)> = traces
            .traces
            .iter()
            .filter(|t| {
                crate::cmd_trace::matches(t, base, elem, op, access_ns, access_frame_level)
                    && !traces.active_var_scopes.contains(&t.scope())
            })
            .map(|t| (t.scope(), t.command.clone()))
            .collect();
        drop(traces);
        if cmds.is_empty() {
            return false;
        }
        let propagate = op == b"read" || op == b"write";
        // Preserve the result object across the callbacks.
        let saved = self.result.get();
        unsafe { obj::incr_ref_count(saved) };

        let mut errored = false;
        for (scope, cmd) in cmds {
            // Append `base element op` as properly-quoted trailing words.
            let args = crate::list::new_list_obj(&[
                new_string(base),
                new_string(elem.unwrap_or(b"")),
                new_string(op),
            ]);
            let mut line = cmd;
            line.push(b' ');
            line.extend_from_slice(&obj_bytes(args));
            drop_fresh(args);
            self.traces
                .borrow_mut()
                .active_var_scopes
                .push(scope.clone());
            let code = self.eval_str(&line);
            let popped = self.traces.borrow_mut().active_var_scopes.pop();
            debug_assert_eq!(popped, Some(scope));
            if propagate && code == Code::Error {
                // Capture the callback's error message; stop firing (C aborts
                // the trace chain on the first error).
                let msg = self.result_bytes();
                self.traces.borrow_mut().pending_err = Some(msg);
                errored = true;
                break;
            }
        }
        // Restore the saved result (release the trace's, adopt our held +1).
        unsafe {
            obj::decr_ref_count(self.result.get());
            self.result.set(saved);
        }
        errored
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
    /// commands"). Re-entrant firing is suppressed (`exec_firing`); the interp
    /// result is preserved across the callbacks.
    fn fire_cmd_trace(&mut self, old_fqn: &[u8], new_fqn: &[u8], op_bit: u8) {
        if self.traces.borrow().exec_firing > 0 {
            return;
        }
        let cmds: Vec<Vec<u8>> = self
            .traces
            .borrow()
            .cmd_traces
            .iter()
            .filter(|t| t.name == old_fqn && (t.ops & op_bit) != 0)
            .map(|t| t.command.clone())
            .collect();
        if cmds.is_empty() {
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

        self.traces.borrow_mut().exec_firing += 1;
        for cmd in cmds {
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
        self.traces.borrow_mut().exec_firing -= 1;

        unsafe {
            obj::decr_ref_count(self.result.get());
            self.result.set(saved);
        }
    }

    /// Fire `enter` execution traces on `fqn` (creation order), invoking each as
    /// `<prefix> {cmd args} enter`. Returns `Some(code)` if a callback completed
    /// non-OK — the command is then aborted with that code and the callback's
    /// result (C's `TclEvalObjvInternal`: `traceCode != TCL_OK ⇒ return`).
    fn fire_exec_enter(&mut self, fqn: &[u8], cmd_word: &[u8]) -> Option<Code> {
        use crate::cmd_trace::ops;
        // C fires `enter` newest-first (the trace list is prepended; the loop
        // walks it head→tail). Our Vec pushes newest-last, so iterate reversed.
        let cmds: Vec<Vec<u8>> = self
            .traces
            .borrow()
            .cmd_traces
            .iter()
            .rev()
            .filter(|t| t.name == fqn && (t.ops & ops::ENTER) != 0)
            .map(|t| t.command.clone())
            .collect();
        if cmds.is_empty() {
            return None;
        }
        let saved = self.result.get();
        unsafe { obj::incr_ref_count(saved) };
        self.traces.borrow_mut().exec_firing += 1;
        let mut abort: Option<Code> = None;
        for cmd in cmds {
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
        let cmds: Vec<Vec<u8>> = self
            .traces
            .borrow()
            .cmd_traces
            .iter()
            .filter(|t| t.name == fqn && (t.ops & ops::LEAVE) != 0)
            .map(|t| t.command.clone())
            .collect();
        if cmds.is_empty() {
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
        for cmd in cmds {
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
        let mut traces = self.traces.borrow_mut();
        let mut moved = false;
        for t in traces.cmd_traces.iter_mut() {
            if t.name == old_fqn {
                t.name = new_fqn.to_vec();
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

    /// Delete the namespace `ns` (by id), e.g. an OO object's instance namespace
    /// when the object is destroyed.
    pub(crate) fn delete_namespace_by_id(&mut self, ns: NsId) {
        self.invalidate_command_environment();
        // Variables in the namespace (and its descendants) are about to be
        // unset; fire their unset traces. Names are built while the namespace
        // still exists, then the namespace is torn down, then the callbacks run
        // — so a callback sees the namespace already gone (C's order; oo-11.8).
        let victims = self.take_ns_unset_traces(ns);
        // The commands in the namespace tree are deleted too: collect+remove
        // their command traces (firing the `delete` ones afterwards). Without
        // this, a delete-trace on `::ns::cmd` would *linger* after `ns` is gone
        // and mis-fire when a same-named command is later created (the stale
        // `::x → namespace delete ::` chain that wiped the global namespace).
        let cmd_victims = self.take_ns_cmd_traces(ns);
        // Ensemble commands are tied to their namespace: delete those whose
        // configured namespace is in the subtree (even if the command itself
        // lives elsewhere, e.g. a default `::ns` ensemble in the global table).
        {
            let ids: std::collections::HashSet<NsId> = self
                .namespaces
                .borrow()
                .descendant_ids(ns)
                .into_iter()
                .collect();
            let removed = self.namespaces.borrow_mut().remove_ensembles_for(&ids);
            for fqn in removed {
                self.on_command_replaced(&fqn);
            }
        }
        self.namespaces.borrow_mut().delete_namespace_by_id(ns);
        self.fire_unset_callbacks(victims);
        self.fire_deleted_cmd_callbacks(cmd_victims);
    }

    /// Remove and return the `(fqn, command)` of every command trace on a command
    /// in `ns` or a descendant (so the command's deletion via namespace teardown
    /// drops its traces, as C does). Only `delete`-op traces are returned for
    /// firing; `rename`-only traces are removed silently (a deletion isn't a
    /// rename).
    fn take_ns_cmd_traces(&self, ns: NsId) -> Vec<(Vec<u8>, Vec<u8>)> {
        if self.traces.borrow().cmd_traces.is_empty() {
            return Vec::new();
        }
        // The fully-qualified prefixes (`::a::b::`) of the namespace and every
        // descendant; a command trace's name is `<homeQual>::<simple>`, so it
        // belongs to the tree iff its qualifier prefix is one of these.
        let quals: std::collections::HashSet<Vec<u8>> = {
            let ns_ref = self.namespaces.borrow();
            ns_ref
                .descendant_ids(ns)
                .into_iter()
                .map(|i| {
                    let mut q = ns_ref.qualified_name(i);
                    if q != b"::" {
                        q.extend_from_slice(b"::");
                    }
                    q
                })
                .collect()
        };
        let mut victims = Vec::new();
        let mut traces = self.traces.borrow_mut();
        let old_len = traces.cmd_traces.len();
        traces.cmd_traces.retain(|t| {
            // The command's home-namespace prefix: everything up to and
            // including the last `::` (global commands are `::cmd` → `::`).
            let qual: &[u8] = match t.name.windows(2).rposition(|w| w == b"::") {
                Some(0) => b"::",
                Some(i) => &t.name[..i + 2],
                None => b"",
            };
            if quals.contains(qual) {
                if (t.ops & crate::cmd_trace::ops::DELETE) != 0 {
                    victims.push((t.name.clone(), t.command.clone()));
                }
                false // drop every trace on a command that is going away
            } else {
                true
            }
        });
        let removed = traces.cmd_traces.len() != old_len;
        drop(traces);
        if removed {
            self.invalidate_guard_domain(GuardDomain::CommandTrace);
        }
        victims
    }

    /// Fire collected command `delete`-trace callbacks as `command oldName {}
    /// delete` after the namespace has been torn down (errors ignored — a delete
    /// trace's result is discarded, matching C).
    fn fire_deleted_cmd_callbacks(&mut self, victims: Vec<(Vec<u8>, Vec<u8>)>) {
        if victims.is_empty() || self.traces.borrow().exec_firing > 0 {
            return;
        }
        let saved = self.result.get();
        unsafe { obj::incr_ref_count(saved) };
        self.traces.borrow_mut().exec_firing += 1;
        for (name, cmd) in victims {
            let args = crate::list::new_list_obj(&[
                new_string(&name),
                new_string(b""),
                new_string(b"delete"),
            ]);
            let mut line = cmd;
            line.push(b' ');
            line.extend_from_slice(&obj_bytes(args));
            drop_fresh(args);
            let _ = self.eval_str(&line);
        }
        self.traces.borrow_mut().exec_firing -= 1;
        unsafe {
            obj::decr_ref_count(self.result.get());
            self.result.set(saved);
        }
    }

    /// Remove and return the `(fullName, command)` of every *unset* variable
    /// trace registered on a namespace variable in `ns` or a descendant (so it
    /// can be fired as the namespace is deleted).
    fn take_ns_unset_traces(&self, ns: NsId) -> Vec<(Vec<u8>, Vec<u8>)> {
        if self.traces.borrow().traces.iter().all(|t| t.ns.is_none()) {
            return Vec::new();
        }
        let ids: std::collections::HashSet<NsId> = self
            .namespaces
            .borrow()
            .descendant_ids(ns)
            .into_iter()
            .collect();
        let mut victims = Vec::new();
        let mut traces = self.traces.borrow_mut();
        let old_len = traces.traces.len();
        let ns_ref = self.namespaces.borrow();
        traces.traces.retain(|t| {
            let hit =
                t.ns.is_some_and(|n| ids.contains(&n) && t.ops.iter().any(|o| o == b"unset"));
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
                victims.push((fqn, t.command.clone()));
            }
            !hit
        });
        let removed = traces.traces.len() != old_len;
        drop(ns_ref);
        drop(traces);
        if removed {
            self.invalidate_guard_domain(GuardDomain::VariableTrace);
        }
        victims
    }

    /// Fire collected unset-trace callbacks as `command name {} unset`. Errors
    /// are ignored (an unset trace's result is discarded, as in C).
    fn fire_unset_callbacks(&mut self, victims: Vec<(Vec<u8>, Vec<u8>)>) {
        if victims.is_empty() {
            return;
        }
        let saved = self.result.get();
        unsafe { obj::incr_ref_count(saved) };
        for (name, cmd) in victims {
            let args = crate::list::new_list_obj(&[
                new_string(&name),
                new_string(b""),
                new_string(b"unset"),
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
                let is_builtin = matches!(ns.resolve(id, name), Some(Command::Builtin(_)));
                let mut full_name = prefix.clone();
                full_name.extend_from_slice(name);
                (!is_builtin || self.builtin_command_visible_for_surface(&full_name))
                    .then(|| name.to_vec())
            })
            .collect()
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
                Command::Imported { source } => cmd = ns.resolve(GLOBAL, &source)?,
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
        let (access_ns, base) = self.trace_var_key(b"::errorInfo");
        let access_frame_level = self.local_trace_level(b"::errorInfo");
        self.traces.borrow().traces.iter().any(|t| {
            crate::cmd_trace::matches(t, &base, None, b"write", access_ns, access_frame_level)
        })
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

    /// Publish an error's trace **if** it has unwound past the outermost
    /// evaluation, leaving `::errorInfo`/`::errorCode` set for an uncaught one.
    ///
    /// [`eval_script_mode`](Self::eval_script_mode) applies this at the end of
    /// the eval loop. Compiled code that dispatches a command directly, without
    /// entering that loop, has no other place to do it, so it calls this and the
    /// depth policy stays in one place rather than being restated per caller.
    pub(crate) fn publish_error_if_uncaught(&mut self) {
        if self.eval_depth.get() == 0 {
            self.publish_error();
        }
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
        let commands = parse::parse_script(src);
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
        let resolved = self
            .namespaces
            .borrow()
            .resolve(self.current_ns.get(), &name);
        let wrapper_hidden = matches!(&resolved, Some(Command::Builtin(_)))
            && self
                .resolve_cmd_fqn(&name)
                .is_some_and(|fqn| !self.builtin_command_visible_for_surface(&fqn));
        if let Some(cmd) = resolved.filter(|_| !wrapper_hidden) {
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
            let unk = self.namespaces.borrow().resolve(GLOBAL, b"unknown");
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
            Command::Alias { target, prefix } => self.dispatch_alias(&target, &prefix, argv),
            Command::Imported { source } => {
                let resolved = self.namespaces.borrow().resolve(GLOBAL, &source);
                match resolved {
                    // Transparent redirect: forward argv unchanged to the source.
                    Some(cmd) => self.invoke(cmd, argv),
                    None => self.invalid_command(&source),
                }
            }
            Command::Ensemble(cfg) => self.dispatch_ensemble(&cfg, argv),
            Command::Proc(def) => self.call_proc(&def, argv),
            Command::ChildInterp(name) => self.dispatch_child(&name, argv),
            Command::OoObject(fqn) => self.oo_dispatch(&fqn, argv),
            Command::ParentAlias { target, prefix } => {
                self.dispatch_parent_alias(&target, &prefix, argv)
            }
        }
    }

    /// Register `cmd` under the (possibly qualified) name `name` — for the OO
    /// object/class commands.
    pub(crate) fn ns_register(&mut self, name: &[u8], cmd: Command) {
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
        let mut seed = n & 0x7FFF_FFFF;
        if seed == 0 || seed == 0x7FFF_FFFF {
            seed ^= 123_459_876;
        }
        self.rand_seed.set(Some(seed));
        self.rand_next()
    }

    /// `expr rand()`: advance the Park–Miller minimal-standard LCG and return a
    /// double in `(0, 1)` (C's `ExprRandFunc`). Seeds nondeterministically on
    /// first use if `srand` hasn't run.
    #[cfg(have_tommath)]
    pub(crate) fn rand_next(&self) -> f64 {
        // Constants from `ExprRandFunc`: IA=16807, IM=2^31-1, IQ=127773, IR=2836.
        const RAND_IA: i64 = 16807;
        const RAND_IM: i64 = 2_147_483_647;
        const RAND_IQ: i64 = 127_773;
        const RAND_IR: i64 = 2836;
        let mut seed = self.rand_seed.get().unwrap_or_else(|| {
            // Nondeterministic first seed, kept in [1, 2^31-2]. The wall clock
            // comes from the host (so the browser/WASI hosts seed it too).
            let t = self.host().clock().now_millis() as i64;
            let mut s = t & 0x7FFF_FFFF;
            if s == 0 || s == 0x7FFF_FFFF {
                s ^= 123_459_876;
            }
            s
        });
        let tmp = seed / RAND_IQ;
        seed = RAND_IA * (seed - tmp * RAND_IQ) - RAND_IR * tmp;
        if seed < 0 {
            seed += RAND_IM;
        }
        self.rand_seed.set(Some(seed));
        seed as f64 * (1.0 / RAND_IM as f64)
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

    /// Dispatch a child-interpreter command (`$child subcommand ?arg ...?`): the
    /// child is addressable like the `interp` ensemble restricted to it.
    fn dispatch_child(&mut self, name: &[u8], argv: &[*mut TclObj]) -> Code {
        if argv.len() < 2 {
            return self.error(b"wrong # args: should be \"interp cmd ?arg ...?\"");
        }
        match obj_bytes(argv[1]).as_slice() {
            b"eval" => {
                let script = join_words(&argv[2..]);
                self.eval_in_child(name, &script)
            }
            b"issafe" => {
                let safe = self.with_child(name, |c| c.is_safe()).unwrap_or(false);
                self.set_result_bytes(if safe { b"1" } else { b"0" });
                Code::Ok
            }
            b"delete" => {
                self.delete_child(name);
                self.set_result_bytes(b"");
                Code::Ok
            }
            b"hide" | b"expose" if argv.len() == 3 => {
                let hide = obj_bytes(argv[1]) == b"hide";
                // A safe interpreter may not touch any hidden-command table
                // (checked on the executing interp).
                if self.is_safe() {
                    return self.error(if hide {
                        b"permission denied: safe interpreter cannot hide commands"
                    } else {
                        b"permission denied: safe interpreter cannot expose commands"
                    });
                }
                let cmd = obj_bytes(argv[2]);
                self.with_child(name, |c| {
                    if hide {
                        c.hide_command(&cmd)
                    } else {
                        c.expose_command(&cmd)
                    }
                });
                self.set_result_bytes(b"");
                Code::Ok
            }
            b"invokehidden" if argv.len() >= 3 => {
                if self.is_safe() {
                    return self
                        .error(b"not allowed to invoke hidden commands from safe interpreter");
                }
                let cmd = obj_bytes(argv[2]);
                let hidden_argv: Vec<*mut TclObj> = argv[2..].to_vec();
                for &a in &hidden_argv {
                    unsafe { obj::incr_ref_count(a) };
                }
                let out = self.with_child(name, |c| {
                    (c.invoke_hidden(&cmd, &hidden_argv), c.result_bytes())
                });
                for &a in &hidden_argv {
                    unsafe { obj::decr_ref_count(a) };
                }
                match out {
                    Some((code, res)) => {
                        self.set_result_bytes(&res);
                        code
                    }
                    None => self.error(b"could not find interpreter"),
                }
            }
            b"hidden" => {
                let names = self
                    .with_child(name, |c| c.hidden_names())
                    .unwrap_or_default();
                let elems: Vec<*mut TclObj> =
                    names.iter().map(|n| obj::new_string_bytes(n)).collect();
                self.set_result(crate::list::new_list_obj(&elems));
                Code::Ok
            }
            b"aliases" => {
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
                    let mut m = b"wrong # args: should be \"".to_vec();
                    m.extend_from_slice(name);
                    m.extend_from_slice(b" recursionlimit ?newlimit?\"");
                    return self.error(&m);
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
                if argv.len() < 2 || argv.len() > 3 {
                    let mut m = b"wrong # args: should be \"".to_vec();
                    m.extend_from_slice(name);
                    m.extend_from_slice(b" bgerror ?cmdPrefix?\"");
                    return self.error(&m);
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
                let opts: Vec<*mut TclObj> = argv[2..].to_vec();
                if argv.len() > 4 {
                    return self
                        .error(b"wrong # args: should be \"interp debug path ?-frame ?bool??\"");
                }
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
                    let mut m = b"wrong # args: should be \"".to_vec();
                    m.extend_from_slice(name);
                    m.extend_from_slice(b" limit limitType ?-option value ...?\"");
                    return self.error(&m);
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
                let mut m = b"interp subcommand \"".to_vec();
                m.extend_from_slice(other);
                m.extend_from_slice(b"\" is not supported in this runtime");
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
        let mut child = Interp::new();
        // A child interpreter is another interpreter of the *same* Tcl build,
        // not a different release — C compiles one library in, so every child
        // reports and behaves as its parent's release. Inherited before the
        // globals are written, so the child's `tcl_version`/`tcl_patchLevel`
        // and its namespace-scope variable resolution both agree with the
        // parent (issue #1328). Resolution still runs against the child's
        // *own* global namespace: the rule is shared, the variables are not.
        child.set_runtime_version(self.runtime_version());
        // A (non-safe) child gets the predefined globals (`tcl_platform`, …) like
        // a real interpreter. The full `init.tcl` (package/auto-load) is deferred.
        child.set_startup_globals();
        // `interp debug -frame` is seeded from the creating interp's
        // `env(TCL_INTERP_DEBUG_FRAME)` (C's `Tcl_CreateChild`).
        if self
            .var_get_elem(b"env", b"TCL_INTERP_DEBUG_FRAME")
            .map(|o| parse_truth(&obj_bytes(o)))
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

    /// `interp hide name`: move command `name` out of the command table into the
    /// hidden table. Returns whether it existed.
    pub(crate) fn hide_command(&mut self, name: &[u8]) -> bool {
        let resolved = self.namespaces.borrow().resolve(GLOBAL, name);
        match resolved {
            Some(cmd) => {
                self.invalidate_interpreter_policy();
                self.namespaces.borrow_mut().delete(GLOBAL, name);
                self.hidden.borrow_mut().insert(name.to_vec(), cmd);
                self.invalidate_command_environment();
                true
            }
            None => false,
        }
    }

    /// `interp expose name`: move a hidden command back into the command table.
    pub(crate) fn expose_command(&mut self, name: &[u8]) -> bool {
        // Invalidate before removing from the hidden table. A missing entry may
        // over-invalidate, which is preferable to a re-entrant visibility gap.
        self.invalidate_interpreter_policy();
        let cmd = self.hidden.borrow_mut().remove(name);
        match cmd {
            Some(cmd) => {
                self.namespaces.borrow_mut().register(name, cmd);
                self.invalidate_command_environment();
                true
            }
            None => false,
        }
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

    /// Make this interp "safe": hide the commands that touch the host
    /// (filesystem, processes, sockets, the interpreter itself) — the core of
    /// `interp create -safe`. The Safe Base's re-aliasing of `source`/`load`/
    /// `file` is a follow-up (needs cross-interp aliases).
    pub(crate) fn make_safe(&mut self) {
        // Variable unsets below can fire callbacks. Stale existing tokens before
        // the first visibility/policy write, not after re-entrant code can run.
        self.invalidate_interpreter_policy();
        // Pinned against real tclsh 8.6.14 (`interp create -safe s; s hidden`):
        // `after` / `vwait` are deliberately NOT on this list — confirmed
        // present and callable inside a real safe child (`s eval {info
        // commands after}` returns `after`). An earlier version of this list
        // incorrectly hid them, which would have broken legitimate
        // safe-interp code using `after idle`/`after cancel`.
        const UNSAFE: &[&[u8]] = &[
            b"exec",
            b"exit",
            b"cd",
            b"pwd",
            b"glob",
            b"open",
            b"socket",
            b"source",
            b"load",
            b"file",
            b"fconfigure",
            b"encoding",
        ];
        for &c in UNSAFE {
            self.hide_command(c);
        }
        // Remove the host-revealing `tcl_platform` elements (C's `Tcl_MakeSafe`
        // unsets os/osVersion/machine/user) plus our backend-introspection keys,
        // so a safe interp exposes only the portable subset (byteOrder, engine,
        // pathSeparator, platform, pointerSize, wordSize).
        const UNSAFE_PLATFORM: &[&[u8]] = &[
            b"os",
            b"osVersion",
            b"machine",
            b"user",
            b"threaded",
            b"runtime",
            b"runtimeVersion",
            b"wasm",
            b"wasi",
            b"wasiVersion",
            b"ebpf",
        ];
        for &k in UNSAFE_PLATFORM {
            self.var_unset_elem(b"tcl_platform", k);
        }
        // A safe interp has no `env` array and no real library/package paths
        // (C's `Tcl_MakeSafe`); the Safe Base re-virtualises an `auto_path`.
        for v in [
            &b"env"[..],
            b"tcl_library",
            b"tclDefaultLibrary",
            b"tcl_pkgPath",
        ] {
            self.var_unset(v);
        }
        // A safe interp's `clock` is aliased to the parent's, so date/time
        // formatting works without the child reaching the timezone files.
        self.ns_register(
            b"clock",
            Command::ParentAlias {
                target: b"clock".to_vec(),
                prefix: Vec::new(),
            },
        );
        self.is_safe.set(true);
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
                if parse_truth(&obj_bytes(opts[1])) {
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
        self.frames.borrow_mut().pop();
        if !self.traces.borrow().traces.is_empty() {
            self.clear_frame_var_traces(proc_level);
        }
        self.current_ns.set(saved_ns);
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
        cfg: &crate::ensemble::EnsembleConfig,
        argv: &[*mut TclObj],
    ) -> Code {
        // `-parameters` formal args sit between the ensemble command and the
        // subcommand (`ens p1 p2 sub …`), so the subcommand is at `1 + nparams`.
        let nparams = cfg.parameters.len();
        if argv.len() < 2 + nparams {
            let mut m = b"wrong # args: should be \"".to_vec();
            m.extend_from_slice(&obj_bytes(argv[0]));
            // Each `-parameters` formal is rendered as a list element, so a
            // multi-word default like `{a b}` keeps its braces.
            for p in &cfg.parameters {
                m.push(b' ');
                crate::list::append_list_element(&mut m, p, false);
            }
            m.extend_from_slice(b" subcommand ?arg ...?\"");
            return self.error(&m);
        }
        // Resolve the subcommand. On a miss, the `-unknown` handler gets one
        // chance (`reparseCount < 1`) to define it (empty result ⇒ reparse) or
        // supply a replacement command prefix (non-empty result).
        let sub = obj_bytes(argv[1 + nparams]);
        let mut reparsed = false;
        loop {
            let subs = self.ensemble_subcommands(cfg);
            if let Some(idx) = crate::ensemble::resolve_subcommand(&subs, &sub, cfg.prefixes) {
                let resolved = &subs[idx];
                // The target command prefix: a `-map` entry, else `<ns>::<sub>`.
                let prefix: Vec<Vec<u8>> = cfg
                    .map
                    .as_ref()
                    .and_then(|m| {
                        m.iter()
                            .find(|(k, _)| k == resolved)
                            .map(|(_, p)| p.clone())
                    })
                    .unwrap_or_else(|| {
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
                source[1 + nparams] = resolved.clone();
                return self.dispatch_ensemble_target(&prefix, argv, nparams, source);
            }
            // Miss: try the `-unknown` handler once.
            if !cfg.unknown.is_empty() && !reparsed {
                reparsed = true;
                match self.ensemble_unknown(cfg, argv) {
                    EnsembleUnknown::Prefix(prefix) => {
                        let source: Vec<Vec<u8>> = argv.iter().map(|&a| obj_bytes(a)).collect();
                        return self.dispatch_ensemble_target(&prefix, argv, nparams, source);
                    }
                    EnsembleUnknown::Reparse => continue,
                    EnsembleUnknown::Failed(code) => return code,
                }
            }
            // A namespace ensemble with no subcommands at all gets a distinct
            // message; otherwise "unknown or ambiguous" (prefixes on) / "unknown"
            // (prefixes off) followed by the candidate list (C's
            // `NsEnsembleImplementationCmdNR`).
            let ecode = {
                let mut c = b"TCL LOOKUP SUBCOMMAND ".to_vec();
                c.extend_from_slice(&sub);
                c
            };
            if subs.is_empty() {
                let mut m = b"unknown subcommand \"".to_vec();
                m.extend_from_slice(&sub);
                m.extend_from_slice(b"\": namespace ");
                m.extend_from_slice(&self.namespaces.borrow().qualified_name(cfg.ns));
                m.extend_from_slice(b" does not export any commands");
                return self.error_with_code(&m, &ecode);
            }
            let mut m = if cfg.prefixes {
                b"unknown or ambiguous subcommand \"".to_vec()
            } else {
                b"unknown subcommand \"".to_vec()
            };
            m.extend_from_slice(&sub);
            m.extend_from_slice(b"\": must be ");
            m.extend_from_slice(&crate::ensemble::must_be(&subs));
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
    /// subcommand's own args follow). `nparams` = `cfg.parameters.len()`.
    fn dispatch_ensemble_target(
        &mut self,
        prefix: &[Vec<u8>],
        argv: &[*mut TclObj],
        nparams: usize,
        source: Vec<Vec<u8>>,
    ) -> Code {
        let mut new_argv: Vec<*mut TclObj> = Vec::with_capacity(prefix.len() + argv.len() - 1);
        for w in prefix {
            let o = new_string(w);
            // SAFETY: fresh obj; take the owning +1 the new argv holds.
            unsafe { obj::incr_ref_count(o) };
            new_argv.push(o);
        }
        for &a in &argv[1..1 + nparams] {
            // SAFETY: live arg; take an owning +1.
            unsafe { obj::incr_ref_count(a) };
            new_argv.push(a);
        }
        for &a in &argv[2 + nparams..] {
            // SAFETY: live arg; take an owning +1.
            unsafe { obj::incr_ref_count(a) };
            new_argv.push(a);
        }
        // Record the call as the user wrote it so a `wrong # args` from the target
        // is reported in ensemble terms (C's `TclInitRewriteEnsemble`): the
        // ensemble command, its `-parameters`, and the subcommand word (`2 +
        // nparams`) are removed; the target prefix + `-parameters` are inserted.
        let is_root = self.begin_ensemble_rewrite(source, 2 + nparams, prefix.len() + nparams);
        let code = self.dispatch(&new_argv);
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
        cfg: &crate::ensemble::EnsembleConfig,
        argv: &[*mut TclObj],
    ) -> EnsembleUnknown {
        let ens_fqn = self
            .resolve_cmd_fqn(&obj_bytes(argv[0]))
            .unwrap_or_else(|| obj_bytes(argv[0]));
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
        let code = self.dispatch(&hv);
        release_all(&hv);
        match code {
            Code::Ok => {
                let res = obj_bytes(self.get_obj_result());
                match crate::parse::split_list(&res) {
                    Ok(prefix) if !prefix.is_empty() => EnsembleUnknown::Prefix(prefix),
                    Ok(_) => EnsembleUnknown::Reparse,
                    Err(e) => EnsembleUnknown::Failed(self.error(e.message())),
                }
            }
            Code::Error => EnsembleUnknown::Failed(Code::Error),
            other => {
                let mut m = b"unknown subcommand handler returned bad code: ".to_vec();
                m.extend_from_slice(match other {
                    Code::Return => b"return".as_slice(),
                    Code::Break => b"break",
                    Code::Continue => b"continue",
                    _ => b"?",
                });
                EnsembleUnknown::Failed(self.error(&m))
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
        let cmd = self
            .namespaces
            .borrow()
            .resolve(self.current_ns.get(), &rel);
        let Some(cmd) = cmd else {
            let mut m = b"invalid command name \"tcl::mathfunc::".to_vec();
            m.extend_from_slice(fname);
            m.push(b'"');
            let mut error_code = b"TCL LOOKUP COMMAND tcl::mathfunc::".to_vec();
            error_code.extend_from_slice(fname);
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
    /// out naturally (the resolved target may itself be an `Alias`).
    fn dispatch_alias(&mut self, target: &[u8], prefix: &[Vec<u8>], argv: &[*mut TclObj]) -> Code {
        let target_cmd = self.namespaces.borrow().resolve(GLOBAL, target);
        let Some(target_cmd) = target_cmd else {
            // Lazily bound: the target was deleted (or never existed).
            return self.invalid_command(target);
        };
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
        let code = self.invoke(target_cmd, &new_argv);
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
        let body = crate::subst::scan(src, flags);
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
            assert!(i.hide_command(b"set"));
            assert_interpreter_guard_stale(i, token);

            let token = prepare_interpreter_guard(i);
            assert!(i.expose_command(b"set"));
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
                assert_eq!(i.eval_str(b"set x {}; expr {$x + 1}"), Code::Error, "{version:?}");
                assert_eq!(i.eval_str(b"set x { }; expr {$x + 1}"), Code::Error, "{version:?}");
                assert_eq!(i.eval_str(b"set x -; expr {$x + 1}"), Code::Error, "{version:?}");
            });
        }
    }

}
