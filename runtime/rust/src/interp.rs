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
//! The Zig runtime defers frees to a drain queue (`tcl_obj_drain_pending`) to
//! survive an aliasing hazard: releasing a command's argv after dispatch could
//! free the result if it aliased an argv element. We avoid the queue (and match
//! `tclObj.c`'s immediate `TclFreeObj`) because [`set_result`] **retains** the
//! result into the interp's result slot — so the slot holds an independent +1,
//! and releasing argv can never free a still-referenced result. Immediate free
//! + retain-into-result is the whole discipline.

use core::ffi::c_char;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crate::builtins;
use crate::frame::{FrameStack, Link, VarError};
use crate::namespace::{Namespaces, NsId, RenameOutcome, GLOBAL};
use crate::obj::{self, TclObj};
use crate::parse::{self, WordBody, WordPart};

/// Tcl completion codes (`tcl.h` `TCL_OK`..`TCL_CONTINUE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    Ok,
    Error,
    Return,
    Break,
    Continue,
}

impl Code {
    /// The Tcl integer completion code (`TCL_OK`=0 … `TCL_CONTINUE`=4) — what
    /// `catch` returns and `return -code` / the `-code` options-dict entry use.
    #[must_use]
    pub(crate) fn as_int(self) -> i64 {
        match self {
            Code::Ok => 0,
            Code::Error => 1,
            Code::Return => 2,
            Code::Break => 3,
            Code::Continue => 4,
        }
    }
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
    /// Added to a body-relative line to get the reported `line`. `0` for
    /// top-level / `eval` / eval-defined procs (body-relative, matching tclsh);
    /// for a proc defined in a `source`d file it is the file line where the body
    /// began minus one, so its commands report file-absolute lines.
    line_base: u32,
    /// The currently-executing command at this level (the `cmd` key) and its
    /// reported source line (the `line` key).
    cmd: Vec<u8>,
    line: u32,
    /// TclOO method context for `info frame`: `(method-name, declarer-kind,
    /// declarer-name)` where kind is `class`/`object`. Present for a method
    /// body, where C reports `method`/`class`|`object` instead of `proc`.
    /// `method-name` is empty for a constructor/destructor.
    oo: Option<(Vec<u8>, Vec<u8>, Vec<u8>)>,
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
            line_base: 0,
            cmd: Vec::new(),
            line: 1,
            oo: None,
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
    /// `::errorCode` (empty ⇒ the `NONE` default is applied when published).
    code: Vec<u8>,
    /// 1-based source line of the innermost logged command, within its own
    /// script (`errorLine`); read by `MakeProcError`'s `line N`.
    line: u32,
    /// `ERR_ALREADY_LOGGED`: the current command has already been logged deeper
    /// in the same script, so its enclosing command must not re-log it.
    already_logged: bool,
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
    arg_lines: Vec<u32>,
    eval_depth: usize,
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

/// A registered command. `Builtin` and the `Alias` redirect today; `Proc {
/// params, body }` (user procs, T1.5) and `External { table_index, client_data }`
/// (extension commands via `Tcl_CreateObjCommand`, §4.6/§13.2, Track 2) are the
/// next variants.
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
pub struct InterpState {
    pub(crate) frames: RefCell<FrameStack>,
    /// The command-table-as-core-service: the namespace tree + the one
    /// `resolve(currentNs, name)` resolver (T1.5).
    namespaces: RefCell<Namespaces>,
    /// The current namespace for command resolution (the eval context; a proc
    /// runs in its *defining* namespace — wired with procs). Global at top level.
    current_ns: Cell<NsId>,
    /// Active proc-call nesting depth — C Tcl's `interp recursionlimit`. Bounds
    /// recursion so an infinite proc loop raises a catchable error instead of
    /// overflowing the (wasm) stack (the tracked PR #557 follow-up).
    recursion_depth: Cell<usize>,
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
    eval_depth: Cell<usize>,
    /// Count of commands dispatched (`info cmdcount`).
    cmd_count: Cell<u64>,
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
    result: Cell<*mut TclObj>,
}

/// An ensemble-rewrite record (see `InterpState::ensemble_rewrite`).
#[derive(Clone)]
pub(crate) struct EnsembleRewrite {
    /// The original command words (e.g. `foo test 1 2 3`).
    pub source: Vec<Vec<u8>>,
    /// How many leading `source` words the rewritten prefix stands in for.
    pub removed: usize,
}

/// The proc-call recursion bound (C Tcl's default `interp recursionlimit`).
const RECURSION_LIMIT: usize = 1000;

impl Interp {
    /// Create an interp: global frame, the built-in command set, an empty
    /// result.
    pub fn new() -> Interp {
        let result = obj::new_obj();
        // SAFETY: `result` is freshly created; the interp takes the owning ref.
        unsafe { obj::incr_ref_count(result) };
        let mut interp = Interp(Rc::new(InterpState {
            frames: RefCell::new(FrameStack::new()),
            namespaces: RefCell::new(Namespaces::new()),
            current_ns: Cell::new(GLOBAL),
            recursion_depth: Cell::new(0),
            packages: RefCell::new(crate::cmd_package::PackageState::with_core()),
            script_stack: RefCell::new(Vec::new()),
            channels: RefCell::new(crate::cmd_chan::ChannelTable::default()),
            return_code: Cell::new(Code::Ok),
            return_level: Cell::new(1),
            traces: RefCell::new(crate::cmd_trace::TraceTable::default()),
            exc: RefCell::new(ExceptionState::default()),
            children: RefCell::new(std::collections::BTreeMap::new()),
            interp_counter: Cell::new(0),
            hidden: RefCell::new(std::collections::BTreeMap::new()),
            parent: RefCell::new(Weak::new()),
            is_safe: Cell::new(false),
            eval_active: Cell::new(0),
            pending_delete: Cell::new(false),
            oo: RefCell::new(crate::cmd_oo::OoState::default()),
            cmd_frames: RefCell::new(Vec::new()),
            arg_lines: RefCell::new(Vec::new()),
            arg_locs: RefCell::new(Vec::new()),
            eval_depth: Cell::new(0),
            cmd_count: Cell::new(0),
            bgerror: RefCell::new(Vec::new()),
            bg_queue: RefCell::new(Vec::new()),
            events: RefCell::new(crate::cmd_event::EventQueue::default()),
            coros: RefCell::new(std::collections::BTreeMap::new()),
            ensemble_rewrite: RefCell::new(None),
            result: Cell::new(result),
        }));
        builtins::install(&mut interp);
        interp
    }

    // -- command registry -----------------------------------------------------

    /// Register a built-in command (a possibly-qualified `name`, creating
    /// intermediate namespaces; overwrites any existing command of `name`).
    pub fn register_builtin(&mut self, name: &[u8], f: BuiltinFn) {
        self.namespaces
            .borrow_mut()
            .register(name, Command::Builtin(f));
    }

    /// Command names in the current namespace, sorted (`info commands`).
    #[must_use]
    pub fn command_names(&self) -> Vec<Vec<u8>> {
        self.namespaces
            .borrow()
            .command_names(self.current_ns.get())
            .iter()
            .map(|s| s.to_vec())
            .collect()
    }

    /// `rename old new` (or `rename old ""` to delete), relative to the current
    /// namespace. Drives the one command table; see [`Namespaces::rename`].
    pub(crate) fn rename_command(&mut self, old: &[u8], new: &[u8]) -> RenameOutcome {
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
                // The trace list (and any OO object) follows to the new name.
                RenameOutcome::Renamed => {
                    let nf = self.fqn_for(new);
                    self.move_cmd_traces(&of, &nf);
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
        // If `name` is a suspended coroutine, terminate its worker first.
        crate::cmd_coro::on_command_deleted(self, name);
        self.namespaces
            .borrow_mut()
            .delete(self.current_ns.get(), name)
    }

    /// Register an ensemble command (`namespace ensemble create`); `name` is the
    /// ensemble command (possibly qualified — rooted at global like any builtin).
    pub(crate) fn create_ensemble(&mut self, name: &[u8], cfg: crate::ensemble::EnsembleConfig) {
        // The `-command` name resolves relative to the current namespace, like a
        // proc name (C's `TclGetNamespaceForQualName(name, cxtPtr=nsPtr, ...)` in
        // `NamespaceEnsembleCmd`). `namespace ensemble create -command path`
        // inside `namespace eval ::tcl::tm` therefore binds `::tcl::tm::path`,
        // not a bare `::path` at global scope.
        let ns = self
            .namespaces
            .borrow_mut()
            .command_home_ns(self.current_ns.get(), name);
        let tail = tcl_syntax::naming::qualifier_segments(name)
            .last()
            .copied()
            .unwrap_or(name)
            .to_vec();
        self.namespaces
            .borrow_mut()
            .bind(ns, &tail, Command::Ensemble(cfg));
    }

    /// Define a user proc (`proc name params body`). The proc's defining
    /// namespace (where its body runs, and where it is bound) is the namespace
    /// `name` lands in — **relative to the current namespace** (so `proc next`
    /// inside `namespace eval counter` binds `::counter::next`, not a global).
    pub(crate) fn define_proc(&mut self, name: &[u8], params: Vec<Param>, body_obj: *mut TclObj) {
        let body = obj_bytes(body_obj);
        let ns = self
            .namespaces
            .borrow_mut()
            .command_home_ns(self.current_ns.get(), name);
        let tail = tcl_syntax::naming::qualifier_segments(name)
            .last()
            .copied()
            .unwrap_or(name)
            .to_vec();
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
    /// (`namespace ensemble configure` set form). `name` must already resolve to
    /// an ensemble; rebinds at the same location `create_ensemble` would choose.
    pub(crate) fn set_ensemble_config(
        &mut self,
        name: &[u8],
        cfg: crate::ensemble::EnsembleConfig,
    ) {
        self.create_ensemble(name, cfg);
    }

    /// Every alias command's name across the whole tree (`interp aliases`).
    pub(crate) fn alias_names(&self) -> Vec<Vec<u8>> {
        self.namespaces.borrow().alias_names()
    }

    /// The current namespace (the eval context) — for the `namespace` builtin.
    pub(crate) fn current_ns(&self) -> NsId {
        self.current_ns.get()
    }

    /// Begin an ensemble-rewrite (a forward / ensemble / constructor replacing
    /// the original command words). Returns `true` if this is the *root* rewrite
    /// (no rewrite was active) — the caller must `clear_ensemble_rewrite` when
    /// its dispatch returns. A nested rewrite is ignored (the root's `source` is
    /// what `wrong # args` reports), matching the common case of C's
    /// `TclInitRewriteEnsemble` chaining.
    pub(crate) fn begin_ensemble_rewrite(&self, source: Vec<Vec<u8>>, removed: usize) -> bool {
        let mut rw = self.ensemble_rewrite.borrow_mut();
        if rw.is_some() {
            return false;
        }
        *rw = Some(EnsembleRewrite { source, removed });
        true
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
        self.namespaces.borrow_mut()
    }

    /// Bootstrap the standard library like C's `Tcl_Init`: set the startup
    /// globals (`tcl_library` from `$TCL_LIBRARY`, version/platform/env/argv,
    /// `auto_path`), then `source $tcl_library/init.tcl`. After this the
    /// pure-Tcl `unknown`/auto-load/`package` machinery is live, so
    /// `package require` works through `pkgIndex.tcl`/`tclIndex`.
    /// Set the predefined startup globals (`tcl_version`/`tcl_platform`/`env`/
    /// `argv`/…) — the cheap half of `Tcl_Init`, shared by the main interp's
    /// `init_library` and each child interpreter (`interp create`).
    pub(crate) fn set_startup_globals(&mut self) {
        let lib = std::env::var("TCL_LIBRARY").unwrap_or_default();
        let set = |i: &mut Interp, name: &[u8], val: &[u8]| {
            let o = new_string(val);
            if i.var_set(name, o).is_err() {
                drop_fresh(o);
            }
        };
        set(self, b"::tcl_library", lib.as_bytes());
        set(self, b"::tcl_version", b"9.0");
        set(self, b"::tcl_patchLevel", b"9.0.3");
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
        // env array from the host environment (no quoting hazards via var_set_elem).
        for (k, v) in std::env::vars() {
            let o = new_string(v.as_bytes());
            if self.var_set_elem(b"env", k.as_bytes(), o).is_err() {
                drop_fresh(o);
            }
        }
    }

    pub fn init_library(&mut self) -> Code {
        self.set_startup_globals();
        let lib = std::env::var("TCL_LIBRARY").unwrap_or_default();
        // Source init.tcl, which sets up unknown/auto-load/package + appends
        // tcl_library (and its parent) to auto_path.
        let init_path = format!("{lib}/init.tcl");
        match std::fs::read(&init_path) {
            Ok(bytes) => self.eval_sourced(&bytes, init_path.as_bytes()),
            Err(_) => {
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
    /// frame `target_level` (the Zig oracle's "restore caller ns + frame depth
    /// together" discovery), then restore. Transparent — the body's completion
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
        self.ns_eval_framed(name, body, None)
    }

    /// `namespace eval` of a single body **object** — like
    /// [`ns_eval`](Self::ns_eval), but a literal obj with a recorded TIP 280
    /// source location runs as `type source` at its file+line (so a `proc`/
    /// command defined inside `namespace eval { … }` reports file-absolute
    /// `info frame` lines), rather than the dynamic `type eval`.
    pub(crate) fn ns_eval_obj(&mut self, name: &[u8], obj: *mut TclObj) -> Code {
        let loc = self.arg_loc(obj);
        let bytes = obj_bytes(obj);
        self.ns_eval_framed(name, &bytes, loc)
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
        let frame = CmdFrame {
            kind,
            file,
            proc: None,
            level: self.frames.borrow().current_level(),
            omit_level: false,
            line_base,
            cmd: Vec::new(),
            line: 1,
            oo: None,
        };
        let code = self.eval_framed(body, frame);
        self.frames.borrow_mut().pop();
        self.current_ns.set(saved);
        code
    }

    /// Whether the active variable frame is a proc call frame (vs. global /
    /// `namespace eval` scope).
    pub(crate) fn in_proc(&self) -> bool {
        self.frames.borrow().in_proc()
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

    pub(crate) fn trace_var_ns(&self, base: &[u8]) -> Option<NsId> {
        crate::vars::home_namespace(
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
        if t.traces.iter().any(|v| v.frame_level == Some(level)) {
            t.traces.retain(|v| v.frame_level != Some(level));
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

    /// `unset name` — returns whether it existed.
    pub(crate) fn var_unset(&mut self, name: &[u8]) -> bool {
        let existed = crate::vars::unset(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
        );
        if existed && !self.traces.borrow().traces.is_empty() {
            let (base, elem) = crate::frame::split_array_ref(name);
            self.fire_var_trace(&base, elem.as_deref(), b"unset");
            // The variable (and its traces) go away — drop every trace on it
            // (C frees the Var's trace list on unset). Element unset drops only
            // that element's traces (whole-variable traces survive).
            let mut t = self.traces.borrow_mut();
            match elem {
                Some(e) => t
                    .traces
                    .retain(|v| !(v.base == base && v.elem.as_deref() == Some(e.as_slice()))),
                None => t.traces.retain(|v| v.base != base),
            }
        }
        existed
    }

    /// `unset name(key)` — returns whether it existed.
    pub(crate) fn var_unset_elem(&mut self, name: &[u8], key: &[u8]) -> bool {
        let existed = crate::vars::unset_elem(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
            key,
        );
        if existed && !self.traces.borrow().traces.is_empty() {
            self.fire_var_trace(name, Some(key), b"unset");
            // Drop this element's traces (whole-array traces survive).
            self.traces
                .borrow_mut()
                .traces
                .retain(|v| !(v.base == name && v.elem.as_deref() == Some(key)));
        }
        existed
    }

    /// Invoke every variable trace matching `(base, elem, op)`, as
    /// `command base element op`. Re-entrant firing is suppressed (the `firing`
    /// guard); the interp result is preserved across the callbacks (the
    /// triggering operation owns the result). For `read`/`write` ops a callback
    /// error is **propagated**: the message is stashed in `pending_err` and the
    /// function returns `true` (the access then fails; C's `TclCallVarTraces`).
    /// `unset`/`array` errors are ignored (C does too). Returns whether a
    /// read/write callback errored.
    fn fire_var_trace(&mut self, base: &[u8], elem: Option<&[u8]>, op: &[u8]) -> bool {
        if self.traces.borrow().firing > 0 {
            return false;
        }
        let access_ns = self.trace_var_ns(base);
        let cmds: Vec<Vec<u8>> = self
            .traces
            .borrow()
            .traces
            .iter()
            .filter(|t| crate::cmd_trace::matches(t, base, elem, op, access_ns))
            .map(|t| t.command.clone())
            .collect();
        if cmds.is_empty() {
            return false;
        }
        let propagate = op == b"read" || op == b"write";
        // Preserve the result object across the callbacks.
        let saved = self.result.get();
        unsafe { obj::incr_ref_count(saved) };

        self.traces.borrow_mut().firing += 1;
        let mut errored = false;
        for cmd in cmds {
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
            let code = self.eval_str(&line);
            if propagate && code == Code::Error {
                // Capture the callback's error message; stop firing (C aborts
                // the trace chain on the first error).
                let msg = self.result_bytes();
                self.traces.borrow_mut().pending_err = Some(msg);
                errored = true;
                break;
            }
        }
        self.traces.borrow_mut().firing -= 1;

        // Restore the saved result (release the trace's, adopt our held +1).
        unsafe {
            obj::decr_ref_count(self.result.get());
            self.result.set(saved);
        }
        errored
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
        for t in self.traces.borrow_mut().cmd_traces.iter_mut() {
            if t.name == old_fqn {
                t.name = new_fqn.to_vec();
            }
        }
    }

    /// Drop every command/execution trace on `fqn` (the command is gone).
    fn remove_cmd_traces(&mut self, fqn: &[u8]) {
        self.traces
            .borrow_mut()
            .cmd_traces
            .retain(|t| t.name != fqn);
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
        self.namespaces
            .borrow_mut()
            .ensure_namespace(self.current_ns.get(), name)
    }

    /// Delete the namespace `ns` (by id), e.g. an OO object's instance namespace
    /// when the object is destroyed.
    pub(crate) fn delete_namespace_by_id(&mut self, ns: NsId) {
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
        victims
    }

    /// Fire collected unset-trace callbacks as `command name {} unset`. Errors
    /// are ignored (an unset trace's result is discarded, as in C).
    fn fire_unset_callbacks(&mut self, victims: Vec<(Vec<u8>, Vec<u8>)>) {
        if victims.is_empty() || self.traces.borrow().firing > 0 {
            return;
        }
        let saved = self.result.get();
        unsafe { obj::incr_ref_count(saved) };
        self.traces.borrow_mut().firing += 1;
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
        self.traces.borrow_mut().firing -= 1;
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
    /// (splitting `arr(key)`, the Zig discovery).
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

    /// `info level` — the current frame level (proc nesting depth).
    pub(crate) fn level(&self) -> usize {
        self.frames.borrow().current_level()
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

    /// `info locals` — the active frame's local variable names (links such as
    /// `global`/`variable`/`upvar` and auto-linked instance vars are excluded).
    pub(crate) fn local_var_names(&self) -> Vec<Vec<u8>> {
        self.frames.borrow().local_names_no_links()
    }

    /// `info globals` — the global namespace's variable names.
    pub(crate) fn global_var_names(&self) -> Vec<Vec<u8>> {
        self.namespaces.borrow().var_names(GLOBAL)
    }

    /// Command names visible from the current namespace (`info commands`):
    /// current ns ∪ global, sorted/deduped.
    pub(crate) fn visible_command_names(&self) -> Vec<Vec<u8>> {
        let ns = self.namespaces.borrow();
        let cur = self.current_ns.get();
        let mut v: Vec<Vec<u8>> = ns.command_names(cur).iter().map(|s| s.to_vec()).collect();
        if cur != GLOBAL {
            v.extend(ns.command_names(GLOBAL).iter().map(|s| s.to_vec()));
        }
        v.sort();
        v.dedup();
        v
    }

    /// Simple command names in the namespace named `qualifier` (absolute or
    /// relative to the current namespace), or empty if it does not exist — for
    /// a namespace-qualified `info commands ::ns::pattern`.
    /// The canonical fully-qualified prefix (ending in `::`) of the namespace a
    /// pattern qualifier addresses (`info commands ns::pat`): `::` for the global
    /// namespace, `::a::b::` otherwise. Resolves a *relative* qualifier against
    /// the current namespace, so `info commands` results are always absolute
    /// (matching C, where names are re-qualified through the namespace's
    /// `fullName`). `None` if the namespace doesn't exist.
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

    pub(crate) fn commands_in_namespace(&self, qualifier: &[u8]) -> Vec<Vec<u8>> {
        let ns = self.namespaces.borrow();
        // An empty qualifier (a leading `::pattern`) addresses the global ns.
        let target = if qualifier.is_empty() {
            Some(GLOBAL)
        } else {
            ns.find_namespace(self.current_ns.get(), qualifier)
        };
        match target {
            Some(id) => {
                let mut v: Vec<Vec<u8>> = ns.command_names(id).iter().map(|s| s.to_vec()).collect();
                v.sort();
                v
            }
            None => Vec::new(),
        }
    }

    /// Simple proc names in the namespace named `qualifier` (`info procs
    /// ::ns::pattern`), or empty if it does not exist.
    pub(crate) fn procs_in_namespace(&self, qualifier: &[u8]) -> Vec<Vec<u8>> {
        let ns = self.namespaces.borrow();
        let target = if qualifier.is_empty() {
            Some(GLOBAL)
        } else {
            ns.find_namespace(self.current_ns.get(), qualifier)
        };
        match target {
            Some(id) => {
                let mut v = ns.proc_names(id);
                v.sort();
                v
            }
            None => Vec::new(),
        }
    }

    /// Proc names visible from the current namespace (`info procs`).
    pub(crate) fn visible_proc_names(&self) -> Vec<Vec<u8>> {
        let ns = self.namespaces.borrow();
        let cur = self.current_ns.get();
        let mut v = ns.proc_names(cur);
        if cur != GLOBAL {
            v.extend(ns.proc_names(GLOBAL));
        }
        v.sort();
        v.dedup();
        v
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

    /// `Tcl_GetObjResult` — borrowed (interp keeps its +1).
    pub fn get_obj_result(&self) -> *mut TclObj {
        self.result.get()
    }

    /// The current result's string bytes (copied).
    pub fn result_bytes(&self) -> Vec<u8> {
        obj_bytes(self.result.get())
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
            line: 1,
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
            line: 1,
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
        *self.bgerror.borrow_mut() = prefix.to_vec();
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
            line: 1,
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
            line: 1,
            already_logged: false,
        };
        Code::Error
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
        // errorLine = 1 + count('\n' in src[0..commandStart]) — C's exact loop.
        let line = line_of(src, cmd.start);
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
        exc.line = line;
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

    pub(crate) fn append_frame_line(&mut self, inner: &[u8]) {
        let line = self.exc.borrow().line;
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

    /// The current accumulated `errorInfo` (for `catch`'s `-errorinfo`): the
    /// trace if any frame was logged, else the bare error message.
    pub(crate) fn error_info(&self) -> Vec<u8> {
        let info = self.exc.borrow().info.clone();
        info.unwrap_or_else(|| self.result_bytes())
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
        // A TclOO method frame reports `method`/`class`|`object` (the declarer)
        // in place of `proc` (C's `TclInfoFrame`).
        if let Some((method, kind, owner)) = &f.oo {
            if !method.is_empty() {
                pairs.push((b"method".to_vec(), method.clone()));
            }
            pairs.push((kind.clone(), owner.clone()));
        } else if let Some(p) = &f.proc {
            pairs.push((b"proc".to_vec(), p.clone()));
        }
        // `level` is the distance from the current call level (omitted for a
        // redirected `uplevel` scope, matching C's reachability check).
        if !f.omit_level {
            let level = self.frames.borrow().current_level().saturating_sub(f.level);
            pairs.push((b"level".to_vec(), level.to_string().into_bytes()));
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
        if exc.code.is_empty() {
            b"NONE".to_vec()
        } else {
            exc.code.clone()
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
    }

    /// Publish + reset, for `catch`/`try` once they have captured the options.
    pub(crate) fn publish_and_reset_error(&mut self) {
        self.publish_error();
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
    fn eval_framed(&mut self, src: &[u8], frame: CmdFrame) -> Code {
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
            line_base,
            cmd: Vec::new(),
            line: 1,
            oo: top.and_then(|f| f.oo.clone()),
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
            return self.dispatch_list_obj(body);
        }
        if let Some((file, line)) = self.arg_loc(body) {
            let mut frame = self.inherited_cmd_frame();
            frame.kind = FrameKind::Source;
            frame.file = file;
            frame.line_base = line.saturating_sub(1);
            let bytes = obj_bytes(body);
            return self.eval_framed(&bytes, frame);
        }
        self.eval_unlocated_body(&obj_bytes(body))
    }

    /// The TIP 280 source location recorded for `obj` (the literal-argument
    /// location table), or `None` for a dynamic value. Lets a command that
    /// re-splits a literal (e.g. `switch`'s single-list-arg form) recover the
    /// enclosing file + the list word's line, to derive its sub-bodies' lines.
    pub(crate) fn arg_location(&self, obj: *mut TclObj) -> Option<(Option<Rc<[u8]>>, u32)> {
        self.arg_loc(obj)
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

    /// Evaluate a body whose source location is unknown (C's switch `line = -1`
    /// case: a dynamically-built list body). It runs as `type eval` with **no
    /// file**, so a command defined inside (e.g. `proc`) is body-relative
    /// (`type proc`, line 1) rather than inheriting the enclosing file's lines.
    pub(crate) fn eval_unlocated_body(&mut self, body: &[u8]) -> Code {
        let mut frame = self.inherited_cmd_frame();
        frame.kind = FrameKind::Eval;
        frame.file = None;
        frame.line_base = 0;
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
        // contained literal keeps its source location — C's list-eval path).
        if crate::list::is_pure_list(obj) {
            return self.dispatch_list_obj(obj);
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
    fn dispatch_list_obj(&mut self, obj: *mut TclObj) -> Code {
        let elems = match crate::list::list_elements(obj) {
            Ok(e) => e,
            Err(e) => return self.error(e.message()),
        };
        if elems.is_empty() {
            self.set_result_bytes(b"");
            return Code::Ok;
        }
        for &e in &elems {
            // SAFETY: live element; take an owning +1 for the call.
            unsafe { obj::incr_ref_count(e) };
        }
        let code = self.dispatch(&elems);
        release_all(&elems);
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
            // Pure list → one command by element identity (see `dispatch_list_obj`).
            self.dispatch_list_obj(obj)
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
        if let Some(cmd) = resolved {
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

    /// The `info cmdtype` classification of `name`, or `None` if no such command.
    pub(crate) fn cmdtype(&self, name: &[u8]) -> Option<&'static [u8]> {
        let cmd = self
            .namespaces
            .borrow()
            .resolve(self.current_ns.get(), name)?;
        Some(match cmd {
            Command::Builtin(_) => b"native",
            Command::Proc(_) => b"proc",
            Command::Alias { .. } | Command::ParentAlias { .. } => b"alias",
            Command::Imported { .. } => b"import",
            Command::Ensemble(_) => b"ensemble",
            Command::OoObject(_) => b"object",
            Command::ChildInterp(_) => b"native",
        })
    }

    /// The `::tcl::mathfunc::*` function names (`info functions`).
    pub(crate) fn mathfunc_names(&self) -> Vec<Vec<u8>> {
        let ns = self.namespaces.borrow();
        match ns.find_namespace(GLOBAL, b"::tcl::mathfunc") {
            Some(id) => ns.command_names(id).iter().map(|n| n.to_vec()).collect(),
            None => Vec::new(),
        }
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
        let name = name.unwrap_or_else(|| {
            let n = format!("interp{}", self.interp_counter.get());
            self.interp_counter.set(self.interp_counter.get() + 1);
            n.into_bytes()
        });
        let mut child = Interp::new();
        // A (non-safe) child gets the predefined globals (`tcl_platform`, …) like
        // a real interpreter. The full `init.tcl` (package/auto-load) is deferred.
        child.set_startup_globals();
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
        let saved_parent = child.parent.replace(Rc::downgrade(&self.0));
        child.eval_active.set(child.eval_active.get() + 1);
        let r = f(&mut child);
        child.eval_active.set(child.eval_active.get() - 1);
        *child.parent.borrow_mut() = saved_parent;
        let teardown = child.pending_delete.get() && child.eval_active.get() == 0;
        drop(child); // release our handle clone before freeing the table's
        if teardown {
            self.children.borrow_mut().remove(name);
            self.namespaces.borrow_mut().delete(GLOBAL, name);
        }
        Some(r)
    }

    /// `interp hide name`: move command `name` out of the command table into the
    /// hidden table. Returns whether it existed.
    pub(crate) fn hide_command(&mut self, name: &[u8]) -> bool {
        let resolved = self.namespaces.borrow().resolve(GLOBAL, name);
        match resolved {
            Some(cmd) => {
                self.namespaces.borrow_mut().delete(GLOBAL, name);
                self.hidden.borrow_mut().insert(name.to_vec(), cmd);
                true
            }
            None => false,
        }
    }

    /// `interp expose name`: move a hidden command back into the command table.
    pub(crate) fn expose_command(&mut self, name: &[u8]) -> bool {
        let cmd = self.hidden.borrow_mut().remove(name);
        match cmd {
            Some(cmd) => {
                self.namespaces.borrow_mut().register(name, cmd);
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
            b"after",
            b"vwait",
        ];
        for &c in UNSAFE {
            self.hide_command(c);
        }
        self.is_safe.set(true);
    }

    /// Whether this interp is safe (`interp issafe`).
    pub(crate) fn is_safe(&self) -> bool {
        self.is_safe.get()
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
                if remove_now {
                    self.children.borrow_mut().remove(name);
                }
                self.namespaces.borrow_mut().delete(GLOBAL, name);
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
            return self.error(&self.proc_wrong_args(usage, params, supplied));
        }
        // Recursion bound (catchable, not a stack overflow).
        if self.recursion_depth.get() >= RECURSION_LIMIT {
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
                return self.error(&self.proc_wrong_args(usage, params, supplied));
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
        let proc_frame = CmdFrame {
            kind: if meta.source.is_some() {
                FrameKind::Source
            } else {
                FrameKind::Proc
            },
            file: meta.source,
            proc: meta.fqn.map(<[u8]>::to_vec),
            level: self.frames.borrow().current_level(),
            omit_level: false,
            line_base: meta.body_line_base,
            cmd: Vec::new(),
            line: 1,
            oo,
        };
        let code = self.eval_framed(body, proc_frame);
        // The frame's local variables (and any traces on them) die with it.
        let proc_level = self.frames.borrow().current_level();
        self.frames.borrow_mut().pop();
        if !self.traces.borrow().traces.is_empty() {
            self.clear_frame_var_traces(proc_level);
        }
        self.current_ns.set(saved_ns);
        self.recursion_depth.set(self.recursion_depth.get() - 1);
        // Apply the return boundary (`return`/`return -code -level`), then a
        // bare `break`/`continue` that escaped the body (no enclosing loop) is an
        // error (C Tcl: `invoked "break" outside of a loop`).
        let settled = match self.settle_return(code) {
            Code::Break if !meta.keep_loop_codes => {
                self.error(b"invoked \"break\" outside of a loop")
            }
            Code::Continue if !meta.keep_loop_codes => {
                self.error(b"invoked \"continue\" outside of a loop")
            }
            other => other,
        };
        // On error, append the `(procedure "name" line N)` / `(lambda term ...)`
        // frame and clear `already_logged` so the proc-call command logs next.
        if settled == Code::Error {
            self.make_proc_error(meta.err);
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
            for p in &cfg.parameters {
                m.push(b' ');
                m.extend_from_slice(p);
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
                return self.dispatch_ensemble_target(&prefix, argv, nparams);
            }
            // Miss: try the `-unknown` handler once.
            if !cfg.unknown.is_empty() && !reparsed {
                reparsed = true;
                match self.ensemble_unknown(cfg, argv) {
                    EnsembleUnknown::Prefix(prefix) => {
                        return self.dispatch_ensemble_target(&prefix, argv, nparams);
                    }
                    EnsembleUnknown::Reparse => continue,
                    EnsembleUnknown::Failed(code) => return code,
                }
            }
            // "unknown or ambiguous" with prefixes on; plain "unknown" otherwise.
            let mut m = if cfg.prefixes {
                b"unknown or ambiguous subcommand \"".to_vec()
            } else {
                b"unknown subcommand \"".to_vec()
            };
            m.extend_from_slice(&sub);
            m.extend_from_slice(b"\": must be ");
            m.extend_from_slice(&crate::ensemble::must_be(&subs));
            return self.error(&m);
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
        let code = self.dispatch(&new_argv);
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
        let mut full = b"::tcl::mathfunc::".to_vec();
        full.extend_from_slice(fname);
        let cmd = self.namespaces.borrow().resolve(GLOBAL, &full);
        let Some(cmd) = cmd else {
            let mut m = b"invalid command name \"tcl::mathfunc::".to_vec();
            m.extend_from_slice(fname);
            m.push(b'"');
            return self.error(&m);
        };
        // Build [name, args…], each owned (+1), and invoke the resolved command
        // directly (no re-resolution through current_ns).
        let mut argv: Vec<*mut TclObj> = Vec::with_capacity(args.len() + 1);
        let name_obj = new_string(&full);
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
                // Object-passthrough fast path (Zig lesson #1/#4): a word that is
                // *exactly one* substitution returns that value's **object**
                // (preserving its internal rep), not a stringified copy. This is
                // what keeps `$list`→`lindex`/`llength` etc. O(1) instead of
                // re-shimmering the string each access (the hidden-O(N²) seam).
                if parts.len() == 1 {
                    match &parts[0] {
                        WordPart::Variable(v) => {
                            let index = match &v.index {
                                Some(p) => Some(self.subst_index(p)?),
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
                                Some(parts) => Some(self.subst_index(parts)?),
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
                    let index = match &v.index {
                        Some(p) => Some(self.subst_index(p)?),
                        None => None,
                    };
                    if let Some(c) = self.fire_read_trace(v.name, index.as_deref()) {
                        return Err(c);
                    }
                    match self.read_var(v.name, index.as_deref()) {
                        Some(bytes) => out.extend_from_slice(&bytes),
                        None => return Err(self.no_such_variable(v.name, index.as_deref())),
                    }
                }
                WordPart::Command(script) => {
                    let code = self.eval_command_subst(src, script);
                    if code != Code::Ok {
                        return Err(code);
                    }
                    out.extend_from_slice(&self.result_bytes());
                }
            }
        }
        Ok(out)
    }

    /// Resolve a `$arr(index)` index (itself substituted) to its bytes.
    fn subst_index(&mut self, parts: &[WordPart]) -> Result<Vec<u8>, Code> {
        let mut buf = Vec::new();
        for part in parts {
            match part {
                WordPart::Text(b) => buf.extend_from_slice(b),
                WordPart::Variable(v) => {
                    let idx = match &v.index {
                        Some(p) => Some(self.subst_index(p)?),
                        None => None,
                    };
                    match self.read_var(v.name, idx.as_deref()) {
                        Some(bytes) => buf.extend_from_slice(&bytes),
                        None => return Err(self.no_such_variable(v.name, idx.as_deref())),
                    }
                }
                WordPart::Command(script) => {
                    let code = self.eval_str(script);
                    if code != Code::Ok {
                        return Err(code);
                    }
                    buf.extend_from_slice(&self.result_bytes());
                }
            }
        }
        Ok(buf)
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
        } else {
            msg.extend_from_slice(b"no such variable");
        }
        msg
    }

    /// Set an error result and return [`Code::Error`] — for builtins.
    pub(crate) fn set_error(&mut self, msg: &[u8]) -> Code {
        self.error(msg)
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
    ) -> Vec<u8> {
        if let Some(rw) = self.ensemble_rewrite() {
            // How many trailing words of `source` were the user's own arguments,
            // and how many formal parameters the inserted prefix already filled.
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
        proc_usage(called, params)
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

fn proc_usage(called: &[u8], params: &[Param]) -> Vec<u8> {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(called);
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
}
