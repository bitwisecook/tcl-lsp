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
//! model does (issue #946).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::{self, Write};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU64, Ordering};

use tcl_bytecode::{FunctionAsm, ModuleAsm};
use tcl_core_types::RecursionLimit;
use tcl_platform::Host;
use tcl_runtime_api::guard::{
    GuardDomain, GuardDomains, GuardError, GuardIdentity, GuardManager, GuardToken,
};
use tcl_runtime_api::{
    Code, CommandId, Commands, CompileService, Completion, FrameId, Frames, Introspect, Namespaces,
    NsId, ProcInfo, ProcParam, Procs, ROOT_NS, Traces, VarStore,
};
use tcl_syntax::expr::{eval, parse_expr};

use crate::command::{BuiltinFn, Command, EnsembleDef, ProcDef, register_builtins};
use crate::error::TclError;
use crate::expr::ExprEval;
use crate::frame::{CallFrame, Local};
use crate::host_native::NativeHost;
use crate::value::Value;

/// The proc-call recursion bound (C Tcl's default `interp recursionlimit`).
/// A deeper nesting is a catchable error, not a native stack overflow.
pub(crate) const RECURSION_LIMIT: usize = 1000;

/// Native-stack safety net for `cmd_control.rs`'s runtime-command fallback
/// (`cmd_if`/`cmd_while`/`cmd_for`/`each_loop`, reached via a computed
/// command name or dynamic body — see that module's own doc comment) —
/// issue #996.
///
/// Deliberately **not** a cap on [`Vm::eval_source`] itself: `eval_source`
/// is also the mechanism behind ordinary `[…]` command substitution
/// (`subst.rs`), `switch`/`try`/`dict with`/OO-method/namespace-eval
/// bodies, event dispatch, `source`, and more — all of it routine, and
/// empirically safe to at least 1000 levels of pure nested substitution on
/// a 2 MiB thread (measured via `probe_cmdsubst_*` during investigation).
/// An early version of this fix capped `eval_source` itself at a low,
/// uniform threshold and broke ordinary iRule execution (nested command
/// substitution plus a few layers of event-dispatch/orchestration
/// scaffolding routinely needs more than a very conservative cap allows,
/// well short of any real danger). The actual danger is narrower:
/// `cmd_control.rs`'s fallback specifically — invoking a *registered
/// command* (full argument-processing machinery) on every recursive
/// level — has a much heavier per-frame native-stack cost than plain
/// substitution recursion. Measured directly: driven through a computed
/// command name (`set c if; $c {1} { … }`, defeating the compiled fast
/// path), it overflowed the stack (SIGABRT) between depth 50 and 60 on a
/// 2 MiB thread, while pure `[subst {…}]` nesting on the same thread
/// survived to at least depth 1000. `tcl-vm` is also consumed from a WASM
/// host with no stack-size guarantee (`tcl-vm-wasm`), so this must hold on
/// a small ambient stack, not just a generously-sized one: 24 leaves
/// better than 2x margin under the measured crash floor while still
/// comfortably covering this fallback path's real (rare, edge-case —
/// ordinary Tcl essentially never nests a *computed-command-name* `if`
/// this deep) usage.
const CONTROL_FALLBACK_DEPTH_LIMIT: RecursionLimit = RecursionLimit(24);

/// Native-stack safety net for `TclOO` method dispatch (`cmd_oo.rs::run_step`)
/// — issue #996.
///
/// Unlike ordinary proc-to-proc calls, which the trampoline
/// ([`Tick::Call`](crate::exec)) runs without growing the *native* Rust
/// stack (each nested call is a new activation pushed onto an explicit
/// `Vec`, not a recursive Rust call), a `$obj method`/`my method`/`next`
/// dispatch is a genuine, undelegated native call chain: `run_step` (reached
/// via `oo_dispatch` → `oo_invoke` for `$obj method`, `cmd_my` →
/// `oo_invoke` for `my method`, or directly for `next`/`nextto`) →
/// [`Vm::oo_run_method`] → [`Vm::run_activation`] (a fresh native call, not
/// a trampoline push) → … back to `run_step` for a nested method call. So a
/// recursive method (or a long `next` chain) consumes real native stack
/// per level, unlike a recursive `proc`. `run_step` is guarded rather than
/// `oo_dispatch` because `my method` and `next`/`nextto` — the two most
/// common ways a method calls another method recursively — reach it
/// directly, bypassing `oo_dispatch` entirely.
/// Measured directly (a `method go {n} { … return [my go [expr {$n-1}]]
/// }`-style self-recursive method): SIGABRT between depth 45 and 48 on a
/// 2 MiB thread (`cargo test`'s per-test default, and Tokio's worker-thread
/// default — the same floor `CONTROL_FALLBACK_DEPTH_LIMIT` is calibrated
/// against). `tcl-vm` is also consumed from a WASM host with no
/// stack-size guarantee (`tcl-vm-wasm`), so this must hold on a small
/// ambient stack, not just a generously-sized one.
///
/// This is a real, deliberate compatibility gap versus C Tcl: an ordinary
/// recursive `proc` is bounded by [`RECURSION_LIMIT`] (1000, matching
/// `interp recursionlimit`'s default) with no native-stack cost at all,
/// while a recursive `TclOO` method hits this much lower bound instead. The
/// architecturally correct fix is routing method dispatch through the same
/// trampoline ordinary calls use (so it becomes as cheap as a proc call);
/// that is a substantially larger change to the dispatch/activation engine
/// than this counter, which is a mitigation, not that fix — an uncatchable
/// process abort is strictly worse than a catchable error arriving earlier
/// than tclsh's own limit would. 20 leaves better than 2x margin under the
/// measured crash floor.
const OO_DISPATCH_DEPTH_LIMIT: RecursionLimit = RecursionLimit(20);

/// The display form of a canonical (unrooted) namespace name: `""` → `::`,
/// `foo` → `::foo`. C's `Namespace.fullName`.
pub(crate) fn display_namespace(canonical: &str) -> String {
    if canonical.is_empty() {
        "::".to_string()
    } else {
        format!("::{canonical}")
    }
}

/// Parse an `interp recursionlimit` integer the way C's `Tcl_GetIntFromObj`
/// reports: a decimal that overflows `i64` is "too large to represent", a
/// non-numeric value is "expected integer but got …".
fn parse_recursion_limit(s: &str) -> Result<i64, String> {
    let t = s.trim();
    if let Ok(n) = t.parse::<i64>() {
        return Ok(n);
    }
    let body = t.strip_prefix(['+', '-']).unwrap_or(t);
    if !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit()) {
        return Err("integer value too large to represent".to_string());
    }
    Err(format!("expected integer but got \"{s}\""))
}

/// Resolve an `interp limit` option by unambiguous prefix against `opts`,
/// matching C's `Tcl_GetIndexFromObj` — through the one shared owner.
///
/// The hand-rolled `starts_with` filter this replaced had #1443's bug verbatim:
/// it could only ever say `bad option`, so the empty word — which is a prefix of
/// *every* option — reported `bad option ""` where C reports
/// `ambiguous option ""`. `OptionTable::abbreviating` owns both verdicts and the
/// `", or"` enumeration.
fn resolve_limit_opt<'a>(arg: &str, opts: &'a [&'a str]) -> Result<&'a str, String> {
    let table = tcl_cmd_core::prefix::OptionTable::abbreviating("option", opts);
    match table.index_of(arg.as_bytes()) {
        Ok(i) => Ok(opts[i]),
        Err(m) => Err(String::from_utf8_lossy(&m).into_owned()),
    }
}

/// Parse an `interp limit` integer option value, mirroring `parse_recursion_limit`'s
/// `expected integer but got "X"` wording.
fn parse_limit_int(s: &str) -> Result<i64, String> {
    s.trim()
        .parse::<i64>()
        .map_err(|_| format!("expected integer but got \"{s}\""))
}

/// The on-demand autoloader bootstrap (see [`Vm::init_auto_load`]). A focused
/// subset of C's `init.tcl`: `auto_load_index` reads each `tclIndex` on
/// `auto_path` (which sets `auto_index(cmd)` to a `::tcl::Pkg::source <file>`
/// loader), `auto_load` evaluates that loader on first reference, and `unknown`
/// drives it — erroring `invalid command name` when no loader applies. No
/// auto-exec of external programs (the VM is not a shell).
const AUTO_LOAD_BOOTSTRAP: &str = r#"
namespace eval ::tcl::Pkg {}
proc ::tcl::Pkg::source {file} { uplevel #0 [list source $file] }
proc auto_load_index {} {
    global auto_index dir
    foreach dir $::auto_path {
        set f [file join $dir tclIndex]
        if {[file exists $f]} { source $f }
    }
}
proc auto_load {cmd args} {
    global auto_index
    if {![info exists auto_index]} { auto_load_index }
    set bare [string trimleft $cmd :]
    foreach name [list $cmd ::$bare $bare] {
        if {[info exists auto_index($name)]} {
            uplevel #0 $auto_index($name)
            if {[llength [info commands $cmd]]} { return 1 }
            if {[llength [info commands ::$bare]]} { return 1 }
        }
    }
    return 0
}
proc unknown {cmd args} {
    if {[auto_load $cmd]} {
        return [uplevel 1 [linsert $args 0 $cmd]]
    }
    return -code error -errorcode [list TCL LOOKUP COMMAND $cmd] \
        "invalid command name \"$cmd\""
}
if {![info exists ::auto_path]} { set ::auto_path {}; catch {lappend ::auto_path [info library]} }
"#;

/// Build an `OK` completion (empty options dict).
pub(crate) fn ok(result: Value) -> Completion<Value> {
    Completion::new(Code::Ok, result, Value::empty())
}

/// Build an `ERROR` completion from a message (empty options dict).
pub(crate) fn err(message: impl Into<String>) -> Completion<Value> {
    let m: String = message.into();
    Completion::new(Code::Error, Value::string(m), Value::empty())
}

/// Build an `ERROR` completion carrying the canonical
/// `wrong # args: should be "usage"` arity message (the
/// [`tcl_cmd_core::CmdError::wrong_args`] catalogue text).
pub(crate) fn err_wrong_args(usage: &str) -> Completion<Value> {
    err(tcl_cmd_core::CmdError::wrong_args(usage).into_message())
}

/// `&str` view of a [`tcl_cmd_core::namespace`] byte-op result (the ops slice
/// at ASCII `:` boundaries, so a `&str` input yields valid UTF-8).
fn str_slice(b: &[u8]) -> &str {
    core::str::from_utf8(b).expect("subslice of valid UTF-8")
}

/// Join `name`'s `::`-separated segments with single separators — colon runs
/// collapse via the canonical [`tcl_syntax::naming::qualifier_segments`] split.
fn join_segments(name: &str) -> String {
    let segs = tcl_syntax::naming::qualifier_segments(name.as_bytes());
    let mut out = String::with_capacity(name.len());
    for (i, s) in segs.iter().enumerate() {
        if i > 0 {
            out.push_str("::");
        }
        out.push_str(core::str::from_utf8(s).expect("subslice of valid UTF-8"));
    }
    out
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
    /// The interp that created this one (`None` for the root).
    parent: Option<InterpId>,
}

impl InterpSlot {
    fn is_root(&self) -> bool {
        self.parent.is_none()
    }
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

/// Canonicalise a command name's separators into the VM's key form: a run of
/// two or more colons is **one** separator (C's `TclGetNamespaceForQualName` —
/// `foo:::bar` names `foo::bar`), the leading root drops (keys are unrooted),
/// and a trailing run survives as a single `::` — it names the empty-tailed
/// (`{}`) command in the qualified namespace (`proc quux::: {} {}` defines
/// `::quux::`, tclsh8.6-verified). Borrows when `name` is already in key form
/// (a lone leading/interior `:` is an ordinary name character).
pub(crate) fn canonical_cmd_key(name: &str) -> std::borrow::Cow<'_, str> {
    if !name.starts_with(':') && !name.contains(":::") {
        return std::borrow::Cow::Borrowed(name);
    }
    let mut out = join_segments(name);
    if !out.is_empty() && tcl_syntax::naming::ends_with_separator(name.as_bytes()) {
        out.push_str("::");
    }
    std::borrow::Cow::Owned(out)
}

/// Canonicalise a namespace name's separators: as [`canonical_cmd_key`], but a
/// trailing separator run drops entirely — `namespace eval c::: {}` creates
/// `::c` (tclsh8.6-verified), never a namespace named `c::`.
fn canonical_ns_name(name: &str) -> std::borrow::Cow<'_, str> {
    if !name.starts_with(':') && !name.contains(":::") && !name.ends_with("::") {
        return std::borrow::Cow::Borrowed(name);
    }
    std::borrow::Cow::Owned(join_segments(name))
}

/// [`tcl_syntax::naming::key_holder_and_tail`] for the VM's **unrooted** key
/// convention: root the key, split by the construction-inverse rule, and
/// unroot the holder.  Distinct from the written-name colon-run split — an
/// unrooted key can begin with `::` when a namespace is named `:` (#934), and
/// the all-colon disambiguation differs between the rooted and unrooted
/// grammars.
pub(crate) fn key_holder_and_tail_unrooted(key: &str) -> (String, String) {
    let rooted = format!("::{key}");
    let (holder, tail) = tcl_syntax::naming::key_holder_and_tail(&rooted);
    let holder = holder.strip_prefix("::").unwrap_or(holder);
    (holder.to_string(), tail.to_string())
}

/// Quote a trace-callback argument as a single Tcl word: empty or
/// whitespace-bearing values are brace-wrapped, simple words are passed bare.
fn tcl_brace(s: &str) -> String {
    if s.is_empty() || s.contains(char::is_whitespace) || s.contains(['[', ']', '$', '{', '}']) {
        format!("{{{s}}}")
    } else {
        s.to_string()
    }
}

/// Split an `arr(key)` variable reference into `(base, key)`, or `None` for a
/// plain scalar/array name — `TclObjLookupVarEx`'s rule, from the shared
/// naming owner. Both halves may be empty: `(x)` is element `x` of the array
/// named `""` (issue #1458).
fn elem_ref(name: &str) -> Option<(&str, &str)> {
    tcl_syntax::naming::split_element_ref(name)
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
/// addressable in the arena throughout (the fix for issue #946 faults 1–2).
pub struct Vm {
    /// Process-unique identity for embedder artefacts. A reusable compiled
    /// handle belongs to the VM that compiled it; generations alone are not a
    /// cross-VM identity domain.
    pub(crate) owner_nonce: u64,
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
    /// Cross-interp alias records for the target-death sweep. Appended when a
    /// [`Command::CrossAlias`] registers; entries are validated lazily at
    /// sweep time (a stale entry — alias since removed or source dead — is
    /// skipped).
    alias_backrefs: Vec<AliasBackref>,
}

/// Identity domain for metadata that follows a command binding.  Tcl permits
/// a hidden token and a visible command to have the same spelling at the same
/// time; consequently a bare `String` is not a command identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CommandSidecarKey {
    Visible(String),
    Hidden(String),
}

/// Relocation-aware identity held by an in-flight command operation.
///
/// Hide, expose, and rename move a command's sidecar domain while its body may
/// still be running. Every active consumer shares this cell, so a relocation
/// updates coroutine parking/retirement and execution-trace leave delivery as
/// one operation.
#[derive(Clone, Debug)]
pub(crate) struct CommandSidecarHandle(Rc<RefCell<Option<CommandSidecarKey>>>);

impl CommandSidecarHandle {
    /// The binding this operation still belongs to.  A deletion detaches an
    /// in-flight handle before Tcl can create another binding at the same
    /// spelling, so it can never be retargeted to that later lifecycle.
    pub(crate) fn key(&self) -> Option<CommandSidecarKey> {
        self.0.borrow().clone()
    }

    /// Whether this in-flight operation still belongs to a command binding.
    /// Deletion clears the cell before Tcl callbacks run; trace iteration must
    /// re-check this after every re-entrant callback rather than continuing a
    /// cloned pre-mutation trace list.
    pub(crate) fn is_attached(&self) -> bool {
        self.0.borrow().is_some()
    }
}

impl CommandSidecarKey {
    pub(crate) fn visible(name: impl Into<String>) -> Self {
        Self::Visible(name.into())
    }

    pub(crate) fn hidden(name: impl Into<String>) -> Self {
        Self::Hidden(name.into())
    }

    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Visible(name) | Self::Hidden(name) => name,
        }
    }
}

/// Metadata and any release-hidden destination displaced while a command is
/// temporarily installed under a new name.  A rename can still be refused
/// after installation when it would make an alias loop, so this keeps the
/// command-table mutation reversible as one semantic operation.
pub(crate) struct CommandRenameTransaction {
    old_key: String,
    new_key: String,
    source_import_origin: Option<CommandSidecarKey>,
    source_builtin_identity: Option<String>,
    /// Whether the source was an engine-installed `TclOO` root. Like
    /// `source_builtin_identity`, this is an *identity* that must travel with
    /// the command: `rename oo::configurable myconf` leaves a command that is
    /// still the registry's `oo::configurable`, so it must still vanish on a
    /// release that has no such builtin.
    source_registry_object_root: Option<String>,
    cross_alias_target: Option<InterpId>,
}

impl CommandRenameTransaction {
    pub(crate) fn old_key(&self) -> &str {
        &self.old_key
    }
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

/// One interpreter's complete state: command table, namespaces, call frames,
/// error state, traces, coroutines, channels, children. The engine ([`Vm`])
/// executes with exactly one of these current at a time and swaps between
/// them at interpreter boundaries.
pub struct InterpState {
    /// The Tcl release whose number/expr grammar this VM emulates —
    /// threaded from `DialectProfile::vm_runtime_version` (dialect-profile
    /// model §5.4). The profile pins the release-specific grammar, command,
    /// and variable-resolution semantics; the VM never infers them from a
    /// dialect name.
    runtime_version: tcl_dialect::TclVersion,
    /// The dialect profile this VM validates its builtin command surface
    /// against (issue #1463): [`Self::builtin_command_visible_for_surface`]
    /// consults this profile's availability mask, so a command the emulated
    /// release does not have (`lassign` at 8.4, `lpop` before 9.0) resolves
    /// like C Tcl — to `invalid command name`. Defaults to the permissive
    /// fallback profile, which hides nothing; `set_runtime_version` pins the
    /// matching plain-Tcl profile and `set_dialect_profile` pins a vendor one.
    dialect_profile: &'static tcl_dialect::DialectProfile,
    /// The profile used solely for builtin command-surface availability.
    /// Normally identical to [`Self::dialect_profile`], but an embedding host
    /// may expose a broader Tcl host surface while retaining a vendor grammar
    /// and bytecode identity (for example, the iRules simulation harness).
    command_surface_profile: &'static tcl_dialect::DialectProfile,
    /// The availability registry for [`Self::command_surface_profile`], resolved once
    /// at pin time (`registry_for_profile` guards its cache with a lock, and
    /// this is consulted on every command resolution). `None` for the
    /// permissive fallback profile, which gates nothing.
    profile_registry: Option<&'static tcl_registry::CommandRegistry>,
    /// Monotonic invalidation generation for bytecode that depends on the
    /// selected profile's grammar or command surface. It is deliberately
    /// independent of `cmd_epoch`: command resolution can be recomputed from
    /// names, whereas specialised bytecode must be recompiled from source.
    profile_generation: u64,
    /// Call-frame stack; `frames[0]` is the global scope.
    frames: Vec<CallFrame>,
    /// Command table (builtins + user procs), keyed by canonical name — a
    /// builtin's simple name, or a proc's namespace-qualified name without the
    /// leading `::` (e.g. `foo::bar`; a global proc is just `bar`).
    commands: HashMap<String, Command>,
    /// The fixed C math-function table used by pre-TIP-232 `expr` (Tcl 8.4).
    ///
    /// Normal command registration also installs these handlers under
    /// `tcl::mathfunc::*` for Tcl 8.5+, but an 8.4 expression must keep using
    /// this immutable builtin table even if a script later creates or replaces
    /// a similarly named command. The registry decides when this table applies;
    /// this map merely retains the registered implementation.
    fixed_math_builtins: HashMap<String, BuiltinFn>,
    /// Pre-compiled proc bodies from the module(s), keyed by qualified name.
    module_procs: HashMap<String, Rc<FunctionAsm>>,
    /// Current-namespace stack (canonical, no leading `::`; `""` = global). The
    /// top governs `proc`/command/variable name resolution. `namespace eval`
    /// and proc activation push/pop it.
    ns_stack: Vec<String>,
    /// Existing namespaces (canonical names; `""` global is implicit).
    namespaces: std::collections::HashSet<String>,
    /// Export patterns per namespace (canonical name → glob patterns), set by
    /// `namespace export` and consulted by `namespace import`.
    ns_exports: HashMap<String, Vec<String>>,
    /// Import provenance: canonical key of a command created by `namespace
    /// import` → the canonical FQN of its origin (source) command. Lets
    /// `namespace forget` remove *only* imported commands (C
    /// `Tcl_ForgetImport`, which matches on `deleteProc == DeleteImportedCmd`),
    /// leaving a real command of the same name intact. Cleared whenever the key
    /// is re-registered (a redefine) or taken (a `rename`).
    imported_commands: HashMap<String, CommandSidecarKey>,
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
    ns_arena: Vec<String>,
    ns_intern: HashMap<String, NsId>,
    /// Per-namespace command resolution path (`namespace path`): canonical
    /// namespace name (no leading `::`, `""` = global) → the ordered list of
    /// namespaces (canonical) consulted after the current namespace and before
    /// the global one during command lookup. Absent / empty = the default
    /// (current → global only).
    ns_paths: HashMap<String, Vec<String>>,
    /// Per-namespace `namespace unknown` handlers (TIP 181): canonical
    /// namespace name → the handler command prefix. Consulted on a
    /// resolution miss for the *current* namespace, then the global
    /// namespace's handler (the interp default), then the plain `unknown`
    /// proc. NOT inherited by child namespaces (tclsh 8.6.16 /
    /// 9.0.4-pinned). Absent / empty = use the default chain.
    ns_unknowns: HashMap<String, Vec<Value>>,
    /// Reentrancy guard: non-zero while a `namespace unknown` handler itself
    /// is being dispatched, so a handler whose own head is unresolvable falls
    /// through to a hard `invalid command name` instead of recursing.
    ns_unknown_depth: u32,
    /// Command-FQN ⇆ dense raw `CommandId` arena for `Namespaces::find_command`
    /// and `Commands::dispatch_id`. Interior-mutable because `find_command` is
    /// `&self` but mints a handle on first sight. Bidirectional: `find_command`
    /// interns an absolute FQN, `dispatch_id` reverses the id to that FQN and
    /// invokes it.
    cmd_arena: RefCell<CmdArena>,
    /// Provided packages → version (`package provide`/`require`).
    packages: HashMap<String, String>,
    /// Variable traces, keyed by a resolved-owner key (frame level + name) so a
    /// trace fires regardless of the access path (`upvar` alias, qualified
    /// name, …). Newest trace last; fired newest-first.
    var_traces: HashMap<String, Vec<VarTrace>>,
    /// `trace add command` registrations (`rename`/`delete` ops), keyed by the
    /// command's table key.  Entries follow the command through `rename` and
    /// are dropped when it is deleted or overwritten (M16.3).
    pub(crate) cmd_traces: HashMap<CommandSidecarKey, Vec<Rc<CmdTraceEntry>>>,
    /// `trace add execution` registrations (`enter`/`leave`/`enterstep`/
    /// `leavestep` ops), keyed like [`Self::cmd_traces`] and moved on rename.
    pub(crate) exec_traces: HashMap<CommandSidecarKey, Vec<Rc<CmdTraceEntry>>>,
    /// Weak registry of in-flight sidecar identities. Relocation updates live
    /// handles without retaining completed invocations.
    active_sidecar_handles: Vec<Weak<RefCell<Option<CommandSidecarKey>>>>,
    /// Step-trace scopes currently active: one entry per `enterstep`/
    /// `leavestep`-bearing trace of each traced proc whose body is running.
    /// Every command dispatched while non-empty fires these (C's step
    /// semantics); the traced proc's frame pops its own pushes on completion.
    pub(crate) exec_step_scopes: Vec<crate::exec::ExecStepScope>,
    /// Bumped on every `trace add|remove execution … enterstep|leavestep …`
    /// (and command rename, which moves such a trace's registration) — the
    /// analogue of C's `compileEpoch` bump on `DONT_COMPILE_CMDS_INLINE`
    /// toggles. `ProcDef::compiled_epoch` records which epoch a proc's body
    /// was compiled under; a stale epoch triggers a recompile (traced or
    /// untraced, per [`Self::step_trace_active`]) the next time the proc is
    /// entered — see [`Vm::ensure_proc_compiled_for_tracing`].
    trace_deopt_epoch: std::cell::Cell<u64>,
    /// Set for the duration of [`Vm::run_cmd_trace_callback`]'s evaluation:
    /// C's `INTERP_TRACE_IN_PROGRESS` (`tclTrace.c`) — while any command or
    /// execution trace callback is running, no further interp-wide trace
    /// firing happens for commands *inside* that callback (`TclCheckInterpTraces`
    /// returns immediately). Without this, a step-traced proc's `enterstep`
    /// callback (e.g. `puts "…"`) would itself be step-observed, and its own
    /// `puts` dispatch would recurse into firing traces again. A `Cell`, not
    /// a plain `bool` field, so a read-only dispatch-site check
    /// (`self.trace_in_progress.get()`) doesn't need `&mut self` — and so it
    /// doesn't count toward `clippy::struct_excessive_bools` alongside the
    /// interp's genuine bool-valued state (`is_safe`, `debug_frame`, …).
    pub(crate) trace_in_progress: std::cell::Cell<bool>,
    /// A traced dispatch that deferred its body to a pushed frame parks its
    /// leave context here for `drive_loop` to move onto that frame — set and
    /// drained within one trampoline step.
    pub(crate) pending_exec_leave: Option<crate::exec::ExecLeaveCtx>,
    /// M16.4 — the command-resolution memo: `(epoch, cxt‹U+1›name → key)`.
    /// C caches a resolution on the name object and invalidates by interp
    /// epoch (`cmdRefEpoch`); the VM has no per-object intreps, so the memo
    /// lives here, valid only while its stored epoch equals
    /// [`Self::cmd_epoch`].  Interior-mutable because resolution is `&self`.
    cmd_resolve_cache: std::cell::RefCell<(u64, HashMap<String, Option<String>>)>,
    /// Bumped by every mutation that can change what a name resolves to:
    /// command registration/removal (any path), `namespace path` writes,
    /// namespace deletion sweeps, and runtime-version flips (the 8.4 path-
    /// tier gate).  See `bump_cmd_epoch`.
    cmd_epoch: std::cell::Cell<u64>,
    /// Runtime-issued speculative guard tokens and mutation-domain snapshots.
    guards: std::cell::RefCell<GuardManager>,
    /// Stable semantic identities explicitly attached to guardable builtins.
    /// Ordinary command registration cannot authorise a fast path.
    guarded_commands: std::cell::RefCell<HashMap<String, BTreeSet<GuardIdentity>>>,
    /// Re-entrancy guard: `"<key>\0<op>"` entries for traces currently firing.
    active_traces: std::collections::HashSet<String>,
    /// Frame depths at which the currently-executing `namespace eval`/`inscope`
    /// bodies started (innermost last). A namespace body runs in the frame that
    /// invoked it, so when the current frame depth matches the innermost entry
    /// we are directly in a namespace script (unqualified names alias namespace
    /// variables); inside a proc called from one, the depths differ and
    /// unqualified names are proc locals.
    ns_script_frames: Vec<usize>,
    // Shared with child interpreters (`interp create`): a child writes `puts`
    // output to the same sink and compiles dynamic scripts with the same
    // (stateless) compile service, so both are `Rc` rather than owned.
    out: Rc<RefCell<Box<dyn Write>>>,
    compiler: Option<Rc<dyn CompileService<Module = ModuleAsm>>>,
    /// Optional debug hook fired once per source command (the execution-control
    /// seam a step debugger drives). `None` in normal runs — the only
    /// per-instruction cost is an `Option` check.
    debug_hook: Option<crate::debug::DebugHook>,
    /// The `(line, span-start)` key of the last command the debug hook fired
    /// for, so it fires once per source command rather than per instruction
    /// (`startCommand` is emitted only conditionally, so it cannot be the
    /// boundary marker).
    last_debug_key: Option<u64>,
    /// Set by the `exit` command to the requested process code. The VM library
    /// never terminates the process itself — a standalone driver (the `tclvm`
    /// CLI) translates this into `std::process::exit`, while an embedder (the
    /// debugger, `tcl-irule-test`, `f5 explain-flow --simulate`) sees the
    /// unwinding completion and survives. `None` until `exit` runs.
    pending_exit: Option<i32>,
    /// Cache of compiled scripts for the runtime-`eval` / command-substitution
    /// path (`eval_source`), keyed by source text. Compilation is a pure
    /// function of the source and the currently selected profile, so a script
    /// re-evaluated every loop iteration — a `switch`/`if`/`while` body, a
    /// `[subst]`ed command, a tcltest `-body` — compiles once instead of each
    /// time. This is the dominant cost in the tcltest workload.
    eval_cache: HashMap<String, Rc<ModuleAsm>>,
    /// Trace-visible counterpart of [`Self::eval_cache`] — the SAME source
    /// text compiled with [`CompileService::compile_traced`] instead of
    /// [`compile`](CompileService::compile), used whenever
    /// [`Self::step_trace_active`] is true. A step-traced proc's `if`/`while`/
    /// `foreach`/`eval`/`uplevel`/`catch`/`try`/… bodies all funnel through
    /// `eval_source`/`compile_source_cached` (their runtime builtins evaluate
    /// bodies that way), so gating the cache choice here — rather than at
    /// each call site — makes every one of them trace-visible with one change
    /// (issue #946 fault 3). Kept as a wholly separate map (not a mode-keyed
    /// entry in `eval_cache`) so the two compiled forms of the same source
    /// never collide or get served to the wrong mode. Both maps are cleared
    /// when the profile changes.
    eval_cache_traced: HashMap<String, Rc<ModuleAsm>>,
    /// The accumulating `errorInfo` source trace (C's `iPtr->errorInfo`): the
    /// error message followed by `while executing` / `invoked from within`
    /// frames, built up as the error unwinds through commands. `None` until the
    /// first frame is logged (which selects `while executing`). Consumed and
    /// reset when an error is caught (`catch`) or published.
    error_info: Option<String>,
    /// C's `ERR_ALREADY_LOGGED`: set once the current command level has logged
    /// its frame (so the same bytecode frame is not re-logged), cleared at a
    /// real frame boundary (a nested `eval`/`[subst]`, a proc/control body) so
    /// the enclosing command logs its own `invoked from within` frame.
    error_logged: bool,
    /// The 1-based source line of the innermost command logged into the current
    /// `errorInfo` trace (C's `iPtr->errorLine`) — the line the `(procedure …
    /// line N)` / `("while" body line N)` frames report.
    error_line: u32,
    /// The word a builtin was invoked under (the source `objv[0]`, before
    /// namespace-path resolution). Lets a builtin report its invocation name in
    /// error messages — e.g. `::tcl::mathop::!` reached via `namespace path` says
    /// `wrong # args: should be "! boolean"`, not the resolved full name.
    invoked_name: Option<String>,
    /// Hidden-domain command currently entering a native handler. Consumed by
    /// lifecycle-aware handlers (currently coroutine resume) so an equal
    /// visible spelling cannot select the wrong sidecar state.
    pub(crate) invoked_sidecar: Option<CommandSidecarKey>,
    /// Open I/O channels (file handles), keyed by channel id (`file3`, …). The
    /// predefined `stdin`/`stdout`/`stderr` are not stored here; commands
    /// special-case those names.
    channels: HashMap<String, crate::cmd_chan::Channel>,
    /// Monotonic counter for minting fresh channel ids.
    chan_counter: u32,
    /// Stack of file paths currently being evaluated by `source`. The top is
    /// what `info script` returns; empty when not inside a `source`.
    script_stack: Vec<String>,
    /// Active call-nesting depth — C Tcl's `interp recursionlimit`. Bounds proc
    /// recursion so an infinite loop is a *catchable* error rather than a native
    /// stack overflow (the trampoline avoids host-stack growth for a *single*
    /// activation, but `uplevel`/`catch`/`[subst]` re-enter `eval_source`, which
    /// does recurse on the host stack). Tracked on every call-frame push/pop.
    recursion_depth: usize,
    /// Native-stack safety counter for `cmd_control.rs`'s runtime-command
    /// fallback specifically — see [`CONTROL_FALLBACK_DEPTH_LIMIT`]'s doc
    /// comment (issue #996). Deliberately a separate counter from
    /// `recursion_depth`: this tracks only the current native call chain
    /// through `cmd_if`/`cmd_while`/`cmd_for`/`each_loop`, a purely
    /// stack-safety bookkeeping value with no Tcl-visible meaning, unlike
    /// `recursion_depth` (which models `info level`/`interp
    /// recursionlimit` and must survive a coroutine suspend/resume via
    /// `swap_flow`). Not swapped there: whenever a coroutine is not
    /// literally paused mid-way through this specific fallback chain, it's
    /// zero regardless, and if it *is*, being slightly imprecise across a
    /// yield/resume is a far smaller risk than the correctness bugs a
    /// second copy of `recursion_depth`'s full save/restore plumbing would
    /// invite for comparatively little benefit.
    control_fallback_depth: usize,
    /// Native-stack safety counter for `TclOO` method dispatch (`cmd_oo.rs`'s
    /// `oo_dispatch`) — see [`OO_DISPATCH_DEPTH_LIMIT`]'s doc comment
    /// (issue #996). A separate counter from `recursion_depth` for the same
    /// reason `control_fallback_depth` is: purely stack-safety bookkeeping,
    /// no Tcl-visible meaning, not swapped on coroutine suspend/resume.
    oo_dispatch_depth: usize,
    /// The host environment: the capability seam (`tcl-platform`) through which
    /// every command reaches the filesystem, clock, env, stdio, subprocess, and
    /// sockets. The bytecode VM is a native target, so this defaults to a
    /// full-capability [`NativeHost`]; [`Vm::set_host`] swaps it (e.g. for a
    /// sandboxed, WASM-posture host in capability tests). An `Rc` (not `Box`) so
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
    children: HashMap<String, InterpId>,
    /// Whether this interp is safe (`interp create -safe` / `interp issafe`).
    is_safe: bool,
    /// Per-interp recursion bound (`interp recursionlimit`), default
    /// [`RECURSION_LIMIT`]. A child carries its own, independent of the parent's.
    recursion_limit: usize,
    /// Commands hidden by `interp create -safe` / `interp hide`, keyed by name —
    /// invocable via `interp invokehidden`, restorable with `interp expose`.
    hidden_commands: HashMap<String, Command>,
    /// Import provenance and stable builtin identities carried while a command
    /// lives in the hidden table.  Hiding is a temporary relocation, not a
    /// metadata-dropping deletion: expose must restore both under its new key.
    hidden_imported_commands: HashMap<String, CommandSidecarKey>,
    hidden_builtin_identities: HashMap<String, String>,
    /// Monotonic counter minting auto-generated child names (`interp0`, …).
    interp_counter: u64,
    /// `interp debug -frame` — a one-way switch (once on, stays on).
    debug_frame: bool,
    /// `interp bgerror` — the background-error handler command prefix.
    bgerror_handler: Value,
    /// `interp limit` configuration. Both limit types are enforced: `time`
    /// by [`Vm::limit_check_tick`], `commands` by [`Vm::charge_command`].
    limits: LimitSet,
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
    /// The `TclOO` object system's runtime state — the class/object registries,
    /// the active method-call stack (for `self`/`my`/`next`), and the current
    /// definition target (`oo::define`/`oo::objdefine`). See [`crate::cmd_oo`].
    pub(crate) oo: crate::cmd_oo::OoState,
    /// The coroutine subsystem: live coroutines, the active-driver stack (for
    /// `[info coroutine]` + the yield-boundary check), and the pending
    /// `yield`/`yieldto` request. See [`crate::cmd_coro`].
    pub(crate) coro: crate::cmd_coro::CoroSystem,
    /// A script an `eval`/`uplevel`/`apply`-style builtin wants run on the
    /// *explicit* stack (so a `yield` in it stays yieldable): the compiled body,
    /// its `errorInfo` body label, and an optional command name to delete once
    /// the pushed frame completes (`apply`'s temporary lambda proc — issue
    /// #1311). Set by the builtin and drained by `dispatch_words` into a
    /// [`Tick::PushScript`](crate::exec) (or run via a nested drive on the
    /// `invoke_command` fallback path), mirroring how `coro.pending` becomes a
    /// `Tick::Suspend`.
    pub(crate) pending_eval: Option<(Rc<FunctionAsm>, Option<&'static str>, Option<String>)>,
    /// A `catch` body an about-to-run `catch` wants evaluated on the *explicit*
    /// stack (so a `yield` in it stays yieldable). Unlike `pending_eval`, the
    /// body's completion is **absorbed** (not propagated): a catch frame runs it
    /// and its epilogue records the result/options and yields the status code.
    /// Set by `cmd_catch`, drained by `dispatch_words` into a
    /// [`Tick::PushCatch`](crate::exec) (or run via a nested drive on the
    /// `invoke_command` fallback).
    pub(crate) pending_catch: Option<crate::exec::CatchReq>,
    /// A `subst` an about-to-run `subst` command wants performed on the *explicit*
    /// stack (so a `yield` in a `[…]` stays yieldable). Set by `cmd_subst`, drained
    /// by `dispatch_words` into a [`Tick::PushSubst`](crate::exec) (or run via a
    /// nested drive on the `invoke_command` fallback). See [`Frame::subst`].
    pub(crate) pending_subst: Option<crate::exec::SubstReq>,
    /// A `foreach`/`lmap` runtime-fallback loop an about-to-run `each_loop` wants
    /// driven on the *explicit* stack (so a `yield` in its body stays yieldable —
    /// issue #1311). Set by `each_loop`, drained by `dispatch_words` into a
    /// [`Tick::PushEachLoop`](crate::exec) (or run via a nested drive on the
    /// `invoke_command` fallback). See [`Frame::each_loop`].
    pub(crate) pending_each_loop: Option<crate::exec::EachLoopReq>,
    /// A `try`'s next phase (body/handler/finally) an about-to-run `try` wants
    /// driven on the *explicit* stack (so a `yield` anywhere in it stays
    /// yieldable — issue #1311). Set by `cmd_try`/`advance_try`, drained by
    /// `dispatch_words` into a [`Tick::PushTry`](crate::exec) (or run via a
    /// nested drive loop on the `invoke_command` fallback). See
    /// [`Frame::try_ctx`].
    pub(crate) pending_try: Option<crate::cmd_try::TryReq>,
    /// The event loop's pending timer/idle events (`after`/`vwait`/`update`).
    /// The scheduler half of the coroutine subsystem. See [`crate::cmd_event`].
    pub(crate) events: crate::cmd_event::EventQueue,
    /// The `thread` package's per-interpreter state — disabled until the
    /// embedder calls [`Vm::enable_threads`]. See [`crate::cmd_thread`].
    pub(crate) thread: crate::cmd_thread::ThreadSystem,
}

/// A suspended coroutine's saved per-flow execution context: the call/namespace
/// stack tails **above** the shared global entry, plus the scalar error/script
/// state and the OO execution stacks. Exchanged with the live context by
/// [`Vm::swap_flow`]. `default()` is a fresh flow rooted at the global level (an
/// about-to-start coroutine, before it has pushed any frame).
#[derive(Default)]
pub(crate) struct ParkedFlow {
    frames: Vec<CallFrame>,
    ns_stack: Vec<String>,
    ns_script_frames: Vec<usize>,
    recursion_depth: usize,
    error_info: Option<String>,
    error_logged: bool,
    error_line: u32,
    invoked_name: Option<String>,
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
}

impl Default for LimitSet {
    fn default() -> Self {
        Self {
            cmd_command: Value::string(""),
            cmd_granularity: 1,
            cmd_value: None,
            time_command: Value::string(""),
            time_granularity: 10,
            time_value: None,
            value_bytes: None,
        }
    }
}

/// The command-identity arena backing `Namespaces::find_command` /
/// `Commands::dispatch_id`: a bijection between a command's absolute FQN and a
/// dense raw `CommandId` (the index into `fqns`). Minted on first `find_command`.
#[derive(Default)]
struct CmdArena {
    ids: HashMap<String, u32>,
    fqns: Vec<String>,
}

/// A single registered variable trace.
#[derive(Clone)]
struct VarTrace {
    /// Operations this trace fires on (`read`/`write`/`unset`/`array`).
    ops: Vec<String>,
    /// The command prefix invoked as `command name1 name2 op`.
    command: String,
}

/// One `trace add command|execution` registration: the op set
/// (`rename`/`delete` or `enter`/`leave`/`enterstep`/`leavestep`) and the
/// callback command prefix.  `firing` disables the entry while its own
/// callback runs (C's re-entrancy rule).
pub(crate) struct CmdTraceEntry {
    pub(crate) ops: Vec<String>,
    pub(crate) callback: String,
    firing: std::cell::Cell<bool>,
}

impl CmdTraceEntry {
    fn new(ops: Vec<String>, callback: String) -> Self {
        Self {
            ops,
            callback,
            firing: std::cell::Cell::new(false),
        }
    }

    /// Whether this entry fires for `op`.
    pub(crate) fn has_op(&self, op: &str) -> bool {
        self.ops.iter().any(|o| o == op)
    }

    /// Whether this entry's callback is currently running (its firings are
    /// suppressed while it is).
    pub(crate) fn firing(&self) -> bool {
        self.firing.get()
    }
}

impl Vm {
    /// A VM writing `puts` output to stdout.
    #[must_use]
    pub fn new() -> Self {
        Self::with_output(Box::new(io::stdout()))
    }

    /// A VM writing `puts` output to `out` (tests pass a capture buffer).
    #[must_use]
    pub fn with_output(out: Box<dyn Write>) -> Self {
        Self::with_shared_output(Rc::new(RefCell::new(out)))
    }

    /// Pin the Tcl release this VM's number/expr grammar emulates —
    /// callers thread `DialectProfile::vm_runtime_version` here, and the
    /// release-reporting globals (`tcl_version` / `tcl_patchLevel`) are
    /// re-derived immediately. The version vocabulary owns the release
    /// predicates used by the VM, so individual command implementations do
    /// not interpret a dialect or version name themselves.
    /// Install *this* interpreter's numeral grammar before it executes.
    ///
    /// The grammar is a property of the interpreter — it emulates exactly one
    /// release for its whole life, as C settles with a build-time `KILL_OCTAL`.
    /// The store backing it is thread-local, though, and one thread can hold
    /// **several** interpreters: a host that builds a 9.0 `Vm` beside a live 8.6
    /// one would otherwise retune the 8.6 one, so `expr {010}` in it started
    /// answering 10 instead of 8. Installing at construction alone is not
    /// enough — that makes the answer depend on which interpreter was built
    /// last.
    ///
    /// Claiming it at every execution entry (`run_module`, `eval_source`,
    /// `eval_expr`) makes each interpreter's own grammar hold for its own work,
    /// whatever else shares the thread, and re-entrancy is safe because the
    /// inner interpreter re-claims on the way in and the outer one re-claims on
    /// its next entry. Pinned by
    /// `a_second_interpreter_does_not_change_the_first_ones_grammar`.
    pub(crate) fn claim_number_grammar(&self) {
        tcl_syntax::number::set_runtime_syntax(self.runtime_version.number_syntax());
    }

    pub fn set_runtime_version(&mut self, version: tcl_dialect::TclVersion) {
        // A bare release pin is the matching plain-Tcl profile: the emulated
        // release is one fact carrying both the runtime semantics and the
        // command-surface availability mask (issue #1463).
        self.set_dialect_profile(tcl_dialect::DialectProfile::by_name(version.dialect_name()));
    }

    /// Pin the dialect profile this VM emulates — the profile form of
    /// [`Self::set_runtime_version`], for hosts whose dialect is a vendor
    /// profile rather than a plain Tcl release. The runtime version follows
    /// the profile's pinned `vm_runtime_version`, and the profile's
    /// availability mask becomes the builtin command-surface filter
    /// ([`Self::builtin_command_visible_for_surface`]).
    pub fn set_dialect_profile(&mut self, profile: &'static tcl_dialect::DialectProfile) {
        // The 8.4 `namespace path` tier gate (M10.1) and the availability
        // gate change resolution outcomes, so the command-resolution memo
        // (M16.4) must not survive a version flip.
        self.bump_cmd_epoch();
        if !std::ptr::eq(self.dialect_profile, profile) {
            self.profile_generation = self.profile_generation.wrapping_add(1);
            // Dynamic modules and their precompiled-proc sidecars may contain
            // profile-sensitive lowerings. ProcDef retains source and is
            // rebuilt lazily; these cache-only forms do not, so discard them.
            // Active frames carry this generation and fail closed at the
            // trampoline seam if a native callback changes profile mid-run.
            self.eval_cache.clear();
            self.eval_cache_traced.clear();
            self.module_procs.clear();
        }
        self.dialect_profile = profile;
        self.command_surface_profile = profile;
        self.profile_registry =
            (!profile.is_fallback()).then(|| tcl_registry::registry_for_profile(profile));
        self.runtime_version = profile.vm_runtime_version;
        // Install the release's numeric grammar for this runtime: `0755` is 493
        // under 8.6 and 755 under 9.0, `0b`/`0o` exist from 8.5 and `0d` / `_`
        // separators from 9.0. C settles this at build time (`KILL_OCTAL`), so
        // it is a property of the runtime rather than of each conversion — see
        // `tcl_syntax::number::set_runtime_syntax`. Embedders call this while
        // building the interpreter; the VM does not support switching release
        // mid-execution.
        tcl_syntax::number::set_runtime_syntax(self.runtime_version.number_syntax());
        self.write_release_globals();
    }

    /// Override only the builtin command-availability surface.
    ///
    /// Compilation, bytecode profile validation, lexer/expr semantics, and
    /// [`Self::dialect_profile`] remain unchanged. This is for embedding hosts
    /// that execute a sandboxed dialect inside a broader host interpreter.
    /// A named override is rejected when its Tcl runtime base is older than
    /// the compilation dialect's base, or when it hides a bytecode-compiled
    /// command available to that dialect. The permissive fallback is the
    /// deliberate universal-surface exception: it never hides a builtin, even
    /// though its inert runtime base remains Tcl 9.0. Otherwise a specialised
    /// opcode could execute a command that ordinary lookup hides. Returns
    /// whether the requested surface was installed.
    #[must_use]
    pub fn set_command_surface_profile(
        &mut self,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> bool {
        // `plain_tcl` deliberately gates no commands, so it is compatible
        // with every dialect independently of its fallback runtime base.
        // Keep the runtime and visibility checks coupled for named profiles:
        // a named surface must still be new enough *and* expose each compiled
        // command from the execution dialect.
        if !profile.is_fallback()
            && (profile.vm_runtime_version < self.dialect_profile.vm_runtime_version
                || !Self::command_surface_covers_compiled_commands(self.dialect_profile, profile))
        {
            return false;
        }
        self.bump_cmd_epoch();
        self.profile_generation = self.profile_generation.wrapping_add(1);
        self.eval_cache.clear();
        self.eval_cache_traced.clear();
        self.module_procs.clear();
        self.command_surface_profile = profile;
        self.profile_registry =
            (!profile.is_fallback()).then(|| tcl_registry::registry_for_profile(profile));
        true
    }

    /// Whether `surface` exposes every registry command whose source-profile
    /// bytecode may bypass normal command lookup.  Runtime-version ordering is
    /// necessary but not sufficient: profiles on the same Tcl release can
    /// carry disjoint availability masks (for example BPF versus BIG-IP).
    ///
    /// `Traits::BYTE_COMPILED` is the registry-owned declaration that a
    /// literal command head may be lowered to bytecode.  Checking its actual
    /// source and target profile visibility preserves compatible cross-profile
    /// hosts such as iRules over a plain Tcl 8.4 host without allowing a
    /// narrowed same-release surface to hide an intrinsic.
    fn command_surface_covers_compiled_commands(
        dialect: &'static tcl_dialect::DialectProfile,
        surface: &'static tcl_dialect::DialectProfile,
    ) -> bool {
        if surface.is_fallback() {
            return true;
        }
        let compiled_registry = tcl_registry::registry_for_profile(dialect);
        let surface_registry = tcl_registry::registry_for_profile(surface);
        compiled_registry.command_names().all(|name| {
            let Some(spec) = compiled_registry.get_for_dialect(name, dialect.availability_mask)
            else {
                return true;
            };
            !spec.traits.contains(tcl_registry::Traits::BYTE_COMPILED)
                || surface_registry
                    .get_for_dialect(name, surface.availability_mask)
                    .is_some()
        })
    }

    /// The profile currently governing builtin command availability.
    #[must_use]
    pub fn command_surface_profile(&self) -> &'static tcl_dialect::DialectProfile {
        self.command_surface_profile
    }

    /// The Tcl release this VM emulates (see
    /// [`Self::set_runtime_version`]).
    #[must_use]
    pub fn runtime_version(&self) -> tcl_dialect::TclVersion {
        self.runtime_version
    }

    /// The dialect profile this VM validates its command surface against
    /// (see [`Self::set_dialect_profile`]).
    #[must_use]
    pub fn dialect_profile(&self) -> &'static tcl_dialect::DialectProfile {
        self.dialect_profile
    }

    /// The release's `${…}` close rule — `Tcl_ParseVarName`'s brace-form
    /// delimiting, which the 8.x family and 9.x disagree about.
    ///
    /// Read from the same `DialectProfile::grammar` the compile path's
    /// `LexerConfig` comes from, so the interpreted `subst` engine and the
    /// compiled word path cannot answer `${a{b}c}` differently (issue #1457).
    #[must_use]
    pub(crate) fn braced_var_style(&self) -> tcl_dialect::BracedVarStyle {
        self.dialect_profile.grammar.braced_var
    }

    /// Generation of the dialect profile used to compile dynamic bytecode.
    #[must_use]
    pub(crate) fn profile_generation(&self) -> u64 {
        self.profile_generation
    }

    /// The backslash-escape grammar this VM decodes under — the pinned
    /// profile's, so a VM emulating 8.5 reads `\x4142` as `B` and one
    /// emulating 9.0 reads it as `A42` (issue #1479).
    #[must_use]
    pub fn escape_syntax(&self) -> tcl_dialect::EscapeSyntax {
        self.dialect_profile.grammar.escapes
    }

    /// A VM writing to an already-shared output sink.
    fn with_shared_output(out: Rc<RefCell<Box<dyn Write>>>) -> Self {
        static NEXT_VM_OWNER: AtomicU64 = AtomicU64::new(1);
        let mut vm = Self {
            owner_nonce: NEXT_VM_OWNER.fetch_add(1, Ordering::Relaxed),
            state: Box::new(InterpState::fresh(out)),
            cur: ROOT_INTERP,
            interps: vec![InterpSlot {
                parked: None,
                active: 0,
                dying: false,
                parent: None,
            }],
            activation_depth: 0,
            alias_backrefs: Vec::new(),
        };
        register_builtins(&mut vm);
        vm.bootstrap_globals();
        vm
    }
}

impl InterpState {
    /// Release/dialect visibility belongs to the interpreter state because
    /// command candidate traversal happens here, including for parked child
    /// interpreters.  Filtering after resolution is too late: an unavailable
    /// namespace-local builtin must not shadow a later path/global candidate.
    /// The registry owns the versioned filters; procedures and aliases are
    /// invariant, while registered builtin/native handlers retain a stable
    /// identity through import, rename, hide, and expose.
    fn builtin_command_visible_for_surface(&self, name: &str, command: &Command) -> bool {
        self.builtin_command_visible_for_identity(name, command, None)
    }

    fn builtin_command_visible_for_identity(
        &self,
        name: &str,
        command: &Command,
        identity: Option<&str>,
    ) -> bool {
        // `Command::Object` joins the gated kinds for the TclOO **roots** only.
        // `oo::object`/`oo::class`/`oo::configurable` are command-table entries
        // the engine installs on behalf of the registry (which dates them
        // TCL86_PLUS / TCL90_PLUS), so an 8.4 or 8.5 surface must not carry
        // them and an 8.6 surface must not carry `oo::configurable`. Every
        // *other* object command is script-created — `oo::class create lpop`
        // is a user command that happens to share a registry name — so it is
        // invariant, exactly like a proc.
        //
        // A root's registry identity is recorded with it, so a renamed root
        // keeps being dated by the name the registry knows rather than by the
        // name it now answers to.
        let root_identity = match command {
            Command::Builtin(_) | Command::Native(_) => None,
            Command::Object(_) => match self.registry_object_roots.get(name) {
                Some(identity) => Some(identity.clone()),
                None => return true,
            },
            _ => return true,
        };
        let origin = root_identity
            .or_else(|| identity.map(str::to_owned))
            .or_else(|| self.builtin_identities.get(name).cloned())
            .unwrap_or_else(|| {
                let mut current = CommandSidecarKey::visible(name);
                for _ in 0..=self.imported_commands.len() + self.hidden_imported_commands.len() {
                    let next = match &current {
                        CommandSidecarKey::Visible(visible) => self.imported_commands.get(visible),
                        CommandSidecarKey::Hidden(hidden) => {
                            self.hidden_imported_commands.get(hidden)
                        }
                    };
                    let Some(next) = next else {
                        break;
                    };
                    current = next.clone();
                }
                current.name().to_owned()
            });
        if !tcl_registry::expr_surface::RuntimeExprSurface::for_tcl_version(self.runtime_version)
            .permits_builtin_math_function_command(&origin)
        {
            return false;
        }
        matches!(command, Command::Native(_)) || self.profile_admits_registry_builtin(&origin)
    }

    fn profile_admits_registry_builtin(&self, name: &str) -> bool {
        let Some(registry) = self.profile_registry else {
            return true;
        };
        registry.get(name).is_none()
            || registry
                .get_for_dialect(name, self.command_surface_profile.availability_mask)
                .is_some()
    }

    /// A fresh interpreter state writing `puts` output to `out` — no commands
    /// registered yet ([`Vm::with_shared_output`] / [`Vm::fork_child`] follow
    /// up with `register_builtins`).
    fn fresh(out: Rc<RefCell<Box<dyn Write>>>) -> Self {
        let mut guards = GuardManager::default();
        // Interpreter-policy and TclOO dispatch mutations do not yet have one
        // central invalidation owner in this runtime. Fail closed until every
        // mutation site routes through such an owner; an epoch that is never
        // advanced must not authorise a speculative path.
        guards.poison(GuardDomain::Interpreter);
        guards.poison(GuardDomain::ObjectDispatch);
        Self {
            runtime_version: tcl_dialect::TclVersion::V9_0,
            dialect_profile: tcl_dialect::DialectProfile::plain_tcl(),
            command_surface_profile: tcl_dialect::DialectProfile::plain_tcl(),
            profile_registry: None,
            profile_generation: 0,
            frames: vec![CallFrame::new(0, ROOT_NS, None, Vec::new())],
            commands: HashMap::new(),
            fixed_math_builtins: HashMap::new(),
            module_procs: HashMap::new(),
            ns_stack: vec![String::new()],
            namespaces: std::collections::HashSet::new(),
            ns_exports: HashMap::new(),
            imported_commands: HashMap::new(),
            builtin_identities: HashMap::new(),
            registry_object_roots: HashMap::new(),
            ns_arena: vec![String::new()],
            ns_intern: HashMap::from([(String::new(), ROOT_NS)]),
            ns_paths: HashMap::new(),
            ns_unknowns: HashMap::new(),
            ns_unknown_depth: 0,
            cmd_arena: RefCell::new(CmdArena::default()),
            packages: HashMap::new(),
            var_traces: HashMap::new(),
            cmd_traces: HashMap::new(),
            exec_traces: HashMap::new(),
            active_sidecar_handles: Vec::new(),
            exec_step_scopes: Vec::new(),
            trace_deopt_epoch: std::cell::Cell::new(0),
            trace_in_progress: std::cell::Cell::new(false),
            pending_exec_leave: None,
            cmd_resolve_cache: std::cell::RefCell::new((0, HashMap::new())),
            cmd_epoch: std::cell::Cell::new(0),
            guards: std::cell::RefCell::new(guards),
            guarded_commands: std::cell::RefCell::new(HashMap::new()),
            active_traces: std::collections::HashSet::new(),
            ns_script_frames: Vec::new(),
            out,
            compiler: None,
            debug_hook: None,
            last_debug_key: None,
            pending_exit: None,
            eval_cache: HashMap::new(),
            eval_cache_traced: HashMap::new(),
            error_info: None,
            error_logged: false,
            error_line: 1,
            invoked_name: None,
            invoked_sidecar: None,
            channels: HashMap::new(),
            chan_counter: 2,
            script_stack: Vec::new(),
            recursion_depth: 0,
            control_fallback_depth: 0,
            oo_dispatch_depth: 0,
            host: Rc::new(NativeHost::new()),
            children: HashMap::new(),
            is_safe: false,
            recursion_limit: RECURSION_LIMIT,
            hidden_commands: HashMap::new(),
            hidden_imported_commands: HashMap::new(),
            hidden_builtin_identities: HashMap::new(),
            interp_counter: 0,
            debug_frame: false,
            bgerror_handler: Value::string("::tcl::Bgerror"),
            limits: LimitSet::default(),
            commands_run: 0,
            limit_tick: 0,
            // A fixed non-zero default so an un-`srand`'d `rand()` is still
            // deterministic; Tcl auto-seeds from the clock, but reproducibility
            // is more useful for the VM and every test seeds explicitly.
            rand_seed: 1,
            oo: crate::cmd_oo::OoState::default(),
            coro: crate::cmd_coro::CoroSystem::default(),
            pending_eval: None,
            pending_catch: None,
            pending_subst: None,
            pending_each_loop: None,
            pending_try: None,
            events: crate::cmd_event::EventQueue::default(),
            thread: crate::cmd_thread::ThreadSystem::default(),
        }
    }
}

impl Vm {
    /// The currently-executing interpreter's id.
    pub(crate) fn cur_interp(&self) -> InterpId {
        self.cur
    }

    /// Read another interpreter's state (or the current one's, uniformly).
    /// `None` for a dead id.
    pub(crate) fn st_of(&self, id: InterpId) -> Option<&InterpState> {
        if id == self.cur {
            Some(&self.state)
        } else {
            self.interps.get(id.0)?.parked.as_deref()
        }
    }

    /// Whether `id` is still addressable (created and not deleted).
    pub(crate) fn interp_alive(&self, id: InterpId) -> bool {
        self.interps
            .get(id.0)
            .is_some_and(|s| !s.dying && (id == self.cur || s.parked.is_some()))
    }

    /// Run `f` with `id` as the current interpreter — the cross-interp calling
    /// convention (C's `Tcl_EvalObjv(targetInterp, …)` on the shared C stack):
    /// the current state parks in its arena slot, the target state becomes
    /// current, and the previous interpreter is restored on the way out.  The
    /// nested evaluation is a plain native re-entry, so re-entering an
    /// interpreter that is already executing deeper on the stack is legal —
    /// its persistent state lives in the arena throughout.  Deferred teardown:
    /// if the target was deleted while (re-)entered, the last exit drops its
    /// state (C's `Tcl_Preserve`/`Tcl_Release`; tclsh-pinned).
    pub(crate) fn in_interp<R>(&mut self, id: InterpId, f: impl FnOnce(&mut Vm) -> R) -> R {
        let prev = self.cur;
        self.interps[id.0].active += 1;
        self.switch_to(id);
        let r = f(self);
        self.switch_to(prev);
        let slot = &mut self.interps[id.0];
        slot.active -= 1;
        if slot.dying && slot.active == 0 {
            self.finalize_slot(id);
        }
        r
    }

    /// Make `id` current by swapping its parked state with the current one.
    fn switch_to(&mut self, id: InterpId) {
        if id == self.cur {
            return;
        }
        let incoming = self.interps[id.0]
            .parked
            .take()
            .expect("switch target's state is parked in its slot");
        let outgoing = std::mem::replace(&mut self.state, incoming);
        self.interps[self.cur.0].parked = Some(outgoing);
        self.cur = id;
    }

    /// Mint a fresh arena slot for a new interpreter owned by `parent`.
    fn new_interp_slot(&mut self, state: Box<InterpState>, parent: InterpId) -> InterpId {
        let id = InterpId(self.interps.len());
        self.interps.push(InterpSlot {
            parked: Some(state),
            active: 0,
            dying: false,
            parent: Some(parent),
        });
        id
    }

    /// Delete interpreter `id`: its subtree dies first (C deletes children
    /// with their parent), cross-interp aliases TARGETING it are swept out of
    /// their source interps (C's target table — tclsh-pinned: the alias
    /// command disappears from `info commands`), and the state itself is
    /// dropped now — or, when the interp is still executing (a re-entered
    /// child, a target mid-callback), when its last in-flight evaluation
    /// unwinds (tclsh-pinned: the in-flight eval completes normally).
    fn retire_interp(&mut self, id: InterpId) {
        if self.interps.get(id.0).is_none_or(|s| s.dying) {
            return;
        }
        let child_ids: Vec<InterpId> = self
            .st_of(id)
            .map(|st| st.children.values().copied().collect())
            .unwrap_or_default();
        for c in child_ids {
            self.retire_interp(c);
        }
        let targeting: Vec<(InterpId, CommandSidecarKey)> = self
            .alias_backrefs
            .iter()
            .filter(|b| b.target == id && b.source != id)
            .map(|b| (b.source, b.key.clone()))
            .collect();
        for (src, key) in targeting {
            let visible = matches!(&key, CommandSidecarKey::Visible(name) if self.st_of(src).is_some_and(|st| matches!(st.commands.get(name), Some(Command::CrossAlias { target, .. }) if *target == id)));
            let hidden = matches!(&key, CommandSidecarKey::Hidden(name) if self.st_of(src).is_some_and(|st| matches!(st.hidden_commands.get(name), Some(Command::CrossAlias { target, .. }) if *target == id)));
            if visible {
                self.in_interp(src, |vm| {
                    vm.remove_command_exact(key.name());
                    vm.on_command_removed_for(&key);
                });
            }
            if hidden {
                self.in_interp(src, |vm| {
                    vm.hidden_commands.remove(key.name());
                    vm.hidden_imported_commands.remove(key.name());
                    vm.hidden_builtin_identities.remove(key.name());
                    vm.on_command_removed_for(&key);
                    vm.drop_alias_backref_key(&key);
                });
            }
        }
        // Hidden aliases are not dispatchable, so a stale/missing backref must
        // not keep one alive after its target dies.  Sweep the hidden tables as
        // a defensive counterpart to C's target-side alias list.
        let hidden_targeting: Vec<(InterpId, String)> = self
            .interps
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| slot.parked.as_ref().map(|st| (InterpId(index), st)))
            .flat_map(|(source, st)| {
                st.hidden_commands
                    .iter()
                    .filter(move |(_, command)| {
                        matches!(command, Command::CrossAlias { target, .. } if *target == id)
                    })
                    .map(move |(key, _)| (source, key.clone()))
            })
            .collect();
        for (source, key) in hidden_targeting {
            self.in_interp(source, |vm| {
                vm.hidden_commands.remove(&key);
                vm.hidden_imported_commands.remove(&key);
                vm.hidden_builtin_identities.remove(&key);
                let sidecar = CommandSidecarKey::hidden(&key);
                vm.on_command_removed_for(&sidecar);
                vm.drop_alias_backref_key(&sidecar);
            });
        }
        self.alias_backrefs.retain(|b| b.target != id);
        let slot = &mut self.interps[id.0];
        slot.dying = true;
        if slot.active == 0 {
            self.finalize_slot(id);
        }
    }

    /// Drop a dying interpreter's state (frames, commands, channels,
    /// coroutines) once nothing is executing in it any more.
    fn finalize_slot(&mut self, id: InterpId) {
        let slot = &mut self.interps[id.0];
        debug_assert!(slot.dying && slot.active == 0);
        slot.parked = None;
        self.alias_backrefs.retain(|b| b.source != id);
    }

    /// Resolve an `interp` path from the current interpreter: `""`/`{}` names
    /// the current interp, and each list element steps into the named child
    /// (C's multi-level paths — `interp eval {a b} …` addresses grandchild
    /// `b`).  A plain name is the common single-element case.
    pub(crate) fn resolve_interp_path(&self, path: &str) -> Result<InterpId, Completion<Value>> {
        if path.is_empty() {
            return Ok(self.cur);
        }
        let needs_split = path
            .bytes()
            .any(|b| b.is_ascii_whitespace() || matches!(b, b'{' | b'}' | b'"' | b'\\'));
        let elems: Vec<String> = if needs_split {
            match tcl_syntax::list::split_list(path) {
                Ok(e) => e.into_iter().map(std::borrow::Cow::into_owned).collect(),
                Err(_) => {
                    return Err(err(format!("could not find interpreter \"{path}\"")));
                }
            }
        } else {
            vec![path.to_string()]
        };
        let mut cur = self.cur;
        for name in &elems {
            let next = self
                .st_of(cur)
                .and_then(|st| st.children.get(name.as_str()).copied());
            match next {
                Some(c) if self.interp_alive(c) => cur = c,
                _ => return Err(err(format!("could not find interpreter \"{path}\""))),
            }
        }
        Ok(cur)
    }

    /// Record a just-registered cross-interp alias for the target-death sweep.
    pub(crate) fn note_alias_backref(&mut self, key: &str, target: InterpId) {
        self.note_alias_backref_key(CommandSidecarKey::visible(key), target);
    }

    fn note_alias_backref_key(&mut self, key: CommandSidecarKey, target: InterpId) {
        let source = self.cur;
        self.alias_backrefs
            .retain(|b| !(b.source == source && b.key.eq(&key)));
        self.alias_backrefs.push(AliasBackref {
            source,
            key,
            target,
        });
    }

    /// Move a cross-interp alias's target-death backref from `old_key` to
    /// `new_key` (an alias renamed in the current interp).
    pub(crate) fn retarget_alias_backref(
        &mut self,
        old_key: &str,
        new_key: &str,
        target: InterpId,
    ) {
        self.retarget_alias_backref_key(
            &CommandSidecarKey::visible(old_key),
            CommandSidecarKey::visible(new_key),
            target,
        );
    }

    fn retarget_alias_backref_key(
        &mut self,
        old_key: &CommandSidecarKey,
        new_key: CommandSidecarKey,
        target: InterpId,
    ) {
        let source = self.cur;
        self.alias_backrefs
            .retain(|b| !(b.source == source && (b.key.eq(old_key) || b.key == new_key)));
        self.alias_backrefs.push(AliasBackref {
            source,
            key: new_key,
            target,
        });
    }

    /// Drop any cross-interp alias backref for `key` in the current interp (the
    /// alias was deleted or overwritten with a non-alias).
    pub(crate) fn drop_alias_backref(&mut self, key: &str) {
        self.drop_alias_backref_key(&CommandSidecarKey::visible(key));
    }

    fn drop_alias_backref_key(&mut self, key: &CommandSidecarKey) {
        let source = self.cur;
        self.alias_backrefs
            .retain(|b| !(b.source == source && b.key.eq(key)));
    }

    /// Reseed the `rand()` generator (`srand(n)`): mask to 31 bits, and nudge a
    /// 0 / `2^31-1` seed off the generator's two fixed points (Tcl's
    /// `srand`).
    pub(crate) fn rand_seed_set(&mut self, value: i64) {
        let mut s = value & 0x7fff_ffff;
        if s == 0 || s == 2_147_483_647 {
            s ^= 0x075b_d924;
        }
        self.rand_seed = s;
    }

    /// Advance the Park–Miller minimal-standard generator one step and return a
    /// double in `[0, 1)` (`expr rand()`), using Schrage's overflow-safe form.
    // `rand_seed` is kept in `[1, 2^31 - 1]` and `IP` is `2^31 - 1`; both are
    // well under `f64`'s 2^53 exact-integer range, so the casts are lossless.
    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn rand_next(&mut self) -> f64 {
        const IA: i64 = 16807;
        const IP: i64 = 2_147_483_647;
        const IQ: i64 = 127_773;
        const IR: i64 = 2_836;
        let test = IA * (self.rand_seed % IQ) - IR * (self.rand_seed / IQ);
        self.rand_seed = if test > 0 { test } else { test + IP };
        self.rand_seed as f64 / IP as f64
    }

    /// Write the release-reporting globals derived from the threaded
    /// runtime version.
    ///
    /// The patch digit comes from [`tcl_dialect::TclVersion::patchlevel`] —
    /// the one table both this VM and `runtime/rust` read, so the two engines
    /// cannot report different patch levels for the same emulated release
    /// (issue #1328's centralisation finding).
    fn write_release_globals(&mut self) {
        self.write_scalar_raw(
            "tcl_version",
            Value::string(self.runtime_version.as_package_version()),
        );
        self.write_scalar_raw(
            "tcl_patchLevel",
            Value::string(self.runtime_version.patchlevel()),
        );
    }

    /// Populate the predefined global variables a fresh interpreter exposes:
    /// the `tcl_platform`/`env` arrays and the `argv`/`argv0`/`argc` scalars,
    /// so library scripts (tcltest) that read them at load time work.
    fn bootstrap_globals(&mut self) {
        use tcl_platform::backend::{self, key};
        let plat = [
            ("platform", "unix"),
            ("os", "Linux"),
            ("osVersion", ""),
            ("machine", std::env::consts::ARCH),
            ("byteOrder", "littleEndian"),
            ("wordSize", "8"),
            ("pointerSize", "8"),
            ("pathSeparator", ":"),
            ("engine", "Tcl"),
            // Honest default: a bare VM has no thread package. `enable_threads`
            // (the embedder opting in, e.g. `tcl-vm-cli`) flips this to `1`.
            ("threaded", "0"),
            ("user", ""),
        ];
        for (k, v) in plat {
            let _ = self.write_array_raw("tcl_platform", k, Value::string(v));
        }
        // Backend-introspection keys (the test-suite constraint overlay reads
        // these). The bytecode VM is a native interpreter, so the wasm / WASI /
        // eBPF facts come from the build's `cfg` (empty on a native build) and
        // may be overridden from the environment to evaluate another backend's
        // skip lists.
        let detected = |k: &str| -> String {
            backend::override_env_var(k)
                .and_then(|var| std::env::var(var).ok())
                .unwrap_or_else(|| {
                    match k {
                        key::WASM => backend::compiled_wasm_spec(),
                        key::WASI => backend::compiled_wasi_spec(),
                        key::WASI_VERSION => backend::compiled_wasi_host(),
                        key::EBPF => backend::compiled_ebpf_spec(),
                        _ => "",
                    }
                    .to_string()
                })
        };
        for (k, v) in [
            (key::RUNTIME, "bytecode".to_string()),
            (key::RUNTIME_VERSION, env!("CARGO_PKG_VERSION").to_string()),
            (key::WASM, detected(key::WASM)),
            (key::WASI, detected(key::WASI)),
            (key::WASI_VERSION, detected(key::WASI_VERSION)),
            (key::EBPF, detected(key::EBPF)),
        ] {
            let _ = self.write_array_raw("tcl_platform", k, Value::string(v.as_str()));
        }
        // `std::env` is unsupported on wasm32-unknown-unknown (a bare wasm host
        // has no process environment; the std shim panics). The VM runs there —
        // e.g. the pure-data coroutine VM on wasm — with an empty `env` array.
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        for (k, v) in std::env::vars() {
            let _ = self.write_array_raw("env", &k, Value::string(v));
        }
        self.write_scalar_raw("argv", Value::list(Vec::new()));
        self.write_scalar_raw("argv0", Value::string("tcltest"));
        self.write_scalar_raw("argc", Value::int(0));
        self.write_release_globals();
        self.write_scalar_raw("tcl_interactive", Value::int(0));
        // `tcl_library` is the directory holding the script library; C Tcl's
        // init derives it from `$env(TCL_LIBRARY)` (set when the caller points
        // the VM at a real library tree). Library scripts (tcltest) read it at
        // load time, so default it to "" rather than leaving it unset.
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        let tcl_library = std::env::var("TCL_LIBRARY").unwrap_or_default();
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        let tcl_library = String::new();
        self.write_scalar_raw("tcl_library", Value::string(tcl_library));
    }

    /// Install the on-demand autoloader: `unknown` / `auto_load` /
    /// `auto_load_index` procs plus `auto_path`, so an unresolved command is
    /// looked up in the library's `tclIndex` and its defining file sourced
    /// (`word.tcl`, etc.) — the mechanism C bootstraps in `init.tcl`. The VM has
    /// no full `init.tcl` path, so this is a focused subset: it does auto-load
    /// (no auto-exec of external programs) and otherwise errors `invalid command
    /// name`, matching C's miss. Requires a compiler (uses `eval_source`); call
    /// after [`Self::set_compiler`]. Returns the bootstrap's completion.
    pub fn init_auto_load(&mut self) -> Completion<Value> {
        match self.eval_source(AUTO_LOAD_BOOTSTRAP) {
            Ok(c) => c,
            Err(e) => err(e.message),
        }
    }

    /// Install a debug hook fired once per source command (the execution-control
    /// seam a step debugger drives). Pass `None` to detach.
    pub fn set_debug_hook(&mut self, hook: Option<crate::debug::DebugHook>) {
        self.debug_hook = hook;
        self.last_debug_key = None;
    }

    /// Record a pending `exit` with the given process code. The VM library does
    /// not terminate the process; the driver decides (see [`Self::take_exit`]).
    pub(crate) fn set_exit(&mut self, code: i32) {
        self.pending_exit = Some(code);
    }

    /// Whether an `exit` is pending (the unwinding completion should propagate
    /// uncatchably, like C Tcl's `Tcl_Exit`).
    #[must_use]
    pub fn exit_pending(&self) -> bool {
        self.pending_exit.is_some()
    }

    /// Take the pending `exit` code, if any. A standalone driver (the `tclvm`
    /// CLI) calls this after running and translates `Some(code)` into
    /// `std::process::exit`; an embedder may ignore it and keep running.
    pub fn take_exit(&mut self) -> Option<i32> {
        self.pending_exit.take()
    }

    /// Fire the debug hook for the command an instruction belongs to, if a hook
    /// is installed and this instruction begins a *new* source command (so the
    /// hook fires once per command, not per instruction). Returns `true` when
    /// the hook asked to stop. `span_start` is the instruction's source-span
    /// start (`None` for synthetic instructions, which never fire).
    pub(crate) fn debug_step(
        &mut self,
        line: u32,
        span_start: Option<u32>,
        cmd_text: &str,
    ) -> bool {
        if self.debug_hook.is_none() {
            return false;
        }
        let Some(start) = span_start else {
            return false;
        };
        let key = (u64::from(line) << 32) | u64::from(start);
        if self.last_debug_key == Some(key) {
            return false;
        }
        self.last_debug_key = Some(key);
        self.fire_debug_hook(line, cmd_text)
    }

    /// Build a [`crate::debug::DebugSnapshot`] of the current interpreter state
    /// for a command at `line` with text `cmd_text`. Reads the call stack (top
    /// first) and the current frame's variables.
    pub(crate) fn debug_snapshot(&self, line: u32, cmd_text: &str) -> crate::debug::DebugSnapshot {
        use crate::debug::{DebugFrame, DebugSnapshot, DebugVar};
        let stack: Vec<DebugFrame> = self
            .frames
            .iter()
            .rev()
            .map(|fr| DebugFrame {
                level: u32::try_from(fr.level).unwrap_or(0),
                name: fr.proc_name.clone().unwrap_or_else(|| "global".to_owned()),
                namespace: fr.ns_eval.clone().unwrap_or_else(|| self.ns_name(fr.ns)),
            })
            .collect();
        let mut variables: Vec<DebugVar> = self
            .frame_var_names(false)
            .into_iter()
            .map(|name| {
                let value = self
                    .get_var(&name)
                    .map(|v| v.to_str().to_string())
                    .unwrap_or_default();
                DebugVar { name, value }
            })
            .collect();
        variables.sort_by(|a, b| a.name.cmp(&b.name));
        DebugSnapshot {
            line,
            command_text: cmd_text.to_owned(),
            level: u32::try_from(self.current_level()).unwrap_or(0),
            stack,
            variables,
        }
    }

    /// Fire the debug hook (if installed) for the command at `line` / `cmd_text`.
    /// Returns `true` when the hook asked to stop the run.
    pub(crate) fn fire_debug_hook(&mut self, line: u32, cmd_text: &str) -> bool {
        if self.debug_hook.is_none() {
            return false;
        }
        let snapshot = self.debug_snapshot(line, cmd_text);
        // Take the hook out so the closure can borrow `self`-derived data
        // (already captured in `snapshot`) without aliasing the field.
        let mut hook = self.debug_hook.take();
        let action = hook
            .as_mut()
            .map_or(crate::debug::DebugAction::Continue, |h| h(&snapshot));
        self.debug_hook = hook;
        action == crate::debug::DebugAction::Stop
    }

    /// Inject the compiler used for runtime `eval` / command substitution.
    pub fn set_compiler(&mut self, compiler: Box<dyn CompileService<Module = ModuleAsm>>) {
        self.compiler = Some(Rc::from(compiler));
    }

    /// The host environment (capability seam) backing the platform commands.
    pub(crate) fn host(&self) -> &dyn Host {
        &*self.host
    }

    /// A cloned handle to the host, so a command can hold `&dyn Host` while also
    /// taking `&mut self` as the `ValueOps` a shared `tcl-cmd-core` helper needs.
    pub(crate) fn host_rc(&self) -> Rc<dyn Host> {
        Rc::clone(&self.host)
    }

    /// Swap the host environment — e.g. a [`NativeHost::sandboxed`] to exercise
    /// the WASM-posture "unsupported" paths natively.
    pub fn set_host(&mut self, host: Rc<dyn Host>) {
        self.host = host;
    }

    pub(crate) fn register(&mut self, name: &str, f: BuiltinFn) {
        // Builtin registrations pass plain literals, bare (`set`) or rooted
        // (`::tcl::array::exists`) — never colon-tree keys — so a single root
        // strip converts to the canonical unrooted key form exactly.
        let canonical = name.strip_prefix("::").unwrap_or(name);
        if tcl_registry::mathfunc::is_in_mathfunc_namespace(canonical) {
            self.fixed_math_builtins.insert(canonical.to_owned(), f);
        }
        self.register_command(canonical, Command::Builtin(f));
    }

    /// Register a builtin together with a stable semantic identity that
    /// offline-generated code may request in a runtime guard.
    ///
    /// Ordinary builtins deliberately have no such identity. Adding one is an
    /// explicit runtime implementation decision, not an inference from the
    /// command's spelling or handler address.
    pub fn register_guarded_builtin(&mut self, name: &str, f: BuiltinFn, identity: GuardIdentity) {
        let canonical = name.strip_prefix("::").unwrap_or(name);
        self.register(canonical, f);
        self.guarded_commands
            .borrow_mut()
            .entry(canonical.to_owned())
            .or_default()
            .insert(identity);
    }

    /// Register a builtin and derive every semantic identity from its registry
    /// specification, including subcommand and form intrinsics.
    pub fn register_spec_builtin(&mut self, spec: &tcl_registry::CommandSpec, f: BuiltinFn) {
        self.register(spec.name, f);
        let identities: BTreeSet<_> = spec
            .intrinsic_ids()
            .into_iter()
            .flat_map(|id| {
                id.guard_semantics_variants().iter().map(move |semantics| {
                    GuardIdentity::registry_intrinsic_with_semantics(id.stable_id(), *semantics)
                })
            })
            .collect();
        if !identities.is_empty() {
            self.guarded_commands
                .borrow_mut()
                .insert(spec.name.trim_start_matches("::").to_owned(), identities);
        }
    }

    /// Verify the live command identity and snapshot the requested mutation
    /// domains. Any active trace in a requested trace domain conservatively
    /// refuses issuance; an epoch snapshot is not an absence proof.
    pub fn prepare_command_guard(
        &self,
        name: &str,
        expected: GuardIdentity,
        domains: GuardDomains,
    ) -> Result<GuardToken, GuardError> {
        if (domains.contains(GuardDomain::CommandTrace)
            && !(self.cmd_traces.is_empty() && self.exec_traces.is_empty()))
            || (domains.contains(GuardDomain::VariableTrace) && !self.var_traces.is_empty())
        {
            return Err(GuardError::PrerequisiteUnsatisfied);
        }
        let observed = self
            .resolve_command_fqn(self.current_ns(), name)
            .and_then(|key| {
                let identities = self.guarded_commands.borrow();
                let identities = identities.get(&key)?;
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

    /// Re-check a command guard against current live identity and epochs.
    #[must_use]
    pub fn check_command_guard(&self, token: GuardToken, name: &str) -> bool {
        let Some(key) = self.resolve_command_fqn(self.current_ns(), name) else {
            return false;
        };
        let identities = self.guarded_commands.borrow();
        let Some(identities) = identities.get(&key) else {
            return false;
        };
        identities
            .iter()
            .any(|identity| self.guards.borrow().check(token, Some(*identity)))
    }

    /// Execute one target-neutral intrinsic over arguments after command and
    /// subcommand dispatch. `None` means this runtime does not implement the ID
    /// or invocation shape and the caller must use its slow path.
    pub fn execute_intrinsic(
        &mut self,
        intrinsic: tcl_registry::IntrinsicId,
        args: &[Value],
    ) -> Option<Completion<Value>> {
        match (intrinsic, args) {
            (tcl_registry::IntrinsicId::StringLength, [value]) => {
                Some(ok(tcl_cmd_core::string::length(self, value)))
            }
            _ => None,
        }
    }

    /// Release one command guard token.
    #[must_use]
    pub fn release_command_guard(&self, token: GuardToken) -> bool {
        self.guards.borrow_mut().release(token)
    }

    /// Invoke the pre-TIP-232 fixed `expr` math-function entry selected by the
    /// registry. This intentionally bypasses normal command lookup: Tcl 8.4's
    /// `expr` has a C function table, not an overridable Tcl command namespace.
    pub(crate) fn invoke_fixed_math_builtin(
        &mut self,
        spec: &'static tcl_registry::CommandSpec,
        args: &[Value],
    ) -> Completion<Value> {
        let key = spec.name.trim_start_matches("::");
        let Some(handler) = self.fixed_math_builtins.get(key).copied() else {
            return err(format!("unknown math function \"{}\"", spec.name));
        };
        self.set_invoked_name(key);
        handler(self, args)
    }

    /// Tcl 8.x resolves an unqualified variable at **namespace scope** to the
    /// global variable when the namespace has none but the global namespace
    /// does — reads *and* writes reach the global; 9.0 removed the fallback
    /// (TIP 278, `TCL_NAMESPACE_ONLY` — 8.6 `tclVar.c:757` vs 9.0 `:935`).
    /// tclsh 8.6/9.0-pinned in `cross_version_vars_e2e.rs`.
    fn ns_var_global_fallback(&self) -> bool {
        self.runtime_version.namespace_var_global_fallback()
    }

    pub(crate) fn register_command(&mut self, name: &str, cmd: Command) {
        // The table is keyed by canonical *unrooted* keys — callers pass
        // `qualify_name` output (or an unrooted literal) verbatim.  No root
        // stripping happens here: an unrooted key can itself begin with `::`
        // when a namespace is legitimately named `:` (#934), so a strip at
        // this boundary would corrupt it, and a rooted-form caller cannot be
        // told apart from that key textually.
        //
        // Overwriting an existing command deletes it (C `Tcl_CreateObjCommand`
        // does the same), so its `delete` command traces fire and every trace
        // on it is dropped — tclsh-pinned: redefining a traced proc fires the
        // delete trace.  This also detaches any in-flight sidecar handle, so
        // overwriting a binding cannot let it follow the replacement.
        if self.commands.contains_key(name) {
            let was_coroutine = crate::cmd_coro::is_coroutine(self, name);
            self.detach_active_sidecars(&CommandSidecarKey::visible(name));
            if was_coroutine {
                crate::cmd_coro::on_command_deleted(self, name);
            }
            self.on_command_removed(name);
        }
        // Overwriting a cross-interp alias drops its target-death backref (the
        // alias-create path re-adds one for the new alias afterwards). Gated
        // on the backref table so the builtin sweep and ordinary command
        // registration pay nothing.
        if !self.alias_backrefs.is_empty() {
            self.drop_alias_backref(name);
        }
        self.bump_cmd_epoch();
        self.imported_commands.remove(name);
        self.builtin_identities.remove(name);
        // The TclOO root marking is an identity, not a reservation on the
        // *name*: it says "the engine installed this entry on the registry's
        // behalf, so date it by the registry". Overwriting the entry replaces
        // that identity with a script-created one, which is release-invariant
        // like any proc — so the marking must not outlive the entry it
        // described. Leaving it behind makes `oo::class create ::oo::
        // configurable {…}` at 8.6 (where no such builtin exists, so the name
        // is the user's to take) create an object the availability gate then
        // hides forever. The engine's own installs re-declare the root
        // immediately after registering, so clearing here cannot unmark them.
        self.registry_object_roots.remove(name);
        self.commands.insert(name.to_owned(), cmd);
    }

    /// Every registered command name, in table order.
    pub(crate) fn registered_command_names(&self) -> Vec<String> {
        self.commands.keys().cloned().collect()
    }

    /// Remove `name` from the command table by exact key — no namespace-path
    /// resolution, which is what a whitelist sweep over
    /// [`Self::registered_command_names`] needs.
    pub(crate) fn remove_registered_command(&mut self, name: &str) {
        if self.commands.remove(name).is_some() {
            let was_coroutine = crate::cmd_coro::is_coroutine(self, name);
            self.detach_active_sidecars(&CommandSidecarKey::visible(name));
            if was_coroutine {
                crate::cmd_coro::on_command_deleted(self, name);
            }
            self.bump_cmd_epoch();
            self.imported_commands.remove(name);
            self.builtin_identities.remove(name);
            self.registry_object_roots.remove(name);
        }
    }

    /// Resolve and remove the command `name`, returning it (for `rename`).
    /// Resolution is the full shared rule — current namespace, `namespace
    /// path`, then global — matching C's `TclRenameCommand`, which finds the
    /// source with `Tcl_FindCommand` (tclsh-pinned: `rename viap viap2`
    /// inside a namespace whose path reaches `::pr::viap` moves that
    /// command).
    pub(crate) fn take_command(&mut self, name: &str) -> Option<Command> {
        let key = self.resolve_command_fqn(self.current_ns(), name)?;
        // A command that is present in the engine table may still be absent
        // from the emulated release's surface.  Treat it exactly like a
        // missing command here: otherwise `rename` (and `interp hide`, which
        // uses the same removal seam) can move a hidden builtin to a registry-
        // unknown name and make it callable again (#1463).
        if self
            .commands
            .get(&key)
            .is_some_and(|command| !self.builtin_command_visible_for_surface(&key, command))
        {
            return None;
        }
        self.take_command_unchecked_key(&key)
    }

    /// Prepare a rename without touching the command table.  Alias-loop
    /// validation runs against the returned logical command, so a rejected
    /// rename cannot fire callbacks or disturb traces, guards, or caches.
    pub(crate) fn prepare_command_rename(
        &self,
        name: &str,
    ) -> Option<(Command, CommandRenameTransaction)> {
        let old_key = self.resolve_command_fqn(self.current_ns(), name)?;
        let command = self.commands.get(&old_key)?;
        if !self.builtin_command_visible_for_surface(&old_key, command) {
            return None;
        }
        let source_import_origin = self.imported_commands.get(&old_key).cloned();
        let source_builtin_identity = self.builtin_identity_for_key(&old_key);
        let source_registry_object_root = self.registry_object_roots.get(&old_key).cloned();
        let cross_alias_target = match command {
            Command::CrossAlias { target, .. } => Some(*target),
            _ => None,
        };
        Some((
            (*command).clone(),
            CommandRenameTransaction {
                old_key,
                new_key: String::new(),
                source_import_origin,
                source_builtin_identity,
                source_registry_object_root,
                cross_alias_target,
            },
        ))
    }

    /// Install the new half of a rename and record every changed semantic map.
    pub(crate) fn install_renamed_command(
        &mut self,
        transaction: &mut CommandRenameTransaction,
        new_key: &str,
        command: Command,
    ) {
        new_key.clone_into(&mut transaction.new_key);
        // Keep the successful rename's established source-removal then
        // destination-registration order.  The caller has already completed
        // every rejection path, so this is now allowed to fire destination
        // delete traces and invalidate command/trace guard state.
        self.take_command_unchecked_key(&transaction.old_key)
            .expect("the prepared rename source remains registered");
        self.register_command(new_key, command);
        if let Some(origin) = &transaction.source_import_origin {
            self.restore_import_origin(new_key, origin.clone());
        }
        if let Some(identity) = &transaction.source_builtin_identity {
            self.restore_builtin_identity(new_key, identity.clone());
        }
        // The `TclOO` root marking is the same kind of fact as the builtin
        // identity above, so it travels the same way. Without this, `rename
        // oo::configurable myconf` produced a command the availability gate
        // read as script-created, so it survived a later switch to an 8.6
        // surface that has no `oo::configurable` at all.
        if let Some(identity) = &transaction.source_registry_object_root {
            self.declare_registry_object_root_as(new_key, &identity.clone());
        }
        self.retarget_imports(&transaction.old_key, new_key);
    }

    /// Remove the source for an already validated `rename old {}`.  Deletion
    /// deliberately does not retarget imports; C leaves them dangling.
    pub(crate) fn delete_prepared_renamed_command(
        &mut self,
        transaction: &CommandRenameTransaction,
    ) {
        self.take_command_unchecked_key(&transaction.old_key)
            .expect("the prepared delete source remains registered");
        self.detach_active_sidecars(&CommandSidecarKey::visible(&transaction.old_key));
    }

    /// Commit a rename after all semantic validation has succeeded.
    pub(crate) fn commit_renamed_command(&mut self, transaction: &CommandRenameTransaction) {
        if let Some(target) = transaction.cross_alias_target {
            self.retarget_alias_backref(&transaction.old_key, &transaction.new_key, target);
        }
    }

    /// Resolve and remove a command without consulting the emulated command
    /// surface.  This is for VM-owned teardown only (temporary `apply`
    /// procedures, coroutine state, and `TclOO` objects); those paths are
    /// deleting an implementation that is already unreachable, not exposing
    /// a command to Tcl code.  User-facing removal must use [`Self::take_command`].
    pub(crate) fn take_command_unchecked(&mut self, name: &str) -> Option<Command> {
        let key = self.resolve_command_fqn_raw(self.current_ns(), name)?;
        let command = self.take_command_unchecked_key(&key)?;
        let was_coroutine = crate::cmd_coro::is_coroutine(self, &key);
        self.detach_active_sidecars(&CommandSidecarKey::visible(&key));
        if was_coroutine {
            crate::cmd_coro::on_command_deleted(self, &key);
        }
        Some(command)
    }

    /// Retire a VM-owned command whether its lifecycle key is currently
    /// visible or hidden. Completion is a real command deletion, so delete
    /// traces fire exactly once before all trace/deopt sidecars are dropped.
    pub(crate) fn retire_command_lifecycle_key(
        &mut self,
        key: &CommandSidecarKey,
    ) -> Option<Command> {
        let command = match key {
            CommandSidecarKey::Visible(name) => self.take_command_unchecked_key(name)?,
            CommandSidecarKey::Hidden(name) => {
                let command = self.hidden_commands.remove(name)?;
                self.bump_cmd_epoch();
                self.hidden_imported_commands.remove(name);
                self.hidden_builtin_identities.remove(name);
                command
            }
        };
        self.drop_alias_backref_key(key);
        self.on_command_removed_for(key);
        Some(command)
    }

    /// Remove a command by its already-resolved table key.
    fn take_command_unchecked_key(&mut self, key: &str) -> Option<Command> {
        self.bump_cmd_epoch();
        self.imported_commands.remove(key);
        self.builtin_identities.remove(key);
        // Clear the `TclOO` root marking with the entry it describes, exactly
        // as the builtin identity above is cleared. A rename re-declares it at
        // the destination; leaving it stranded under the vacated name would
        // gate an unrelated command that later takes that name.
        self.registry_object_roots.remove(key);
        self.commands.remove(key)
    }

    /// Remove a command by its exact table key (no name resolution) — the
    /// rollback path for a refused alias creation.
    pub(crate) fn remove_command_exact(&mut self, key: &str) -> Option<Command> {
        let command = self.take_command_unchecked_key(key)?;
        let was_coroutine = crate::cmd_coro::is_coroutine(self, key);
        self.detach_active_sidecars(&CommandSidecarKey::visible(key));
        if was_coroutine {
            crate::cmd_coro::on_command_deleted(self, key);
        }
        Some(command)
    }

    /// `interp alias srcPath srcCmd targetPath target…` — route the alias by
    /// its two `interp` paths (`{}` = this interp, else a path reachable from
    /// it), install it in the SOURCE interp's table, and refuse a loop the way
    /// C's `TclPreventAliasLoop` does: create first, walk the chain, roll back
    /// on a hit (tclsh-pinned: a failed self-alias also destroys the proc it
    /// clobbered — C does not restore it).  Source and target may be any two
    /// interpreters in the tree (child→parent, parent→child, and sibling↔
    /// sibling all route through the shared engine — C supports every pairing).
    pub(crate) fn interp_alias_create(
        &mut self,
        src_path: &str,
        src_cmd: &str,
        target_path: &str,
        target_words: Vec<Value>,
    ) -> Completion<Value> {
        let src = match self.resolve_interp_path(src_path) {
            Ok(s) => s,
            Err(c) => return c,
        };
        let target = match self.resolve_interp_path(target_path) {
            Ok(t) => t,
            Err(c) => return c,
        };
        let words = Rc::new(target_words);
        let cmd = if src == target {
            Command::Alias(Rc::clone(&words))
        } else {
            Command::CrossAlias {
                target,
                words: Rc::clone(&words),
            }
        };
        // The written source name qualifies in the SOURCE interp to the
        // canonical unrooted key (`interp alias {} ::a::f {} g` creates
        // `a::f`; a raw registration would corrupt colon names, #934).
        let src_key = self.in_interp(src, |vm| {
            let src_key = vm.qualify_name(src_cmd);
            vm.register_command(&src_key, cmd);
            if src != target {
                vm.note_alias_backref(&src_key, target);
            }
            src_key
        });
        if self.alias_chain_loops(src, &src_key) {
            self.in_interp(src, |vm| {
                vm.remove_command_exact(&src_key);
                vm.drop_alias_backref(&src_key);
            });
            let tail = key_holder_and_tail_unrooted(&src_key).1;
            return err(format!(
                "cannot define or rename alias \"{tail}\": would create a loop"
            ));
        }
        ok(Value::string(src_cmd))
    }

    /// C's `TclPreventAliasLoop` (8.6 `tclInterp.c`), on the just-installed
    /// alias at (`defining_interp`, `defining_key`): follow the chain — each
    /// hop resolves the alias's target name from the TARGET interp's
    /// **global** namespace at define time; an unresolved target ends the
    /// chain (legal — aliases late-bind), a non-alias ends it, and a hop
    /// landing back on the defining command is a loop.  Cross-interp hops
    /// address the target by its stable [`InterpId`], so the walk follows the
    /// alias graph across the whole tree.  Terminates because every *existing*
    /// alias already passed this check (no pre-existing loops to spin in).
    fn alias_chain_loops_from(
        &self,
        defining_interp: InterpId,
        defining_key: &str,
        first: &Command,
    ) -> bool {
        let mut command = first;
        let mut interp = defining_interp;
        loop {
            let (target_interp, target_words) = match command {
                Command::Alias(words) => (interp, Rc::clone(words)),
                Command::CrossAlias { target, words } => (*target, Rc::clone(words)),
                _ => return false,
            };
            let Some(target_name) = target_words.first().map(|v| v.to_str().to_string()) else {
                return false;
            };
            let Some(target_st) = self.st_of(target_interp) else {
                return false;
            };
            // The just-renamed alias is not installed yet.  Let its candidate
            // key shadow an existing hidden builtin (or an absent name) for
            // this logical validation only; mutating the command table first
            // would fire irreversible delete traces on that builtin.
            let next = if target_interp == defining_interp
                && canonical_cmd_key(&target_name) == defining_key
            {
                defining_key.to_owned()
            } else if let Some(next) = target_st.resolve_command_fqn("", &target_name) {
                next
            } else {
                return false;
            };
            if target_interp == defining_interp && next == defining_key {
                return true;
            }
            let Some(next_st) = self.st_of(target_interp) else {
                return false;
            };
            let Some(next_command) = next_st.commands.get(&next) else {
                return false;
            };
            interp = target_interp;
            command = next_command;
        }
    }

    fn alias_chain_loops(&self, defining_interp: InterpId, defining_key: &str) -> bool {
        let Some(st) = self.st_of(defining_interp) else {
            return false;
        };
        let Some(first) = st.commands.get(defining_key) else {
            return false;
        };
        self.alias_chain_loops_from(defining_interp, defining_key, first)
    }

    /// Check whether a not-yet-installed same-interpreter alias would loop if
    /// it were renamed to `key`.
    pub(crate) fn alias_chain_loops_for_rename(&self, key: &str, alias: &Command) -> bool {
        self.alias_chain_loops_from(self.cur, key, alias)
    }

    /// The names `info functions` exposes on the selected Tcl release.
    ///
    /// Tcl 8.4 introspects its closed fixed table, while Tcl 8.5 and later
    /// enumerate the open command table (which also admits user functions).
    /// Both branches are selected through the registry surface.
    pub(crate) fn math_function_names(&self) -> Vec<String> {
        let surface =
            tcl_registry::expr_surface::RuntimeExprSurface::for_tcl_version(self.runtime_version);
        if !surface.has_math_function_command_table() {
            return surface
                .builtin_math_function_names()
                .into_iter()
                .map(str::to_owned)
                .collect();
        }
        let mut names: Vec<String> = self
            .commands
            .iter()
            .filter(|(name, command)| self.builtin_command_visible_for_surface(name, command))
            .filter_map(|(name, _)| {
                tcl_registry::mathfunc::global_command_bare_name(name).map(str::to_owned)
            })
            .collect();
        names.sort_unstable();
        names
    }

    /// The `info cmdtype` kind of `name` (`native`/`proc`/`alias`), or `None`
    /// when there is no such command.
    pub(crate) fn command_kind(&self, name: &str) -> Option<&'static str> {
        self.lookup_command(name).map(|c| match c {
            Command::Builtin(_) | Command::Native(_) => "native",
            Command::Proc(_) => "proc",
            // Cross-interp aliases are aliases (C's `info cmdtype` says
            // `alias` for every `interp alias` product).
            Command::Alias(_) | Command::CrossAlias { .. } => "alias",
            Command::ChildInterp(_) => "interp",
            Command::Ensemble(_) => "ensemble",
            Command::Object(_) => "object",
        })
    }

    /// A fresh child interpreter sharing this one's output sink, compile
    /// service, and host — its command table, namespaces, variables, and
    /// channels are otherwise independent (`interp create`).
    /// A fresh child interpreter's *bare* state (no builtins yet): its own
    /// command table, namespaces, variables, and channels, but the parent's
    /// output sink, compile service, host, and runtime version (`interp
    /// create` inherits these). Populated by [`Self::create_child`] once it is
    /// in the arena and can be entered.
    fn fork_child_state(&self) -> Box<InterpState> {
        let mut child = InterpState::fresh(Rc::clone(&self.out));
        child.compiler.clone_from(&self.compiler);
        child.host = Rc::clone(&self.host);
        child.runtime_version = self.runtime_version;
        child.dialect_profile = self.dialect_profile;
        child.command_surface_profile = self.command_surface_profile;
        child.profile_registry = self.profile_registry;
        Box::new(child)
    }

    /// `interp create ?-safe? ?name?` — make a child interpreter in the arena,
    /// registering it as a command in the current interp. Returns the (possibly
    /// auto-generated) name.
    pub(crate) fn create_child(&mut self, name: Option<String>, safe: bool) -> String {
        let name = name.unwrap_or_else(|| {
            let n = format!("interp{}", self.interp_counter);
            self.interp_counter += 1;
            n
        });
        let parent = self.cur;
        let child = self.fork_child_state();
        let id = self.new_interp_slot(child, parent);
        // Populate the child by making it current and running the ordinary
        // interpreter bootstrap (register builtins, seed globals, optionally
        // make it safe) — the same paths the root interp uses.
        self.in_interp(id, |vm| {
            register_builtins(vm);
            vm.bootstrap_globals();
            if safe {
                vm.make_safe();
            }
        });
        // The interp *path* name keys `children` as written (interp names are
        // their own universe, never namespace-parsed); the command it creates
        // in this interp is an ordinary command name, so its key qualifies.
        let cmd_key = self.qualify_name(&name);
        self.children.insert(name.clone(), id);
        self.register_command(&cmd_key, Command::ChildInterp(id));
        name
    }

    /// The [`InterpId`] of a direct child by name, or `None`.
    pub(crate) fn child_id(&self, name: &str) -> Option<InterpId> {
        self.children.get(name).copied()
    }

    /// Select the dialect profile of a direct child interpreter. This is the
    /// host-side counterpart to configuring a standalone [`Vm`]: child state
    /// is parked in the interpreter arena, so route through `in_interp` rather
    /// than mutating a copied state and thereby skipping its bytecode-cache
    /// invalidation owner.
    pub fn set_child_dialect_profile(
        &mut self,
        name: &str,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> bool {
        let Some(id) = self.child_id(name) else {
            return false;
        };
        self.in_interp(id, |child| child.set_dialect_profile(profile));
        true
    }

    /// Whether a child interpreter `name` exists (and is not being torn down).
    pub(crate) fn child_exists(&self, name: &str) -> bool {
        self.children
            .get(name)
            .is_some_and(|&id| self.interp_alive(id))
    }

    /// `interp debug ?-frame ?bool??` on this interp. `-frame` is a one-way
    /// switch (once on, stays on); returns the settings list / the frame bool.
    pub(crate) fn debug_apply(&mut self, args: &[Value]) -> Result<Value, String> {
        match args {
            [] => Ok(Value::list(vec![
                Value::string("-frame"),
                Value::int(i64::from(self.debug_frame)),
            ])),
            [opt] if &*opt.to_str() == "-frame" => Ok(Value::int(i64::from(self.debug_frame))),
            [opt, val] if &*opt.to_str() == "-frame" => {
                if val.as_bool().unwrap_or(false) {
                    self.debug_frame = true;
                }
                Ok(Value::int(i64::from(self.debug_frame)))
            }
            _ => Err(format!(
                "bad option \"{}\": must be -frame",
                args.first()
                    .map(|v| v.to_str().to_string())
                    .unwrap_or_default()
            )),
        }
    }

    /// `interp bgerror ?cmdPrefix?` on this interp — get/set the background-error
    /// handler.
    pub(crate) fn bgerror_apply(&mut self, args: &[Value]) -> Value {
        if let [prefix] = args {
            self.bgerror_handler = prefix.clone();
        }
        self.bgerror_handler.clone()
    }

    /// The effective background-error handler command prefix for the event loop,
    /// or `""` when none is callable (so a handler error is not routed through
    /// the `unknown` fallback). Used by [`crate::cmd_event`].
    ///
    /// Prefers the configured `interp bgerror` prefix when its head command
    /// exists; otherwise falls back to a user-defined `bgerror` proc — C's
    /// default handler `::tcl::Bgerror` (an init.tcl library proc the VM does not
    /// load) merely formats the error and dispatches to `bgerror`.
    pub(crate) fn bgerror_handler_prefix(&self) -> String {
        let prefix = self.bgerror_handler.to_str();
        let head = tcl_syntax::list::split_list_lenient(&prefix)
            .into_iter()
            .next()
            .unwrap_or_default()
            .to_string();
        if !head.is_empty() && self.lookup_command(&head).is_some() {
            return prefix.to_string();
        }
        if self.lookup_command("bgerror").is_some() {
            return "bgerror".to_string();
        }
        String::new()
    }

    /// Whether this interp's `time` limit has elapsed. The stored time value is
    /// an absolute wall-clock deadline (`-seconds`/`-milliseconds`); the
    /// execution trampoline polls this so an unbounded bytecode loop still
    /// honours `interp limit $i time`. Returns `false` when no time limit is set.
    pub(crate) fn time_limit_exceeded(&self) -> bool {
        match self.limits.time_value {
            Some((secs, millis)) => {
                let deadline = i128::from(secs) * 1000 + i128::from(millis);
                self.host_rc().clock().now_millis() >= deadline
            }
            None => false,
        }
    }

    /// Whether a `time` limit is configured at all — the cheap guard the
    /// trampoline checks before paying for a wall-clock read.
    pub(crate) fn has_time_limit(&self) -> bool {
        self.limits.time_value.is_some()
    }

    /// Advance the limit-poll counter and, when a `time` limit is armed and the
    /// throttle window elapses, return the `time limit exceeded` error. Called
    /// from the bytecode trampoline (per tick) and the loop commands (per
    /// iteration) so both pure-bytecode and command-driven infinite loops are
    /// trapped. A no-op (beyond the guard) when no time limit is set.
    pub(crate) fn limit_check_tick(&mut self) -> Option<Completion<Value>> {
        if !self.has_time_limit() {
            return None;
        }
        self.limit_tick = self.limit_tick.wrapping_add(1);
        // Poll roughly every 4096 ticks (the low 12 bits clear).
        if self.limit_tick.trailing_zeros() >= 12 && self.time_limit_exceeded() {
            return Some(err("time limit exceeded"));
        }
        None
    }

    /// Set (or clear) the absolute wall-clock deadline the `time` limit polls
    /// against, in host-clock milliseconds. The embedder-facing wrapper is
    /// [`Vm::set_wall_clock_budget`].
    pub(crate) fn set_time_limit_deadline(&mut self, deadline_millis: Option<i128>) {
        self.limits.time_value = deadline_millis.map(|deadline| {
            let seconds = i64::try_from(deadline / 1000).unwrap_or(i64::MAX);
            let millis = i64::try_from(deadline % 1000).unwrap_or(0);
            (seconds, millis)
        });
    }

    /// Charge one command against this interp's `commands` limit, returning
    /// the `command count limit exceeded` completion when the budget is spent
    /// (issue #1373 finding 1: the limit was stored but never enforced).
    ///
    /// Called from both command funnels — the trampoline's
    /// [`dispatch_words`](crate::exec) and the native
    /// [`invoke_command`](Vm::invoke_command) re-entry — so a body that loops
    /// in bytecode and one that loops through native re-entry are charged
    /// alike. Unlike C Tcl this checks every command rather than every
    /// `-granularity`th: the check is an increment and a compare, and an
    /// exactly-enforced budget is what the `SpecTcl` containment guarantee needs
    /// from a hook body.
    pub(crate) fn charge_command(&mut self) -> Option<Completion<Value>> {
        self.commands_run = self.commands_run.saturating_add(1);
        let limit = self.limits.cmd_value?;
        let limit = u64::try_from(limit).unwrap_or(0);
        (self.commands_run > limit).then(|| err("command count limit exceeded"))
    }

    /// Refuse a single allocation of `bytes` when it would exceed the armed
    /// value-size limit, returning the `value size limit exceeded` completion.
    ///
    /// The `commands` and `time` limits cannot bound this. `string repeat`
    /// builds its result in one opcode: it dispatches one command and returns
    /// in microseconds, so both budgets are still nearly full while the
    /// process has already asked the allocator for gigabytes. Left unbounded
    /// that is an OOM abort — the one failure the sandbox cannot contain and
    /// report, because it takes the whole server with it rather than the hook.
    ///
    /// Checked *before* allocating, so the refusal costs nothing and the
    /// memory is never requested.
    pub(crate) fn charge_allocation(&self, bytes: u64) -> Option<Completion<Value>> {
        let limit = self.limits.value_bytes?;
        (bytes > limit).then(|| err("value size limit exceeded"))
    }

    /// The value-size limit, if one is armed.
    pub(crate) fn value_size_limit_value(&self) -> Option<u64> {
        self.limits.value_bytes
    }

    /// Arm (or disarm) the value-size limit.
    pub(crate) fn set_value_size_limit_value(&mut self, limit: Option<u64>) {
        self.limits.value_bytes = limit;
    }

    /// The `commands` limit value, if one is armed.
    pub(crate) fn command_limit_value(&self) -> Option<i64> {
        self.limits.cmd_value
    }

    /// Arm (or disarm) the `commands` limit.
    pub(crate) fn set_command_limit_value(&mut self, limit: Option<i64>) {
        self.limits.cmd_value = limit;
    }

    /// Commands dispatched since the counter was last reset.
    pub(crate) fn command_count(&self) -> u64 {
        self.commands_run
    }

    /// Zero the command counter — an embedder's per-invocation fuel refill.
    pub(crate) fn reset_command_count_inner(&mut self) {
        self.commands_run = 0;
    }

    /// `interp limit limitType ?-option value …?` on this interp (query / set;
    /// enforced for `time`).
    pub(crate) fn limit_apply(&mut self, ltype: &str, args: &[Value]) -> Result<Value, String> {
        match ltype {
            "commands" => self.limit_commands(args),
            "time" => self.limit_time(args),
            other => Err(format!(
                "bad limit type \"{other}\": must be commands or time"
            )),
        }
    }

    fn limit_commands(&mut self, args: &[Value]) -> Result<Value, String> {
        const OPTS: &[&str] = &["-command", "-granularity", "-value"];
        let l = &self.limits;
        let query = |opt: &str| match opt {
            "-command" => l.cmd_command.clone(),
            "-granularity" => Value::int(l.cmd_granularity),
            _ => l.cmd_value.map_or_else(|| Value::string(""), Value::int),
        };
        match args {
            [] => Ok(Value::list(vec![
                Value::string("-command"),
                l.cmd_command.clone(),
                Value::string("-granularity"),
                Value::int(l.cmd_granularity),
                Value::string("-value"),
                l.cmd_value.map_or_else(|| Value::string(""), Value::int),
            ])),
            [opt] => Ok(query(resolve_limit_opt(&opt.to_str(), OPTS)?)),
            _ => {
                for pair in args.chunks(2) {
                    // A trailing option with no value is a catchable error, not a
                    // panic (`interp limit c commands -value 1 -granularity`).
                    let [opt, val] = pair else {
                        return Err("wrong # args: should be \"interp limit path commands \
                                    ?-option value ...?\""
                            .into());
                    };
                    let opt = resolve_limit_opt(&opt.to_str(), OPTS)?;
                    match opt {
                        "-command" => self.limits.cmd_command = val.clone(),
                        "-granularity" => {
                            let n = parse_limit_int(&val.to_str())?;
                            if n < 1 {
                                return Err("granularity must be at least 1".into());
                            }
                            self.limits.cmd_granularity = n;
                        }
                        _ => {
                            let n = parse_limit_int(&val.to_str())?;
                            if n < 0 {
                                return Err("command limit value must be at least 0".into());
                            }
                            self.limits.cmd_value = Some(n);
                        }
                    }
                }
                Ok(Value::string(""))
            }
        }
    }

    fn limit_time(&mut self, args: &[Value]) -> Result<Value, String> {
        const OPTS: &[&str] = &["-command", "-granularity", "-milliseconds", "-seconds"];
        let l = &self.limits;
        let query = |opt: &str| match opt {
            "-command" => l.time_command.clone(),
            "-granularity" => Value::int(l.time_granularity),
            "-seconds" => l
                .time_value
                .map_or_else(|| Value::string(""), |(s, _)| Value::int(s)),
            _ => l
                .time_value
                .map_or_else(|| Value::string(""), |(_, m)| Value::int(m)),
        };
        match args {
            [] => Ok(Value::list(vec![
                Value::string("-command"),
                l.time_command.clone(),
                Value::string("-granularity"),
                Value::int(l.time_granularity),
                Value::string("-milliseconds"),
                l.time_value
                    .map_or_else(|| Value::string(""), |(_, m)| Value::int(m)),
                Value::string("-seconds"),
                l.time_value
                    .map_or_else(|| Value::string(""), |(s, _)| Value::int(s)),
            ])),
            [opt] => Ok(query(resolve_limit_opt(&opt.to_str(), OPTS)?)),
            _ => {
                let (mut sec, mut ms) = self.limits.time_value.unwrap_or((0, 0));
                let mut touched = self.limits.time_value.is_some();
                for pair in args.chunks(2) {
                    let [opt, val] = pair else {
                        return Err("wrong # args: should be \"interp limit path time \
                                    ?-option value ...?\""
                            .into());
                    };
                    let opt = resolve_limit_opt(&opt.to_str(), OPTS)?;
                    match opt {
                        "-command" => self.limits.time_command = val.clone(),
                        "-granularity" => {
                            let n = parse_limit_int(&val.to_str())?;
                            if n < 1 {
                                return Err("granularity must be at least 1".into());
                            }
                            self.limits.time_granularity = n;
                        }
                        "-seconds" => {
                            let n = parse_limit_int(&val.to_str())?;
                            if n < 0 {
                                return Err("seconds must be non-negative".into());
                            }
                            sec = n;
                            touched = true;
                        }
                        _ => {
                            let n = parse_limit_int(&val.to_str())?;
                            if n < 0 {
                                return Err("milliseconds must be non-negative".into());
                            }
                            ms = n;
                            touched = true;
                        }
                    }
                }
                if touched {
                    // Normalise excess milliseconds into seconds.
                    sec += ms.div_euclid(1000);
                    ms = ms.rem_euclid(1000);
                    self.limits.time_value = Some((sec, ms));
                }
                Ok(Value::string(""))
            }
        }
    }

    /// Sorted names of this interp's direct children (`interp children`).
    pub(crate) fn child_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.children.keys().cloned().collect();
        names.sort();
        names
    }

    /// `interp children path` — the direct child names of the named child interp
    /// (one level down), or `None` when that child does not exist.
    pub(crate) fn child_child_names(&self, name: &str) -> Option<Vec<String>> {
        let id = self.child_id(name)?;
        let mut names: Vec<String> = self.st_of(id)?.children.keys().cloned().collect();
        names.sort();
        Some(names)
    }

    /// `interp issafe ?path?` for the current interp.
    pub(crate) fn is_safe(&self) -> bool {
        self.state.is_safe
    }

    /// Whether the named child is safe (`$child issafe` / `interp issafe path`).
    pub(crate) fn child_is_safe(&self, name: &str) -> Option<bool> {
        let id = self.child_id(name)?;
        self.st_of(id).map(|st| st.is_safe)
    }

    /// Get/set a child's recursion limit; `None` if the child does not exist.
    pub(crate) fn child_recursion_limit_apply(
        &mut self,
        name: &str,
        newlimit: Option<&str>,
    ) -> Option<Result<i64, String>> {
        let id = self.child_id(name)?;
        Some(self.in_interp(id, |vm| vm.recursion_limit_apply(newlimit)))
    }

    /// Sorted hidden-command names of a child (`interp hidden path`).
    pub(crate) fn child_hidden_names(&self, name: &str) -> Option<Vec<String>> {
        let id = self.child_id(name)?;
        let mut names: Vec<String> = self.st_of(id)?.hidden_commands.keys().cloned().collect();
        names.sort();
        Some(names)
    }

    /// `interp invokehidden path cmd ?arg ...?` — invoke a hidden command inside
    /// the child. The command is temporarily restored to the child's visible
    /// table for the call (then the previous binding is put back), so it runs in
    /// the child without permanently un-hiding it. `None` if the child is gone.
    pub(crate) fn invoke_hidden_in_child(
        &mut self,
        name: &str,
        cmd: &str,
        args: &[Value],
    ) -> Option<Completion<Value>> {
        if name.is_empty() {
            return Some(self.invoke_own_hidden(cmd, args));
        }
        let id = self.child_id(name).filter(|&id| self.interp_alive(id))?;
        Some(self.in_interp(id, |vm| vm.invoke_own_hidden(cmd, args)))
    }

    /// Invoke one of this interpreter's hidden commands without making its
    /// temporary visibility observable after dispatch.  The command's stable
    /// identity and import provenance must accompany the transient visible
    /// entry: command-surface checks and `namespace origin` consult those
    /// tables while a command is running.
    fn invoke_own_hidden(&mut self, cmd: &str, args: &[Value]) -> Completion<Value> {
        let Some(hidden) = self.hidden_commands.get(cmd).cloned() else {
            return err(format!("invalid hidden command name \"{cmd}\""));
        };
        let identity = self.hidden_builtin_identities.get(cmd).map(String::as_str);
        if !self.builtin_command_visible_for_identity(cmd, &hidden, identity) {
            return err(format!("invalid command name \"{cmd}\""));
        }
        self.invoke_resolved_command(cmd, CommandSidecarKey::hidden(cmd), hidden, args)
    }

    pub(crate) fn take_invoked_sidecar(&mut self) -> Option<CommandSidecarKey> {
        self.invoked_sidecar.take()
    }

    /// `interp marktrusted path` — clear a child's safe flag (exposing every
    /// hidden command). A no-op if the child does not exist.
    pub(crate) fn child_mark_trusted(&mut self, name: &str) {
        let Some(id) = self.child_id(name) else {
            return;
        };
        self.mark_interp_trusted(id);
    }

    fn mark_interp_trusted(&mut self, id: InterpId) {
        let names: Vec<String> = self
            .st_of(id)
            .map(|state| state.hidden_commands.keys().cloned().collect())
            .unwrap_or_default();
        self.in_interp(id, |vm| {
            vm.bump_cmd_epoch();
            for name in names {
                if !vm.commands.contains_key(&name) {
                    vm.expose_own_command(&name, &name)
                        .expect("a non-colliding hidden command exposes");
                }
            }
            vm.is_safe = false;
        });
    }

    /// Stable registry identity for a visible builtin, if any.  A direct
    /// registered builtin derives it from its key; a moved builtin carries it
    /// in [`Self::builtin_identities`].
    fn builtin_identity_for_key(&self, key: &str) -> Option<String> {
        matches!(
            self.commands.get(key),
            Some(Command::Builtin(_) | Command::Native(_))
        )
        .then(|| {
            self.builtin_identities
                .get(key)
                .cloned()
                .unwrap_or_else(|| self.command_origin_key(key))
        })
    }

    /// `interp hide {} cmd` — move one visible command plus all of its
    /// metadata into the hidden table.  This is the only public hide seam;
    /// every caller therefore observes the same release-visibility check.
    pub(crate) fn hide_command(&mut self, cmd: &str, token: &str) -> Result<(), String> {
        if self.hidden_commands.contains_key(token) {
            return Err(format!("hidden command named \"{token}\" already exists"));
        }
        let Some(source) = self.resolve_command_fqn(self.current_ns(), cmd) else {
            return Ok(());
        };
        let import_origin = self.imported_commands.get(&source).cloned();
        let builtin_identity = self.builtin_identity_for_key(&source);
        let Some(command) = self.take_command(cmd) else {
            return Ok(());
        };
        let cross_target = match &command {
            Command::CrossAlias { target, .. } => Some(*target),
            _ => None,
        };
        self.hidden_commands.insert(token.to_owned(), command);
        crate::cmd_coro::on_command_hidden(self, &source, token);
        self.move_command_traces(
            &CommandSidecarKey::visible(&source),
            CommandSidecarKey::hidden(token),
        );
        if let Some(origin) = import_origin {
            self.hidden_imported_commands
                .insert(token.to_owned(), origin);
        }
        if let Some(identity) = builtin_identity {
            self.hidden_builtin_identities
                .insert(token.to_owned(), identity);
        }
        // Hiding does not change the Tcl-visible origin. Retarget the internal
        // reference into the hidden domain so the lineage remains traversable
        // without colliding with an equal visible token.
        self.retarget_imports_key(
            &CommandSidecarKey::visible(&source),
            &CommandSidecarKey::hidden(token),
        );
        if let Some(target) = cross_target {
            self.retarget_alias_backref_key(
                &CommandSidecarKey::visible(&source),
                CommandSidecarKey::hidden(token),
                target,
            );
        }
        Ok(())
    }

    /// `interp expose {} hidden ?token?` — restore one of *this* interp's
    /// hidden commands, optionally under a new name (`token`).
    pub(crate) fn expose_own_command(&mut self, cmd: &str, token: &str) -> Result<(), String> {
        // `interp expose` always installs the destination in the global
        // namespace.  A contextual lookup here would incorrectly see (and
        // reject because of) a same-spelled local command in the caller's
        // namespace, instead of checking the exact global table owner.
        let destination = canonical_cmd_key(token);
        if self.visible_command_exists_exact(&destination) {
            return Err(format!("exposed command \"{token}\" already exists"));
        }
        if let Some(c) = self.hidden_commands.remove(cmd) {
            let cross_target = match &c {
                Command::CrossAlias { target, .. } => Some(*target),
                _ => None,
            };
            self.register_command(&destination, c);
            crate::cmd_coro::on_command_exposed(self, cmd, destination.as_ref());
            self.move_command_traces(
                &CommandSidecarKey::hidden(cmd),
                CommandSidecarKey::visible(destination.as_ref()),
            );
            if let Some(origin) = self.hidden_imported_commands.remove(cmd) {
                self.imported_commands
                    .insert(destination.to_string(), origin);
            }
            if let Some(identity) = self.hidden_builtin_identities.remove(cmd) {
                self.builtin_identities
                    .insert(destination.to_string(), identity);
            }
            // Reconnect internal references to the visible domain; their
            // Tcl-visible ultimate source lineage remains unchanged.
            self.retarget_imports_key(
                &CommandSidecarKey::hidden(cmd),
                &CommandSidecarKey::visible(destination.as_ref()),
            );
            if let Some(target) = cross_target {
                self.retarget_alias_backref_key(
                    &CommandSidecarKey::hidden(cmd),
                    CommandSidecarKey::visible(destination.as_ref()),
                    target,
                );
            }
        }
        Ok(())
    }

    /// Sorted hidden-command names of *this* interp (`interp hidden {}`).
    pub(crate) fn own_hidden_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.hidden_commands.keys().cloned().collect();
        names.sort();
        names
    }

    /// `interp hide|expose path cmd ?token?` on a child. When hiding, the
    /// command `cmd` is filed under `token`; when exposing, the hidden `cmd` is
    /// restored as the command `token`.
    pub(crate) fn child_hide(
        &mut self,
        name: &str,
        cmd: &str,
        token: &str,
        hide: bool,
    ) -> Result<bool, String> {
        let Some(id) = self.child_id(name) else {
            return Ok(false);
        };
        // Enter the child so public hide uses the same visibility-aware
        // removal seam as `rename` and same-interpreter `interp hide`.
        self.in_interp(id, |vm| {
            if hide {
                vm.hide_command(cmd, token)
            } else {
                vm.expose_own_command(cmd, token)
            }
        })?;
        Ok(true)
    }

    /// Make this interp safe (`interp create -safe`): hide the commands that
    /// reach the host (filesystem, processes, sockets, the interpreter loader).
    /// Hidden commands move to `hidden_commands`, invocable via
    /// `interp invokehidden` and restorable with `interp expose`.
    fn make_safe(&mut self) {
        // Pinned against real tclsh 8.6.14 (`interp create -safe s; s hidden`):
        // `after` / `vwait` are deliberately NOT on this list — confirmed
        // present and callable inside a real safe child (`s eval {info
        // commands after}` returns `after`). An earlier version of this list
        // incorrectly hid them, which would have broken legitimate
        // safe-interp code using `after idle`/`after cancel`.
        const UNSAFE: &[&str] = &[
            "exec",
            "exit",
            "cd",
            "pwd",
            "glob",
            "open",
            "socket",
            "source",
            "load",
            "file",
            "fconfigure",
            "encoding",
        ];
        // The host-revealing `tcl_platform` elements (C's `Tcl_MakeSafe` unsets
        // os/osVersion/machine/user) plus our backend-introspection keys, so a
        // safe interp exposes only the portable subset.
        const UNSAFE_PLATFORM: &[&str] = &[
            "os",
            "osVersion",
            "machine",
            "user",
            "threaded",
            "runtime",
            "runtimeVersion",
            "wasm",
            "wasi",
            "wasiVersion",
            "ebpf",
        ];
        self.bump_cmd_epoch();
        for &c in UNSAFE {
            let import_origin = self.imported_commands.get(c).cloned();
            let builtin_identity = self.builtin_identity_for_key(c);
            if let Some(cmd) = self.commands.remove(c) {
                self.imported_commands.remove(c);
                self.builtin_identities.remove(c);
                self.hidden_commands.insert(c.to_string(), cmd);
                if let Some(origin) = import_origin {
                    self.hidden_imported_commands.insert(c.to_string(), origin);
                }
                if let Some(identity) = builtin_identity {
                    self.hidden_builtin_identities
                        .insert(c.to_string(), identity);
                }
            }
        }
        for &k in UNSAFE_PLATFORM {
            let _ = self.unset_one(&format!("tcl_platform({k})"), false);
        }
        // A safe interp has no `env` array and no real library/package paths
        // (C's `Tcl_MakeSafe`); the Safe Base re-virtualises an `auto_path`.
        for v in ["env", "tcl_library", "tclDefaultLibrary", "tcl_pkgPath"] {
            let _ = self.unset_one(v, false);
        }
        self.is_safe = true;
    }

    /// Evaluate `script` in the interpreter `id` (`interp eval path …` /
    /// `$child eval …`): switch the engine to that interp on the shared native
    /// stack (C's `Tcl_EvalObjv(targetInterp, …)`), evaluate, and switch back —
    /// so a child→parent alias, a parent→child alias, or a re-entry into an
    /// interp already executing deeper on the stack is a plain nested call, its
    /// state addressable in the arena throughout (issue #946 faults 1–2).
    /// Errors and completion codes propagate through the ordinary return path
    /// (the target's `-errorcode`/`-errorinfo` reach the caller — tclsh-pinned).
    pub(crate) fn eval_in_interp(&mut self, id: InterpId, script: &str) -> Completion<Value> {
        if !self.interp_alive(id) {
            return err("could not find interpreter");
        }
        self.in_interp(id, |vm| match vm.eval_source(script) {
            Ok(c) => c,
            Err(e) => err(e.message),
        })
    }

    /// Evaluate assembled alias-target words (`target prefix… args…`) as one
    /// command in the target interpreter `target_interp`, at its **global**
    /// frame and namespace (C's `TCL_EVAL_INVOKE` — the tclsh-pinned alias
    /// rule).  When the target is the current interp the caller's frame is kept
    /// (a same-interp alias to `set` writes the caller's locals); when it is a
    /// different interp the engine switches into it first.  A `yield` inside
    /// still cannot cross this native re-entry (the same boundary every
    /// `invoke_command` re-entry has — tclsh reports `cannot yield: C stack
    /// busy`), but a cross-interp alias call now *can*, because the target's
    /// interpreter is reached by switching state rather than by suspending.
    pub(crate) fn invoke_alias_words(
        &mut self,
        target_interp: InterpId,
        argv: &[Value],
    ) -> Completion<Value> {
        if target_interp == self.cur {
            self.push_ns(String::new());
            // Alias words are recompiled as a script so aliases to control
            // commands retain Tcl's syntax.  Validate the fixed head through
            // normal resolution first, though: otherwise a compiler-special
            // form (for example `lassign`) can run after its target was
            // renamed away or gated out by the selected release.
            if let Some((head, tail)) = argv.split_first()
                && self.lookup_command(&head.to_str()).is_none()
            {
                let completion = self.invoke_command(&head.to_str(), tail);
                self.pop_ns();
                return completion;
            }
            let script = crate::exec::alias_invoke_script(argv);
            let evaled = self.eval_source(&script);
            self.pop_ns();
            return match evaled {
                Ok(c) => c,
                Err(e) => err(e.message),
            };
        }
        if !self.interp_alive(target_interp) {
            return err("could not find interpreter");
        }
        let script = crate::exec::alias_invoke_script(argv);
        self.in_interp(target_interp, |vm| {
            vm.push_ns(String::new());
            if let Some((head, tail)) = argv.split_first()
                && vm.lookup_command(&head.to_str()).is_none()
            {
                let completion = vm.invoke_command(&head.to_str(), tail);
                vm.pop_ns();
                return completion;
            }
            let evaled = vm.eval_source(&script);
            vm.pop_ns();
            match evaled {
                Ok(c) => c,
                Err(e) => err(e.message),
            }
        })
    }

    /// `interp delete path …` — destroy interpreter `id`: unhook it from its
    /// parent (the `children` map entry and the command that names it) and tear
    /// it down.  The state drops now, or (if it is still executing — a
    /// re-entered child, a target mid-callback) when its last in-flight
    /// evaluation unwinds; its children and the aliases targeting it are swept
    /// regardless (see [`Self::retire_interp`]).  The root interp cannot be
    /// deleted.  Returns whether the interp existed.
    pub(crate) fn delete_interp(&mut self, id: InterpId) -> bool {
        if !self
            .interps
            .get(id.0)
            .is_some_and(|s| !s.dying && !s.is_root())
        {
            return false;
        }
        // Unhook the child mapping + command in the owning parent (the interp
        // that created it — where the `children` entry and the naming command
        // live), which may or may not be the current interp.
        if let Some(parent) = self.interps[id.0].parent {
            let name = self.st_of(parent).and_then(|st| {
                st.children
                    .iter()
                    .find(|&(_, &v)| v == id)
                    .map(|(k, _)| k.clone())
            });
            if let Some(name) = name {
                self.in_interp(parent, |vm| {
                    vm.children.remove(&name);
                    vm.bump_cmd_epoch();
                    vm.commands.remove(name.strip_prefix("::").unwrap_or(&name));
                });
            }
        }
        self.retire_interp(id);
        true
    }

    /// Dispatch a child-as-command call (`$child sub ?arg …?`) — the `interp`
    /// ensemble restricted to that child.
    /// The exported command tails of namespace `ns` (the default ensemble
    /// subcommand set): commands directly in `ns` whose tail matches an export
    /// pattern.
    fn exported_command_tails(&self, ns: &str) -> Vec<String> {
        let prefix = if ns.is_empty() {
            String::new()
        } else {
            format!("{ns}::")
        };
        let patterns = self.ns_exports.get(ns).cloned().unwrap_or_default();
        let mut out = Vec::new();
        for key in self.commands.keys() {
            let Some(tail) = key.strip_prefix(&prefix) else {
                continue;
            };
            if tail.is_empty() || tail.contains("::") {
                continue;
            }
            if patterns
                .iter()
                .any(|p| tcl_syntax::glob::string_match(p, tail))
            {
                out.push(tail.to_string());
            }
        }
        out
    }

    /// Dispatch a `namespace ensemble` call (`ens ?param …? sub ?arg …?`):
    /// resolve `sub` against the ensemble's subcommands through the shared
    /// `tcl_cmd_core::ensemble` scan and invoke the mapped target (`-map`, else
    /// `namespace::sub`), threading any `-parameters` values in after the
    /// target prefix (`NsEnsembleImplementationCmd`, `tclEnsemble.c`).
    pub(crate) fn dispatch_ensemble(
        &mut self,
        ens_name: &str,
        e: &EnsembleDef,
        argv: &[Value],
    ) -> Completion<Value> {
        // `-parameters` formals sit between the ensemble command and the
        // subcommand word, so the subcommand is at `nparams`.
        let nparams = e.parameters.len();
        if argv.len() <= nparams {
            let mut usage = ens_name.to_string();
            for p in &e.parameters {
                usage.push(' ');
                usage.push_str(p);
            }
            return err(format!(
                "wrong # args: should be \"{usage} subcommand ?arg ...?\""
            ));
        }
        let params = &argv[..nparams];
        let sub = argv[nparams].to_str().to_string();
        let rest = &argv[nparams + 1..];
        let mut subs: Vec<String> = match &e.subcommands {
            Some(list) => list.clone(),
            None => self.exported_command_tails(&e.namespace),
        };
        for (k, _) in &e.map {
            if !subs.contains(k) {
                subs.push(k.clone());
            }
        }
        subs.sort();
        subs.dedup();
        match tcl_cmd_core::ensemble::resolve_subcommand(&subs, sub.as_bytes(), e.prefixes) {
            Some(index) => {
                let resolved = &subs[index];
                let mut full: Vec<Value> = match e
                    .map
                    .iter()
                    .find(|(k, _)| k == resolved)
                    .map(|(_, words)| words)
                {
                    Some(words) => words.clone(),
                    None => vec![Value::string(if e.namespace.is_empty() {
                        resolved.clone()
                    } else {
                        format!("{}::{resolved}", e.namespace)
                    })],
                };
                full.extend_from_slice(params);
                full.extend_from_slice(rest);
                let target = full[0].to_str().to_string();
                self.invoke_command(&target, &full[1..])
            }
            None if e.unknown.is_some() => {
                let mut full = e.unknown.clone().unwrap_or_default();
                full.push(Value::string(ens_name));
                full.extend_from_slice(argv);
                let target = full[0].to_str().to_string();
                self.invoke_command(&target, &full[1..])
            }
            None => err(String::from_utf8_lossy(
                &tcl_cmd_core::ensemble::unknown_subcommand_message(
                    &subs,
                    sub.as_bytes(),
                    e.prefixes,
                    display_namespace(&e.namespace).as_bytes(),
                ),
            )
            .into_owned()),
        }
    }

    /// Dispatch a child-as-command call (`$child sub ?arg …?`): the `interp`
    /// ensemble restricted to interpreter `id`. `name` is the command word
    /// (the child's own command name) used in error messages.
    pub(crate) fn dispatch_child(
        &mut self,
        name: &str,
        id: InterpId,
        argv: &[Value],
    ) -> Completion<Value> {
        let Some((sub, rest)) = argv.split_first() else {
            return err(format!("wrong # args: should be \"{name} cmd ?arg ...?\""));
        };
        match &*sub.to_str() {
            "eval" => {
                let script = rest
                    .iter()
                    .map(|v| v.to_str().to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                self.eval_in_interp(id, &script)
            }
            "issafe" => ok(Value::bool(self.st_of(id).is_some_and(|st| st.is_safe))),
            "delete" => {
                self.delete_interp(id);
                ok(Value::empty())
            }
            "hidden" => {
                let mut names: Vec<String> = self
                    .st_of(id)
                    .map(|st| st.hidden_commands.keys().cloned().collect())
                    .unwrap_or_default();
                names.sort();
                ok(Value::list(names.into_iter().map(Value::string).collect()))
            }
            "hide" | "expose" if rest.len() == 1 => {
                let hide = &*sub.to_str() == "hide";
                if self.is_safe() {
                    let verb = if hide { "hide" } else { "expose" };
                    return err(format!(
                        "permission denied: safe interpreter cannot {verb} commands"
                    ));
                }
                let c = rest[0].to_str();
                match self.child_hide_by_id(id, &c, &c, hide) {
                    Ok(()) => ok(Value::empty()),
                    Err(message) => err(message),
                }
            }
            "marktrusted" => {
                if self.is_safe() {
                    return err("permission denied: safe interpreter cannot mark trusted");
                }
                self.mark_interp_trusted(id);
                ok(Value::empty())
            }
            "invokehidden" if !rest.is_empty() => self.dispatch_child_invokehidden(name, id, rest),
            "recursionlimit" if rest.len() <= 1 => {
                self.dispatch_child_recursionlimit(name, id, rest)
            }
            "recursionlimit" => err(format!(
                "wrong # args: should be \"{name} recursionlimit ?newlimit?\""
            )),
            "limit" => self.dispatch_child_limit(name, id, rest),
            other => err(format!(
                "bad option \"{other}\": must be alias, aliases, bgerror, eval, \
                 expose, hide, hidden, issafe, invokehidden, limit, marktrusted, \
                 recursionlimit, or transfer"
            )),
        }
    }

    /// [`Self::dispatch_child`]'s `invokehidden` arm.
    fn dispatch_child_invokehidden(
        &mut self,
        name: &str,
        id: InterpId,
        rest: &[Value],
    ) -> Completion<Value> {
        if self.is_safe() {
            return err("not allowed to invoke hidden commands from safe interpreter");
        }
        // Skip unmodelled `-namespace ns` / `--` flags.
        let mut i = 0;
        while i < rest.len() {
            match &*rest[i].to_str() {
                "-namespace" => i += 2,
                "--" => {
                    i += 1;
                    break;
                }
                s if s.starts_with('-') => i += 1,
                _ => break,
            }
        }
        match rest.get(i) {
            Some(cmd) => self
                .invoke_hidden_by_id(id, &cmd.to_str(), &rest[i + 1..])
                .unwrap_or_else(|| err(format!("could not find interpreter \"{name}\""))),
            None => err(format!(
                "wrong # args: should be \"{name} invokehidden ?-namespace ns? ?--? cmd ?arg ..?\""
            )),
        }
    }

    /// [`Self::dispatch_child`]'s `recursionlimit` arm.
    fn dispatch_child_recursionlimit(
        &mut self,
        name: &str,
        id: InterpId,
        rest: &[Value],
    ) -> Completion<Value> {
        let nl = rest.first().map(|v| v.to_str().to_string());
        if !self.interp_alive(id) {
            return err(format!("could not find interpreter \"{name}\""));
        }
        match self.in_interp(id, |vm| vm.recursion_limit_apply(nl.as_deref())) {
            Ok(n) => ok(Value::int(n)),
            Err(m) => err(m),
        }
    }

    /// [`Self::dispatch_child`]'s `limit` arm.
    fn dispatch_child_limit(
        &mut self,
        name: &str,
        id: InterpId,
        rest: &[Value],
    ) -> Completion<Value> {
        let (ltype, opts) = match rest {
            [ltype, opts @ ..] => (ltype.to_str().to_string(), opts.to_vec()),
            _ => {
                return err(format!(
                    "wrong # args: should be \"{name} limit limitType ?-option value ...?\""
                ));
            }
        };
        if !self.interp_alive(id) {
            return err(format!("could not find interpreter \"{name}\""));
        }
        match self.in_interp(id, |vm| vm.limit_apply(&ltype, &opts)) {
            Ok(v) => ok(v),
            Err(m) => err(m),
        }
    }

    /// [`Self::child_hide`] addressing the child by id (the `$child hide/expose`
    /// form, whose id is already resolved).
    fn child_hide_by_id(
        &mut self,
        id: InterpId,
        cmd: &str,
        token: &str,
        hide: bool,
    ) -> Result<(), String> {
        if !self.interp_alive(id) {
            return Err("interpreter no longer exists".to_string());
        }
        // This is the `$child hide` spelling of the same public operation.
        self.in_interp(id, |vm| {
            if hide {
                vm.hide_command(cmd, token)
            } else {
                vm.expose_own_command(cmd, token)
            }
        })
    }

    /// [`Self::invoke_hidden_in_child`] addressing the child by id.
    fn invoke_hidden_by_id(
        &mut self,
        id: InterpId,
        cmd: &str,
        args: &[Value],
    ) -> Option<Completion<Value>> {
        if !self.interp_alive(id) {
            return None;
        }
        Some(self.in_interp(id, |vm| vm.invoke_own_hidden(cmd, args)))
    }

    pub(crate) fn lookup_command(&self, name: &str) -> Option<Command> {
        let key = self.resolve_command_fqn(self.current_ns(), name)?;
        let command = self.commands.get(&key).cloned()?;
        if !self.builtin_command_visible_for_surface(&key, &command) {
            return None;
        }
        Some(command)
    }

    /// Whether the exact visible command-table owner exists on the active
    /// release surface.  Unlike [`Self::lookup_command`], this never applies
    /// current-namespace or namespace-path resolution.
    fn visible_command_exists_exact(&self, key: &str) -> bool {
        self.commands
            .get(key)
            .is_some_and(|command| self.builtin_command_visible_for_surface(key, command))
    }

    /// Get the current namespace's command resolution path (`namespace path`)
    /// as a list of canonical names (no leading `::`); empty by default.
    pub(crate) fn ns_path_get(&self) -> Vec<String> {
        self.ns_paths
            .get(self.current_ns())
            .cloned()
            .unwrap_or_default()
    }

    /// Set the current namespace's command resolution path to `path` (canonical
    /// names, no leading `::`).
    pub(crate) fn ns_path_set(&mut self, path: Vec<String>) {
        self.bump_cmd_epoch();
        let cur = self.current_ns().to_string();
        self.ns_paths.insert(cur, path);
    }

    /// The current namespace's `namespace unknown` handler prefix, or empty
    /// when unset (the caller reports the `::unknown` default).
    pub(crate) fn ns_unknown_get(&self) -> Vec<Value> {
        self.ns_unknowns
            .get(self.current_ns())
            .cloned()
            .unwrap_or_default()
    }

    /// Set (non-empty) or reset (empty) the current namespace's
    /// `namespace unknown` handler prefix.
    pub(crate) fn ns_unknown_set(&mut self, handler: Vec<Value>) {
        let cur = self.current_ns().to_string();
        if handler.is_empty() {
            self.ns_unknowns.remove(&cur);
        } else {
            self.ns_unknowns.insert(cur, handler);
        }
    }

    /// The `namespace unknown` handler a resolution miss in the current
    /// namespace falls back to: the current namespace's own handler, else
    /// the global namespace's (the interp default — TIP 181; handlers are
    /// NOT inherited from parent namespaces, tclsh-pinned). `None` when
    /// neither is set (callers then try the plain `unknown` proc) or while
    /// a handler is already being dispatched (reentrancy guard).
    pub(crate) fn ns_unknown_handler(&self) -> Option<Vec<Value>> {
        if self.ns_unknown_depth > 0 {
            return None;
        }
        self.ns_unknowns
            .get(self.current_ns())
            .or_else(|| self.ns_unknowns.get(""))
            .cloned()
    }

    /// Run `f` with the `namespace unknown` reentrancy guard held.
    pub(crate) fn with_ns_unknown_guard<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        self.ns_unknown_depth += 1;
        let out = f(self);
        self.ns_unknown_depth -= 1;
        out
    }

    /// The current namespace (canonical, no leading `::`; `""` = global).
    pub(crate) fn current_ns(&self) -> &str {
        self.ns_stack.last().map_or("", String::as_str)
    }

    /// Intern a canonical namespace name to its stable `NsId`, minting one on
    /// first sight (`ROOT_NS` = 0 = `""`). The handle a `Frames::push` caller
    /// passes — and what `Namespaces::current` will return — round-trips through
    /// [`ns_name`](Self::ns_name).
    pub fn intern_ns(&mut self, name: &str) -> NsId {
        if let Some(&id) = self.ns_intern.get(name) {
            return id;
        }
        let id = NsId(u32::try_from(self.ns_arena.len()).expect("namespace count fits u32"));
        self.ns_arena.push(name.to_string());
        self.ns_intern.insert(name.to_string(), id);
        id
    }

    /// The canonical namespace name for an interned `NsId` (`""` for `ROOT_NS`
    /// or any unknown id — a `Frames::push` into the global namespace).
    fn ns_name(&self, id: NsId) -> String {
        self.ns_arena
            .get(id.0 as usize)
            .cloned()
            .unwrap_or_default()
    }

    /// Whether at least one `enterstep`/`leavestep`-capable execution trace is
    /// registered anywhere in this interp — C's "any trace forbidding inline
    /// compilation exists" test (`iPtr->tracesForbiddingInline`). While true,
    /// every proc entered compiles trace-visible (deoptimised) instead of
    /// fast/inlined; see [`Vm::ensure_proc_compiled_for_tracing`].
    pub(crate) fn step_trace_active(&self) -> bool {
        self.exec_traces
            .values()
            .any(|list| list.iter().any(|t| is_step_capable(&t.ops)))
    }

    /// Bump the trace-deopt epoch (see [`Self::trace_deopt_epoch`]).
    pub(crate) fn bump_trace_deopt_epoch(&self) {
        self.trace_deopt_epoch
            .set(self.trace_deopt_epoch.get().wrapping_add(1));
    }

    /// Invalidate every offline guard that depends on `domain`.
    ///
    /// Keep this at the mutation owner: guard epochs are useful only when no
    /// registration path can bypass the bump.
    fn invalidate_guard_domain(&self, domain: GuardDomain) {
        self.guards.borrow_mut().invalidate(domain);
    }

    /// The current trace-deopt epoch.
    pub(crate) fn trace_deopt_epoch(&self) -> u64 {
        self.trace_deopt_epoch.get()
    }
}

/// Whether a `trace add execution` op set includes `enterstep`/`leavestep` —
/// the C-Tcl-significant "forbids inline compilation" test
/// (`TCL_TRACE_ENTER_DURING_EXEC | TCL_TRACE_LEAVE_DURING_EXEC`). A plain
/// `enter`/`leave` (without step) does not: the traced command's own
/// dispatch already fires it (procs are never opcode-inlined), so it needs
/// no deoptimisation.
fn is_step_capable(ops: &[String]) -> bool {
    ops.iter().any(|o| o == "enterstep" || o == "leavestep")
}

/// Command-resolution methods live on [`InterpState`] (not [`Vm`]) so the
/// engine can resolve a command in *any* interpreter, current or parked — the
/// cross-interp alias-loop walk queries a foreign interp's table. Deref
/// coercion keeps every `impl Vm` caller (`self.resolve_command_fqn(…)`)
/// working against the current interp unchanged.
impl InterpState {
    /// Invalidate the command-resolution memo (M16.4): every command-table /
    /// `namespace path` / runtime-version mutation routes through here so a
    /// cached `(namespace, name) → key` can never outlive the state it was
    /// computed from.
    pub(crate) fn bump_cmd_epoch(&self) {
        // Clearing on every mutation makes the resolution memo safe even if its
        // diagnostic epoch saturates. Guard epochs use their own checked,
        // poison-on-exhaustion counters.
        self.cmd_resolve_cache.borrow_mut().1.clear();
        self.cmd_epoch.set(self.cmd_epoch.get().saturating_add(1));
        self.guarded_commands.borrow_mut().clear();
        let mut guards = self.guards.borrow_mut();
        guards.invalidate(GuardDomain::CommandEnvironment);
        guards.invalidate(GuardDomain::Namespace);
        guards.invalidate(GuardDomain::UnknownHandling);
    }

    /// Resolve `name` from namespace `cxt` to its command's canonical key
    /// (the `commands` map key — a qualified name without the leading `::`),
    /// via the shared C-Tcl resolution rule
    /// ([`tcl_syntax::naming::resolve_command_with`]): an absolute `::name`
    /// directly, else `cxt`'s candidate, then each of `cxt`'s
    /// `namespace path` entries in order, then global — dispatching the
    /// first that **exists**.  `None` if unresolved.
    pub(crate) fn resolve_command_fqn(&self, cxt: &str, name: &str) -> Option<String> {
        // M16.4 memo — C caches the resolution on the name object and
        // invalidates by interp epoch; here the memo is a per-Vm map keyed
        // `cxt␁name`, cleared whenever its stored epoch is stale.
        let epoch = self.cmd_epoch.get();
        let memo_key = format!("{cxt}\u{0001}{name}");
        {
            let cache = self.cmd_resolve_cache.borrow();
            if cache.0 == epoch
                && let Some(hit) = cache.1.get(&memo_key)
            {
                return hit.clone();
            }
        }
        let res = self.resolve_command_fqn_uncached(cxt, name, true);
        let mut cache = self.cmd_resolve_cache.borrow_mut();
        if cache.0 != epoch {
            cache.0 = epoch;
            cache.1.clear();
        }
        // A dynamic-name flood must not grow the memo without bound (C's is
        // naturally bounded by object lifetimes).
        if cache.1.len() >= 4096 {
            cache.1.clear();
        }
        cache.1.insert(memo_key, res.clone());
        res
    }

    /// Resolve a registered command for VM/embedder teardown without applying
    /// the emulated release's public command-surface filter.
    ///
    /// This deliberately bypasses only availability filtering. Namespace,
    /// namespace-path, and global candidate ordering remain the active Tcl
    /// release's resolution rules. Never use this for Tcl-visible dispatch.
    fn resolve_command_fqn_raw(&self, cxt: &str, name: &str) -> Option<String> {
        self.resolve_command_fqn_uncached(cxt, name, false)
    }

    /// The uncached resolution rule — see [`Self::resolve_command_fqn`].
    fn resolve_command_fqn_uncached(
        &self,
        cxt: &str,
        name: &str,
        filter_public_surface: bool,
    ) -> Option<String> {
        // Collapse separator runs up front — C treats any colon run as one
        // separator, so `foo:::bar` dispatches `foo::bar` and `quux:::` the
        // `{}` command `quux::` (tclsh8.6-verified). The key form keeps the
        // shared resolver's candidates aligned with the command table; a
        // rooted name stays absolute.
        let cleaned = canonical_cmd_key(name);
        let name: std::borrow::Cow<'_, str> = if name.starts_with("::") {
            std::borrow::Cow::Owned(format!("::{cleaned}"))
        } else {
            cleaned
        };
        // `ns_paths` entries are stored in the VM's canonical form — always
        // absolute, no leading `::` (rooted at set time by `canon_ns`).
        // Root them before handing to the shared resolver, whose unrooted
        // entries mean *current-namespace-relative* (the Tcl source form —
        // tclsh-pinned; see `command_resolution_candidates`).
        //
        // The path tier itself is a Tcl 8.5 feature (TIP 181): 8.4 resolves
        // current-namespace → global only (M10.1; 8.4 `tclNamesp.c` has no
        // path walk).  Gated here at resolution time — not at `ns_path_set`
        // recording — so flipping `set_runtime_version` mid-life re-applies
        // the correct tier to paths recorded earlier.
        let rooted: Vec<String> = if self.runtime_version.has_namespace_path() {
            self.ns_paths
                .get(cxt)
                .map_or_else(Vec::new, |p| p.iter().map(|e| format!("::{e}")).collect())
        } else {
            Vec::new()
        };
        // Candidates from the shared resolver are rooted constructed keys;
        // the VM's table is keyed unrooted.  Strip exactly ONE root — a
        // char-pattern trim would collapse a lone-colon key (`":::"`, the
        // proc named `:`) into the empty-name `{}` key (#934).
        let unroot = |c: &str| {
            c.strip_prefix("::")
                .map_or_else(|| c.to_string(), String::from)
        };
        tcl_syntax::naming::resolve_command_with(cxt, &rooted, &name, |candidate| {
            let key = unroot(candidate);
            self.commands.get(&key).is_some_and(|command| {
                !filter_public_surface || self.builtin_command_visible_for_surface(&key, command)
            })
        })
        .map(|winner| unroot(&winner))
    }
}

impl Vm {
    /// Intern an absolute command FQN to a stable, dense raw `CommandId`, minting
    /// one on first sight. Backs `Namespaces::find_command`.
    fn intern_cmd(&self, fqn: &str) -> u32 {
        let mut a = self.cmd_arena.borrow_mut();
        if let Some(&id) = a.ids.get(fqn) {
            return id;
        }
        let id = u32::try_from(a.fqns.len()).expect("command count fits u32");
        a.fqns.push(fqn.to_string());
        a.ids.insert(fqn.to_string(), id);
        id
    }

    /// The absolute FQN an interned raw `CommandId` was minted from, or `None`
    /// for a fabricated/out-of-range id. Backs `Commands::dispatch_id`'s reverse.
    fn command_fqn(&self, id: u32) -> Option<String> {
        self.cmd_arena.borrow().fqns.get(id as usize).cloned()
    }

    /// Push a namespace onto the resolution stack (created if new). The name is
    /// normalised to canonical form first ([`canonical_ns_name`]), so
    /// `namespace eval c::: {}` enters (and creates) `c`, matching tclsh.
    pub(crate) fn push_ns(&mut self, ns: String) {
        let ns = match canonical_ns_name(&ns) {
            std::borrow::Cow::Borrowed(_) => ns,
            std::borrow::Cow::Owned(o) => o,
        };
        if !ns.is_empty() {
            self.namespaces.insert(ns.clone());
        }
        // Ensure it has an `NsId` so `Namespaces::current` (a `&self` lookup) can
        // resolve it without minting.
        self.intern_ns(&ns);
        self.ns_stack.push(ns);
    }

    /// Pop the current namespace (the global base is never popped).
    pub(crate) fn pop_ns(&mut self) {
        if self.ns_stack.len() > 1 {
            self.ns_stack.pop();
        }
    }

    /// Canonicalise a name (no leading `::`) relative to the current namespace:
    /// an absolute `::a::b` drops the leading `::`; anything else is qualified
    /// with the current namespace. Separator runs collapse to the key form
    /// ([`canonical_cmd_key`]), so `foo:::bar` names `foo::bar`.
    pub(crate) fn qualify_name(&self, name: &str) -> String {
        if name.starts_with("::") {
            return canonical_cmd_key(name).into_owned();
        }
        // Canonicalise the *written* relative name first, then join the
        // current namespace key with one exact separator — canonicalising the
        // concatenation would collapse a lone-colon name into the `{}` key
        // (#934: `proc :` inside namespace `a` is `a` + `::` + `:`, distinct
        // from `a::`, the empty-named proc).
        let canonical = canonical_cmd_key(name);
        let cur = self.current_ns();
        if cur.is_empty() {
            canonical.into_owned()
        } else {
            format!("{cur}::{canonical}")
        }
    }

    /// Whether namespace `ns` (canonical, unrooted; `""` is the always-present
    /// global namespace) currently exists.
    pub(crate) fn namespace_exists(&self, ns: &str) -> bool {
        ns.is_empty() || self.namespaces.contains(ns)
    }

    /// Register an existing namespace (and its ancestors). The name is
    /// normalised to canonical form first, and the ancestor walk uses the
    /// shared separator-run-aware split ([`tcl_cmd_core::namespace`]), so a
    /// colon-run name (`a:::b`) registers `a::b` under parent `a` — never a
    /// bogus `a:` (the old `rsplit_once("::")` drift).
    /// [`Self::declare_namespace`] for an already-canonical **key** (a
    /// construction-inverse holder): no written-name canonicalisation, which
    /// would collapse a lone-colon segment (#934), and the parent chain walks
    /// the construction-inverse split for the same reason.
    pub(crate) fn declare_namespace_key(&mut self, ns_key: &str) {
        if ns_key.is_empty() {
            return;
        }
        self.namespaces.insert(ns_key.to_string());
        self.intern_ns(ns_key);
        let (parent, _tail) = key_holder_and_tail_unrooted(ns_key);
        if !parent.is_empty() {
            self.declare_namespace_key(&parent);
        }
    }

    pub(crate) fn declare_namespace(&mut self, ns: &str) {
        let ns = canonical_ns_name(ns);
        if ns.is_empty() {
            return;
        }
        self.namespaces.insert(ns.to_string());
        // Mint a stable `NsId` (handle) so the `Namespaces` nav methods are pure
        // `&self` lookups — every namespace, however created, has an id.
        self.intern_ns(&ns);
        let parent = tcl_cmd_core::namespace::qualifiers(ns.as_bytes());
        if !parent.is_empty() {
            self.declare_namespace(core::str::from_utf8(parent).expect("subslice of valid UTF-8"));
        }
    }

    /// Record `namespace export` patterns for the current namespace. C's
    /// `Tcl_Export` skips a pattern already in the array, so the list is a
    /// set in insertion order — `namespace export a; namespace export a`
    /// still reports `a`.
    pub(crate) fn add_exports(&mut self, patterns: &[String]) {
        let ns = self.current_ns().to_string();
        let entry = self.ns_exports.entry(ns).or_default();
        for pattern in patterns {
            if !entry.iter().any(|existing| existing == pattern) {
                entry.push(pattern.clone());
            }
        }
    }

    /// The current namespace's `namespace export` pattern list (C's
    /// `Tcl_AppendExportList`, the `namespace export` query form).
    pub(crate) fn exports_get(&self) -> Vec<String> {
        self.ns_exports
            .get(self.current_ns())
            .cloned()
            .unwrap_or_default()
    }

    /// Drop the current namespace's export patterns — the state change
    /// `namespace export -clear` makes (`Tcl_Export`'s `resetListFirst`).
    pub(crate) fn clear_exports(&mut self) {
        let ns = self.current_ns().to_string();
        self.ns_exports.remove(&ns);
    }

    /// The unqualified names of the current namespace's commands that
    /// `namespace import` created — C's `NamespaceImportCmd` introspection
    /// form (`objc == 1`), a `cmdTable` walk for `deleteProc ==
    /// DeleteImportedCmd`. Sorted, since the VM's table has no hash order to
    /// reproduce.
    pub(crate) fn imported_command_tails(&self) -> Vec<String> {
        let ns = self.current_ns();
        let mut names: Vec<String> = self
            .imported_commands
            .keys()
            .filter_map(|key| direct_member_tail(key, ns).map(str::to_owned))
            .collect();
        names.sort();
        names
    }

    /// Declare a built-in namespace (`ns`, unrooted) and record its
    /// `namespace export` patterns directly. Used for namespaces whose commands
    /// are created in Rust rather than by a script `namespace export` — e.g.
    /// `::tcl::mathop`, which C exports so `namespace import ::tcl::mathop::*`
    /// works.
    pub(crate) fn declare_namespace_exports(&mut self, ns: &str, patterns: &[&str]) {
        self.declare_namespace(ns);
        let entry = self.ns_exports.entry(ns.to_string()).or_default();
        for p in patterns {
            if !entry.iter().any(|e| e == p) {
                entry.push((*p).to_string());
            }
        }
    }

    /// `namespace import` for `pattern` (e.g. `::tcltest::*`): alias every
    /// exported command of the source namespace matching the glob into the
    /// current namespace under its tail name. Returns the imported tail names.
    pub(crate) fn import_commands(&mut self, pattern: &str) -> Vec<String> {
        // Split the glob tail off at the last separator *run* and canonicalise
        // the qualifier — `namespace import ::src:::im*` imports from `src`
        // (tclsh8.6-verified; the old `rsplit_once("::")` left `src:` behind).
        let pb = pattern.as_bytes();
        let glob = str_slice(tcl_cmd_core::namespace::tail(pb)).to_string();
        let src_ns =
            canonical_ns_name(str_slice(tcl_cmd_core::namespace::qualifiers(pb))).into_owned();
        let exports = self.ns_exports.get(&src_ns).cloned().unwrap_or_default();
        let prefix = if src_ns.is_empty() {
            String::new()
        } else {
            format!("{src_ns}::")
        };
        // Candidate commands: those in the source namespace whose tail matches
        // the import glob and an export pattern.
        let mut to_import: Vec<(String, Command, Option<String>)> = Vec::new();
        for (cmd_name, cmd) in &self.commands {
            let Some(tail) = cmd_name.strip_prefix(&prefix) else {
                continue;
            };
            if tail.is_empty() || tail.contains("::") {
                continue;
            }
            if tcl_syntax::glob::string_match(&glob, tail)
                && exports
                    .iter()
                    .any(|p| tcl_syntax::glob::string_match(p, tail))
                // C imports only a command visible in the current release.
                // Skipping a hidden source avoids manufacturing a local clone
                // at 8.4 and keeps import chains' provenance meaningful.
                && self.builtin_command_visible_for_surface(cmd_name, cmd)
            {
                let builtin_identity = matches!(cmd, Command::Builtin(_) | Command::Native(_))
                    .then(|| {
                        self.builtin_identities
                            .get(cmd_name)
                            .cloned()
                            .unwrap_or_else(|| cmd_name.clone())
                    });
                to_import.push((tail.to_string(), cmd.clone(), builtin_identity));
            }
        }
        let mut imported = Vec::new();
        for (tail, cmd, builtin_identity) in to_import {
            let alias = self.qualify_name(&tail);
            let origin = format!("{prefix}{tail}");
            self.register_command(&alias, cmd);
            // `register_command` cleared any stale provenance; now stamp this key
            // as an import so `namespace forget` can target it.
            self.imported_commands
                .insert(alias.clone(), CommandSidecarKey::visible(origin));
            if let Some(identity) = builtin_identity {
                self.builtin_identities.insert(alias, identity);
            }
            imported.push(tail);
        }
        imported
    }

    /// The command key `key` ultimately came from, following the
    /// `namespace import` chain to its source — C's `TclGetOriginalCommand`,
    /// which backs the `originCmd` opcode (`namespace origin`). `key` itself
    /// when it names no import. Keys are canonical (unrooted), as
    /// `imported_commands` stores them.
    pub(crate) fn command_origin_key(&self, key: &str) -> String {
        self.origin_key_of(&CommandSidecarKey::visible(key))
    }

    /// [`Self::command_origin_key`] from an explicit domain, so a hidden token
    /// can be walked as itself rather than as an equally-named visible
    /// command.
    pub(crate) fn origin_key_of(&self, start: &CommandSidecarKey) -> String {
        let mut cur = start.clone();
        // Bounded walk: a chain cannot be longer than the import table, so a
        // (malformed) cycle terminates instead of spinning.
        for _ in 0..self.imported_commands.len() + self.hidden_imported_commands.len() {
            let next = match &cur {
                CommandSidecarKey::Visible(visible) => self.imported_commands.get(visible),
                CommandSidecarKey::Hidden(hidden) => self.hidden_imported_commands.get(hidden),
            };
            match next {
                Some(next) => cur = next.clone(),
                None => break,
            }
        }
        cur.name().to_owned()
    }

    /// Retarget imports when their source command is renamed.  The command
    /// table stores an import as a cloned dispatcher, whereas C stores the
    /// source command token; rewrite the provenance explicitly so dispatch,
    /// visibility, and `namespace origin` continue to identify the final
    /// builtin under its new name.  A deletion deliberately does not take this
    /// path: C leaves the imported command dangling in that case.
    pub(crate) fn retarget_imports(&mut self, old_key: &str, new_key: &str) {
        self.retarget_imports_key(
            &CommandSidecarKey::visible(old_key),
            &CommandSidecarKey::visible(new_key),
        );
    }

    fn retarget_imports_key(&mut self, old_key: &CommandSidecarKey, new_key: &CommandSidecarKey) {
        for source in self.imported_commands.values_mut() {
            if source == old_key {
                source.clone_from(new_key);
            }
        }
        for source in self.hidden_imported_commands.values_mut() {
            if source == old_key {
                source.clone_from(new_key);
            }
        }
    }

    /// Restore a renamed imported command's own origin record.
    pub(crate) fn restore_import_origin(&mut self, key: &str, origin: CommandSidecarKey) {
        self.imported_commands.insert(key.to_owned(), origin);
    }

    /// Re-point every `namespace import` clone of `origin_key` at `cmd`,
    /// keeping each clone's import provenance.
    ///
    /// C shares the source's command token with its imports, so a change to
    /// the source is observed through every spelling at once. The VM instead
    /// stores an import as a cloned dispatcher, so a rebind of the source has
    /// to be pushed to the clones explicitly — otherwise the alias keeps
    /// dispatching the stale definition (`namespace ensemble configure` on an
    /// imported ensemble: tclsh 9.0.4 sees the new config through both
    /// spellings). `register_command` drops the provenance record as part of
    /// overwriting, so it is captured first and restored after, leaving
    /// `namespace origin` still answering the source.
    ///
    /// Membership is by **ultimate** origin, not by the direct edge: an import
    /// of an import (`::S::e` imported and re-exported by `::A`, then imported
    /// by `::B`) is one shared token in C, so every spelling in the chain has
    /// to be refreshed, not just `::A::e`. Hidden clones count too — `interp
    /// hide` moves the entry but not its provenance, and `invokehidden` would
    /// otherwise still reach the stale definition.
    pub(crate) fn resync_import_clones(&mut self, origin_key: &str, cmd: &Command) {
        let visible: Vec<(String, CommandSidecarKey)> = self
            .imported_commands
            .iter()
            .filter(|(key, _)| self.command_origin_key(key) == origin_key)
            .map(|(key, source)| (key.clone(), source.clone()))
            .collect();
        let hidden: Vec<String> = self
            .hidden_imported_commands
            .keys()
            .filter(|token| {
                self.origin_key_of(&CommandSidecarKey::hidden(token.as_str())) == origin_key
            })
            .cloned()
            .collect();
        for (key, source) in visible {
            self.register_command(&key, cmd.clone());
            self.restore_import_origin(&key, source);
        }
        for token in hidden {
            // The hidden table holds only the command; its provenance lives in
            // `hidden_imported_commands` and is untouched by this write.
            self.hidden_commands.insert(token, cmd.clone());
        }
    }

    /// Restore a renamed builtin's stable registry identity.
    pub(crate) fn restore_builtin_identity(&mut self, key: &str, identity: String) {
        self.builtin_identities.insert(key.to_owned(), identity);
    }

    /// Record `key` as an engine-installed `TclOO` root object command, so the
    /// release-availability gate treats it like a builtin (see
    /// [`InterpState::registry_object_roots`]).
    pub(crate) fn declare_registry_object_root(&mut self, key: &str) {
        self.declare_registry_object_root_as(key, key);
    }

    /// [`Self::declare_registry_object_root`] with an explicit registry
    /// identity, for a root that has been renamed away from the name the
    /// registry dates it by.
    pub(crate) fn declare_registry_object_root_as(&mut self, key: &str, identity: &str) {
        self.registry_object_roots
            .insert(key.to_owned(), identity.to_owned());
    }

    /// `namespace forget pattern` — remove previously imported commands matching
    /// `pattern` from the current namespace (C `Tcl_ForgetImport`). Only commands
    /// created by `namespace import` are removed; a real command of the same name
    /// is left intact. A simple (unqualified) pattern matches imported commands
    /// in the current namespace by name; a qualified `ns::pat` pattern matches
    /// those whose origin lives in `ns` and whose origin tail matches `pat`.
    /// Returns `Err` for an unknown namespace in a qualified pattern.
    pub(crate) fn forget_imports(&mut self, pattern: &str) -> Result<(), String> {
        // Split a canonical command key into (namespace, tail) — the shared
        // separator-run-aware byte ops (a canonical key has single `::`s, so
        // this is the plain last-separator split).
        fn split_key(key: &str) -> (&str, &str) {
            let kb = key.as_bytes();
            (
                str_slice(tcl_cmd_core::namespace::qualifiers(kb)),
                str_slice(tcl_cmd_core::namespace::tail(kb)),
            )
        }
        let cur = self.current_ns().to_string();
        let victims: Vec<String> = if tcl_syntax::naming::is_qualified(pattern.as_bytes()) {
            // Qualified pattern: source namespace + simple pattern on the
            // origin. Splitting at the last separator *run* keeps colon-run
            // patterns working (`namespace forget ::src:::im*` forgets from
            // `src`, tclsh8.6-verified; the old `rsplit_once("::")` produced
            // `src:` — and panicked outright on `:::pat`).
            let pb = pattern.as_bytes();
            let simple = str_slice(tcl_cmd_core::namespace::tail(pb));
            let src_ns =
                canonical_ns_name(str_slice(tcl_cmd_core::namespace::qualifiers(pb))).into_owned();
            if !self.namespace_exists(&src_ns) {
                return Err(format!(
                    "unknown namespace in namespace forget pattern \"{pattern}\""
                ));
            }
            self.imported_commands
                .iter()
                .filter(|(key, _)| {
                    split_key(key).0 == cur && {
                        // C's TclGetOriginalCommand follows an import chain;
                        // qualified forget matches that ultimate source, not
                        // merely the immediately imported alias.
                        let origin = self.command_origin_key(key);
                        let (o_ns, o_tail) = split_key(&origin);
                        o_ns == src_ns && tcl_syntax::glob::string_match(simple, o_tail)
                    }
                })
                .map(|(key, _)| key.clone())
                .collect()
        } else {
            // Simple pattern: imported commands in the current namespace whose
            // own name matches.
            self.imported_commands
                .keys()
                .filter(|key| {
                    let (ns, tail) = split_key(key);
                    ns == cur && tcl_syntax::glob::string_match(pattern, tail)
                })
                .cloned()
                .collect()
        };
        self.bump_cmd_epoch();
        for key in victims {
            self.imported_commands.remove(&key);
            self.builtin_identities.remove(&key);
            if self.commands.remove(&key).is_some() {
                self.detach_active_sidecars(&CommandSidecarKey::visible(key));
            }
        }
        Ok(())
    }

    /// Delete namespace `canonical` (no leading `::`) and every descendant,
    /// removing their commands/procs, namespace variables, export patterns, and
    /// interned ids. Returns `false` (deleting nothing) when the namespace does
    /// not exist — the caller reports `unknown namespace`. The global namespace
    /// (`""`) is never deletable.
    pub(crate) fn delete_namespace(&mut self, canonical: &str) -> bool {
        // Callers pass the canonical form; still normalise separator runs so
        // `namespace delete a:::b` removes `a::b` (tclsh8.6-verified).
        let canonical: &str = &canonical_ns_name(canonical);
        if canonical.is_empty() || !self.namespaces.contains(canonical) {
            return false;
        }
        let prefix = format!("{canonical}::");
        let in_tree = |k: &str| k == canonical || k.starts_with(&prefix);
        // C deletes the namespace's ensembles *first* (`Tcl_DeleteNamespace`,
        // `tclNamesp.c:944-959`: `while (nsPtr->ensembles != NULL)` →
        // `Tcl_DeleteCommandFromToken`). An ensemble command is owned by the
        // namespace it dispatches into, wherever the command itself is bound —
        // `namespace ensemble create -command ::myens` inside `::ens1` puts the
        // command in the global table, and `namespace delete ::ens1` must still
        // take it with it.
        // Commands and namespace variables are keyed by their fully-qualified
        // (unrooted) name, so a member of the namespace or a descendant begins
        // with `canonical::`.
        let removed_commands: HashSet<String> = self
            .commands
            .iter()
            .filter(|(key, command)| {
                key.starts_with(&prefix)
                    || matches!(command, Command::Ensemble(def) if in_tree(&def.namespace))
            })
            .map(|(key, _)| key.clone())
            .collect();
        self.bump_cmd_epoch();
        self.commands.retain(|k, _| !removed_commands.contains(k));
        for key in &removed_commands {
            let was_coroutine = crate::cmd_coro::is_coroutine(self, key);
            self.detach_active_sidecars(&CommandSidecarKey::visible(key));
            if was_coroutine {
                crate::cmd_coro::on_command_deleted(self, key);
            }
        }
        self.imported_commands
            .retain(|k, _| !removed_commands.contains(k));
        self.builtin_identities
            .retain(|k, _| !removed_commands.contains(k));
        if let Some(g) = self.frames.first_mut() {
            g.locals.retain(|k, _| !k.starts_with(&prefix));
        }
        self.namespaces.retain(|n| !in_tree(n));
        self.ns_exports.retain(|k, _| !in_tree(k));
        self.ns_intern.retain(|k, _| !in_tree(k));
        // `TclTeardownNamespace` resets the namespace's own parameters
        // (`tclNamesp.c:1148-1165`): it drops its `namespace path`
        // (`UnlinkNsPath`), frees its `namespace unknown` handler, and then
        // walks `commandPathSourceList` to NULL *this* namespace out of every
        // other namespace's path (bumping their `cmdRefEpoch`). Both
        // directions matter: without the first a recreated namespace inherits
        // its predecessor's path and unknown handler, without the second a
        // stale path entry resurrects when the name comes back.
        self.ns_paths.retain(|k, _| !in_tree(k));
        self.ns_unknowns.retain(|k, _| !in_tree(k));
        for path in self.ns_paths.values_mut() {
            path.retain(|entry| !in_tree(entry));
        }
        true
    }

    /// Immediate child namespaces of `parent` (canonical names).
    pub(crate) fn child_namespaces(&self, parent: &str) -> Vec<String> {
        let prefix = if parent.is_empty() {
            String::new()
        } else {
            format!("{parent}::")
        };
        self.namespaces
            .iter()
            .filter(|ns| {
                ns.strip_prefix(&prefix)
                    .is_some_and(|rest| !rest.is_empty() && !rest.contains("::"))
            })
            .cloned()
            .collect()
    }

    /// Record a provided package version.
    pub(crate) fn provide_package(&mut self, name: &str, version: &str) {
        self.packages.insert(name.to_string(), version.to_string());
    }

    /// The provided version of a package, if any.
    pub(crate) fn package_version(&self, name: &str) -> Option<&str> {
        self.packages.get(name).map(String::as_str)
    }

    /// Names of all provided packages.
    pub(crate) fn package_names(&self) -> Vec<String> {
        self.packages.keys().cloned().collect()
    }

    // -- variable traces (`trace add|remove|info variable`) --

    /// The resolved-owner key a variable name's traces are stored under, so a
    /// trace fires regardless of access path (alias / qualified name). An array
    /// *element* reference (`arr(key)`) keys on the resolved element so element
    /// traces are distinct from each other and from whole-array traces.
    fn trace_key(&self, name: &str) -> String {
        if let Some((base, key)) = elem_ref(name) {
            let base = self.trace_qualify(base);
            let (lvl, nm) = self.locate(&base);
            format!("{lvl}\u{0}{nm}({key})")
        } else {
            let name = self.trace_qualify(name);
            let (lvl, nm) = self.locate(&name);
            format!("{lvl}\u{0}{nm}")
        }
    }

    /// Resolve a bare variable name to its namespace-qualified form when the
    /// current scope is a `namespace eval` body, matching how `set`/`variable`
    /// bind a namespace variable. Unlike [`Self::ns_var_fallback`] this does not
    /// require the variable to already exist — `trace add variable foo …` at
    /// namespace-script level targets `::ns::foo` even before it is created, so
    /// the key matches the resolved name a later read inside a proc produces.
    fn trace_qualify(&self, name: &str) -> String {
        if name.contains("::") || !self.in_ns_script() {
            return name.to_string();
        }
        let cur = self.current_ns();
        if cur.is_empty() {
            return name.to_string();
        }
        // A genuine local in the namespace-eval frame shadows the namespace var.
        if self
            .frames
            .last()
            .is_some_and(|f| f.locals.contains_key(name))
        {
            return name.to_string();
        }
        let qualified = format!("{cur}::{name}");
        // Under the 8.x fallback the bare name resolves to the GLOBAL
        // variable, so the trace must be keyed on the global — keying it on
        // `ns::name` here would make a write through the fallback silently
        // miss the global's trace (issue #1328).  Shares one predicate with
        // `locate`, so the two cannot drift.
        if self.ns_fallback_targets_global(&qualified, name) {
            return name.to_string();
        }
        format!("::{qualified}")
    }

    /// Does the Tcl 8.x namespace-scope fallback send bare `name` (whose
    /// namespace-qualified spelling is `qualified`) to the **global**
    /// variable?
    ///
    /// True only under an 8.x runtime, when the namespace has no such variable
    /// but the global namespace does.  The single definition of the rule —
    /// [`Self::locate`] applies it to resolve an access, `trace_qualify`
    /// applies it to key a trace, and they must agree or a trace fires on the
    /// wrong variable (issue #1328).
    fn ns_fallback_targets_global(&self, qualified: &str, name: &str) -> bool {
        let qualified = qualified.strip_prefix("::").unwrap_or(qualified);
        self.ns_var_global_fallback()
            && self.frames.first().is_some_and(|global| {
                !global.locals.contains_key(qualified) && global.locals.contains_key(name)
            })
    }

    /// Register a `trace add variable` callback.
    pub(crate) fn add_var_trace(&mut self, name: &str, ops: Vec<String>, command: String) {
        let key = self.trace_key(name);
        self.var_traces
            .entry(key)
            .or_default()
            .push(VarTrace { ops, command });
        self.invalidate_guard_domain(GuardDomain::VariableTrace);
    }

    /// Remove one `trace remove variable` callback matching `ops` + `command`.
    pub(crate) fn remove_var_trace(&mut self, name: &str, ops: &[String], command: &str) {
        let key = self.trace_key(name);
        let mut removed = false;
        if let Some(list) = self.var_traces.get_mut(&key) {
            if let Some(index) = list
                .iter()
                .position(|t| t.ops == ops && t.command == command)
            {
                list.remove(index);
                removed = true;
            }
            if list.is_empty() {
                self.var_traces.remove(&key);
            }
        }
        if removed {
            self.invalidate_guard_domain(GuardDomain::VariableTrace);
        }
    }

    /// Register a `trace add command|execution` callback on `name` — which,
    /// unlike a variable trace, must resolve to an existing command
    /// (tclsh-pinned: `unknown command "missing"`).
    pub(crate) fn add_cmd_trace(
        &mut self,
        execution: bool,
        name: &str,
        ops: Vec<String>,
        callback: String,
    ) -> Completion<Value> {
        let Some(key) = self.resolve_command_fqn(self.current_ns(), name) else {
            return err(format!("unknown command \"{name}\""));
        };
        let is_step = is_step_capable(&ops);
        let table = if execution {
            &mut self.exec_traces
        } else {
            &mut self.cmd_traces
        };
        table
            .entry(CommandSidecarKey::visible(key))
            .or_default()
            .push(Rc::new(CmdTraceEntry::new(ops, callback)));
        self.invalidate_guard_domain(GuardDomain::CommandTrace);
        // A new enterstep/leavestep-capable trace forces every proc in this
        // interp to (re)compile trace-visible on next call — C's
        // `DONT_COMPILE_CMDS_INLINE` (tclTrace.c). Bumped even if a step
        // trace was already active elsewhere: cheap, and simpler than
        // tracking the exact 0→1 transition.
        if execution && is_step {
            self.bump_trace_deopt_epoch();
        }
        ok(Value::empty())
    }

    /// Remove one command/execution trace matching `ops` + `callback`.
    pub(crate) fn remove_cmd_trace(
        &mut self,
        execution: bool,
        name: &str,
        ops: &[String],
        callback: &str,
    ) -> Completion<Value> {
        let Some(key) = self.resolve_command_fqn(self.current_ns(), name) else {
            return err(format!("unknown command \"{name}\""));
        };
        let is_step = is_step_capable(ops);
        let table = if execution {
            &mut self.exec_traces
        } else {
            &mut self.cmd_traces
        };
        let key = CommandSidecarKey::visible(key);
        let mut removed = false;
        if let Some(list) = table.get_mut(&key) {
            if let Some(index) = list
                .iter()
                .position(|t| t.ops == ops && t.callback == callback)
            {
                list.remove(index);
                removed = true;
            }
            if list.is_empty() {
                table.remove(&key);
            }
        }
        if removed {
            self.invalidate_guard_domain(GuardDomain::CommandTrace);
        }
        // Removing a step-capable trace may re-enable fast (inlined)
        // compilation for procs that no longer have one active anywhere;
        // recompute lazily on next call, same as an add.
        if execution && is_step {
            self.bump_trace_deopt_epoch();
        }
        ok(Value::empty())
    }

    /// The `{ops callback}` pairs registered on command `name` (newest first),
    /// for `trace info command|execution`.
    pub(crate) fn cmd_trace_entries(&self, execution: bool, name: &str) -> Value {
        let table = if execution {
            &self.exec_traces
        } else {
            &self.cmd_traces
        };
        let Some(list) = self
            .resolve_command_fqn(self.current_ns(), name)
            .and_then(|key| table.get(&CommandSidecarKey::visible(key)))
        else {
            return Value::empty();
        };
        Value::list(
            list.iter()
                .rev()
                .map(|t| {
                    Value::list(vec![
                        Value::list(t.ops.iter().map(|o| Value::string(o.clone())).collect()),
                        Value::string(t.callback.clone()),
                    ])
                })
                .collect(),
        )
    }

    /// Run one trace callback with `args` appended (list-quoted), in the
    /// current frame — C evaluates trace callbacks in the context where the
    /// traced operation occurred.  The entry is disabled while its own
    /// callback runs (C's re-entrancy rule), a nested fire returning ok.
    pub(crate) fn run_cmd_trace_callback(
        &mut self,
        entry: &CmdTraceEntry,
        args: &[Value],
    ) -> Completion<Value> {
        if entry.firing.get() {
            return ok(Value::empty());
        }
        entry.firing.set(true);
        let mut script = entry.callback.clone();
        for a in args {
            script.push(' ');
            script.push_str(&tcl_syntax::list::list_element(&a.to_str()));
        }
        // C's `INTERP_TRACE_IN_PROGRESS`: no interp-wide trace fires for a
        // command dispatched *by* this callback (saved/restored, not merely
        // set — a callback can itself be dispatched from inside another
        // callback's evaluation only if re-entrant nesting is legitimate, but
        // the common case is one level; save-restore keeps either correct).
        let saved = self.trace_in_progress.get();
        self.trace_in_progress.set(true);
        let res = match self.eval_source(&script) {
            Ok(c) => c,
            Err(e) => err(e.message),
        };
        self.trace_in_progress.set(saved);
        entry.firing.set(false);
        res
    }

    /// Fire the `rename`/`delete` command traces of `key` as
    /// `callback oldName newName op` — names fully qualified (tclsh-pinned:
    /// `::victim ::victim2 rename`; a delete passes `{}` for the new name).
    /// Callback errors are ignored (C: "Any errors in these traces are
    /// ignored").
    fn fire_command_traces_for(&mut self, key: &CommandSidecarKey, new_display: &str, op: &str) {
        // C's `INTERP_TRACE_IN_PROGRESS`: a rename/delete triggered by a
        // trace callback's own body does not itself fire further traces.
        if self.cmd_traces.is_empty() || self.trace_in_progress.get() {
            return;
        }
        let Some(entries) = self.cmd_traces.get(key).cloned() else {
            return;
        };
        let old_display = format!("::{}", key.name());
        for entry in entries {
            if entry.ops.iter().any(|o| o == op) {
                let _ = self.run_cmd_trace_callback(
                    &entry,
                    &[
                        Value::string(old_display.clone()),
                        Value::string(new_display),
                        Value::string(op),
                    ],
                );
            }
        }
    }

    /// A command at `key` is about to be deleted or overwritten: fire its
    /// `delete` traces (tclsh-pinned: redefining a traced proc fires them
    /// too) and drop every trace registered on it.
    pub(crate) fn on_command_removed(&mut self, key: &str) {
        self.on_command_removed_for(&CommandSidecarKey::visible(key));
    }

    fn on_command_removed_for(&mut self, key: &CommandSidecarKey) {
        // A key is a location, not an eternal command identity.  Do this
        // before delete callbacks: they may create, hide, expose, or rename a
        // replacement binding with the same key while this operation is still
        // in flight.  That replacement must not inherit the old handle.
        self.detach_active_sidecars(key);
        let mut removed_trace = false;
        if !self.cmd_traces.is_empty() {
            self.fire_command_traces_for(key, "", "delete");
            removed_trace |= self.cmd_traces.remove(key).is_some();
        }
        if !self.exec_traces.is_empty()
            && let Some(removed) = self.exec_traces.remove(key)
        {
            removed_trace = true;
            // Dropping a step-capable trace this way (delete/overwrite,
            // not `trace remove`) needs the same epoch bump.
            if removed.iter().any(|t| is_step_capable(&t.ops)) {
                self.bump_trace_deopt_epoch();
            }
        }
        if removed_trace {
            self.invalidate_guard_domain(GuardDomain::CommandTrace);
        }
    }

    /// A command moved `old_key` → `new_key` (`rename`): fire its `rename`
    /// traces with both fully-qualified names, then move every trace with it
    /// (tclsh-pinned: an execution trace keeps firing under the new name).
    pub(crate) fn on_command_renamed_traces(&mut self, old_key: &str, new_key: &str) {
        let old_key = CommandSidecarKey::visible(old_key);
        let new_key = CommandSidecarKey::visible(new_key);
        self.relocate_active_sidecars(&old_key, &new_key);
        let mut moved_trace = false;
        if !self.cmd_traces.is_empty() {
            self.fire_command_traces_for(&old_key, &format!("::{}", new_key.name()), "rename");
            if let Some(entries) = self.cmd_traces.remove(&old_key) {
                self.cmd_traces.insert(new_key.clone(), entries);
                moved_trace = true;
            }
        }
        if !self.exec_traces.is_empty()
            && let Some(entries) = self.exec_traces.remove(&old_key)
        {
            self.exec_traces.insert(new_key, entries);
            moved_trace = true;
        }
        if moved_trace {
            self.invalidate_guard_domain(GuardDomain::CommandTrace);
        }
    }

    /// Move trace registrations with a command through the hidden table.  A
    /// hide/expose is not a Tcl rename, so no callback is fired.
    fn move_command_traces(&mut self, old_key: &CommandSidecarKey, new_key: CommandSidecarKey) {
        self.relocate_active_sidecars(old_key, &new_key);
        let mut moved = false;
        if let Some(entries) = self.cmd_traces.remove(old_key) {
            self.cmd_traces.insert(new_key.clone(), entries);
            moved = true;
        }
        if let Some(entries) = self.exec_traces.remove(old_key) {
            self.exec_traces.insert(new_key, entries);
            moved = true;
        }
        if moved {
            self.invalidate_guard_domain(GuardDomain::CommandTrace);
        }
    }

    /// Create an active identity that follows this command through every
    /// visible/hidden/renamed sidecar relocation.
    pub(crate) fn active_sidecar(&mut self, key: CommandSidecarKey) -> CommandSidecarHandle {
        self.active_sidecar_handles
            .retain(|handle| handle.strong_count() != 0);
        let cell = Rc::new(RefCell::new(Some(key)));
        self.active_sidecar_handles.push(Rc::downgrade(&cell));
        CommandSidecarHandle(cell)
    }

    fn relocate_active_sidecars(
        &mut self,
        old_key: &CommandSidecarKey,
        new_key: &CommandSidecarKey,
    ) {
        self.active_sidecar_handles.retain(|weak| {
            let Some(handle) = weak.upgrade() else {
                return false;
            };
            if handle.borrow().as_ref() == Some(old_key) {
                *handle.borrow_mut() = Some(new_key.clone());
            }
            true
        });
    }

    /// Disconnect all active users of a deleted command binding.  Retaining
    /// the weak cells is harmless; they are pruned when the in-flight calls
    /// complete, while their `None` identity prevents later relocation.
    pub(crate) fn detach_active_sidecars(&mut self, key: &CommandSidecarKey) {
        self.active_sidecar_handles.retain(|weak| {
            let Some(handle) = weak.upgrade() else {
                return false;
            };
            if handle.borrow().as_ref() == Some(key) {
                *handle.borrow_mut() = None;
            }
            true
        });
    }

    /// The traces on `name` as `(ops, command)` pairs (newest first), for
    /// `trace info variable`.
    pub(crate) fn var_trace_info(&self, name: &str) -> Vec<(Vec<String>, String)> {
        let key = self.trace_key(name);
        self.var_traces.get(&key).map_or_else(Vec::new, |list| {
            list.iter()
                .rev()
                .map(|t| (t.ops.clone(), t.command.clone()))
                .collect()
        })
    }

    /// Fire the variable traces for `name` on operation `op`, running each
    /// callback as `command name1 name2 op`. A read/write callback error aborts
    /// the access (`can't read`/`can't set "name": …`); unset errors are
    /// ignored. Re-entrant firing of the same variable+op is suppressed.
    pub(crate) fn fire_var_traces(
        &mut self,
        name: &str,
        op: &str,
    ) -> Result<(), Completion<Value>> {
        if self.var_traces.is_empty() {
            return Ok(());
        }
        let (base, elem) = elem_ref(name).map_or_else(
            || (name.to_string(), String::new()),
            |(b, k)| (b.to_string(), k.to_string()),
        );
        // Tcl suppresses *all* of a variable's traces while any one of them is
        // being handled (the documented "traces are disabled during the
        // handling of other traces" behaviour — see tcltest's outputChannel
        // notes). The unit of suppression is the whole variable (every array
        // element, every operation), so a read trace that writes the same
        // variable won't re-enter its write trace, and a whole-array read trace
        // fires only once per top-level access rather than per element.
        let (base_lvl, base_nm) = self.locate(&base);
        let active_key = format!("{base_lvl}\u{0}{base_nm}");
        if self.active_traces.contains(&active_key) {
            return Ok(());
        }
        self.active_traces.insert(active_key.clone());
        let r = self.fire_var_traces_inner(name, op, &base, &elem);
        self.active_traces.remove(&active_key);
        r
    }

    /// Inner firing loop for [`Self::fire_var_traces`]: run the element-specific
    /// traces, then the whole-array traces, with the variable already marked
    /// active by the caller.
    fn fire_var_traces_inner(
        &mut self,
        name: &str,
        op: &str,
        base: &str,
        elem: &str,
    ) -> Result<(), Completion<Value>> {
        // Fire the element-specific traces, then the whole-array traces (for an
        // element write a trace on the base array also fires).
        let mut keys = vec![self.trace_key(name)];
        if elem_ref(name).is_some() {
            let (lvl, nm) = self.locate(base);
            keys.push(format!("{lvl}\u{0}{nm}"));
        }
        for key in keys {
            let Some(traces) = self.var_traces.get(&key).cloned() else {
                continue;
            };
            for tr in traces.iter().rev() {
                if !tr.ops.iter().any(|o| o == op) {
                    continue;
                }
                let script = format!(
                    "{} {} {} {}",
                    tr.command,
                    tcl_brace(base),
                    tcl_brace(elem),
                    op
                );
                let r = self.eval_source(&script);
                let failed = match r {
                    Ok(c) if c.code.is_ok() => None,
                    Ok(c) => Some(c.result.to_str().to_string()),
                    Err(e) => Some(e.message),
                };
                if let Some(msg) = failed {
                    match op {
                        "write" | "read" => {
                            // C's `TclCallVarTraces` logs a `(write|read trace
                            // on "name")` frame, then clears ERR_ALREADY_LOGGED
                            // so the command that triggered the trace logs its
                            // own `invoked from within` frame as the error
                            // unwinds (set-2.4 / set-4.4).
                            self.append_var_trace_frame(op, name);
                            let verb = if op == "write" { "set" } else { "read" };
                            return Err(err(format!("can't {verb} \"{name}\": {msg}")));
                        }
                        _ => {} // unset trace errors are ignored
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn module_proc(&self, qname: &str) -> Option<Rc<FunctionAsm>> {
        self.module_procs.get(qname).cloned()
    }

    /// Compile a proc body at runtime (for `proc` with a dynamically-built
    /// body that wasn't pre-compiled into a module). The body is compiled as a
    /// script; its parameters resolve through the call frame (`loadStk`), so a
    /// top-level compilation runs correctly as a proc body. Any procs the body
    /// itself defines are merged into the registry.
    pub(crate) fn compile_dynamic_body(&mut self, src: &str) -> Option<Rc<FunctionAsm>> {
        let module = self
            .compiler
            .as_ref()?
            .compile_for_profile(src, self.dialect_profile)
            .ok()?;
        self.validate_module_profile(&module).ok()?;
        self.merge_procs(&module.procedures);
        Some(Rc::new(module.top_level))
    }

    /// Enforce the bytecode artifact's exact profile at every consumption
    /// boundary.  The permissive fallback profile is still a real compile
    /// target: bytecode lowered with its all-Tcl registry can contain an
    /// intrinsic unavailable in a named release, so it is only executable by
    /// a fallback-profile VM.
    pub(crate) fn validate_module_profile(&self, module: &ModuleAsm) -> Result<(), TclError> {
        if std::ptr::eq(module.profile, self.dialect_profile) {
            Ok(())
        } else {
            Err(TclError::new(format!(
                "bytecode compiled for dialect profile {} cannot run under {}",
                module.profile.name, self.dialect_profile.name
            )))
        }
    }

    /// Merge a module's pre-compiled proc bodies into the registry.
    pub(crate) fn merge_procs(&mut self, procs: &HashMap<String, FunctionAsm>) {
        for (qname, asm) in procs {
            self.module_procs
                .entry(qname.clone())
                .or_insert_with(|| Rc::new(asm.clone()));
        }
    }

    /// Ensure `proc`'s compiled body matches the interp's current trace-deopt
    /// and dialect-profile generations, recompiling `body_src` — trace-visible
    /// ([`CompileService::compile_traced`]) or fast
    /// ([`CompileService::compile`]), per [`Self::step_trace_active`] —
    /// when it doesn't. This is the general recompile-on-demand mechanism
    /// behind issue #946 fault 3: C Tcl forces a step-traced proc "out of
    /// bytecode" (`DONT_COMPILE_CMDS_INLINE`) on its next entry after a
    /// step-capable trace is added anywhere, and reverts it the same way once
    /// the last one is removed. Returns `proc` unchanged when it is already
    /// current, no compiler is available, or the recompile itself errors (the
    /// existing body — which already compiled successfully once — keeps
    /// running rather than failing the call over a trace-visibility nicety).
    pub(crate) fn ensure_proc_traced(&mut self, proc: Rc<ProcDef>) -> Rc<ProcDef> {
        let want_epoch = self.trace_deopt_epoch();
        let want_profile = self.profile_generation;
        if proc.compiled_epoch == want_epoch && proc.compiled_profile_generation == want_profile {
            return proc;
        }
        let Some(svc) = self.compiler.clone() else {
            return proc;
        };
        let src = proc.body_src.to_str();
        let recompiled = if self.step_trace_active() {
            svc.compile_traced_for_profile(&src, self.dialect_profile)
        } else {
            svc.compile_for_profile(&src, self.dialect_profile)
        };
        let Ok(module) = recompiled else {
            return proc;
        };
        if self.validate_module_profile(&module).is_err() {
            return proc;
        }
        self.merge_procs(&module.procedures);
        let mut fresh = (*proc).clone();
        fresh.body = Rc::new(module.top_level);
        fresh.compiled_epoch = want_epoch;
        fresh.compiled_profile_generation = want_profile;
        Rc::new(fresh)
    }

    /// Refresh a stored procedure and memoise it in the command domain from
    /// which this invocation was resolved. Hidden and visible commands may
    /// share the same spelling, so publishing by `ProcDef::name` would leak a
    /// hidden proc back into the visible table (and overwrite a replacement).
    pub(crate) fn ensure_proc_ready_in(
        &mut self,
        proc: Rc<ProcDef>,
        sidecar: Option<&CommandSidecarKey>,
        sidecar_handle: Option<&CommandSidecarHandle>,
    ) -> Rc<ProcDef> {
        if proc.compiled_epoch == self.trace_deopt_epoch()
            && proc.compiled_profile_generation == self.profile_generation
        {
            return proc;
        }
        let fresh = self.ensure_proc_traced(proc);
        // A traced invocation's handle follows the command through hide/expose
        // and becomes detached when a callback deletes it.  Never fall back to
        // the pre-callback key in that case: doing so would resurrect the stale
        // proc or overwrite a replacement installed by the callback.
        let memo_sidecar = match sidecar_handle {
            Some(handle) => handle.key(),
            None => sidecar.cloned(),
        };
        match memo_sidecar.as_ref() {
            Some(CommandSidecarKey::Hidden(token)) => {
                self.hidden_commands
                    .insert(token.clone(), Command::Proc(Rc::clone(&fresh)));
            }
            Some(CommandSidecarKey::Visible(name)) => {
                self.commands
                    .insert(name.clone(), Command::Proc(Rc::clone(&fresh)));
            }
            None => {
                self.commands
                    .insert(fresh.name.clone(), Command::Proc(Rc::clone(&fresh)));
            }
        }
        fresh
    }

    // -- frames --

    pub(crate) fn current_level(&self) -> usize {
        self.frames.len() - 1
    }

    /// Whether the current frame is a **procedure** activation.
    ///
    /// A proc frame carries its name; the global frame and a `namespace eval`
    /// body do not (the latter runs in the current frame and pushes none). This
    /// is the condition `Tcl_GlobalObjCmd` tests before doing anything at all —
    /// outside a proc, `global` is a no-op (issue #1458's guard is scoped to it).
    pub(crate) fn in_proc_frame(&self) -> bool {
        self.frames.last().is_some_and(|f| f.proc_name.is_some())
    }

    pub(crate) fn push_call_frame(
        &mut self,
        proc_name: Option<String>,
        call_argv: Vec<Value>,
    ) -> usize {
        let level = self.frames.len();
        self.frames
            .push(CallFrame::new(level, ROOT_NS, proc_name, call_argv));
        self.recursion_depth += 1;
        level
    }

    pub(crate) fn pop_call_frame(&mut self) {
        if self.frames.len() > 1 {
            self.fire_frame_unset_traces();
            self.frames.pop();
            self.recursion_depth = self.recursion_depth.saturating_sub(1);
        }
    }

    /// Fire `unset` variable traces on the current frame's genuine locals just
    /// before the frame is destroyed — C Tcl unsets a call frame's locals when
    /// the frame is deleted, firing their unset traces (coroutine-4.1/4.2, and
    /// the general proc-return case). Links are skipped: they alias a variable
    /// owned by another frame, whose trace fires when *that* frame goes. The
    /// fired traces are then dropped, since their variables no longer exist.
    /// Guarded on `var_traces` being non-empty, so it is free in the common case.
    fn fire_frame_unset_traces(&mut self) {
        if self.var_traces.is_empty() {
            return;
        }
        let Some(frame) = self.frames.last() else {
            return;
        };
        let level = frame.level;
        // Genuine locals (scalars/arrays), sorted for a deterministic order.
        let mut names: Vec<String> = frame
            .locals
            .iter()
            .filter(|(_, l)| matches!(l, Local::Scalar(_) | Local::Array(_)))
            .map(|(n, _)| n.clone())
            .collect();
        names.sort();
        for nm in &names {
            // The frame is still on the stack, so the name resolves to this
            // level; C ignores errors from unset traces, as does `fire_var_traces`.
            let _ = self.fire_var_traces(nm, "unset");
        }
        // Drop every trace scoped to this frame level — those variables are gone.
        let prefix = format!("{level}\u{0}");
        self.var_traces.retain(|k, _| !k.starts_with(&prefix));
    }

    /// Fire `unset` traces on a suspended coroutine's parked locals as the
    /// coroutine is torn down: its frozen frames are about to vanish, so swap the
    /// parked flow in, unwind it frame by frame (each `pop_call_frame` fires that
    /// frame's unset traces), then swap the caller's flow back (`parked` is left
    /// emptied for the caller to drop). Matches C Tcl unsetting a deleted
    /// coroutine's variables (coroutine-4.3). Free when no traces are registered.
    pub(crate) fn fire_parked_unset_traces(&mut self, parked: &mut ParkedFlow) {
        if self.var_traces.is_empty() {
            return;
        }
        self.swap_flow(parked);
        while self.frames.len() > 1 {
            self.pop_call_frame();
            self.pop_ns();
        }
        self.swap_flow(parked);
    }

    /// Push a non-proc call frame for a `namespace eval`/`inscope` body running
    /// in namespace `ns` (canonical, no leading `::`). Like a proc call this is a
    /// real frame — `info level` counts it and `uplevel`/`upvar` can target it —
    /// but it is marked [`ns_eval`](CallFrame::ns_eval) so an unqualified variable
    /// in its body resolves to a namespace variable rather than a frame local.
    /// `call_argv` is the invoking command words (for `info level N`).
    pub(crate) fn push_ns_eval_frame(&mut self, ns: &str, call_argv: Vec<Value>) -> usize {
        let level = self.frames.len();
        let mut frame = CallFrame::new(level, ROOT_NS, None, call_argv);
        frame.ns_eval = Some(ns.to_owned());
        self.frames.push(frame);
        self.recursion_depth += 1;
        level
    }

    /// The current call-nesting depth (proc recursion bound).
    pub(crate) fn recursion_depth(&self) -> usize {
        self.recursion_depth
    }

    /// Enter one level of `cmd_control.rs`'s runtime-command fallback
    /// recursion — see [`CONTROL_FALLBACK_DEPTH_LIMIT`]'s doc comment
    /// (issue #996). Checked before incrementing; pair with
    /// [`Self::exit_control_fallback`] (even on an early-error return) to
    /// keep the counter balanced.
    pub(crate) fn enter_control_fallback(&mut self) -> Result<(), Completion<Value>> {
        if CONTROL_FALLBACK_DEPTH_LIMIT
            .exceeded(u32::try_from(self.control_fallback_depth + 1).unwrap_or(u32::MAX))
        {
            return Err(err("too many nested evaluations (infinite loop?)"));
        }
        self.control_fallback_depth += 1;
        Ok(())
    }

    /// Leave one level of `cmd_control.rs`'s runtime-command fallback
    /// recursion — see [`Self::enter_control_fallback`].
    pub(crate) fn exit_control_fallback(&mut self) {
        self.control_fallback_depth = self.control_fallback_depth.saturating_sub(1);
    }

    /// Enter one level of `TclOO` method-dispatch recursion — see
    /// [`OO_DISPATCH_DEPTH_LIMIT`]'s doc comment (issue #996). Checked
    /// before incrementing; pair with [`Self::exit_oo_dispatch`] (even on
    /// an early-error return) to keep the counter balanced.
    pub(crate) fn enter_oo_dispatch(&mut self) -> Result<(), Completion<Value>> {
        if OO_DISPATCH_DEPTH_LIMIT
            .exceeded(u32::try_from(self.oo_dispatch_depth + 1).unwrap_or(u32::MAX))
        {
            return Err(err("too many nested evaluations (infinite loop?)"));
        }
        self.oo_dispatch_depth += 1;
        Ok(())
    }

    /// Leave one level of `TclOO` method-dispatch recursion — see
    /// [`Self::enter_oo_dispatch`].
    pub(crate) fn exit_oo_dispatch(&mut self) {
        self.oo_dispatch_depth = self.oo_dispatch_depth.saturating_sub(1);
    }

    /// This interp's recursion bound (`interp recursionlimit`).
    pub(crate) fn recursion_limit(&self) -> usize {
        self.recursion_limit
    }

    /// `interp recursionlimit` get / set on this interp. `newlimit` is the
    /// optional new-limit string; returns the resulting limit or the error
    /// message the caller should raise.
    pub(crate) fn recursion_limit_apply(&mut self, newlimit: Option<&str>) -> Result<i64, String> {
        match newlimit {
            None => Ok(i64::try_from(self.recursion_limit).unwrap_or(i64::MAX)),
            Some(s) => {
                let n = parse_recursion_limit(s)?;
                if n <= 0 {
                    return Err("recursion limit must be > 0".to_string());
                }
                self.recursion_limit = usize::try_from(n).unwrap_or(usize::MAX);
                Ok(n)
            }
        }
    }

    /// Evaluate `src` as if `target` were the current call frame (`uplevel`).
    /// The frames above `target` are set aside for the duration and restored
    /// afterwards, so the script's variable references and `info level` resolve
    /// against the target activation. A `Return` completion is passed through to
    /// the calling frame (as the reference `uplevel` does).
    /// Push the path of a file being `source`d; the matching [`Self::pop_script`]
    /// restores the previous one. Drives `info script`.
    pub(crate) fn push_script(&mut self, path: String) {
        self.script_stack.push(path);
    }

    /// Pop the current `source` path (see [`Self::push_script`]).
    pub(crate) fn pop_script(&mut self) {
        self.script_stack.pop();
    }

    /// The path of the file currently being `source`d (`info script`); empty
    /// when evaluation is not inside a `source`.
    pub(crate) fn current_script(&self) -> &str {
        self.script_stack.last().map_or("", String::as_str)
    }

    pub(crate) fn eval_at_level(&mut self, target: usize, src: &str) -> Completion<Value> {
        if target >= self.frames.len() {
            return err(format!("bad level \"{target}\""));
        }
        let saved = self.frames.split_off(target + 1);
        // The namespace stack is kept aligned 1:1 with the call-frame stack (every
        // proc call and `namespace eval` pushes one of each), so the target frame's
        // namespace is `ns_stack[target]`. Set it aside with the frames so the
        // uplevel'd script resolves commands/variables in the target frame's
        // namespace — what makes `uplevel 1`/tcltest's body eval inside a
        // `namespace eval` reach that namespace's procs and variables.
        let ns_cut = (target + 1).min(self.ns_stack.len());
        let saved_ns = self.ns_stack.split_off(ns_cut);
        let saved_depth = self.recursion_depth;
        let result = self.eval_source(src);
        // Restore any frames the script left in place, then re-attach the ones
        // we set aside (the script's own proc activations are already balanced).
        // `truncate` may drop frames the script left unbalanced (an error mid
        // proc), which `pop_call_frame` never saw — so reset the recursion depth
        // to its pre-eval value rather than leak it.
        self.frames.truncate(target + 1);
        self.frames.extend(saved);
        self.ns_stack.truncate(ns_cut);
        self.ns_stack.extend(saved_ns);
        self.recursion_depth = saved_depth;
        match result {
            Ok(c) => c,
            Err(e) => err(e.message),
        }
    }

    /// Exchange the live per-flow execution context with `p` — the coroutine
    /// context switch (`cmd_coro::resume`). Its own inverse: two calls restore
    /// the original. The **shared** global frame (`frames[0]`) and global
    /// namespace (`ns_stack[0]`) stay in place — only the supra-global tails move
    /// — so globals, `uplevel #0`, and `::x` stay coherent across flows and each
    /// coroutine roots at frame level 1. Everything else (error trace, script
    /// stack, OO call/def stacks) swaps wholesale. Registries, channels, commands,
    /// etc. are shared and untouched (they are not part of a flow).
    ///
    /// Modelled on [`Self::eval_at_level`]'s split-off/restore of `frames` +
    /// `ns_stack` + `recursion_depth`.
    pub(crate) fn swap_flow(&mut self, p: &mut ParkedFlow) {
        // frames / ns_stack: exchange the tail above the shared global entry.
        let mut ftail = self.frames.split_off(1);
        std::mem::swap(&mut ftail, &mut p.frames);
        self.frames.append(&mut ftail);
        let ns_cut = 1.min(self.ns_stack.len());
        let mut nstail = self.ns_stack.split_off(ns_cut);
        std::mem::swap(&mut nstail, &mut p.ns_stack);
        self.ns_stack.append(&mut nstail);
        // Scalars / stacks: plain exchange.
        std::mem::swap(&mut self.ns_script_frames, &mut p.ns_script_frames);
        std::mem::swap(&mut self.recursion_depth, &mut p.recursion_depth);
        std::mem::swap(&mut self.error_info, &mut p.error_info);
        std::mem::swap(&mut self.error_logged, &mut p.error_logged);
        std::mem::swap(&mut self.error_line, &mut p.error_line);
        std::mem::swap(&mut self.invoked_name, &mut p.invoked_name);
        std::mem::swap(&mut self.script_stack, &mut p.script_stack);
        self.oo.swap_exec(&mut p.oo);
    }

    /// The unqualified names of commands (or, with `procs_only`, just user
    /// procedures) defined **directly** in namespace `canonical` (unrooted; `""`
    /// = global) — the `Namespaces::commands_in`/`procs_in` enumeration, filtering
    /// the flat command map. Direct members only (`foo::sub::x` is not in `foo`).
    pub(crate) fn names_directly_in(&self, canonical: &str, procs_only: bool) -> Vec<String> {
        self.commands
            .iter()
            .filter(|(key, command)| self.builtin_command_visible_for_surface(key, command))
            .filter(|(_, c)| !procs_only || matches!(c, Command::Proc(_)))
            .filter_map(|(key, _)| direct_member_tail(key, canonical).map(str::to_owned))
            .collect()
    }

    /// The `ProcDef` for a user proc, if `name` resolves to one (`info body`/`args`).
    pub(crate) fn proc_def(&self, name: &str) -> Option<Rc<crate::command::ProcDef>> {
        match self.lookup_command(name) {
            Some(Command::Proc(p)) => Some(p),
            _ => None,
        }
    }

    /// Whether `name` is already a defined user proc — distinguishes a `proc`
    /// redefinition (which must recompile its body) from a first definition.
    pub(crate) fn is_proc_defined(&self, name: &str) -> bool {
        matches!(self.lookup_command(name), Some(Command::Proc(_)))
    }

    /// The invocation argv of the frame at absolute `level` (`info level N`).
    pub(crate) fn frame_argv(&self, level: usize) -> Option<Vec<Value>> {
        self.frames.get(level).map(|f| f.call_argv.clone())
    }

    /// The variable names of the current (active) frame — the `Frames::var_names`
    /// enumeration. Genuine locals (scalars, arrays) always; `upvar`/`global`/
    /// `variable` links iff `include_links` (`info vars` lists links by their
    /// local alias, `info locals` does not).
    pub(crate) fn frame_var_names(&self, include_links: bool) -> Vec<String> {
        self.frames.last().map_or_else(Vec::new, |f| {
            f.locals
                .iter()
                .filter(|(_, l)| include_links || matches!(l, Local::Scalar(_) | Local::Array(_)))
                .map(|(n, _)| n.clone())
                .collect()
        })
    }

    /// The variables defined **directly** in namespace `canonical` (unrooted; `""`
    /// = global) — the `Namespaces::vars_in` enumeration. Namespace variables live
    /// in the global frame keyed by their qualified name (`foo::v`), so this is the
    /// variable analogue of [`names_directly_in`](Self::names_directly_in): the
    /// global frame's genuine variables (scalars/arrays, not links) whose key is a
    /// direct member of `canonical`.
    pub(crate) fn vars_directly_in(&self, canonical: &str) -> Vec<String> {
        self.frames.first().map_or_else(Vec::new, |f| {
            f.locals
                .iter()
                // A namespace-scoped `Link` is a real cell in the namespace's
                // table — see `namespace_var_exists` for why C's
                // `CompiledLocal` exclusion does not apply to it.
                .filter(|(_, l)| {
                    matches!(l, Local::Scalar(_) | Local::Array(_) | Local::Link { .. })
                })
                .filter_map(|(key, _)| direct_member_tail(key, canonical).map(str::to_owned))
                .collect()
        })
    }

    /// Set a local directly in the current frame (proc argument binding).
    pub(crate) fn set_local(&mut self, name: &str, value: Value) {
        if let Some(f) = self.frames.last_mut() {
            f.locals.insert(name.to_owned(), Local::Scalar(value));
        }
    }

    /// Install a cross-frame link in the current frame (`upvar`/`global`).
    pub(crate) fn add_link(&mut self, local: &str, level: usize, target: &str) {
        if let Some(f) = self.frames.last_mut() {
            f.locals.insert(
                local.to_owned(),
                Local::Link {
                    level,
                    name: target.to_owned(),
                },
            );
        }
    }

    /// Install a link in the global frame keyed by `alias` (a canonical,
    /// namespace-qualified name). Used for namespace-level `upvar` aliases so
    /// they coincide with the `variable`-resolved namespace variable.
    pub(crate) fn add_global_link(&mut self, alias: &str, level: usize, target: &str) {
        if let Some(f) = self.frames.first_mut() {
            f.locals.insert(
                alias.to_owned(),
                Local::Link {
                    level,
                    name: target.to_owned(),
                },
            );
        }
    }

    /// Whether a `namespace eval`/`inscope` body is *directly* executing in the
    /// current frame. Returns `false` inside a proc called from such a body —
    /// a proc activation has its own scope where unqualified names are locals,
    /// not namespace variables. We test this by recording the frame depth at
    /// which each namespace script started and checking the innermost against
    /// the current depth.
    pub(crate) fn in_ns_script(&self) -> bool {
        self.ns_script_frames.last() == Some(&self.frames.len())
    }

    /// Enter/leave a `namespace eval`/`inscope` body (around its evaluation).
    /// The body runs in the current frame, so we remember that frame depth.
    pub(crate) fn enter_ns_script(&mut self) {
        let depth = self.frames.len();
        self.ns_script_frames.push(depth);
    }
    pub(crate) fn leave_ns_script(&mut self) {
        self.ns_script_frames.pop();
    }

    /// Resolve `name` to the (frame level, owning name) that actually owns it,
    /// following `upvar`/`global`/`variable` links. Any namespace-qualified name
    /// (containing `::`, including a plain `::global`) lives in the global frame
    /// keyed by its canonical name (leading `::` stripped) — this is where
    /// namespace variables (`tcltest::numTests`) are stored.
    fn locate(&self, name: &str) -> (usize, String) {
        self.locate_from(name, self.frames.len().saturating_sub(1))
    }

    /// Canonicalise a written qualified variable name using the shared Tcl
    /// naming owner.  Qualified relative names are rooted at the current
    /// namespace; absolute names retain their root.  `canonical_written_command`
    /// collapses colon runs (`a:::b` → `a::b`) and preserves a trailing run as
    /// the empty variable name (`foo:::` → `foo::`).
    fn canonical_var_name(&self, name: &str) -> String {
        if !tcl_syntax::naming::is_qualified(name.as_bytes()) {
            return name.to_owned();
        }
        tcl_syntax::naming::qualify(self.current_ns(), name)
    }

    /// Validate the namespace portion of a qualified variable write.  Tcl's
    /// `TclGetNamespaceForQualName` rejects a missing parent namespace rather
    /// than creating it implicitly.
    fn validate_var_parent(&self, name: &str) -> Result<(), Completion<Value>> {
        if !tcl_syntax::naming::is_qualified(name.as_bytes()) {
            return Ok(());
        }
        let rooted = self.canonical_var_name(name);
        let (parent, _) = tcl_syntax::naming::key_holder_and_tail(&rooted);
        let parent = parent.strip_prefix("::").unwrap_or(parent);
        if !self.namespace_exists(parent) {
            return Err(err(format!(
                "can't set \"{name}\": parent namespace doesn't exist"
            )));
        }
        Ok(())
    }

    /// Like [`Self::locate`] but begins link resolution at frame `start` (used
    /// by frame-addressed public storage operations). Written qualified names
    /// are canonicalised once at this boundary; link targets are already
    /// internal keys and must not be qualified again.
    fn locate_from(&self, name: &str, start: usize) -> (usize, String) {
        let canonical = self.canonical_var_name(name);
        let stripped = canonical.strip_prefix("::").unwrap_or(&canonical);
        let qualified = tcl_syntax::naming::is_qualified(canonical.as_bytes());
        let level = if qualified { 0 } else { start };
        let key = if qualified { stripped } else { name };
        self.locate_key_from(key, level)
    }

    /// Follow links from an already-canonical internal variable key. Unlike
    /// [`Self::locate_from`], this never interprets a relative qualified key in
    /// the current namespace a second time.
    fn locate_key_from(&self, name: &str, start: usize) -> (usize, String) {
        let mut level = start;
        let mut nm = name.to_owned();
        for _ in 0..64 {
            match self.frames.get(level).and_then(|f| f.locals.get(&nm)) {
                Some(Local::Link {
                    level: tl,
                    name: tn,
                }) => {
                    level = *tl;
                    nm.clone_from(tn);
                }
                _ => break,
            }
        }
        // A bare name landing on a `namespace eval` body frame (no genuine local
        // there) is a *namespace* variable: redirect it to `ns::name` in the
        // global frame, where namespace variables live. This mirrors what
        // `ns_var_fallback` does for the current frame, but also covers an
        // `upvar`/`uplevel` link that reaches a namespace-eval frame — so a proc's
        // `upvar 1 v` into a `namespace eval` body resolves the namespace variable,
        // and a plain `set x` at namespace-script level *creates* `ns::x`.
        if !nm.contains("::")
            && let Some(f) = self.frames.get(level)
            && let Some(ns) = &f.ns_eval
            && !ns.is_empty()
            && !f.locals.contains_key(&nm)
        {
            let qualified = format!("{ns}::{nm}");
            // Tcl 8.x namespace-scope fallback (M11): when the namespace has
            // no such variable but the global namespace does, the bare name
            // resolves to the GLOBAL variable — reads and writes both (a
            // `variable` declaration installs a link above, so a declared
            // name never reaches this redirect and correctly blocks the
            // fallback).  9.0 always binds in the namespace (TIP 278).
            if self.ns_fallback_targets_global(&qualified, &nm) {
                return (0, nm);
            }
            return (0, qualified);
        }
        (level, nm)
    }

    /// Read a scalar (following links). A link may resolve to an array element
    /// name (`upvar 0 arr(key) alias`), in which case the element is read.
    #[must_use]
    pub fn get_var(&self, name: &str) -> Option<Value> {
        let resolved = self.ns_var_fallback(name);
        let name = resolved.as_deref().unwrap_or(name);
        let (lvl, nm) = self.locate(name);
        if let Some((base, key)) = elem_ref(&nm) {
            // The base may itself be a link (`variable`/`upvar` to a namespace
            // array), so resolve it onward from the frame it landed on.
            let (blvl, bnm) = self.locate_key_from(base, lvl);
            return match self.frames.get(blvl)?.locals.get(&bnm) {
                Some(Local::Array(m)) => m.get(key).cloned(),
                _ => None,
            };
        }
        match self.frames.get(lvl)?.locals.get(&nm) {
            Some(Local::Scalar(v)) => Some(v.clone()),
            _ => None,
        }
    }

    /// Write a scalar with no trace firing (frame argument binding, rollback).
    /// A link resolving to an array element name writes that element.
    fn write_scalar_raw(&mut self, name: &str, value: Value) {
        let resolved = self.ns_var_fallback(name);
        let name = resolved.as_deref().unwrap_or(name);
        let (lvl, nm) = self.locate(name);
        if let Some((base, key)) = elem_ref(&nm) {
            // Resolve the array base onward (it may be a link to a namespace
            // array) before writing the element.
            let key = key.to_owned();
            let (blvl, bnm) = self.locate_key_from(base, lvl);
            if let Some(f) = self.frames.get_mut(blvl) {
                match f.locals.get_mut(&bnm) {
                    Some(Local::Array(m)) => {
                        m.insert(key, value);
                    }
                    Some(Local::Undefined) => {
                        f.locals.insert(bnm, Local::Scalar(value));
                    }
                    Some(_) => {}
                    None => {
                        let mut m = BTreeMap::new();
                        m.insert(key, value);
                        f.locals.insert(bnm, Local::Array(m));
                    }
                }
            }
            return;
        }
        if let Some(f) = self.frames.get_mut(lvl) {
            f.locals.insert(nm, Local::Scalar(value));
        }
    }

    /// Whether a scalar write to `name` would land on an existing array variable
    /// — a `can't set "x": variable is array` error rather than a silent
    /// overwrite (the resolution mirrors [`write_scalar_raw`](Self::write_scalar_raw)).
    fn scalar_write_hits_array(&self, name: &str) -> bool {
        let resolved = self.ns_var_fallback(name);
        let name = resolved.as_deref().unwrap_or(name);
        let (lvl, nm) = self.locate(name);
        if elem_ref(&nm).is_some() {
            return false; // an element write resolves the array base separately
        }
        matches!(
            self.frames.get(lvl).and_then(|f| f.locals.get(&nm)),
            Some(Local::Array(_))
        )
    }

    /// Resolve `name` to the `(frame, local-name)` owning its cell, mirroring
    /// the scalar write path — the key for constant tracking (TIP 677).
    fn const_slot(&self, name: &str) -> (usize, String) {
        let resolved = self.ns_var_fallback(name);
        self.locate(resolved.as_deref().unwrap_or(name))
    }

    /// Mark `name`'s scalar cell as a `const` (immutable).
    pub(crate) fn mark_constant(&mut self, name: &str) {
        let (lvl, nm) = self.const_slot(name);
        if let Some(f) = self.frames.get_mut(lvl) {
            f.consts.insert(nm);
        }
    }

    /// Whether `name` resolves to a `const` cell.
    pub(crate) fn is_constant(&self, name: &str) -> bool {
        let (lvl, nm) = self.const_slot(name);
        self.frames.get(lvl).is_some_and(|f| f.consts.contains(&nm))
    }

    /// The `const` names visible in the current frame matching `info consts`.
    pub(crate) fn constant_names(&self) -> Vec<String> {
        let lvl = self.frames.len().saturating_sub(1);
        self.frames
            .get(lvl)
            .map(|f| f.consts.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Write a scalar, firing `write` traces afterwards (the old value is
    /// restored if a trace callback aborts the write).
    pub fn set_var(&mut self, name: &str, value: Value) -> Result<(), Completion<Value>> {
        self.validate_var_parent(name)?;
        if self.is_constant(name) {
            return Err(err(format!("can't set \"{name}\": variable is a constant")));
        }
        if self.scalar_write_hits_array(name) {
            return Err(err(format!("can't set \"{name}\": variable is array")));
        }
        if self.var_traces.is_empty() {
            self.write_scalar_raw(name, value);
            return Ok(());
        }
        let old = self.get_var(name);
        self.write_scalar_raw(name, value);
        if let Err(e) = self.fire_var_traces(name, "write") {
            if let Some(o) = old {
                self.write_scalar_raw(name, o);
            } else {
                let (lvl, nm) = self.locate(name);
                if let Some(f) = self.frames.get_mut(lvl) {
                    f.locals.remove(&nm);
                }
            }
            return Err(e);
        }
        Ok(())
    }

    /// Remove a scalar; returns whether it existed.
    pub fn unset_var(&mut self, name: &str) -> bool {
        // Unset traces fire before removal; their errors are ignored.
        let _ = self.fire_var_traces(name, "unset");
        let (lvl, nm) = self.locate(name);
        let existed = self
            .frames
            .get_mut(lvl)
            .is_some_and(|f| f.locals.remove(&nm).is_some());
        // A variable's traces are dropped when it is unset.
        if existed {
            let key = self.trace_key(name);
            if self.var_traces.remove(&key).is_some() {
                self.invalidate_guard_domain(GuardDomain::VariableTrace);
            }
        }
        existed
    }

    /// Unset a single variable by name, the shared core of the `unset` command
    /// and the unset opcodes. An `a(k)` array element is removed via
    /// [`array_unset_elem`](Self::array_unset_elem) (which splits the base/key);
    /// a scalar/array variable is removed via [`unset_var`](Self::unset_var).
    /// When `complain` is set and the variable did not exist, this returns the
    /// Tcl `can't unset "name": no such variable` error (array-element removal
    /// never complains, matching C Tcl / `cmd_unset`).
    pub(crate) fn unset_one(
        &mut self,
        name: &str,
        complain: bool,
    ) -> Result<(), Completion<Value>> {
        if let Some((array, key)) = elem_ref(name) {
            self.array_unset_elem(array, key);
            return Ok(());
        }
        // A constant cannot be unset; `-nocomplain` leaves it intact (var-26.12).
        if self.is_constant(name) {
            if complain {
                return Err(err(format!(
                    "can't unset \"{name}\": variable is a constant"
                )));
            }
            return Ok(());
        }
        if !self.unset_var(name) && complain {
            return Err(err(format!("can't unset \"{name}\": no such variable")));
        }
        Ok(())
    }

    // frame-addressed storage (the `VarStore` `FrameId`-honouring path)
    //
    // These resolve `name` starting from an *explicit* frame (following links),
    // touching only storage: no current-eval-context namespace fallback and no
    // trace firing (both are current-frame concerns). The `VarStore` impl uses
    // them only for a non-current `FrameId`; a `FrameId` equal to the current
    // frame delegates to the full inherent accessors above, so the common case
    // keeps its exact behaviour (fallback + traces).

    /// Frame-addressed scalar read (the storage half of [`get_var`](Self::get_var)).
    pub(crate) fn get_var_from(&self, start: usize, name: &str) -> Option<Value> {
        let (lvl, nm) = self.locate_from(name, start);
        if let Some((base, key)) = elem_ref(&nm) {
            let (blvl, bnm) = self.locate_key_from(base, lvl);
            return match self.frames.get(blvl)?.locals.get(&bnm) {
                Some(Local::Array(m)) => m.get(key).cloned(),
                _ => None,
            };
        }
        match self.frames.get(lvl)?.locals.get(&nm) {
            Some(Local::Scalar(v)) => Some(v.clone()),
            _ => None,
        }
    }

    /// Frame-addressed scalar write (the storage half of
    /// [`write_scalar_raw`](Self::write_scalar_raw)).
    pub(crate) fn write_scalar_from(&mut self, start: usize, name: &str, value: Value) {
        let (lvl, nm) = self.locate_from(name, start);
        if let Some((base, key)) = elem_ref(&nm) {
            let key = key.to_owned();
            let (blvl, bnm) = self.locate_key_from(base, lvl);
            if let Some(f) = self.frames.get_mut(blvl) {
                match f.locals.get_mut(&bnm) {
                    Some(Local::Array(m)) => {
                        m.insert(key, value);
                    }
                    Some(Local::Undefined) => {
                        f.locals.insert(bnm, Local::Scalar(value));
                    }
                    Some(_) => {}
                    None => {
                        let mut m = BTreeMap::new();
                        m.insert(key, value);
                        f.locals.insert(bnm, Local::Array(m));
                    }
                }
            }
            return;
        }
        if let Some(f) = self.frames.get_mut(lvl) {
            f.locals.insert(nm, Local::Scalar(value));
        }
    }

    /// Frame-addressed scalar existence (the storage half of
    /// [`var_exists`](Self::var_exists)).
    pub(crate) fn exists_from(&self, start: usize, name: &str) -> bool {
        let (lvl, nm) = self.locate_from(name, start);
        self.frames
            .get(lvl)
            .is_some_and(|f| matches!(f.locals.get(&nm), Some(Local::Scalar(_))))
    }

    /// Frame-addressed unset (storage only — no unset-trace firing).
    pub(crate) fn unset_from(&mut self, start: usize, name: &str) -> bool {
        let (lvl, nm) = self.locate_from(name, start);
        self.frames
            .get_mut(lvl)
            .is_some_and(|f| f.locals.remove(&nm).is_some())
    }

    /// Whether a scalar or array variable named `name` exists (`info exists`).
    pub(crate) fn has_var(&self, name: &str) -> bool {
        let (lvl, nm) = self.locate(name);
        matches!(
            self.frames.get(lvl).and_then(|f| f.locals.get(&nm)),
            Some(Local::Scalar(_) | Local::Array(_))
        )
    }

    /// Whether `name` resolves to an array variable (the `set a` array/scalar
    /// diagnostic; `array exists`).
    pub(crate) fn var_is_array(&self, name: &str) -> bool {
        let resolved = self.ns_var_fallback(name);
        let lookup = resolved.as_deref().unwrap_or(name);
        let (lvl, nm) = self.locate(lookup);
        matches!(
            self.frames.get(lvl).and_then(|f| f.locals.get(&nm)),
            Some(Local::Array(_))
        )
    }

    /// C's three-way read-miss message (`tclVar.c`): a scalar read of an array
    /// (`variable is array`), a missing element of an existing array (`no such
    /// element in array`), an existing scalar accessed with an index (`variable
    /// isn't array`), or a wholly missing variable (`no such variable`).
    pub(crate) fn read_miss_msg(&self, name: &str) -> String {
        let (base, has_idx) = elem_ref(name).map_or((name, false), |(b, _)| (b, true));
        let what = if self.var_is_array(base) {
            if has_idx {
                "no such element in array"
            } else {
                "variable is array"
            }
        } else if has_idx && self.has_var(base) {
            "variable isn't array"
        } else {
            "no such variable"
        };
        format!("can't read \"{name}\": {what}")
    }

    /// Ensure `name` is an array (creating an empty one if unset) — `array set
    /// name {}` with an empty value list still materialises the array (C's
    /// `TclArraySet`). A scalar `name` errors `variable isn't array`; this is the
    /// empty-list path, which C words under the command (`can't array set "n"`),
    /// unlike the per-element write that names `n(key)`.
    pub(crate) fn ensure_array(&mut self, name: &str) -> Result<(), Completion<Value>> {
        let resolved = self.ns_var_fallback(name);
        let lookup = resolved.as_deref().unwrap_or(name).to_string();
        let (lvl, nm) = self.locate(&lookup);
        if let Some(f) = self.frames.get_mut(lvl) {
            match f.locals.get(&nm) {
                Some(Local::Array(_) | Local::Link { .. }) => {}
                Some(Local::Undefined) | None => {
                    f.locals.insert(nm, Local::Array(BTreeMap::new()));
                }
                Some(Local::Scalar(_)) => {
                    return Err(err(format!(
                        "can't array set \"{name}\": variable isn't array"
                    )));
                }
            }
        }
        Ok(())
    }

    /// Materialise the unresolved variable cell used by `trace add variable`.
    /// This is deliberately not a write: Tcl keeps the cell unset, so `info
    /// exists` remains false until the caller actually stores a value.
    pub(crate) fn ensure_trace_variable(&mut self, name: &str) -> Result<(), Completion<Value>> {
        if let Some((base, _)) = elem_ref(name) {
            let base = self.trace_qualify(base);
            self.ensure_trace_namespace(name, &base)?;
            return self
                .ensure_array(&base)
                .map_err(|_| err(format!("can't trace \"{name}\": variable isn't array")));
        }
        let lookup = self.trace_qualify(name);
        self.ensure_trace_namespace(name, &lookup)?;
        let (lvl, nm) = self.locate(&lookup);
        if let Some(frame) = self.frames.get_mut(lvl)
            && !frame.locals.contains_key(&nm)
        {
            frame.locals.insert(nm, Local::Undefined);
        }
        Ok(())
    }

    fn ensure_trace_namespace(
        &self,
        written_name: &str,
        resolved_name: &str,
    ) -> Result<(), Completion<Value>> {
        if resolved_name.contains("::") {
            let namespace = canonical_ns_name(str_slice(tcl_cmd_core::namespace::qualifiers(
                resolved_name.as_bytes(),
            )));
            if !self.namespace_exists(&namespace) {
                return Err(err(format!(
                    "can't trace \"{written_name}\": parent namespace doesn't exist"
                )));
            }
        }
        Ok(())
    }

    /// Whether `name` exists, resolving an `arr(key)` element reference to the
    /// element — the `info exists` / `existStk` semantic.
    pub(crate) fn exists_var(&self, name: &str) -> bool {
        if let Some((base, key)) = elem_ref(name) {
            return self.get_array_elem(base, key).is_some();
        }
        self.has_var(name)
    }

    /// Read `name`, resolving an `arr(key)` reference to the array element —
    /// the runtime-name analogue used by `set`/`incr`/`append`/`lappend`.
    pub(crate) fn var_get(&self, name: &str) -> Option<Value> {
        if let Some((base, key)) = elem_ref(name) {
            return self.get_array_elem(base, key);
        }
        self.get_var(name)
    }

    /// Write `name`, resolving an `arr(key)` reference to the array element.
    pub(crate) fn var_set(&mut self, name: &str, value: Value) -> Result<(), Completion<Value>> {
        if let Some((base, key)) = elem_ref(name) {
            return self.set_array_elem(base, key, value);
        }
        self.set_var(name, value)
    }

    // -- arrays (link-aware via `locate`) --

    pub(crate) fn get_array_elem(&self, name: &str, key: &str) -> Option<Value> {
        let resolved = self.ns_var_fallback(name);
        let lookup = resolved.as_deref().unwrap_or(name);
        let (lvl, nm) = self.locate(lookup);
        match self.frames.get(lvl)?.locals.get(&nm) {
            Some(Local::Array(m)) => m.get(key).cloned(),
            _ => None,
        }
    }

    /// When an unqualified `name` is not a local in the current frame but the
    /// current namespace has a variable `ns::name`, resolve to that qualified
    /// name. This is the namespace-variable fallback (a namespace script or an
    /// undeclared access reaching an existing namespace variable). Only
    /// resolves to variables that already exist, so frame locals are unaffected.
    fn ns_var_fallback(&self, name: &str) -> Option<String> {
        if name.contains("::") {
            return None;
        }
        // Only a namespace-eval body resolves a bare name to a namespace
        // variable. Inside a proc (even one defined in the namespace), an
        // unqualified name is a local unless declared via `variable`/`global`.
        if !self.in_ns_script() {
            return None;
        }
        let cur = self.current_ns();
        if cur.is_empty() {
            return None;
        }
        let top = self.frames.last()?;
        if top.locals.contains_key(name) {
            return None;
        }
        let q = format!("{cur}::{name}");
        if self
            .frames
            .first()
            .is_some_and(|g| g.locals.contains_key(&q))
        {
            Some(format!("::{q}"))
        } else {
            None
        }
    }

    /// Write an array element with no trace firing.
    pub(crate) fn write_array_raw(
        &mut self,
        name: &str,
        key: &str,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        self.validate_var_parent(name)?;
        let resolved = self.ns_var_fallback(name);
        let name = resolved.as_deref().unwrap_or(name);
        let (lvl, nm) = self.locate(name);
        let frame = self
            .frames
            .get_mut(lvl)
            .expect("locate returns a valid level");
        match frame.locals.get_mut(&nm) {
            Some(Local::Array(m)) => {
                m.insert(key.to_owned(), value);
                Ok(())
            }
            Some(Local::Scalar(_)) => Err(err(format!(
                "can't set \"{name}({key})\": variable isn't array"
            ))),
            Some(Local::Undefined) | None => {
                let mut m = BTreeMap::new();
                m.insert(key.to_owned(), value);
                frame.locals.insert(nm, Local::Array(m));
                Ok(())
            }
            Some(Local::Link { .. }) => unreachable!("locate resolves links"),
        }
    }

    pub(crate) fn set_array_elem(
        &mut self,
        name: &str,
        key: &str,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        if self.var_traces.is_empty() {
            return self.write_array_raw(name, key, value);
        }
        let old = self.get_array_elem(name, key);
        self.write_array_raw(name, key, value)?;
        let full = format!("{name}({key})");
        if let Err(e) = self.fire_var_traces(&full, "write") {
            match old {
                Some(o) => {
                    let _ = self.write_array_raw(name, key, o);
                }
                None => self.array_unset_elem(name, key),
            }
            return Err(e);
        }
        Ok(())
    }

    pub(crate) fn array_is(&self, name: &str) -> bool {
        let (lvl, nm) = self.locate(name);
        matches!(
            self.frames.get(lvl).and_then(|f| f.locals.get(&nm)),
            Some(Local::Array(_))
        )
    }

    pub(crate) fn array_pairs(&self, name: &str) -> Vec<(String, Value)> {
        let (lvl, nm) = self.locate(name);
        match self.frames.get(lvl).and_then(|f| f.locals.get(&nm)) {
            Some(Local::Array(m)) => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            _ => Vec::new(),
        }
    }

    pub(crate) fn array_unset_elem(&mut self, name: &str, key: &str) {
        if !self.var_traces.is_empty() {
            let _ = self.fire_var_traces(&format!("{name}({key})"), "unset");
        }
        let (lvl, nm) = self.locate(name);
        if let Some(Local::Array(m)) = self.frames.get_mut(lvl).and_then(|f| f.locals.get_mut(&nm))
        {
            m.remove(key);
        }
    }

    /// Append one `while executing` / `invoked from within` frame for the
    /// command `cmd_text` to the accumulating `errorInfo` trace (C's
    /// `TclLogCommandInfo`). The first frame seeds the trace from the error
    /// message `msg` and reads "while executing"; later frames read "invoked
    /// from within". A command level already logged in the same bytecode frame
    /// (`error_logged`) is not re-logged. Command text over 150 bytes is
    /// truncated with `...`, as in C.
    pub(crate) fn log_command_info(&mut self, cmd_text: &str, msg: &str, line: u32) {
        if self.error_logged {
            return;
        }
        // The innermost logged command's line drives the enclosing `(procedure …
        // line N)` / `("while" body line N)` frames (C's `iPtr->errorLine`).
        if line != 0 {
            self.error_line = line;
        }
        let started = self.error_info.is_some();
        let info = self.error_info.get_or_insert_with(|| msg.to_string());
        let verb = if started {
            "invoked from within"
        } else {
            "while executing"
        };
        // Truncate at a UTF-8 boundary near 150 bytes (C truncates at 150).
        let (slice, overflow) = if cmd_text.len() > 150 {
            let mut end = 150;
            while end > 0 && !cmd_text.is_char_boundary(end) {
                end -= 1;
            }
            (&cmd_text[..end], true)
        } else {
            (cmd_text, false)
        };
        info.push_str("\n    ");
        info.push_str(verb);
        info.push_str("\n\"");
        info.push_str(slice);
        if overflow {
            info.push_str("...");
        }
        info.push('"');
        self.error_logged = true;
    }

    /// Append a `("<label>" body line N)` frame to the trace — the frame an
    /// *uncompiled* `while`/`for`/`foreach` (the runtime command form) adds when
    /// its body errors (C's interpreted `Tcl_WhileObjCmd` &c). `N` is the
    /// innermost logged command's line. Clears `error_logged` so the enclosing
    /// command then logs its own `invoked from within` frame.
    pub(crate) fn append_body_frame(&mut self, label: &str) {
        self.append_body_frame_line(label, self.error_line);
    }

    /// Append a `("<label>" body line N)` frame with an explicit `line` — for an
    /// *inlined* body (`FunctionAsm::error_regions`), whose body-relative line is
    /// the innermost command's line minus the body's `line_base` (the uncompiled
    /// [`Self::append_body_frame`] uses `error_line` directly, since that path
    /// compiles the body standalone so its lines are already body-relative).
    pub(crate) fn append_body_frame_line(&mut self, label: &str, line: u32) {
        let info = self.error_info.get_or_insert_with(String::new);
        info.push_str("\n    (\"");
        info.push_str(label);
        info.push_str("\" body line ");
        info.push_str(&line.to_string());
        info.push(')');
        self.error_logged = false;
    }

    /// Seed `errorInfo` for an error that *originates* in a command (not from a
    /// sub-command) with a context frame (C's `Tcl_AppendObjToErrorInfo`): start
    /// it from `msg` if unset, append `frame` verbatim, and clear `error_logged`
    /// so the enclosing command then logs its `invoked from within` frame.
    /// Used by `apply` for the `(parsing lambda expression "…")` frame.
    pub(crate) fn seed_error_info_frame(&mut self, msg: &str, frame: &str) {
        let info = self.error_info.get_or_insert_with(|| msg.to_string());
        info.push_str(frame);
        self.error_logged = false;
    }

    /// The current call frame's proc name (unqualified), or `None` at the global
    /// frame / a non-proc activation.
    pub(crate) fn current_proc_name(&self) -> Option<String> {
        self.frames
            .last()
            .and_then(|f| f.proc_name.as_ref())
            // `proc_name` is an unrooted key: construction-inverse tail (#934).
            .map(|q| key_holder_and_tail_unrooted(q).1)
    }

    /// Append a `(procedure "<name>" line N)` frame — the frame a proc body adds
    /// to errorInfo as an error unwinds out of it (C's `errorInfo` proc frame).
    /// `line` is the proc-relative line of the innermost logged command. Clears
    /// `error_logged` so the call site then logs its `invoked from within` frame.
    pub(crate) fn append_proc_frame(&mut self, name: &str, line: u32) {
        let info = self.error_info.get_or_insert_with(String::new);
        info.push_str("\n    (procedure \"");
        info.push_str(name);
        info.push_str("\" line ");
        info.push_str(&line.to_string());
        info.push(')');
        self.error_logged = false;
    }

    /// Append a `(<op> trace on "<name>")` frame — the context frame C's
    /// `TclCallVarTraces` adds to `errorInfo` when a variable read/write trace
    /// callback errors, before the triggering command (`set x 1`) logs its own
    /// `invoked from within` frame. Clears `error_logged` so that command's
    /// frame is logged next (set-2.4 / set-4.4).
    pub(crate) fn append_var_trace_frame(&mut self, op: &str, name: &str) {
        let info = self.error_info.get_or_insert_with(String::new);
        info.push_str("\n    (");
        info.push_str(op);
        info.push_str(" trace on \"");
        info.push_str(name);
        info.push_str("\")");
        self.error_logged = false;
    }

    /// The innermost logged command's source line (C's `iPtr->errorLine`).
    pub(crate) fn error_line(&self) -> u32 {
        self.error_line
    }

    /// Set the innermost error line directly — the proc epilogue's
    /// break/continue→error transform pins the offending command's line before
    /// the `(procedure …)` frame is appended.
    pub(crate) fn set_error_line(&mut self, line: u32) {
        self.error_line = line;
    }

    /// Record the word a builtin is being invoked under (its source `objv[0]`).
    pub(crate) fn set_invoked_name(&mut self, name: &str) {
        self.invoked_name = Some(name.to_owned());
    }

    /// The word the current builtin was invoked under, if recorded.
    pub(crate) fn invoked_name(&self) -> Option<&str> {
        self.invoked_name.as_deref()
    }

    /// Clear `ERR_ALREADY_LOGGED` at a frame boundary (a nested `eval`/`[subst]`
    /// returned an error), so the enclosing command logs its own frame.
    pub(crate) fn clear_error_logged(&mut self) {
        self.error_logged = false;
    }

    /// Seed the `errorInfo` trace with an explicit value and mark it logged —
    /// C's `error msg info` / `return -errorinfo`, which set the trace directly
    /// and suppress the command's own `while executing` frame.
    pub(crate) fn seed_error_info(&mut self, info: String) {
        self.error_info = Some(info);
        self.error_logged = true;
    }

    /// Take the accumulated `errorInfo` trace (if any) and reset it for the next
    /// error — used when `catch` reports an error.
    pub(crate) fn take_error_info(&mut self) -> Option<String> {
        self.error_logged = false;
        self.error_info.take()
    }

    /// Publish `errorInfo` / `errorCode` into the global frame.
    pub(crate) fn publish_error(&mut self, info: &str, code: &Value) {
        if let Some(g) = self.frames.first_mut() {
            g.locals
                .insert("errorInfo".to_owned(), Local::Scalar(Value::string(info)));
            g.locals
                .insert("errorCode".to_owned(), Local::Scalar(code.clone()));
        }
    }

    pub(crate) fn write_output(&mut self, s: &str, newline: bool) {
        let mut out = self.out.borrow_mut();
        let _ = out.write_all(s.as_bytes());
        if newline {
            let _ = out.write_all(b"\n");
        }
        let _ = out.flush();
    }

    /// Register a freshly opened channel, returning its minted id (`file3`, …).
    pub(crate) fn add_channel(&mut self, chan: crate::cmd_chan::Channel) -> String {
        let id = format!("file{}", self.chan_counter);
        self.chan_counter += 1;
        self.channels.insert(id.clone(), chan);
        id
    }

    /// Borrow an open channel by id (`None` for unknown ids and the predefined
    /// std channels, which callers handle by name).
    pub(crate) fn channel_mut(&mut self, id: &str) -> Option<&mut crate::cmd_chan::Channel> {
        self.channels.get_mut(id)
    }

    /// Close and drop a channel by id, returning `true` if it existed.
    pub(crate) fn remove_channel(&mut self, id: &str) -> bool {
        self.channels.remove(id).is_some()
    }

    /// Register a user procedure under its canonical (namespace-qualified)
    /// name, and ensure its namespace exists. The namespace is taken with the
    /// shared separator-run-aware split, so a colon-run name never declares a
    /// bogus `a:`-style namespace.
    pub(crate) fn define_proc(&mut self, proc: ProcDef) {
        let key = proc.name.clone();
        // The holder namespace comes from the construction-inverse split of
        // the (unrooted) key — the written-name colon-run rule would collapse
        // a lone-colon segment (#934).
        let (holder, _tail) = key_holder_and_tail_unrooted(&key);
        if !holder.is_empty() {
            self.declare_namespace_key(&holder);
        }
        let cmd = Command::Proc(Rc::new(proc));
        self.register_command(&key, cmd);
    }

    /// Parse and evaluate a Tcl expression string against this VM.
    ///
    /// An expression that does not parse is a Tcl error, exactly as in C — see
    /// [`Self::expr_syntax_error`] for the message and error-code fidelity.
    ///
    /// Version enforcement is split: the parse is deliberately permissive (the
    /// latest grammar, `dialect = None`) and `RuntimeExprSurface::validate` then
    /// rejects anything the emulated release lacks. That works for constructs
    /// which become AST nodes carrying a version floor — a 9.0-only `lt` under
    /// 8.6 is caught there and reported as the bareword it is — but it cannot
    /// see a *lexical* construct that leaves no node behind. TIP 582 `#`
    /// comments are the one such case: `tcl_dialect::ExprCommentStyle` gates
    /// them correctly for every dialect-aware consumer (the expr lexer, so the
    /// LSP, analyser and compiler all honour it), but a VM emulating 8.x still
    /// *evaluates* `expr {1 + 2 # note}` as 3 where C 8.6 raises
    /// `invalid character "#"`. Closing that would mean threading the runtime
    /// version into this parse, which would also move the `lt` rejection from
    /// `validate` to the lexer and change its pinned message — so it is left as
    /// follow-up rather than done here.
    pub fn eval_expr(&mut self, src: &str) -> Result<Value, TclError> {
        self.claim_number_grammar();
        // The VM emulates exactly one release, so its expressions are parsed
        // under that release's grammar rather than the permissive default: a
        // 8.6 interpreter must reject `1.0_2` and an expression `#` comment,
        // both of which are valid only from 9.0.
        let node = parse_expr(src, Some(self.runtime_version.dialect_name()));
        if matches!(node, tcl_syntax::expr::ExprNode::Raw { .. }) {
            return Err(self.expr_syntax_error(src));
        }
        let surface =
            tcl_registry::expr_surface::RuntimeExprSurface::for_tcl_version(self.runtime_version);
        if let Err(error) = surface.validate(&node) {
            let message = error.message(src);
            self.seed_parsing_expression_frame(src, &message);
            return Err(TclError::with_error_code(message, error.error_code()));
        }
        let mut ops = ExprEval { vm: self };
        eval(&node, &mut ops)
    }

    /// C Tcl's syntax error for an expression `parse_expr` could not parse.
    ///
    /// `parse_expr`'s [`ExprNode::Raw`](tcl_syntax::expr::ExprNode::Raw)
    /// fallback is deliberately reason-free (the analysis pipeline needs it
    /// never to fail), so the reason is recovered here by a diagnosis-only
    /// re-scan of the same tokens — `tcl_syntax::expr::ExprSyntaxError`, which
    /// reproduces `ParseExpr`'s messages, its `_@_` insert mark, and its
    /// `TCL PARSE EXPR <detail>` code (`tclCompExpr.c:1397-1471`). The
    /// `(parsing expression "…")` `errorInfo` frame C appends at the same site
    /// is seeded here too.
    ///
    /// The scan is dialect-blind, like the parse it explains: a VM knows only
    /// its emulated [`tcl_dialect::TclVersion`], never a vendor dialect, so an
    /// iRules word operator (`contains`, `starts_with`, …) is diagnosed as the
    /// bareword it genuinely is in every plain-Tcl grammar. Compiled iRules code
    /// never reaches here — the dialect-aware compiler lowers those operators to
    /// their own opcodes.
    fn expr_syntax_error(&mut self, src: &str) -> TclError {
        // Diagnose under the emulated release too, or the message names the
        // wrong lexeme: C 8.6 reports `invalid character "_"` for `1.0_2`
        // (its number lexeme ends at `1.0`) where 9.0 accepts the whole thing.
        let error = tcl_syntax::expr::ExprSyntaxError::diagnose(
            src,
            Some(self.runtime_version.dialect_name()),
        );
        let message = error.message(src);
        self.seed_parsing_expression_frame(src, &message);
        TclError::with_error_code(message, error.error_code())
    }

    /// Add C's `(parsing expression "…")` `errorInfo` frame for a rejected
    /// expression (`tclCompExpr.c:1461-1466`).
    fn seed_parsing_expression_frame(&mut self, src: &str, message: &str) {
        let frame = tcl_syntax::expr::ExprSyntaxError::error_info_frame(src);
        self.seed_error_info_frame(message, &frame);
    }

    /// Compile `src` via the injected [`CompileService`], from the cache when
    /// possible — the shared body of [`eval_source`](Self::eval_source) and
    /// [`compile_source_cached`](Self::compile_source_cached). Picks
    /// [`CompileService::compile_traced`] and [`Self::eval_cache_traced`]
    /// instead of the fast pair whenever [`Self::step_trace_active`] is true,
    /// so a step-traced proc's `if`/`while`/`foreach`/`eval`/`uplevel`/
    /// `catch`/`try`/… bodies — every one of which funnels through this
    /// method via its runtime builtin — compile trace-visible too (issue
    /// #946 fault 3), without each call site needing to know that.
    fn compile_cached(&mut self, src: &str) -> Result<Rc<ModuleAsm>, TclError> {
        let traced = self.step_trace_active();
        let cache = if traced {
            &self.eval_cache_traced
        } else {
            &self.eval_cache
        };
        if let Some(m) = cache.get(src) {
            return Ok(Rc::clone(m));
        }
        let Some(c) = self.compiler.as_ref() else {
            return Err(TclError::new(
                "eval / command substitution requires a CompileService",
            ));
        };
        let compiled = if traced {
            c.compile_traced_for_profile(src, self.dialect_profile)
        } else {
            c.compile_for_profile(src, self.dialect_profile)
        };
        let m = Rc::new(compiled.map_err(|e| TclError::new(e.0))?);
        self.validate_module_profile(&m)?;
        let cache = if traced {
            &mut self.eval_cache_traced
        } else {
            &mut self.eval_cache
        };
        cache.insert(src.to_string(), Rc::clone(&m));
        Ok(m)
    }

    /// Compile and run a Tcl source string via the injected [`CompileService`]
    /// (the runtime-`eval` / command-substitution path) in the *current* frame.
    pub fn eval_source(&mut self, src: &str) -> Result<Completion<Value>, TclError> {
        self.claim_number_grammar();
        let module = self.compile_cached(src)?;
        let comp = self.run_module(&module);
        // Crossing back out of a nested script is a frame boundary: clear
        // `ERR_ALREADY_LOGGED` so the enclosing command (the `eval`/`[subst]`/
        // proc call site) logs its own `invoked from within` frame.
        if comp.code == Code::Error {
            self.clear_error_logged();
        }
        Ok(comp)
    }

    /// Compile `src` to its top-level activation (module cached exactly like
    /// [`eval_source`](Self::eval_source)) and register the module's procs,
    /// *without* running it. `EVAL_STK` pushes the returned activation onto the
    /// explicit stack (a transparent [script frame](crate::exec::Frame)) so a
    /// `yield` inside the evaluated script stays yieldable — the yieldable
    /// counterpart of the nested drive `eval_source` performs.
    pub(crate) fn compile_source_cached(&mut self, src: &str) -> Result<Rc<FunctionAsm>, TclError> {
        let module = self.compile_cached(src)?;
        self.validate_module_profile(&module)?;
        self.merge_procs(&module.procedures);
        Ok(Rc::new(module.top_level.clone()))
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

/// The Family-B variable store over the call-frame stack. `FrameId` is the
/// absolute frame level (`GLOBAL_FRAME` = 0); access resolves from that frame,
/// following links. A `FrameId` naming the *current* frame delegates to the full
/// by-name accessors (namespace fallback + traces); any other frame uses the
/// frame-addressed storage helpers (no current-eval-context fallback/traces).
impl VarStore for Vm {
    type Value = Value;

    fn get(&self, frame: FrameId, name: &str) -> Option<Value> {
        if frame.0 == self.current_level() {
            self.get_var(name)
        } else {
            self.get_var_from(frame.0, name)
        }
    }

    fn set(&mut self, frame: FrameId, name: &str, value: Value) {
        if frame.0 == self.current_level() {
            let _ = self.set_var(name, value);
        } else {
            self.write_scalar_from(frame.0, name, value);
        }
    }

    fn unset(&mut self, frame: FrameId, name: &str) -> bool {
        if frame.0 == self.current_level() {
            self.unset_var(name)
        } else {
            self.unset_from(frame.0, name)
        }
    }

    fn exists(&self, frame: FrameId, name: &str) -> bool {
        if frame.0 == self.current_level() {
            // The *complete* existence check: a scalar, an array, or an array
            // element (`exists_var`) — not the scalar-only `var_exists`, which
            // would miss arrays like `::env` (a `VarStore` contract bug surfaced
            // by routing `info exists` through this method).
            self.exists_var(name)
        } else {
            self.exists_from(frame.0, name)
        }
    }

    // Element access: the VM's by-name accessors already parse `base(key)`, so
    // get/set/exists reconstruct the name and delegate (honouring `FrameId`).
    // `unset` is the exception — `unset_var` is element-blind — so it removes the
    // element directly (active frame; the cores always pass the current frame).

    fn get_elem(&self, frame: FrameId, name: &str, key: &str) -> Option<Value> {
        self.get(frame, &format!("{name}({key})"))
    }

    fn set_elem(&mut self, frame: FrameId, name: &str, key: &str, value: Value) {
        self.set(frame, &format!("{name}({key})"), value);
    }

    fn unset_elem(&mut self, _frame: FrameId, name: &str, key: &str) -> bool {
        let existed = self.get_array_elem(name, key).is_some();
        self.array_unset_elem(name, key);
        existed
    }

    fn exists_elem(&self, frame: FrameId, name: &str, key: &str) -> bool {
        self.exists(frame, &format!("{name}({key})"))
    }

    fn array_keys(&self, _frame: FrameId, name: &str) -> Option<Vec<String>> {
        // `array_is` distinguishes an (empty-or-not) array from a scalar/unset;
        // `array_pairs` yields the keys (active frame — the cores pass current).
        if self.array_is(name) {
            Some(self.array_pairs(name).into_iter().map(|(k, _)| k).collect())
        } else {
            None
        }
    }
}

/// Runtime introspection backing the `info` family (`info level`/`info level N`).
///
/// Handle-free — the reconciliation finding is that `Introspect` fits *both*
/// runtime models as-drafted (no `FrameId`/`NsId` reshape needed), so it is the
/// first Family-B role trait both the VM and `runtime/rust` satisfy with shared
/// semantics: [`level`](Introspect::level) is the current stack depth and
/// [`level_argv`](Introspect::level_argv) the retained invoking words at an
/// absolute level, `None` for a level with no call (the global frame).
impl Introspect for Vm {
    type Value = Value;

    fn level(&self) -> usize {
        self.current_level()
    }

    fn level_argv(&self, level: usize) -> Option<Value> {
        self.frame_argv(level)
            .filter(|av| !av.is_empty())
            .map(Value::list)
    }
}

/// Proc introspection (`info body`/`args`/`default`) over the VM's retained
/// [`ProcDef`](crate::command::ProcDef). The body (`body_src`) and any defaults
/// are flattened to owned bytes so the shared `info` core can rebuild the result
/// values through `ValueOps` — a string round-trip that is observably identical
/// (only the value's string content is significant to these subcommands).
impl Procs for Vm {
    fn proc_info(&self, name: &str) -> Option<ProcInfo> {
        let p = self.proc_def(name)?;
        Some(ProcInfo {
            body: p.body_src.to_str().as_bytes().to_vec(),
            params: p
                .params
                .iter()
                .map(|pp| ProcParam {
                    name: pp.name.as_bytes().to_vec(),
                    default: pp.default.as_ref().map(|d| d.to_str().as_bytes().to_vec()),
                })
                .collect(),
        })
    }
}

/// Command dispatch: resolve `name` in the current namespace context and run it
/// with `argv` (name-stripped) to a [`Completion`]. Builtins run inline, procs
/// run to completion in a nested activation, aliases re-evaluate their target,
/// and an unknown name yields an error completion. The owned-`Value` model keeps
/// the refcount discipline implicit (`Rc` clones), unlike the runtime's
/// `*mut TclObj` impl.
impl Commands for Vm {
    type Value = Value;

    fn dispatch(&mut self, name: &str, argv: &[Value]) -> Completion<Value> {
        self.invoke_command(name, argv)
    }

    fn dispatch_id(&mut self, cmd: CommandId, argv: &[Value]) -> Completion<Value> {
        // Reverse the handle to its absolute FQN, then invoke that — the
        // resolve-then-invoke pairing with `Namespaces::find_command`.
        match self.command_fqn(cmd.0) {
            Some(fqn) => self.invoke_command(&fqn, argv),
            None => err("invalid command id"),
        }
    }
}

/// Variable traces: fire `var`'s `op` (`read`/`write`/`unset`) traces, aborting
/// the access if a callback errors. The VM's [`fire_var_traces`](Vm::fire_var_traces)
/// already produces the user-facing `can't read/set "var": <msg>` completion
/// (and swallows `unset`/`array` errors, matching C); the trait keeps only its
/// error result value (`options` is irrelevant to an aborted access).
impl Traces for Vm {
    type Value = Value;

    fn fire(&mut self, var: &str, op: &str) -> Result<(), Value> {
        self.fire_var_traces(var, op).map_err(|c| c.result)
    }
}

/// The call-frame stack. The VM tracks namespace context by `String`, so
/// [`push`](Frames::push) resolves the `NsId` to its name (via the intern arena)
/// and pushes a bare call frame plus that namespace context; [`pop`](Frames::pop)
/// unwinds both. [`link`](Frames::link) installs an `upvar`-style alias in the
/// current frame (the only frame `upvar` targets) — the VM stores variables
/// (globals included) in their frame's locals, so a plain level-addressed link
/// suffices.
impl Frames for Vm {
    fn push(&mut self, ns: NsId) -> FrameId {
        let name = self.ns_name(ns);
        let level = self.push_call_frame(None, Vec::new());
        self.push_ns(name);
        FrameId(level)
    }

    fn pop(&mut self) {
        self.pop_ns();
        self.pop_call_frame();
    }

    fn current(&self) -> FrameId {
        FrameId(self.current_level())
    }

    fn link(&mut self, here: FrameId, target: FrameId, local: &str, target_name: &str) {
        debug_assert_eq!(
            here.0,
            self.current_level(),
            "upvar installs in the current frame"
        );
        self.add_link(local, target.0, target_name);
    }

    fn in_proc(&self) -> bool {
        // A proc activation carries its name; the global (0) and `namespace eval`
        // frames do not (the latter runs in the current frame, pushing no frame).
        self.frames.last().is_some_and(|f| f.proc_name.is_some())
    }

    fn var_names(&self, include_links: bool) -> Vec<String> {
        self.frame_var_names(include_links)
    }
}

/// Namespace name resolution over the VM's String-based namespace model, bridged
/// to opaque `NsId`/`CommandId` handles via the intern arenas.
/// [`current`](Namespaces::current) returns the interned id of the current
/// namespace (interned when pushed). [`find_command`](Namespaces::find_command)
/// resolves `name` from `cxt` to its command key and interns that to a stable
/// `CommandId`. Note: the handle is produced for command *identity* only —
/// nothing dispatches by it yet (the `Commands` trait dispatches by name), the
/// open `find_command`/`CommandId` consumer question.
impl Namespaces for Vm {
    fn find_command(&self, cxt: NsId, name: &str) -> Option<CommandId> {
        let cxt_name = self.ns_name(cxt);
        // Intern the *absolute* FQN (the `commands` key is unrooted) so
        // `dispatch_id` can re-dispatch it unambiguously regardless of context.
        let key = self.resolve_command_fqn(&cxt_name, name)?;
        Some(CommandId(self.intern_cmd(&format!("::{key}"))))
    }

    fn current(&self) -> NsId {
        self.ns_intern
            .get(self.current_ns())
            .copied()
            .unwrap_or(ROOT_NS)
    }

    fn name(&self, ns: NsId) -> String {
        // The arena holds the canonical (unrooted) name; `namespace current`
        // reports the absolute form (`""` → `"::"`).
        let canonical = self.ns_name(ns);
        if canonical.is_empty() {
            "::".to_string()
        } else {
            format!("::{canonical}")
        }
    }

    fn command_name(&self, cmd: CommandId) -> Option<String> {
        self.command_fqn(cmd.0)
    }

    // Namespace-tree navigation over the arena. Every namespace is interned on
    // creation (`push_ns`/`declare_namespace`), so these are pure `&self` lookups
    // — the String model honouring the `NsId` handle contract.
    fn find_namespace(&self, cxt: NsId, name: &str) -> Option<NsId> {
        // Resolve `name` (absolute, or relative to `cxt`) to a canonical name.
        // Separator runs collapse and a trailing run drops (the namespace
        // rule), so `namespace exists a:::b` finds `a::b` (tclsh8.6-verified;
        // the old literal join looked up `a:::b` and missed).
        let canonical: String = if name.starts_with("::") {
            canonical_ns_name(name).into_owned()
        } else {
            let cxt_name = self.ns_name(cxt);
            if cxt_name.is_empty() {
                canonical_ns_name(name).into_owned()
            } else {
                canonical_ns_name(&format!("{cxt_name}::{name}")).into_owned()
            }
        };
        self.ns_intern.get(&canonical).copied()
    }

    fn parent(&self, ns: NsId) -> Option<NsId> {
        let name = self.ns_name(ns);
        if name.is_empty() {
            return None; // the global root has no parent
        }
        // The shared separator-run-aware qualifier split (canonical names
        // have single `::`s, but the shared op is the one source of truth).
        let parent = str_slice(tcl_cmd_core::namespace::qualifiers(name.as_bytes()));
        self.ns_intern.get(parent).copied()
    }

    fn children(&self, ns: NsId) -> Vec<NsId> {
        self.child_namespaces(&self.ns_name(ns))
            .iter()
            .filter_map(|c| self.ns_intern.get(c).copied())
            .collect()
    }

    // Command enumeration over the flat command map (keyed by canonical unrooted
    // name): the direct members of namespace `ns`, as unqualified tails.
    fn commands_in(&self, ns: NsId) -> Vec<String> {
        self.names_directly_in(&self.ns_name(ns), false)
    }

    fn procs_in(&self, ns: NsId) -> Vec<String> {
        self.names_directly_in(&self.ns_name(ns), true)
    }

    fn vars_in(&self, ns: NsId) -> Vec<String> {
        self.vars_directly_in(&self.ns_name(ns))
    }

    // `Tcl_FindNamespaceVar`'s single probe. The VM keeps namespace variables
    // in the global frame keyed by their canonical (unrooted) FQN, so the
    // namespace's own table is the set of `canonical::simple` cells.
    //
    // A `Link` counts. C's exclusion is of `CompiledLocal`s — a *proc*-local
    // `upvar`/`global` alias, which lives in the proc's compiled frame and is
    // never in a namespace `varTable`. A namespace-scoped link is a different
    // animal: `namespace upvar :: x y` (or an `upvar` at the global level)
    // puts a real `VAR_LINK` cell in the namespace's own table, which
    // `Tcl_FindNamespaceVar` and `info vars` both see (tclsh 9.0.4:
    // `namespace which -variable y` → `::n::y`). Proc-locals cannot leak in
    // here regardless, because this only ever probes `frames.first()` — the
    // global frame — and a proc's locals live in its own frame.
    //
    // `Undefined` stays excluded: that is a materialised-but-unset cell.
    fn namespace_var_exists(&self, ns: NsId, simple: &str) -> bool {
        let canonical = self.ns_name(ns);
        let key = if canonical.is_empty() {
            simple.to_owned()
        } else {
            format!("{canonical}::{simple}")
        };
        self.frames.first().is_some_and(|frame| {
            matches!(
                frame.locals.get(&key),
                Some(Local::Scalar(_) | Local::Array(_) | Local::Link { .. })
            )
        })
    }

    fn command_origin(&self, cmd: CommandId) -> Option<CommandId> {
        let fqn = self.command_fqn(cmd.0)?;
        let key = fqn.strip_prefix("::").unwrap_or(&fqn);
        // The trait's `None` means "not an imported command", which is C's
        // `cmdPtr->deleteProc == DeleteImportedCmd` test — *not* "the walk
        // ended where it started". Comparing names cannot stand in for it: a
        // visible import whose chain ends at an equally-named hidden token
        // would compare equal while genuinely being an import. The presence of
        // a provenance record is the VM's exact spelling of C's predicate.
        if !self.imported_commands.contains_key(key) {
            return None;
        }
        // `command_origin_key` owns the walk because the VM's import links are
        // name-keyed across two domains (visible / hidden token), which a bare
        // FQN cannot distinguish.
        Some(CommandId(
            self.intern_cmd(&format!("::{}", self.command_origin_key(key))),
        ))
    }
}

/// The unqualified tail of `key` if it names a command **directly** in namespace
/// `canonical` (unrooted; `""` = global), else `None`. A direct member's key is
/// `canonical::tail` (or a bare `tail` at the global level) with no further `::`
/// in the tail — so descendants (`foo::sub::x` for `foo`) and the namespace
/// itself are excluded. The tail may be empty: `quux::` is the `{}`-named
/// command, a listable direct member of `quux` (tclsh8.6: `info commands
/// ::quux::*` reports `::quux::`) — but a bare namespace-named key (`quux`
/// for `quux`, which strips to no separator) is not.
fn direct_member_tail<'a>(key: &'a str, canonical: &str) -> Option<&'a str> {
    let tail = if canonical.is_empty() {
        key
    } else {
        key.strip_prefix(canonical)?.strip_prefix("::")?
    };
    if tcl_syntax::naming::is_qualified(tail.as_bytes()) {
        None
    } else {
        Some(tail)
    }
}

#[cfg(test)]
mod family_b_tests {
    use super::*;
    use crate::command::NativeCommand;
    use tcl_compiler::cfg_builder::build_cfg_codegen;
    use tcl_compiler::codegen::codegen_module;
    use tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect as lower_to_ir;
    use tcl_dialect::DialectProfile;
    use tcl_runtime_api::{CompileError, GLOBAL_FRAME};

    const GUARDED_IDENTITY: GuardIdentity = GuardIdentity::new(1, 41);

    struct TestCompiler;

    impl CompileService for TestCompiler {
        type Module = ModuleAsm;

        fn compile(&self, src: &str) -> Result<Self::Module, CompileError> {
            self.compile_for_profile(src, DialectProfile::plain_tcl())
        }

        fn compile_for_profile(
            &self,
            src: &str,
            profile: &'static DialectProfile,
        ) -> Result<Self::Module, CompileError> {
            let registry = tcl_registry::registry_for_profile(profile);
            let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
            let ir = lower_to_ir(src, registry, config, Some(profile));
            let cfg = build_cfg_codegen(&ir, false);
            Ok(codegen_module(&cfg, &ir, registry))
        }
    }

    #[test]
    fn invokehidden_rechecks_a_hidden_builtin_against_the_child_profile() {
        let mut vm = Vm::new();
        let child_name = vm.create_child(Some("child".to_string()), false);
        let child = vm.child_id(&child_name).unwrap();
        vm.in_interp(child, |child_vm| {
            child_vm.set_dialect_profile(DialectProfile::by_name("tcl8.5"));
            child_vm.hide_command("lassign", "held").unwrap();
            child_vm.set_dialect_profile(DialectProfile::by_name("tcl8.4"));
        });
        let named = vm.invoke_hidden_in_child("child", "held", &[]).unwrap();
        assert_eq!(named.code, Code::Error);
        let by_id = vm.invoke_hidden_by_id(child, "held", &[]).unwrap();
        assert_eq!(by_id.code, Code::Error);
        assert!(
            vm.st_of(child)
                .unwrap()
                .hidden_commands
                .contains_key("held")
        );
        assert!(!vm.st_of(child).unwrap().commands.contains_key("held"));
    }

    #[test]
    fn root_invokehidden_rechecks_identity_and_restores_hidden_state_after_error() {
        let mut vm = Vm::new();
        vm.set_dialect_profile(DialectProfile::by_name("tcl8.5"));
        vm.hide_command("lassign", "held").unwrap();
        vm.set_dialect_profile(DialectProfile::by_name("tcl8.4"));

        let result = vm.invoke_hidden_in_child("", "held", &[]).unwrap();
        assert_eq!(result.code, Code::Error);
        assert!(vm.hidden_commands.contains_key("held"));
        assert!(!vm.commands.contains_key("held"));
        assert!(!vm.builtin_identities.contains_key("held"));
        assert!(!vm.imported_commands.contains_key("held"));
    }

    fn eval_value(vm: &mut Vm, script: &str) -> String {
        let completion = vm.eval_source(script).expect("script compiles");
        assert_eq!(completion.code, Code::Ok, "{script}: {completion:?}");
        completion.result.to_str().to_string()
    }

    fn prepare_stale_hidden_proc(vm: &mut Vm) {
        vm.set_compiler(Box::new(TestCompiler));
        vm.set_dialect_profile(DialectProfile::by_name("tcl8.5"));
        assert_eq!(eval_value(vm, "proc p {} { return hidden }"), "");
        vm.hide_command("p", "held").unwrap();
        assert_eq!(eval_value(vm, "proc p {} { return replacement }"), "");
        vm.set_dialect_profile(DialectProfile::by_name("tcl8.6"));
    }

    fn prepare_trace_deleted_hidden_proc(vm: &mut Vm) {
        vm.set_compiler(Box::new(TestCompiler));
        vm.set_dialect_profile(DialectProfile::by_name("tcl8.5"));
        assert_eq!(eval_value(vm, "proc p {} { return hidden }"), "");
        assert_eq!(
            eval_value(
                vm,
                "proc replace args { rename p {}; interp expose {} held p; rename p {}; proc p {} { return replacement } }"
            ),
            ""
        );
        assert_eq!(eval_value(vm, "trace add execution p enter replace"), "");
        vm.hide_command("p", "held").unwrap();
        assert_eq!(eval_value(vm, "proc p {} { return initial }"), "");
        vm.set_dialect_profile(DialectProfile::by_name("tcl8.6"));
    }

    #[test]
    fn refreshed_hidden_proc_stays_hidden_and_preserves_visible_replacement() {
        let mut vm = Vm::new();
        prepare_stale_hidden_proc(&mut vm);
        let hidden = vm.invoke_hidden_in_child("", "held", &[]).unwrap();
        assert_eq!(hidden.code, Code::Ok);
        assert_eq!(hidden.result.to_str().as_ref(), "hidden");
        assert_eq!(eval_value(&mut vm, "p"), "replacement");
        assert!(matches!(
            vm.hidden_commands.get("held"),
            Some(Command::Proc(proc))
                if proc.compiled_profile_generation == vm.profile_generation
        ));

        let child_name = vm.create_child(Some("child".to_owned()), false);
        let child = vm.child_id(&child_name).unwrap();
        vm.in_interp(child, prepare_stale_hidden_proc);

        let named = vm.invoke_hidden_in_child("child", "held", &[]).unwrap();
        assert_eq!(named.code, Code::Ok);
        assert_eq!(named.result.to_str().as_ref(), "hidden");
        assert_eq!(
            vm.in_interp(child, |child| eval_value(child, "p")),
            "replacement"
        );

        vm.in_interp(child, |child| {
            child.set_dialect_profile(DialectProfile::by_name("tcl8.5"));
        });
        let by_id = vm.invoke_hidden_by_id(child, "held", &[]).unwrap();
        assert_eq!(by_id.code, Code::Ok);
        assert_eq!(by_id.result.to_str().as_ref(), "hidden");
        assert_eq!(
            vm.in_interp(child, |child| eval_value(child, "p")),
            "replacement"
        );
        let state = vm.st_of(child).unwrap();
        assert!(matches!(
            state.hidden_commands.get("held"),
            Some(Command::Proc(proc))
                if proc.compiled_profile_generation == state.profile_generation
        ));
    }

    #[test]
    fn trace_deleted_hidden_refresh_never_resurrects_or_overwrites_replacement() {
        let mut vm = Vm::new();
        prepare_trace_deleted_hidden_proc(&mut vm);
        let root = vm.invoke_hidden_in_child("", "held", &[]).unwrap();
        assert_eq!(root.code, Code::Error);
        assert_eq!(
            root.result.to_str().as_ref(),
            "attempt to invoke a deleted command"
        );
        assert_eq!(eval_value(&mut vm, "p"), "replacement");
        assert!(!vm.hidden_commands.contains_key("held"));

        for by_id in [false, true] {
            let child_name = vm.create_child(None, false);
            let child = vm.child_id(&child_name).unwrap();
            vm.in_interp(child, prepare_trace_deleted_hidden_proc);
            let result = if by_id {
                vm.invoke_hidden_by_id(child, "held", &[]).unwrap()
            } else {
                vm.invoke_hidden_in_child(&child_name, "held", &[]).unwrap()
            };
            assert_eq!(result.code, Code::Error);
            assert_eq!(
                result.result.to_str().as_ref(),
                "attempt to invoke a deleted command"
            );
            assert_eq!(
                vm.in_interp(child, |child| eval_value(child, "p")),
                "replacement"
            );
            assert!(
                !vm.st_of(child)
                    .unwrap()
                    .hidden_commands
                    .contains_key("held")
            );
        }
    }

    #[test]
    fn relocated_hidden_refresh_persists_at_the_live_visible_key() {
        let mut vm = Vm::new();
        vm.set_compiler(Box::new(TestCompiler));
        vm.set_dialect_profile(DialectProfile::by_name("tcl8.5"));
        assert_eq!(eval_value(&mut vm, "proc p {} { return hidden }"), "");
        assert_eq!(
            eval_value(
                &mut vm,
                "proc expose args { rename p {}; interp expose {} held p; trace remove execution p enter expose }"
            ),
            ""
        );
        assert_eq!(
            eval_value(&mut vm, "trace add execution p enter expose"),
            ""
        );
        vm.hide_command("p", "held").unwrap();
        assert_eq!(eval_value(&mut vm, "proc p {} { return replacement }"), "");
        vm.set_dialect_profile(DialectProfile::by_name("tcl8.6"));

        let result = vm.invoke_hidden_in_child("", "held", &[]).unwrap();
        assert_eq!(result.code, Code::Ok);
        assert_eq!(result.result.to_str().as_ref(), "hidden");
        assert!(!vm.hidden_commands.contains_key("held"));
        assert_eq!(eval_value(&mut vm, "p"), "hidden");
    }

    #[test]
    fn imported_proc_profile_refresh_updates_the_resolved_import_not_its_source_name() {
        let mut vm = Vm::new();
        vm.set_compiler(Box::new(TestCompiler));
        vm.set_dialect_profile(DialectProfile::by_name("tcl8.5"));
        assert_eq!(
            eval_value(
                &mut vm,
                "namespace eval src {proc p {} {return original}; namespace export p}; \\
                 namespace eval dst {namespace import ::src::p}; \\
                 rename ::src::p ::src::q; proc ::src::p {} {return replacement}",
            ),
            ""
        );

        vm.set_dialect_profile(DialectProfile::by_name("tcl8.6"));
        assert_eq!(eval_value(&mut vm, "::dst::p"), "original");
        assert_eq!(eval_value(&mut vm, "::src::p"), "replacement");
        assert_eq!(eval_value(&mut vm, "::src::q"), "original");
        assert!(matches!(
            vm.commands.get("dst::p"),
            Some(Command::Proc(proc))
                if proc.compiled_profile_generation == vm.profile_generation
        ));
    }

    struct TestNative;

    impl NativeCommand for TestNative {
        fn invoke(&self, _vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
            ok(Value::empty())
        }
    }

    struct OriginNative(Rc<RefCell<Option<String>>>);

    impl NativeCommand for OriginNative {
        fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
            *self.0.borrow_mut() = Some(vm.command_origin_key("held"));
            ok(Value::empty())
        }
    }

    #[test]
    fn invokehidden_rechecks_a_hidden_native_identity_for_both_child_paths() {
        let mut vm = Vm::new();
        let child_name = vm.create_child(Some("child".to_string()), false);
        let child = vm.child_id(&child_name).unwrap();
        vm.in_interp(child, |child_vm| {
            child_vm.set_dialect_profile(DialectProfile::by_name("tcl9.0"));
            child_vm.register_native_command("tcl::mathfunc::isfinite", Rc::new(TestNative));
            child_vm
                .hide_command("tcl::mathfunc::isfinite", "held")
                .unwrap();
            child_vm.set_dialect_profile(DialectProfile::by_name("tcl8.6"));
        });

        assert_eq!(
            vm.invoke_hidden_in_child("child", "held", &[])
                .unwrap()
                .code,
            Code::Error
        );
        assert_eq!(
            vm.invoke_hidden_by_id(child, "held", &[]).unwrap().code,
            Code::Error
        );
        let state = vm.st_of(child).unwrap();
        assert!(state.hidden_commands.contains_key("held"));
        assert!(!state.commands.contains_key("held"));
        assert!(!state.builtin_identities.contains_key("held"));
    }

    #[test]
    fn invokehidden_temporarily_restores_imported_command_provenance() {
        let observed = Rc::new(RefCell::new(None));
        let mut vm = Vm::new();
        vm.register_native_command("src::probe", Rc::new(OriginNative(Rc::clone(&observed))));
        vm.declare_namespace_exports("src", &["*"]);
        assert_eq!(vm.import_commands("::src::*"), vec!["probe"]);
        vm.hide_command("probe", "held").unwrap();

        assert!(
            vm.invoke_hidden_in_child("", "held", &[])
                .unwrap()
                .code
                .is_ok()
        );
        // Direct hidden dispatch must not publish hidden provenance under the
        // token: normal command lookup from the body sees only visible state.
        assert_eq!(observed.borrow().as_deref(), Some("held"));
        assert_eq!(
            vm.hidden_imported_commands
                .get("held")
                .map(CommandSidecarKey::name),
            Some("src::probe")
        );
        assert!(!vm.imported_commands.contains_key("held"));
        assert!(!vm.commands.contains_key("held"));
    }

    fn guarded_builtin(_vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        ok(Value::empty())
    }

    #[test]
    fn command_guard_requires_identity_and_has_exact_lifecycle() {
        let mut vm = Vm::new();
        vm.register_guarded_builtin("guarded", guarded_builtin, GUARDED_IDENTITY);
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
        assert_eq!(
            vm.prepare_command_guard("guarded", GuardIdentity::new(1, 42), domains),
            Err(GuardError::IdentityMismatch)
        );
        let token = vm
            .prepare_command_guard("guarded", GUARDED_IDENTITY, domains)
            .unwrap();
        assert!(vm.check_command_guard(token, "guarded"));
        assert!(vm.release_command_guard(token));
        assert!(!vm.release_command_guard(token));
        assert!(!vm.check_command_guard(token, "guarded"));
    }

    #[test]
    fn any_command_mutation_invalidates_guard_and_live_identity_attestation() {
        let mut vm = Vm::new();
        vm.register_guarded_builtin("guarded", guarded_builtin, GUARDED_IDENTITY);
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
        let token = vm
            .prepare_command_guard("guarded", GUARDED_IDENTITY, domains)
            .unwrap();

        vm.register("unrelated", guarded_builtin);

        assert!(!vm.check_command_guard(token, "guarded"));
        assert_eq!(
            vm.prepare_command_guard("guarded", GUARDED_IDENTITY, domains),
            Err(GuardError::IdentityUnavailable)
        );
    }

    #[test]
    fn trace_registration_invalidates_the_matching_guard_domain() {
        let mut vm = Vm::new();
        vm.register_guarded_builtin("guarded", guarded_builtin, GUARDED_IDENTITY);

        let command_token = vm
            .prepare_command_guard(
                "guarded",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::CommandTrace),
            )
            .unwrap();
        let added = vm.add_cmd_trace(
            false,
            "guarded",
            vec!["rename".to_owned()],
            "callback".to_owned(),
        );
        assert_eq!(added.code, Code::Ok);
        assert!(!vm.check_command_guard(command_token, "guarded"));

        let variable_token = vm
            .prepare_command_guard(
                "guarded",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::VariableTrace),
            )
            .unwrap();
        vm.add_var_trace("watched", vec!["write".to_owned()], "callback".to_owned());
        assert!(!vm.check_command_guard(variable_token, "guarded"));
    }

    #[test]
    fn unsupported_guard_domains_are_poisoned() {
        let mut vm = Vm::new();
        vm.register_guarded_builtin("guarded", guarded_builtin, GUARDED_IDENTITY);

        for domain in [GuardDomain::Interpreter, GuardDomain::ObjectDispatch] {
            assert_eq!(
                vm.prepare_command_guard("guarded", GUARDED_IDENTITY, GuardDomains::one(domain)),
                Err(GuardError::Poisoned),
                "{domain:?} must decline until its mutation ownership is centralised",
            );
        }
    }

    #[test]
    fn string_length_base_guard_declines_and_generic_dispatch_remains_available() {
        let mut vm = Vm::new();
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            tcl_registry::IntrinsicId::StringLength.stable_id(),
            tcl_registry::IntrinsicId::StringLength.guard_semantics_key(vm.runtime_version()),
        );
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment)
            .with(GuardDomain::Namespace)
            .with(GuardDomain::CommandTrace)
            .with(GuardDomain::Interpreter);

        assert_eq!(
            vm.prepare_command_guard("string", identity, domains),
            Err(GuardError::Poisoned),
        );

        let completion = vm.dispatch("string", &[Value::string("length"), Value::string("abc")]);
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(completion.result.as_int().unwrap(), 3);
    }

    #[test]
    fn string_length_intrinsic_uses_the_selected_runtime_character_model() {
        let mut vm = Vm::new();
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
        for intrinsic in [
            tcl_registry::IntrinsicId::StringLength,
            tcl_registry::IntrinsicId::StringIndex,
        ] {
            let identity = GuardIdentity::registry_intrinsic_with_semantics(
                intrinsic.stable_id(),
                intrinsic.guard_semantics_key(vm.runtime_version()),
            );
            let token = vm
                .prepare_command_guard("string", identity, domains)
                .unwrap();
            assert!(vm.check_command_guard(token, "string"));
        }

        let value = Value::string("é🙂");
        let completion = vm
            .execute_intrinsic(
                tcl_registry::IntrinsicId::StringLength,
                std::slice::from_ref(&value),
            )
            .expect("StringLength is implemented");
        assert_eq!(completion.result.as_int().unwrap(), 2);

        vm.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        let completion = vm
            .execute_intrinsic(
                tcl_registry::IntrinsicId::StringLength,
                std::slice::from_ref(&value),
            )
            .expect("StringLength is implemented for Tcl 8");
        assert_eq!(completion.result.as_int().unwrap(), 3);
        let completion = vm.dispatch("string", &[Value::string("length"), Value::string("é🙂")]);
        assert_eq!(completion.result.as_int().unwrap(), 3);

        vm.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        let completion = vm.dispatch("string", &[Value::string("length"), Value::string("é🙂")]);
        assert_eq!(completion.result.as_int().unwrap(), 2);
        assert!(
            vm.execute_intrinsic(tcl_registry::IntrinsicId::ListLength, &[])
                .is_none()
        );
    }

    #[test]
    fn renaming_spec_registered_string_stales_its_intrinsic_guard() {
        let mut vm = Vm::new();
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            tcl_registry::IntrinsicId::StringLength.stable_id(),
            tcl_registry::IntrinsicId::StringLength.guard_semantics_key(vm.runtime_version()),
        );
        let token = vm
            .prepare_command_guard(
                "string",
                identity,
                GuardDomains::one(GuardDomain::CommandEnvironment),
            )
            .unwrap();
        let command = vm
            .take_command("string")
            .expect("registered string command");
        vm.register_command("moved", command);
        assert!(!vm.check_command_guard(token, "string"));
        assert!(!vm.check_command_guard(token, "moved"));
    }

    #[test]
    fn command_trace_stales_the_string_intrinsic_guard() {
        let mut vm = Vm::new();
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            tcl_registry::IntrinsicId::StringLength.stable_id(),
            tcl_registry::IntrinsicId::StringLength.guard_semantics_key(vm.runtime_version()),
        );
        let token = vm
            .prepare_command_guard(
                "string",
                identity,
                GuardDomains::one(GuardDomain::CommandTrace),
            )
            .unwrap();
        let added = vm.add_cmd_trace(
            false,
            "string",
            vec!["rename".to_owned()],
            "callback".to_owned(),
        );
        assert_eq!(added.code, Code::Ok);
        assert!(!vm.check_command_guard(token, "string"));
    }

    #[test]
    fn runtime_version_threads_into_the_release_globals() {
        // §5.4 VM parity: the default is the 9.0.4 reference release the
        // VM's semantics are re-derived from…
        let vm = Vm::new();
        assert_eq!(vm.runtime_version(), tcl_dialect::TclVersion::V9_0);
        assert_eq!(
            vm.get_var("tcl_version").map(|v| v.to_str().to_string()),
            Some("9.0".to_owned())
        );
        assert_eq!(
            vm.get_var("tcl_patchLevel").map(|v| v.to_str().to_string()),
            Some("9.0.4".to_owned())
        );
        // …while a profile selects its own emulated core release. iRules is
        // an embedded Tcl 8.4 surface, so routing it through the profile must
        // also update the release globals rather than retaining the default.
        let mut vm = Vm::new();
        vm.set_runtime_version(tcl_dialect::DialectProfile::irules().vm_runtime_version);
        assert_eq!(vm.runtime_version(), tcl_dialect::TclVersion::V8_4);
        assert_eq!(
            vm.get_var("tcl_version").map(|v| v.to_str().to_string()),
            Some("8.4".to_owned())
        );
        assert_eq!(
            vm.get_var("tcl_patchLevel").map(|v| v.to_str().to_string()),
            Some("8.4.20".to_owned())
        );
    }

    #[test]
    fn introspect_level_and_argv() {
        let mut vm = Vm::new();
        // Top level: depth 0, the global frame has no invoking call.
        assert_eq!(Introspect::level(&vm), 0);
        assert!(Introspect::level_argv(&vm, 0).is_none());
        // A proc-call frame with its invoking words.
        vm.push_call_frame(
            Some("p".to_string()),
            vec![Value::string("p"), Value::string("x")],
        );
        assert_eq!(Introspect::level(&vm), 1);
        assert_eq!(&*Introspect::level_argv(&vm, 1).unwrap().to_str(), "p x");
        vm.pop_call_frame();
        assert_eq!(Introspect::level(&vm), 0);
    }

    #[test]
    fn varstore_honours_frame_id() {
        let mut vm = Vm::new();
        // Write into the global frame while it is current.
        vm.set(GLOBAL_FRAME, "g", Value::string("global"));
        // Enter a proc-call frame; the global var is not in it.
        vm.push_call_frame(Some("p".to_string()), vec![Value::string("p")]);
        let here = FrameId(vm.current_level());
        assert_ne!(here, GLOBAL_FRAME);
        vm.set(here, "loc", Value::string("local"));
        // FrameId is honoured: each frame sees only its own var.
        assert_eq!(
            vm.get(GLOBAL_FRAME, "g").map(|v| v.to_str().to_string()),
            Some("global".to_string())
        );
        assert!(vm.get(here, "g").is_none());
        assert!(vm.exists(here, "loc"));
        assert!(!vm.exists(GLOBAL_FRAME, "loc"));
        // Reach back into the global frame from the child frame.
        vm.set(GLOBAL_FRAME, "g2", Value::string("two"));
        vm.pop_call_frame();
        assert_eq!(
            vm.get(GLOBAL_FRAME, "g2").map(|v| v.to_str().to_string()),
            Some("two".to_string())
        );
        assert!(vm.unset(GLOBAL_FRAME, "g2"));
        assert!(!vm.exists(GLOBAL_FRAME, "g2"));
    }

    #[test]
    fn varstore_array_elements() {
        let mut vm = Vm::new();
        assert!(!vm.exists_elem(GLOBAL_FRAME, "a", "k"));
        vm.set_elem(GLOBAL_FRAME, "a", "k", Value::string("v"));
        assert!(vm.exists_elem(GLOBAL_FRAME, "a", "k"));
        assert_eq!(
            vm.get_elem(GLOBAL_FRAME, "a", "k")
                .map(|v| v.to_str().to_string()),
            Some("v".to_string())
        );
        assert!(!vm.exists_elem(GLOBAL_FRAME, "a", "nope"));
        assert!(vm.unset_elem(GLOBAL_FRAME, "a", "k"));
        assert!(!vm.exists_elem(GLOBAL_FRAME, "a", "k"));
    }

    #[test]
    fn commands_dispatch_builtin_and_unknown() {
        let mut vm = Vm::new();
        // A builtin runs inline and yields its result.
        let c = vm.dispatch("list", &[Value::string("a"), Value::string("b c")]);
        assert_eq!(c.code, Code::Ok);
        assert_eq!(&*c.result.to_str(), "a {b c}");
        // An unknown command name is an error completion.
        let c = vm.dispatch("no_such_command", &[]);
        assert_eq!(c.code, Code::Error);
        assert_eq!(
            &*c.result.to_str(),
            "invalid command name \"no_such_command\""
        );
    }

    #[test]
    fn frames_push_pop_current_link() {
        let mut vm = Vm::new();
        let to_s = |v: Option<Value>| v.map(|v| v.to_str().to_string());
        // A global, set while the global frame is current.
        vm.set(GLOBAL_FRAME, "g", Value::string("orig"));
        let outer = Frames::current(&vm);
        assert_eq!(outer, GLOBAL_FRAME);
        // Push a proc-call frame in a fresh namespace (interned to an NsId, which
        // round-trips through the arena back to its name on push).
        let ns = vm.intern_ns("foo");
        let inner = Frames::push(&mut vm, ns);
        assert_ne!(inner, outer);
        assert_eq!(Frames::current(&vm), inner);
        // `upvar`: link `gg` (inner) to the outer frame's global `g`; reads and
        // writes through `gg` reach `g`.
        Frames::link(&mut vm, inner, outer, "gg", "g");
        assert_eq!(to_s(vm.get(inner, "gg")), Some("orig".to_string()));
        vm.set(inner, "gg", Value::string("changed"));
        // Pop back to the global frame: the link is gone, the global updated.
        Frames::pop(&mut vm);
        assert_eq!(Frames::current(&vm), outer);
        assert_eq!(to_s(vm.get(GLOBAL_FRAME, "g")), Some("changed".to_string()));
    }

    #[test]
    fn namespaces_current_and_find_command() {
        let mut vm = Vm::new();
        // At the top level the current namespace is the global root.
        assert_eq!(Namespaces::current(&vm), ROOT_NS);
        // Builtins resolve from the global namespace to stable, distinct ids.
        let a = Namespaces::find_command(&vm, ROOT_NS, "list").expect("list resolves");
        assert_eq!(a, Namespaces::find_command(&vm, ROOT_NS, "list").unwrap());
        assert_ne!(
            a,
            Namespaces::find_command(&vm, ROOT_NS, "llength").unwrap()
        );
        assert!(Namespaces::find_command(&vm, ROOT_NS, "no_such_command").is_none());
        // `current` tracks the namespace pushed by `Frames::push`.
        let foo = vm.intern_ns("foo");
        Frames::push(&mut vm, foo);
        assert_eq!(Namespaces::current(&vm), foo);
        Frames::pop(&mut vm);
        assert_eq!(Namespaces::current(&vm), ROOT_NS);
    }

    #[test]
    fn commands_dispatch_id_composes() {
        let mut vm = Vm::new();
        // Resolve a command to a handle, then invoke it *by that handle* — the
        // find_command -> dispatch_id composition.
        let id = Namespaces::find_command(&vm, ROOT_NS, "list").expect("list resolves");
        let c = vm.dispatch_id(id, &[Value::string("a"), Value::string("b")]);
        assert_eq!(c.code, Code::Ok);
        assert_eq!(&*c.result.to_str(), "a b");
        // A fabricated id yields an error completion (no such command).
        let c = vm.dispatch_id(CommandId(9999), &[]);
        assert_eq!(c.code, Code::Error);
        assert_eq!(&*c.result.to_str(), "invalid command id");
    }

    #[test]
    fn smoke_eval_canonical_snippet() {
        let mut vm = Vm::new();
        let result = vm
            .eval_expr("21 * 2")
            .expect("canonical expr must evaluate");
        assert_eq!(result.to_str().as_ref(), "42");
    }
}
