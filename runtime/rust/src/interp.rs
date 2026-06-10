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
use std::rc::Rc;

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

/// The error stack-trace accumulator — the runtime's analogue of `iPtr`'s
/// `errorInfo`/`errorCode`/`errorLine`/`ERR_ALREADY_LOGGED` (PC-4). The trace is
/// built **incrementally as the error unwinds** (`TclLogCommandInfo` +
/// `MakeProcError`, `proc-call-and-stack-traces.md` §1.5), not at the throw, and
/// published to the `::errorInfo`/`::errorCode` globals when the error is caught
/// or reaches the outermost eval.
#[derive(Default)]
struct ExceptionState {
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
}

/// One formal parameter of a [`ProcDef`]: a name and an optional default value.
pub struct Param {
    pub name: Vec<u8>,
    pub default: Option<Vec<u8>>,
}

/// A compiled `proc` definition: its parameters, body script, and the namespace
/// it was defined in (which becomes the current namespace while it runs).
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

/// `Tcl_Interp`. Owns the frame stack, the command table, and the current
/// result object (a `+1` it holds; never null after `new`).
#[repr(C)]
pub struct Interp {
    pub(crate) frames: FrameStack,
    /// The command-table-as-core-service: the namespace tree + the one
    /// `resolve(currentNs, name)` resolver (T1.5).
    namespaces: Namespaces,
    /// The current namespace for command resolution (the eval context; a proc
    /// runs in its *defining* namespace — wired with procs). Global at top level.
    current_ns: NsId,
    /// Active proc-call nesting depth — C Tcl's `interp recursionlimit`. Bounds
    /// recursion so an infinite proc loop raises a catchable error instead of
    /// overflowing the (wasm) stack (the tracked PR #557 follow-up).
    recursion_depth: usize,
    /// The package database (`package provide`/`require`/`ifneeded`/`unknown`).
    pub(crate) packages: crate::cmd_package::PackageState,
    /// The `source` script stack (`info script` — the file being sourced).
    script_stack: Vec<Vec<u8>>,
    /// Open channels (`open`/`read`/`gets`/`puts`/`close`).
    pub(crate) channels: crate::cmd_chan::ChannelTable,
    /// Pending `return -code`/`-level` state (`TclUpdateReturnInfo`): the code to
    /// complete with once `-level` boundaries are unwound.
    return_code: Code,
    return_level: usize,
    /// Variable-trace registry (`trace add|remove|info variable`).
    pub(crate) traces: crate::cmd_trace::TraceTable,
    /// The error stack-trace accumulator (PC-4).
    exc: ExceptionState,
    /// Child interpreters (`interp create`), each a full `Interp`, keyed by name.
    /// The name is also a command in this interp (`Command::ChildInterp`).
    children: std::collections::BTreeMap<Vec<u8>, Box<Interp>>,
    /// Counter for auto-generated child names (`interp0`, `interp1`, …).
    interp_counter: usize,
    /// The source-location stack (`cmdFramePtr`; PC-5) — what `info frame` reads.
    cmd_frames: Vec<CmdFrame>,
    /// `eval_str` nesting depth. The outermost eval (depth returning to 0)
    /// publishes the accumulated error trace to the `::errorInfo`/`::errorCode`
    /// globals; nested evals (proc bodies, `[cmd]` subst, control bodies) just
    /// accumulate.
    eval_depth: usize,
    result: *mut TclObj,
}

/// The proc-call recursion bound (C Tcl's default `interp recursionlimit`).
const RECURSION_LIMIT: usize = 1000;

impl Interp {
    /// Create an interp: global frame, the built-in command set, an empty
    /// result.
    pub fn new() -> Box<Interp> {
        let result = obj::new_obj();
        // SAFETY: `result` is freshly created; the interp takes the owning ref.
        unsafe { obj::incr_ref_count(result) };
        let mut interp = Box::new(Interp {
            frames: FrameStack::new(),
            namespaces: Namespaces::new(),
            current_ns: GLOBAL,
            recursion_depth: 0,
            packages: crate::cmd_package::PackageState::with_core(),
            script_stack: Vec::new(),
            channels: crate::cmd_chan::ChannelTable::default(),
            return_code: Code::Ok,
            return_level: 1,
            traces: crate::cmd_trace::TraceTable::default(),
            exc: ExceptionState::default(),
            children: std::collections::BTreeMap::new(),
            interp_counter: 0,
            cmd_frames: Vec::new(),
            eval_depth: 0,
            result,
        });
        builtins::install(&mut interp);
        interp
    }

    // -- command registry -----------------------------------------------------

    /// Register a built-in command (a possibly-qualified `name`, creating
    /// intermediate namespaces; overwrites any existing command of `name`).
    pub fn register_builtin(&mut self, name: &[u8], f: BuiltinFn) {
        self.namespaces.register(name, Command::Builtin(f));
    }

    /// Command names in the current namespace, sorted (`info commands`).
    #[must_use]
    pub fn command_names(&self) -> Vec<&[u8]> {
        self.namespaces.command_names(self.current_ns)
    }

    /// `rename old new` (or `rename old ""` to delete), relative to the current
    /// namespace. Drives the one command table; see [`Namespaces::rename`].
    pub(crate) fn rename_command(&mut self, old: &[u8], new: &[u8]) -> RenameOutcome {
        self.namespaces.rename(self.current_ns, old, new)
    }

    /// Install an `interp alias` redirect named `name` → `target ?prefix...?`.
    pub(crate) fn install_alias(&mut self, name: &[u8], target: Vec<u8>, prefix: Vec<Vec<u8>>) {
        self.namespaces
            .register(name, Command::Alias { target, prefix });
    }

    /// The `(target, prefix)` of the alias bound to `name` (the query form), or
    /// `None` if `name` resolves to something that isn't an alias.
    pub(crate) fn alias_info(&self, name: &[u8]) -> Option<(Vec<u8>, Vec<Vec<u8>>)> {
        match self.namespaces.resolve(self.current_ns, name) {
            Some(Command::Alias { target, prefix }) => Some((target, prefix)),
            _ => None,
        }
    }

    /// Delete the command bound to `name` (the alias-clear form); returns whether
    /// it existed.
    pub(crate) fn delete_command(&mut self, name: &[u8]) -> bool {
        self.namespaces.delete(self.current_ns, name)
    }

    /// Register an ensemble command (`namespace ensemble create`); `name` is the
    /// ensemble command (possibly qualified — rooted at global like any builtin).
    pub(crate) fn create_ensemble(&mut self, name: &[u8], cfg: crate::ensemble::EnsembleConfig) {
        self.namespaces.register(name, Command::Ensemble(cfg));
    }

    /// Define a user proc (`proc name params body`). The proc's defining
    /// namespace (where its body runs, and where it is bound) is the namespace
    /// `name` lands in — **relative to the current namespace** (so `proc next`
    /// inside `namespace eval counter` binds `::counter::next`, not a global).
    pub(crate) fn define_proc(&mut self, name: &[u8], params: Vec<Param>, body: Vec<u8>) {
        let ns = self.namespaces.command_home_ns(self.current_ns, name);
        let tail = tcl_syntax::naming::qualifier_segments(name)
            .last()
            .copied()
            .unwrap_or(name)
            .to_vec();
        // The proc's FQN (`info frame` `proc` key): `<ns>::<tail>`, the global ns
        // contributing just the leading `::`.
        let qn = self.namespaces.qualified_name(ns);
        let mut fqn = qn.clone();
        if qn != b"::" {
            fqn.extend_from_slice(b"::");
        }
        fqn.extend_from_slice(&tail);
        // A proc defined while a file is being sourced records that file, so its
        // body frame reports `type source`. Its body lines are then file-absolute
        // (C's literal line table): the base is the file line of the `proc`
        // command minus one (the body opens on that line). Eval-defined procs
        // keep body-relative lines (base 0).
        let source = self.script_stack.last().map(|f| Rc::from(f.as_slice()));
        let body_line_base = if source.is_some() {
            self.current_cmd_line().saturating_sub(1)
        } else {
            0
        };
        let def = Rc::new(ProcDef {
            params,
            body,
            ns,
            fqn,
            source,
            body_line_base,
        });
        self.namespaces.bind(ns, &tail, Command::Proc(def));
    }

    /// The reported `line` of the command currently executing at the top of the
    /// `info frame` stack (for fixing a source-defined proc's body line base).
    fn current_cmd_line(&self) -> u32 {
        self.cmd_frames.last().map_or(1, |f| f.line)
    }

    /// Whether `name` resolves to an ensemble command (`namespace ensemble
    /// exists`).
    pub(crate) fn is_ensemble(&self, name: &[u8]) -> bool {
        matches!(
            self.namespaces.resolve(self.current_ns, name),
            Some(Command::Ensemble(_))
        )
    }

    /// Every alias command's name across the whole tree (`interp aliases`).
    pub(crate) fn alias_names(&self) -> Vec<Vec<u8>> {
        self.namespaces.alias_names()
    }

    /// The current namespace (the eval context) — for the `namespace` builtin.
    pub(crate) fn current_ns(&self) -> NsId {
        self.current_ns
    }

    /// The namespace tree (read) — for the `namespace` builtin's queries.
    pub(crate) fn namespaces(&self) -> &Namespaces {
        &self.namespaces
    }

    /// The namespace tree (mutable) — for the `namespace` builtin's mutations
    /// (`export`/`import`/`forget`/`path`).
    pub(crate) fn namespaces_mut(&mut self) -> &mut Namespaces {
        &mut self.namespaces
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
        self.return_level = level;
        self.return_code = code;
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
        self.return_level = self.return_level.saturating_sub(1);
        if self.return_level == 0 {
            let c = self.return_code;
            self.return_code = Code::Ok;
            c
        } else {
            Code::Return
        }
    }

    /// `source`: evaluate `script` as a sourced file named `name`, tracking it on
    /// the script stack (`info script`). A top-level `return` ends the file (the
    /// return boundary maps `return` → Ok); other codes propagate.
    pub fn eval_sourced(&mut self, script: &[u8], name: &[u8]) -> Code {
        self.script_stack.push(name.to_vec());
        // A `source`d file is its own `info frame` level: `type source` + the
        // file path, inheriting the enclosing proc/level. Its commands are
        // numbered by the file's own lines (base 0, the file *is* the script).
        let mut frame = self.inherited_cmd_frame();
        frame.kind = FrameKind::Source;
        frame.file = Some(Rc::from(name));
        frame.line_base = 0;
        let code = self.eval_framed(script, frame);
        self.script_stack.pop();
        self.settle_return(code)
    }

    /// `info script` — the file currently being sourced (empty at top level).
    pub(crate) fn current_script(&self) -> Vec<u8> {
        self.script_stack.last().cloned().unwrap_or_default()
    }

    /// `uplevel`: evaluate `script` in the variable scope **and** namespace of
    /// frame `target_level` (the Zig oracle's "restore caller ns + frame depth
    /// together" discovery), then restore. Transparent — the body's completion
    /// code (incl. `return`) propagates unchanged.
    pub(crate) fn eval_uplevel(&mut self, target_level: usize, script: &[u8]) -> Code {
        let prev_level = self.frames.set_active_level(target_level);
        let prev_ns = self.current_ns;
        self.current_ns = self.frames.frame_ns(target_level);
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
        self.frames.set_active_level(prev_level);
        self.current_ns = prev_ns;
        code
    }

    /// `namespace eval name body`: switch the current namespace to `name`
    /// (creating it, relative to the current ns unless `::`-anchored), evaluate
    /// `body` there, then restore. The current-ns switch is what makes commands
    /// defined in `body` land in the right table.
    pub(crate) fn ns_eval(&mut self, name: &[u8], body: &[u8]) -> Code {
        let target = self.namespaces.ensure_namespace(self.current_ns, name);
        let saved = self.current_ns;
        self.current_ns = target;
        // A namespace frame: a new scope whose unqualified vars resolve to the
        // namespace (so `set`/`variable`/`upvar 0` inside `namespace eval` —
        // including when nested in a proc — target the namespace, not the
        // enclosing proc's locals).
        self.frames.push_namespace(target);
        let code = self.eval_str(body);
        self.frames.pop();
        self.current_ns = saved;
        code
    }

    /// Whether the active variable frame is a proc call frame (vs. global /
    /// `namespace eval` scope).
    pub(crate) fn in_proc(&self) -> bool {
        self.frames.in_proc()
    }

    // -- variables (the var resolver; `crate::vars`) --------------------------
    //
    // Every variable op routes through the one classification + link walk
    // (frame-local vs namespace, qualified vs not), instead of the old flat
    // per-frame table. The `name` here is the array *base* (callers split
    // `a(k)` via `split_array_ref` first), so `::ns::base` qualifies correctly.

    /// `set name` — borrowed value (the table keeps its +1), or `None`.
    pub(crate) fn var_get(&self, name: &[u8]) -> Option<*mut TclObj> {
        crate::vars::get(&self.frames, &self.namespaces, self.current_ns, name)
    }

    /// `set name(key)` — borrowed.
    pub(crate) fn var_get_elem(&self, name: &[u8], key: &[u8]) -> Option<*mut TclObj> {
        crate::vars::get_elem(&self.frames, &self.namespaces, self.current_ns, name, key)
    }

    /// `set name value` — the cell takes a **+1** on `obj`.
    pub(crate) fn var_set(&mut self, name: &[u8], obj: *mut TclObj) -> Result<(), VarError> {
        let r = crate::vars::set(
            &mut self.frames,
            &mut self.namespaces,
            self.current_ns,
            name,
            obj,
        );
        if r.is_ok() && !self.traces.traces.is_empty() {
            let (base, elem) = crate::frame::split_array_ref(name);
            self.fire_var_trace(&base, elem.as_deref(), b"write");
        }
        r
    }

    /// `set name(key) value`.
    pub(crate) fn var_set_elem(
        &mut self,
        name: &[u8],
        key: &[u8],
        obj: *mut TclObj,
    ) -> Result<(), VarError> {
        let r = crate::vars::set_elem(
            &mut self.frames,
            &mut self.namespaces,
            self.current_ns,
            name,
            key,
            obj,
        );
        if r.is_ok() && !self.traces.traces.is_empty() {
            self.fire_var_trace(name, Some(key), b"write");
        }
        r
    }

    /// Fire a read trace for `name` before a read (the `&mut` chokepoints that
    /// resolve `$var` call this).
    pub(crate) fn fire_read_trace(&mut self, name: &[u8], key: Option<&[u8]>) {
        if self.traces.traces.is_empty() {
            return;
        }
        let (base, elem) = crate::frame::split_array_ref(name);
        let key = key.or(elem.as_deref());
        self.fire_var_trace(&base, key, b"read");
    }

    /// `unset name` — returns whether it existed.
    pub(crate) fn var_unset(&mut self, name: &[u8]) -> bool {
        let existed = crate::vars::unset(
            &mut self.frames,
            &mut self.namespaces,
            self.current_ns,
            name,
        );
        if existed && !self.traces.traces.is_empty() {
            let (base, elem) = crate::frame::split_array_ref(name);
            self.fire_var_trace(&base, elem.as_deref(), b"unset");
        }
        existed
    }

    /// `unset name(key)` — returns whether it existed.
    pub(crate) fn var_unset_elem(&mut self, name: &[u8], key: &[u8]) -> bool {
        let existed = crate::vars::unset_elem(
            &mut self.frames,
            &mut self.namespaces,
            self.current_ns,
            name,
            key,
        );
        if existed && !self.traces.traces.is_empty() {
            self.fire_var_trace(name, Some(key), b"unset");
        }
        existed
    }

    /// Invoke every variable trace matching `(base, elem, op)`, as
    /// `command base element op`. Re-entrant firing is suppressed (the `firing`
    /// guard) and callback errors are swallowed; the interp result is preserved
    /// across the callbacks (the triggering operation owns the result).
    fn fire_var_trace(&mut self, base: &[u8], elem: Option<&[u8]>, op: &[u8]) {
        if self.traces.firing > 0 {
            return;
        }
        let cmds: Vec<Vec<u8>> = self
            .traces
            .traces
            .iter()
            .filter(|t| crate::cmd_trace::matches(t, base, elem, op))
            .map(|t| t.command.clone())
            .collect();
        if cmds.is_empty() {
            return;
        }
        // Preserve the result object across the callbacks.
        let saved = self.result;
        unsafe { obj::incr_ref_count(saved) };

        self.traces.firing += 1;
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
            let _ = self.eval_str(&line);
        }
        self.traces.firing -= 1;

        // Restore the saved result (release the trace's, adopt our held +1).
        unsafe {
            obj::decr_ref_count(self.result);
            self.result = saved;
        }
    }

    /// Whether `name` resolves to an array variable (`set a` array-vs-scalar
    /// diagnostic, `array exists`).
    pub(crate) fn var_is_array(&self, name: &[u8]) -> bool {
        crate::vars::is_array(&self.frames, &self.namespaces, self.current_ns, name)
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
            self.namespaces.var_home(context_ns, name)
        } else {
            Some((context_ns, name.to_vec()))
        }
    }

    /// The current call-frame level (`upvar` relative-level arithmetic).
    pub(crate) fn current_level(&self) -> usize {
        self.frames.current_level()
    }

    /// `variable tail` / `global tail` — link `tail` in the current frame to
    /// `target_ns::tail` (a no-op when the current context already is that var).
    pub(crate) fn make_variable(&mut self, target_ns: NsId, tail: &[u8]) {
        crate::vars::make_variable(
            &mut self.frames,
            &mut self.namespaces,
            self.current_ns,
            target_ns,
            tail,
        );
    }

    /// Resolve (creating if needed) a namespace by name, relative to the current
    /// namespace — for `apply`'s optional namespace term.
    pub(crate) fn ensure_namespace(&mut self, name: &[u8]) -> NsId {
        self.namespaces.ensure_namespace(self.current_ns, name)
    }

    // -- introspection (`info` / `array`) -------------------------------------

    /// `info exists name` — whether a scalar/array/element variable is set
    /// (splitting `arr(key)`, the Zig discovery).
    pub(crate) fn var_exists(&self, name: &[u8]) -> bool {
        let (base, elem) = crate::frame::split_array_ref(name);
        match elem {
            Some(k) => {
                crate::vars::exists_elem(&self.frames, &self.namespaces, self.current_ns, &base, &k)
            }
            None => crate::vars::exists(&self.frames, &self.namespaces, self.current_ns, &base),
        }
    }

    /// The element names of array `name` (`array names`/`get`), or `None`.
    pub(crate) fn array_names(&self, name: &[u8]) -> Option<Vec<Vec<u8>>> {
        crate::vars::array_names(&self.frames, &self.namespaces, self.current_ns, name)
    }

    /// `info level` — the current frame level (proc nesting depth).
    pub(crate) fn level(&self) -> usize {
        self.frames.current_level()
    }

    /// The invoking command words at call `level` (`info level N`), or `None`
    /// when the level has none.
    pub(crate) fn level_words(&self, level: usize) -> Option<Vec<Vec<u8>>> {
        self.frames
            .words_at(level)
            .filter(|w| !w.is_empty())
            .map(<[Vec<u8>]>::to_vec)
    }

    /// Variable names visible in the current scope (`info vars`): the active
    /// frame's locals in a proc, else the current namespace's variables.
    pub(crate) fn visible_var_names(&self) -> Vec<Vec<u8>> {
        if self.frames.in_proc() {
            self.frames.local_names()
        } else {
            self.namespaces.var_names(self.current_ns)
        }
    }

    /// `info locals` — the active frame's local variable names.
    pub(crate) fn local_var_names(&self) -> Vec<Vec<u8>> {
        self.frames.local_names()
    }

    /// `info globals` — the global namespace's variable names.
    pub(crate) fn global_var_names(&self) -> Vec<Vec<u8>> {
        self.namespaces.var_names(GLOBAL)
    }

    /// Command names visible from the current namespace (`info commands`):
    /// current ns ∪ global, sorted/deduped.
    pub(crate) fn visible_command_names(&self) -> Vec<Vec<u8>> {
        self.visible(self.namespaces.command_names(self.current_ns), |ns| {
            ns.command_names(GLOBAL)
        })
    }

    /// Proc names visible from the current namespace (`info procs`).
    pub(crate) fn visible_proc_names(&self) -> Vec<Vec<u8>> {
        let mut v = self.namespaces.proc_names(self.current_ns);
        if self.current_ns != GLOBAL {
            v.extend(self.namespaces.proc_names(GLOBAL));
        }
        v.sort();
        v.dedup();
        v
    }

    fn visible(
        &self,
        local: Vec<&[u8]>,
        global: impl Fn(&Namespaces) -> Vec<&[u8]>,
    ) -> Vec<Vec<u8>> {
        let mut v: Vec<Vec<u8>> = local.iter().map(|s| s.to_vec()).collect();
        if self.current_ns != GLOBAL {
            v.extend(global(&self.namespaces).iter().map(|s| s.to_vec()));
        }
        v.sort();
        v.dedup();
        v
    }

    /// The proc definition bound to `name` (for `info body`/`args`/`default`).
    pub(crate) fn proc_def(&self, name: &[u8]) -> Option<Rc<ProcDef>> {
        match self.namespaces.resolve(self.current_ns, name) {
            Some(Command::Proc(def)) => Some(def),
            _ => None,
        }
    }

    /// `upvar` — link `local` in the current frame to the resolved `target`.
    pub(crate) fn make_upvar(&mut self, target: Link, local: &[u8]) {
        crate::vars::make_upvar(
            &mut self.frames,
            &mut self.namespaces,
            self.current_ns,
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
        let old = self.result;
        // SAFETY: `obj` live (caller); `old` is the interp's owned result.
        unsafe {
            obj::incr_ref_count(obj);
            self.result = obj;
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
        self.result
    }

    /// The current result's string bytes (copied).
    pub fn result_bytes(&self) -> Vec<u8> {
        obj_bytes(self.result)
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
    fn error(&mut self, msg: &[u8]) -> Code {
        self.set_result_bytes(msg);
        let code: &[u8] = if msg.starts_with(b"wrong # args:") {
            b"TCL WRONGARGS"
        } else {
            b"NONE"
        };
        self.exc = ExceptionState {
            info: None,
            code: code.to_vec(),
            line: 1,
            already_logged: false,
        };
        Code::Error
    }

    /// Pre-seed the error trace for `error msg info ?code?` / `throw`: the result
    /// is `msg`, the trace starts at `info` (so the throwing command is **not**
    /// re-logged — `ERR_ALREADY_LOGGED`), and `-errorcode` is `code`. Returns
    /// [`Code::Error`].
    pub(crate) fn raise_with_info(&mut self, msg: &[u8], info: &[u8], code: &[u8]) -> Code {
        self.set_result_bytes(msg);
        self.exc = ExceptionState {
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
        self.exc = ExceptionState {
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
        if self.exc.already_logged {
            return;
        }
        // errorLine = 1 + count('\n' in src[0..commandStart]) — C's exact loop.
        self.exc.line = line_of(src, cmd.start);
        let started = self.exc.info.is_some();
        if !started {
            // First frame: errorInfo is seeded from the error message (the result).
            let msg = self.result_bytes();
            self.exc.info = Some(msg);
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
        let buf = self.exc.info.as_mut().expect("seeded above");
        buf.extend_from_slice(b"\n    ");
        buf.extend_from_slice(verb);
        buf.extend_from_slice(b"\n\"");
        buf.extend_from_slice(slice);
        if overflow {
            buf.extend_from_slice(b"...");
        }
        buf.push(b'"');
        self.exc.already_logged = true;
    }

    /// Append the `(procedure "NAME" line N)` / `(lambda term "..." line N)`
    /// frame when a proc/lambda body unwinds with an error (`MakeProcError` /
    /// `MakeLambdaError`, `tclProc.c`), then clear `already_logged` so the
    /// proc-call command itself is logged by its enclosing eval. The line is the
    /// body-relative `error_line` the innermost body command recorded.
    fn make_proc_error(&mut self, frame: ProcFrame) {
        // `(procedure "NAME" line N)` / `(lambda term "NAME" line N)` — the name
        // quoted, truncated to 60 bytes (`...` on overflow).
        let (kind, name): (&[u8], &[u8]) = match frame {
            ProcFrame::Proc(n) => (b"procedure", n),
            ProcFrame::Lambda(n) => (b"lambda term", n),
        };
        let overflow = name.len() > 60;
        let trunc = if overflow { &name[..60] } else { name };
        let mut inner = Vec::with_capacity(kind.len() + trunc.len() + 8);
        inner.extend_from_slice(kind);
        inner.extend_from_slice(b" \"");
        inner.extend_from_slice(trunc);
        if overflow {
            inner.extend_from_slice(b"...");
        }
        inner.push(b'"');
        self.append_frame_line(&inner);
        self.exc.already_logged = false;
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
        self.exc.already_logged = false;
    }

    /// Shared tail of the `(... line N)` frames: append `"\n    (<inner> line
    /// <N>)"` to `errorInfo` (seeding it from the result message if no frame has
    /// been logged yet), where `inner` is the caller-built body — e.g.
    /// `procedure "p"`, `lambda term "..."`, or `"eval" body`.
    fn append_frame_line(&mut self, inner: &[u8]) {
        let line = self.exc.line;
        if self.exc.info.is_none() {
            let msg = self.result_bytes();
            self.exc.info = Some(msg);
        }
        let buf = self.exc.info.as_mut().expect("seeded above");
        buf.extend_from_slice(b"\n    (");
        buf.extend_from_slice(inner);
        buf.extend_from_slice(b" line ");
        buf.extend_from_slice(line.to_string().as_bytes());
        buf.push(b')');
    }

    /// The current accumulated `errorInfo` (for `catch`'s `-errorinfo`): the
    /// trace if any frame was logged, else the bare error message.
    pub(crate) fn error_info(&self) -> Vec<u8> {
        self.exc.info.clone().unwrap_or_else(|| self.result_bytes())
    }

    /// `info frame` (no arg): the depth of the source-location stack.
    pub(crate) fn cmd_frame_depth(&self) -> usize {
        self.cmd_frames.len()
    }

    /// The `info frame N` description for stack position `n` (C's level
    /// arithmetic: `n > 0` is absolute, 1-based from the root; `n <= 0` is
    /// relative to the current top, `0` = current). Returns the dict's
    /// (key, value) pairs in C's key order (`type line [file] cmd [proc]
    /// level`), or `None` if out of range.
    pub(crate) fn cmd_frame_info(&self, n: i64) -> Option<Vec<(Vec<u8>, Vec<u8>)>> {
        let depth = self.cmd_frames.len() as i64;
        let pos = if n > 0 { n } else { depth + n };
        if pos < 1 || pos > depth {
            return None;
        }
        let f = &self.cmd_frames[(pos - 1) as usize];
        let mut pairs = vec![
            (b"type".to_vec(), f.kind.as_bytes().to_vec()),
            (b"line".to_vec(), f.line.to_string().into_bytes()),
        ];
        if let Some(file) = &f.file {
            pairs.push((b"file".to_vec(), file.to_vec()));
        }
        pairs.push((b"cmd".to_vec(), f.cmd.clone()));
        if let Some(p) = &f.proc {
            pairs.push((b"proc".to_vec(), p.clone()));
        }
        // `level` is the distance from the current call level (omitted for a
        // redirected `uplevel` scope, matching C's reachability check).
        if !f.omit_level {
            let level = self.frames.current_level().saturating_sub(f.level);
            pairs.push((b"level".to_vec(), level.to_string().into_bytes()));
        }
        Some(pairs)
    }

    /// The current `errorCode` (for `catch`'s `-errorcode`): the stamped value,
    /// or `NONE`.
    pub(crate) fn error_code(&self) -> Vec<u8> {
        if self.exc.code.is_empty() {
            b"NONE".to_vec()
        } else {
            self.exc.code.clone()
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
        self.exc = ExceptionState::default();
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
        let owned = self.cmd_frames.is_empty().then(CmdFrame::root);
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
        self.eval_depth += 1;
        let owns_frame = owned.is_some();
        if let Some(f) = owned {
            self.cmd_frames.push(f);
        }
        let mut last = Code::Ok;
        let commands = parse::parse_script(src);
        for cmd in &commands {
            last = self.eval_command(src, cmd, owns_frame);
            if last != Code::Ok {
                break; // error/return/break/continue propagate up
            }
        }
        if owns_frame {
            self.cmd_frames.pop();
        }
        self.eval_depth -= 1;
        // The outermost eval publishes the accumulated trace to the globals so
        // an uncaught error leaves `::errorInfo`/`::errorCode` set, exactly as a
        // `catch` would (`catch` publishes earlier, at depth > 0).
        if self.eval_depth == 0 && last == Code::Error {
            self.publish_error();
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
        let top = self.cmd_frames.last();
        CmdFrame {
            kind: top.map_or(FrameKind::Eval, |f| f.kind),
            file: top.and_then(|f| f.file.clone()),
            proc: top.and_then(|f| f.proc.clone()),
            level: top.map_or(0, |f| f.level),
            omit_level: false,
            line_base: self.current_cmd_line().saturating_sub(1),
            cmd: Vec::new(),
            line: 1,
        }
    }

    /// Evaluate an `eval`/`uplevel` body as its own `info frame` level (it
    /// inherits the enclosing proc/level). The errorInfo `("eval" body line N)`
    /// frame is appended separately by the caller.
    pub(crate) fn eval_body(&mut self, script: &[u8]) -> Code {
        let frame = self.inherited_cmd_frame();
        self.eval_framed(script, frame)
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
        if let Some(top) = self.cmd_frames.last_mut() {
            if owns_frame {
                // `line_base` shifts a source-defined proc's body lines to be
                // file-absolute; it is 0 elsewhere (body-relative).
                top.line = top.line_base + line_of(src, cmd.start);
            }
            top.cmd = src[cmd.start..cmd.end].to_vec();
        }
        let code = self.eval_words(&cmd.words);
        if code == Code::Error {
            self.log_command_info(src, cmd);
        }
        code
    }

    /// Substitute each word of a command (with `{*}` expansion), then dispatch.
    fn eval_words(&mut self, words: &[parse::Word]) -> Code {
        let mut argv: Vec<*mut TclObj> = Vec::new();
        for w in words {
            let obj = match self.substitute_word(&w.body) {
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
                match parse::split_list(&bytes) {
                    Ok(elems) => {
                        for e in elems {
                            let eo = new_string(&e);
                            unsafe { obj::incr_ref_count(eo) };
                            argv.push(eo);
                        }
                    }
                    Err(e) => {
                        release_all(&argv);
                        return self.error(e.message());
                    }
                }
            } else {
                argv.push(obj); // already owned (+1)
            }
        }

        if argv.is_empty() {
            return Code::Ok;
        }

        let code = self.dispatch(&argv);
        // Safe to release argv now: a command that made an argv element its
        // result did so via set_obj_result, which holds an independent +1.
        release_all(&argv);
        code
    }

    /// Look up `argv[0]` and invoke it; on a miss, fall to the `unknown` handler
    /// (auto-load / `package` / friendly errors — the pure-Tcl `unknown` proc),
    /// matching C's `TclEvalObjvInternal`.
    fn dispatch(&mut self, argv: &[*mut TclObj]) -> Code {
        let name = obj_bytes(argv[0]);
        if let Some(cmd) = self.namespaces.resolve(self.current_ns, &name) {
            return self.invoke(cmd, argv);
        }
        // Command miss: dispatch through `unknown <name> <args…>` if it exists
        // (and we're not already resolving `unknown` itself).
        if name != b"unknown" {
            if let Some(unk) = self.namespaces.resolve(GLOBAL, b"unknown") {
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
            Command::Imported { source } => match self.namespaces.resolve(GLOBAL, &source) {
                // Transparent redirect: forward argv unchanged to the source.
                Some(cmd) => self.invoke(cmd, argv),
                None => self.invalid_command(&source),
            },
            Command::Ensemble(cfg) => self.dispatch_ensemble(&cfg, argv),
            Command::Proc(def) => self.call_proc(&def, argv),
            Command::ChildInterp(name) => self.dispatch_child(&name, argv),
        }
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
                self.set_result_bytes(b"0");
                Code::Ok
            }
            b"delete" => {
                self.delete_child(name);
                self.set_result_bytes(b"");
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
            let n = format!("interp{}", self.interp_counter);
            self.interp_counter += 1;
            n.into_bytes()
        });
        let mut child = Interp::new();
        // A (non-safe) child gets the predefined globals (`tcl_platform`, …) like
        // a real interpreter. The full `init.tcl` (package/auto-load) is deferred.
        child.set_startup_globals();
        self.children.insert(name.clone(), child);
        self.namespaces
            .register(&name, Command::ChildInterp(name.clone()));
        name
    }

    /// Whether a child interpreter `name` exists.
    pub(crate) fn child_exists(&self, name: &[u8]) -> bool {
        self.children.contains_key(name)
    }

    /// The names of this interp's direct child interpreters (sorted).
    pub(crate) fn child_names(&self) -> Vec<Vec<u8>> {
        self.children.keys().cloned().collect()
    }

    /// Delete a child interpreter (and its command). Returns whether it existed.
    pub(crate) fn delete_child(&mut self, name: &[u8]) -> bool {
        let existed = self.children.remove(name).is_some();
        if existed {
            self.namespaces.delete(GLOBAL, name);
        }
        existed
    }

    /// Evaluate `script` in child interpreter `name`, copying its result/code
    /// back. `None`-returning if the child doesn't exist (caller raises).
    pub(crate) fn eval_in_child(&mut self, name: &[u8], script: &[u8]) -> Code {
        let Some(child) = self.children.get_mut(name) else {
            let mut m = b"could not find interpreter \"".to_vec();
            m.extend_from_slice(name);
            m.push(b'"');
            return self.error(&m);
        };
        let code = child.eval_str(script);
        let result = child.result_bytes();
        self.set_result_bytes(&result);
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
        // Arity (defaults assumed trailing — the common shape): supplied must
        // cover the no-default params, and not exceed the positionals unless an
        // `args` catch-all soaks up the rest.
        let supplied = call_args.len();
        let required = positional.iter().filter(|p| p.default.is_none()).count();
        if supplied < required || (!has_args && supplied > positional.len()) {
            return self.error(&proc_usage(usage_called, params));
        }
        // Recursion bound (catchable, not a stack overflow).
        if self.recursion_depth >= RECURSION_LIMIT {
            return self.error(b"too many nested evaluations (infinite loop?)");
        }
        self.recursion_depth += 1;

        self.frames.push(ns);
        // Record the invocation words for `info level N`: the invoked name plus
        // the supplied arguments.
        let mut words = Vec::with_capacity(call_args.len() + 1);
        words.push(usage_called.to_vec());
        words.extend(call_args.iter().map(|&a| obj_bytes(a)));
        self.frames.set_words(words);
        let saved_ns = self.current_ns;
        self.current_ns = ns;

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
                self.frames.pop();
                self.current_ns = saved_ns;
                self.recursion_depth -= 1;
                return self.error(&proc_usage(usage_called, params));
            };
            if stored.is_err() {
                self.frames.pop();
                self.current_ns = saved_ns;
                self.recursion_depth -= 1;
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
        let proc_frame = CmdFrame {
            kind: if meta.source.is_some() {
                FrameKind::Source
            } else {
                FrameKind::Proc
            },
            file: meta.source,
            proc: meta.fqn.map(<[u8]>::to_vec),
            level: self.frames.current_level(),
            omit_level: false,
            line_base: meta.body_line_base,
            cmd: Vec::new(),
            line: 1,
        };
        let code = self.eval_framed(body, proc_frame);
        self.frames.pop();
        self.current_ns = saved_ns;
        self.recursion_depth -= 1;
        // Apply the return boundary (`return`/`return -code -level`), then a
        // bare `break`/`continue` that escaped the body (no enclosing loop) is an
        // error (C Tcl: `invoked "break" outside of a loop`).
        let settled = match self.settle_return(code) {
            Code::Break => self.error(b"invoked \"break\" outside of a loop"),
            Code::Continue => self.error(b"invoked \"continue\" outside of a loop"),
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
        if argv.len() < 2 {
            let mut m = b"wrong # args: should be \"".to_vec();
            m.extend_from_slice(&obj_bytes(argv[0]));
            m.extend_from_slice(b" subcommand ?arg ...?\"");
            return self.error(&m);
        }
        // The valid subcommand set (sorted): explicit `-subcommands`, else the
        // `-map` keys, else the namespace's exported commands.
        let mut subs: Vec<Vec<u8>> = match (&cfg.subcommands, &cfg.map) {
            (Some(list), _) => list.clone(),
            (None, Some(map)) => map.iter().map(|(k, _)| k.clone()).collect(),
            (None, None) => self.namespaces.exported_commands(cfg.ns),
        };
        subs.sort();
        subs.dedup();

        let sub = obj_bytes(argv[1]);
        let Some(idx) = crate::ensemble::resolve_subcommand(&subs, &sub, cfg.prefixes) else {
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
        };
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
                let mut t = self.namespaces.qualified_name(cfg.ns);
                if cfg.ns != GLOBAL {
                    t.extend_from_slice(b"::");
                }
                t.extend_from_slice(resolved);
                vec![t]
            });

        // Build [target…, argv[2..]…], each owned (+1), and re-dispatch.
        let mut new_argv: Vec<*mut TclObj> = Vec::with_capacity(prefix.len() + argv.len() - 2);
        for w in &prefix {
            let o = new_string(w);
            // SAFETY: fresh obj; take the owning +1 the new argv holds.
            unsafe { obj::incr_ref_count(o) };
            new_argv.push(o);
        }
        for &a in &argv[2..] {
            // SAFETY: live arg; take an owning +1.
            unsafe { obj::incr_ref_count(a) };
            new_argv.push(a);
        }
        let code = self.dispatch(&new_argv);
        release_all(&new_argv);
        code
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
        let Some(cmd) = self.namespaces.resolve(GLOBAL, &full) else {
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
        let Some(target_cmd) = self.namespaces.resolve(GLOBAL, target) else {
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
    fn invalid_command(&mut self, name: &[u8]) -> Code {
        let mut msg = b"invalid command name \"".to_vec();
        msg.extend_from_slice(name);
        msg.push(b'"');
        self.error(&msg)
    }

    /// Substitute one word's body into an **owned** (`+1`) object.
    /// A `Variable` reference to an unset variable, or a `[cmd]` that errors,
    /// returns `Err(code)` with the interp result already set.
    fn substitute_word(&mut self, body: &WordBody) -> Result<*mut TclObj, Code> {
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
                            self.fire_read_trace(v.name, index.as_deref());
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
                            let code = self.eval_str(script);
                            if code != Code::Ok {
                                return Err(code);
                            }
                            let r = self.result;
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
                            self.fire_read_trace(v.name, index.as_deref());
                            match self.read_var(v.name, index.as_deref()) {
                                Some(bytes) => buf.extend_from_slice(&bytes),
                                None => return Err(self.no_such_variable(v.name, index.as_deref())),
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
        let body = crate::subst::scan(src, flags);
        match &body {
            WordBody::Literal(b) => Ok(b.to_vec()),
            WordBody::Parts(parts) => self.resolve_subst_parts(parts),
        }
    }

    /// Resolve substitution `parts` to bytes, propagating any non-OK code (the
    /// `subst`-command path; cf. `substitute_word`, which builds an object).
    fn resolve_subst_parts(&mut self, parts: &[WordPart]) -> Result<Vec<u8>, Code> {
        let mut out = Vec::new();
        for part in parts {
            match part {
                WordPart::Text(b) => out.extend_from_slice(b),
                WordPart::Variable(v) => {
                    let index = match &v.index {
                        Some(p) => Some(self.subst_index(p)?),
                        None => None,
                    };
                    self.fire_read_trace(v.name, index.as_deref());
                    match self.read_var(v.name, index.as_deref()) {
                        Some(bytes) => out.extend_from_slice(&bytes),
                        None => return Err(self.no_such_variable(v.name, index.as_deref())),
                    }
                }
                WordPart::Command(script) => {
                    let code = self.eval_str(script);
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
        crate::vars::resolve_var_bytes(&self.frames, &self.namespaces, self.current_ns, name, index)
    }

    fn no_such_variable(&mut self, name: &[u8], index: Option<&[u8]>) -> Code {
        let mut msg = b"can't read \"".to_vec();
        msg.extend_from_slice(name);
        if let Some(i) = index {
            msg.push(b'(');
            msg.extend_from_slice(i);
            msg.push(b')');
        }
        msg.extend_from_slice(b"\": no such variable");
        self.error(&msg)
    }

    /// Set an error result and return [`Code::Error`] — for builtins.
    pub(crate) fn set_error(&mut self, msg: &[u8]) -> Code {
        self.error(msg)
    }
}

/// The `wrong # args: should be "name p1 ?p2? ?arg ...?"` message for a proc
/// call — required params bare, defaulted params `?p?`, the `args` catch-all
/// `?arg ...?` (mirrors C's `Tcl_WrongNumArgs` for procs).
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

impl Drop for Interp {
    fn drop(&mut self) {
        // Release the result; the FrameStack field drops afterwards, releasing
        // all variable refs. The command table holds no object refs.
        // SAFETY: `result` is the interp's owned reference, dropped once.
        unsafe { obj::decr_ref_count(self.result) };
        self.result = core::ptr::null_mut();
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
