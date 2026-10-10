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

//! The non-recursive (NRE / trampoline) execution engine.
//!
//! The VM owns an explicit stack of **activation records** and trampolines over
//! it, mirroring C Tcl's `TEBCresume`. A proc call pushes a new [`Frame`] (and a
//! [`crate::interp::Vm`] call-frame) instead of recursing, so deep recursion /
//! `tailcall` / coroutines need no host-stack rewrite (steering §8b). Completion
//! codes unwind the activation stack: `Return` is absorbed at a proc boundary
//! (→ `Ok`), `Error`/`Break`/`Continue` propagate to the top of this `run`
//! (where `catch`, which invoked us via `eval_source`, observes them).

#[cfg(test)]
#[path = "exec/native_child_completion_tests.rs"]
mod native_child_completion_tests;

#[path = "exec/native_jim_script.rs"]
mod native_script;

#[path = "exec/native_each.rs"]
mod native_each;
#[cfg(test)]
#[path = "exec/native_fixed_math_tests.rs"]
mod native_fixed_math_tests;
#[cfg(test)]
#[path = "exec/native_list_index_tests.rs"]
mod native_list_index_tests;
#[cfg(test)]
#[path = "exec/native_list_operations_tests.rs"]
mod native_list_operations_tests;
#[cfg(test)]
mod native_namespace_string_tests;
mod native_scalar;
#[cfg(test)]
#[path = "exec/native_string_trim_tests.rs"]
mod native_string_trim_tests;
#[cfg(test)]
#[path = "exec/native_try_tests.rs"]
mod native_try_tests;
#[cfg(test)]
#[path = "exec/native_uplevel_tests.rs"]
mod native_uplevel_tests;

use std::collections::HashMap;
use std::rc::Rc;

use tcl_bytecode::{
    ErrorRegion, ErrorStackContext, FunctionAsm, INDEX_END, Instruction, ModuleAsm, Op, Operand,
};
use tcl_runtime_api::{Code, Completion, FatalTail};
use tcl_syntax::expr::{BinOp, UnaryOp};
use tcl_syntax::value::ValueOps;

use crate::command::{Command, ProcDef};
use crate::error::TclError;
use crate::expr;
use crate::interp::{CmdTraceEntry, CommandSidecarHandle, CommandSidecarKey, Vm, err, ok};
use crate::value::Value;

/// Active `foreach` iteration state (C Tcl `ForeachInfo` + the loop counters).
struct ForeachState {
    /// Actual compiler auxiliary; absence denotes an independently lowered loop.
    native: Option<std::sync::Arc<tcl_bytecode::NativeEachAuxiliary>>,
    /// Real untyped count/auxiliary headers owned by the stack iterator.
    /// Cursor data remains in this private opcode owner, never in a public type.
    _native_iterator_headers: Option<[Value; 2]>,
    /// Same genuine compiled lmap accumulator, born by LIST 0 before values.
    native_accumulator: Option<Value>,
    /// Loop-variable groups (from the `FOREACH_START` aux).
    var_groups: Vec<Vec<tcl_bytecode::CompiledVariableTarget>>,
    /// The value list for each group.
    lists: Vec<crate::NativeListItems>,
    /// Original containers remain owned throughout native iteration.
    _list_roots: Vec<Value>,
    /// Current iteration (0-based).
    iter_num: usize,
    /// Total iterations = max over groups of ceil(listLen / numVars).
    iter_max: usize,
    /// Instruction index of the loop body (just after `FOREACH_START`).
    body_idx: usize,
    /// This is a collecting loop (`lmap`): `LMAP_COLLECT` appends each
    /// fall-through iteration's result to `accum`, and `FOREACH_END` pushes
    /// `list(accum)` as the loop result. `false` for a plain `foreach`.
    collect: bool,
    /// Collected per-iteration results for a collecting loop (the `lmap`
    /// accumulator). Lives here rather than on the operand stack so it survives a
    /// `break`/`continue` that leaves the stack at a statement boundary.
    accum: Vec<Value>,
}

/// Iterator state for a compiled `dict for` / `dict map` loop, keyed by the
/// local slot named in the `dictFirst` / `dictNext` operand. The closed search
/// retains the original dictionary and its native representation until the
/// iterator slot is unset or its frame exits.
struct DictIterState {
    search: crate::value::NativeDictionarySearch,
}

/// One live in-frame catch range (see [`Frame::catch_ranges`]): where its
/// handler starts, the depths to restore on absorption, and the instruction
/// index of its `BEGIN_CATCH4` (the loop-vs-catch innermost-ness tiebreak for
/// a caught `break`/`continue` — see [`Vm::absorb_catch_range`]). Absorption
/// trims the operand stack *and* the in-flight `foreach` / `{*}`-expansion
/// stacks, as C's exception handling pops `auxObjList` entries opened inside
/// the range.
struct CatchRange {
    target_idx: usize,
    start_idx: usize,
    end_idx: usize,
    stack_len: usize,
    foreach_len: usize,
    expand_len: usize,
    begin_idx: usize,
    /// The range has fired and execution is in its handler. It stays on the
    /// stack so its `END_CATCH` pops it (C leaves the catchStack entry for
    /// `endCatch`), but it can no longer absorb — an exception raised *inside*
    /// the handler belongs to the enclosing range, as C's pc-containment test
    /// decides (the handler lies outside the range's code region).
    in_handler: bool,
    /// The completion this range absorbed, read by its own epilogue:
    /// `PUSH_RESULT` pushes the result, `PUSH_RETURN_CODE` the code,
    /// `PUSH_RETURN_OPTS` the options dict (the analogue of the interp result +
    /// `Tcl_GetReturnOptions` state a C catch range leaves behind).
    ///
    /// Scoped to the range rather than the frame so it cannot leak: the
    /// epilogue reads run *before* the paired `END_CATCH`, so the firing range
    /// is still innermost when they execute, and a range that never fired
    /// reports `None` — an OK completion. Frame-wide state would make a
    /// successful `catch` inherit an earlier (or inner) catch's `-code` /
    /// `-errorinfo` on its fall-through path, since both paths share
    /// `PUSH_RETURN_OPTS`.
    caught: Option<CaughtState>,
}

/// A completion absorbed by a catch range, pre-digested for the compiled catch
/// epilogue: the result value, the numeric return code, and the full options
/// dict (`catch_error_options` shape for an error — `-errorinfo`/`-errorcode`/
/// `-errorline` attached — `completion_options` otherwise).
struct CaughtState {
    result: Value,
    code: Code,
    options: Value,
}

/// Original argument storage retained by an authentic Jim object-vector call.
pub(crate) struct NativeJimVectorFrame {
    frame_index: usize,
    words: crate::NativeListItems,
}

/// One activation record owned by the interpreter or a parked coroutine.
pub(crate) struct Frame {
    jim_script_entry: Option<crate::native_jim_script::NativeJimScriptEntry>,
    jim_script: Option<crate::native_jim_script::NativeJimScriptState>,
    asm: Rc<FunctionAsm>,
    fixed_math_calls: Option<Rc<HashMap<usize, crate::interp::NativeFixedMathCall>>>,
    literal_pool: crate::literal_pool::NativeLiteralPoolReceipt,
    direct_source_operands: Option<crate::literal_pool::NativeDirectSourceOperands>,
    jim_evaluation: tcl_runtime_api::jim_error_stack::JimEvaluationFrame<Value>,
    source_location: Option<tcl_runtime_api::script_source_location::ScriptSourceLocation>,
    /// Canonical unrooted namespace whose command bindings this bytecode was
    /// specialised against.
    source_namespace: tcl_core_types::ByteNamespacePath,
    /// Dialect-profile generation under which this activation's bytecode was
    /// compiled.
    /// Stored frames cannot be rewound/recompiled after a profile switch (in
    /// particular a suspended coroutine has a live PC and operand stack), so
    /// the trampoline rejects stale execution before another specialised
    /// opcode can run.
    profile_generation: u64,
    native_policy: Option<crate::compiled::NativeCompilerPolicy>,
    /// Command-binding generation last validated for this live activation.
    command_epoch: u64,
    /// Compiler-service generation that produced this activation's bytecode.
    /// Native Script activations have no compiler generation, including after
    /// their deferred preparation fails or their active backing is retired.
    compiler_generation: Option<u64>,
    /// The manifest of the module this activation's unit came from.
    manifest: Option<std::sync::Arc<tcl_runtime_api::ArtefactIdentityManifest>>,
    off2idx: Rc<HashMap<i32, usize>>,
    /// `FOREACH_START` index → paired `FOREACH_STEP` index (the implicit jump).
    foreach_pairs: Rc<HashMap<usize, usize>>,
    pc: usize,
    stack: Vec<Value>,
    /// Original tailcall Lists retain their member headers through target completion.
    tailcall_owners: Vec<Value>,
    original_invocation: Option<Value>,
    /// Original invoking stack references remain owned through the callee
    /// activation; the reporting CallFrame separately borrows lifetime views.
    _procedure_invocation: Option<(Value, Vec<Value>)>,
    /// Literal command tokens resolved at their command-head push, before the
    /// remaining words perform substitutions. Entries are keyed by the
    /// matching command continuation so nested substitutions compose without
    /// relying on names or rerunning an already-completed substitution.
    entered_commands: Vec<EnteredCommand>,
    /// Native callable tokens captured once at this admitted chunk entry.
    chunk_commands: HashMap<tcl_runtime_api::CommandBindingIdentity, CapturedNativeCommand>,
    /// Exact native compiler selections certified at this chunk's entry.
    chunk_native_compiler_selections: std::collections::HashSet<
        tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite,
    >,
    /// Selected compiler operations awaiting their command continuation.
    entered_native_compiler_selections: Vec<(usize, usize)>,
    /// Actual stock prerequisites certified at this chunk's admission.
    chunk_native_operations: std::collections::HashSet<(usize, usize)>,
    /// Selected stock operations retain only their own prerequisite lookups.
    entered_native_operations: Vec<EnteredNativeOperation>,
    /// Active `foreach` loops in this activation (innermost last).
    foreach_stack: Vec<ForeachState>,
    /// Compiled `dict for`/`map` iterators, keyed by their local slot.
    dict_iters: HashMap<i32, DictIterState>,
    /// Stack-depth markers for in-progress `{*}` argument expansions (innermost
    /// last). `EXPAND_START` records the word-list start; `INVOKE_EXPANDED`
    /// pops it to recover the (post-expansion) argument count.
    expand_markers: Vec<usize>,
    /// Active in-frame catch ranges (innermost last) — the analogue of C Tcl's
    /// per-execution `catchStack` + `ExceptionRange` table. Pushed by a
    /// `BEGIN_CATCH4` carrying an out-of-band handler label
    /// (`Instruction::catch_target`), popped by `END_CATCH`. An exceptional
    /// completion unwinding into this frame is absorbed by the innermost range:
    /// the stack is trimmed to its depth, the completion is recorded on that
    /// range ([`CatchRange::caught`]), and execution resumes at the handler.
    catch_ranges: Vec<CatchRange>,
    /// Lifetime view of the actual result after a completed command.
    last_result: Option<crate::NativeObjectLifetimeLease>,
    native_bytecode_entered: bool,
    /// A replacement dispatch handed to this activation after procedure exit.
    deferred_dispatch: Option<Box<Tick>>,
    /// Carried return options in this evaluation frame. An ordinary successful
    /// command preserves them; a command that supplies options replaces them,
    /// and a fresh proc/eval/catch frame begins empty. Keeping the options beside
    /// `last_result` makes compiled catch ranges and transparent child
    /// activations observe the same completion as native command dispatch.
    last_options: Value,
    /// Whether this activation owns a `Vm` call-frame (a proc body) that must be
    /// popped, and whose boundary absorbs `Return`.
    is_proc: bool,
    /// A *transparent* script activation (`eval`/`uplevel`/`catch`/`subst` body
    /// run on the explicit stack so a `yield` inside it stays yieldable). It owns
    /// no `Vm` call-frame; on completion its result is delivered to the parent
    /// exactly as an inline command's would be — an `ok` result is pushed to the
    /// parent's operand stack, and a `break`/`continue` is offered to the parent's
    /// enclosing loop rather than unwinding through it. Mutually exclusive with
    /// `is_proc`.
    is_script: bool,
    /// Ambient frame namespace replaced by a transparent boundary-replay
    /// activation.  Replay changes command resolution without adding a Tcl
    /// call frame, so the namespace entry at the current frame depth is
    /// replaced in place and restored when this activation unwinds.
    replay_namespace_restore: Option<(tcl_core_types::NameBytes, tcl_runtime_api::NsId)>,
    /// Caller frames hidden by a foreign `uplevel`, retained across suspension.
    selected_frame_restore: Option<crate::interp::SelectedFrameRestore>,
    /// For a script activation, the `errorInfo` body label the uncompiled command
    /// would add on error (`("eval" body line N)` / `("uplevel" body line N)`).
    /// `None` for a command-substitution `EVAL_STK`, which adds no body frame.
    body_label: Option<&'static str>,
    /// Entered invocation retained when a tailcall removes its issuing frame.
    body_invocation: Option<(crate::NativeListItems, u32)>,
    /// Set on a **catch** activation: the body runs on the explicit stack (like a
    /// script frame) but its completion — of *any* code — is absorbed by the catch
    /// epilogue (`Vm::finish_catch`) rather than propagated. Carries the optional
    /// result / options variable names to bind.
    catch: Option<Box<CatchCtx>>,
    /// Set on a **subst** activation: a scanner-driven frame (no bytecode) that
    /// scans a `subst` template, running each top-level `[…]` as a yieldable child
    /// script frame and folding its completion back in by subst rules. Resumable
    /// once per bracket, so a `yield` inside a bracket freezes the whole scan with
    /// the coroutine.
    subst: Option<Box<crate::subst::SubstState>>,
    /// Set on an **each-loop** activation: a scanner-driven frame (no bytecode,
    /// like `subst`) running a `foreach`/`lmap` runtime-fallback loop, one
    /// iteration's body per yieldable child script frame, folding each result
    /// back in by `each_loop`'s collect/continue/break rules.
    each_loop: Option<Box<EachLoopState>>,
    control: Option<Box<crate::cmd_control::ControlState>>,
    expression: Option<Box<ExpressionReq>>,
    /// Set on a **try-phase** activation: runs one phase (body/handler/
    /// `finally`) of a `try` construct's real bytecode (unlike `subst`/
    /// `each_loop`, not scanner-driven — the phase's script dispatches
    /// normally). On completion, `Vm::unwind` calls `cmd_try::advance_try`
    /// with the taken state, which decides whether to push a fresh try-phase
    /// activation for the next phase or deliver the construct's final
    /// completion.
    ///
    /// A completed phase is transparent to the caller's enclosing loop when
    /// no handler/finally clause consumes its `break`/`continue`, just like an
    /// `eval` body. The common unwind hand-off below owns that decision for all
    /// child activations, including a proc boundary that turned
    /// `return -code continue` into `TCL_CONTINUE`.
    try_ctx: Option<Box<crate::cmd_try::TryState>>,
    /// Execution-trace leave contexts this activation settles on completion
    /// (M16.3): each traced dispatch that deferred its body to this frame. A
    /// try command transfers its context between phase frames, hence a Vec.
    /// Fired (and step scopes popped) as the owning frame unwinds.
    exec_leave: Vec<ExecLeaveCtx>,
    /// A command name to delete once this activation completes, regardless of
    /// completion code — `apply`'s temporary lambda proc, torn
    /// down the same way whether the call returned, errored, or unwound a
    /// `break`/`continue`/`return`. `None` for every other script activation.
    cleanup_proc: Option<String>,
    /// Actual manufactured lambda publication, following relocation and deletion.
    lambda_registration: Option<crate::interp::CommandSidecarHandle>,
    /// The parse error to raise once this activation's commands have run, for
    /// a script whose *later* commands do not parse.  C parses one command at
    /// a time, so the clean prefix runs first and this is what it raises
    /// afterwards (#1603).  Applied only to an `ok` completion: an error in an
    /// earlier command is what C reports, the malformed tail never having been
    /// parsed.  `None` for every script that parses whole.
    fatal_tail: Option<FatalTail>,
}

/// One traced dispatch's leave-side state: the invoked command string, the
/// still-attached owner eligible for live leave lookup, and how many step
/// scopes the dispatch pushed (popped before firing).
pub(crate) struct ExecLeaveCtx {
    jim_commands: Vec<crate::interp::native_jim_lookup::JimCommandLease>,
    cmd_string: Value,
    leave_owner: Option<CommandSidecarHandle>,
    step_scopes: usize,
}

/// A step trace remains attached to the command invocation that installed it.
/// A copied trace entry alone is not enough: its command may be deleted and
/// replaced while an inner command is running.
#[derive(Clone)]
pub(crate) struct ExecStepScope {
    pub(crate) entry: Rc<CmdTraceEntry>,
    pub(crate) key: CommandSidecarHandle,
}

impl ExecStepScope {
    fn active_for(&self, op: &str) -> bool {
        self.key.is_attached() && self.entry.has_op(op) && !self.entry.firing()
    }
}

/// A `catch`'s bind targets, carried on its activation until it completes.
pub(crate) struct CatchCtx {
    resvar: Option<Value>,
    optvar: Option<Value>,
    ignored_codes: u64,
    fatal_tail: Option<FatalTail>,
}

/// Actual prepared procedure and argv, without a script/list projection.
pub(crate) struct OriginalProcedureCallReq {
    proc: crate::command::PreparedProcedureActivation,
    invoked: Value,
    arguments: Vec<Value>,
    registration: crate::interp::CommandSidecarHandle,
}

/// A script deferred to the explicit stack through its actual compilation owner.
pub(crate) struct EvalReq {
    pub(crate) script: crate::compiled::CompiledUnit,
    /// The `errorInfo` body-frame label (`Some("eval")`/`Some("uplevel")`), or
    /// `None` for a command substitution.
    pub(crate) label: Option<&'static str>,
    /// A command name to delete once the activation completes — `apply`'s
    /// temporary lambda proc.
    pub(crate) cleanup_proc: Option<String>,
    /// The parse error to raise once the body's clean prefix has run, for a
    /// body whose later commands do not parse (see [`Frame::fatal_tail`]).
    pub(crate) fatal_tail: Option<FatalTail>,
    /// Physical caller frames to restore when this body activation completes.
    pub(crate) selected_frame_restore: Option<crate::interp::SelectedFrameRestore>,
}

/// A `catch` body deferred to the explicit stack: the compiled body plus the
/// variable names to bind once it completes. Mirrors [`EvalReq`], but
/// its completion is absorbed (see [`Frame::catch`]).
pub(crate) struct CatchReq {
    pub(crate) script: crate::compiled::CompiledUnit,
    pub(crate) resvar: Option<Value>,
    pub(crate) optvar: Option<Value>,
    pub(crate) ignored_codes: u64,
    pub(crate) fatal_tail: Option<FatalTail>,
}

/// Expression state owned by the shared syntax evaluator.
pub(crate) struct ExpressionReq {
    pub(crate) awaiting_array: Option<Vec<u8>>,
    pub(crate) state: tcl_syntax::expr::ExprEvalState<Value, Vec<u8>>,
    pub(crate) normalize: bool,
    pub(crate) jim_objects:
        Option<Rc<tcl_syntax::expr::native_objects::JimExpressionObjects<Value>>>,
    pub(crate) restore_primary: Option<Box<dyn FnOnce()>>,
}

impl Drop for ExpressionReq {
    fn drop(&mut self) {
        if let Some(restore) = self.restore_primary.take() {
            restore();
        }
    }
}

/// Scanner-driven substitution request, retaining each script on the VM stack.
pub(crate) struct SubstReq {
    pub(crate) compiled: Option<crate::compiled::CompiledUnit>,
    pub(crate) original: Option<crate::NativeObjectLifetimeLease>,
    pub(crate) control: crate::subst::SubstitutionControl,
    pub(crate) template: crate::subst::SubstSource,
    pub(crate) backslashes: bool,
    pub(crate) commands: bool,
    pub(crate) variables: bool,
}

/// Selected original root or genuine copied native List header.
pub(crate) enum EachLoopRoot {
    Original(crate::value::NativeObjectLifetimeLease),
    Header(Value),
}
impl EachLoopRoot {
    pub(crate) fn value(&self) -> &Value {
        match self {
            Self::Original(root) => root.value(),
            Self::Header(root) => root,
        }
    }
}
pub(crate) struct EachLoopGroup {
    pub(crate) variables: EachLoopRoot,
    pub(crate) values: EachLoopRoot,
    pub(crate) variable_items: Option<crate::NativeListItems>,
    pub(crate) value_items: Option<crate::NativeListItems>,
}
/// Original argv, selected header/iterator receipts, and delayed body activation.
#[derive(Clone, Copy)]
struct NativeWordDispatch<'a> {
    selected_key: &'a Option<String>,
    sidecar_handle: Option<&'a CommandSidecarHandle>,
    entered: Option<&'a EnteredCommand>,
    context: tcl_core_types::NsId,
    usage: &'a [crate::command::NativeArgumentUsageRewrite],
}

pub(crate) struct EachLoopReq {
    pub(crate) protocol: tcl_registry::native_each_loop::NativeEachLoopProtocol,
    pub(crate) groups: Vec<EachLoopGroup>,
    pub(crate) cursor: tcl_cmd_core::native_each_loop::EachLoopState,
    pub(crate) body: crate::value::NativeObjectLifetimeLease,
    pub(crate) _arguments: Option<crate::NativeListItems>,
    pub(crate) jim_empty: Option<EachLoopRoot>,
    pub(crate) invocation: Option<(crate::NativeListItems, u32)>,
}
struct EachLoopState {
    request: EachLoopReq,
    body_owner: Option<Value>,
    collected: Vec<Value>,
}

/// The outcome of folding an each-loop body child's completion into its loop
/// frame (see [`Vm::fold_each_loop`]): either resume the loop (re-tick the
/// frame for the next iteration, or its final result) or drop the frame and
/// keep unwinding with this completion (an error, or an uncaught `return`).
enum EachLoopFold {
    Resume,
    Unwind(Completion<Value>),
}

/// The outcome of folding a subst `[…]` bracket completion into its subst frame
/// (see [`Vm::fold_subst_bracket`]): either resume the scan (re-tick the frame)
/// or drop the frame and keep unwinding with this completion.
enum SubstFold {
    Resume,
    Unwind(Completion<Value>),
}

#[derive(Clone, Copy)]
enum SelectedNativeExecutionTraces {
    /// Compiler selection admitted an intrinsic while its command had no trace.
    /// A later command-sidecar mutation does not change that selected operation.
    Omitted,
}

#[derive(Clone)]
struct CapturedNativeCommand {
    command: Command,
    builtin_identity: Option<String>,
    sidecar: CommandSidecarHandle,
    execution_traces: SelectedNativeExecutionTraces,
}

/// One literal command whose token was resolved when its head was pushed. The
/// relocation-aware sidecar follows rename/hide/expose while `command` retains
/// the callable token itself across deletion or replacement.
struct EnteredCommand {
    /// First instruction after the argument-substitution region.
    resume: usize,
    continuation: usize,
    command: Command,
    builtin_identity: Option<String>,
    sidecar: CommandSidecarHandle,
    execution_traces: SelectedNativeExecutionTraces,
}

struct EnteredNativeOperation {
    site: (usize, usize),
    start: usize,
    end: usize,
    requirements: Vec<tcl_runtime_api::CommandBindingIdentity>,
}

impl Frame {
    #[cfg(test)]
    pub(crate) fn lambda_registration(&self) -> Option<&crate::interp::CommandSidecarHandle> {
        self.lambda_registration.as_ref()
    }

    fn close_native_script(&mut self) {
        if self.jim_script.is_some() || self.jim_script_entry.is_some() {
            self.jim_evaluation.invocation = Value::empty();
            drop(self.jim_script.take());
            drop(self.jim_script_entry.take());
        }
    }

    pub(crate) fn new(unit: crate::compiled::CompiledUnit, is_proc: bool) -> Self {
        if unit.jim_script.is_none() {
            unit.asm
                .validate_native_compilation_entry()
                .expect("compiler provider admits native preflight before activation");
        }
        let crate::compiled::CompiledUnit {
            // Procedure entry installs this retained declaration layout on the
            // actual call frame. Script entry only consumes the asm's borrowed
            // layout prerequisite; it must not install a new variable layout.
            compiled_local_layout: _declaration_layout,
            jim_script,
            asm,
            fixed_math_calls,
            literal_pool,
            direct_source_operands,
            source_location,
            source_namespace,
            profile_generation,
            command_epoch,
            native_cache,
            interpreter: _,
            compiler,
            fatal_tail,
            native_local_names: _,
            manifest,
        } = unit;
        let off2idx = Rc::new(build_off2idx(&asm));
        let foreach_pairs = Rc::new(pair_foreach(&asm));
        let compiler_generation = jim_script.is_none().then(|| compiler.generation());
        Self {
            jim_script_entry: jim_script,
            jim_script: None,
            jim_evaluation: tcl_runtime_api::jim_error_stack::JimEvaluationFrame {
                procedure_level: 0,
                command_name: None,
                is_procedure: false,
                script: Some(tcl_runtime_api::jim_error_stack::JimScriptLocation {
                    file: Value::string(
                        source_location
                            .as_ref()
                            .map_or("", |location| location.file.as_str()),
                    ),
                    line: source_location.as_ref().map_or(1, |location| location.line),
                }),
                invocation: Value::empty(),
            },
            asm,
            fixed_math_calls,
            literal_pool,
            direct_source_operands,
            source_location,
            source_namespace,
            profile_generation,
            native_policy: native_cache.map(|stamp| stamp.policy),
            command_epoch,
            compiler_generation,
            manifest,
            off2idx,
            foreach_pairs,
            pc: 0,
            stack: Vec::new(),
            tailcall_owners: Vec::new(),
            original_invocation: None,
            _procedure_invocation: None,
            entered_commands: Vec::new(),
            chunk_native_compiler_selections: std::collections::HashSet::new(),
            entered_native_compiler_selections: Vec::new(),
            chunk_native_operations: std::collections::HashSet::new(),
            entered_native_operations: Vec::new(),
            chunk_commands: HashMap::new(),
            foreach_stack: Vec::new(),
            dict_iters: HashMap::new(),
            expand_markers: Vec::new(),
            catch_ranges: Vec::new(),
            last_result: None,
            native_bytecode_entered: false,
            deferred_dispatch: None,
            last_options: Value::empty(),
            is_proc,
            is_script: false,
            replay_namespace_restore: None,
            selected_frame_restore: None,
            body_label: None,
            body_invocation: None,
            catch: None,
            subst: None,
            each_loop: None,
            control: None,
            expression: None,
            try_ctx: None,
            exec_leave: Vec::new(),
            cleanup_proc: None,
            lambda_registration: None,
            // A unit compiled from only the clean prefix of a malformed body
            // carries the error to raise once that prefix has run.
            fatal_tail,
        }
    }

    /// Report stale bytecode in this activation. The list-only foreach/lmap
    /// driver is generation-invariant; its separately-owned body activation is
    /// still checked when entered.
    fn stale_compilation_message(
        &self,
        current_profile: u64,
        current_compiler: u64,
        current_policy: &crate::compiled::NativeCompilerPolicy,
    ) -> Option<&'static str> {
        if self.each_loop.is_some() {
            None
        } else if self.profile_generation != current_profile {
            Some("cannot continue bytecode after dialect profile changed")
        } else if self
            .direct_source_operands
            .is_some_and(|direct| !direct.is_current(current_policy))
        {
            Some("cannot continue direct source after native invocation policy changed")
        } else if self
            .native_policy
            .is_some_and(|policy| &policy != current_policy)
        {
            Some("cannot continue bytecode after native invocation policy changed")
        } else if self
            .compiler_generation
            .is_some_and(|generation| generation != current_compiler)
        {
            Some("cannot continue bytecode after compile service changed")
        } else {
            None
        }
    }

    fn namespace_stale_message(
        &self,
        current_namespace: &tcl_core_types::ByteNamespacePath,
    ) -> Option<&'static str> {
        (self.each_loop.is_none()
            && self.compiler_generation.is_some()
            && &self.source_namespace != current_namespace)
            .then_some("cannot continue bytecode after namespace changed")
    }

    /// Build a bytecode frame from the generations owned by its compiled
    /// script. Keeping this extraction here prevents individual deferred
    /// consumers from accidentally substituting the VM's current generations.
    fn from_compiled_unit(unit: crate::compiled::CompiledUnit) -> Self {
        Self::new(unit, false)
    }

    /// A transparent script activation (see [`Frame::is_script`]) for a body run
    /// yieldably on the explicit stack (`EVAL_STK`, and the `eval`/`uplevel`
    /// builtins routed through it). `label` is the `errorInfo` body-frame label
    /// (`Some("eval")`/`Some("uplevel")`), or `None` for a command substitution.
    pub(crate) fn new_script(
        script: crate::compiled::CompiledUnit,
        label: Option<&'static str>,
    ) -> Self {
        let mut f = Self::from_compiled_unit(script);
        f.is_script = true;
        f.body_label = label;
        f
    }

    /// Transfer all deferred eval lifecycle ownership to one transparent frame.
    fn new_eval(req: EvalReq) -> Self {
        let mut frame = Self::new_script(req.script, req.label);
        frame.fatal_tail = req.fatal_tail.or(frame.fatal_tail);
        frame.cleanup_proc = req.cleanup_proc;
        frame.selected_frame_restore = req.selected_frame_restore;
        frame
    }

    /// A **catch** activation: the body runs on the explicit stack (yieldable),
    /// but its completion is absorbed by the catch epilogue rather than delivered
    /// to the parent (see [`Frame::catch`]).
    pub(crate) fn new_catch(req: CatchReq) -> Self {
        let mut f = Self::from_compiled_unit(req.script);
        f.catch = Some(Box::new(CatchCtx {
            resvar: req.resvar,
            optvar: req.optvar,
            ignored_codes: req.ignored_codes,
            fatal_tail: req.fatal_tail,
        }));
        f
    }

    /// A **subst** activation: a scanner-driven frame (empty placeholder asm — it
    /// never executes bytecode) carrying the resumable scan state. `tick` runs the
    /// scanner instead of the bytecode dispatch when `subst` is set.
    pub(crate) fn new_subst(req: SubstReq, placeholder: crate::compiled::CompiledUnit) -> Self {
        if let Some(unit) = req.compiled {
            return Self::new_script(unit, None);
        }
        if let Some(original) = req.original {
            let flags = u8::from(!req.variables)
                | (u8::from(!req.commands) << 1)
                | (u8::from(!req.backslashes) << 2)
                | if req.control == crate::subst::SubstitutionControl::Command {
                    128
                } else {
                    0
                };
            let mut unit = placeholder;
            unit.jim_script = Some(crate::native_jim_script::NativeJimScriptEntry::borrowed(
                original,
                Some(flags),
            ));
            unit.compiler = crate::compiled::CompilerProvenance::NativeScript;
            return Self::new(unit, false);
        }
        let mut f = Self::new(placeholder, false);
        f.subst = Some(Box::new(crate::subst::SubstState::new(
            req.template,
            req.backslashes,
            req.commands,
            req.variables,
            req.control,
        )));
        f
    }

    /// An **each-loop** activation: a scanner-driven frame (empty placeholder
    /// asm, like `subst`) carrying the resumable `foreach`/`lmap` iteration
    /// state. `tick` runs the loop driver instead of the bytecode dispatch when
    /// `each_loop` is set.
    pub(crate) fn new_each_loop(
        mut req: EachLoopReq,
        placeholder: crate::compiled::CompiledUnit,
    ) -> Self {
        let mut f = Self::new(placeholder, false);
        f.body_invocation = req.invocation.take();
        f.each_loop = Some(Box::new(EachLoopState {
            request: req,
            body_owner: None,
            collected: Vec::new(),
        }));
        f
    }

    pub(crate) fn new_original_invocation(
        unit: crate::compiled::CompiledUnit,
        original: Value,
    ) -> Self {
        let mut frame = Self::new(unit, false);
        frame.original_invocation = Some(original);
        frame.native_bytecode_entered = true;
        frame
    }

    fn new_expression(req: ExpressionReq, placeholder: crate::compiled::CompiledUnit) -> Self {
        let mut frame = Self::new(placeholder, false);
        frame.expression = Some(Box::new(req));
        frame
    }

    fn new_control(
        mut state: crate::cmd_control::ControlState,
        placeholder: crate::compiled::CompiledUnit,
    ) -> Self {
        let mut frame = Self::new(placeholder, false);
        if let Some((label, restore)) = state.take_object_context() {
            frame.is_script = true;
            frame.body_label = Some(label);
            frame.selected_frame_restore = restore;
        }
        frame.control = Some(Box::new(state));
        frame
    }

    /// A **try-phase** activation ([`Frame::try_ctx`]): runs `req.script` (the
    /// current phase's compiled script) as ordinary bytecode, tagged with the
    /// state `Vm::unwind` hands to `cmd_try::advance_try` on completion.
    pub(crate) fn new_try(req: crate::cmd_try::TryReq, initial_options: Value) -> Self {
        let mut f = Self::from_compiled_unit(req.script);
        f.last_options = initial_options;
        f.try_ctx = Some(Box::new(req.state));
        f
    }

    /// Push `v` onto this frame's operand stack. Used by a coroutine `resume` to
    /// deliver the resume value as the result of the `yield` that suspended here
    /// — exactly where the normal builtin-return path (`f.stack.push`) would have
    /// put the command's result, so the following instruction is oblivious.
    pub(crate) fn push_operand(&mut self, v: Value) {
        self.stack.push(v.into_native_reference());
    }

    /// Take the innermost literal command token whose substitution region ends
    /// at the current continuation. Nested commands can share an activation,
    /// so match by continuation instead of assuming a single pending token.
    fn take_entered_command(&mut self) -> Option<EnteredCommand> {
        let index = self
            .entered_commands
            .iter()
            .rposition(|entered| entered.continuation == self.pc)?;
        Some(self.entered_commands.remove(index))
    }
}

/// Pair each `FOREACH_START` with its `FOREACH_STEP` by nesting order
/// (`foreach_start … body … foreach_step` nests like balanced brackets).
fn pair_foreach(asm: &FunctionAsm) -> HashMap<usize, usize> {
    let mut pairs = HashMap::new();
    let mut stack: Vec<usize> = Vec::new();
    for (i, instr) in asm.instructions.iter().enumerate() {
        match instr.op {
            Op::FOREACH_START => stack.push(i),
            Op::FOREACH_STEP => {
                if let Some(s) = stack.pop() {
                    pairs.insert(s, i);
                }
            }
            _ => {}
        }
    }
    pairs
}

/// What one instruction step does to the trampoline.
enum Tick {
    /// Stay in the current frame.
    Continue,
    /// The current frame finished — unwind with this completion.
    Return(Completion<Value>),
    /// Call a proc — push a new activation + call-frame.
    Call {
        proc: Box<crate::command::PreparedProcedureActivation>,
        /// The command spelling before namespace resolution.
        invoked: Value,
        argv: Vec<Value>,
        lambda_registration: Option<crate::interp::CommandSidecarHandle>,
    },
    /// Run a compiled script on the explicit stack — push a *transparent* script
    /// activation ([`Frame::new_script`]). Used by `EVAL_STK` (and the
    /// `eval`/`uplevel`/`apply` builtins routed through it) so a `yield` inside
    /// the body stays yieldable instead of re-entering the evaluator on the
    /// native stack. `label` is the `errorInfo` body-frame label (`None` for
    /// command subst); `cleanup_proc` is a command name to delete once the
    /// pushed frame completes (`apply`'s temporary lambda proc).
    PushScript {
        script: Box<crate::compiled::CompiledUnit>,
        label: Option<&'static str>,
        cleanup_proc: Option<String>,
        fatal_tail: Option<FatalTail>,
        namespace: ScriptNamespace,
    },
    /// Run a `catch` body on the explicit stack (yieldable) via a catch
    /// activation ([`Frame::new_catch`]); its completion is absorbed by the catch
    /// epilogue. Drained from `Vm.pending.catch`, mirroring `PushScript`.
    PushCatch(Box<CatchReq>),
    /// An eval request carrying selected-frame restoration through suspension.
    PushEval(Box<EvalReq>),
    /// Run a `subst` on the explicit stack (yieldable) via a subst activation
    /// ([`Frame::new_subst`]); its `[…]` bodies run as child script frames and are
    /// folded back by subst rules. Drained from `Vm.pending.subst`.
    PushSubst {
        req: Box<SubstReq>,
        placeholder: Box<crate::compiled::CompiledUnit>,
    },
    /// Run a `foreach`/`lmap` runtime-fallback loop on the explicit stack
    /// (yieldable) via an each-loop activation ([`Frame::new_each_loop`]); each
    /// iteration's body runs as a child script frame and is folded back by
    /// `each_loop`'s collect/continue/break rules. Drained from
    /// `Vm.pending.each_loop`.
    PushEachLoop {
        req: Box<EachLoopReq>,
        placeholder: Box<crate::compiled::CompiledUnit>,
    },
    PushExpression {
        req: Box<ExpressionReq>,
        placeholder: Box<crate::compiled::CompiledUnit>,
    },
    PushControl {
        state: Box<crate::cmd_control::ControlState>,
        placeholder: Box<crate::compiled::CompiledUnit>,
    },
    /// Run one phase (body/handler/`finally`) of a `try` on the explicit stack
    /// (yieldable) via a try-phase activation ([`Frame::new_try`]); its
    /// completion decides the next phase via `cmd_try::advance_try`. Drained
    /// from `Vm.pending.try_phase`.
    PushTry {
        req: Box<crate::cmd_try::TryReq>,
        initial_options: Value,
    },
    /// Schedule a procedure-owned replacement and issue ordinary `TCL_RETURN`.
    /// Catch may absorb that completion while the frame retains the request.
    Tailcall(TailcallReq),
    /// `yield`/`yieldto` — suspend the running coroutine, freezing the whole
    /// activation stack. Only reachable in [`DriveMode::CoroDriver`]; the driver
    /// returns [`RunExit::Yielded`] leaving `acts` intact (pc already past the
    /// suspend point).
    Suspend(YieldReq),
}

/// Namespace semantics for a transparent script activation.
///
/// Dynamic Tcl scripts inherit their caller's namespace. A stale compiled
/// command boundary is different: executable inlining may have copied that
/// command from another namespace, so its replay must temporarily restore the
/// compiler-recorded source-site namespace.
enum ScriptNamespace {
    Inherit,
    CommandBoundary(tcl_runtime_api::CompiledNamespaceContext),
}

impl Tick {
    fn selected_frame_restore(&self) -> Option<&crate::interp::SelectedFrameRestore> {
        match self {
            Self::PushEval(request) => request.selected_frame_restore.as_ref(),
            Self::PushControl { state, .. } => state.selected_frame_restore(),
            _ => None,
        }
    }

    fn take_selected_frame_restore(&mut self) -> Option<crate::interp::SelectedFrameRestore> {
        match self {
            Self::PushEval(request) => request.selected_frame_restore.take(),
            Self::PushControl { state, .. } => state.take_selected_frame_restore(),
            _ => None,
        }
    }
}

/// A coroutine suspend request, produced by the `yield`/`yieldto` builtins and
/// carried out of [`Vm::drive`](Vm) to the coroutine's `resume`.
pub(crate) enum YieldReq {
    /// `yield ?value?` — `value` is what the resumer receives.
    Yield(Value),
    /// Relay lookup uses the namespace selected in the coroutine, while the
    /// target executes in the resumer's variable frame.
    YieldTo { original: Value },
}

/// How [`Vm::drive`](Vm) treats a [`Tick::Suspend`]: `Plain` is an ordinary
/// activation (a top-level yield is an error); `CoroDriver` is a coroutine's
/// `resume`, which suspends on yield.  Cross-interp evaluation no longer needs
/// a dedicated mode: an interpreter boundary is a plain native re-entry (the
/// engine swaps interp state — see [`crate::interp::Vm::in_interp`]), so a
/// cross-interp alias runs on whichever mode the current drive is in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DriveMode {
    Plain,
    CoroDriver,
}

/// The common result of consuming one [`Tick`]. Activation-bearing ticks have
/// already installed their frame when `Resume` is returned; every other shape
/// stays explicit so both the ordinary drive and a nested tailcall use the same
/// exhaustive protocol.
enum TickAction {
    Resume,
    Complete(Completion<Value>),
    Suspend(YieldReq),
}

/// A retained driver consumes its child's completion before ordinary bytecode.
enum ChildCompletion {
    Resume,
    Unwind(Completion<Value>),
    Bytecode(Completion<Value>),
}

/// Deferred native tailcall: lookup belongs to the issuing activation,
/// while the target executes after that activation has left.
pub(crate) struct TailcallReq {
    pub(crate) namespace: Value,
    pub(crate) words: Vec<Value>,
    pub(crate) original_list: Option<Value>,
}

/// How a [`Vm::drive`](Vm) invocation ended: the activation stack emptied
/// (`Done`), or a coroutine suspended (`Yielded`, `acts` left frozen).
pub(crate) enum RunExit {
    Done(Completion<Value>),
    Yielded(YieldReq),
}

fn build_off2idx(asm: &FunctionAsm) -> HashMap<i32, usize> {
    asm.instructions
        .iter()
        .enumerate()
        .map(|(i, instr)| (instr.offset, i))
        .collect()
}

fn imm0(instr: &Instruction) -> i32 {
    match instr.operands.first() {
        Some(Operand::Imm(n)) => *n,
        _ => 0,
    }
}

fn imm_at(instr: &Instruction, idx: usize) -> i32 {
    match instr.operands.get(idx) {
        Some(Operand::Imm(n)) => *n,
        _ => 0,
    }
}

fn label0(instr: &Instruction) -> Option<&str> {
    match instr.operands.first() {
        Some(Operand::Label(s)) => Some(s.as_str()),
        _ => None,
    }
}

fn pop(f: &mut Frame) -> Value {
    f.stack.pop().unwrap_or_else(Value::empty)
}

/// The dict decode/rebuild helpers behind the `dict*` opcodes.
///
/// Every one of them starts from [`Vm::dict_pairs`], the VM's binding of the
/// shared [`ValueOps::dict_pairs`] owner — so the opcodes see the same
/// **canonical** pair list `dict size`, `dict get` via `dispatch_canon`, and
/// the WASM runtime see: first-occurrence position, **last value winning** on a
/// duplicate key (`SetDictFromAny`, tclDictObj.c(9.0.4):589, feeding
/// `Tcl_DictObjPut`'s hash overwrite). Decoding the list rep straight into
/// `chunks_exact(2)` pairs instead would leave both values of a duplicate key
/// in the list, so every `find` would read or rewrite the *first* one instead
/// of the canonical last.
impl Vm {
    /// The dict's canonical ordered `(key-string, value)` pairs.
    ///
    /// A parse failure is reported as a **dict**, not a list: C's
    /// `SetDictFromAny` passes the type strings `dict`/`DICTIONARY` down to
    /// `FindElement`, so `dict size {a 1 {b}c d}` says `dict element in braces
    /// …` with `errorCode` `TCL VALUE DICTIONARY JUNK`. The shared codec is
    /// list-worded (it is the *list* element codec), so the noun is restored
    /// here — the one place every VM dict path decodes through, so the wording
    /// only needs to be right in one place.
    #[cfg(test)]
    pub(crate) fn dict_pairs(
        &mut self,
        v: &Value,
    ) -> Result<Vec<(String, Value)>, Completion<Value>> {
        let pairs = tcl_syntax::value::ValueOps::dict_pairs(self, v)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
        pairs
            .into_iter()
            .map(|(key, value)| {
                key.try_to_str()
                    .map(|key| (key.to_string(), value))
                    .map_err(|error| crate::command::completion_from_tcl_error(self, error.into()))
            })
            .collect()
    }

    #[cfg(test)]
    fn dict_set_path(
        &mut self,
        current: &Value,
        keys: &[Value],
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        let objects = crate::cmd_dict::VmDictionaryObjects::selected(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let retained_input = current.clone();
        tcl_cmd_core::native_dictionary::set_path(&objects, Some(&retained_input), keys, value)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))
    }

    #[cfg(test)]
    fn dict_unset_path(
        &mut self,
        current: &Value,
        keys: &[Value],
    ) -> Result<Value, Completion<Value>> {
        let objects = crate::cmd_dict::VmDictionaryObjects::selected(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let retained_input = current.clone();
        tcl_cmd_core::native_dictionary::remove_path(&objects, Some(&retained_input), keys)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))
    }
}

fn ilen(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// Resolve a bytecode-immediate index (`N`, or `end`-relative encoded relative
/// to `INDEX_END`) against a length.
///
/// `end-N` encodes as `INDEX_END - N` and `end+N` as `INDEX_END + N`
/// (`parse_tcl_index`), so an end-relative index occupies a band *around*
/// `INDEX_END`, not just `<= INDEX_END`. Detecting only `<= INDEX_END` would
/// misread `end+N` as a huge negative plain index, so e.g. `lrange $l
/// end+1 0` and `lrange $l 0 end+1` would disagree with the uncompiled command
/// (lrange-5 battery). The band's half-width `1 << 29` sits midway between
/// `INDEX_END` (`-2^30`) and 0: any plausible `end±N` lands inside it, while no
/// realistic literal plain index is that far negative.
fn imm_index(imm: i32, len: usize) -> isize {
    let n = isize::try_from(len).unwrap_or(isize::MAX);
    if imm <= INDEX_END + (1 << 29) {
        (n - 1) + (imm as isize - INDEX_END as isize)
    } else {
        imm as isize
    }
}

/// Original end-relative immediate coordinates for the portable string owner.
fn encoded_string_index(encoded: i32) -> Value {
    if encoded <= INDEX_END + (1 << 29) {
        let offset = i64::from(encoded) - i64::from(INDEX_END);
        Value::string(if offset == 0 {
            "end".to_owned()
        } else {
            format!("end{offset:+}")
        })
    } else {
        Value::int(i64::from(encoded))
    }
}

/// Resolve a runtime index value, erroring on a non-integer spec (`bad index`)
/// — the validating form C's index-taking opcodes all use, since every one of
/// them routes through `TclGetIntForIndexM` and treats its failure as an error
/// (`INST_STR_INDEX`, `INST_STR_RANGE`, `INST_STR_REPLACE`, `INST_LREPLACE4`);
/// the non-validating `unwrap_or` forms it replaced turned a garbage index into
/// a plausible-looking value.
fn checked_index_value(vm: &mut Vm, v: &Value, len: usize) -> Result<isize, Completion<Value>> {
    let s = v.to_str();
    crate::command::resolve_index(vm, &s, len).ok_or_else(|| crate::command::bad_index(vm, &s))
}

/// List element at signed index, or empty when out of range.
fn get_at(items: &[Value], i: isize) -> Value {
    usize::try_from(i)
        .ok()
        .and_then(|x| items.get(x))
        .cloned()
        .unwrap_or_else(Value::empty)
}

/// Sublist `[lo..=hi]` clamped to bounds; empty when the range is empty.
fn slice(items: &[Value], lo: isize, hi: isize) -> Value {
    let len = isize::try_from(items.len()).unwrap_or(isize::MAX);
    if hi < 0 || lo >= len {
        return Value::list(Vec::new());
    }
    let lo = usize::try_from(lo.max(0)).unwrap_or(0);
    let hi = usize::try_from(hi.min(len - 1)).unwrap_or(0);
    if lo > hi {
        return Value::list(Vec::new());
    }
    Value::list(items[lo..=hi].to_vec())
}

/// Character index of `needle` in `hay` (`string first`/`last`), or -1.
///
/// An **empty needle is a miss**: both C helpers open with an explicit
/// `if (ln == 0) { /* We don't find empty substrings.  Bizarre! */ goto …End; }`
/// that leaves the result at `-1` (`tclStringObj.c:3853-3858` `TclStringFirst`,
/// `:3948-3956` `TclStringLast`). Rust's `str::find`/`rfind` instead *do* match
/// the empty needle, at 0 and at the haystack length respectively, so
/// `INST_STR_FIND`/`INST_STR_FIND_LAST` returned `0`/`len` where C returns `-1`
/// (and where our own `string first`/`last` builtins already returned `-1`).
fn char_find(hay: &str, needle: &str, last: bool) -> i64 {
    if needle.is_empty() {
        return -1;
    }
    let byte = if last {
        hay.rfind(needle)
    } else {
        hay.find(needle)
    };
    byte.map_or(-1, |b| {
        i64::try_from(hay[..b].chars().count()).unwrap_or(-1)
    })
}

fn label_to_idx(asm: &FunctionAsm, off2idx: &HashMap<i32, usize>, label: &str) -> Option<usize> {
    let off = *asm.labels.get(label)?;
    let off_i32 = i32::try_from(off).ok()?;
    Some(
        off2idx
            .get(&off_i32)
            .copied()
            .unwrap_or(asm.instructions.len()),
    )
}

/// Resolve a jump operand (`Operand::Label`) to a target instruction index.
fn jump_target(
    asm: &FunctionAsm,
    off2idx: &HashMap<i32, usize>,
    instr: &Instruction,
) -> Option<usize> {
    label_to_idx(asm, off2idx, label0(instr)?)
}

fn bin(vm: &mut Vm, f: &mut Frame, op: BinOp) -> Result<(), Completion<Value>> {
    let dialect = vm.native_invocation_dialect();
    let b = pop(f);
    let a = pop(f);
    match expr::arith_in(vm.numeric_context(), op, &a, &b) {
        Ok(v) => {
            f.stack.push(v.with_native_double_format(dialect));
            Ok(())
        }
        // Through `completion_from_tcl_error`, not `err`: C stamps an
        // `-errorcode` on the arithmetic failures (`ARITH DIVZERO`,
        // `ARITH DOMAIN`), and a bare `err` would drop it, leaving the
        // compiled `expr {…}` path reporting `NONE` where the dynamic
        // `expr $e` path reports the real code.
        Err(e) => Err(crate::command::completion_from_tcl_error(vm, e)),
    }
}

fn cmp(vm: &mut Vm, f: &mut Frame, op: BinOp) -> Result<(), Completion<Value>> {
    let b = pop(f);
    let a = pop(f);
    match expr::compare_in(vm.numeric_context(), op, &a, &b) {
        Ok(t) => {
            f.stack
                .push(expr::native_boolean_result(vm.numeric_context(), t));
            Ok(())
        }
        Err(e) => Err(crate::command::completion_from_tcl_error(vm, e)),
    }
}

/// Eager (non-short-circuit) logical AND/OR: pop both operands, coerce each
/// to boolean, and push the boolean result.
///
/// C8.4's native compiler normalises the original left operand through a
/// registered `0`/`1`, then emits a short-circuit jump and eager LAND/LOR.
/// Later compiler recipes use branches around both operands instead.
fn land_lor(vm: &mut Vm, f: &mut Frame, is_and: bool) -> Result<(), Completion<Value>> {
    let b = pop(f);
    let a = pop(f);
    let value = expr::logical_value_for_vm(vm, a, &b, is_and)
        .map_err(|error| crate::command::completion_from_tcl_error(vm, error))?;
    f.stack.push(value);
    Ok(())
}

fn un(vm: &mut Vm, f: &mut Frame, op: UnaryOp) -> Result<(), Completion<Value>> {
    let dialect = vm.native_invocation_dialect();
    let v = pop(f);
    match expr::unary_for_vm(vm, op, &v) {
        Ok(r) => {
            f.stack.push(r.with_native_double_format(dialect));
            Ok(())
        }
        // Keeps C's `-errorcode` (`ARITH DOMAIN <description>` for an
        // operand-type error); a bare `err` would drop it.
        Err(e) => Err(crate::command::completion_from_tcl_error(vm, e)),
    }
}

/// Take the top `imm0(instr)` stack words, deepest first — the operand-counted
/// word list an invocation-shaped opcode (`tclooNext`/`tclooNextClass`) consumes.
/// `mnemonic` names the opcode in the underflow error.
fn take_words(
    f: &mut Frame,
    instr: &Instruction,
    mnemonic: &str,
) -> Result<Vec<Value>, Completion<Value>> {
    let count = usize::try_from(imm0(instr)).unwrap_or(0);
    if count == 0 || f.stack.len() < count {
        return Err(err(format!("{mnemonic}: stack underflow")));
    }
    Ok(f.stack.split_off(f.stack.len() - count))
}

/// An iRules-dialect binary operator (`IRULE_*`): the operands are on the stack
/// left-then-right, exactly as [`bin`]/[`cmp`] take them, and the semantics come
/// from the shared [`expr::irule_binary`].
fn irule(vm: &mut Vm, f: &mut Frame, op: BinOp) -> Result<(), Completion<Value>> {
    let provider = tcl_registry::native_expression_program::authored_f5_string_predicate_provider(
        vm.expression_evaluation_policy().as_ref(),
    );
    let b = pop(f);
    let a = pop(f);
    match expr::irule_binary(provider, op, &a, &b) {
        Ok(v) => {
            f.stack.push(v);
            Ok(())
        }
        Err(e) => Err(crate::command::completion_from_tcl_error(vm, e)),
    }
}

/// The Tcl `wrong # args` usage message for a proc.
fn proc_usage(vm: &mut Vm, proc: &ProcDef) -> Result<Vec<u8>, Completion<Value>> {
    let plain = [Value::from_native_string_bytes(
        proc.actual_command_slot().simple.as_bytes().to_vec(),
    )];
    let words = proc.usage_name.as_deref().unwrap_or(&plain);
    let usage = vm.native_argument_usage_header(words)?;
    let suffix = tcl_syntax::formal_params::formal_parameter_usage_bytes(
        &proc.formal_parameters(),
        proc.parameter_grammar,
    );
    let protocol = vm
        .native_invocation_dialect()
        .usage_protocol(Some(
            tcl_registry::native_usage::LogicalUsageProvider::Tcl84CoreSimulation,
        ))
        .ok_or_else(|| {
            vm.refuse_host_command("native procedure usage protocol is unavailable".into())
        })?;
    Ok(protocol.render_procedure_message(&usage, &suffix))
}

impl Vm {
    /// Run a module: register its compiled procs, then run the top-level script.
    ///
    /// # Panics
    ///
    /// Panics before activation when a foreign artifact requires unresolved
    /// native preflight. Embedders accepting such artifacts use
    /// [`Self::try_run_module`] to retain the typed host admission error.
    pub fn run_module(&mut self, module: &ModuleAsm) -> Completion<Value> {
        self.try_run_module(module)
            .expect("module requires a genuine native compiler preflight provider")
    }

    /// Admit the entering script independently of its unentered procedures.
    /// Admission failure is a host error, never a Tcl completion.
    ///
    /// # Errors
    ///
    /// Returns the typed native admission obligation before any activation.
    pub fn try_run_module(
        &mut self,
        module: &ModuleAsm,
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        self.validate_foreign_native_compilation_entry(&module.top_level)?;
        let completion = self.run_admitted_module(module);
        self.finish_host_execution(completion)
    }

    fn run_admitted_module(&mut self, module: &ModuleAsm) -> Completion<Value> {
        if let Err(error) = self.validate_module_profile(module) {
            return crate::command::completion_from_tcl_error(self, error);
        }
        let namespace = self.source_namespace_path();
        let namespace_mismatch = module.source_namespace != namespace;
        let replacement = if self.step_trace_active()
            || namespace_mismatch
            || !self.function_command_bindings_match_with_manifest(
                &module.top_level,
                module.manifest.as_deref(),
            ) {
            if module.source.is_empty() {
                return err("stale bytecode module has no source for plain dispatch");
            }
            match self.compile_plain_module_bytes(&module.source, &namespace) {
                Ok(module) => Some(module),
                Err(error) => return crate::command::completion_from_tcl_error(self, error),
            }
        } else {
            None
        };
        self.claim_number_grammar();
        if let Some(module) = replacement.as_ref() {
            return self.run_current_module(module);
        }
        // `ModuleAsm` is a public embedder boundary. Its exact profile and
        // command bindings have been admitted above, but the VM cannot claim
        // that its current CompileService produced this assembly. Execute the
        // top level as supplied; mark reusable procedures foreign so their
        // source is lazily recompiled through the current service on entry.
        self.merge_foreign_procs(module);
        let unit = self
            .admitted_foreign_unit(
                Rc::new(module.top_level.clone()),
                module.source_namespace.clone(),
            )
            .with_manifest(module.manifest.clone());
        self.run_compiled_unit(unit)
    }

    /// Run a module just returned by this VM's current compile service.
    pub(crate) fn run_current_module(&mut self, module: &ModuleAsm) -> Completion<Value> {
        self.run_current_module_at(module, None)
    }

    pub(crate) fn run_current_module_at(
        &mut self,
        module: &ModuleAsm,
        location: Option<tcl_runtime_api::script_source_location::ScriptSourceLocation>,
    ) -> Completion<Value> {
        if let Err(error) = self.validate_module_profile(module) {
            return crate::command::completion_from_tcl_error(self, error);
        }
        if let Err(error) =
            Self::validate_module_namespace_bytes(module, &self.source_namespace_path())
        {
            return crate::command::completion_from_tcl_error(self, error);
        }
        self.claim_number_grammar();
        self.merge_procs(module);
        let unit = self
            .compiled_unit(
                Rc::new(module.top_level.clone()),
                module.source_namespace.clone(),
            )
            .with_manifest(module.manifest.clone());
        let unit = if location.is_some() {
            unit.with_source_location(location)
        } else {
            unit
        };
        self.run_compiled_unit(unit)
    }

    /// Run one profile-less bytecode function to completion via the NRE
    /// trampoline.
    ///
    /// A bare [`FunctionAsm`] carries no dialect-profile identity, so this
    /// low-level assembler API is deliberately available only while the VM is
    /// using the permissive fallback profile. Named profiles must enter via
    /// [`run_module`](Self::run_module), whose [`ModuleAsm`] is validated, or
    /// via a VM-owned [`FunctionHandle`](crate::embed::FunctionHandle).
    ///
    /// Deep-copies `asm` into the `Rc` the activation needs. A fallback embedder
    /// invoking the same body repeatedly should hold a
    /// [`FunctionHandle`](crate::embed::FunctionHandle) and call
    /// [`Vm::invoke_function`] instead, which pays that copy once.
    ///
    /// # Panics
    ///
    /// Panics before activation for unresolved foreign native preflight. Use
    /// [`Self::try_run_function`] to retain the typed host admission error.
    pub fn run_function(&mut self, asm: &FunctionAsm) -> Completion<Value> {
        self.try_run_function(asm)
            .expect("function requires a genuine native compiler preflight provider")
    }

    /// Admit a bare function without converting unresolved native preflight
    /// into a catch-visible Tcl error or executing its earlier stores.
    ///
    /// # Errors
    ///
    /// Returns the typed native admission obligation before any activation.
    pub fn try_run_function(
        &mut self,
        asm: &FunctionAsm,
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        self.validate_foreign_native_compilation_entry(asm)?;
        let completion = self.run_admitted_function(asm);
        self.finish_host_execution(completion)
    }

    fn validate_foreign_native_compilation_entry(
        &self,
        asm: &FunctionAsm,
    ) -> Result<(), tcl_runtime_api::NativeCompilationAdmissionError> {
        asm.validate_native_compilation_entry()?;
        let math_matches = self.native_math_table_prerequisite_matches(asm);
        let compilers_match = math_matches.then(|| self.native_compiler_prerequisites_match(asm));
        if !math_matches || compilers_match == Some(false) {
            return Err(tcl_runtime_api::NativeCompilationAdmissionError::NativePreflightRequired);
        }
        Ok(())
    }

    fn run_admitted_function(&mut self, asm: &FunctionAsm) -> Completion<Value> {
        if !self.dialect_profile().is_fallback() {
            return err(format!(
                "profile-less bytecode cannot run under dialect profile {}",
                self.dialect_profile().name
            ));
        }
        let namespace = self.source_namespace_path();
        if self.step_trace_active()
            || !Self::function_resolution_namespace_matches(asm, &namespace)
            || !self.function_command_bindings_match_with_manifest(asm, None)
        {
            return err("stale profile-less bytecode has no source for plain dispatch");
        }
        let unit = self.admitted_foreign_unit(Rc::new(asm.clone()), namespace);
        self.run_compiled_unit(unit)
    }

    pub(crate) fn run_compiled_unit(
        &mut self,
        unit: crate::compiled::CompiledUnit,
    ) -> Completion<Value> {
        if !self.compiled_local_layout_matches(&unit.asm) {
            return self.refuse_host_command(
                "borrowed compiled-local layout is no longer available".into(),
            );
        }
        if !self.native_compiler_prerequisites_match(&unit.asm) {
            return self.refuse_host_command(
                "native compiler preparation prerequisites are no longer available".into(),
            );
        }
        if !self.function_identity_claims_match(&unit.asm, unit.manifest.as_deref()) {
            return self.refuse_host_command(
                "compiled artifact identity or pack claims are no longer available".into(),
            );
        }
        self.run_activation(Frame::new(unit, false))
    }

    /// Run a retained canonical object body through the ordinary control driver.
    pub(crate) fn run_original_object_body(
        &mut self,
        original: Value,
        words: crate::NativeListItems,
    ) -> Completion<Value> {
        let state = crate::cmd_control::ControlState::object_body_eval(original, words);
        let placeholder = self.current_placeholder_unit();
        self.run_activation(Frame::new_control(state, placeholder))
    }

    /// Run one activation stack to completion (the ordinary, non-coroutine path).
    /// The initial frame's `is_proc` decides whether the outermost body is a proc
    /// call (so `unwind` pops a call-frame + namespace and absorbs `return`):
    /// `false` for a module top-level,
    /// `true` for [`invoke_command`](Self::invoke_command) running a proc body.
    fn run_activation(&mut self, initial: Frame) -> Completion<Value> {
        let mut acts = Vec::new();
        self.push_frame(&mut acts, initial);
        match self.drive(&mut acts, DriveMode::Plain) {
            RunExit::Done(c) => c,
            // A `yield` reached at top level (outside a coroutine driver) is
            // rejected by the `yield` builtin before it sets `coro.pending`, so
            // `Plain` mode never observes a suspend.
            RunExit::Yielded(_) => unreachable!("Plain-mode drive cannot suspend"),
        }
    }

    /// Drive the NRE trampoline over `acts` until it empties (`RunExit::Done`) or,
    /// in [`DriveMode::CoroDriver`], a `yield`/`yieldto` suspends the coroutine
    /// (`RunExit::Yielded`, `acts` left frozen). Both `run_activation` and a
    /// coroutine's `resume` funnel through here so the trampoline logic
    /// (`enter_proc`/`unwind`/`dispatch_tailcall`/inline-loop redirection) is shared.
    ///
    /// `activation_depth` counts nested `drive` invocations — the host re-entry
    /// counter `yield` consults to reject a suspend across a `catch`/`uplevel`/
    /// `eval`/OO-method boundary (`cannot yield: C stack busy`).
    fn drive(&mut self, acts: &mut Vec<Frame>, mode: DriveMode) -> RunExit {
        let current_evaluations = self
            .jim_errors
            .frames
            .iter()
            .map(Self::borrow_jim_evaluation_frame)
            .collect();
        let previous_outer = std::mem::replace(&mut self.jim_errors.outer, current_evaluations);
        self.activation_depth += 1;
        let exit = self.drive_loop(acts, mode);
        let errors = &mut self.jim_errors;
        errors.frames = errors
            .outer
            .iter()
            .map(Self::borrow_jim_evaluation_frame)
            .collect();
        self.jim_errors.outer = previous_outer;
        self.activation_depth -= 1;
        exit
    }

    /// Drive a coroutine's activation stack until it suspends (`yield`) or
    /// completes — the `resume` entry point (`cmd_coro`). Runs in
    /// [`DriveMode::CoroDriver`] so a `Tick::Suspend` freezes `acts` and returns
    /// `RunExit::Yielded` instead of erroring.
    pub(crate) fn drive_coro(&mut self, acts: &mut Vec<Frame>) -> RunExit {
        self.drive(acts, DriveMode::CoroDriver)
    }

    fn refresh_jim_evaluation_frames(&mut self, acts: &[Frame]) {
        if self.uses_jim_error_stack() {
            let errors = &mut self.jim_errors;
            errors.frames = errors
                .outer
                .iter()
                .map(Self::borrow_jim_evaluation_frame)
                .collect();
            self.jim_errors.frames.extend(
                acts.iter()
                    .map(|frame| Self::borrow_jim_evaluation_frame(&frame.jim_evaluation)),
            );
        }
    }

    fn push_frame(&mut self, acts: &mut Vec<Frame>, mut frame: Frame) {
        if self.uses_jim_error_stack() {
            let parent = acts
                .last()
                .map(|frame| &frame.jim_evaluation)
                .or_else(|| self.jim_errors.outer.last());
            frame.jim_evaluation.procedure_level =
                parent.map_or(0, |frame| frame.procedure_level) + u32::from(frame.is_proc);
            self.jim_errors.stack.mark_reset();
        }
        for required in frame.asm.instructions.iter().filter_map(|instruction| {
            instruction
                .native_compiler_selection
                .as_ref()
                .map(|site| &site.prerequisite)
        }) {
            if required.guard() == tcl_runtime_api::CommandBindingGuard::ChunkEntry
                && self.native_compiler_selection_prerequisite_matches(required)
            {
                frame
                    .chunk_native_compiler_selections
                    .insert(required.clone());
            }
        }
        for (instruction_index, instruction) in frame.asm.instructions.iter().enumerate() {
            for (site_index, site) in instruction.native_operation_selections.iter().enumerate() {
                if site
                    .compiler_selection_prerequisite()
                    .as_ref()
                    .is_none_or(|required| {
                        required.guard() != tcl_runtime_api::CommandBindingGuard::ChunkEntry
                            || self.native_compiler_selection_prerequisite_matches(required)
                    })
                    && site
                        .requirements
                        .iter()
                        .filter(|binding| {
                            binding.guard == tcl_runtime_api::CommandBindingGuard::ChunkEntry
                        })
                        .all(|binding| self.command_binding_matches(binding))
                {
                    frame
                        .chunk_native_operations
                        .insert((instruction_index, site_index));
                }
            }
        }
        for binding in frame.asm.instructions.iter().filter_map(|instruction| {
            instruction
                .entered_command
                .as_ref()
                .map(|entered| &entered.binding)
        }) {
            if binding.guard != tcl_runtime_api::CommandBindingGuard::ChunkEntry
                || frame.chunk_commands.contains_key(binding)
            {
                continue;
            }
            if let Some(captured) = self.capture_native_command(binding) {
                frame.chunk_commands.insert(binding.clone(), captured);
            }
        }
        if let Some(ctx) = self.pending_exec_leave.take() {
            frame.exec_leave.push(ctx);
        }
        acts.push(frame);
    }

    fn settle_pending_exec_leave(&mut self, completion: Completion<Value>) -> Completion<Value> {
        match self.pending_exec_leave.take() {
            Some(ctx) => self
                .finish_exec_leave(&ctx, &completion)
                .unwrap_or(completion),
            None => completion,
        }
    }

    fn settle_exec_leaves(
        &mut self,
        contexts: &mut Vec<ExecLeaveCtx>,
        mut completion: Completion<Value>,
    ) -> Completion<Value> {
        for ctx in contexts.drain(..).rev() {
            if let Some(replacement) = self.finish_exec_leave(&ctx, &completion) {
                completion = replacement;
            }
        }
        completion
    }

    /// Consume every tick shape at the single activation-install boundary.
    /// Both the ordinary trampoline and a command dispatched by `tailcall` use
    /// this path, so a deferred builtin cannot silently look synchronous in one
    /// of them. Each pushed frame takes the trace context for the command that
    /// created it, and each placeholder unit was stamped when the tick was
    /// issued rather than reconstructed here from later VM state.
    fn install_tick(&mut self, acts: &mut Vec<Frame>, tick: Tick) -> TickAction {
        match tick {
            Tick::Continue => TickAction::Resume,
            Tick::Return(completion) => TickAction::Complete(completion),
            Tick::Call {
                proc,
                invoked,
                argv,
                lambda_registration,
            } => match self.enter_proc(&proc.proc, &proc.body, &invoked, &argv) {
                Ok(()) => {
                    self.install_native_procedure_binding(proc.declaration_binding);
                    let mut frame = Frame::new(proc.body, true);
                    // The C bytecode caller keeps its operand stack until
                    // TclNREvalObjv returns (tclExecute.c doInvocation/cleanupV).
                    // Move those same owning references with this activation.
                    frame._procedure_invocation = Some((invoked, argv));
                    frame.lambda_registration = lambda_registration;
                    self.push_frame(acts, frame);
                    TickAction::Resume
                }
                Err(completion) => {
                    if let Some(registration) = lambda_registration {
                        self.retire_original_lambda_registration(&registration);
                    }
                    TickAction::Complete(self.settle_pending_exec_leave(completion))
                }
            },
            Tick::PushScript {
                script,
                label,
                cleanup_proc,
                fatal_tail,
                namespace,
            } => {
                let mut frame = Frame::new_script(*script, label);
                frame.cleanup_proc = cleanup_proc;
                frame.fatal_tail = fatal_tail.or(frame.fatal_tail);
                if let ScriptNamespace::CommandBoundary(namespace) = namespace {
                    match self.enter_replay_namespace(&namespace) {
                        Ok(previous) => frame.replay_namespace_restore = previous,
                        Err(message) => {
                            if let Some(name) = frame.cleanup_proc.take() {
                                self.take_command_unchecked(&name);
                            }
                            return TickAction::Complete(
                                self.settle_pending_exec_leave(err(message)),
                            );
                        }
                    }
                }
                self.push_frame(acts, frame);
                TickAction::Resume
            }
            Tick::PushEval(req) => {
                self.push_frame(acts, Frame::new_eval(*req));
                TickAction::Resume
            }
            Tick::PushCatch(req) => {
                self.push_frame(acts, Frame::new_catch(*req));
                TickAction::Resume
            }
            Tick::PushSubst { req, placeholder } => {
                self.push_frame(acts, Frame::new_subst(*req, *placeholder));
                TickAction::Resume
            }
            Tick::PushEachLoop { req, placeholder } => {
                self.push_frame(acts, Frame::new_each_loop(*req, *placeholder));
                TickAction::Resume
            }
            Tick::PushExpression { req, placeholder } => {
                self.push_frame(acts, Frame::new_expression(*req, *placeholder));
                TickAction::Resume
            }
            Tick::PushControl { state, placeholder } => {
                self.push_frame(acts, Frame::new_control(*state, *placeholder));
                TickAction::Resume
            }
            Tick::PushTry {
                req,
                initial_options,
            } => {
                self.push_frame(acts, Frame::new_try(*req, initial_options));
                TickAction::Resume
            }
            Tick::Tailcall(request) => {
                TickAction::Complete(match self.schedule_tailcall(request) {
                    Ok(()) => Completion::new(Code::Return, Value::empty(), Value::empty()),
                    Err(completion) => completion,
                })
            }
            Tick::Suspend(req) => TickAction::Suspend(req),
        }
    }

    fn drive_loop(&mut self, acts: &mut Vec<Frame>, mode: DriveMode) -> RunExit {
        loop {
            if self.execution_refusal.is_some() {
                return RunExit::Done(self.abort_refused_activations(acts));
            }
            if acts
                .last()
                .is_some_and(|frame| !self.compiled_local_layout_matches(&frame.asm))
            {
                let _ = self.refuse_host_command(
                    "borrowed compiled-local layout is no longer available".into(),
                );
                return RunExit::Done(self.abort_refused_activations(acts));
            }
            // Enforce `interp limit $i time` for unbounded bytecode loops: the
            // counter is Vm-scoped so it survives the short activations a
            // command-driven loop re-enters (see `limit_check_tick`).
            if let Some(c) = self.limit_check_tick() {
                if let Some(done) = self.unwind(acts, c) {
                    return RunExit::Done(done);
                }
                continue;
            }
            match self.unwind_stale_compilation_frame(acts, None) {
                Ok(false) => {}
                Ok(true) => continue,
                Err(exit) => return exit,
            }
            if let Some(completion) = self.native_script_parse_failure(acts) {
                if let Some(done) = self.settle_completion(acts, completion) {
                    return RunExit::Done(done);
                }
                continue;
            }
            self.refresh_jim_evaluation_frames(acts);
            let mut tick = {
                let top = acts.last_mut().expect("activation stack is non-empty");
                self.tick(top)
            };
            if self.execution_refusal.is_some() {
                self.restore_execution_frame(tick.take_selected_frame_restore());
                return RunExit::Done(self.abort_refused_activations(acts));
            }
            // Uplevel has selected its variable frame, while its child
            // activation still belongs to this tick. The parent consumes the
            // namespace retained by that selection, not the child's namespace.
            let parent_namespace = tick
                .selected_frame_restore()
                .and_then(crate::interp::SelectedFrameRestore::original_namespace)
                .map(|namespace| self.namespace_path_for_token(namespace));
            if parent_namespace.as_ref().is_some_and(|namespace| {
                self.top_activation_stale_message_in(acts, namespace)
                    .is_some()
            }) {
                // A failed handoff must restore the selected physical frame
                // before the parent's catch/unwind consumes the failure.
                self.restore_execution_frame(tick.take_selected_frame_restore());
            }
            match self.unwind_stale_compilation_frame(acts, parent_namespace.as_ref()) {
                Ok(false) => {}
                Ok(true) => continue,
                Err(exit) => return exit,
            }
            match self.install_tick(acts, tick) {
                TickAction::Resume => {}
                TickAction::Complete(completion) => {
                    if let Some(done) = self.settle_completion(acts, completion) {
                        return RunExit::Done(done);
                    }
                }
                TickAction::Suspend(req) => {
                    if let Some(exit) = self.handle_suspend(acts, mode, req) {
                        return exit;
                    }
                }
            }
        }
    }

    fn native_script_parse_failure(&mut self, acts: &mut [Frame]) -> Option<Completion<Value>> {
        if self.native_invocation_dialect().script_parse_timing()
            != Some(tcl_registry::invocation_words::NativeScriptParseTiming::BeforeScript)
        {
            return None;
        }
        let frame = acts.last_mut()?;
        if frame.pc != 0 {
            return None;
        }
        let tail = frame.fatal_tail.take()?;
        let level = frame.jim_evaluation.procedure_level;
        let location = frame.source_location.clone();
        self.refresh_jim_evaluation_frames(&acts[..acts.len() - 1]);
        Some(self.raise_jim_script_parse_failure(tail, level, location))
    }

    /// Restore hidden frame owners before a parked coroutine is destroyed.
    /// Walk innermost first so nested selections retire only their own children.
    pub(crate) fn restore_parked_execution_frames(&mut self, acts: &mut [Frame]) {
        for activation in acts.iter_mut().rev() {
            self.restore_execution_frame(activation.selected_frame_restore.take());
        }
    }

    /// Retire temporary lambda publications after the parked locals have unwound.
    pub(crate) fn retire_parked_lambda_registrations(&mut self, acts: &mut [Frame]) {
        for activation in acts.iter_mut().rev() {
            if let Some(registration) = activation.lambda_registration.take() {
                self.retire_original_lambda_registration(&registration);
            }
        }
    }

    /// Retire only the actual publication, preserving a later same-name replacement.
    fn retire_original_lambda_registration(
        &mut self,
        registration: &crate::interp::CommandSidecarHandle,
    ) {
        if let Some(key) = registration.key() {
            self.retire_command_lifecycle_key(&key);
        }
    }

    /// Retire engine storage without catch/try/finally, completion presentation,
    /// or guest leave/unset callbacks. The returned carrier is internal only;
    /// the host boundary returns the retained typed refusal instead.
    fn abort_refused_activations(&mut self, acts: &mut Vec<Frame>) -> Completion<Value> {
        while let Some(mut activation) = acts.pop() {
            activation.close_native_script();
            if activation.is_proc {
                self.release_native_procedure_execution();
                self.pop_call_frame();
                self.pop_ns();
            } else if let Some(previous) = activation.replay_namespace_restore.take() {
                self.leave_replay_namespace(previous);
            }
            self.restore_execution_frame(activation.selected_frame_restore.take());
            for context in activation.exec_leave {
                self.pop_exec_step_scopes(context.step_scopes);
            }
            if let Some(name) = activation.cleanup_proc {
                self.take_command_unchecked(&name);
            }
            if let Some(registration) = activation.lambda_registration {
                self.retire_original_lambda_registration(&registration);
            }
        }
        if let Some(req) = self.pending.procedure_call.take() {
            self.retire_original_lambda_registration(&req.registration);
        }
        self.pending = crate::interp::PendingControl::default();
        if let Some(context) = self.pending_exec_leave.take() {
            self.pop_exec_step_scopes(context.step_scopes);
        }
        err("")
    }

    fn handle_suspend(
        &mut self,
        acts: &mut Vec<Frame>,
        mode: DriveMode,
        req: YieldReq,
    ) -> Option<RunExit> {
        match mode {
            // A coroutine `yield`/`yieldto` freezes `acts` in place (pc already
            // past the suspend point) and hands the request out to its resume.
            DriveMode::CoroDriver => {
                if let Some(message) =
                    self.activation_stack_stale_message(acts, &self.source_namespace_path())
                {
                    // A freshly compiled try handler/finally can sit above an
                    // activation made stale by the command that selected it.
                    // Never park that mixed stack.
                    return self
                        .settle_stale_activation(acts, message)
                        .map(RunExit::Done);
                }
                Some(RunExit::Yielded(req))
            }
            // Defensive: the `yield` builtin's boundary check rejects a
            // top-level suspend (`cannot yield: C stack busy`) before here.
            DriveMode::Plain => self
                .unwind(acts, err("cannot yield: C stack busy"))
                .map(RunExit::Done),
        }
    }

    /// Reject a frame whose profile or compile service changed while it was
    /// live. Stored activations can have a PC and operand stack in the middle
    /// of a lowered command, so they cannot be safely recompiled like an
    /// unentered proc.
    /// `Ok(true)` means unwinding consumed the stale frame and the caller must
    /// continue its drive loop; an empty stack is returned as a completed run.
    fn unwind_stale_compilation_frame(
        &mut self,
        acts: &mut Vec<Frame>,
        namespace: Option<&tcl_core_types::ByteNamespacePath>,
    ) -> Result<bool, RunExit> {
        let current_profile = self.profile_generation();
        let current_compiler = self.compiler_generation();
        if let Some(frame) = acts.last_mut()
            && (frame.profile_generation != current_profile
                || frame
                    .compiler_generation
                    .is_some_and(|generation| generation != current_compiler))
            && frame.each_loop.is_some()
        {
            // The foreach/lmap driver contains no bytecode or profile-sensitive
            // parser state: list grouping is invariant and variable writes use
            // the VM's live semantics. Its reusable compiled body owns the
            // originating profile separately, so it is safe to advance the
            // driver itself and let the next body activation enforce freshness.
            frame.profile_generation = current_profile;
            frame.compiler_generation = frame.compiler_generation.map(|_| current_compiler);
        }
        let stale_message = namespace.map_or_else(
            || self.top_activation_stale_message(acts),
            |namespace| self.top_activation_stale_message_in(acts, namespace),
        );
        if let Some(message) = stale_message {
            // This is a command-like failure at the current instruction
            // boundary. Route it through the ordinary settlement seam so an
            // enclosing inline `catch` observes it before the activation is
            // unwound.
            return match self.settle_completion(acts, err(message)) {
                Some(done) => Err(RunExit::Done(done)),
                None => Ok(true),
            };
        }
        Ok(false)
    }

    /// Validate every frozen activation before a coroutine is resumed or
    /// parked. A current handler/finally frame must not hide a stale ancestor.
    pub(crate) fn activation_stack_stale_message(
        &self,
        acts: &[Frame],
        current_namespace: &tcl_core_types::ByteNamespacePath,
    ) -> Option<&'static str> {
        let current_profile = self.profile_generation();
        let current_compiler = self.compiler_generation();
        acts.iter()
            .find_map(|frame| {
                frame.stale_compilation_message(
                    current_profile,
                    current_compiler,
                    &self.native_compiler_policy(),
                )
            })
            .or_else(|| {
                acts.last()
                    .and_then(|frame| frame.namespace_stale_message(current_namespace))
            })
    }

    /// The single activation-boundary freshness query. Scanner-only drivers
    /// declare their exemption in `Frame::stale_compilation_message`; every
    /// execution, unwind, resume, and suspension boundary consults this owner.
    fn top_activation_stale_message(&self, acts: &[Frame]) -> Option<&'static str> {
        self.top_activation_stale_message_in(acts, &self.source_namespace_path())
    }

    fn top_activation_stale_message_in(
        &self,
        acts: &[Frame],
        current_namespace: &tcl_core_types::ByteNamespacePath,
    ) -> Option<&'static str> {
        let current_profile = self.profile_generation();
        let current_compiler = self.compiler_generation();
        acts.last().and_then(|frame| {
            frame
                .stale_compilation_message(
                    current_profile,
                    current_compiler,
                    &self.native_compiler_policy(),
                )
                .or_else(|| frame.namespace_stale_message(current_namespace))
        })
    }

    fn settle_stale_activation(
        &mut self,
        acts: &mut Vec<Frame>,
        message: &'static str,
    ) -> Option<Completion<Value>> {
        self.settle_completion(acts, err(message))
    }

    /// Reject a frozen coroutine through the ordinary settlement/unwind path.
    /// Plain drive mode turns any attempted yield during cleanup into another
    /// unwinding error, so a stale continuation can never become suspended
    /// again.
    pub(crate) fn unwind_stale_coroutine(
        &mut self,
        acts: &mut Vec<Frame>,
        message: &'static str,
    ) -> RunExit {
        match self.settle_stale_activation(acts, message) {
            Some(done) => RunExit::Done(done),
            None => self.drive(acts, DriveMode::Plain),
        }
    }

    /// A completion is about to cross from a child activation into its parent.
    /// A stale parent owns the boundary, so replace even an OK/return completion
    /// with its provenance error before catch or loop settlement sees it.
    fn validate_unwind_boundary(&self, acts: &[Frame], c: Completion<Value>) -> Completion<Value> {
        self.top_activation_stale_message(acts).map_or(c, err)
    }

    /// Settle a frame's exceptional completion (`Tick::Return`): a live catch
    /// range in the faulting frame absorbs it first (C's catchStack; it defers
    /// internally when an inline loop is nested inside the range); then a
    /// `break`/`continue` *returned by a command* (`if {…} $z`, `eval break`)
    /// inside an inline loop body jumps to the loop's exit/continue point
    /// rather than unwinding out of the function (the inline `JUMP` only
    /// covers a *literal* break/continue — this mirrors C Tcl's loop exception
    /// ranges); anything still unhandled unwinds. `Some(done)` ends the drive.
    fn settle_completion(
        &mut self,
        acts: &mut Vec<Frame>,
        c: Completion<Value>,
    ) -> Option<Completion<Value>> {
        let c = match self.publish_native_interp_completion(c) {
            Ok(completion) => completion,
            Err(error) => {
                return Some(crate::command::completion_from_tcl_error(
                    self,
                    error.into(),
                ));
            }
        };
        if c.code == Code::Error {
            self.observe_native_error_result(&c.result);
        }
        if c.code == Code::Error && self.uses_jim_error_stack() {
            self.refresh_jim_evaluation_frames(acts);
            self.capture_jim_error_stack();
            if let Some(frame) = acts.last_mut() {
                frame.jim_evaluation.invocation = Value::empty();
            }
        }
        if c.code == Code::Error
            && let Some((text, line, context)) = acts.last().and_then(|frame| {
                frame
                    .asm
                    .instructions
                    .get(frame.pc.saturating_sub(1))
                    .map(|instruction| {
                        let context = match instruction.error_stack_context.as_ref() {
                            Some(ErrorStackContext::CommandResult { head, .. }) => {
                                Value::list(vec![Value::string(head.as_str()), c.result.clone()])
                            }
                            Some(ErrorStackContext::ReturnImmediate { .. }) | None => {
                                Value::from_native_string_bytes(
                                    instruction.source_cmd_text.bytes().to_vec(),
                                )
                            }
                        };
                        let text = match instruction.error_stack_context.as_ref() {
                            Some(ErrorStackContext::CommandResult {
                                error_info_command, ..
                            }) => error_info_command.clone(),
                            Some(ErrorStackContext::ReturnImmediate { error_info_command })
                                if !error_info_command.is_empty() =>
                            {
                                error_info_command.clone()
                            }
                            Some(ErrorStackContext::ReturnImmediate { .. }) | None => {
                                instruction.source_cmd_text.bytes().to_vec()
                            }
                        };
                        (text, instruction.source_line, context)
                    })
            })
        {
            let message = c.result.string_bytes();
            self.log_command_info_with_context(&text, context, &message, line);
        }
        let c = match self
            .absorb_catch_range(acts.last_mut().expect("activation stack is non-empty"), c)
        {
            Ok(()) => return None,
            Err(c) => c,
        };
        if matches!(c.code, Code::Break | Code::Continue)
            && Self::catch_loop_completion(acts.last_mut().expect("still non-empty"), c.code)
        {
            return None;
        }
        self.unwind(acts, c)
    }

    /// If the instruction that just produced a `break`/`continue` completion is
    /// inside an inline loop body (per `FunctionAsm::loop_targets`), redirect the
    /// frame's `pc` to the loop's break / continue target and return `true`;
    /// otherwise return `false` (the completion keeps unwinding). The stack is at
    /// a statement boundary here (the failing command consumed its args and
    /// pushed no result), matching what the loop-end / header expects.
    fn catch_loop_completion(f: &mut Frame, code: Code) -> bool {
        let idx = f.pc.saturating_sub(1);
        let Some(&(brk, cont)) = f.asm.loop_targets.get(&idx) else {
            return false;
        };
        let target = if code == Code::Break { brk } else { cont };
        let Some(off) = target else {
            return false; // no target (e.g. a for-step continue) → propagate
        };
        let Some(&tidx) = f.off2idx.get(&off) else {
            return false;
        };
        f.pc = tidx;
        true
    }

    /// As an error unwinds out of activation `act`, synthesise the `errorInfo`
    /// body frames for every inlined command body (`FunctionAsm::error_regions`)
    /// covering the failing instruction — the compiled analogue of C's
    /// per-command `CmdFrame`. Innermost region first (a deeper region has the
    /// larger start): each appends its `("LABEL" body line N)` frame — the
    /// failing command's line made body-relative — then its enclosing command's
    /// `invoked from within "…"` frame, whose line then drives the next-outer
    /// frame (an enclosing region, or this activation's proc frame).
    fn apply_error_regions(&mut self, act: &Frame) {
        if act.asm.error_regions.is_empty() {
            return;
        }
        // The failing instruction's source span — a region covers it when the
        // span lies within the region's (the enclosing command's). Containment,
        // not an index range, so an interleaved non-body instruction is excluded
        // and layout reordering is irrelevant.
        let Some(span) = act
            .asm
            .instructions
            .get(act.pc.saturating_sub(1))
            .and_then(|i| i.source_span)
        else {
            return;
        };
        let (s, e) = (span.start(), span.end());
        let mut covering: Vec<&ErrorRegion> = act
            .asm
            .error_regions
            .iter()
            .filter(|r| r.start <= s && e <= r.end)
            .collect();
        // Innermost first: a deeper body has the larger start (ties broken by the
        // smaller end). Each appends its body frame then its enclosing command's
        // `invoked from within` frame, whose line drives the next-outer frame.
        covering.sort_by(|a, b| b.start.cmp(&a.start).then(a.end.cmp(&b.end)));
        for r in covering {
            let body_line = self.error_line().saturating_sub(r.line_base).max(1);
            self.append_body_frame_line(&r.label, body_line);
            self.log_command_info_only(&r.cmd_text, "", r.cmd_line);
        }
    }

    /// Unwind one or more activations with completion `c`. Returns `Some` when
    /// the whole `run` is finished, `None` when a parent activation resumed.
    /// Offer an exceptional completion to `f`'s innermost live catch range
    /// (C's `TclGetExceptionRangeForPc(catchOnly)` at the faulting pc).
    /// `Ok(())` means the range absorbed it: the operand/foreach/expand stacks
    /// are trimmed to the range's depths, the digested completion is recorded
    /// for the epilogue's `PUSH_RESULT`/`PUSH_RETURN_CODE`/`PUSH_RETURN_OPTS`,
    /// and `pc` moves to the handler. `Err(c)` gives the completion back:
    /// no live range, an uncatchable `exit`, or a `break`/`continue` whose
    /// inline loop is nested *inside* the range (the loop's redirect wins, as
    /// C picks the smallest enclosing exception range).
    fn absorb_catch_range(
        &mut self,
        f: &mut Frame,
        c: Completion<Value>,
    ) -> Result<(), Completion<Value>> {
        if self.exit_pending() {
            return Err(c);
        }
        // Innermost range not already in its handler (an exception inside a
        // handler belongs to the *enclosing* range, per C's pc containment).
        let fault = f.pc.wrapping_sub(1);
        let Some(pos) = f
            .catch_ranges
            .iter()
            .rposition(|r| !r.in_handler && fault >= r.start_idx && fault < r.end_idx)
        else {
            return Err(c);
        };
        if matches!(c.code, Code::Break | Code::Continue) {
            // The faulting instruction sits in an inline loop body iff
            // `loop_targets` has it; the catch range is *inner* to that loop
            // iff its `BEGIN_CATCH4` sits in the same body (same target
            // pair). Otherwise the loop is the smaller range — defer to
            // `catch_loop_completion`.
            let fault = f.pc.wrapping_sub(1);
            if let Some(pair) = f.asm.loop_targets.get(&fault) {
                let begin = f.catch_ranges[pos].begin_idx;
                if f.asm.loop_targets.get(&begin) != Some(pair) {
                    return Err(c);
                }
            }
        }
        // Ranges nested above the firing one belong to abandoned constructs —
        // their `END_CATCH`s are skipped by the handler jump — so drop them;
        // the firing range itself stays (armed) for its own `END_CATCH`.
        f.catch_ranges.truncate(pos + 1);
        let range = f.catch_ranges.last_mut().expect("pos is in bounds");
        range.in_handler = true;
        let (target_idx, stack_len, foreach_len, expand_len) = (
            range.target_idx,
            range.stack_len,
            range.foreach_len,
            range.expand_len,
        );
        f.stack.truncate(stack_len);
        f.foreach_stack.truncate(foreach_len);
        f.expand_markers.truncate(expand_len);
        f.pc = target_idx;
        let options = self
            .digest_catch_options(&c)
            .map_err(|error| crate::command::completion_from_tcl_error(self, error))?;
        let caught = CaughtState {
            result: c.result,
            code: c.code,
            options,
        };
        if let Some(range) = f.catch_ranges.last_mut() {
            range.caught = Some(caught);
        }
        Ok(())
    }

    /// The completion absorbed by the innermost live catch range, which is the
    /// one whose epilogue is running (the reads precede the paired `END_CATCH`).
    /// `None` when no range fired — the fall-through path, an OK completion.
    fn innermost_caught(f: &Frame) -> Option<&CaughtState> {
        f.catch_ranges.last().and_then(|r| r.caught.as_ref())
    }

    /// Add the error frames owned by one completed activation before its proc
    /// boundary is unwound. Keeping this here gives compiled `eval`/`uplevel`
    /// bodies the same single error-context path as every other activation.
    fn apply_activation_error_context(&mut self, act: &mut Frame, acts: &[Frame]) {
        self.apply_error_regions(act);
        let Some(label) = act.body_label else {
            return;
        };
        self.append_body_frame(label);
        if let Some((words, line)) = &act.body_invocation {
            let original = Value::invocation_list_view(words);
            let Some(protocol) = self
                .actual_native_invocation_dialect()
                .native_error_log_protocol()
            else {
                let _ =
                    self.refuse_host_command("tailcall error-frame protocol unavailable".into());
                return;
            };
            let text = match original.native_string_bytes(protocol.string_protocol()) {
                Ok(text) => text,
                Err(error) => {
                    let _ = self.refuse_host_command(error.to_string());
                    return;
                }
            };
            self.log_command_info_with_context(&text, original, "", *line);
            // The entered replacement and the original procedure call are
            // separate Tcl command frames, even though tailcall removed the
            // procedure activation between them.
            self.clear_error_logged();
        }
        // Tcl's uplevel callback appends the body context, then restores the
        // variable frame before its caller logs the enclosing command. The
        // child log owns the UP entry; the enclosing log sees the caller again.
        self.restore_execution_frame(act.selected_frame_restore.take());
        if let Some((cmd, line)) = acts.last().and_then(|parent| {
            parent
                .asm
                .instructions
                .get(parent.pc.saturating_sub(1))
                .map(|instruction| (instruction.source_cmd_text.clone(), instruction.source_line))
        }) {
            self.log_command_info(&cmd, "", line);
        }
    }

    fn settle_activation_parse_tail(&mut self, act: &mut Frame, c: &mut Completion<Value>) {
        if c.code == Code::Ok
            && let Some(tail) = act.fatal_tail.take()
        {
            *c = self.raise_fatal_tail(tail);
        }
    }

    fn unwind_activation_state(
        &mut self,
        act: &mut Frame,
        acts: &[Frame],
        c: &mut Completion<Value>,
    ) -> Option<TailcallReq> {
        self.settle_activation_parse_tail(act, c);
        // An error unwinding through an inlined command body (`eval {…}`)
        // adds the body frames the uncompiled command would, before this
        // activation's own proc frame (innermost first) — the compiled
        // analogue of C's `CmdFrame` trace.
        if c.code == Code::Error {
            self.apply_activation_error_context(act, acts);
        }
        let tailcall = act.is_proc.then(|| self.take_frame_tailcall()).flatten();
        if act.is_proc {
            self.unwind_proc_frame(act, acts, c);
        } else if let Some(previous) = act.replay_namespace_restore.take() {
            self.leave_replay_namespace(previous);
        }
        self.restore_execution_frame(act.selected_frame_restore.take());
        tailcall
    }

    fn settle_unwound_activation(
        &mut self,
        act: &mut Frame,
        acts: &mut Vec<Frame>,
        mut c: Completion<Value>,
    ) -> Result<(Completion<Value>, Option<TailcallReq>), ()> {
        let tailcall = self.unwind_activation_state(act, acts, &mut c);
        // `catch` and `try` still have command-owned completion work below:
        // catch absorbs its body's completion, while try may advance through
        // several handler/finally phase activations. Their execution-trace
        // leave contexts therefore settle only after that work is complete.
        // Every other activation settles here, preserving the established
        // proc-boundary and apply-cleanup ordering.
        let defer_exec_leave = act.catch.is_some()
            || act.try_ctx.is_some()
            || act.control.is_some()
            || act.expression.is_some();
        if !defer_exec_leave && !act.exec_leave.is_empty() {
            c = self.settle_exec_leaves(&mut act.exec_leave, c);
        }
        // `apply`'s temporary lambda proc is torn down here, once its script
        // activation completes — on every completion code, matching a
        // nested-drive `cmd_apply`'s unconditional `vm.take_command` after
        // `eval_source` returns.
        if let Some(name) = act.cleanup_proc.take() {
            self.take_command_unchecked(&name);
        }
        if let Some(registration) = act.lambda_registration.take() {
            self.retire_original_lambda_registration(&registration);
        }
        // A `catch` activation absorbs the body's completion of *any* code:
        // its epilogue binds the result / options variables and yields the
        // status code as an integer, delivered to the parent as this `catch`
        // command's result (`exit` stays uncatchable — `finish_catch` passes
        // it straight through).
        c = match self.settle_control_activation(act, acts, c) {
            Ok(completion) => completion,
            Err(()) => return Err(()),
        };
        // Settle a traced catch only after its body completion has been
        // absorbed into catch's successful integer result. A traced try
        // reaches here only for its final delivered phase; intermediate
        // phases transferred these contexts above. A leave-trace failure
        // now escapes the completed control command instead of being
        // caught or handled by that same command.
        if defer_exec_leave && !act.exec_leave.is_empty() {
            c = self.settle_exec_leaves(&mut act.exec_leave, c);
        }
        Ok((c, tailcall))
    }

    fn unwind(
        &mut self,
        acts: &mut Vec<Frame>,
        mut c: Completion<Value>,
    ) -> Option<Completion<Value>> {
        if self.execution_refusal.is_some() {
            return Some(self.abort_refused_activations(acts));
        }
        // Every direct exceptional hand-off must validate the activation that
        // is about to consume it before that activation is popped. Most ticks
        // are already checked by `drive_loop`, but a synchronous tailcall
        // target can change the compiler and return a non-OK completion from
        // inside `dispatch_tailcall`. Letting that completion enter the pop loop
        // first would discard the stale parent before the ordinary child-to-
        // parent boundary check could see it.
        c = self.validate_unwind_boundary(acts, c);
        loop {
            let mut act = acts.pop().expect("unwinding a non-empty stack");
            // The clean prefix of a partly-malformed body has now run: raise
            // the parse error C raises after it (#1603).  This happens *first*,
            // before any of the completion processing below, so the deferred
            // error is an error for all of it — error-context frames, the proc
            // boundary and leave traces each see code 1 rather than the
            // prefix's `ok`, and `-errorinfo` is built the same way it is for a
            // runtime error in the same position.  An earlier command's own
            // completion wins, exactly as in `catch`/`try`.
            act.close_native_script();
            let Ok((settled, tailcall)) = self.settle_unwound_activation(&mut act, acts, c) else {
                return None;
            };
            c = settled;
            // Crossing an activation boundary is itself a provenance
            // consumption point. In particular, a freshly compiled computed-
            // head try handler/finally must not return through a proc made stale
            // by the try body. The replacement error then follows the parent's
            // ordinary catch-range and loop settlement below.
            c = self.validate_unwind_boundary(acts, c);
            if c.code.is_ok()
                && let Some(request) = tailcall
            {
                return self.dispatch_tailcall(acts, request);
            }
            if let Some(parent) = acts.last_mut() {
                match self.fold_child_completion(parent, c) {
                    ChildCompletion::Resume => return None,
                    ChildCompletion::Unwind(completion) => {
                        c = completion;
                        continue;
                    }
                    ChildCompletion::Bytecode(completion) => c = completion,
                }
            }
            match acts.last_mut() {
                None => return Some(c),
                Some(parent) => {
                    if c.code.is_ok() {
                        parent.jim_evaluation.invocation = Value::empty();
                        parent.last_options = c.options;
                        parent.stack.push(c.result.into_native_reference());
                        return None;
                    }
                    // A live catch range in the parent absorbs the child's
                    // exceptional completion — the child was invoked from
                    // inside the range, so this is C's "command returned
                    // non-OK inside a catch range" path at the parent's
                    // invoke instruction.
                    match self.absorb_catch_range(parent, c) {
                        Ok(()) => return None,
                        Err(back) => c = back,
                    }
                    // A child activation that leaves an unhandled
                    // `break`/`continue` delivers it to the enclosing frame's
                    // loop exactly as an inline completion would. This covers
                    // transparent script bodies, unhandled `try` phases, and a
                    // proc boundary after `return -code break|continue` has
                    // lowered the carried return to its requested completion.
                    // `catch`, `subst`, and runtime-fallback each-loops consume
                    // their own completions above before reaching this hand-off.
                    if matches!(c.code, Code::Break | Code::Continue)
                        && Self::catch_loop_completion(parent, c.code)
                    {
                        return None;
                    }
                    // Error / uncaught Break|Continue / unabsorbed Return keep unwinding.
                }
            }
        }
    }

    /// Consume the original child result through the retained parent's driver.
    /// Synchronous tailcall targets and popped child activations share this
    /// boundary; neither reconstructs a command, token or completion.
    fn fold_child_completion(
        &mut self,
        parent: &mut Frame,
        completion: Completion<Value>,
    ) -> ChildCompletion {
        parent.tailcall_owners.clear();
        if parent.subst.is_some() {
            return match self.fold_subst_bracket(parent, completion) {
                SubstFold::Resume => ChildCompletion::Resume,
                SubstFold::Unwind(completion) => ChildCompletion::Unwind(completion),
            };
        }
        if parent.each_loop.is_some() {
            return match Self::fold_each_loop(parent, completion) {
                EachLoopFold::Resume => ChildCompletion::Resume,
                EachLoopFold::Unwind(completion) => ChildCompletion::Unwind(completion),
            };
        }
        if parent.expression.is_some() {
            return match self.fold_expression_result(parent, completion) {
                Ok(()) => ChildCompletion::Resume,
                Err(completion) => ChildCompletion::Unwind(completion),
            };
        }
        if let Some(state) = parent.jim_script.as_mut() {
            if completion.code == Code::Ok {
                parent.jim_evaluation.invocation = Value::empty();
            }
            state.accept(self, completion);
            return ChildCompletion::Resume;
        }
        if let Some(state) = parent.control.as_mut() {
            state.accept(self, completion);
            return ChildCompletion::Resume;
        }
        ChildCompletion::Bytecode(completion)
    }

    /// Deliver a tailcall target after the issuing procedure has retired.
    fn deliver_tailcall_completion(
        &mut self,
        acts: &mut Vec<Frame>,
        completion: Completion<Value>,
    ) -> Option<Completion<Value>> {
        let completion = self.validate_unwind_boundary(acts, completion);
        let Some(parent) = acts.last_mut() else {
            return Some(match self.publish_native_interp_completion(completion) {
                Ok(completion) => completion,
                Err(error) => crate::command::completion_from_tcl_error(self, error.into()),
            });
        };
        match self.fold_child_completion(parent, completion) {
            ChildCompletion::Resume => None,
            ChildCompletion::Unwind(completion) => self.unwind(acts, completion),
            ChildCompletion::Bytecode(completion) if completion.code == Code::Ok => {
                parent.jim_evaluation.invocation = Value::empty();
                parent.last_options = completion.options;
                parent.stack.push(completion.result.into_native_reference());
                None
            }
            ChildCompletion::Bytecode(completion) => self.settle_completion(acts, completion),
        }
    }

    fn settle_control_activation(
        &mut self,
        act: &mut Frame,
        acts: &mut Vec<Frame>,
        mut c: Completion<Value>,
    ) -> Result<Completion<Value>, ()> {
        if let Some(ctx) = act.catch.take() {
            if c.code == Code::Ok
                && let Some(tail) = ctx.fatal_tail
            {
                c = self.raise_fatal_tail(tail);
            }
            c = self.finish_catch(
                c,
                ctx.resvar.as_ref(),
                ctx.optvar.as_ref(),
                ctx.ignored_codes,
            );
        }
        // A try-phase activation (body/handler/`finally`) just completed:
        // `advance_try` decides whether another phase follows (pushed here,
        // directly — `act` itself is already gone, unlike `each_loop`'s
        // parent-stays-put pattern) or the whole `try` is done, in which
        // case `c` becomes its completion and unwinding continues below
        // exactly as for any other completed activation.
        if let Some(mut ctx) = act.try_ctx.take() {
            if c.code == Code::Ok
                && let Some(tail) = ctx.fatal_tail.take()
            {
                c = self.raise_fatal_tail(tail);
            }
            match crate::cmd_try::advance_try(self, *ctx, c) {
                crate::cmd_try::TryOutcome::Push(req) => {
                    let mut next = Frame::new_try(*req, Value::empty());
                    next.exec_leave.append(&mut act.exec_leave);
                    self.push_frame(acts, next);
                    return Err(());
                }
                crate::cmd_try::TryOutcome::Deliver(fc) => c = fc,
            }
        }
        Ok(c)
    }

    fn fold_expression_result(
        &mut self,
        parent: &mut Frame,
        mut c: Completion<Value>,
    ) -> Result<(), Completion<Value>> {
        if c.code == Code::Ok
            && let Some(base) = parent
                .expression
                .as_mut()
                .expect("expression state present")
                .awaiting_array
                .take()
        {
            let key = match self.native_name_operand_bytes(&c.result) {
                Ok(key) => key,
                Err(_refusal) => {
                    return Err(crate::command::completion_from_tcl_error(
                        self,
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "native array index",
                        )
                        .into(),
                    ));
                }
            };
            c = match self.read_variable_result_bytes(&base, Some(&key)) {
                Ok(value) => ok(value),
                Err(completion) => completion,
            };
        }
        if c.code == Code::Ok {
            parent.last_options = c.options;
            parent
                .expression
                .as_mut()
                .expect("expression state present")
                .state
                .resume(c.result);
            return Ok(());
        }
        Err(c)
    }

    /// Unwind one proc activation `act`: on error add its `(procedure "name" line
    /// N)` errorInfo frame, pop the call-frame + namespace, apply
    /// `TclUpdateReturnInfo` (a proc boundary decrements a carried `-code`/`-level`
    /// return), and on error log the caller's `invoked from
    /// within "…"` frame. Mutates `c` in place. Split out of [`Vm::unwind`].
    fn unwind_proc_frame(&mut self, act: &Frame, acts: &[Frame], c: &mut Completion<Value>) {
        self.release_native_procedure_execution();
        // C's `InterpProcNR2` proc epilogue (`tclProc.c:1864`): a proc body that
        // reaches its boundary with a bare `break`/`continue` — i.e. a
        // `break`/`continue` or `return -level 0 -code break` that produced
        // `TCL_BREAK`/`TCL_CONTINUE` directly, *not* a `return -code break` that
        // the level-decrement below turns into break — is transformed into the
        // error `invoked "break" outside of a loop` with errorcode
        // `TCL RESULT UNEXPECTED`.  Done before the `(procedure …)` frame is
        // appended so the error is logged with the proc's trace, exactly as C's
        // `errorProc` does after the transform.
        if matches!(c.code, Code::Break | Code::Continue) {
            let word = if c.code == Code::Break {
                "break"
            } else {
                "continue"
            };
            // Report the offending command's own line in the proc frame (C's
            // `iPtr->errorLine`), taken from the instruction that produced the
            // completion.
            if let Some(line) = act
                .asm
                .instructions
                .get(act.pc.saturating_sub(1))
                .map(|i| i.source_line)
                .filter(|&l| l != 0)
            {
                self.set_error_line(line);
            }
            *c = crate::command::err_with_code(
                format!("invoked \"{word}\" outside of a loop"),
                "TCL RESULT UNEXPECTED",
            );
            self.seed_error_info(format!("invoked \"{word}\" outside of a loop"));
        }
        // The `(procedure … line N)` frame reports the body-relative line of the
        // failing command. A runtime-compiled proc body (the common case) carries
        // body-relative instruction lines with `body_base_line == 0`, so the line
        // is used as-is; a proc compiled inside a module carries absolute lines, so
        // subtract its definition line (`absolute − base + 1`).
        if c.code == Code::Error
            && let Some(name) = self.current_proc_name()
        {
            let base = act.asm.body_base_line;
            let n = if base == 0 {
                self.error_line().max(1)
            } else {
                self.error_line()
                    .saturating_sub(base)
                    .saturating_add(1)
                    .max(1)
            };
            self.append_proc_frame(&name, n);
        }
        self.pop_call_frame();
        self.pop_ns();
        if c.code == Code::Return && self.uses_jim_error_stack() {
            c.code = self.jim_errors.pending_return.settle_procedure(c.code);
            c.options = self.jim_return_options(c.code);
        } else if c.code == Code::Return
            && matches!(
                c.option_origin,
                tcl_core_types::CompletionOptionOrigin::MergedReturnOptions { .. }
            )
        {
            let tcl_core_types::CompletionOptionOrigin::MergedReturnOptions { code, level } =
                c.option_origin
            else {
                unreachable!()
            };
            let level = level.saturating_sub(1);
            c.option_origin =
                tcl_core_types::CompletionOptionOrigin::MergedReturnOptions { code, level };
            self.settle_native_c_return_level(level);
            if level == 0 {
                c.code = Code::from_int(code);
                if c.code == Code::Error {
                    self.clear_error_logged();
                }
            }
        } else if c.code == Code::Return {
            // While the carried return level stays positive the TCL_RETURN keeps
            // unwinding one proc level at a time; when it reaches 0 the carried
            // `-code` takes effect. `Code::from_int` (never a 0..=4-only map):
            // `return -code N` for a non-standard N must surface `Code::Other(N)`,
            // not collapse to `Ok` (coroutine-2.4).
            let level = match crate::command::option_integer_checked(self, &c.options, b"-level", 1)
            {
                Ok(level) => level,
                Err(error) => {
                    *c = crate::command::completion_from_tcl_error(self, error);
                    return;
                }
            };
            if level > 1 {
                c.options = match crate::command::with_return_level(self, &c.options, level - 1) {
                    Ok(options) => options,
                    Err(error) => {
                        *c = crate::command::completion_from_tcl_error(self, error);
                        return;
                    }
                };
                self.settle_native_c_return_level(level - 1);
            } else {
                let code = match crate::command::option_code_checked(self, &c.options, Code::Ok) {
                    Ok(code) => code,
                    Err(error) => {
                        *c = crate::command::completion_from_tcl_error(self, error);
                        return;
                    }
                };
                c.options = match crate::command::with_return_level(self, &c.options, 0) {
                    Ok(options) => options,
                    Err(error) => {
                        *c = crate::command::completion_from_tcl_error(self, error);
                        return;
                    }
                };
                c.code = code;
                self.settle_native_c_return_level(0);
                if code == Code::Error {
                    // A carried `-errorinfo` suppresses the `return` command's
                    // own frame, but the call site that receives the settled
                    // error must still be appended.
                    self.clear_error_logged();
                }
            }
        }
        if c.code == Code::Error && self.uses_jim_error_stack() {
            self.refresh_jim_evaluation_frames(acts);
            self.capture_jim_error_stack();
        }
        if c.code == Code::Error
            && let Some((cmd, line)) = acts.last().and_then(|parent| {
                parent
                    .asm
                    .instructions
                    .get(parent.pc.saturating_sub(1))
                    .map(|i| (i.source_cmd_text.clone(), i.source_line))
            })
        {
            self.log_command_info(&cmd, "", line);
        }
    }

    /// Fold a subst `[…]` bracket body's completion into the parent subst frame's
    /// scan, per C's per-bracket `subst` rules (subst-8.x/10.x): `Ok`/`Return`/any
    /// other code appends the value and `continue` drops it (both `Resume` the
    /// scan — the caller re-ticks the subst frame); a `break` finalises the result
    /// with the output accumulated so far and an error propagates (both `Unwind`,
    /// dropping the subst frame). The scan state is cleared on `Unwind` so the
    /// dropped frame delivers its `ok`/error result normally.
    fn fold_subst_bracket(&mut self, parent: &mut Frame, mut c: Completion<Value>) -> SubstFold {
        let state = parent.subst.as_mut().expect("subst state present");
        if let Some(base) = state.pending_array.take()
            && c.code == Code::Ok
        {
            let key = match self.native_name_operand_bytes(&c.result) {
                Ok(key) => key,
                Err(refusal) => {
                    parent.subst = None;
                    return SubstFold::Unwind(crate::command::completion_from_tcl_error(
                        self,
                        tcl_syntax::value::ValueError::NativeStringAccess(
                            tcl_syntax::raw_string::NativeStringAccessError::Unavailable(refusal),
                        )
                        .into(),
                    ));
                }
            };
            c = match self.read_variable_result_bytes(&base, Some(&key)) {
                Ok(value) => ok(value),
                Err(completion) => completion,
            };
        }
        if let crate::subst::SubstitutionControl::Expression(policy) = state.control {
            c = crate::subst::settle_expression_quote(self, c, policy);
            if c.code == Code::Ok {
                parent.last_options = c.options.clone();
            }
        }
        if state.control != crate::subst::SubstitutionControl::Command && c.code != Code::Ok {
            parent.subst = None;
            return SubstFold::Unwind(c);
        }
        match c.code {
            Code::Ok | Code::Return | Code::Other(_) => {
                let v = match self.native_name_operand_bytes(&c.result) {
                    Ok(bytes) => bytes,
                    Err(refusal) => {
                        parent.subst = None;
                        return SubstFold::Unwind(crate::command::completion_from_tcl_error(
                            self,
                            tcl_syntax::value::ValueError::NativeStringAccess(
                                tcl_syntax::raw_string::NativeStringAccessError::Unavailable(
                                    refusal,
                                ),
                            )
                            .into(),
                        ));
                    }
                };
                parent
                    .subst
                    .as_mut()
                    .expect("subst state")
                    .out
                    .extend_from_slice(&v);
                SubstFold::Resume
            }
            Code::Continue => SubstFold::Resume,
            Code::Break => {
                let out = std::mem::take(&mut parent.subst.as_mut().expect("subst state").out);
                parent.subst = None;
                SubstFold::Unwind(ok(Value::from_string_bytes(out)))
            }
            Code::Error => {
                parent.subst = None;
                SubstFold::Unwind(c)
            }
        }
    }

    /// Push a call-frame and bind `argv` to the proc's parameters.
    fn procedure_argument_bindings(
        &mut self,
        proc: &ProcDef,
        argv: &[Value],
    ) -> Result<Vec<tcl_syntax::formal_params::FormalByteArgumentBinding>, Completion<Value>> {
        tcl_syntax::formal_params::bind_formal_argument_bytes(
            &proc.formal_parameters(),
            argv.len(),
            proc.parameter_grammar,
        )
        .map_err(|_| match proc_usage(self, proc) {
            Ok(message) => crate::command::native_wrong_arguments_message(self, message),
            Err(error) => error,
        })
    }

    /// Native Jim validates arity and handles an empty body before creating
    /// any frame or acquiring a Script activation. C prepares its body first.
    fn early_procedure_activation(
        &mut self,
        proc: &ProcDef,
        argv: &[Value],
    ) -> Result<Option<Completion<Value>>, Completion<Value>> {
        let Some(protocol) = tcl_registry::native_procedure::procedure_activation_protocol(
            self.native_invocation_dialect(),
        ) else {
            return Err(self.refuse_host_command(
                "native procedure activation protocol is unavailable".into(),
            ));
        };
        if protocol.validates_arguments_before_body() {
            self.procedure_argument_bindings(proc, argv)?;
        }
        if protocol.empty_body_skips_activation() {
            let body = self
                .native_name_operand_bytes(&proc.body_src)
                .map_err(|error| self.refuse_host_command(error.to_string()))?;
            if body.is_empty() {
                return Ok(Some(ok(Value::empty())));
            }
        }
        Ok(None)
    }

    /// Prepare an actual manufactured lambda and retain its original argument objects.
    pub(crate) fn defer_original_lambda_call(
        &mut self,
        definition: crate::command::OriginalLambdaDefinition,
        invoked: Value,
        arguments: &[Value],
    ) -> Completion<Value> {
        match self.early_procedure_activation(&definition.binding, arguments) {
            Ok(Some(completion)) | Err(completion) => {
                self.retire_original_lambda_registration(&definition.registration);
                return completion;
            }
            Ok(None) => {}
        }
        let proc = match self.ensure_proc_ready_in(definition.binding, None, None) {
            Ok(proc) => proc,
            Err(error) => {
                self.retire_original_lambda_registration(&definition.registration);
                return crate::command::completion_from_tcl_error(self, error);
            }
        };
        self.pending.procedure_call = Some(OriginalProcedureCallReq {
            proc,
            invoked,
            arguments: arguments.to_vec(),
            registration: definition.registration,
        });
        ok(Value::empty())
    }

    /// Push a call-frame and bind `argv` to the prepared procedure activation.
    fn enter_proc(
        &mut self,
        proc: &Rc<ProcDef>,
        body: &crate::compiled::CompiledUnit,
        invoked: &Value,
        argv: &[Value],
    ) -> Result<(), Completion<Value>> {
        if body.jim_script.is_none() {
            body.asm
                .validate_native_compilation_entry()
                .expect("compiler provider admits native preflight before formal binding");
        }
        if let Some(failure) = &body.asm.native_compilation_failure {
            let name = self
                .native_name_operand_bytes(invoked)
                .map_err(|error| self.refuse_host_command(error.to_string()))?;
            return Err(self.raise_native_compilation_failure(failure, Some(&name)));
        }
        let Some(protocol) = tcl_registry::native_procedure::procedure_activation_protocol(
            self.native_invocation_dialect(),
        ) else {
            return Err(self.refuse_host_command(
                "native procedure activation protocol is unavailable".into(),
            ));
        };
        if protocol.parse_failure_precedes_arguments()
            && let Some(tail) = body.fatal_tail.clone()
        {
            return Err(self.raise_procedure_parse_failure(tail, invoked));
        }
        let bindings = self.procedure_argument_bindings(proc, argv)?;
        let skip_bindings = if proc.parameter_grammar == tcl_dialect::ParameterGrammar::Jim {
            self.native_name_operand_bytes(&proc.body_src)
                .map_err(|error| self.refuse_host_command(error.to_string()))?
                .is_empty()
        } else {
            false
        };
        if !skip_bindings && self.recursion_depth() >= self.recursion_limit() {
            return Err(err("too many nested evaluations (infinite loop?)"));
        }
        let call_argv = proc.call_identity.as_ref().map_or_else(
            || {
                let mut words = Vec::with_capacity(argv.len() + 1);
                words.push(invoked.native_lifetime_lease().into_value());
                words.extend(
                    argv.iter()
                        .map(|word| word.native_lifetime_lease().into_value()),
                );
                words
            },
            |words| {
                words
                    .iter()
                    .map(|word| word.native_lifetime_lease().into_value())
                    .collect()
            },
        );
        self.push_call_frame(Some(proc.actual_name()), call_argv);
        if let Some(namespace) = proc.retained_jim_namespace() {
            self.install_jim_procedure_namespace_owner(proc.actual_namespace_id(), namespace);
        }
        if !protocol.retains_prepared_body() {
            self.install_procedure_object_owners(
                proc.body_src.clone(),
                proc.native_parameters.clone(),
            );
        }
        self.install_procedure_statics(proc.retained_statics());

        self.push_proc_ns(&proc.actual_namespace(), proc.actual_namespace_id());
        if !skip_bindings
            && let Err(error) = body
                .compiled_local_layout
                .as_ref()
                .map_or(Ok(()), |layout| self.install_compiled_local_layout(layout))
                .and_then(|()| self.install_native_variable_name_owners(proc, body))
                .and_then(|()| self.bind_proc_arguments(proc, argv, &bindings))
        {
            self.pop_call_frame();
            self.pop_ns();
            return Err(error);
        }
        self.acquire_native_procedure_execution(proc);
        Ok(())
    }

    fn bind_proc_arguments(
        &mut self,
        proc: &ProcDef,
        argv: &[Value],
        bindings: &[tcl_syntax::formal_params::FormalByteArgumentBinding],
    ) -> Result<(), Completion<Value>> {
        use tcl_syntax::formal_params::FormalByteArgumentBinding;
        for binding in bindings {
            let parameter_slot = match binding {
                FormalByteArgumentBinding::Value { parameter, .. }
                | FormalByteArgumentBinding::Default { parameter }
                | FormalByteArgumentBinding::Rest { parameter, .. }
                | FormalByteArgumentBinding::CallerLink { parameter, .. } => *parameter,
            };
            let (name, value) = match binding {
                FormalByteArgumentBinding::Value {
                    parameter,
                    argument,
                } => (
                    proc.params[*parameter].name.as_bytes(),
                    argv[*argument].clone(),
                ),
                FormalByteArgumentBinding::Default { parameter } => {
                    let parameter = &proc.params[*parameter];
                    (
                        parameter.name.as_bytes(),
                        parameter.default.clone().expect("planned default"),
                    )
                }
                FormalByteArgumentBinding::Rest {
                    name, start, len, ..
                } => (
                    name.as_slice(),
                    Value::list(argv[*start..*start + *len].to_vec()),
                ),
                FormalByteArgumentBinding::CallerLink { name, argument, .. } => {
                    let target =
                        self.native_name_operand_bytes(&argv[*argument])
                            .map_err(|error| {
                                self.refuse_host_command(format!(
                                    "formal link name is unavailable: {error}"
                                ))
                            })?;
                    let caller = self.current_level().saturating_sub(1);
                    if self.get_var_from_bytes(caller, &target).is_none() {
                        return Err(self.read_miss_completion_bytes_from(caller, &target));
                    }
                    self.link_upvar_original(
                        caller,
                        &argv[*argument],
                        &Value::new_native_string_bytes(name.as_slice()),
                    )?;
                    continue;
                }
            };
            match proc.parameter_grammar {
                tcl_dialect::ParameterGrammar::Tcl => {
                    self.bind_compiled_formal_slot(parameter_slot, value)?;
                }
                tcl_dialect::ParameterGrammar::Jim => self.set_var_bytes(name, value)?,
            }
        }
        Ok(())
    }

    /// One scan step of a subst activation: append the next literal / `$…` run and
    /// either finish (`Return` with the accumulated output), pause for a top-level
    /// `[…]` (compile it and push a yieldable child script frame), or fail. The
    /// bracket's completion is folded back into the scan state by the subst rules
    /// in [`Vm::unwind`].
    fn tick_subst(&mut self, f: &mut Frame) -> Tick {
        let step = {
            let st = f.subst.as_mut().expect("subst frame carries scan state");
            crate::subst::subst_scan_step(self, st)
        };
        match step {
            crate::subst::SubstStep::Done(out) => Tick::Return(Completion::new(
                Code::Ok,
                Value::from_string_bytes(out),
                f.last_options.clone(),
            )),
            crate::subst::SubstStep::Error(error) => {
                Tick::Return(crate::command::completion_from_tcl_error(self, error))
            }
            crate::subst::SubstStep::ArrayIndex(template) => Tick::PushSubst {
                req: Box::new(SubstReq {
                    compiled: None,
                    original: None,
                    template,
                    backslashes: true,
                    commands: true,
                    variables: true,
                    control: crate::subst::SubstitutionControl::Word,
                }),
                placeholder: Box::new(self.current_placeholder_unit()),
            },
            crate::subst::SubstStep::Bracket(inner) => {
                match self.compile_script_cached_bytes(&tcl_lexer::SourceImage::native(inner)) {
                    Ok(script) => Tick::PushScript {
                        script: Box::new(script),
                        label: None,
                        cleanup_proc: None,
                        fatal_tail: None,
                        namespace: ScriptNamespace::Inherit,
                    },
                    Err(e) => Tick::Return(crate::command::completion_from_tcl_error(self, e)),
                }
            }
        }
    }

    /// Drive the selected native cursor without recreating names or members.
    fn tick_each_loop(&mut self, f: &mut Frame) -> Tick {
        use tcl_cmd_core::native_each_loop::EachLoopAction;
        use tcl_runtime_api::native_each_loop::NativeEachLoopKind;
        let Some(strings) = self.native_invocation_dialect().native_string_protocol() else {
            return Tick::Return(self.refuse_host_command("each-loop native List issuer".into()));
        };
        loop {
            let st = f.each_loop.as_mut().expect("each-loop state");
            let recipe = st.request.protocol.recipe();
            match st.request.cursor.advance() {
                EachLoopAction::Check(index) => {
                    let group = &mut st.request.groups[index];
                    group.value_items = None;
                    let items =
                        match self.native_object_list_elements_in(group.values.value(), strings) {
                            Ok(items) => items,
                            Err(error) => {
                                return Tick::Return(crate::command::completion_from_cmd_error(
                                    self,
                                    error.into(),
                                ));
                            }
                        };
                    st.request.cursor.set_value_length(index, items.len());
                    group.value_items = Some(items);
                }
                EachLoopAction::Refresh(index) => {
                    if let Err(tick) = self.refresh_each_loop_group(st, index, strings) {
                        return *tick;
                    }
                }
                EachLoopAction::Assign {
                    group,
                    variable,
                    value,
                } => {
                    let group = &st.request.groups[group];
                    let name = &group
                        .variable_items
                        .as_ref()
                        .expect("selected variable header")[variable];
                    let assigned = match value {
                        Some(index) => group.value_items.as_ref().expect("selected value header")
                            [index]
                            .clone(),
                        None => st
                            .request
                            .jim_empty
                            .as_ref()
                            .map_or_else(Value::empty, |root| {
                                root.value().native_lifetime_lease().into_value()
                            }),
                    };
                    let transient = recipe.pins_assignment_value().then(|| assigned.clone());
                    let stored = self.store_original_named_variable(name, assigned);
                    drop(transient);
                    if let Err(completion) = stored {
                        return Tick::Return(self.each_loop_setter_failure(
                            recipe,
                            name,
                            completion,
                            st.request.protocol.kind(),
                        ));
                    }
                }
                EachLoopAction::Body => {
                    if let Err(tick) = self.enter_each_loop_body(f) {
                        return *tick;
                    }
                }
                EachLoopAction::Finish => {
                    let st = f.each_loop.take().expect("finished each-loop");
                    let collect = st.request.protocol.kind() == NativeEachLoopKind::Lmap;
                    let result = if collect
                        && (st.request.cursor.entered_body() || recipe.empty_lmap_publishes_list())
                    {
                        Value::native_list_constructor(st.collected, strings)
                    } else {
                        st.request
                            .jim_empty
                            .map_or_else(Value::empty, |root| match root {
                                EachLoopRoot::Original(root) => root.into_value(),
                                EachLoopRoot::Header(root) => root,
                            })
                    };
                    return Tick::Return(Completion::new(
                        Code::Ok,
                        result,
                        if collect {
                            f.last_options.clone()
                        } else {
                            Value::empty()
                        },
                    ));
                }
            }
        }
    }

    fn refresh_each_loop_group(
        &mut self,
        st: &mut EachLoopState,
        index: usize,
        strings: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<(), Box<Tick>> {
        let recipe = st.request.protocol.recipe();

        let group = &mut st.request.groups[index];
        group.variable_items = None;
        let variables = match self.native_object_list_elements_in(group.variables.value(), strings)
        {
            Ok(items) => items,
            Err(error) if recipe.refetches_groups() => {
                return Err(Box::new(Tick::Return(self.refuse_host_command(format!(
                    "native Tcl 8.4 each-loop variable-list refetch fatal boundary: {error}"
                )))));
            }
            Err(error) => {
                return Err(Box::new(Tick::Return(
                    crate::command::completion_from_cmd_error(self, error.into()),
                )));
            }
        };
        let count = variables.len();
        group.variable_items = Some(variables);
        if !recipe.live_iterators() || st.request.cursor.variable_cursor() < count {
            group.value_items = None;
            let values = match self.native_object_list_elements_in(group.values.value(), strings) {
                Ok(items) => items,
                Err(error) if recipe.refetches_groups() => {
                    return Err(Box::new(Tick::Return(self.refuse_host_command(format!(
                        "native Tcl 8.4 each-loop value-list refetch fatal boundary: {error}"
                    )))));
                }
                Err(error) => {
                    return Err(Box::new(Tick::Return(
                        crate::command::completion_from_cmd_error(self, error.into()),
                    )));
                }
            };
            st.request.cursor.set_lengths(index, count, values.len());
            group.value_items = Some(values);
        } else {
            st.request.cursor.set_lengths(
                index,
                count,
                group.value_items.as_ref().map_or(0, |items| items.len()),
            );
        }

        Ok(())
    }

    fn enter_each_loop_body(&mut self, f: &mut Frame) -> Result<(), Box<Tick>> {
        let st = f.each_loop.as_mut().expect("each-loop state");
        let recipe = st.request.protocol.recipe();

        let body = st.request.body.value();
        match crate::command::original_script_list(
            self,
            body,
            tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
        ) {
            Ok(Some(words)) => {
                return Err(Box::new(Tick::PushControl {
                    state: Box::new(crate::cmd_control::ControlState::object_body_eval(
                        body.clone(),
                        words,
                    )),
                    placeholder: Box::new(self.current_placeholder_unit()),
                }));
            }
            Err(completion) => return Err(Box::new(Tick::Return(completion))),
            Ok(None) => {}
        }
        if !recipe.live_iterators() {
            st.body_owner = Some(body.clone());
        }
        let completion = match self.prepare_script_commands_value(body) {
            Ok(prepared) if prepared.prefix.is_some() => {
                return Err(Box::new(Tick::PushScript {
                    script: Box::new(prepared.prefix.expect("selected prefix")),
                    label: None,
                    cleanup_proc: None,
                    fatal_tail: prepared.fatal_tail,
                    namespace: ScriptNamespace::Inherit,
                }));
            }
            Ok(prepared) => prepared
                .fatal_tail
                .map_or_else(|| ok(Value::empty()), |tail| self.raise_fatal_tail(tail)),
            Err(error) => crate::command::completion_from_tcl_error(self, error),
        };
        match Self::fold_each_loop(f, completion) {
            EachLoopFold::Resume => {}
            EachLoopFold::Unwind(completion) => return Err(Box::new(Tick::Return(completion))),
        }

        Ok(())
    }

    fn each_loop_setter_failure(
        &mut self,
        recipe: tcl_runtime_api::native_each_loop::NativeEachLoopRecipe,
        name: &Value,
        completion: Completion<Value>,
        kind: tcl_runtime_api::native_each_loop::NativeEachLoopKind,
    ) -> Completion<Value> {
        let bytes = match self.native_string_bytes(name) {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(self, error.into()),
        };
        match tcl_cmd_core::native_each_loop::setter_failure(recipe, kind, &bytes) {
            tcl_cmd_core::native_each_loop::SetterFailure::Preserve => completion,
            tcl_cmd_core::native_each_loop::SetterFailure::Replace(error) => {
                crate::command::completion_from_cmd_error(self, error)
            }
            tcl_cmd_core::native_each_loop::SetterFailure::Context(context) => {
                let message = match self.native_string_bytes(&completion.result) {
                    Ok(message) => message,
                    Err(error) => {
                        return crate::command::completion_from_cmd_error(self, error.into());
                    }
                };
                self.seed_error_info_frame(&message, context);
                completion
            }
        }
    }

    /// Drive the shared expression AST; Tcl scripts and substitutions run on
    /// the interpreter stack so suspension retains their real call frames.
    fn tick_expression(&mut self, frame: &mut Frame) -> Tick {
        use tcl_syntax::expr::{ExprEvalRequest, ExprEvalStep};
        loop {
            let objects = frame
                .expression
                .as_ref()
                .expect("expression state present")
                .jim_objects
                .clone();
            let step = frame
                .expression
                .as_mut()
                .expect("expression state present")
                .state
                .advance(&mut crate::expr::ExprEval::with_jim_objects(self, objects));
            let request = match step {
                Err(error) => {
                    return Tick::Return(crate::command::completion_from_tcl_error(self, error));
                }
                Ok(ExprEvalStep::Complete(value)) => {
                    return self.complete_expression(frame, value);
                }
                Ok(ExprEvalStep::Request(request)) => request,
            };
            let completion = match request {
                ExprEvalRequest::SubstitutedString {
                    text: template,
                    start,
                    end,
                } => {
                    if let Some(tick) = self.expression_quoted_request(frame, template, start, end)
                    {
                        return tick;
                    }
                    continue;
                }
                ExprEvalRequest::Variable { reference, .. } => {
                    match self.expression_variable_reference(frame, &reference) {
                        Ok(completion) => completion,
                        Err(tick) => return *tick,
                    }
                }
                ExprEvalRequest::Command {
                    text: script,
                    start,
                    end,
                } => match self.expression_command_request(frame, script, start, end) {
                    Ok(completion) => completion,
                    Err(tick) => return *tick,
                },
                ExprEvalRequest::Call { function, args, .. } => {
                    match self.expression_math_call(
                        frame,
                        std::str::from_utf8(&function).expect("ASCII function token"),
                        args,
                    ) {
                        Ok(completion) => completion,
                        Err(tick) => return *tick,
                    }
                }
            };
            if completion.code != Code::Ok {
                return Tick::Return(completion);
            }
            frame.last_options = completion.options;
            frame
                .expression
                .as_mut()
                .expect("expression state present")
                .state
                .resume(completion.result);
        }
    }

    fn expression_quoted_request(
        &mut self,
        frame: &mut Frame,
        template: Vec<u8>,
        start: u32,
        end: u32,
    ) -> Option<Tick> {
        if let Some(objects) = frame
            .expression
            .as_ref()
            .and_then(|state| state.jim_objects.as_ref())
        {
            let Some((term, original)) = objects.at(start, Some(end)) else {
                return Some(Tick::Return(
                    self.refuse_host_command("original Jim quoted term extent".into()),
                ));
            };
            if term.kind == tcl_lexer::ExprTermKind::String {
                let value = original.clone();
                frame
                    .expression
                    .as_mut()
                    .expect("expression state present")
                    .state
                    .resume(value);
                return None;
            }
        }
        let Some(policy) = self.expression_quote_control() else {
            return Some(Tick::Return(self.refuse_host_command(
                "native expression quote settlement is unavailable".into(),
            )));
        };
        Some(Tick::PushSubst {
            req: Box::new(SubstReq {
                compiled: None,
                original: None,
                control: crate::subst::SubstitutionControl::Expression(policy),
                template: template.into(),
                backslashes: true,
                commands: true,
                variables: true,
            }),
            placeholder: Box::new(self.current_placeholder_unit()),
        })
    }

    fn expression_command_request(
        &mut self,
        frame: &Frame,
        script: Vec<u8>,
        start: u32,
        end: u32,
    ) -> Result<Completion<Value>, Box<Tick>> {
        Ok({
            let original = match frame
                .expression
                .as_ref()
                .and_then(|state| state.jim_objects.as_ref())
            {
                Some(objects) => match objects.at(start, Some(end)) {
                    Some((_, original)) => original.clone(),
                    None => {
                        return Err(Box::new(Tick::Return(
                            self.refuse_host_command("original Jim command term extent".into()),
                        )));
                    }
                },
                None => Value::from_string_bytes(script),
            };
            match self.prepare_script_commands_value(&original) {
                Ok(prepared) if prepared.prefix.is_some() => {
                    return Err(Box::new(Tick::PushScript {
                        script: Box::new(prepared.prefix.expect("prefix present")),
                        label: None,
                        cleanup_proc: None,
                        fatal_tail: prepared.fatal_tail,
                        namespace: ScriptNamespace::Inherit,
                    }));
                }
                Ok(prepared) => prepared
                    .fatal_tail
                    .map_or_else(|| ok(Value::empty()), |tail| self.raise_fatal_tail(tail)),
                Err(error) => crate::command::completion_from_tcl_error(self, error),
            }
        })
    }

    fn complete_expression(&mut self, frame: &Frame, value: Value) -> Tick {
        let value = if frame
            .expression
            .as_ref()
            .expect("expression state present")
            .normalize
        {
            match crate::expr::normalize_result_for_vm(self, &value, tcl_registry::native_boolean_truth::NativeBooleanExpressionResultProduction::PublicExpressionApi) {
                Ok(value) => value.with_native_double_format(self.native_invocation_dialect()),
                Err(error) => {
                    return Tick::Return(crate::command::completion_from_tcl_error(self, error));
                }
            }
        } else {
            value
        };
        Tick::Return(Completion::new(Code::Ok, value, frame.last_options.clone()))
    }

    fn expression_variable_reference(
        &mut self,
        frame: &mut Frame,
        reference: &[u8],
    ) -> Result<Completion<Value>, Box<Tick>> {
        let parsed = match tcl_lexer::word_parts::scan_var_ref(reference, 0, self.lexer_config()) {
            Ok(Some(parsed)) => parsed,
            Ok(None) => {
                return Err(Box::new(Tick::Return(err(
                    "invalid expression variable reference",
                ))));
            }
            Err(error) => {
                return Err(Box::new(Tick::Return(
                    crate::command::completion_from_tcl_error(self, TclError::new(error)),
                )));
            }
        };
        if let Some(index) = parsed.index {
            frame
                .expression
                .as_mut()
                .expect("expression state present")
                .awaiting_array = Some(parsed.name.to_vec());
            return Err(Box::new(Tick::PushSubst {
                req: Box::new(SubstReq {
                    compiled: None,
                    original: None,
                    template: index.to_vec().into(),
                    backslashes: true,
                    commands: true,
                    variables: true,
                    control: crate::subst::SubstitutionControl::Word,
                }),
                placeholder: Box::new(self.current_placeholder_unit()),
            }));
        }
        Ok(match self.read_variable_result_bytes(parsed.name, None) {
            Ok(value) => ok(value),
            Err(completion) => completion,
        })
    }

    fn expression_math_call(
        &mut self,
        frame: &mut Frame,
        function: &str,
        args: Vec<Value>,
    ) -> Result<Completion<Value>, Box<Tick>> {
        use tcl_syntax::expr::ExprOps;
        // Resumable evaluation uses the same explicit authored capability as
        // synchronous ExprEval; that capability never becomes a native table.
        if self.authored_math_provider().is_some() {
            return Ok(crate::cmd_math::invoke_authored_function(
                self, function, &args,
            ));
        }

        let dispatch = tcl_registry::native_expression_program::expression_function_dispatch(
            self.expression_evaluation_policy().as_ref(),
            self.actual_native_invocation_dialect(),
        );
        if dispatch.is_none() {
            return Ok(self
                .refuse_host_command("expression function dispatch policy is unavailable".into()));
        }
        if dispatch == Some(tcl_registry::mathfunc::NativeMathFunctionDispatch::CommandTable) {
            let name = tcl_registry::mathfunc::qualified_name(function)
                .trim_start_matches("::")
                .to_owned();
            // TIP 232 uses ordinary namespace-relative command resolution.
            // Unknown math functions bypass the user unknown handler.
            if self.lookup_command(&name).is_none() {
                return Err(Box::new(Tick::Return(
                    crate::command::completion_from_tcl_error(
                        self,
                        crate::error::TclError::with_error_code(
                            format!("invalid command name \"{name}\""),
                            format!("TCL LOOKUP COMMAND {name}"),
                        ),
                    ),
                )));
            }
            let mut words = Vec::with_capacity(args.len() + 1);
            words.push(Value::string(name));
            words.extend(args);
            let words = crate::NativeListItems::invocation_view(Rc::new(words));
            Ok(match self.dispatch_words(frame, &words) {
                Ok(Some(tick)) => return Err(Box::new(tick)),
                Ok(None) => Completion::new(Code::Ok, pop(frame), frame.last_options.clone()),
                Err(completion) => completion,
            })
        } else {
            let mut ops = crate::expr::ExprEval::new(self);
            Ok(match ops.call(function, args) {
                Ok(value) => ops.finish(value),
                Err(error) => crate::command::completion_from_tcl_error(ops.vm, error),
            })
        }
    }

    fn tick_control(&mut self, frame: &mut Frame) -> Tick {
        loop {
            if let Some(completion) = self.limit_check_tick() {
                return Tick::Return(completion);
            }
            let step = frame
                .control
                .as_mut()
                .expect("control state present")
                .next();
            match step {
                crate::cmd_control::ControlStep::Invocation(words) => {
                    let completion = if words.is_empty() {
                        ok(Value::empty())
                    } else {
                        match self.dispatch_words(frame, &words) {
                            Ok(Some(tick)) => return tick,
                            Ok(None) => {
                                Completion::new(Code::Ok, pop(frame), frame.last_options.clone())
                            }
                            Err(completion) => completion,
                        }
                    };
                    frame
                        .control
                        .as_mut()
                        .expect("control state present")
                        .accept(self, completion);
                }
                crate::cmd_control::ControlStep::ResumeInvocation => {
                    let completion =
                        Completion::new(Code::Ok, pop(frame), frame.last_options.clone());
                    frame
                        .control
                        .as_mut()
                        .expect("control state present")
                        .accept(self, completion);
                }
                crate::cmd_control::ControlStep::Complete(completion) => {
                    return Tick::Return(completion);
                }
                crate::cmd_control::ControlStep::Expression(expression) => {
                    match self.prepare_expression_value(&expression) {
                        Ok(node) => {
                            return Tick::PushExpression {
                                req: Box::new(ExpressionReq {
                                    state: tcl_syntax::expr::ExprEvalState::new(node),
                                    awaiting_array: None,
                                    normalize: false,
                                    jim_objects: expression.native_jim_expression_objects(),
                                    restore_primary: expression.retain_expression_primary(),
                                }),
                                placeholder: Box::new(self.current_placeholder_unit()),
                            };
                        }
                        Err(error) => {
                            let completion = crate::command::completion_from_tcl_error(self, error);
                            frame
                                .control
                                .as_mut()
                                .expect("control state present")
                                .accept(self, completion);
                        }
                    }
                }
                crate::cmd_control::ControlStep::Script(script) => {
                    match self.prepare_script_commands_value(&script) {
                        Ok(prepared) if prepared.prefix.is_some() => {
                            return Tick::PushScript {
                                script: Box::new(prepared.prefix.expect("prefix present")),
                                label: None,
                                cleanup_proc: None,
                                fatal_tail: prepared.fatal_tail,
                                namespace: ScriptNamespace::Inherit,
                            };
                        }
                        Ok(prepared) => {
                            let completion = prepared.fatal_tail.map_or_else(
                                || ok(Value::empty()),
                                |tail| self.raise_fatal_tail(tail),
                            );
                            frame
                                .control
                                .as_mut()
                                .expect("control state present")
                                .accept(self, completion);
                        }
                        Err(error) => {
                            let completion = crate::command::completion_from_tcl_error(self, error);
                            frame
                                .control
                                .as_mut()
                                .expect("control state present")
                                .accept(self, completion);
                        }
                    }
                }
            }
        }
    }

    /// Keep the original body completion and its selected error metadata.
    fn fold_each_loop(parent: &mut Frame, completion: Completion<Value>) -> EachLoopFold {
        use tcl_cmd_core::native_each_loop::BodyDecision;
        let st = parent.each_loop.as_mut().expect("each-loop state");
        st.body_owner = None;
        let decision = st.request.cursor.body_completion(completion.code);
        match decision {
            BodyDecision::Propagate => {
                let name = st.request.protocol.kind().name();
                parent.each_loop = None;
                if completion.code == Code::Error {
                    parent.body_label = Some(name);
                }
                EachLoopFold::Unwind(completion)
            }
            BodyDecision::Collect => {
                st.collected.push(completion.result.into_native_reference());
                parent.last_options = completion.options;
                EachLoopFold::Resume
            }
            BodyDecision::Continue => {
                parent.last_options = completion.options;
                EachLoopFold::Resume
            }
            BodyDecision::Finish => {
                parent.last_options = Value::empty();
                EachLoopFold::Resume
            }
        }
    }

    /// Select a compiler-hook-capable live callable, excluding execution traces.
    fn capture_native_command(
        &mut self,
        binding: &tcl_runtime_api::CommandBindingIdentity,
    ) -> Option<CapturedNativeCommand> {
        if !self.command_binding_matches(binding) {
            return None;
        }
        let (key, command) = self.lookup_compiled_command_binding(binding)?;
        let command = match command {
            Command::Builtin(_) => command,
            Command::Ensemble(ensemble) => {
                ensemble.config().native_implementation()?;
                // Retain the public token, whose generic handler selects the
                // live map and worker after arguments. Capturing its compiler
                // adapter would bypass a replaced private member.
                Command::Ensemble(ensemble)
            }
            _ => return None,
        };
        let builtin_identity = self.stock_native_identity(&key);
        let sidecar_key = CommandSidecarKey::visible(key);
        if self.command_has_execution_trace(&sidecar_key) {
            return None;
        }
        Some(CapturedNativeCommand {
            command,
            builtin_identity,
            sidecar: self.active_sidecar(sidecar_key),
            execution_traces: SelectedNativeExecutionTraces::Omitted,
        })
    }

    /// Capture the callable token for a typed generic surrogate at its literal
    /// head push. This is deliberately before every argument substitution: a
    /// trace, rename, or replacement performed by an earlier word belongs to
    /// the next invocation, not the command Tcl has already entered.
    fn capture_entered_command(
        &mut self,
        f: &mut Frame,
        asm: &FunctionAsm,
        instr: &Instruction,
        resume: usize,
    ) -> Result<(), Completion<Value>> {
        let Some(entered) = instr.entered_command.as_ref() else {
            return Ok(());
        };
        let captured = match entered.binding.guard {
            tcl_runtime_api::CommandBindingGuard::ChunkEntry => {
                f.chunk_commands.get(&entered.binding).cloned()
            }
            tcl_runtime_api::CommandBindingGuard::BeforeArguments => {
                self.capture_native_command(&entered.binding)
            }
        };
        let Some(captured) = captured else {
            return Ok(());
        };
        let Some(continuation) = label_to_idx(asm, &f.off2idx, &entered.end) else {
            return Err(err("entered command has no valid continuation"));
        };
        f.entered_commands.push(EnteredCommand {
            resume,
            continuation,
            command: captured.command,
            builtin_identity: captured.builtin_identity,
            sidecar: captured.sidecar,
            execution_traces: captured.execution_traces,
        });
        Ok(())
    }

    fn active_native_bindings_match(&self, frame: &Frame, asm: &FunctionAsm) -> bool {
        let selected = frame
            .entered_native_operations
            .iter()
            .flat_map(|operation| &operation.requirements)
            .collect::<Vec<_>>();
        self.function_live_command_bindings_match(asm, &selected, frame.manifest.as_deref())
    }

    fn enter_native_operations(
        &mut self,
        frame: &mut Frame,
        asm: &FunctionAsm,
        instruction: &Instruction,
    ) -> Option<Tick> {
        for (site_index, site) in instruction.native_operation_selections.iter().enumerate() {
            let continuation = label_to_idx(asm, &frame.off2idx, &site.end)
                .expect("validated native operation continuation");
            let identity = (frame.pc, site_index);
            let selected = frame.chunk_native_operations.contains(&identity)
                && site
                    .compiler_selection_prerequisite()
                    .as_ref()
                    .is_none_or(|required| {
                        required.guard() != tcl_runtime_api::CommandBindingGuard::BeforeArguments
                            || self.native_compiler_selection_prerequisite_matches(required)
                    })
                && site
                    .requirements
                    .iter()
                    .filter(|binding| {
                        binding.guard == tcl_runtime_api::CommandBindingGuard::BeforeArguments
                    })
                    .all(|binding| self.command_binding_matches(binding));
            if selected {
                frame
                    .entered_native_operations
                    .retain(|entered| entered.site != identity);
                frame
                    .entered_native_operations
                    .push(EnteredNativeOperation {
                        site: identity,
                        start: frame.pc,
                        end: continuation,
                        requirements: site.requirements.clone(),
                    });
                continue;
            }
            // The neutral operation site retains a rooted source namespace;
            // VM compilation and resolution stacks use its unrooted key.
            let context = site.replay_namespace_context();
            if self.resolve_compiled_namespace_context(&context).is_none() {
                return Some(Tick::Return(err(
                    "native replay namespace context is unavailable",
                )));
            }
            let namespace = context.path();
            let child = match self.compile_plain_function_cached_bytes(
                tcl_runtime_api::ScriptCompileTargetBytes {
                    source: &site.source,
                    namespace,
                },
            ) {
                Ok(child) => child,
                Err(error) => {
                    return Some(Tick::Return(crate::command::completion_from_tcl_error(
                        self, error,
                    )));
                }
            };
            frame.pc = continuation;
            return Some(Tick::PushScript {
                script: Box::new(self.compiled_unit(child, namespace.to_owned())),
                label: None,
                cleanup_proc: None,
                fatal_tail: None,
                namespace: ScriptNamespace::CommandBoundary(context),
            });
        }
        None
    }

    /// Select an independently audited native compiler operation.
    /// The range records selection, rather than repeatedly consulting a live
    /// callable during substitutions that may replace that very callable.
    fn enter_native_compiler_selection(
        &mut self,
        frame: &mut Frame,
        asm: &FunctionAsm,
        instruction: &Instruction,
    ) -> Option<Tick> {
        let site = instruction.native_compiler_selection.as_ref()?;
        let Some(continuation) = label_to_idx(asm, &frame.off2idx, &site.end) else {
            return Some(Tick::Return(err(
                "native compiler selection has no valid continuation",
            )));
        };
        let selected = match site.prerequisite.guard() {
            tcl_runtime_api::CommandBindingGuard::ChunkEntry => frame
                .chunk_native_compiler_selections
                .contains(&site.prerequisite),
            tcl_runtime_api::CommandBindingGuard::BeforeArguments => {
                self.native_compiler_selection_prerequisite_matches(&site.prerequisite)
            }
        };
        if selected {
            frame
                .entered_native_compiler_selections
                .push((frame.pc, continuation));
            return None;
        }
        if self
            .resolve_compiled_namespace_context(&instruction.source_namespace_context())
            .is_none()
        {
            return Some(Tick::Return(err(
                "native replay namespace context is unavailable",
            )));
        }
        let child = match self.compile_plain_function_cached_bytes(
            tcl_runtime_api::ScriptCompileTargetBytes {
                source: &instruction.source_cmd_text,
                namespace: &instruction.source_command_namespace,
            },
        ) {
            Ok(child) => child,
            Err(error) => {
                return Some(Tick::Return(crate::command::completion_from_tcl_error(
                    self, error,
                )));
            }
        };
        frame.pc = continuation;
        Some(Tick::PushScript {
            script: Box::new(
                self.compiled_unit(child, instruction.source_command_namespace.clone()),
            ),
            label: None,
            cleanup_proc: None,
            fatal_tail: None,
            namespace: ScriptNamespace::CommandBoundary(instruction.source_namespace_context()),
        })
    }

    fn frame_result_transport(
        &self,
        frame: &Frame,
    ) -> Result<Value, tcl_syntax::value::ValueError> {
        if let Some(result) = frame.last_result.as_ref() {
            return Ok(result.clone().into_value());
        }
        self.with_native_interp_result(|result| result.native_lifetime_lease().into_value())
    }

    fn finish_frame_result(&mut self, frame: &mut Frame) -> Tick {
        let result = frame
            .stack
            .pop()
            .map_or_else(|| self.frame_result_transport(frame), Ok);
        match result {
            Ok(result) => Tick::Return(Completion::new(
                Code::Ok,
                result,
                frame.last_options.clone(),
            )),
            Err(error) => Tick::Return(crate::command::completion_from_tcl_error(
                self,
                error.into(),
            )),
        }
    }

    /// Execute a single instruction of the top activation.
    #[allow(clippy::too_many_lines)] // One match over every opcode; the VM's central dispatch is clearer whole.
    fn tick(&mut self, f: &mut Frame) -> Tick {
        if let Some(dispatch) = f.deferred_dispatch.take() {
            return *dispatch;
        }
        if let Some(original) = f.original_invocation.take() {
            return self.tick_original_invocation(f, original);
        }
        // Compilation precedes every body word, even a malformed command
        // appearing after an otherwise executable store. Procedure entry has
        // already admitted this before formal binding; script entry has no
        // parameter frame and presents its compiler contexts here.
        if f.pc == 0
            && let Some(failure) = &f.asm.native_compilation_failure
        {
            return Tick::Return(self.raise_native_compilation_failure(failure, None));
        }
        if f.jim_script_entry.is_some() || f.jim_script.is_some() {
            return self.tick_native_jim_script(f);
        }
        if f.expression.is_some() {
            return self.tick_expression(f);
        }
        if f.control.is_some() {
            return self.tick_control(f);
        }
        // A subst activation is scanner-driven, not bytecode-driven: run the next
        // scan step (native literal/`$` runs, pausing at each top-level `[…]`)
        // instead of the instruction dispatch below.
        if f.subst.is_some() {
            return self.tick_subst(f);
        }
        // An each-loop activation (`foreach`/`lmap` runtime fallback) is
        // likewise driver-stepped, not bytecode-driven.
        if f.each_loop.is_some() {
            return self.tick_each_loop(f);
        }
        if !f.native_bytecode_entered {
            if let Err(refusal) = self.reset_native_ensemble_rewrite(
                tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent::BytecodeEntry,
            ) {
                return Tick::Return(refusal);
            }
            f.native_bytecode_entered = true;
        }
        // A control transfer may skip an entered command's invoke (an error,
        // catch jump, or stale-command replay). Drop only entries whose
        // continuation has been reached; enclosing nested commands have later
        // continuations and remain live.
        f.entered_commands
            .retain(|entered| entered.continuation > f.pc);
        f.entered_native_compiler_selections
            .retain(|(start, end)| *start <= f.pc && f.pc < *end);
        f.entered_native_operations
            .retain(|operation| operation.start <= f.pc && f.pc < operation.end);
        let asm = Rc::clone(&f.asm);
        if f.pc >= asm.instructions.len() {
            return self.finish_frame_result(f);
        }
        let instr = &asm.instructions[f.pc];
        if let Some(tick) = self.enter_native_operations(f, &asm, instr) {
            return tick;
        }
        if let Some(tick) = self.enter_native_compiler_selection(f, &asm, instr) {
            return tick;
        }
        // Each source command resets the interpreter's result owner. Frame
        // mirrors preserve lifetime without introducing native references.
        if instr.source_command_boundary.is_start() {
            f.last_result = None;
            if let Err(error) = self.reset_native_jim_result() {
                return Tick::Return(crate::command::completion_from_tcl_error(
                    self,
                    error.into(),
                ));
            }
        }
        if instr.op != Op::START_CMD
            && instr.source_command_boundary.is_start()
            && !f
                .entered_commands
                .iter()
                .any(|entered| entered.resume <= f.pc && f.pc < entered.continuation)
            && !f
                .entered_native_compiler_selections
                .iter()
                .any(|(start, end)| *start <= f.pc && f.pc < *end)
            && f.command_epoch != self.trace_deopt_epoch()
        {
            if self.active_native_bindings_match(f, &asm) && !self.step_trace_active() {
                f.command_epoch = self.trace_deopt_epoch();
            } else {
                if self
                    .resolve_compiled_namespace_context(&instr.source_namespace_context())
                    .is_none()
                {
                    return Tick::Return(err("native replay namespace context is unavailable"));
                }
                let child = match self.compile_plain_function_cached_bytes(
                    tcl_runtime_api::ScriptCompileTargetBytes {
                        source: &instr.source_cmd_text,
                        namespace: &instr.source_command_namespace,
                    },
                ) {
                    Ok(asm) => asm,
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_tcl_error(
                            self, error,
                        ));
                    }
                };
                let mut after = f.pc + 1;
                while after < asm.instructions.len() {
                    let candidate = &asm.instructions[after];
                    if candidate.source_command_boundary.is_start() {
                        // A command substitution has its own executable source
                        // boundary inside the enclosing command. Replaying the
                        // enclosing command already executes that substitution,
                        // so resuming at the nested boundary would execute the
                        // outer command repeatedly and eventually consume a
                        // half-rebuilt operand stack. The compiler's exact spans
                        // give the nesting relation without naming either
                        // command; the first non-contained boundary is this
                        // command's continuation.
                        let nested = instr.source_span.zip(candidate.source_span).is_some_and(
                            |(outer, inner)| {
                                outer.start() <= inner.start() && inner.end() <= outer.end()
                            },
                        );
                        if !nested {
                            break;
                        }
                    }
                    after += 1;
                }
                // Resume on this command's trailing POP so the transparent
                // child's result becomes the parent's ordinary last result.
                f.pc = if after > f.pc + 1 && asm.instructions[after - 1].op == Op::POP {
                    after - 1
                } else {
                    after
                };
                return Tick::PushScript {
                    script: Box::new(
                        self.compiled_unit(child, instr.source_command_namespace.clone()),
                    ),
                    label: None,
                    cleanup_proc: None,
                    fatal_tail: None,
                    namespace: ScriptNamespace::CommandBoundary(instr.source_namespace_context()),
                };
            }
        }
        if let Err(completion) =
            self.capture_entered_command(f, &asm, instr, f.pc.saturating_add(1))
        {
            return Tick::Return(completion);
        }
        if instr
            .completion_option_scope
            .is_some_and(tcl_runtime_api::completion_options::ActivationOptionScope::begins_fresh)
        {
            f.last_options = Value::empty();
        }
        f.pc += 1;
        // Line-watch seam: keep the embedder's cell on the dispatching
        // instruction's source line (see `Vm::set_line_watch`).
        self.note_line(instr.source_line);
        // Step-debugger seam: fire once per source command, before it runs.
        #[allow(clippy::redundant_closure_for_method_calls)] // Span isn't named in this crate.
        let span_start = instr.source_span.map(|s| s.start());
        if self.debug_step(instr.source_line, span_start, &instr.source_cmd_text) {
            return Tick::Return(Completion::new(
                Code::Error,
                Value::string("debug: terminated"),
                Value::empty(),
            ));
        }
        let lits = asm.literals.entries();
        let lvt = asm.lvt.entries();

        // Unwrap a fallible step, unwinding the instruction on an error. The
        // value form lets a store arm push what the variable reads back.
        macro_rules! try_op {
            ($e:expr) => {
                match $e {
                    Ok(v) => v,
                    Err(c) => return Tick::Return(c),
                }
            };
        }

        // Portable operation failures retain byte diagnostics and host refusal tags.
        macro_rules! try_core {
            ($operation:expr) => {
                match $operation {
                    Ok(value) => value,
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            error.into(),
                        ));
                    }
                }
            };
        }

        // Like `try_op!`, but logs this instruction's command-source frame to
        // `errorInfo` before unwinding — for compiled command-boundary ops
        // (`set`/`incr` → STORE_*/INCR_*) so a failing compiled command
        // contributes its `while executing`/`invoked from within "<cmd>"`
        // frame just as the INVOKE path and reference Tcl's bytecode engine do
        // (set-2.4: a write-trace rejection's `errorInfo` must reach the
        // triggering `set x 1`).
        macro_rules! try_cmd {
            ($e:expr) => {
                match $e {
                    Ok(v) => v,
                    Err(c) => {
                        let cmd_text = instr.source_cmd_text.clone();
                        let line = instr.source_line;
                        let msg = c.result.string_bytes();
                        self.log_command_info(&cmd_text, &msg, line);
                        return Tick::Return(c);
                    }
                }
            };
        }

        macro_rules! native_operand_bytes {
            ($value:expr) => {{
                let operand = $value;
                match self.native_name_operand_bytes(&operand) {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        return Tick::Return(self.refuse_host_command(format!(
                            "native variable operand is unavailable: {error}"
                        )));
                    }
                }
            }};
        }

        let lvt_name = |slot: i32| -> tcl_core_types::NameBytes {
            lvt.get(usize::try_from(slot).unwrap_or(usize::MAX))
                .cloned()
                .unwrap_or_default()
        };

        match instr.op {
            // Stack.
            Op::PUSH1 | Op::PUSH4 => {
                let raw = usize::try_from(imm0(instr))
                    .ok()
                    .and_then(|idx| lits.get(idx))
                    .cloned()
                    .unwrap_or_default();
                // A verbatim (braced / constant) literal suppresses all word
                // substitution — it is pushed exactly as the codegen interned
                // it (the codegen sets this flag for braced words / `proc`
                // bodies / constant assignments).
                if instr.push_verbatim {
                    let value = if let Some(direct) = f.direct_source_operands {
                        let Some(literal) = usize::try_from(imm0(instr))
                            .ok()
                            .and_then(|index| lits.get(index))
                        else {
                            return Tick::Return(self.refuse_host_command(
                                "native DIRECT source operand index is unavailable".into(),
                            ));
                        };
                        direct.value(literal)
                    } else {
                        let pool = match &f.literal_pool {
                            Ok(pool) => pool,
                            Err(error) => {
                                return Tick::Return(self.refuse_host_command(error.to_string()));
                            }
                        };
                        let Some(value) = usize::try_from(imm0(instr))
                            .ok()
                            .and_then(|index| pool.value(index))
                        else {
                            return Tick::Return(self.refuse_host_command(
                                "native literal object-array index is unavailable".into(),
                            ));
                        };
                        value
                    };
                    if let Some(line) = instr.source_value_line {
                        let base = f.source_location.as_ref();
                        value.retain_source_location(
                            tcl_runtime_api::script_source_location::ScriptSourceLocation {
                                file: base
                                    .map_or("", |location| location.file.as_str())
                                    .to_string(),
                                line: base
                                    .map_or(1, |location| location.line)
                                    .saturating_add(line.saturating_sub(1)),
                            },
                        );
                    }
                    f.stack.push(value);
                } else {
                    match crate::subst::subst_word_bytes(raw.bytes(), self) {
                        Ok(v) => f.stack.push(v),
                        // A `break`/`continue`/`return` carried out of a `[…]`
                        // substitution propagates with its own code (an enclosing
                        // loop's exception range / a proc boundary handles it).
                        Err(e) => {
                            return Tick::Return(crate::command::completion_from_tcl_error(
                                self, e,
                            ));
                        }
                    }
                }
            }
            Op::POP => {
                // Native POP discards TOS; only DONE or a command publisher
                // installs it as the interpreter result.
                drop(f.stack.pop());
                match self.with_native_interp_result(Value::native_lifetime_lease) {
                    Ok(result) => f.last_result = Some(result),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_tcl_error(
                            self,
                            error.into(),
                        ));
                    }
                }
            }
            Op::DUP => {
                if let Some(v) = f.stack.last().cloned() {
                    f.stack.push(v);
                }
            }
            Op::OVER => {
                let n = usize::try_from(imm0(instr)).unwrap_or(0);
                if let Some(i) = f.stack.len().checked_sub(n + 1) {
                    let v = f.stack[i].clone();
                    f.stack.push(v);
                }
            }
            Op::STR_CONCAT1 => {
                let n = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < n {
                    return Tick::Return(err("strcat: stack underflow"));
                }
                let parts = f.stack.split_off(f.stack.len() - n);
                if let Some(protocol) = self
                    .actual_native_invocation_dialect()
                    .native_string_protocol()
                    && protocol.tcl_version().is_some()
                {
                    let value = try_core!(tcl_cmd_core::native_cat::concatenate_compiled(
                        &crate::value::VmAppendObjects,
                        protocol,
                        &parts
                    ));
                    f.stack.push(value);
                } else {
                    let mut bytes = Vec::new();
                    for part in &parts {
                        bytes.extend_from_slice(&part.string_bytes());
                    }
                    f.stack.push(Value::from_string_bytes(bytes));
                }
            }
            Op::START_CMD => {
                let current_epoch = self.trace_deopt_epoch();
                // Compiler-only markers have no exact owning Tcl command and
                // therefore are not safe acknowledgement/replay points. Leave
                // the frame stale until a real source boundary is reached.
                if f.command_epoch != current_epoch && !instr.source_cmd_text.is_empty() {
                    if self.active_native_bindings_match(f, &asm) && !self.step_trace_active() {
                        f.command_epoch = current_epoch;
                    } else {
                        let Some(target) =
                            label0(instr).and_then(|label| label_to_idx(&asm, &f.off2idx, label))
                        else {
                            return Tick::Return(err(
                                "stale startCommand has no valid continuation",
                            ));
                        };
                        if self
                            .resolve_compiled_namespace_context(&instr.source_namespace_context())
                            .is_none()
                        {
                            return Tick::Return(err(
                                "native replay namespace context is unavailable",
                            ));
                        }
                        let child = match self.compile_plain_function_cached_bytes(
                            tcl_runtime_api::ScriptCompileTargetBytes {
                                source: &instr.source_cmd_text,
                                namespace: &instr.source_command_namespace,
                            },
                        ) {
                            Ok(asm) => asm,
                            Err(error) => {
                                return Tick::Return(crate::command::completion_from_tcl_error(
                                    self, error,
                                ));
                            }
                        };
                        // The transparent child produces this command's result
                        // on the parent stack; resume at the original
                        // startCommand continuation, skipping stale opcodes.
                        f.pc = target;
                        return Tick::PushScript {
                            script: Box::new(
                                self.compiled_unit(child, instr.source_command_namespace.clone()),
                            ),
                            label: None,
                            cleanup_proc: None,
                            fatal_tail: None,
                            namespace: ScriptNamespace::CommandBoundary(
                                instr.source_namespace_context(),
                            ),
                        };
                    }
                }
            }
            Op::NOP => {}

            // A `BEGIN_CATCH4` carrying an out-of-band handler label opens a
            // live catch range (C `INST_BEGIN_CATCH4` pushing the catchStack);
            // without one it is a *decorative* range — C-faithful reference
            // bytecode whose construct the VM protects via its activation
            // stack instead (`dict for`/`dict map`/`try` epilogues) — and
            // stays inert. The 4-byte operand keeps C's meaning (range index)
            // and is not consulted.
            Op::BEGIN_CATCH4 => {
                if let Some(idx) = instr
                    .catch_target
                    .as_deref()
                    .and_then(|label| label_to_idx(&asm, &f.off2idx, label))
                {
                    f.catch_ranges.push(CatchRange {
                        target_idx: idx,
                        start_idx: instr
                            .catch_start
                            .as_deref()
                            .and_then(|label| label_to_idx(&asm, &f.off2idx, label))
                            .unwrap_or(f.pc),
                        end_idx: instr
                            .catch_end
                            .as_deref()
                            .and_then(|label| label_to_idx(&asm, &f.off2idx, label))
                            .unwrap_or(idx),
                        stack_len: f.stack.len(),
                        foreach_len: f.foreach_stack.len(),
                        expand_len: f.expand_markers.len(),
                        begin_idx: f.pc - 1,
                        in_handler: false,
                        caught: None,
                    });
                }
            }
            // `END_CATCH` closes the innermost range. A decorative
            // `BEGIN_CATCH4` pushed nothing, so its paired `END_CATCH` finds
            // the stack empty and stays a no-op.
            Op::END_CATCH => {
                if f.catch_ranges.pop().is_some() {
                    // The protected completion's options have already been read
                    // by PUSH_RETURN_OPTS. A live END_CATCH completes `catch`
                    // itself, whose successful integer result has ordinary
                    // options. Decorative try/dict cleanup ranges preserve the
                    // completion they are forwarding.
                    f.last_options = Value::empty();
                }
            }

            // Variables (stack form, by name)
            // The `*ScalarStk` opcodes share their C `TEBCresume` case with the
            // general `*Stk` form (the compiler emits them when the name is known
            // to carry no `(index)` part), so they are the same arm here.
            Op::LOAD_STK | Op::LOAD_SCALAR_STK => {
                let name = native_operand_bytes!(pop(f));
                f.stack
                    .push(try_op!(self.read_variable_result_bytes(&name, None)));
            }
            Op::STORE_STK | Op::STORE_SCALAR_STK => {
                let value = pop(f);
                let name = native_operand_bytes!(pop(f));
                let stored = try_cmd!(self.store_var_result_bytes(&name, value));
                f.stack.push(stored);
            }
            Op::INCR_STK_IMM | Op::INCR_SCALAR_STK_IMM => {
                let name = native_operand_bytes!(pop(f));
                let amount = Value::int(i64::from(imm0(instr)));
                let updated = try_op!(self.increment_captured_bytes(&name, None, &amount));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::INCR_STK | Op::INCR_SCALAR_STK => {
                let amount = pop(f);
                let name = native_operand_bytes!(pop(f));
                let updated = try_op!(self.increment_captured_bytes(&name, None, &amount));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }

            // Variables (LVT form, proc bodies)
            Op::LOAD_SCALAR1 | Op::LOAD_SCALAR4 => {
                f.stack.push(try_op!(self.read_compiled_variable_result(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    None
                )));
            }
            Op::STORE_SCALAR1 | Op::STORE_SCALAR4 => {
                let value = pop(f);
                let stored = try_op!(self.store_compiled_variable_result(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    None,
                    value
                ));
                f.stack.push(stored);
            }
            Op::INCR_SCALAR1 => {
                let amount = pop(f);
                let updated = try_op!(self.increment_compiled_variable(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    None,
                    &amount
                ));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::INCR_SCALAR1_IMM => {
                let amount = Value::int(i64::from(imm_at(instr, 1)));
                let updated = try_op!(self.increment_compiled_variable(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    None,
                    &amount
                ));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::INCR_ARRAY_STK_IMM => {
                let key = native_operand_bytes!(pop(f));
                let name = native_operand_bytes!(pop(f));
                let amount = Value::int(i64::from(imm0(instr)));
                let updated = try_op!(self.increment_captured_bytes(&name, Some(&key), &amount));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::INCR_ARRAY_STK => {
                let amount = pop(f);
                let key = native_operand_bytes!(pop(f));
                let name = native_operand_bytes!(pop(f));
                let updated = try_op!(self.increment_captured_bytes(&name, Some(&key), &amount));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::INCR_ARRAY1 => {
                let amount = pop(f);
                let key = native_operand_bytes!(pop(f));
                let updated = try_op!(self.increment_compiled_variable(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    Some(&key),
                    &amount
                ));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::INCR_ARRAY1_IMM => {
                let key = native_operand_bytes!(pop(f));
                let amount = Value::int(i64::from(imm_at(instr, 1)));
                let updated = try_op!(self.increment_compiled_variable(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    Some(&key),
                    &amount
                ));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::EXIST_SCALAR => {
                let found = try_op!(self.exists_compiled_original_variable(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    None
                ));
                f.stack.push(try_op!(self.compiled_existence_result(found)));
            }
            Op::EXIST_STK => {
                let original = pop(f);
                let found = if self.native_c_variable_name_protocol().is_some() {
                    try_op!(self.exists_original_c_parts(&original, None))
                } else {
                    let name = native_operand_bytes!(original);
                    self.exists_var_traced_bytes(&name)
                };
                f.stack.push(try_op!(self.compiled_existence_result(found)));
            }
            Op::EXIST_ARRAY => {
                let index = pop(f);
                let found = try_op!(self.exists_compiled_original_variable(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    Some(&index)
                ));
                f.stack.push(try_op!(self.compiled_existence_result(found)));
            }
            Op::EXIST_ARRAY_STK => {
                let index = pop(f);
                let original = pop(f);
                let found = if self.native_c_variable_name_protocol().is_some() {
                    try_op!(self.exists_original_c_parts(&original, Some(&index)))
                } else {
                    let index = native_operand_bytes!(index);
                    let name = native_operand_bytes!(original);
                    self.exists_elem_traced_bytes(&name, &index)
                };
                f.stack.push(try_op!(self.compiled_existence_result(found)));
            }

            // Arrays.
            Op::LOAD_ARRAY_STK => {
                let key = native_operand_bytes!(pop(f));
                let name = native_operand_bytes!(pop(f));
                f.stack
                    .push(try_op!(self.read_variable_result_bytes(&name, Some(&key))));
            }
            Op::STORE_ARRAY_STK => {
                let value = pop(f);
                let key = native_operand_bytes!(pop(f));
                let name = native_operand_bytes!(pop(f));
                f.stack
                    .push(try_op!(self.store_elem_result_bytes(&name, &key, value)));
            }
            Op::LOAD_ARRAY1 | Op::LOAD_ARRAY4 => {
                let key = native_operand_bytes!(pop(f));
                f.stack.push(try_op!(self.read_compiled_variable_result(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    Some(&key)
                )));
            }
            Op::STORE_ARRAY1 | Op::STORE_ARRAY4 => {
                let value = pop(f);
                let key = native_operand_bytes!(pop(f));
                f.stack.push(try_op!(self.store_compiled_variable_result(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    Some(&key),
                    value
                )));
            }
            Op::ARRAY_EXISTS_IMM => {
                let found = try_op!(self.array_exists_compiled(
                    usize::try_from(imm0(instr)).expect("local slot index")
                ));
                let version = self.actual_native_invocation_dialect().tcl_version;
                let Some(version) = version else {
                    return Tick::Return(
                        self.refuse_host_command("native array existence issuer".into()),
                    );
                };
                let result = try_op!(self.native_c_execution_boolean(found, version).map_err(
                    |error| crate::command::completion_from_cmd_error(self, error.into())
                ));
                f.stack.push(result);
            }
            Op::ARRAY_EXISTS_STK => {
                let original = pop(f);
                let found = try_op!(self.array_exists_original_opcode(&original));
                let Some(version) = self.actual_native_invocation_dialect().tcl_version else {
                    return Tick::Return(
                        self.refuse_host_command("native Array constant issuer".into()),
                    );
                };
                let result = try_op!(self.native_c_execution_boolean(found, version).map_err(
                    |error| crate::command::completion_from_cmd_error(self, error.into())
                ));
                f.stack.push(result);
            }
            // `array set`'s materialising half (C `INST_ARRAY_MAKE_*`): make the
            // variable an empty array when undefined, no-op when it already is
            // one, and error on a scalar or an array element.
            Op::ARRAY_MAKE_IMM => {
                try_op!(self.ensure_array_compiled(
                    usize::try_from(imm0(instr)).expect("local slot index")
                ));
            }
            Op::ARRAY_MAKE_STK => {
                let original = pop(f);
                try_op!(self.array_make_original_opcode(&original));
            }

            // Lists (inline opcodes)
            Op::LIST => {
                let n = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < n {
                    return Tick::Return(err("list: stack underflow"));
                }
                let items = f.stack.split_off(f.stack.len() - n);
                let value = if let Some(protocol) = self
                    .actual_native_invocation_dialect()
                    .native_string_protocol()
                {
                    Value::native_list_constructor(items, protocol)
                } else {
                    Value::list(items)
                };
                f.stack.push(value);
            }
            Op::LIST_LENGTH if instr.native_switch_version.is_some() => {
                let original = pop(f);
                let result = try_core!(self.execute_native_scalar_length(
                    &original,
                    tcl_registry::native_scalar_compilation::NativeScalarOperation::ListLength,
                    instr.native_switch_version.unwrap(),
                ));
                drop(original);
                f.stack.push(result);
            }
            Op::LIST_LENGTH => {
                let l = pop(f);
                match l.as_list() {
                    Ok(items) => f.stack.push(Value::int(ilen(items.len()))),
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_tcl_error(self, e));
                    }
                }
            }
            // `[lindex $list $i]` — C `INST_LIST_INDEX` (`tclExecute.c:4696-4768`).
            // The integer fast path is only a fast path: when the index object
            // does not parse as one, C falls through to
            // `TclLindexList(interp, valuePtr, value2Ptr)` (line 4754), which
            // treats the index word as an index *list* — so a list-valued index
            // drills nested sublists (`lindex {{a b} {c d}} {1 0}` → `c`), an
            // empty index list is the whole list, and a spec that is neither
            // (`foo`) is `bad index "foo": …` (`objResultPtr == NULL` →
            // `goto gotError`, 4758-4761). That is exactly the runtime `lindex`
            // command's core, so both paths single-source it.
            Op::LIST_INDEX => {
                let idx = pop(f);
                let l = pop(f);
                match self.original_list_index(&l, std::slice::from_ref(&idx), true) {
                    Ok(v) => f.stack.push(v),
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(self, e));
                    }
                }
            }
            Op::LIST_INDEX_IMM => {
                let list = pop(f);
                if let Some(coordinate) = instr.native_list_index {
                    match tcl_cmd_core::native_list_index::immediate(self, &list, coordinate) {
                        Ok(value) => f.stack.push(value),
                        Err(error) => {
                            return Tick::Return(crate::command::completion_from_cmd_error(
                                self, error,
                            ));
                        }
                    }
                } else {
                    let items = match list.as_list() {
                        Ok(items) => items,
                        Err(error) => {
                            return Tick::Return(crate::command::completion_from_tcl_error(
                                self, error,
                            ));
                        }
                    };
                    let index = imm_index(imm0(instr), items.len());
                    f.stack.push(get_at(&items, index));
                }
            }
            Op::LIST_RANGE_IMM => {
                let l = pop(f);
                if let Some(range) = instr.native_list_range {
                    let Some(protocol) = self
                        .actual_native_invocation_dialect()
                        .native_string_protocol()
                    else {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "native compiled List range issuer",
                            )
                            .into(),
                        ));
                    };
                    let discard = asm.instructions.get(f.pc).is_some_and(|next| {
                        next.op == Op::POP
                            && next.native_compiler_selection.is_none()
                            && next.native_operation_selections.is_empty()
                            && !next.source_command_boundary.is_start()
                            && next.completion_option_scope.is_none()
                    });
                    if discard {
                        match l.native_list_range_validate(protocol) {
                            Ok(()) => {
                                f.pc += 1;
                            }
                            Err(error) => {
                                return Tick::Return(crate::command::completion_from_cmd_error(
                                    self,
                                    error.into(),
                                ));
                            }
                        }
                    } else {
                        match l.native_list_range(range, protocol) {
                            Ok(value) => f.stack.push(value),
                            Err(error) => {
                                return Tick::Return(crate::command::completion_from_cmd_error(
                                    self,
                                    error.into(),
                                ));
                            }
                        }
                    }
                    return Tick::Continue;
                }
                if imm0(instr) == 0 && imm_at(instr, 1) == tcl_bytecode::INDEX_END {
                    if let Some(protocol) = self
                        .actual_native_invocation_dialect()
                        .native_string_protocol()
                        .filter(|protocol| {
                            protocol
                                .tcl_version()
                                .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0)
                        })
                    {
                        match l.native_full_list_range(protocol) {
                            Ok(value) => f.stack.push(value),
                            Err(error) => {
                                return Tick::Return(crate::command::completion_from_cmd_error(
                                    self,
                                    error.into(),
                                ));
                            }
                        }
                    } else {
                        let items = try_op!(l.as_list().map_err(|error| {
                            crate::command::completion_from_tcl_error(self, error)
                        }));
                        f.stack.push(slice(
                            &items,
                            0,
                            isize::try_from(items.len()).unwrap_or(isize::MAX) - 1,
                        ));
                    }
                } else {
                    let items = match l.as_list() {
                        Ok(i) => i,
                        Err(e) => {
                            return Tick::Return(crate::command::completion_from_tcl_error(
                                self, e,
                            ));
                        }
                    };
                    let lo = imm_index(imm0(instr), items.len()).max(0);
                    let hi = imm_index(imm_at(instr, 1), items.len());
                    f.stack.push(slice(&items, lo, hi));
                }
            }
            Op::LIST_IN | Op::LIST_NOT_IN => {
                let list = pop(f);
                let needle = pop(f);
                let items = match list.as_list() {
                    Ok(i) => i,
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_tcl_error(self, e));
                    }
                };
                let n = needle.to_str();
                let found = items.iter().any(|v| *v.to_str() == *n);
                let r = if instr.op == Op::LIST_IN {
                    found
                } else {
                    !found
                };
                f.stack.push(Value::bool(r));
            }
            Op::UNSET_STK => {
                // operand = flags; a non-zero flag means "complain" (error when
                // the variable is absent). The name (possibly `a(k)`) is on TOS.
                let complain = imm0(instr) != 0;
                let name = pop(f);
                try_op!(self.unset_original_named_variable(&name, complain));
            }
            Op::UNSET_SCALAR => {
                // Operands [flags, slot]; flags bit 0 ⇒ complain when absent.
                f.dict_iters.remove(&imm_at(instr, 1));
                let complain = imm0(instr) != 0;
                try_op!(self.unset_compiled_variable(
                    usize::try_from(imm_at(instr, 1)).expect("local slot index"),
                    None,
                    complain
                ));
            }
            Op::UNSET_ARRAY => {
                // Operands [flags, slot]; the element key is on TOS. C
                // `INST_UNSET_ARRAY` reads the flags operand
                // (`flags = TclGetUInt1AtPtr(pc + 1) ? TCL_LEAVE_ERR_MSG : 0`,
                // `tclExecute.c:3814`) and, when it is set and the element is
                // absent, deliberately falls to `slowUnsetArray` (3837-3839,
                // 3851-3862) so the miss is reported. Ignoring the operand made
                // `unset B(nope)` silently succeed while `UNSET_SCALAR`/
                // `UNSET_STK` complained.
                let complain = imm0(instr) != 0;
                let key_original = pop(f);
                let key = native_operand_bytes!(key_original.native_lifetime_lease().into_value());
                try_op!(self.unset_compiled_variable(
                    usize::try_from(imm_at(instr, 1)).expect("local slot index"),
                    Some(&key),
                    complain
                ));
            }
            Op::UNSET_ARRAY_STK => {
                // Operand = flags (non-zero ⇒ C's `TCL_LEAVE_ERR_MSG`: complain
                // when the element is absent); the key is on TOS over the array
                // name, matching C's `part2Ptr = OBJ_AT_TOS` /
                // `part1Ptr = OBJ_UNDER_TOS` (`tclExecute.c:3866-3872`).
                let complain = imm0(instr) != 0;
                let key = pop(f);
                let name = pop(f);
                if self.native_c_variable_name_protocol().is_some() {
                    try_op!(self.unset_original_c_variable_parts(&name, &key, complain));
                } else {
                    let key_bytes = native_operand_bytes!(key.native_lifetime_lease().into_value());
                    let name_bytes =
                        native_operand_bytes!(name.native_lifetime_lease().into_value());
                    try_op!(self.unset_array_elem_checked_bytes(&name_bytes, &key_bytes, complain));
                }
            }

            // Single-value append opcodes do not fire a read trace; list
            // expansion opcodes use the retained-cell read/update protocol.
            Op::APPEND_SCALAR1 | Op::APPEND_SCALAR4 => {
                let value = pop(f);
                f.stack.push(try_op!(self.append_compiled_variable(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    None,
                    &[value]
                )));
            }
            Op::APPEND_ARRAY1 | Op::APPEND_ARRAY4 => {
                let value = pop(f);
                let key = native_operand_bytes!(pop(f));
                f.stack.push(try_op!(self.append_compiled_variable(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    Some(&key),
                    &[value]
                )));
            }
            Op::LAPPEND_SCALAR1 | Op::LAPPEND_SCALAR4 => {
                let value = pop(f);
                f.stack.push(try_op!(self.lappend_compiled_single(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    None,
                    &value
                )));
            }
            Op::LAPPEND_ARRAY1 | Op::LAPPEND_ARRAY4 => {
                let value = pop(f);
                let key = native_operand_bytes!(pop(f));
                f.stack.push(try_op!(self.lappend_compiled_single(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    Some(&key),
                    &value
                )));
            }
            Op::APPEND_STK | Op::LAPPEND_STK => {
                let value = pop(f);
                let name = native_operand_bytes!(pop(f));
                let stored = if instr.op == Op::APPEND_STK {
                    self.append_variable_bytes(&name, None, value)
                } else {
                    self.lappend_instruction_single_bytes(&name, None, &value)
                };
                f.stack.push(try_op!(stored));
            }
            Op::APPEND_ARRAY_STK | Op::LAPPEND_ARRAY_STK => {
                let value = pop(f);
                let key = native_operand_bytes!(pop(f));
                let name = native_operand_bytes!(pop(f));
                let stored = if instr.op == Op::APPEND_ARRAY_STK {
                    self.append_variable_bytes(&name, Some(&key), value)
                } else {
                    self.lappend_instruction_single_bytes(&name, Some(&key), &value)
                };
                f.stack.push(try_op!(stored));
            }
            Op::LAPPEND_LIST => {
                let additions = pop(f);
                let updated = try_op!(self.lappend_instruction_compiled_list(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    None,
                    &additions
                ));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::LAPPEND_LIST_STK => {
                let additions = pop(f);
                let name = native_operand_bytes!(pop(f));
                let updated = try_op!(self.lappend_instruction_list_bytes(&name, None, &additions));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::LAPPEND_LIST_ARRAY => {
                let additions = pop(f);
                let key = native_operand_bytes!(pop(f));
                let updated = try_op!(self.lappend_instruction_compiled_list(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    Some(&key),
                    &additions
                ));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::LAPPEND_LIST_ARRAY_STK => {
                let additions = pop(f);
                let key = native_operand_bytes!(pop(f));
                let name = native_operand_bytes!(pop(f));
                let updated =
                    try_op!(self.lappend_instruction_list_bytes(&name, Some(&key), &additions));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }

            Op::CONST_IMM => {
                let value = pop(f);
                try_op!(self.declare_compiled_constant(
                    usize::try_from(imm0(instr)).unwrap_or(usize::MAX),
                    value
                ));
            }
            Op::CONST_STK => {
                let value = pop(f);
                let name = native_operand_bytes!(pop(f));
                try_op!(self.declare_constant_bytes(&name, value, false));
            }

            // -- global / upvar links. The namespace ("::") or level reference
            // is left on the stack for the next link op to reuse; a trailing
            // `pop` discards it (matching C Tcl's nsupvar/upvar codegen). --
            Op::NSUPVAR => {
                let other = pop(f);
                let Some(namespace_operand) = f.stack.last() else {
                    return Tick::Return(err("nsupvar: stack underflow"));
                };
                let namespace = try_core!(self.namespace_object_lookup(namespace_operand));
                let Some(namespace) = namespace else {
                    let namespace_bytes = native_operand_bytes!(
                        namespace_operand.native_lifetime_lease().into_value()
                    );
                    return Tick::Return(self.namespace_lookup_error_bytes(&namespace_bytes));
                };
                try_op!(self.link_compiled_namespace_original(
                    usize::try_from(imm0(instr)).unwrap_or(usize::MAX),
                    namespace,
                    &other,
                    false,
                ));
            }
            Op::UPVAR => {
                let other = pop(f);
                let Some(level_operand) = f
                    .stack
                    .last()
                    .map(|value| value.native_lifetime_lease().into_value())
                else {
                    return Tick::Return(err("upvar: stack underflow"));
                };
                let (_, target) = try_op!(crate::command::runtime_frame_selection(
                    self,
                    tcl_registry::FrameEffectSpec::UPVAR,
                    &[level_operand]
                ));
                try_op!(self.link_compiled_upvar(
                    usize::try_from(imm0(instr)).unwrap_or(usize::MAX),
                    target,
                    &other,
                ));
            }
            // `variable` (C `INST_VARIABLE`): link the local slot to the
            // namespace variable named by the popped original object. Namespace-only
            // lookup updates its selected name cache before binding the local slot.
            Op::VARIABLE => {
                let original = pop(f);
                try_op!(self.link_compiled_namespace_original(
                    usize::try_from(imm0(instr)).unwrap_or(usize::MAX),
                    self.current_ns_id(),
                    &original,
                    true,
                ));
            }

            // Concat (stack form): Tcl-concat the top N values.
            Op::CONCAT_STK => {
                let n = usize::try_from(imm0(instr)).unwrap_or(0);
                let take = f.stack.len().saturating_sub(n);
                let vals: Vec<Value> = f.stack.split_off(take);
                match tcl_cmd_core::list::concat_selected(self, &vals) {
                    Ok(value) => f.stack.push(value),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self, error,
                        ));
                    }
                }
            }
            Op::LIST_CONCAT => {
                let source = pop(f);
                let target = pop(f);
                let protocol = match crate::cmd_dict::VmDictionaryObjects::selected(self) {
                    Ok(objects) => objects.string_protocol(),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self, error,
                        ));
                    }
                };
                let result = target.native_list_concatenate(&source, protocol);
                match result {
                    Ok(value) => f.stack.push(value),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            error.into(),
                        ));
                    }
                }
            }
            // Inline `lreplace`/`linsert` (C Tcl `INST_LREPLACE4`). Operands:
            // [argc, mode] where mode 1 = lreplace, 2 = linsert. Stack holds
            // `argc` values in push order: list, index arg(s), then elements.
            Op::LREPLACE4 => {
                let argc = usize::try_from(imm0(instr)).unwrap_or(0);
                let mode = imm_at(instr, 1);
                let take = f.stack.len().saturating_sub(argc);
                let vals: Vec<Value> = f.stack.split_off(take);
                let result = match vals.split_first() {
                    None => Vec::new(),
                    Some((list_v, _)) => {
                        let items = match list_v.as_list() {
                            Ok(l) => (*l).clone(),
                            Err(e) => {
                                return Tick::Return(crate::command::completion_from_tcl_error(
                                    self, e,
                                ));
                            }
                        };
                        let n = items.len();
                        let nlen = isize::try_from(n).unwrap_or(isize::MAX);
                        if mode == 1 {
                            // lreplace: list first last ?elem ...?
                            let first = match vals.get(1) {
                                Some(v) => match checked_index_value(self, v, n) {
                                    Ok(i) => i.max(0),
                                    Err(c) => return Tick::Return(c),
                                },
                                None => 0,
                            };
                            let last = match vals.get(2) {
                                Some(v) => match checked_index_value(self, v, n) {
                                    Ok(i) => i,
                                    Err(c) => return Tick::Return(c),
                                },
                                None => -1,
                            };
                            let new_elems = vals.get(3..).unwrap_or(&[]);
                            let fu = usize::try_from(first).unwrap_or(0).min(n);
                            let mut r = items[..fu].to_vec();
                            r.extend_from_slice(new_elems);
                            if last >= first {
                                let last = last.min(nlen - 1);
                                let lu = usize::try_from(last + 1).unwrap_or(n).min(n);
                                r.extend_from_slice(&items[lu..]);
                            } else {
                                r.extend_from_slice(&items[fu..]);
                            }
                            r
                        } else {
                            // linsert: list index ?elem ...? ("end" → after last).
                            let idx = match vals.get(1) {
                                Some(v) => match checked_index_value(self, v, n + 1) {
                                    Ok(i) => i,
                                    Err(c) => return Tick::Return(c),
                                },
                                None => 0,
                            };
                            let idx = usize::try_from(idx.max(0)).unwrap_or(0).min(n);
                            let new_elems = vals.get(2..).unwrap_or(&[]);
                            let mut r = items[..idx].to_vec();
                            r.extend_from_slice(new_elems);
                            r.extend_from_slice(&items[idx..]);
                            r
                        }
                    }
                };
                f.stack.push(Value::list(result));
            }

            // -- foreach (C Tcl INST_FOREACH_*): aux var groups on the
            //    instruction; foreach_start jumps to step; step binds the loop
            //    vars and jumps back to the body, or falls through to end. --
            Op::FOREACH_START => {
                if let Some(auxiliary) = &instr.native_each {
                    return self.native_compiled_each_start(f, auxiliary, instr.foreach_collect);
                }
                let policy = if instr.foreach_collect {
                    tcl_runtime_api::completion_options::ControlOptionPolicy::FRESH_FORWARDED
                } else {
                    tcl_runtime_api::completion_options::ControlOptionPolicy::FRESH_SETTLED
                };
                if policy.begins_fresh() {
                    f.last_options = Value::empty();
                }
                let start_idx = f.pc - 1;
                let body_idx = f.pc;
                let groups = instr.foreach_vars.clone().unwrap_or_default();
                let n = groups.len();
                if f.stack.len() < n {
                    return Tick::Return(err("foreach: stack underflow"));
                }
                let raw = f.stack.split_off(f.stack.len() - n);
                let mut value_lists = Vec::with_capacity(n);
                let mut iter_max = 0;
                let Some(protocol) = self
                    .actual_native_invocation_dialect()
                    .native_string_protocol()
                else {
                    return Tick::Return(self.refuse_host_command(
                        "native foreach list protocol is unavailable".into(),
                    ));
                };
                for (gi, lv) in raw.iter().enumerate() {
                    let items = match self.native_object_list_elements_in(lv, protocol) {
                        Ok(items) => items,
                        Err(error) => {
                            return Tick::Return(crate::command::completion_from_cmd_error(
                                self,
                                error.into(),
                            ));
                        }
                    };
                    let nv = groups[gi].len().max(1);
                    iter_max = iter_max.max(items.len().div_ceil(nv));
                    value_lists.push(items);
                }
                f.foreach_stack.push(ForeachState {
                    native: None,
                    _native_iterator_headers: None,
                    native_accumulator: None,
                    var_groups: groups,
                    lists: value_lists,
                    _list_roots: raw,
                    iter_num: 0,
                    iter_max,
                    body_idx,
                    collect: instr.foreach_collect,
                    accum: Vec::new(),
                });
                if let Some(&step) = f.foreach_pairs.get(&start_idx) {
                    f.pc = step;
                }
            }
            Op::FOREACH_STEP => {
                if f.foreach_stack
                    .last()
                    .is_some_and(|state| state.native.is_some())
                {
                    return self.native_compiled_each_step(f);
                }
                let (binds, body, more) = {
                    match f.foreach_stack.last_mut() {
                        None => (Vec::new(), 0usize, false),
                        Some(st) if st.iter_num >= st.iter_max => (Vec::new(), 0, false),
                        Some(st) => {
                            let it = st.iter_num;
                            let mut binds: Vec<(tcl_bytecode::CompiledVariableTarget, Value)> =
                                Vec::new();
                            for (gi, group) in st.var_groups.iter().enumerate() {
                                let nv = group.len();
                                for (j, var) in group.iter().enumerate() {
                                    let v = st.lists[gi]
                                        .get(it * nv + j)
                                        .cloned()
                                        .unwrap_or_else(Value::empty);
                                    binds.push((var.clone(), v));
                                }
                            }
                            st.iter_num += 1;
                            (binds, st.body_idx, true)
                        }
                    }
                };
                if more {
                    f.last_options = Value::empty();
                    for (name, v) in binds {
                        match name {
                            tcl_bytecode::CompiledVariableTarget::Slot(slot) => {
                                try_op!(self.store_compiled_variable_result(slot, None, v));
                            }
                            tcl_bytecode::CompiledVariableTarget::Name(name) => {
                                try_op!(self.set_var_bytes(name.as_bytes(), v));
                            }
                        }
                    }
                    f.pc = body;
                }
            }
            Op::LMAP_COLLECT => {
                // Append this fall-through iteration's result to the collecting
                // loop's accumulator. A `break`/`continue` redirect jumps past this
                // opcode (to `FOREACH_END` / `FOREACH_STEP`), so a skipped iteration
                // contributes nothing — matching C `lmap`.
                let v = pop(f);
                if let Some(st) = f.foreach_stack.last_mut() {
                    if let (Some(accumulator), Some(auxiliary)) =
                        (&st.native_accumulator, &st.native)
                    {
                        let protocol =
                            tcl_syntax::native_string::NativeStringProtocol::C(auxiliary.version);
                        if let Err(error) =
                            accumulator.native_list_append_prepared_elements(&[v], protocol)
                        {
                            return Tick::Return(crate::command::completion_from_cmd_error(
                                self,
                                error.into(),
                            ));
                        }
                    } else {
                        st.accum.push(v);
                    }
                }
            }
            Op::FOREACH_END => {
                let st = f.foreach_stack.pop();
                // A collecting loop (`lmap`) yields `list(accum)`; a plain
                // `foreach` yields nothing (its `""` result is pushed by the
                // loop-end block).
                if let Some(st) = st {
                    if st.collect {
                        f.stack.push(
                            st.native_accumulator
                                .unwrap_or_else(|| Value::list(st.accum)),
                        );
                    } else {
                        f.last_options = Value::empty();
                    }
                }
            }

            // Compiled `dict for` / `dict map` iteration primitives. `dictFirst
            // <slot>` pops the dict (TOS), records an iterator in `<slot>`, and
            // pushes `value, key, done` (done on top). `dictNext <slot>`
            // advances and pushes the next `value, key, done`. `done` is the
            // "exhausted" flag the compiled loop tests with `jumpTrue`
            // (dictFirst: skip an empty dict) / `jumpFalse` (dictNext: loop
            // while more). On exhaustion the pushed key/value are empty
            // placeholders the loop epilogue pops.
            Op::DICT_FIRST => {
                let slot = imm0(instr);
                let dict = pop(f);
                let Some(protocol) = self
                    .actual_native_invocation_dialect()
                    .native_string_protocol()
                else {
                    return Tick::Return(self.refuse_host_command(
                        "native dictionary search protocol is unavailable".into(),
                    ));
                };
                let mut search = match dict.into_native_dictionary_search(protocol) {
                    Ok(search) => search,
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            error.into(),
                        ));
                    }
                };
                #[cfg(test)]
                {
                    self.actual_dictionary_search_entries += 1;
                }
                let pair = match search.next_pair() {
                    Ok(pair) => pair,
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            error.into(),
                        ));
                    }
                };
                let done = pair.is_none();
                let (k, v) = pair.unwrap_or_else(|| (Value::empty(), Value::empty()));
                f.dict_iters.insert(slot, DictIterState { search });
                f.stack.push(v);
                f.stack.push(k);
                f.stack.push(Value::bool(done));
            }
            Op::DICT_NEXT => {
                let pair = match f.dict_iters.get_mut(&imm0(instr)) {
                    Some(state) => match state.search.next_pair() {
                        Ok(pair) => pair,
                        Err(error) => {
                            return Tick::Return(crate::command::completion_from_cmd_error(
                                self,
                                error.into(),
                            ));
                        }
                    },
                    None => None,
                };
                let done = pair.is_none();
                let (k, v) = pair.unwrap_or_else(|| (Value::empty(), Value::empty()));
                f.stack.push(v);
                f.stack.push(k);
                f.stack.push(Value::bool(done));
            }
            Op::DICT_UPDATE_START => {
                let slot = usize::try_from(imm0(instr)).unwrap_or(usize::MAX);
                let vars = instr.dict_vars.as_deref().unwrap_or_default();
                let Some(keys) = f.stack.last() else {
                    return Tick::Return(err("dictUpdateStart: stack underflow"));
                };
                try_op!(self.start_compiled_dictionary_update(slot, keys, vars));
            }
            Op::DICT_UPDATE_END => {
                let slot = usize::try_from(imm0(instr)).unwrap_or(usize::MAX);
                let keys = pop(f);
                try_op!(self.finish_compiled_dictionary_update(
                    slot,
                    &keys,
                    instr.dict_vars.as_deref().unwrap_or_default()
                ));
            }
            Op::DICT_EXPAND => {
                let path = pop(f);
                let dictionary = pop(f);
                let keys = try_op!(crate::cmd_dict::expand_dictionary_scope(
                    self,
                    &dictionary,
                    &path
                ));
                f.stack.push(keys);
            }
            Op::DICT_RECOMBINE_IMM | Op::DICT_RECOMBINE_STK => {
                let state = pop(f);
                let path = pop(f);
                let (name, publication) = if instr.op == Op::DICT_RECOMBINE_STK {
                    (tcl_core_types::NameBytes::from(native_operand_bytes!(pop(f)).as_ref()),
                     crate::interp::native_dictionary::DictionaryVariablePublication::RetainedLocalCell)
                } else {
                    (lvt_name(imm0(instr)), crate::interp::native_dictionary::DictionaryVariablePublication::RetainedCompiledLocal(
                        usize::try_from(imm0(instr)).unwrap_or(usize::MAX)))
                };
                try_op!(crate::cmd_dict::recombine_dictionary_scope(
                    self,
                    name.as_bytes(),
                    publication,
                    &path,
                    &state
                ));
            }
            Op::VERIFY_DICT => {
                let top = pop(f);
                match tcl_syntax::value::ValueOps::dict_pairs(self, &top) {
                    Ok(pairs) => drop(pairs),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            error.into(),
                        ));
                    }
                }
            }

            // Arithmetic / bitwise / shift.
            Op::ADD => try_op!(bin(self, f, BinOp::Add)),
            Op::SUB => try_op!(bin(self, f, BinOp::Sub)),
            Op::MULT => try_op!(bin(self, f, BinOp::Mul)),
            Op::DIV => try_op!(bin(self, f, BinOp::Div)),
            Op::MOD => try_op!(bin(self, f, BinOp::Mod)),
            Op::EXPON => try_op!(bin(self, f, BinOp::Pow)),
            Op::LSHIFT => try_op!(bin(self, f, BinOp::LShift)),
            Op::RSHIFT => try_op!(bin(self, f, BinOp::RShift)),
            Op::BITAND => try_op!(bin(self, f, BinOp::BitAnd)),
            Op::BITOR => try_op!(bin(self, f, BinOp::BitOr)),
            Op::BITXOR => try_op!(bin(self, f, BinOp::BitXor)),
            Op::LAND => try_op!(land_lor(self, f, true)),
            Op::LOR => try_op!(land_lor(self, f, false)),

            // Comparisons.
            Op::EQ => try_op!(cmp(self, f, BinOp::Eq)),
            Op::NEQ => try_op!(cmp(self, f, BinOp::Ne)),
            Op::LT => try_op!(cmp(self, f, BinOp::Lt)),
            Op::GT => try_op!(cmp(self, f, BinOp::Gt)),
            Op::LE => try_op!(cmp(self, f, BinOp::Le)),
            Op::GE => try_op!(cmp(self, f, BinOp::Ge)),
            Op::ERROR_PREFIX_EQ => {
                let prefix = pop(f);
                let original = pop(f);
                let Some(protocol) = self
                    .actual_native_invocation_dialect()
                    .native_string_protocol()
                    .filter(|protocol| {
                        protocol
                            .tcl_version()
                            .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_1)
                    })
                else {
                    return Tick::Return(self.refuse_host_command(
                        "native error-prefix comparison recipe is unavailable".into(),
                    ));
                };
                let left = try_core!(self.native_object_list_elements_in(&original, protocol));
                let right = try_core!(self.native_object_list_elements_in(&prefix, protocol));
                let length = usize::try_from(imm0(instr)).expect("native error-prefix length");
                let mut matched = true;
                for index in 0..length {
                    matched = match (left.get(index), right.get(index)) {
                        (Some(left), Some(right)) => {
                            try_core!(tcl_cmd_core::switch::compiled_equal(
                                self,
                                left,
                                right,
                                tcl_dialect::TclVersion::V9_1
                            ))
                        }
                        (Some(value), None) | (None, Some(value)) => {
                            use tcl_syntax::native_object::{
                                NativeObjectStringEmptiness, native_c_string_emptiness,
                            };
                            let snapshot = try_core!(
                                tcl_syntax::value::ValueOps::native_object_snapshot(self, value)
                            );
                            match native_c_string_emptiness(
                                tcl_dialect::TclVersion::V9_1,
                                &snapshot,
                            ) {
                                Ok(NativeObjectStringEmptiness::Empty) => true,
                                Ok(NativeObjectStringEmptiness::Nonempty) => false,
                                Ok(NativeObjectStringEmptiness::Unknown) => {
                                    try_core!(value.native_string_bytes(protocol).map_err(
                                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable
                                ))
                                    .is_empty()
                                }
                                Err(_) => {
                                    return Tick::Return(
                                        self.refuse_host_command(
                                            "native error-prefix emptiness recipe is unavailable"
                                                .into(),
                                        ),
                                    );
                                }
                            }
                        }
                        (None, None) => true,
                    };
                    if !matched {
                        break;
                    }
                }
                f.stack.push(Value::int(i64::from(matched)));
            }
            Op::STR_EQ if instr.native_switch_version.is_some() => {
                let right = pop(f);
                let left = pop(f);
                let matched = try_core!(tcl_cmd_core::switch::compiled_equal(
                    self,
                    &left,
                    &right,
                    instr.native_switch_version.unwrap()
                ));
                drop(right);
                let result = try_core!(self.native_compiled_match_result(
                    left,
                    matched,
                    instr.native_switch_version.unwrap(),
                    tcl_registry::native_string_compilation::NativeStringMatchOperation::Equal
                ));
                f.stack.push(result);
            }
            Op::STR_EQ => try_op!(cmp(self, f, BinOp::StrEq)),
            Op::STR_NEQ => try_op!(cmp(self, f, BinOp::StrNe)),
            Op::STR_LT => try_op!(cmp(self, f, BinOp::StrLt)),
            Op::STR_GT => try_op!(cmp(self, f, BinOp::StrGt)),
            Op::STR_LE => try_op!(cmp(self, f, BinOp::StrLe)),
            Op::STR_GE => try_op!(cmp(self, f, BinOp::StrGe)),
            Op::STR_CMP => {
                let b = pop(f);
                let a = pop(f);
                let ord = (*a.to_str()).cmp(&b.to_str());
                f.stack.push(Value::int(match ord {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                }));
            }

            // iRules dialect operators.
            // The F5 word operators (`contains`/`starts_with`/`ends_with`/
            // `equals`/`matches`/`matches_glob`/`matches_regex`/`and`/`or`/
            // `not`), which
            // `Op::from_binop`/`from_unaryop` emit for an iRules expression.
            // They have no C Tcl counterpart; the semantics live in `expr` next
            // to the standard operators, and reuse the same glob matcher, ARE
            // engine and string comparison the equivalent commands do. Operands
            // arrive left-then-right (subject below, needle/pattern on top).
            Op::IRULE_CONTAINS => try_op!(irule(self, f, BinOp::Contains)),
            Op::IRULE_STARTS_WITH => try_op!(irule(self, f, BinOp::StartsWith)),
            Op::IRULE_ENDS_WITH => try_op!(irule(self, f, BinOp::EndsWith)),
            Op::IRULE_EQUALS => try_op!(irule(self, f, BinOp::StrEquals)),
            Op::IRULE_MATCHES_GLOB => try_op!(irule(self, f, BinOp::MatchesGlob)),
            Op::IRULE_MATCHES_REGEX => try_op!(irule(self, f, BinOp::MatchesRegex)),
            Op::IRULE_MATCHES => try_op!(irule(self, f, BinOp::Matches)),
            Op::IRULE_WORD_AND => try_op!(irule(self, f, BinOp::WordAnd)),
            Op::IRULE_WORD_OR => try_op!(irule(self, f, BinOp::WordOr)),
            Op::IRULE_WORD_NOT => {
                try_op!(un(self, f, UnaryOp::WordNot));
            }

            // String ops (inline; char-based, mirroring the reference VM)
            Op::STR_LEN if instr.native_switch_version.is_some() => {
                let original = pop(f);
                let result = try_core!(self.execute_native_scalar_length(
                    &original,
                    tcl_registry::native_scalar_compilation::NativeScalarOperation::StringLength,
                    instr.native_switch_version.unwrap(),
                ));
                drop(original);
                f.stack.push(result);
            }
            Op::STR_LEN => {
                let value = pop(f);
                match tcl_cmd_core::string::length(self, &value) {
                    Ok(value) => f.stack.push(value),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self, error,
                        ));
                    }
                }
            }
            // C `INST_STR_INDEX` (`tclExecute.c:5336-5380`): the index goes
            // through `TclGetIntForIndexM` and a *syntactically* bad spec is
            // `goto gotError` (`bad index "foo": …`) — only the separate
            // `index < 0 || index >= slength` test yields the empty string. The
            // old `resolve_index(...).and_then(...)` chain collapsed both into
            // the empty string, so a garbage index looked like a miss.
            Op::STR_INDEX if instr.native_switch_version.is_some() => {
                let index = pop(f);
                let subject = pop(f);
                let result = try_core!(tcl_cmd_core::string::compiled_index(
                    self,
                    &subject,
                    &index,
                    instr.native_switch_version.unwrap(),
                ));
                f.stack.push(result);
            }
            Op::STR_INDEX => {
                let index = pop(f);
                let value = pop(f);
                let result = try_core!(tcl_cmd_core::string::index(self, &value, &index));
                f.stack.push(result);
            }
            Op::STR_RANGE if instr.native_switch_version.is_some() => {
                let last = pop(f);
                let first = pop(f);
                let subject = pop(f);
                let result = try_core!(tcl_cmd_core::string::compiled_range(
                    self,
                    &subject,
                    &first,
                    &last,
                    instr.native_switch_version.unwrap()
                ));
                f.stack.push(result);
            }
            Op::STR_RANGE => {
                let last = pop(f);
                let first = pop(f);
                let value = pop(f);
                let result = try_core!(tcl_cmd_core::string::range(self, &value, &first, &last));
                f.stack.push(result);
            }
            Op::STR_RANGE_IMM => {
                let value = pop(f);
                let first = encoded_string_index(imm0(instr));
                let last = encoded_string_index(imm_at(instr, 1));
                let result = try_core!(tcl_cmd_core::string::range(self, &value, &first, &last));
                f.stack.push(result);
            }
            Op::STR_FIND | Op::STR_RFIND if instr.native_switch_version.is_some() => {
                let subject = pop(f);
                let needle = pop(f);
                let version = instr.native_switch_version.unwrap();
                let index = try_core!(tcl_cmd_core::string::compiled_find(
                    self,
                    &needle,
                    &subject,
                    version,
                    instr.op == Op::STR_RFIND
                ));
                let result = Value::int(index);
                try_core!(result.set_native_unshared_integer(index, version));
                f.stack.push(result);
            }
            Op::STR_FIND => {
                let s = pop(f).to_str();
                let needle = pop(f).to_str();
                f.stack.push(Value::int(char_find(&s, &needle, false)));
            }
            Op::STR_RFIND => {
                let s = pop(f).to_str();
                let needle = pop(f).to_str();
                f.stack.push(Value::int(char_find(&s, &needle, true)));
            }
            Op::STR_MATCH if instr.native_switch_version.is_some() => {
                let subject = pop(f);
                let pattern = pop(f);
                let matched = try_core!(tcl_cmd_core::switch::compiled_glob(
                    self,
                    &pattern,
                    &subject,
                    instr.native_switch_version.unwrap(),
                    imm0(instr) != 0
                ));
                drop(subject);
                let result = try_core!(self.native_compiled_match_result(
                    pattern,
                    matched,
                    instr.native_switch_version.unwrap(),
                    tcl_registry::native_string_compilation::NativeStringMatchOperation::Glob {
                        nocase: imm0(instr) != 0
                    }
                ));
                f.stack.push(result);
            }
            Op::STR_MATCH => {
                let s = pop(f).to_str();
                let pat = pop(f).to_str();
                let nocase = imm0(instr) != 0;
                f.stack
                    .push(Value::bool(tcl_syntax::glob::string_case_match(
                        &pat, &s, nocase,
                    )));
            }
            // C `INST_STR_UPPER`/`INST_STR_LOWER`/`INST_STR_TITLE`
            // (`tclExecute.c:5284-5334`) call `Tcl_UtfToUpper`/`ToLower`/`ToTitle`,
            // which map **one code point to one code point** through Tcl's own
            // tables (`Tcl_UniCharToUpper` &co, `tclUtf.c:1777-1858`) and so never
            // change the character count. Rust's `str::to_uppercase()` implements
            // *full* Unicode mapping and expands (`ß` → `SS`, `İ` → `i` + U+0307),
            // which changed `string length` of the result; the shared
            // `simple_*` helpers restrict it to the 1:1 mapping and are the same
            // ones the `string toupper`/`tolower`/`totitle` commands use.
            Op::STR_LOWER if instr.native_switch_version.is_some() => {
                let original = pop(f);
                let version = instr.native_switch_version.unwrap();
                let bytes = try_core!(tcl_syntax::value::ValueOps::native_string_bytes(
                    self, &original
                ));
                let lowered = tcl_syntax::native_glob::lower_c_string_bytes(version, &bytes);
                let value = Value::new_native_string_bytes(lowered);
                if original.native_object_is_shared() {
                    let materialization = self
                        .native_invocation_dialect()
                        .native_string_materialization(None)
                        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "compiled string lower issuer",
                        ));
                    let materialization = try_core!(materialization);
                    try_core!(value.retain_native_string_representation(materialization));
                    f.stack.push(value);
                } else {
                    try_core!(
                        original.adopt_native_object_representation_with_string_mutation(
                            &value,
                            self.native_invocation_dialect(),
                            tcl_core_types::ResidentStringMutation::Replace
                        )
                    );
                    f.stack.push(original);
                }
            }
            Op::STR_UPPER | Op::STR_LOWER => {
                let s = pop(f).to_str();
                let map = if instr.op == Op::STR_UPPER {
                    tcl_cmd_core::string::simple_upper
                } else {
                    tcl_cmd_core::string::simple_lower
                };
                f.stack
                    .push(Value::string(s.chars().map(map).collect::<String>()));
            }
            Op::STR_TITLE => {
                let s = pop(f).to_str();
                let mut out = String::with_capacity(s.len());
                for (i, c) in s.chars().enumerate() {
                    out.push(if i == 0 {
                        tcl_cmd_core::string::simple_title(c)
                    } else {
                        tcl_cmd_core::string::simple_title_rest(c)
                    });
                }
                f.stack.push(Value::string(out));
            }
            Op::STR_REVERSE => {
                let value = pop(f);
                let result = try_core!(tcl_cmd_core::string::reverse(self, &value));
                f.stack.push(result);
            }
            // `strmap` is the one-pair, always-case-sensitive `string map` C
            // compiles a two-element literal charMap to (`INST_STR_MAP`). The
            // stack is `from to string` — the subject on top — and the mapping
            // itself is the `string map` command's (`cmd_string::map_apply`).
            Op::STR_MAP => {
                let s = pop(f).to_str();
                let to = pop(f).to_str().to_string();
                let from = pop(f).to_str().to_string();
                let out = crate::cmd_string::map_apply(&[(from, to)], &s, false);
                f.stack.push(Value::string(out));
            }
            Op::STR_REPEAT => {
                let count = pop(f);
                let value = pop(f);
                let count = try_core!(tcl_cmd_core::string::prepare_repeat_count(self, &count));
                let bytes = value.string_bytes();
                let wanted = u64::try_from(bytes.len())
                    .unwrap_or(u64::MAX)
                    .saturating_mul(u64::try_from(count.max(0)).unwrap_or(u64::MAX));
                if let Some(refusal) = self.charge_allocation(wanted) {
                    return Tick::Return(refusal);
                }
                let result =
                    try_core!(tcl_cmd_core::string::repeat_with_count(self, &value, count));
                f.stack.push(result);
            }
            Op::STR_TRIM | Op::STR_TRIM_LEFT | Op::STR_TRIM_RIGHT
                if instr.native_switch_version.is_some() =>
            {
                let characters = pop(f);
                let subject = pop(f);
                let result = try_core!(tcl_cmd_core::string::compiled_trim(
                    self,
                    &subject,
                    &characters,
                    instr.native_switch_version.unwrap(),
                    instr.op != Op::STR_TRIM_RIGHT,
                    instr.op != Op::STR_TRIM_LEFT
                ));
                f.stack.push(result);
            }
            Op::STR_TRIM | Op::STR_TRIM_LEFT | Op::STR_TRIM_RIGHT => {
                let chars = pop(f).to_str();
                let s = pop(f).to_str();
                let set: Vec<char> = chars.chars().collect();
                let pred = |c: char| set.contains(&c);
                let out = match instr.op {
                    Op::STR_TRIM => s.trim_matches(pred),
                    Op::STR_TRIM_LEFT => s.trim_start_matches(pred),
                    _ => s.trim_end_matches(pred),
                };
                f.stack.push(Value::string(out));
            }

            // Numeric/boolean coercion checks (expr result validation)
            Op::TRY_CVT_TO_NUMERIC => {
                // Canonical normalisation (C Tcl `INST_TRY_CVT_TO_NUMERIC`): a
                // numeric result's string rep is regenerated from the number
                // (`expr {1e3}` → `1000.0`, not `1e3`), a non-numeric value
                // (`expr {1 ? "big" : "x"}`) passes through, and a bare `NaN` is
                // the domain error.
                let v = pop(f);
                match crate::expr::normalize_numeric_instruction_for_vm(self, &v) {
                    Ok(nv) => f
                        .stack
                        .push(nv.with_native_double_format(self.native_invocation_dialect())),
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_tcl_error(self, e));
                    }
                }
            }
            // C `INST_TRY_CVT_TO_BOOLEAN` (`tclExecute.c:6404-6414`; table
            // `tclCompile.c:616` — `{"tryCvtToBoolean", 1, +1, 0, …}`, "Try
            // converting stktop to boolean if possible. **No errors.** Stack:
            // … value => … value isStrictBool"): the value is *left in place*
            // and a 0/1 "is a valid boolean" flag is pushed on top, and the
            // instruction never raises. Popping/re-pushing the value (and
            // erroring on a non-boolean) desynchronised the operand stack from
            // our own `string is boolean` codegen
            // (`tcl-compiler/src/codegen/cmd_subst.rs`), which emits C's
            // `tryCvtToBoolean; jumpTrue L; push ""; streq; jump end; L: pop;
            // push "1"` sequence and so needs both the flag and the value.
            // The flag is C's *strict* test — `TclSetBooleanFromAny` /
            // `ParseBoolean` (`tclObj.c:2100-2280`), hence the table's
            // `isStrictBool` — which accepts only `0`, `1` and the unique
            // case-insensitive prefixes of `yes`/`no`/`true`/`false`/`on`/`off`,
            // *not* `Value::as_bool`'s `Tcl_GetBooleanFromObj` leniency (any
            // non-zero number). So `string is boolean 2` is 0 while `if {2}`
            // still runs, and the opcode agrees with the `string is boolean`
            // command (`tcl_cmd_core::string_is`, same acceptor).
            Op::TRY_CVT_TO_BOOLEAN => {
                let is_bool = f.stack.last().is_some_and(|v| {
                    tcl_syntax::boolean::parse_boolean_strict(&v.to_str()).is_some()
                });
                f.stack.push(Value::bool(is_bool));
            }

            // Unary.
            Op::UMINUS => try_op!(un(self, f, UnaryOp::Neg)),
            Op::UPLUS => try_op!(un(self, f, UnaryOp::Pos)),
            Op::BITNOT => try_op!(un(self, f, UnaryOp::BitNot)),
            Op::NOT | Op::LNOT => try_op!(un(self, f, UnaryOp::Not)),

            // Control flow.
            Op::JUMP1 | Op::JUMP4 => {
                if let Some(idx) = jump_target(&asm, &f.off2idx, instr) {
                    f.pc = idx;
                }
            }
            Op::JUMP_TRUE1 | Op::JUMP_TRUE4 => {
                let c = pop(f);
                match expr::boolean_for_vm(
                    self,
                    &c,
                    tcl_registry::native_boolean_truth::NativeBooleanTruthPurpose::ConditionalJump,
                ) {
                    Ok(true) => {
                        if let Some(idx) = jump_target(&asm, &f.off2idx, instr) {
                            f.pc = idx;
                        }
                    }
                    Ok(false) => {}
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_tcl_error(self, e));
                    }
                }
            }
            Op::JUMP_FALSE1 | Op::JUMP_FALSE4 => {
                let c = pop(f);
                match expr::boolean_for_vm(
                    self,
                    &c,
                    tcl_registry::native_boolean_truth::NativeBooleanTruthPurpose::ConditionalJump,
                ) {
                    Ok(false) => {
                        if let Some(idx) = jump_target(&asm, &f.off2idx, instr) {
                            f.pc = idx;
                        }
                    }
                    Ok(true) => {}
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_tcl_error(self, e));
                    }
                }
            }
            Op::JUMP_TABLE if instr.native_switch_version.is_some() => {
                let original = pop(f);
                let label = if let Some(table) = &instr.native_switch_integers {
                    let integer = try_core!(tcl_syntax::value::ValueOps::as_int(self, &original));
                    table.get(&integer)
                } else if let Some(table) = &instr.native_switch_bytes {
                    let bytes = try_core!(tcl_syntax::value::ValueOps::native_string_bytes(
                        self, &original
                    ));
                    table.get(tcl_core_types::c_string_extent(&bytes))
                } else {
                    unreachable!("authenticated native switch table");
                };
                if let Some(label) = label
                    && let Some(index) = label_to_idx(&asm, &f.off2idx, label)
                {
                    f.pc = index;
                }
            }
            Op::JUMP_TABLE => {
                let key = pop(f).to_str();
                if let Some(jt) = &instr.jump_table
                    && let Some(label) = jt.get(&*key)
                    && let Some(idx) = label_to_idx(&asm, &f.off2idx, label)
                {
                    f.pc = idx;
                }
            }

            // Break / continue.
            Op::BREAK => {
                return Tick::Return(Completion::new(Code::Break, Value::empty(), Value::empty()));
            }
            Op::CONTINUE => {
                return Tick::Return(Completion::new(
                    Code::Continue,
                    Value::empty(),
                    Value::empty(),
                ));
            }

            // Return.
            // `SYNTAX` shares `RETURN_IMM`'s arm in C too (`tclExecute.c:2287`
            // falls through the two labels): both carry `(code, level)`
            // immediates and read `OBJ_AT_TOS` as the return-options dict,
            // `OBJ_UNDER_TOS` as the result — `CompileReturnInternal` pushes the
            // options *last*. The inline `if {…}`/`expr` codegen emits `syntax`
            // to raise a compile-time expression error at runtime.
            //
            // `TclProcessReturn` (`tclResult.c:777-785`) decides the completion:
            // `level != 0` raises TCL_RETURN carrying `-code`/`-level` for the
            // proc-boundary countdown, while `level == 0` makes the completion
            // *be* `code` — so `(0, 0)` is not a return at all, it falls through
            // to the next instruction with the result left on the stack
            // (`INST_RETURN_IMM`'s `NEXT_INST_F(9, 1, 0)` pops only the dict).
            //
            // The immediates are the authoritative merged `-code`/`-level`:
            // `TclMergeReturnOptions` consumes both keys out of the literal
            // dict, which is why they ride the operands. Appending them after
            // `-options` lets them win, exactly as C's out-params do.
            Op::RETURN_IMM | Op::SYNTAX => {
                // C's compiler guarantees both stack slots. Ours only pushes the
                // options literal outside a proc — inside one the plain return is
                // expected to fold to `done` — so an unfolded proc-body
                // `returnImm` arrives with the result alone; treat that as empty
                // options rather than eating whatever sits underneath.
                let (result, options) = if f.stack.len() >= 2 {
                    let opts = pop(f);
                    let res = pop(f);
                    (res, opts)
                } else {
                    (pop(f), Value::empty())
                };
                let application = if instr.op == Op::SYNTAX {
                    tcl_registry::native_return_options::NativeReturnOptionsApplication::Syntax
                } else {
                    tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate
                };
                let native_context = self
                    .native_invocation_dialect()
                    .native_return_options_application(application)
                    .and_then(tcl_registry::native_return_options::NativeReturnOptionsApplicationProtocol::inner_context_name)
                    .map(|name| {
                        (
                            name,
                            result.native_lifetime_lease(),
                            options.native_lifetime_lease(),
                        )
                    });
                let c = if self
                    .native_invocation_dialect()
                    .native_return_options_application(application)
                    .is_some()
                {
                    crate::native_return_merge::process(
                        self,
                        application,
                        imm0(instr),
                        i64::from(imm_at(instr, 1)),
                        &options,
                        result,
                    )
                } else {
                    crate::command::apply_return_options(
                        self,
                        &[
                            Value::string("-options"),
                            options,
                            Value::string("-code"),
                            Value::int(i64::from(imm0(instr))),
                            Value::string("-level"),
                            Value::int(i64::from(imm_at(instr, 1))),
                            result,
                        ],
                    )
                };
                if c.code == Code::Error
                    && let Some((name, result, options)) = native_context
                {
                    self.capture_original_return_instruction_context(
                        application,
                        name,
                        &[result.into_value(), options.into_value()],
                    );
                }
                if let Err(c) = self.deliver_sync(f, c) {
                    return Tick::Return(c);
                }
            }
            // `returnStk` is C-ordered — result on top, the return-options dict
            // under it — and applies the dict exactly as `return -options $opts
            // $result` would: `-code`/`-level` drive the completion (level 0 ⇒
            // the code takes effect immediately, level > 0 ⇒ `TCL_RETURN`
            // counted down at proc boundaries), and a malformed dict overrides
            // the result with its own error (C `INST_RETURN_STK` →
            // `Tcl_SetReturnOptions`). An outcome of TCL_OK (`-level 0 -code
            // ok`) pushes the result and *continues* — C only routes non-OK
            // codes to `processExceptionReturn`.
            Op::RETURN_STK => {
                let result = pop(f);
                let opts = pop(f);
                let c = if self
                    .native_invocation_dialect()
                    .native_return_options_application(
                        tcl_registry::native_return_options::NativeReturnOptionsApplication::Stack,
                    )
                    .is_some()
                {
                    crate::native_return_merge::process_stack(self, &opts, result)
                } else {
                    crate::command::apply_return_options(
                        self,
                        &[Value::string("-options"), opts, result],
                    )
                };
                if let Err(c) = self.deliver_sync(f, c) {
                    return Tick::Return(c);
                }
            }

            // Command dispatch / expr.
            Op::CALL_FUNC1 => {
                use tcl_runtime_api::native_compilation::NativeMathFunctionResolution;

                let Some(binding) = instr.native_fixed_math_call.as_ref() else {
                    return Tick::Return(self.refuse_host_command(
                        "fixed math-function compilation metadata is unavailable".into(),
                    ));
                };
                let retained =
                    asm.native_math_table_prerequisite
                        .as_ref()
                        .is_some_and(|required| {
                            matches!(required.table.lookup_bytes(binding.name.as_bytes()),
                        NativeMathFunctionResolution::Present(actual) if actual == binding)
                        });
                let [Operand::Imm(argc)] = instr.operands.as_slice() else {
                    return Tick::Return(self.refuse_host_command(
                        "fixed math-function argument metadata is unavailable".into(),
                    ));
                };
                let Some(argc) = usize::try_from(*argc).ok().filter(|argc| *argc <= 255) else {
                    return Tick::Return(self.refuse_host_command(
                        "fixed math-function argument metadata is unavailable".into(),
                    ));
                };
                if !retained || binding.arity != Some(argc) {
                    return Tick::Return(self.refuse_host_command(
                        "fixed math-function compilation metadata is inconsistent".into(),
                    ));
                }
                if f.stack.len() < argc {
                    return Tick::Return(err("fixed math-function call: stack underflow"));
                }
                let Some(selected) = f
                    .fixed_math_calls
                    .as_ref()
                    .and_then(|calls| f.pc.checked_sub(1).and_then(|index| calls.get(&index)))
                    .filter(|selected| &selected.binding == binding)
                    .cloned()
                else {
                    return Tick::Return(self.refuse_host_command(
                        "original fixed math-function handler is unavailable".into(),
                    ));
                };
                let arguments = f.stack.split_off(f.stack.len() - argc);
                let completion = self.invoke_native_fixed_math_call(&selected, &arguments);
                if let Err(completion) = self.deliver_sync(f, completion) {
                    return Tick::Return(completion);
                }
            }
            Op::INVOKE_STK1 | Op::INVOKE_STK4 => {
                let argc = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < argc || argc == 0 {
                    return Tick::Return(err("invoke: stack underflow"));
                }
                let words = crate::NativeListItems::invocation_view(Rc::new(
                    f.stack.split_off(f.stack.len() - argc),
                ));
                let entered = f.take_entered_command();
                match self.dispatch_words_with_entered(f, &words, entered.as_ref()) {
                    Ok(Some(call)) => return call,
                    Ok(None) => {}
                    Err(c) => {
                        if c.code == Code::Error {
                            let cmd_text = instr.source_cmd_text.clone();
                            let msg = c.result.string_bytes();
                            let line = instr.source_line;
                            self.log_command_info_with_context(
                                &cmd_text,
                                Value::invocation_list_view(&words),
                                &msg,
                                line,
                            );
                        }
                        return Tick::Return(c);
                    }
                }
            }

            // {*} argument expansion.
            // `EXPAND_START` records the current stack depth: every word pushed
            // after it belongs to the command being built. `EXPAND_STKTOP`
            // expands the top word (a list) in place. `INVOKE_EXPANDED` recovers
            // the post-expansion argument count from the marker and invokes.
            Op::EXPAND_START => {
                f.expand_markers.push(f.stack.len());
            }
            Op::EXPAND_STKTOP => {
                let Some(list_v) = f.stack.pop() else {
                    return Tick::Return(err("expand: stack underflow"));
                };
                let elements = self
                    .actual_native_invocation_dialect()
                    .native_string_protocol()
                    .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original expansion List conversion",
                    ))
                    .and_then(|protocol| self.native_object_list_elements_in(&list_v, protocol));
                match elements {
                    Ok(items) => f.stack.extend(items.iter().cloned()),
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            e.into(),
                        ));
                    }
                }
            }
            // `expandDrop` abandons the innermost expansion: pop its marker and
            // truncate the stack back to the depth `EXPAND_START` recorded (C
            // `INST_EXPAND_DROP` pops `CURR_DEPTH - marker` operands).
            Op::EXPAND_DROP => {
                if let Some(marker) = f.expand_markers.pop()
                    && marker <= f.stack.len()
                {
                    f.stack.truncate(marker);
                }
            }
            Op::INVOKE_EXPANDED => {
                let marker = f.expand_markers.pop().unwrap_or(f.stack.len());
                if f.stack.len() < marker {
                    return Tick::Return(err("invoke: stack underflow"));
                }
                let words =
                    crate::NativeListItems::invocation_view(Rc::new(f.stack.split_off(marker)));
                if words.is_empty() {
                    if let Err(c) = self.deliver_sync(f, ok(Value::empty())) {
                        return Tick::Return(c);
                    }
                } else {
                    let entered = f.take_entered_command();
                    match self.dispatch_words_with_entered(f, &words, entered.as_ref()) {
                        Ok(Some(call)) => return call,
                        Ok(None) => {}
                        Err(c) => {
                            if c.code == Code::Error {
                                let cmd_text = instr.source_cmd_text.clone();
                                let msg = c.result.string_bytes();
                                let line = instr.source_line;
                                self.log_command_info_with_context(
                                    &cmd_text,
                                    Value::invocation_list_view(&words),
                                    &msg,
                                    line,
                                );
                            }
                            return Tick::Return(c);
                        }
                    }
                }
            }
            // Ensemble-rewrite invoke (C Tcl `INST_INVOKE_REPLACE`): operands are
            // `(objc, opnd)` — `objc` words sit on the stack with the resolved
            // implementation word on top. The first `opnd` original words (e.g.
            // `string equal`) are replaced by the popped implementation
            // (`::tcl::string::equal`); the effective command is
            // `impl + words[opnd..]`. The same selected prefix rewrite supplies
            // native wrong-argument presentation without recompiling argv.
            Op::INVOKE_REPLACE => {
                let objc = usize::try_from(imm0(instr)).unwrap_or(0);
                let opnd = usize::try_from(imm_at(instr, 1)).unwrap_or(0);
                let repl = pop(f);
                if f.stack.len() < objc {
                    return Tick::Return(err("invokeReplace: stack underflow"));
                }
                let words = f.stack.split_off(f.stack.len() - objc);
                // The entered outer ensemble protects this specialised range
                // from replay after argument-time mutation, but it is not the
                // command this opcode invokes. Tcl resolves the rewritten
                // implementation word after substitution, so an argument may
                // replace `::tcl::string::equal` for this very invocation.
                let _entered_range = f.take_entered_command();
                let mut rewritten = Vec::with_capacity(objc.saturating_sub(opnd) + 1);
                rewritten.push(repl);
                if opnd < words.len() {
                    rewritten.extend_from_slice(&words[opnd..]);
                }
                let usage = vec![crate::command::NativeArgumentUsageRewrite {
                    original_prefix: words[..opnd.min(words.len())].to_vec(),
                    removed_words: 1,
                }];
                let rewritten = crate::NativeListItems::invocation_view(Rc::new(rewritten));
                match self.dispatch_words_entry_at(
                    f,
                    &rewritten,
                    None,
                    self.current_ns_id(),
                    &usage,
                    false,
                ) {
                    Ok(Some(call)) => return call,
                    Ok(None) => {}
                    Err(c) => {
                        if c.code == Code::Error {
                            let cmd_text = instr.source_cmd_text.clone();
                            let msg = c.result.string_bytes();
                            self.log_command_info_with_context(
                                &cmd_text,
                                Value::invocation_list_view(&rewritten),
                                &msg,
                                instr.source_line,
                            );
                        }
                        return Tick::Return(c);
                    }
                }
            }
            Op::EXPR_STK => {
                let source = pop(f);
                return match self.prepare_expression_value(&source) {
                    Ok(node) => Tick::PushExpression {
                        req: Box::new(ExpressionReq {
                            state: tcl_syntax::expr::ExprEvalState::new(node),
                            awaiting_array: None,
                            normalize: false,
                            jim_objects: source.native_jim_expression_objects(),
                            restore_primary: source.retain_expression_primary(),
                        }),
                        placeholder: Box::new(self.current_placeholder_unit()),
                    },
                    Err(error) => {
                        Tick::Return(crate::command::completion_from_tcl_error(self, error))
                    }
                };
            }

            // Dicts (LVT form, proc bodies)
            // `dict set var k1 ?k2 …? value` — operands [Imm(N), Imm(slot)];
            // stack holds the N keys then the value. Writes the variable and
            // leaves the new dict on the stack (the codegen POPs it).
            Op::DICT_PUT => {
                let value = pop(f);
                let key = pop(f);
                let original = pop(f);
                if !self
                    .actual_native_invocation_dialect()
                    .native_string_protocol()
                    .is_some_and(|protocol| {
                        protocol
                            .tcl_version()
                            .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_1)
                    })
                {
                    return Tick::Return(self.refuse_host_command(
                        "native dictionary-put recipe is unavailable".into(),
                    ));
                }
                let updated = try_core!(crate::cmd_dict::put_original_member(
                    self, &original, &key, value
                ));
                f.stack.push(updated);
            }
            Op::DICT_SET => {
                let nkeys = usize::try_from(imm0(instr)).unwrap_or(0);
                let name = lvt_name(imm_at(instr, 1));
                let value = pop(f);
                if f.stack.len() < nkeys || nkeys == 0 {
                    return Tick::Return(err("dictSet: stack underflow"));
                }
                let keys = f.stack.split_off(f.stack.len() - nkeys);
                let updated = try_op!(crate::cmd_dict::dictionary_path_update_bytes(self,
                    name.as_bytes(), crate::interp::native_dictionary::DictionaryVariablePublication::RetainedCompiledLocal(usize::try_from(imm_at(instr, 1)).unwrap_or(usize::MAX)),
                    &keys, Some(value)));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            // `dict unset var k1 ?k2 …?` — operands [Imm(N), Imm(slot)]; stack
            // holds the N keys.
            Op::DICT_UNSET => {
                let nkeys = usize::try_from(imm0(instr)).unwrap_or(0);
                let name = lvt_name(imm_at(instr, 1));
                if f.stack.len() < nkeys || nkeys == 0 {
                    return Tick::Return(err("dictUnset: stack underflow"));
                }
                let keys = f.stack.split_off(f.stack.len() - nkeys);
                let updated = try_op!(crate::cmd_dict::dictionary_path_update_bytes(self,
                    name.as_bytes(), crate::interp::native_dictionary::DictionaryVariablePublication::RetainedCompiledLocal(usize::try_from(imm_at(instr, 1)).unwrap_or(usize::MAX)),
                    &keys, None));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            // `dict incr var key ?amount?` — operands [Imm(amount), Imm(slot)];
            // stack holds the key.
            Op::DICT_INCR_IMM => {
                let amount = i64::from(imm0(instr));
                let name = lvt_name(imm_at(instr, 1));
                let key = pop(f);
                let increment = Value::int(amount);
                let updated = try_op!(crate::cmd_dict::dictionary_member_update_bytes(self,
                    name.as_bytes(), crate::interp::native_dictionary::DictionaryVariablePublication::RetainedCompiledLocal(usize::try_from(imm_at(instr, 1)).unwrap_or(usize::MAX)),
                    &key, |vm, old| crate::cmd_dict::increment_dictionary_member(vm, old, &increment, true)));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            // `dict append var key value` — operand [Imm(slot)]; stack holds the
            // key then the value.
            Op::DICT_APPEND => {
                let name = lvt_name(imm0(instr));
                let value = pop(f);
                let key = pop(f);
                let updated = try_op!(crate::cmd_dict::dictionary_member_update_bytes(self,
                    name.as_bytes(), crate::interp::native_dictionary::DictionaryVariablePublication::RetainedCompiledLocal(usize::try_from(imm0(instr)).unwrap_or(usize::MAX)),
                    &key, |vm, old| crate::cmd_dict::append_compiled_member_value(vm, old, &value)));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            // `dict lappend var key value` — operand [Imm(slot)]; stack holds the
            // key then the value.
            Op::DICT_LAPPEND => {
                let name = lvt_name(imm0(instr));
                let value = pop(f);
                let key = pop(f);
                let updated = try_op!(crate::cmd_dict::dictionary_member_update_bytes(self,
                    name.as_bytes(), crate::interp::native_dictionary::DictionaryVariablePublication::RetainedCompiledLocal(usize::try_from(imm0(instr)).unwrap_or(usize::MAX)),
                    &key, |vm, old| crate::cmd_dict::lappend_member_values(vm, old, &[value])));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            // `dict get $d k1 ?k2 …?` — operand [Imm(N)]; stack holds the dict
            // then the N keys. Leaves the looked-up value on the stack.
            Op::DICT_GET => {
                let nkeys = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < nkeys + 1 {
                    return Tick::Return(err("dictGet: stack underflow"));
                }
                let keys = f.stack.split_off(f.stack.len() - nkeys);
                let dictionary = pop(f);
                match tcl_cmd_core::dict::get(self, &dictionary, &keys) {
                    Ok(value) => f.stack.push(value),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self, error,
                        ));
                    }
                }
            }
            // `dict getdef $d k1 ?k2 …? default` — operand [Imm(N)]; stack holds
            // the dict, the N keys, then the default (C `INST_DICT_GET_DEF`). A
            // key missing at any depth yields the default; a *malformed* dict
            // still errors (C walks the path with `DICT_PATH_EXISTS`, which
            // distinguishes "absent" from "not a dictionary").
            Op::DICT_GET_DEF => {
                let nkeys = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < nkeys + 2 {
                    return Tick::Return(err("dictGetDef: stack underflow"));
                }
                let default = pop(f);
                let keys = f.stack.split_off(f.stack.len() - nkeys);
                let dictionary = pop(f);
                match tcl_cmd_core::dict::getdef(self, &dictionary, &keys, &default) {
                    Ok(value) => f.stack.push(value),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self, error,
                        ));
                    }
                }
            }
            // `dict exists $d k1 ?k2 …?` — operand [Imm(N)]; stack holds the dict
            // then the N keys. Leaves a boolean on the stack.
            Op::DICT_EXISTS => {
                let nkeys = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < nkeys + 1 {
                    return Tick::Return(err("dictExists: stack underflow"));
                }
                let keys = f.stack.split_off(f.stack.len() - nkeys);
                let dictionary = pop(f);
                match tcl_cmd_core::dict::exists(self, &dictionary, &keys) {
                    Ok(value) => f.stack.push(value),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self, error,
                        ));
                    }
                }
            }

            // Reverse the top `N` stack elements in place (C Tcl `INST_REVERSE`;
            // operand = N). Used to reorder operands an inline emitter pushed in
            // the convenient order.
            Op::REVERSE | Op::SWAP => {
                let n = if instr.op == Op::SWAP {
                    2
                } else {
                    usize::try_from(imm0(instr)).unwrap_or(0)
                };
                let len = f.stack.len();
                if n > len {
                    return Tick::Return(err("reverse: stack underflow"));
                }
                f.stack[len - n..].reverse();
            }
            // `[lindex $list i1 i2 …]` with ≥ 2 indices — C
            // `INST_LIST_INDEX_MULTI` (`tclExecute.c:4833-4858`); operand =
            // list + index count. The list is deepest, then the indices. C
            // hands the whole run to `TclLindexFlat`, where each index is *one*
            // spec (never re-split as an index list), an out-of-range index
            // yields the empty string, and a malformed one returns `NULL` →
            // `goto gotError` (`bad index "foo": …`).
            Op::LINDEX_MULTI => {
                let opnd = usize::try_from(imm0(instr)).unwrap_or(0);
                if opnd == 0 || f.stack.len() < opnd {
                    return Tick::Return(err("lindexMulti: stack underflow"));
                }
                let items = f.stack.split_off(f.stack.len() - opnd);
                let Some((list, idxs)) = items.split_first() else {
                    return Tick::Return(err("lindexMulti: stack underflow"));
                };
                match self.original_list_index(list, idxs, false) {
                    Ok(v) => f.stack.push(v),
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(self, e));
                    }
                }
            }
            // `string replace $s $first $last $new` — C Tcl `INST_STR_REPLACE`.
            // Stack holds the string, the first index, the last index, then the
            // replacement (top). Character-indexed, `end`/`end±N` aware.
            Op::STR_REPLACE => {
                let newstr = pop(f);
                let last = pop(f);
                let first = pop(f);
                let string = pop(f).to_str().to_string();
                let chars: Vec<char> = string.chars().collect();
                let len = chars.len();

                let first_s = first.to_str();
                let last_s = last.to_str();
                let Some(from) = crate::command::resolve_index(self, &first_s, len) else {
                    return Tick::Return(crate::command::bad_index(self, &first_s));
                };
                let Some(to) = crate::command::resolve_index(self, &last_s, len) else {
                    return Tick::Return(crate::command::bad_index(self, &last_s));
                };
                let slen = isize::try_from(len).unwrap_or(isize::MAX) - 1;
                if to < 0 || from > slen || to < from {
                    f.stack.push(Value::string(string));
                } else {
                    let from = usize::try_from(from.max(0)).unwrap_or(0);
                    let to = usize::try_from(to.min(slen)).unwrap_or(0);
                    if from == 0 && usize::try_from(slen).unwrap_or(0) == to {
                        f.stack.push(newstr);
                    } else {
                        let mut out: String = chars[..from].iter().collect();
                        out.push_str(&newstr.to_str());
                        out.extend(&chars[to + 1..]);
                        f.stack.push(Value::string(out));
                    }
                }
            }
            // `lset var ?index? value` (single index, or a `{i j …}` index
            // list) — C Tcl `INST_LSET_LIST`. Stack holds the index list, the
            // new value, then the list (top); leaves the rebuilt list.
            Op::LSET_LIST => {
                let list = pop(f);
                let value = pop(f);
                let index_list = pop(f);
                let release = self.runtime_version();
                match tcl_cmd_core::list::lset(
                    self,
                    &list,
                    std::slice::from_ref(&index_list),
                    value,
                    release,
                ) {
                    Ok(r) => f.stack.push(r),
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(self, e));
                    }
                }
            }
            // `lset var i1 i2 ?…? value` (≥ 2 flat indices) — C Tcl
            // `INST_LSET_FLAT`; operand = index-count + 2. Stack holds the
            // indices (deepest first), the value, then the list (top).
            Op::LSET_FLAT => {
                let num_indices = usize::try_from(imm0(instr) - 2).unwrap_or(0);
                let list = pop(f);
                let value = pop(f);
                if f.stack.len() < num_indices {
                    return Tick::Return(err("lsetFlat: stack underflow"));
                }
                let path = f.stack.split_off(f.stack.len() - num_indices);
                let release = self.runtime_version();
                match tcl_cmd_core::list::lset(self, &list, &path, value, release) {
                    Ok(r) => f.stack.push(r),
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(self, e));
                    }
                }
            }
            // `[regexp $pat $str]` in value position — operand [Imm(cflags)];
            // stack holds the pattern then the string (string on top, matching
            // C Tcl's `INST_REGEXP`: valuePtr = OBJ_AT_TOS, value2Ptr =
            // OBJ_UNDER_TOS). `cflags` carries the regexp compile flags; only the
            // `TCL_REG_NOCASE` bit (010 octal = 8) is meaningful here, since the
            // codegen routes `-nocase` glob-equivalents through `STR_MATCH` and
            // emits `TCL_REG_ADVANCED` (3) for the plain form. Leaves the match
            // boolean on the stack.
            Op::REGEXP if instr.native_switch_version.is_some() => {
                let subject = pop(f);
                let pattern = pop(f);
                let version = instr.native_switch_version.unwrap();
                let mut flags = tcl_cmd_core::regex::RegexFlags::for_release(version);
                flags.nocase = imm0(instr) & 8 != 0;
                let matched = try_core!(
                    tcl_cmd_core::regex::compiled_match_original::<
                        Vm,
                        crate::cmd_regexp::CrateEngine,
                    >(self, &pattern, &subject, flags, version)
                    .map_err(tcl_cmd_core::regex::RegexError::into_cmd_error)
                );
                f.stack.push(Value::int(i64::from(matched)));
            }
            Op::REGEXP => {
                const TCL_REG_NOCASE: i32 = 0o10;
                let nocase = imm0(instr) & TCL_REG_NOCASE != 0;
                let subject = pop(f);
                let pattern = pop(f);
                if self
                    .actual_native_invocation_dialect()
                    .native_jim_regex_protocol()
                    .is_some()
                {
                    match crate::cmd_regexp::invoke_jim_regexp(self, &pattern, &subject, nocase) {
                        Ok(matched) => f.stack.push(Value::bool(matched)),
                        Err(completion) => return Tick::Return(completion),
                    }
                    return Tick::Continue;
                }
                let s = subject.to_str();
                let pat = pattern.to_str();
                let version = self.runtime_version();
                match crate::cmd_regexp::regexp_matches(&pat, &s, nocase, version) {
                    Ok(m) => f.stack.push(Value::bool(m)),
                    Err(detail) => {
                        let prefix = version.regex_compile_error_prefix();
                        return Tick::Return(err(format!("{prefix}{detail}")));
                    }
                }
            }
            // `[string is CLASS $str]` per-character class test — operand
            // [Imm(class-id)]; pops the string, pushes a boolean. Mirrors C Tcl's
            // `INST_STR_CLASS`: the empty string matches, otherwise every
            // character must satisfy the class comparator. Shares the classifier
            // with the `string is` command.
            Op::STR_CLASS => {
                let class_id = u8::try_from(imm0(instr)).unwrap_or(u8::MAX);
                let Some(class) = tcl_bytecode::str_class_name(class_id) else {
                    return Tick::Return(err(format!("strclass: bad class id {class_id}")));
                };
                let s = pop(f).to_str();
                let numbers = self.runtime_version().number_syntax();
                let (member, _fail) =
                    tcl_cmd_core::string_is::class_check(class, &s, false, numbers);
                f.stack.push(Value::bool(member));
            }
            // `numericType` — pops a value, pushes its numeric-tower code (C Tcl
            // `INST_NUM_TYPE` / `GetNumberFromObj`): 0 = not a number,
            // `TCL_NUMBER_INT` = 2, `TCL_NUMBER_BIG` = 3, `TCL_NUMBER_DOUBLE` = 4,
            // `TCL_NUMBER_NAN` = 5. The `string is integer/double` inline forms
            // test this against `<= 3` (integer) or `!= 0` (double/any numeric).
            Op::NUMERIC_TYPE => {
                use tcl_syntax::number::{Number, parse_whole};
                let v = pop(f);
                let code = match parse_whole(v.to_str().trim()) {
                    Some(Number::Int(_)) => 2,
                    Some(Number::Big { .. }) => 3,
                    Some(Number::Double(_)) => 4,
                    Some(Number::Nan { .. }) => 5,
                    None => 0,
                };
                f.stack.push(Value::int(code));
            }

            Op::TAILCALL | Op::TAILCALL4 => {
                if !self.frame_owns_local_variables(self.current_level()) {
                    return Tick::Return(crate::command::err_with_code(
                        "tailcall can only be called from a proc, lambda or method",
                        "TCL TAILCALL ILLEGAL",
                    ));
                }
                let count = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < count || count == 0 {
                    return Tick::Return(err("tailcall: stack underflow"));
                }
                let mut words = f.stack.split_off(f.stack.len() - count);
                if instr.op == Op::TAILCALL {
                    words[0] = match tcl_cmd_core::namespace::current_original(self) {
                        Ok(value) => value,
                        Err(error) => {
                            return Tick::Return(crate::command::completion_from_cmd_error(
                                self, error,
                            ));
                        }
                    };
                }
                let Some(protocol) = self
                    .actual_native_invocation_dialect()
                    .native_string_protocol()
                else {
                    return Tick::Return(
                        self.refuse_host_command("native tailcall List issuer".into()),
                    );
                };
                let list = Value::native_list_constructor(words, protocol);
                return self.native_tailcall_tick(list);
            }
            Op::TAILCALL_LIST => {
                let list = pop(f);
                return self.native_tailcall_tick(list);
            }

            // Termination.
            Op::DONE => return self.finish_frame_result(f),

            // The compiled catch epilogue's reads (C `INST_PUSH_RESULT`/
            // `INST_PUSH_RETURN_CODE`/`INST_PUSH_RETURN_OPTS`): after a range
            // absorbed a completion they report *it*; on the fall-through (no
            // catch fired) path they report the current result / an ok options
            // dict, as C's untouched interp state would.
            Op::PUSH_RESULT => {
                let v = match Self::innermost_caught(f) {
                    Some(caught) => caught.result.clone(),
                    None => try_core!(self.frame_result_transport(f)),
                };
                f.stack.push(v.into_native_reference());
            }
            Op::PUSH_RETURN_CODE => {
                let code = Self::innermost_caught(f).map_or(0, |c| c.code.as_int());
                f.stack.push(Value::int(code));
            }
            Op::PUSH_RETURN_OPTS => {
                let result = try_core!(self.frame_result_transport(f));
                let opts = match Self::innermost_caught(f) {
                    Some(caught) => caught.options.clone(),
                    None => match crate::command::completion_options(
                        self,
                        &Completion::new(Code::Ok, result, f.last_options.clone()),
                    ) {
                        Ok(options) => options,
                        Err(error) => {
                            return Tick::Return(crate::command::completion_from_tcl_error(
                                self, error,
                            ));
                        }
                    },
                };
                f.stack.push(opts);
            }
            // Pop a (non-ok) return code and branch into the 5-slot `jump1`
            // table the compiler lays out right after this instruction: target
            // byte = `2*code − 1` past the opcode, error (1) first; anything
            // outside `error..continue` lands on the fifth slot (C
            // `INST_RETURN_CODE_BRANCH`, which panics on TCL_OK / non-integers
            // — the VM errors instead of aborting).
            Op::RETURN_CODE_BRANCH => {
                let Ok(code) = pop(f).as_int() else {
                    return Tick::Return(err("returnCodeBranch: TOS not a return code"));
                };
                if code == 0 {
                    return Tick::Return(err("returnCodeBranch: TOS is TCL_OK"));
                }
                let code = if (1..=4).contains(&code) { code } else { 5 };
                let target_off = instr.offset + 2 * i32::try_from(code).unwrap_or(5) - 1;
                match f.off2idx.get(&target_off) {
                    Some(&idx) => f.pc = idx,
                    None => {
                        return Tick::Return(err("returnCodeBranch: no jump table"));
                    }
                }
            }
            // Evaluate a popped script string and push its result — value-position
            // multi-command substitution `[a; b]`. A non-OK
            // completion unwinds like any other command error.
            Op::UPLEVEL => {
                let script = pop(f);
                let level = pop(f);
                let completion = crate::command::compiled_uplevel(self, &level, script);
                match self.settle_native_dispatch(f, completion) {
                    Ok(Some(tick)) => return tick,
                    Ok(None) => {}
                    Err(completion) => return Tick::Return(completion),
                }
            }
            Op::EVAL_STK => {
                // Run the script on the *explicit* stack (a transparent script
                // frame) rather than a nested drive, so a `yield` inside it stays
                // yieldable. Its result/`break`/`continue`/error
                // is delivered to this frame by `unwind` exactly as the old inline
                // push/`Tick::Return` did.
                let original = pop(f);
                let source = match self.native_script_source_image(&original) {
                    Ok(source) => source,
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_tcl_error(
                            self, error,
                        ));
                    }
                };
                match self.compile_script_cached_bytes(&source) {
                    Ok(script) => {
                        return Tick::PushScript {
                            script: Box::new(script),
                            label: None,
                            cleanup_proc: None,
                            fatal_tail: None,
                            namespace: ScriptNamespace::Inherit,
                        };
                    }
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_tcl_error(self, e));
                    }
                }
            }

            // Introspection (C Tcl's "general introspector" instructions)
            // Each routes through the same core its command form uses, so the
            // compiled and dispatched paths cannot drift.
            Op::CURRENT_NAMESPACE => match tcl_cmd_core::namespace::current_original(self) {
                Ok(value) => f.stack.push(value),
                Err(error) => {
                    return Tick::Return(crate::command::completion_from_cmd_error(self, error));
                }
            },
            Op::INFO_LEVEL_NUM => match self.native_info_level(None) {
                Ok(v) => f.stack.push(v),
                Err(e) => return Tick::Return(crate::command::completion_from_cmd_error(self, e)),
            },
            Op::INFO_LEVEL_ARGS => {
                let n = pop(f);
                match self.native_info_level(Some(&n)) {
                    Ok(v) => f.stack.push(v),
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(self, e));
                    }
                }
            }
            // Both instructions reach the same original-object getter and name
            // reporting owner as their namespace command forms.
            Op::RESOLVE_CMD => {
                let original = pop(f);
                match self.native_namespace_command_name(&original, false) {
                    Ok(bytes) => f
                        .stack
                        .push(bytes.map_or_else(Value::empty, Value::from_native_string_bytes)),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            error.into(),
                        ));
                    }
                }
            }
            Op::ORIGIN_CMD => {
                let original = pop(f);
                match self.native_namespace_command_name(&original, true) {
                    Ok(Some(bytes)) => match self.native_namespace_origin_result(&bytes) {
                        Ok(result) => f.stack.push(result),
                        Err(error) => {
                            return Tick::Return(crate::command::completion_from_cmd_error(
                                self,
                                error.into(),
                            ));
                        }
                    },
                    Ok(None) => {
                        return Tick::Return(self.native_namespace_origin_failure(&original));
                    }
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            error.into(),
                        ));
                    }
                }
            }
            // `clockRead <which>` reads the same host clock `clock clicks` /
            // `clock seconds` do (`cmd_clock`), so the opcode and the command
            // agree. C: 0 = clicks (wide clicks — microseconds here, as
            // `clock clicks` returns), 1 = µs, 2 = ms, 3 = s.
            Op::CLOCK_READ => {
                let clock = self.host_rc();
                let clock = clock.clock();
                let wide = match imm0(instr) {
                    2 => i64::try_from(clock.now_millis()).unwrap_or(i64::MAX),
                    3 => clock.now_secs(),
                    // 0 (clicks) and 1 (microseconds) read the same counter.
                    _ => i64::try_from(clock.now_micros()).unwrap_or(i64::MAX),
                };
                f.stack.push(Value::int(wide));
            }

            // -- coroutines (C `INST_YIELD` / `INST_YIELD_TO_INVOKE` /
            // `INST_COROUTINE_NAME`) --
            // The compiled `yield`/`yieldto` record their suspend request through
            // the very core the builtins use (`cmd_coro::request_*`, boundary
            // checks included) and then drain it into the `Tick::Suspend` the
            // builtin path gets from `dispatch_words`. C pops the yielded value
            // and pushes the resume value in its place (`pc++; cleanup = 1;
            // TEBC_YIELD()`), which is exactly what popping here plus `resume`'s
            // delivered-value push does.
            Op::YIELD => {
                let value = pop(f);
                if let Err(c) = crate::cmd_coro::request_yield(self, value) {
                    return Tick::Return(c);
                }
                if let Some(req) = self.coro.pending.take() {
                    return Tick::Suspend(req);
                }
            }
            // TOS is the captured namespace followed by relay command argv.
            // The namespace was evaluated before the original operands.
            Op::YIELD_TO_INVOKE => {
                let original = pop(f);
                if let Err(c) = crate::cmd_coro::request_yieldto_original(self, original) {
                    return Tick::Return(c);
                }
                if let Some(req) = self.coro.pending.take() {
                    return Tick::Suspend(req);
                }
            }
            Op::CORO_NAME => {
                f.stack.push(crate::cmd_coro::current_coroutine(self));
            }

            // TclOO (C's "start of TclOO support instructions" block)
            // Each routes through the core its command form uses: `self object`,
            // `info object class`/`namespace`/`isa object`, `next`, `nextto`. The
            // context checks are the opcodes' own, since C's compiled forms report
            // "X may only be called from inside a method" where the command forms
            // report an unresolvable command name.
            Op::TCLOO_SELF => match crate::cmd_oo::current_object_name(self) {
                Some(name) => f.stack.push(name),
                None => {
                    return Tick::Return(crate::cmd_oo::native_context::helper_context_error(
                        self, b"self",
                    ));
                }
            },
            Op::TCLOO_IS_OBJECT => {
                let name = pop(f);
                match crate::cmd_oo::is_object(self, &name) {
                    Ok(present) => match self.native_oo_predicate_result(present) {
                        Ok(value) => f.stack.push(value),
                        Err(error) => {
                            return Tick::Return(self.refuse_host_command(error.to_string()));
                        }
                    },
                    Err(completion) => return Tick::Return(completion),
                }
            }
            Op::TCLOO_CLASS => {
                let name = pop(f);
                match crate::cmd_oo::object_key(self, &name) {
                    Ok(key) => {
                        let v = crate::cmd_oo::object_class_name(self, key);
                        f.stack.push(v);
                    }
                    Err(c) => return Tick::Return(c),
                }
            }
            Op::TCLOO_NS => {
                let name = pop(f);
                match crate::cmd_oo::object_key(self, &name) {
                    Ok(key) => match crate::cmd_oo::object_namespace_name(self, key) {
                        Ok(value) => f.stack.push(value),
                        Err(completion) => return Tick::Return(completion),
                    },
                    Err(c) => return Tick::Return(c),
                }
            }
            Op::TCLOO_ID => {
                let dialect = self.actual_native_invocation_dialect();
                if dialect.family() != Some(tcl_dialect::model::Family::Tcl)
                    || dialect.tcl_version != Some(tcl_dialect::TclVersion::V9_1)
                {
                    return Tick::Return(
                        self.refuse_host_command("native TclOO creation-id instruction".to_owned()),
                    );
                }
                let name = pop(f);
                match crate::cmd_oo::object_key(self, &name)
                    .and_then(|key| crate::cmd_oo::object_creation_id(self, key))
                {
                    Ok(value) => f.stack.push(value),
                    Err(completion) => return Tick::Return(completion),
                }
            }
            Op::TCLOO_NEXT | Op::TCLOO_NEXT4 => try_op!(self.tcloo_next(f, instr, false)),
            Op::TCLOO_NEXT_CLASS | Op::TCLOO_NEXT_CLASS4 => {
                try_op!(self.tcloo_next(f, instr, true));
            }
            Op::TCLOO_NEXT_LIST => try_op!(self.tcloo_next_list(f, false)),
            Op::TCLOO_NEXT_CLASS_LIST => try_op!(self.tcloo_next_list(f, true)),
        }

        Tick::Continue
    }

    /// The `tclooNext` / `tclooNextClass` opcodes (`nextto` when `nextto` is set).
    ///
    /// The operand counts *all* the words C pushed, the first being the
    /// `next`/`nextto` command word itself (C's `skip`), so the arguments are
    /// `words[1..]` — which for `nextto` still starts with the class, exactly what
    /// its core expects. The method-context check is the opcode's own: C's
    /// compiled forms report "`next` may only be called from inside a method"
    /// where the command forms report an unresolvable command name.
    fn tcloo_next(
        &mut self,
        f: &mut Frame,
        instr: &Instruction,
        nextto: bool,
    ) -> Result<(), Completion<Value>> {
        let (mnemonic, verb) = if nextto {
            ("tclooNextClass", "nextto")
        } else {
            ("tclooNext", "next")
        };
        let words = take_words(f, instr, mnemonic)?;
        let bytes = if self.actual_native_invocation_dialect().tcl_version
            == Some(tcl_dialect::TclVersion::V9_1)
        {
            let bytes = self
                .native_name_operand_bytes(&words[0])
                .map_err(|error| self.refuse_host_command(error.to_string()))?;
            tcl_core_types::c_string_extent(&bytes).to_vec()
        } else {
            verb.as_bytes().to_vec()
        };
        if !crate::cmd_oo::in_method(self) {
            return Err(crate::cmd_oo::native_context::helper_context_error(
                self, &bytes,
            ));
        }
        let args = &words[1..];
        let res = if nextto {
            crate::cmd_oo::cmd_nextto_original(self, &words[0], args)
        } else {
            crate::cmd_oo::cmd_next_original(self, &words[0], args)
        };
        // `deliver_sync` pushes the result on `OK` and hands the completion back
        // otherwise; the chained method runs on the native stack, so there is
        // never a `Tick` to propagate from here.
        self.deliver_sync(f, res).map(|_| ())
    }

    fn tcloo_next_list(&mut self, f: &mut Frame, nextto: bool) -> Result<(), Completion<Value>> {
        let original = pop(f);
        let protocol = self
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .ok_or_else(|| {
                self.refuse_host_command("native helper List protocol unavailable".to_owned())
            })?;
        let members = self
            .native_object_list_elements_in(&original, protocol)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
        let words = members.as_ref();
        if words.len() < if nextto { 2 } else { 1 } {
            return Err(self.refuse_host_command(
                "native TclOO invocation List has insufficient words".into(),
            ));
        }
        // C9.1 obtains the original head after GetElements, before checking the frame.
        let bytes = self
            .native_name_operand_bytes(&words[0])
            .map_err(|error| self.refuse_host_command(error.to_string()))?
            .to_vec();
        if !crate::cmd_oo::in_method(self) {
            return Err(crate::cmd_oo::native_context::helper_context_error(
                self,
                tcl_core_types::c_string_extent(&bytes),
            ));
        }
        let result = if nextto {
            crate::cmd_oo::cmd_nextto_original(self, &words[0], &words[1..])
        } else {
            crate::cmd_oo::cmd_next_original(self, &words[0], &words[1..])
        };
        self.deliver_sync(f, result).map(|_| ())
    }

    /// Dispatch a fully-assembled command word list. Returns `Some(Tick::Call)`
    /// for a proc (the trampoline performs the activation), `None` when a builtin
    /// ran and pushed its result, or `Err` to unwind on a non-`OK` completion or
    /// an unknown command.
    fn dispatch_words(
        &mut self,
        f: &mut Frame,
        words: &crate::NativeListItems,
    ) -> Result<Option<Tick>, Completion<Value>> {
        self.dispatch_words_with_entered(f, words, None)
    }

    /// Dispatch with an optional command token captured at the command-head
    /// push. Arguments are the already-substituted values; the captured token
    /// changes only command selection and never replays them.
    fn dispatch_words_with_entered(
        &mut self,
        f: &mut Frame,
        words: &crate::NativeListItems,
        entered: Option<&EnteredCommand>,
    ) -> Result<Option<Tick>, Completion<Value>> {
        self.dispatch_words_selected(f, words, entered, None, &[])
    }

    fn dispatch_words_selected(
        &mut self,
        f: &mut Frame,
        words: &crate::NativeListItems,
        entered: Option<&EnteredCommand>,
        lookup_namespace: Option<&str>,
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Result<Option<Tick>, Completion<Value>> {
        let context = self
            .native_command_lookup_context(lookup_namespace)
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        self.dispatch_words_selected_at(f, words, entered, context, usage)
    }

    fn dispatch_words_selected_at(
        &mut self,
        f: &mut Frame,
        words: &crate::NativeListItems,
        entered: Option<&EnteredCommand>,
        context: tcl_core_types::NsId,
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Result<Option<Tick>, Completion<Value>> {
        self.dispatch_words_entry_at(f, words, entered, context, usage, false)
    }

    fn dispatch_words_entry_at(
        &mut self,
        f: &mut Frame,
        words: &crate::NativeListItems,
        entered: Option<&EnteredCommand>,
        context: tcl_core_types::NsId,
        usage: &[crate::command::NativeArgumentUsageRewrite],
        ordinary: bool,
    ) -> Result<Option<Tick>, Completion<Value>> {
        use tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent as Event;
        if ordinary {
            self.reset_native_ensemble_rewrite(Event::BeforeOrdinaryLookup)?;
        }
        let selected = entered.is_some()
            || match words.first() {
                Some(head) => self
                    .resolve_original_command_key_at(context, head)
                    .map_err(|error| self.refuse_host_command(error.to_string()))?
                    .is_some(),
                None => false,
            };
        if ordinary && selected {
            self.reset_native_ensemble_rewrite(Event::AfterSuccessfulOrdinaryLookup)?;
        }
        if let Some(exceeded) = self.charge_command() {
            return Err(exceeded);
        }
        if !self.exec_traces_can_fire() {
            return self.dispatch_words_inner(f, words, None, entered, context, usage);
        }
        self.dispatch_words_traced(f, words, entered, context, usage)
    }

    /// Resolve the trace owner and take the own-trace snapshot for one invoke.
    /// A retained specialised entry had no execution trace at command entry, so
    /// traces added by its arguments are deliberately absent from the snapshot.
    fn dispatch_trace_owner(
        &mut self,
        original: &Value,
        entered: Option<&EnteredCommand>,
        context: tcl_core_types::NsId,
    ) -> Result<(CommandSidecarHandle, Vec<Rc<CmdTraceEntry>>), Completion<Value>> {
        let sidecar = if let Some(entered) = entered {
            entered.sidecar.clone()
        } else {
            let key = self
                .resolve_original_command_key_at(context, original)
                .map_err(|error| self.refuse_host_command(error.to_string()))?
                .unwrap_or_default();
            let key = self.renamed_command_key(CommandSidecarKey::visible(key));
            self.active_sidecar(key)
        };
        let own = if entered.is_some_and(|entered| {
            matches!(
                entered.execution_traces,
                SelectedNativeExecutionTraces::Omitted
            )
        }) {
            Vec::new()
        } else {
            sidecar
                .key()
                .and_then(|key| self.exec_traces.get(&key).cloned())
                .unwrap_or_default()
        };
        Ok((sidecar, own))
    }

    /// Validate the command owner and snapshot the step-capable traces which
    /// own its body immediately after all `enter` callbacks have completed and
    /// immediately before dispatch.
    ///
    /// C Tcl lets an `enter` callback add a `leave`/step trace for this same
    /// invocation. Step scopes are fixed at this point; leave traces are looked
    /// up live at settlement, but only when this validated owner already had an
    /// execution trace at entry. Generic dispatch also re-resolves the invoked
    /// spelling after the callbacks: if a callback renamed, deleted, or
    /// replaced the entry-time owner, its traces do not own the different
    /// command selected by that re-resolution. An already-resolved invocation
    /// passes no name and continues to own its captured command through
    /// relocation.
    fn post_enter_trace_snapshot(
        &self,
        generic_name: Option<&str>,
        lookup_namespace: Option<&str>,
        sidecar: &CommandSidecarHandle,
        selected_traces: Option<SelectedNativeExecutionTraces>,
    ) -> Option<Vec<Rc<CmdTraceEntry>>> {
        // A specialised entered token had no execution trace before argument
        // substitution. A trace added by an argument belongs only to a later
        // invocation, even though this helper runs immediately before dispatch.
        if matches!(
            selected_traces,
            Some(SelectedNativeExecutionTraces::Omitted)
        ) {
            return None;
        }
        let owner = sidecar.key()?;
        if let Some(name) = generic_name {
            let resolved = self
                .resolve_command_fqn(lookup_namespace.unwrap_or(self.current_ns()), name)
                .map(CommandSidecarKey::visible)
                .map(|key| self.renamed_command_key(key));
            if resolved.as_ref() != Some(&owner) {
                return None;
            }
        }
        Some(self.exec_traces.get(&owner).cloned().unwrap_or_default())
    }

    /// Install the step scopes from the post-enter body-owner snapshot.
    fn push_exec_step_scopes(
        &mut self,
        sidecar: &CommandSidecarHandle,
        own: &[Rc<CmdTraceEntry>],
    ) -> usize {
        let was_disabled = self.step_trace_active();
        let mut pushed = 0usize;
        for entry in own.iter().rev() {
            if !sidecar.is_attached() {
                break;
            }
            if !self.exec_trace_entry_live(sidecar, entry) {
                continue;
            }
            if (entry.has_op("enterstep") || entry.has_op("leavestep"))
                && !self
                    .exec_step_scopes
                    .iter()
                    .any(|scope| Rc::ptr_eq(&scope.entry, entry) && !scope.entry.untraced())
            {
                self.exec_step_scopes.push(ExecStepScope {
                    entry: Rc::clone(entry),
                    key: sidecar.clone(),
                });
                pushed += 1;
            }
        }
        self.publish_native_inline_trace_transition(was_disabled);
        pushed
    }

    fn pop_exec_step_scopes(&mut self, count: usize) {
        let was_disabled = self.step_trace_active();
        for _ in 0..count {
            self.exec_step_scopes.pop();
        }
        self.publish_native_inline_trace_transition(was_disabled);
    }

    /// Whether the `enterstep`/`leavestep` machinery may fire right now. This
    /// is C's one read of `INTERP_TRACE_IN_PROGRESS` — `TclCheckInterpTraces`
    /// (`tclTrace.c` 9.0.4:1426) returns immediately while an execution
    /// callback is running, so the callback's own commands are never
    /// step-observed. Scopes are still *pushed* while one runs, as C still
    /// installs its interp trace; only the firing is gated.
    fn step_scopes_can_fire(&self) -> bool {
        !self.exec_step_scopes.is_empty() && !self.trace_in_progress.get()
    }

    /// The step scopes to fire for this command — empty while an execution
    /// callback is running (see [`Self::step_scopes_can_fire`]).
    fn step_scopes_to_fire(&self) -> Vec<ExecStepScope> {
        if self.step_scopes_can_fire() {
            self.exec_step_scopes.clone()
        } else {
            Vec::new()
        }
    }

    /// Whether this dispatch needs the traced path at all. Being inside a trace
    /// callback is *not* a reason to skip it: C's `TclCheckExecutionTraces`
    /// (:1301) never consults the flag, so a command a callback dispatches
    /// still fires its own `enter`/`leave` traces — only the step machinery is
    /// gated. Re-entering the *same* trace is bounded per trace instead, by
    /// `CmdTraceEntry::firing` in [`Vm::run_cmd_trace_callback`].
    fn exec_traces_can_fire(&self) -> bool {
        !self.exec_traces.is_empty() || self.step_scopes_can_fire()
    }

    /// The traced dispatch path (M16.3): fire `enterstep` for every active
    /// step scope and `enter` for the command's own execution traces (a trace
    /// error aborts the command with that error — tclsh-pinned), push the
    /// command's own step scopes, dispatch, then settle the leave side — for
    /// synchronous completions here, for deferred bodies when their frame
    /// unwinds (the context rides on the frame via `pending_exec_leave`).
    fn settle_traced_word_dispatch(
        &mut self,
        f: &mut Frame,
        mut ctx: ExecLeaveCtx,
        dispatched: Result<Option<Tick>, Completion<Value>>,
    ) -> Result<Option<Tick>, Completion<Value>> {
        match dispatched {
            Ok(Some(tick)) => {
                match &tick {
                    // The body runs on a pushed frame: the context rides
                    // along and settles when that frame completes.
                    Tick::Call { .. }
                    | Tick::PushScript { .. }
                    | Tick::PushCatch(_)
                    | Tick::PushEval(_)
                    | Tick::PushSubst { .. }
                    | Tick::PushEachLoop { .. }
                    | Tick::PushControl { .. }
                    | Tick::PushExpression { .. }
                    | Tick::PushTry { .. } => {
                        if let Some(mut prior) = self.pending_exec_leave.take() {
                            ctx.jim_commands.append(&mut prior.jim_commands);
                        }
                        self.pending_exec_leave = Some(ctx);
                    }
                    // Control shapes with no owning frame (a traced `yield` /
                    // `tailcall` builtin itself): settle with an empty ok —
                    // their real result forms elsewhere.
                    Tick::Continue | Tick::Return(_) | Tick::Tailcall(_) | Tick::Suspend(_) => {
                        let _ = self.finish_exec_leave(&ctx, &ok(Value::empty()));
                    }
                }
                Ok(Some(tick))
            }
            Ok(None) => {
                // Synchronous completion: the result is on top of `f`'s stack.
                let result = f.stack.last().cloned().unwrap_or_else(Value::empty);
                match self.finish_exec_leave(&ctx, &ok(result)) {
                    None => Ok(None),
                    Some(replacement) => {
                        // A leave-trace error replaces the command's result
                        // (tclsh-pinned: LEAVEFAIL).
                        f.stack.pop();
                        Err(replacement)
                    }
                }
            }
            Err(c) => match self.finish_exec_leave(&ctx, &c) {
                None => Err(c),
                Some(replacement) => Err(replacement),
            },
        }
    }

    fn dispatch_words_traced(
        &mut self,
        f: &mut Frame,
        words: &crate::NativeListItems,
        entered: Option<&EnteredCommand>,
        context: tcl_core_types::NsId,
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Result<Option<Tick>, Completion<Value>> {
        let usage_revision = self.native_invocation.usage_revision;
        let original = &words[0];
        // The complete current command, list-merged (tclsh-pinned shape:
        // `p one {t w o}`).
        let cmd_string = Value::list(words.to_vec());
        for scope in self.step_scopes_to_fire() {
            if scope.active_for("enterstep") && self.exec_trace_entry_live(&scope.key, &scope.entry)
            {
                let r = self.run_cmd_trace_callback(
                    &scope.entry,
                    &[cmd_string.clone(), Value::string("enterstep")],
                );
                if !r.code.is_ok() {
                    return Err(r);
                }
            }
        }
        // A retained entry represents a command that was specialised while it
        // had no execution trace. A trace installed by one of its argument
        // substitutions applies only to later invocations in C Tcl; consulting
        // the live table here would make it fire retroactively. When a trace
        // already existed at command entry no token was retained, so ordinary
        // live lookup remains correct for that generic invocation. The owner
        // resolver also follows an open rename window to the command's moved
        // trace list while leaving `cmd_string` in the caller's spelling.
        let (sidecar, own_at_entry) = self.dispatch_trace_owner(original, entered, context)?;
        for entry in own_at_entry.iter().rev() {
            if !sidecar.is_attached() {
                break;
            }
            if !self.exec_trace_entry_live(&sidecar, entry) {
                continue;
            }
            if entry.has_op("enter") {
                let r = self
                    .run_cmd_trace_callback(entry, &[cmd_string.clone(), Value::string("enter")]);
                if !r.code.is_ok() {
                    return Err(r);
                }
            }
        }
        let post_enter = if entered.is_some_and(|entered| {
            matches!(
                entered.execution_traces,
                SelectedNativeExecutionTraces::Omitted
            )
        }) {
            None
        } else {
            let selected = self
                .resolve_original_command_key_at(context, original)
                .map_err(|error| self.refuse_host_command(error.to_string()))?
                .map(CommandSidecarKey::visible)
                .map(|key| self.renamed_command_key(key));
            let attached = sidecar.key();
            if entered.is_some() || selected.as_ref() == attached.as_ref() {
                attached.map(|key| self.exec_traces.get(&key).cloned().unwrap_or_default())
            } else {
                None
            }
        };
        let leave_owner =
            (!own_at_entry.is_empty() && post_enter.is_some()).then(|| sidecar.clone());
        let pushed =
            self.push_exec_step_scopes(&sidecar, post_enter.as_deref().unwrap_or_default());
        let ctx = ExecLeaveCtx {
            jim_commands: Vec::new(),
            cmd_string,
            leave_owner,
            step_scopes: pushed,
        };
        let usage = if usage_revision == self.native_invocation.usage_revision {
            usage
        } else {
            &[]
        };
        let dispatched =
            self.dispatch_words_inner(f, words, Some(&sidecar), entered, context, usage);
        self.settle_traced_word_dispatch(f, ctx, dispatched)
    }

    /// Settle the leave side of one traced dispatch: pop the step scopes it
    /// pushed, fire its own `leave` traces, then the still-active outer
    /// scopes' `leavestep` — each as `callback cmd-string code result op`
    /// (tclsh-pinned shapes).  Returns `Some(error)` when a trace errored —
    /// the error REPLACES the command's completion (tclsh-pinned) — else
    /// `None`.
    pub(crate) fn finish_exec_leave(
        &mut self,
        ctx: &ExecLeaveCtx,
        c: &Completion<Value>,
    ) -> Option<Completion<Value>> {
        debug_assert!(
            ctx.jim_commands
                .iter()
                .all(super::interp::native_jim_lookup::JimCommandLease::is_active)
        );
        self.pop_exec_step_scopes(ctx.step_scopes);
        if let Some(refused) = self.refused_completion() {
            return Some(refused);
        }
        if self.exec_traces.is_empty() && self.exec_step_scopes.is_empty() {
            return None;
        }
        let args = |result: Value, op: &str| {
            [
                ctx.cmd_string.clone(),
                Value::string(c.code.as_int().to_string()),
                result,
                Value::string(op),
            ]
        };
        let mut replacement = None;
        // Tcl threads a successful leave callback's result to the next
        // callback, while the traced command's own completion remains `c`.
        // The list is intentionally live: when this owner already had a trace
        // at entry, a leave trace added by an enter callback or by the command
        // body joins the current invocation. Removal, deletion and relocation
        // are observed through the sidecar at settlement.
        let mut trace_result = c.result.clone();
        if let Some(owner) = &ctx.leave_owner
            && let Some(key) = owner.key()
            && let Some(entries) = self.exec_traces.get(&key).cloned()
        {
            for entry in entries {
                if !owner.is_attached() {
                    break;
                }
                if !self.exec_trace_entry_live(owner, &entry) {
                    continue;
                }
                if entry.has_op("leave") {
                    let r =
                        self.run_cmd_trace_callback(&entry, &args(trace_result.clone(), "leave"));
                    if !r.code.is_ok() {
                        replacement = Some(r);
                        break;
                    }
                    trace_result = r.result;
                }
            }
        }
        let mut leaving = self.step_scopes_to_fire();
        leaving.reverse();
        for scope in leaving {
            if scope.active_for("leavestep") && self.exec_trace_entry_live(&scope.key, &scope.entry)
            {
                let r = self
                    .run_cmd_trace_callback(&scope.entry, &args(trace_result.clone(), "leavestep"));
                if !r.code.is_ok() {
                    replacement = Some(r);
                    break;
                }
                trace_result = r.result;
            }
        }
        replacement
    }

    /// A cloned trace snapshot may outlive an earlier callback removing its
    /// registration.  Rc pointer identity distinguishes duplicate callbacks;
    /// a later addition is absent from the snapshot and therefore excluded.
    fn exec_trace_entry_live(
        &self,
        handle: &CommandSidecarHandle,
        entry: &Rc<CmdTraceEntry>,
    ) -> bool {
        handle.key().is_some_and(|key| {
            self.exec_traces
                .get(&key)
                .is_some_and(|entries| entries.iter().any(|live| Rc::ptr_eq(live, entry)))
        })
    }

    /// Deliver a synchronously-completed dispatch into `f`: an ok result is
    /// pushed onto the operand stack, a non-ok completion unwinds.
    fn deliver_sync(
        &mut self,
        f: &mut Frame,
        res: Completion<Value>,
    ) -> Result<Option<Tick>, Completion<Value>> {
        if res.code.is_ok() {
            // Jim's evaluation frame borrows argv only while this command is
            // active. Completed diagnostic views must not keep arguments shared.
            f.jim_evaluation.invocation = Value::empty();
            #[cfg(test)]
            if std::env::var_os("TCL_LSP_TRACE_NATIVE_OPTIONS").is_some() {
                res.options
                    .report_native_compound_ownership("deliver-sync-options");
            }
            let protocol = self
                .actual_native_invocation_dialect()
                .native_string_protocol();
            let present = res
                .options
                .native_return_options_nonempty(protocol)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
            if present {
                f.last_options = res.options;
            }
            f.stack.push(res.result.into_native_reference());
            Ok(None)
        } else {
            Err(res)
        }
    }

    /// Build the generation-bearing empty unit used by scanner activations at
    /// the moment their native command hands control to the trampoline.
    fn current_placeholder_unit(&mut self) -> crate::compiled::CompiledUnit {
        self.compiled_unit(
            Rc::new(FunctionAsm::default()),
            self.source_namespace_path(),
        )
    }

    /// Settle a synchronously-run native command (a [`BuiltinFn`] or an
    /// embedder's [`NativeCommand`](crate::command::NativeCommand)): drain any
    /// body it deferred to the explicit stack, else deliver its completion.
    fn settle_native_dispatch(
        &mut self,
        f: &mut Frame,
        res: Completion<Value>,
    ) -> Result<Option<Tick>, Completion<Value>> {
        if let Some(refused) = self.refused_completion() {
            return Err(refused);
        }
        // A `yield`/`yieldto` sets `coro.pending` (after its own boundary
        // check); convert it into a suspend `Tick` that freezes the whole
        // activation stack. The builtin's placeholder result is dropped —
        // the resume value replaces it on the operand stack.
        if let Some(req) = self.coro.pending.take() {
            return Ok(Some(Tick::Suspend(req)));
        }
        if let Some(req) = self.pending.procedure_call.take() {
            return Ok(Some(Tick::Call {
                proc: Box::new(req.proc),
                invoked: req.invoked,
                argv: req.arguments,
                lambda_registration: Some(req.registration),
            }));
        }
        // An `eval`/`uplevel`/`apply`-style builtin defers its body to the
        // explicit stack (yieldable): drain it into a `PushScript`, whose
        // frame result replaces this builtin's placeholder (as for yield).
        if let Some(req) = self.pending.eval.take() {
            return Ok(Some(Tick::PushEval(Box::new(req))));
        }
        // A `catch` defers its body the same way, but into a catch frame
        // whose completion the epilogue absorbs (see `Frame::catch`).
        if let Some(req) = self.pending.catch.take() {
            return Ok(Some(Tick::PushCatch(Box::new(req))));
        }
        // A `subst` defers to a scanner-driven subst frame, whose `[…]`
        // bodies run yieldably as child frames (see `Frame::subst`).
        if let Some(req) = self.pending.subst.take() {
            return Ok(Some(Tick::PushSubst {
                req: Box::new(req),
                placeholder: Box::new(self.current_placeholder_unit()),
            }));
        }
        // A `foreach`/`lmap` runtime-fallback loop defers to a
        // scanner-driven each-loop frame, whose iterations run yieldably
        // as child frames (see `Frame::each_loop`).
        if let Some(req) = self.pending.each_loop.take() {
            return Ok(Some(Tick::PushEachLoop {
                req: Box::new(req),
                placeholder: Box::new(self.current_placeholder_unit()),
            }));
        }
        if let Some(req) = self.pending.expression.take() {
            return Ok(Some(Tick::PushExpression {
                req: Box::new(req),
                placeholder: Box::new(self.current_placeholder_unit()),
            }));
        }
        if let Some(state) = self.pending.control.take() {
            return Ok(Some(Tick::PushControl {
                state: Box::new(state),
                placeholder: Box::new(self.current_placeholder_unit()),
            }));
        }
        // A `try` defers its body (and, from `advance_try`, each
        // subsequent phase) to a try-phase frame (see `Frame::try_ctx`).
        if let Some(req) = self.pending.try_phase.take() {
            return Ok(Some(Tick::PushTry {
                req: Box::new(req),
                initial_options: f.last_options.clone(),
            }));
        }
        let res = self
            .publish_native_interp_completion(res)
            .map_err(|error| crate::command::completion_from_tcl_error(self, error.into()))?;
        self.deliver_sync(f, res)
    }

    fn retain_jim_dispatch(
        &mut self,
        f: &mut Frame,
        words: &crate::NativeListItems,
        selected_key: Option<&str>,
        command: Option<&Command>,
    ) {
        if self.uses_jim_error_stack() {
            f.jim_evaluation.command_name = self
                .jim_original_command_reporting_name(&words[0])
                .or_else(|| selected_key.map(|key| self.command_display_key_bytes(key)))
                .map(|bytes| Value::from_native_string_bytes(bytes.as_bytes()));
            f.jim_evaluation.is_procedure = matches!(command, Some(Command::Proc(_)));
            f.jim_evaluation.invocation = if command.is_some() {
                Value::invocation_list_view(words)
            } else {
                Value::empty()
            };
            if let Some(location) = f.jim_evaluation.script.as_mut() {
                let line = f
                    .asm
                    .instructions
                    .get(f.pc.saturating_sub(1))
                    .map_or(1, |instruction| instruction.source_line.max(1));
                location.line = f
                    .source_location
                    .as_ref()
                    .map_or(1, |location| location.line)
                    .saturating_add(line.saturating_sub(1));
            }
            if let Some(current) = self.jim_errors.frames.last_mut() {
                *current = Self::borrow_jim_evaluation_frame(&f.jim_evaluation);
            }
        }
    }

    /// The untraced dispatch body — see [`Self::dispatch_words`].
    fn prepare_dispatch_procedure(
        &mut self,
        p: crate::command::NativeProcedureCommand,
        entered: Option<&EnteredCommand>,
        sidecar_handle: Option<&CommandSidecarHandle>,
        selected: Option<CommandSidecarKey>,
    ) -> Result<crate::command::PreparedProcedureActivation, Completion<Value>> {
        if let Some(entered) = entered {
            let sidecar = entered.sidecar.key();
            if sidecar.is_some() {
                self.ensure_proc_ready_in(p, sidecar.as_ref(), Some(&entered.sidecar))
            } else {
                // The entered token can outlive deletion, but publishing
                // its refreshed body by ProcDef::name would resurrect it
                // or overwrite a replacement created during substitution.
                self.ensure_proc_traced(p)
            }
        } else {
            let sidecar = selected;
            // A traced dispatch re-resolves after its callbacks. Keep the
            // handle only when it still identifies that resolved binding;
            // a callback may have renamed or replaced the original key.
            let sidecar_handle =
                sidecar_handle.filter(|handle| handle.key().as_ref() == sidecar.as_ref());
            self.ensure_proc_ready_in(p, sidecar.as_ref(), sidecar_handle)
        }
        .map_err(|error| crate::command::completion_from_tcl_error(self, error))
    }

    fn dispatch_words_inner(
        &mut self,
        f: &mut Frame,
        words: &crate::NativeListItems,
        sidecar_handle: Option<&CommandSidecarHandle>,
        entered: Option<&EnteredCommand>,
        context: tcl_core_types::NsId,
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Result<Option<Tick>, Completion<Value>> {
        let original = &words[0];
        let selected = if let Some(entered) = entered {
            let key = entered.sidecar.key().and_then(|key| match key {
                CommandSidecarKey::Visible(key) => Some(key),
                CommandSidecarKey::Hidden(_) => None,
            });
            (key, Some(entered.command.clone()))
        } else {
            match self
                .lookup_original_command_at(context, original)
                .map_err(|error| self.refuse_host_command(error.to_string()))?
            {
                Some((key, command)) => (Some(key), Some(command)),
                None => (None, None),
            }
        };
        let (selected_key, command) = selected;
        let jim_lease = command
            .as_ref()
            .and_then(|_| self.retain_original_jim_command(original, selected_key.as_deref()));
        self.retain_jim_dispatch(f, words, selected_key.as_deref(), command.as_ref());
        if command.is_some() {
            self.reset_native_jim_result()
                .map_err(|error| crate::command::completion_from_tcl_error(self, error.into()))?;
        }
        let saved_handler = self.take_native_handler_metadata();
        let usage_scope = self.enter_native_usage_scope(usage);
        self.set_invoked_name_value(original, selected_key.as_deref().unwrap_or(""));
        let result = self.dispatch_selected_words(
            f,
            words,
            command,
            NativeWordDispatch {
                selected_key: &selected_key,
                sidecar_handle,
                entered,
                context,
                usage,
            },
        );
        self.leave_native_usage_scope(usage_scope);
        self.restore_native_handler_metadata(saved_handler);
        if matches!(
            &result,
            Ok(Some(
                Tick::Call { .. }
                    | Tick::PushScript { .. }
                    | Tick::PushCatch(_)
                    | Tick::PushEval(_)
                    | Tick::PushSubst { .. }
                    | Tick::PushEachLoop { .. }
                    | Tick::PushControl { .. }
                    | Tick::PushExpression { .. }
                    | Tick::PushTry { .. }
            ))
        ) && let Some(lease) = jim_lease
        {
            let context = self.pending_exec_leave.get_or_insert_with(|| ExecLeaveCtx {
                jim_commands: Vec::new(),
                cmd_string: Value::empty(),
                leave_owner: None,
                step_scopes: 0,
            });
            context.jim_commands.push(lease);
        }
        result
    }

    fn dispatch_selected_words(
        &mut self,
        f: &mut Frame,
        words: &crate::NativeListItems,
        command: Option<Command>,
        dispatch: NativeWordDispatch<'_>,
    ) -> Result<Option<Tick>, Completion<Value>> {
        let original = &words[0];
        match command {
            Some(Command::Proc(p)) => {
                if let Some(result) = self.early_procedure_activation(&p, &words[1..])? {
                    self.deliver_sync(f, result)?;
                    return Ok(None);
                }
                let p = self.prepare_dispatch_procedure(
                    p,
                    dispatch.entered,
                    dispatch.sidecar_handle,
                    dispatch
                        .selected_key
                        .clone()
                        .map(CommandSidecarKey::visible),
                )?;
                Ok(Some(Tick::Call {
                    proc: Box::new(p),
                    invoked: words[0].clone(),
                    argv: words[1..].to_vec(),
                    lambda_registration: None,
                }))
            }
            Some(Command::Builtin(bf)) => {
                self.dispatch_builtin_words(f, words, bf, dispatch.entered, dispatch.context)
            }
            Some(Command::Native(cmd)) => {
                self.dispatch_native_words(f, words, cmd.as_ref(), dispatch)
            }
            Some(Command::CallerAlias(original_prefix)) => {
                let target = original_prefix
                    .native_object_list_elements(
                        tcl_syntax::native_string::NativeStringProtocol::Jim084,
                    )
                    .map_err(|error| {
                        crate::command::completion_from_cmd_error(self, error.into())
                    })?;
                let mut argv = target.as_ref().clone();
                argv.extend_from_slice(&words[1..]);
                let usage = self.alias_usage_rewrites_value(original, target.len(), dispatch.usage);
                self.dispatch_alias_words(
                    f,
                    &crate::NativeListItems::invocation_view(Rc::new(argv)),
                    dispatch.context,
                    &usage,
                )
            }
            Some(Command::Alias { words: target, .. }) => {
                let mut argv = (*target).clone();
                argv.extend_from_slice(&words[1..]);
                let usage = self.alias_usage_rewrites_value(original, target.len(), dispatch.usage);
                self.dispatch_alias_words(
                    f,
                    &crate::NativeListItems::invocation_view(Rc::new(argv)),
                    tcl_core_types::ROOT_NS,
                    &usage,
                )
            }
            Some(Command::CrossAlias {
                target: target_interp,
                words: target,
                ..
            }) => {
                let mut argv = (*target).clone();
                argv.extend_from_slice(&words[1..]);
                let usage = self.alias_usage_rewrites_value(original, target.len(), dispatch.usage);
                let res = self.invoke_alias_words_with_usage(
                    target_interp,
                    &argv,
                    tcl_registry::AliasTargetLookup::Global,
                    &usage,
                );
                self.settle_native_dispatch(f, res)
            }
            Some(Command::ChildInterp(child)) => {
                let res = self.dispatch_child(original, child, &words[1..]);
                self.settle_native_dispatch(f, res)
            }
            Some(Command::Ensemble(e)) => {
                let res = self.dispatch_ensemble("", &e, &words[1..]);
                self.settle_native_dispatch(f, res)
            }
            Some(Command::Object(key)) => {
                let res = crate::cmd_oo::oo_dispatch(self, key, original, &words[1..]);
                self.settle_native_dispatch(f, res)
            }
            None => {
                let result = self.invoke_missing_command_value(
                    dispatch.context,
                    original,
                    &words[1..],
                    tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
                );
                self.settle_native_dispatch(f, result)
            }
        }
    }

    fn dispatch_native_words(
        &mut self,
        frame: &mut Frame,
        words: &crate::NativeListItems,
        command: &dyn crate::command::NativeCommand,
        dispatch: NativeWordDispatch<'_>,
    ) -> Result<Option<Tick>, Completion<Value>> {
        let selected = dispatch
            .entered
            .and_then(|entered| entered.sidecar.key())
            .or_else(|| dispatch.sidecar_handle.and_then(CommandSidecarHandle::key))
            .or_else(|| {
                dispatch
                    .selected_key
                    .clone()
                    .map(CommandSidecarKey::visible)
            });
        let previous = std::mem::replace(&mut self.native_invocation.sidecar, selected);
        let res = command.invoke(self, &words[1..]);
        self.native_invocation.sidecar = previous;
        self.settle_native_dispatch(frame, res)
    }

    fn dispatch_builtin_words(
        &mut self,
        frame: &mut Frame,
        words: &crate::NativeListItems,
        builtin: crate::command::BuiltinFn,
        entered: Option<&EnteredCommand>,
        context: tcl_core_types::NsId,
    ) -> Result<Option<Tick>, Completion<Value>> {
        let key = self
            .resolve_original_command_key_at(context, &words[0])
            .map_err(|error| self.refuse_host_command(error.to_string()))?;
        self.set_invoked_name_value(&words[0], key.as_deref().unwrap_or(""));
        let identity = match entered {
            Some(entered) => entered.builtin_identity.clone(),
            None => self
                .jim_original_builtin_identity(&words[0])
                .or_else(|| key.as_ref().and_then(|key| self.stock_native_identity(key))),
        };
        self.retain_invoked_builtin_identity(identity);
        let previous_sidecar = std::mem::replace(
            &mut self.native_invocation.sidecar,
            key.map(CommandSidecarKey::visible),
        );
        let previous_arguments = self
            .native_invocation
            .arguments
            .replace(words.lifetime_view());
        let result = builtin(self, &words[1..]);
        self.native_invocation.sidecar = previous_sidecar;
        if result.code == Code::Error {
            self.observe_native_error_result(&result.result);
        }
        self.native_invocation.arguments = previous_arguments;
        self.settle_native_dispatch(frame, result)
    }

    fn dispatch_alias_words(
        &mut self,
        frame: &mut Frame,
        words: &crate::NativeListItems,
        context: tcl_core_types::NsId,
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Result<Option<Tick>, Completion<Value>> {
        let Some((head, arguments)) = words.split_first() else {
            return Err(err("alias target has no command"));
        };
        if self
            .resolve_original_command_key_at(context, head)
            .map_err(|error| self.refuse_host_command(error.to_string()))?
            .is_some()
        {
            return self.dispatch_words_entry_at(frame, words, None, context, usage, false);
        }
        let result = self.invoke_missing_command_value(
            context,
            head,
            arguments,
            tcl_registry::command_lookup::CommandLookupOrigin::AliasInvocation,
        );
        self.settle_native_dispatch(frame, result)
    }

    fn command_lookup_error_value(&mut self, original: &Value) -> Completion<Value> {
        let bytes = match self.native_name_operand_bytes(original) {
            Ok(bytes) => bytes,
            Err(error) => return self.refuse_host_command(error.to_string()),
        };
        // Both native command error formatters consume C-string reporting operands.
        // This reporting extent is independent from the already-selected table key.
        let Some(policy) = self.name_policy_protocol() else {
            return self
                .refuse_host_command("native command reporting protocol is unavailable".into());
        };
        let name = match tcl_syntax::naming::report_native_name_bytes(
            policy.recipe(),
            tcl_syntax::naming::NativeNameReportPurpose::CommandLookupError,
            &bytes,
        ) {
            Ok(name) => name,
            Err(error) => return self.refuse_host_command(format!("{error:?}")),
        };
        let mut message = b"invalid command name \"".to_vec();
        message.extend_from_slice(name);
        message.push(b'"');
        if policy.recipe().is_jim084() {
            let completion = crate::command::completion_from_cmd_error(
                self,
                tcl_cmd_core::CmdError::new_bytes(message),
            );
            return self.publish_native_lookup_failure(completion);
        }
        let mut code = b"TCL LOOKUP COMMAND ".to_vec();
        tcl_syntax::list::append_list_element(&mut code, name, false);
        let mut error = tcl_cmd_core::CmdError::with_error_code_bytes(message, code);
        if policy.authority() == tcl_syntax::naming::NamePolicyAuthority::Native {
            let Some(producer) = self
                .actual_native_invocation_dialect()
                .native_object_vector_protocol()
                .filter(|producer| producer.strings() == policy.string_protocol())
            else {
                return self.refuse_host_command(
                    "native missing-command result producer is unavailable".into(),
                );
            };
            if producer.missing_command_has_string_primary() {
                error = error.with_native_string_result(producer.strings());
            }
        }
        let completion = crate::command::completion_from_cmd_error(self, error);
        self.publish_native_lookup_failure(completion)
    }

    /// Failed lookup has no entered-handler epilogue to publish its result.
    /// Use the same interpreter result owner as an actual handler completion.
    fn publish_native_lookup_failure(
        &mut self,
        completion: Completion<Value>,
    ) -> Completion<Value> {
        self.publish_native_interp_completion(completion)
            .unwrap_or_else(|error| crate::command::completion_from_tcl_error(self, error))
    }

    fn invoke_missing_command_value(
        &mut self,
        context: tcl_core_types::NsId,
        original: &Value,
        arguments: &[Value],
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    ) -> Completion<Value> {
        use tcl_registry::command_lookup::{
            UnknownHandlerNamespace, native_lookup_fallback_policy,
        };
        let Some(policy) = native_lookup_fallback_policy(self.native_invocation_dialect(), origin)
        else {
            return self.command_lookup_error_value(original);
        };
        let mut words = match policy.namespace_handler {
            Some(selection) => {
                let selected = match selection {
                    UnknownHandlerNamespace::Caller => self.current_ns_id(),
                    UnknownHandlerNamespace::Lookup => context,
                };
                let root = self.ns_unknown_handler_at(selected);
                match tcl_syntax::value::ValueOps::list_elements(self, &root) {
                    Ok(words) => words,
                    Err(error) => {
                        return crate::command::completion_from_cmd_error(self, error.into());
                    }
                }
            }
            None => vec![Value::string(policy.default_handler)],
        };
        let Some(head) = words.first().cloned() else {
            return self.command_lookup_error_value(original);
        };
        let selected = match self.lookup_original_command_at(context, &head) {
            Ok(Some(selected)) => selected,
            Ok(None) => return self.command_lookup_error_value(original),
            Err(error) => return self.refuse_host_command(error.to_string()),
        };
        if let Err(refusal) = self.reset_native_ensemble_rewrite(
            tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent::AfterSuccessfulOrdinaryLookup,
        ) {
            return refusal;
        }
        words.push(original.clone());
        words.extend_from_slice(arguments);
        let previous = self.enter_missing_handler_namespace(context);
        let completion = self.invoke_resolved_command_value_with_usage(
            &head,
            CommandSidecarKey::visible(selected.0),
            selected.1,
            &words[1..],
            &[],
        );
        self.leave_missing_handler_namespace(previous);
        completion
    }

    /// Evaluate an original command object in an actual retained namespace token,
    /// including the selected ordinary-lookup rewrite reset events.
    pub(crate) fn invoke_command_value_at(
        &mut self,
        context: tcl_core_types::NsId,
        original: &Value,
        arguments: &[Value],
        usage: &[crate::command::NativeArgumentUsageRewrite],
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    ) -> Completion<Value> {
        self.invoke_command_value_entry_at(context, original, arguments, usage, origin, true)
    }

    /// Invoke an internally selected alias or ensemble target without ordinary
    /// evaluation reset events. Native handler metadata remains scoped.
    pub(crate) fn invoke_command_value_internal_at(
        &mut self,
        context: tcl_core_types::NsId,
        original: &Value,
        arguments: &[Value],
        usage: &[crate::command::NativeArgumentUsageRewrite],
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    ) -> Completion<Value> {
        self.invoke_command_value_entry_at(context, original, arguments, usage, origin, false)
    }

    fn prepare_command_value_entry(
        &mut self,
        context: tcl_core_types::NsId,
        original: &Value,
        ordinary: bool,
    ) -> Result<(Option<String>, u64), Completion<Value>> {
        use tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent as Event;
        if ordinary
            && let Err(refusal) = self.reset_native_ensemble_rewrite(Event::BeforeOrdinaryLookup)
        {
            return Err(refusal);
        }
        let key = match self.resolve_original_command_key_at(context, original) {
            Ok(key) => key,
            Err(error) => return Err(self.refuse_host_command(error.to_string())),
        };
        if ordinary
            && key.is_some()
            && let Err(refusal) =
                self.reset_native_ensemble_rewrite(Event::AfterSuccessfulOrdinaryLookup)
        {
            return Err(refusal);
        }
        let usage_revision = self.native_invocation.usage_revision;
        if let Some(refused) = self.refused_completion() {
            return Err(refused);
        }
        if let Some(exceeded) = self.charge_command() {
            return Err(exceeded);
        }
        Ok((key, usage_revision))
    }

    fn invoke_command_value_entry_at(
        &mut self,
        context: tcl_core_types::NsId,
        original: &Value,
        arguments: &[Value],
        usage: &[crate::command::NativeArgumentUsageRewrite],
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
        ordinary: bool,
    ) -> Completion<Value> {
        let (key, usage_revision) =
            match self.prepare_command_value_entry(context, original, ordinary) {
                Ok(key) => key,
                Err(completion) => return completion,
            };
        if !self.exec_traces_can_fire() {
            let result =
                self.invoke_command_value_inner(context, original, arguments, None, usage, origin);
            if result.code == Code::Error {
                self.observe_native_error_result(&result.result);
            }
            return result;
        }
        let mut words = vec![original.clone()];
        words.extend_from_slice(arguments);
        let cmd_string = Value::list(words);
        for scope in self.step_scopes_to_fire() {
            if scope.active_for("enterstep") && self.exec_trace_entry_live(&scope.key, &scope.entry)
            {
                let result = self.run_cmd_trace_callback(
                    &scope.entry,
                    &[cmd_string.clone(), Value::string("enterstep")],
                );
                if !result.code.is_ok() {
                    return result;
                }
            }
        }
        let sidecar_key =
            self.renamed_command_key(CommandSidecarKey::visible(key.unwrap_or_default()));
        let sidecar = self.active_sidecar(sidecar_key.clone());
        let own = self
            .exec_traces
            .get(&sidecar_key)
            .cloned()
            .unwrap_or_default();
        for entry in own.iter().rev() {
            if !sidecar.is_attached() {
                break;
            }
            if self.exec_trace_entry_live(&sidecar, entry) && entry.has_op("enter") {
                let result = self
                    .run_cmd_trace_callback(entry, &[cmd_string.clone(), Value::string("enter")]);
                if !result.code.is_ok() {
                    return result;
                }
            }
        }
        let resolved = match self.resolve_original_command_key_at(context, original) {
            Ok(key) => key
                .map(CommandSidecarKey::visible)
                .map(|key| self.renamed_command_key(key)),
            Err(error) => return self.refuse_host_command(error.to_string()),
        };
        let post = if resolved.as_ref() == sidecar.key().as_ref() {
            sidecar
                .key()
                .map(|key| self.exec_traces.get(&key).cloned().unwrap_or_default())
        } else {
            None
        };
        let leave_owner = (!own.is_empty() && post.is_some()).then(|| sidecar.clone());
        let step_scopes = self.push_exec_step_scopes(&sidecar, post.as_deref().unwrap_or_default());
        let ctx = ExecLeaveCtx {
            jim_commands: Vec::new(),
            cmd_string,
            leave_owner,
            step_scopes,
        };
        let usage = if usage_revision == self.native_invocation.usage_revision {
            usage
        } else {
            &[]
        };
        let result = self.invoke_command_value_inner(
            context,
            original,
            arguments,
            Some(&sidecar),
            usage,
            origin,
        );
        let result = self.finish_exec_leave(&ctx, &result).unwrap_or(result);
        if result.code == Code::Error {
            self.observe_native_error_result(&result.result);
        }
        result
    }

    /// Dispatch a fully-resolved command by `name` + `argv` to a completion —
    /// the `Commands::dispatch` engine. Unlike [`dispatch_words`](Self::dispatch_words)
    /// (bytecode-frame-coupled: it pushes the result onto `f.stack` and defers a
    /// proc call to the trampoline as a `Tick::Call`), this is self-contained and
    /// usable outside the bytecode loop: a builtin runs inline, a proc body runs
    /// to completion in a nested `is_proc` activation (so its call-frame and
    /// `return` are handled), and an alias re-evaluates its target prefix.
    pub fn invoke_command(&mut self, name: &str, argv: &[Value]) -> Completion<Value> {
        self.host_execution_depth += 1;
        let completion = self.invoke_host_object_vector(name, argv);
        self.host_execution_depth -= 1;
        assert!(
            self.activation_depth != 0
                || self.host_execution_depth != 0
                || self.execution_refusal.is_none(),
            "command requires a genuine native expression provider"
        );
        completion
    }

    /// Invoke an actual command while keeping neutral host refusals outside
    /// every Tcl completion code and guest control handler.
    ///
    /// # Errors
    /// Returns an expression-provider refusal after any preceding effects.
    pub fn try_invoke_command(
        &mut self,
        name: &str,
        argv: &[Value],
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        self.host_execution_depth += 1;
        let completion = self.invoke_host_object_vector(name, argv);
        self.host_execution_depth -= 1;
        self.finish_host_execution(completion)
    }

    /// Public object-vector evaluation has a native error-log frontier that
    /// internal command relays and bytecode invocation do not share.
    fn invoke_host_object_vector(&mut self, name: &str, argv: &[Value]) -> Completion<Value> {
        let original_head = Value::new_native_string_bytes(name.as_bytes());
        self.invoke_host_original_object_vector(&original_head, argv)
    }

    /// Actual object-vector entry, preserving the original counted command name.
    pub(crate) fn invoke_host_original_object_vector(
        &mut self,
        original_head: &Value,
        argv: &[Value],
    ) -> Completion<Value> {
        if self.uses_jim_error_stack() {
            return self.invoke_native_jim_object_vector(original_head, argv);
        }
        let completion = self.invoke_command_value_at(
            self.current_ns_id(),
            original_head,
            argv,
            &[],
            tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
        );
        if completion.code != Code::Error || self.execution_refusal.is_some() {
            return completion;
        }
        let Some(protocol) = self
            .actual_native_invocation_dialect()
            .native_error_log_protocol()
        else {
            return completion;
        };
        if protocol.observes_error_words(self.command_error_is_logged()) {
            let (text, retained_words) = if protocol.retains_list_words() {
                let mut words = Vec::with_capacity(argv.len() + 1);
                words.push(original_head.clone());
                words.extend_from_slice(argv);
                let original_list =
                    Value::native_list_constructor(words, protocol.string_protocol());
                match original_list.native_string_bytes(protocol.string_protocol()) {
                    Ok(bytes) => (bytes, Some(original_list)),
                    Err(error) => return self.refuse_host_command(error.to_string()),
                }
            } else {
                let mut words = Vec::with_capacity(argv.len() + 1);
                for original in std::iter::once(original_head).chain(argv) {
                    match original.native_string_bytes(protocol.string_protocol()) {
                        Ok(bytes) => words.push(bytes),
                        Err(error) => return self.refuse_host_command(error.to_string()),
                    }
                }
                (
                    Rc::<[u8]>::from(protocol.object_vector_command(&words)),
                    None,
                )
            };
            let message = match completion
                .result
                .native_string_bytes(protocol.string_protocol())
            {
                Ok(bytes) => bytes,
                Err(error) => return self.refuse_host_command(error.to_string()),
            };
            self.log_command_info(&text, &message, 1);
            drop(retained_words);
        }
        self.clear_error_logged();
        completion
    }

    fn invoke_native_jim_object_vector(
        &mut self,
        original: &Value,
        arguments: &[Value],
    ) -> Completion<Value> {
        let mut values = Vec::with_capacity(arguments.len() + 1);
        values.push(original.clone());
        values.extend_from_slice(arguments);
        // Jim_EvalObjVector retains every actual argument for the duration of
        // the invocation; frame metadata borrows that same vector.
        let words = crate::NativeListItems::invocation_view(Rc::new(values));
        let frame_index = self.jim_errors.frames.len();
        let parent = self
            .jim_errors
            .frames
            .last()
            .map(Self::borrow_jim_evaluation_frame);
        self.jim_errors
            .frames
            .push(tcl_runtime_api::jim_error_stack::JimEvaluationFrame {
                procedure_level: parent.as_ref().map_or(0, |frame| frame.procedure_level),
                command_name: None,
                is_procedure: false,
                script: parent.and_then(|frame| frame.script),
                invocation: Value::empty(),
            });
        self.jim_errors.vectors.push(NativeJimVectorFrame {
            frame_index,
            words: words.lifetime_view(),
        });
        let completion = self.invoke_command_value_at(
            self.current_ns_id(),
            &words[0],
            &words[1..],
            &[],
            tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
        );
        let invoked = self
            .jim_errors
            .frames
            .get(frame_index)
            .is_some_and(|frame| frame.command_name.is_some());
        if invoked && completion.code == Code::Error && self.execution_refusal.is_none() {
            self.capture_jim_error_stack();
        }
        let vector = self
            .jim_errors
            .vectors
            .pop()
            .expect("entered Jim vector frame");
        self.jim_errors.frames.truncate(vector.frame_index);
        // Pop the borrowed evaluation frame before releasing actual argv refs.
        drop(vector);
        drop(words);
        completion
    }

    fn enter_native_jim_vector_command(&mut self, original: &Value, command: &Command) {
        let Some(vector) = self.jim_errors.vectors.last() else {
            return;
        };
        if !vector.words[0].is_same_object(original) {
            return;
        }
        let frame_index = vector.frame_index;
        let invocation = Value::invocation_list_view(&vector.words);
        let Some(frame) = self.jim_errors.frames.get_mut(frame_index) else {
            return;
        };
        frame.command_name = Some(original.native_lifetime_lease().into_value());
        frame.is_procedure = matches!(command, Command::Proc(_));
        frame.invocation = invocation;
    }

    /// Resolve a coroutine relay in its captured namespace without changing
    /// the resumer's variable frame or the target's authored invocation name.
    pub(crate) fn invoke_command_in_lookup_namespace(
        &mut self,
        namespace: &str,
        name: &str,
        argv: &[Value],
    ) -> Completion<Value> {
        self.invoke_command_in_lookup_namespace_with_usage(namespace, name, argv, &[])
    }

    pub(crate) fn invoke_command_in_lookup_namespace_with_usage(
        &mut self,
        namespace: &str,
        name: &str,
        argv: &[Value],
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Completion<Value> {
        self.invoke_command_in_lookup_namespace_with_origin(
            namespace,
            name,
            argv,
            usage,
            tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
        )
    }

    pub(crate) fn invoke_command_in_lookup_namespace_with_origin(
        &mut self,
        namespace: &str,
        name: &str,
        argv: &[Value],
        usage: &[crate::command::NativeArgumentUsageRewrite],
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    ) -> Completion<Value> {
        let context = match self.native_command_lookup_context(Some(namespace)) {
            Ok(context) => context,
            Err(error) => return self.refuse_host_command(error.to_string()),
        };
        self.invoke_command_value_at(context, &Value::string(name), argv, usage, origin)
    }

    /// Invoke an already-resolved command without consulting the visible
    /// command table.  Hidden-command dispatch uses this to keep a hidden
    /// binding private while preserving its display spelling and trace key.
    pub(crate) fn invoke_resolved_command(
        &mut self,
        display_name: &str,
        trace_key: CommandSidecarKey,
        command: Command,
        argv: &[Value],
    ) -> Completion<Value> {
        self.invoke_resolved_command_with_usage(display_name, trace_key, command, argv, &[])
    }

    fn invoke_resolved_command_with_usage(
        &mut self,
        display_name: &str,
        trace_key: CommandSidecarKey,
        command: Command,
        argv: &[Value],
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Completion<Value> {
        self.invoke_resolved_command_value_with_usage(
            &Value::string(display_name),
            trace_key,
            command,
            argv,
            usage,
        )
    }

    pub(crate) fn invoke_resolved_command_value_with_usage(
        &mut self,
        original: &Value,
        trace_key: CommandSidecarKey,
        command: Command,
        argv: &[Value],
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Completion<Value> {
        if let Some(exceeded) = self.charge_command() {
            return exceeded;
        }
        if !self.exec_traces_can_fire() {
            return self.invoke_resolved_command_inner(
                original,
                command,
                argv,
                Some(trace_key),
                None,
                usage,
            );
        }
        let mut words = Vec::with_capacity(argv.len() + 1);
        words.push(original.clone());
        words.extend_from_slice(argv);
        let cmd_string = Value::list(words);
        // As in `dispatch_words_traced`: a rename window moved the trace list
        // to the destination key, and both names reach it.
        let trace_key = self.renamed_command_key(trace_key);
        for scope in self.step_scopes_to_fire() {
            if scope.active_for("enterstep") && self.exec_trace_entry_live(&scope.key, &scope.entry)
            {
                let r = self.run_cmd_trace_callback(
                    &scope.entry,
                    &[cmd_string.clone(), Value::string("enterstep")],
                );
                if !r.code.is_ok() {
                    return r;
                }
            }
        }
        let sidecar = self.active_sidecar(trace_key.clone());
        let own_at_entry = self
            .exec_traces
            .get(&trace_key)
            .cloned()
            .unwrap_or_default();
        for entry in own_at_entry.iter().rev() {
            if !sidecar.is_attached() {
                break;
            }
            if !self.exec_trace_entry_live(&sidecar, entry) {
                continue;
            }
            if entry.has_op("enter") {
                let r = self
                    .run_cmd_trace_callback(entry, &[cmd_string.clone(), Value::string("enter")]);
                if !r.code.is_ok() {
                    return r;
                }
            }
        }
        let post_enter = self.post_enter_trace_snapshot(None, None, &sidecar, None);
        let leave_owner =
            (!own_at_entry.is_empty() && post_enter.is_some()).then(|| sidecar.clone());
        let pushed =
            self.push_exec_step_scopes(&sidecar, post_enter.as_deref().unwrap_or_default());
        let ctx = ExecLeaveCtx {
            jim_commands: Vec::new(),
            cmd_string,
            leave_owner,
            step_scopes: pushed,
        };
        let Some(live_key) = sidecar.key() else {
            return err("attempt to invoke a deleted command");
        };
        let c = self.invoke_resolved_command_inner(
            original,
            command,
            argv,
            Some(live_key),
            Some(&sidecar),
            usage,
        );
        match self.finish_exec_leave(&ctx, &c) {
            Some(replacement) => replacement,
            None => c,
        }
    }

    /// Settle a synchronously-run native command on the native re-entry path
    /// ([`Self::invoke_command`]): a deferred body runs via a nested drive,
    /// else the completion is the command's own. The
    /// [`dispatch_words`](Self::dispatch_words) twin is
    /// [`settle_native_dispatch`](Self::settle_native_dispatch); the two differ
    /// only in having a trampoline to push onto.
    fn settle_native_invoke(&mut self, res: Completion<Value>) -> Completion<Value> {
        if let Some(refused) = self.refused_completion() {
            return refused;
        }
        if let Some(req) = self.pending.procedure_call.take() {
            return match self.enter_proc(
                &req.proc.proc,
                &req.proc.body,
                &req.invoked,
                &req.arguments,
            ) {
                Ok(()) => {
                    self.install_native_procedure_binding(req.proc.declaration_binding);
                    let mut frame = Frame::new(req.proc.body, true);
                    frame.lambda_registration = Some(req.registration);
                    self.run_activation(frame)
                }
                Err(completion) => {
                    self.retire_original_lambda_registration(&req.registration);
                    completion
                }
            };
        }
        if let Some(req) = self.pending.expression.take() {
            let placeholder = self.current_placeholder_unit();
            return self.run_activation(Frame::new_expression(req, placeholder));
        }
        if let Some(state) = self.pending.control.take() {
            let placeholder = self.current_placeholder_unit();
            return self.run_activation(Frame::new_control(state, placeholder));
        }
        // An `eval`/`uplevel`/`apply`-style builtin deferred its body to
        // the pending eval request. This call site is on the native Rust stack (no
        // trampoline to push onto), so run the body via a nested drive —
        // a `yield` inside cannot cross it, exactly like every other
        // `invoke_command` re-entry.
        if let Some(req) = self.pending.eval.take() {
            return self.run_activation(Frame::new_eval(req));
        }
        // A `catch` deferred its body: run it via a nested drive (a `yield`
        // inside cannot cross this native re-entry), then absorb its
        // completion with the catch epilogue.
        if let Some(req) = self.pending.catch.take() {
            let mut comp = self.run_activation(Frame::new_script(req.script, None));
            if comp.code == Code::Ok
                && let Some(tail) = req.fatal_tail
            {
                comp = self.raise_fatal_tail(tail);
            }
            return self.finish_catch(
                comp,
                req.resvar.as_ref(),
                req.optvar.as_ref(),
                req.ignored_codes,
            );
        }
        // A `subst` deferred: run the scanner-driven subst frame via a
        // nested drive (its `[…]` bodies can't yield across this native
        // re-entry), returning its accumulated result.
        if let Some(req) = self.pending.subst.take() {
            let placeholder = self.current_placeholder_unit();
            return self.run_activation(Frame::new_subst(req, placeholder));
        }
        // A `foreach`/`lmap` runtime-fallback loop deferred: run the
        // scanner-driven each-loop frame via a nested drive (a `yield` in
        // its body can't cross this native re-entry either).
        if let Some(req) = self.pending.each_loop.take() {
            let placeholder = self.current_placeholder_unit();
            return self.run_activation(Frame::new_each_loop(req, placeholder));
        }
        // A `try` deferred its body: run it (and, via `Vm::unwind`'s own
        // `try_ctx` handling, every subsequent phase `advance_try`
        // decides on) via a nested drive — a `yield` anywhere in it
        // can't cross this native re-entry either. `unwind` pushes each
        // next phase onto the same nested `acts`, so one
        // `run_activation` call carries the whole construct through to
        // its final completion.
        if let Some(req) = self.pending.try_phase.take() {
            return self.run_activation(Frame::new_try(req, Value::empty()));
        }
        res
    }

    fn invoke_command_value_inner(
        &mut self,
        context: tcl_core_types::NsId,
        original: &Value,
        argv: &[Value],
        sidecar_handle: Option<&CommandSidecarHandle>,
        usage: &[crate::command::NativeArgumentUsageRewrite],
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    ) -> Completion<Value> {
        match self.lookup_original_command_at(context, original) {
            Ok(Some((key, command))) => {
                let sidecar = Some(CommandSidecarKey::visible(key));
                let handle =
                    sidecar_handle.filter(|handle| handle.key().as_ref() == sidecar.as_ref());
                self.invoke_resolved_command_inner(original, command, argv, sidecar, handle, usage)
            }
            Ok(None) => self.invoke_missing_command_value(context, original, argv, origin),
            Err(error) => self.refuse_host_command(error.to_string()),
        }
    }

    fn invoke_resolved_command_inner(
        &mut self,
        original: &Value,
        command: Command,
        argv: &[Value],
        sidecar: Option<CommandSidecarKey>,
        sidecar_handle: Option<&CommandSidecarHandle>,
        usage: &[crate::command::NativeArgumentUsageRewrite],
    ) -> Completion<Value> {
        let selected_key = sidecar
            .as_ref()
            .and_then(|key| match key {
                CommandSidecarKey::Visible(key) => Some(key.as_str()),
                CommandSidecarKey::Hidden(_) => None,
            })
            .unwrap_or("");
        let _jim_lease = self.retain_original_jim_command(original, Some(selected_key));
        self.enter_native_jim_vector_command(original, &command);
        let saved_handler = self.take_native_handler_metadata();
        let usage_scope = self.enter_native_usage_scope(usage);
        self.set_invoked_name_value(original, selected_key);
        if let Err(error) = self.reset_native_jim_result() {
            self.leave_native_usage_scope(usage_scope);
            self.restore_native_handler_metadata(saved_handler);
            return crate::command::completion_from_tcl_error(self, error.into());
        }
        let result = (|| {
            match command {
                Command::Builtin(bf) => {
                    let saved = std::mem::replace(&mut self.native_invocation.sidecar, sidecar);
                    let res = bf(self, argv);
                    self.native_invocation.sidecar = saved;
                    self.settle_native_invoke(res)
                }
                Command::Native(cmd) => {
                    let saved = std::mem::replace(&mut self.native_invocation.sidecar, sidecar);
                    let res = cmd.invoke(self, argv);
                    self.native_invocation.sidecar = saved;
                    self.settle_native_invoke(res)
                }
                Command::Proc(p) => {
                    match self.early_procedure_activation(&p, argv) {
                        Ok(Some(result)) => return result,
                        Ok(None) => {}
                        Err(error) => return error,
                    }
                    let p = match self.ensure_proc_ready_in(p, sidecar.as_ref(), sidecar_handle) {
                        Ok(proc) => proc,
                        Err(error) => {
                            return crate::command::completion_from_tcl_error(self, error);
                        }
                    };
                    match self.enter_proc(&p.proc, &p.body, original, argv) {
                        Ok(()) => {
                            self.install_native_procedure_binding(p.declaration_binding);
                            self.run_activation(Frame::new(p.body, true))
                        }
                        Err(c) => c,
                    }
                }
                Command::CallerAlias(original_prefix) => {
                    let target = match original_prefix.native_object_list_elements(
                        tcl_syntax::native_string::NativeStringProtocol::Jim084,
                    ) {
                        Ok(target) => target,
                        Err(error) => {
                            return crate::command::completion_from_cmd_error(self, error.into());
                        }
                    };
                    let mut full: Vec<Value> = target.as_ref().clone();
                    full.extend_from_slice(argv);
                    let usage = self.alias_usage_rewrites_value(original, target.len(), usage);
                    self.invoke_alias_words_with_usage(
                        self.cur_interp(),
                        &full,
                        tcl_registry::AliasTargetLookup::CallerNamespace,
                        &usage,
                    )
                }
                Command::Alias { words: target, .. } => {
                    // Target resolves in the GLOBAL namespace, caller's frame kept
                    // — see the `dispatch_words` Alias arm for the tclsh pins.
                    let mut full: Vec<Value> = (*target).clone();
                    full.extend_from_slice(argv);
                    let here = self.cur_interp();
                    let usage = self.alias_usage_rewrites_value(original, target.len(), usage);
                    self.invoke_alias_words_with_usage(
                        here,
                        &full,
                        tcl_registry::AliasTargetLookup::Global,
                        &usage,
                    )
                }
                // Cross-interp alias: switch to the target interp and run the words
                // there.  Identical to the bytecode path — a cross-interp alias
                // works from a native re-entry (coroutine resume, `lsort
                // -command`, `invoke_command`), unlike C Tcl's shared C stack,
                // where the same re-entry raises `cannot invoke parent-interp
                // alias: C stack busy`.
                Command::CrossAlias {
                    target: target_interp,
                    words: target,
                    ..
                } => {
                    let mut full: Vec<Value> = (*target).clone();
                    full.extend_from_slice(argv);
                    let usage = self.alias_usage_rewrites_value(original, target.len(), usage);
                    self.invoke_alias_words_with_usage(
                        target_interp,
                        &full,
                        tcl_registry::AliasTargetLookup::Global,
                        &usage,
                    )
                }
                Command::ChildInterp(child) => self.dispatch_child(original, child, argv),
                Command::Ensemble(e) => self.dispatch_ensemble("", &e, argv),
                Command::Object(key) => crate::cmd_oo::oo_dispatch(self, key, original, argv),
                // Miss fallback chain (see `dispatch_words`): `namespace unknown`
                // handler first, then the plain `unknown` proc, then a hard error.
            }
        })();
        self.leave_native_usage_scope(usage_scope);
        self.restore_native_handler_metadata(saved_handler);
        self.publish_native_interp_completion(result)
            .unwrap_or_else(|error| crate::command::completion_from_tcl_error(self, error.into()))
    }

    /// Run a `TclOO` method body: enter the proc activation, link the object's
    /// instance variables into the fresh frame, push the OO call context, run
    /// the body to completion, then pop the context. This is the OO analogue of
    /// [`invoke_command`](Self::invoke_command)'s `Proc` arm plus the
    /// instance-variable linking the runtime's `run_proc` does.
    ///
    /// `link_vars` retain exact local and object-namespace storage keys. The
    /// actual procedure namespace token selects their physical table. The `frame` carries
    /// the resolved method chain that `self`/`my`/
    /// `next` consult while the body runs.
    pub(crate) fn oo_run_method(
        &mut self,
        activation: crate::command::PreparedProcedureActivation,
        argv: &[Value],
        link_vars: &[(tcl_core_types::NameBytes, tcl_core_types::NameBytes)],
        frame: crate::cmd_oo::OoFrame,
    ) -> Completion<Value> {
        let proc = &activation.proc;
        let invoked =
            Value::from_native_string_bytes(proc.actual_command_slot().simple.as_bytes().to_vec());
        if let Err(c) = self.enter_proc(proc, &activation.body, &invoked, argv) {
            return c;
        }
        if let Some(layout) = &activation.body.compiled_local_layout {
            let protocol = self
                .name_policy_protocol()
                .expect("entered method naming protocol")
                .recipe();
            for (slot, primary) in layout.names.iter().enumerate() {
                let Some(primary) = primary else { continue };
                for (local, storage) in link_vars {
                    let matches = tcl_syntax::naming::native_oo_variable_resolver_matches(
                        protocol,
                        tcl_syntax::naming::NativeOoVariableResolverPurpose::CompiledPrimary,
                        local.as_bytes(),
                        primary.as_bytes(),
                    );
                    match matches {
                        Ok(false) => continue,
                        Ok(true) => {}
                        Err(error) => {
                            self.pop_call_frame();
                            self.pop_ns();
                            return self.refuse_host_command(format!(
                                "TclOO compiled variable resolver: {error:?}"
                            ));
                        }
                    }
                    if let Err(error) = self.add_tcloo_compiled_instance_link(
                        slot,
                        proc.actual_namespace_id(),
                        storage.as_bytes(),
                    ) {
                        self.pop_call_frame();
                        self.pop_ns();
                        return crate::command::upvar_link_error_bytes(
                            error,
                            storage.as_bytes(),
                            primary.as_bytes(),
                        );
                    }
                    break;
                }
            }
        }
        self.install_native_procedure_binding(activation.declaration_binding);
        let mut frame = frame;
        frame.activation = Some(self.native_oo_variable_activation());
        self.oo.call_stack.push(frame);
        let result = self.run_activation(Frame::new(activation.body, true));
        self.oo.call_stack.pop();
        result
    }

    fn tick_original_invocation(&mut self, frame: &mut Frame, original: Value) -> Tick {
        let Some(protocol) = self
            .actual_native_invocation_dialect()
            .native_string_protocol()
        else {
            return Tick::Return(
                self.refuse_host_command("original coroutine invocation List issuer".into()),
            );
        };
        let members = match self.native_object_list_elements_in(&original, protocol) {
            Ok(members) => members,
            Err(error) => {
                return Tick::Return(crate::command::completion_from_tcl_error(
                    self,
                    error.into(),
                ));
            }
        };
        let Some((namespace, words)) = members.split_first() else {
            return Tick::Return(
                self.refuse_host_command("original coroutine namespace operand".into()),
            );
        };
        let context = match self.namespace_object_lookup(namespace) {
            Ok(Some(namespace)) => namespace,
            Ok(None) => {
                return Tick::Return(match self.native_name_operand_bytes(namespace) {
                    Ok(bytes) => self.namespace_lookup_error_bytes(&bytes),
                    Err(error) => self.refuse_host_command(error.to_string()),
                });
            }
            Err(error) => {
                return Tick::Return(crate::command::completion_from_cmd_error(
                    self,
                    error.into(),
                ));
            }
        };
        let words = crate::NativeListItems::invocation_view(Rc::new(
            words
                .iter()
                .map(|word| word.native_lifetime_lease().into_value())
                .collect(),
        ));
        frame.tailcall_owners.push(original);
        match self.dispatch_words_entry_at(frame, &words, None, context, &[], true) {
            Ok(Some(tick)) => tick,
            Ok(None) => Tick::Continue,
            Err(completion) => Tick::Return(completion),
        }
    }

    pub(crate) fn original_tailcall_request(
        &mut self,
        words: &[Value],
    ) -> Result<TailcallReq, Completion<Value>> {
        if !self.frame_owns_local_variables(self.current_level()) {
            return Err(crate::command::err_with_code(
                "tailcall can only be called from a proc, lambda or method",
                "TCL TAILCALL ILLEGAL",
            ));
        }
        let namespace = tcl_cmd_core::namespace::current_original(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let protocol = self
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .ok_or_else(|| self.refuse_host_command("original tailcall List issuer".into()))?;
        let mut members = Vec::with_capacity(words.len() + 1);
        members.push(namespace);
        members.extend_from_slice(words);
        let original = Value::native_list_constructor(members, protocol);
        let values = self
            .native_object_list_elements_in(&original, protocol)
            .map_err(|error| crate::command::completion_from_tcl_error(self, error.into()))?;
        let (namespace, words) = values.split_first().expect("original namespace prefix");
        Ok(TailcallReq {
            namespace: namespace.native_lifetime_lease().into_value(),
            words: words
                .iter()
                .map(|word| word.native_lifetime_lease().into_value())
                .collect(),
            original_list: Some(original),
        })
    }

    /// Preserve the real namespace-prefixed List and borrow its original members.
    fn native_tailcall_tick(&mut self, list: Value) -> Tick {
        let Some(protocol) = self
            .actual_native_invocation_dialect()
            .native_string_protocol()
        else {
            return Tick::Return(self.refuse_host_command("native tailcall List issuer".into()));
        };
        let members = match self.native_object_list_elements_in(&list, protocol) {
            Ok(members) => members,
            Err(error) => {
                return Tick::Return(crate::command::completion_from_tcl_error(
                    self,
                    error.into(),
                ));
            }
        };
        let Some((namespace, words)) = members.split_first() else {
            return Tick::Return(err("tailcall: missing namespace"));
        };
        let request = TailcallReq {
            namespace: namespace.native_lifetime_lease().into_value(),
            words: words
                .iter()
                .map(|value| value.native_lifetime_lease().into_value())
                .collect(),
            original_list: Some(list),
        };
        Tick::Tailcall(request)
    }

    /// Dispatch a procedure-owned replacement after its frame and leave
    /// callbacks have completed. The target's own activation remains yieldable.
    fn dispatch_tailcall(
        &mut self,
        acts: &mut Vec<Frame>,
        request: TailcallReq,
    ) -> Option<Completion<Value>> {
        let TailcallReq {
            namespace,
            words,
            original_list,
        } = request;
        let context = match self.namespace_object_lookup(&namespace) {
            Ok(Some(context)) => context,
            Ok(None) => {
                let completion = match self.native_name_operand_bytes(&namespace) {
                    Ok(bytes) => self.namespace_lookup_error_bytes(&bytes),
                    Err(error) => self.refuse_host_command(error.to_string()),
                };
                return self.deliver_tailcall_completion(acts, completion);
            }
            Err(error) => {
                let completion = crate::command::completion_from_cmd_error(self, error.into());
                return self.deliver_tailcall_completion(acts, completion);
            }
        };
        let words = crate::NativeListItems::invocation_view(Rc::new(words));
        if acts.is_empty() {
            let asm = FunctionAsm {
                instructions: vec![Instruction::new(Op::DONE, vec![])],
                ..FunctionAsm::default()
            };
            let unit = self.compiled_unit(Rc::new(asm), self.source_namespace_path());
            self.push_frame(acts, Frame::new(unit, false));
        }
        let parent = acts
            .last_mut()
            .expect("replacement result activation present");
        if let Some(owner) = original_list {
            parent.tailcall_owners.push(owner);
        }
        match self.dispatch_words_entry_at(parent, &words, None, context, &[], true) {
            Ok(Some(mut tick)) => {
                if let Tick::PushEachLoop { req, .. } = &mut tick {
                    req.invocation = Some((words.clone(), 1));
                }
                parent.deferred_dispatch = Some(Box::new(tick));
                None
            }
            Ok(None) => {
                let completion = Completion::new(
                    Code::Ok,
                    pop(parent),
                    std::mem::replace(&mut parent.last_options, Value::empty()),
                );
                self.deliver_tailcall_completion(acts, completion)
            }
            Err(completion) => self.deliver_tailcall_completion(acts, completion),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{char_find, imm_index};
    use crate::interp::Vm;
    use crate::value::Value;
    use tcl_bytecode::INDEX_END;
    use tcl_syntax::value::ValueOps;

    #[test]
    fn procedure_activation_retains_original_invocation_references_until_exit() {
        // Source contract: naming.error.original-invocation-context-capture
        // docs/design/analysis/name-resolution-proofs/error-original-invocation-context-capture.md
        // This checks VM ownership, independently of native public stack bytes.
        let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        let invoked = Value::new_native_string_bytes(b"original-head".as_slice());
        let argument = Value::new_native_string_bytes(b"original-argument".as_slice());
        let head_loan = invoked.native_lifetime_lease();
        let argument_loan = argument.native_lifetime_lease();
        let mut frame = super::Frame::new(vm.current_placeholder_unit(), true);
        frame._procedure_invocation = Some((invoked, vec![argument]));
        let (head, arguments) = frame._procedure_invocation.as_ref().unwrap();
        assert!(head.is_same_object(head_loan.value()));
        assert!(arguments[0].is_same_object(argument_loan.value()));
        assert_eq!(head.native_object_reference_count(), 1);
        assert_eq!(arguments[0].native_object_reference_count(), 1);
        assert!(head.native_object_is_live());
        assert!(arguments[0].native_object_is_live());
        drop(frame);
        assert!(!head_loan.value().native_object_is_live());
        assert!(!argument_loan.value().native_object_is_live());
    }

    #[test]
    fn original_trace_procedure_heads_remain_live_during_their_actual_activation() {
        // Source contract: naming.error.original-invocation-context-capture
        // docs/design/analysis/name-resolution-proofs/error-original-invocation-context-capture.md
        // The guest source is a software lifetime control, not a native fixture.
        struct Inspect(std::rc::Rc<std::cell::Cell<usize>>);
        impl crate::command::NativeCommand for Inspect {
            fn invoke(&self, vm: &mut Vm, _: &[Value]) -> tcl_runtime_api::Completion<Value> {
                let words = vm.frame_argv(vm.current_level()).unwrap();
                let head = &words[0];
                assert!(head.native_object_is_live());
                assert!(head.native_object_reference_count() > 1);
                self.0.set(self.0.get() + 1);
                crate::interp::ok(Value::empty())
            }
        }
        for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = crate::native_fixture::interpreter(profile);
            let count = std::rc::Rc::new(std::cell::Cell::new(0));
            vm.register_native_command("inspect", std::rc::Rc::new(Inspect(count.clone())));
            let completion = vm.try_eval_source(
                "proc observer {n1 n2 op} {inspect; error READ_FAIL}; proc work {} {set d {k 3}; trace add variable d read observer; catch {dict lappend d k Y Z} m; trace remove variable d read observer; return $m}; work",
            ).unwrap();
            assert_eq!(completion.code, tcl_runtime_api::Code::Ok, "{engine}");
            assert_eq!(count.get(), 1, "{engine}");
        }
    }

    #[test]
    fn direct_object_vector_errors_observe_original_words_at_the_native_frontier() {
        struct Failure;
        impl crate::command::NativeCommand for Failure {
            fn invoke(&self, _: &mut Vm, _: &[Value]) -> tcl_runtime_api::Completion<Value> {
                crate::interp::err("FAIL")
            }
        }
        let observations = [
            (
                "tcl8.4",
                include_str!("../tests/data/native_object_vector_log/8.4.20.tsv"),
            ),
            (
                "tcl8.5",
                include_str!("../tests/data/native_object_vector_log/8.5.19.tsv"),
            ),
            (
                "tcl8.6",
                include_str!("../tests/data/native_object_vector_log/8.6.18.tsv"),
            ),
            (
                "tcl9.0",
                include_str!("../tests/data/native_object_vector_log/9.0.4.tsv"),
            ),
            (
                "tcl9.1",
                include_str!("../tests/data/native_object_vector_log/9.1.0.tsv"),
            ),
            (
                "jim",
                include_str!("../tests/data/native_object_vector_log/Jim.tsv"),
            ),
        ];
        let mut compared = 0;
        for (engine, rows) in observations {
            for row in rows.lines().filter(|row| row.starts_with("0\t")) {
                let fields = row.split('\t').collect::<Vec<_>>();
                tcl_test_support::oracle_row_progress("objectvector24", engine, fields[1], None);
                let mut vm = Vm::new();
                vm.set_dialect_profile(
                    tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
                );
                vm.register_native_command("fail", std::rc::Rc::new(Failure));
                let original = match fields[1] {
                    "0" => Value::int(17),
                    "1" | "3" => Value::new_native_string_bytes(b"A\0B".as_slice()),
                    "2" => Value::new_native_string_bytes(b"#hash".as_slice()),
                    _ => unreachable!("native fixed controls"),
                };
                let resident = |value: &Value| {
                    usize::from(value.resident_string_bytes().is_some()).to_string()
                };
                assert_eq!(
                    resident(&original),
                    fields[2].strip_prefix("before=").unwrap(),
                    "{engine}/{row}"
                );
                let mut arguments = vec![
                    Value::new_native_string_bytes(b"direct".as_slice()),
                    original,
                ];
                if fields[1] == "3" {
                    arguments.push(Value::new_native_string_bytes(b"TAIL".as_slice()));
                }
                let completion = vm.try_invoke_command("fail", &arguments).unwrap();
                assert_eq!(
                    completion.code.as_int().to_string(),
                    fields[4].strip_prefix("code=").unwrap(),
                    "{engine}/{row}"
                );
                assert_eq!(
                    resident(&arguments[1]),
                    fields[3].strip_prefix("after=").unwrap(),
                    "{engine}/{row}"
                );
                let info = vm.error_info_value().unwrap_or_default().iter().fold(
                    String::new(),
                    |mut text, byte| {
                        use std::fmt::Write as _;
                        write!(&mut text, "{byte:02x}").unwrap();
                        text
                    },
                );
                assert_eq!(
                    info,
                    fields[5].strip_prefix("info=").unwrap(),
                    "{engine}/{row}"
                );
                compared += 1;
            }
        }
        assert_eq!(compared, 24);
    }

    #[test]
    fn namespace_unknown_getter_retains_the_accepted_root_object() {
        for version in tcl_dialect::TclVersion::ALL
            .into_iter()
            .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
        {
            let mut vm = Vm::new();
            vm.set_runtime_version(version);
            let root = Value::string("::unknown tag");
            let length = ValueOps::list_len(&mut vm, &root).unwrap();
            vm.ns_unknown_set(root.clone(), length);
            assert!(vm.ns_unknown_get().unwrap().is_same_object(&root));
            let blank = Value::string(" \t ");
            let length = ValueOps::list_len(&mut vm, &blank).unwrap();
            assert_eq!(length, 0);
            vm.ns_unknown_set(blank, length);
            assert!(!vm.ns_unknown_get().unwrap().is_same_object(&root));
        }
    }

    #[test]
    fn unknown_handler_lookup_mutates_its_original_command_object() {
        use std::rc::Rc;
        use tcl_core_types::NameBytes;
        use tcl_runtime_api::{CommandSlot, ROOT_NS};
        struct Handler(&'static str);
        impl crate::command::NativeCommand for Handler {
            fn invoke(&self, _vm: &mut Vm, args: &[Value]) -> tcl_runtime_api::Completion<Value> {
                assert_eq!(
                    args[0].resident_string_bytes().unwrap().as_ref(),
                    b"missing"
                );
                crate::interp::ok(Value::string(self.0))
            }
        }
        for version in tcl_dialect::TclVersion::ALL
            .into_iter()
            .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
        {
            let mut vm = Vm::new();
            vm.set_runtime_version(version);
            let slot = CommandSlot {
                namespace: ROOT_NS,
                simple: NameBytes::from(b"h\xff".as_slice()),
            };
            vm.register_command_in_slot(
                slot.clone(),
                crate::command::Command::Native(Rc::new(Handler("FIRST"))),
            );
            let head = Value::new_native_string_bytes(b"h\xff".to_vec());
            let protocol = vm
                .native_invocation_dialect()
                .native_string_materialization(None)
                .unwrap()
                .protocol();
            vm.ns_unknown_set(
                Value::native_list_constructor(vec![head.clone()], protocol),
                1,
            );
            assert!(head.native_command_name_cache().is_none());
            let missing = Value::string("missing");
            let invoke = |vm: &mut Vm| {
                vm.invoke_command_value_at(
                    ROOT_NS,
                    &missing,
                    &[],
                    &[],
                    tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
                )
            };
            let first = invoke(&mut vm);
            assert!(first.code.is_ok());
            assert_eq!(
                first.result.resident_string_bytes().unwrap().as_ref(),
                b"FIRST"
            );
            let before = head
                .native_command_name_cache()
                .expect("native unknown-prefix getter");
            assert_eq!(before.version, version);
            vm.register_command_in_slot(
                slot,
                crate::command::Command::Native(Rc::new(Handler("SECOND"))),
            );
            let second = invoke(&mut vm);
            assert!(second.code.is_ok());
            assert_eq!(
                second.result.resident_string_bytes().unwrap().as_ref(),
                b"SECOND"
            );
            assert_ne!(head.native_command_name_cache().unwrap(), before);
            assert_eq!(head.resident_string_bytes().unwrap().as_ref(), b"h\xff");
        }
    }

    #[test]
    fn namespace_introspection_instructions_use_original_native_command_objects() {
        use std::rc::Rc;
        use tcl_bytecode::{FunctionAsm, Instruction, Op};
        use tcl_core_types::NameBytes;
        use tcl_runtime_api::{CommandSlot, ROOT_NS};
        struct EmptyCommand;
        impl crate::command::NativeCommand for EmptyCommand {
            fn invoke(&self, _vm: &mut Vm, _args: &[Value]) -> tcl_runtime_api::Completion<Value> {
                crate::interp::ok(Value::empty())
            }
        }
        fn unhex(text: &str) -> Vec<u8> {
            text.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        }
        let fixtures = [
            include_str!("../tests/data/native_namespace_command_names/8.4.20.tsv"),
            include_str!("../tests/data/native_namespace_command_names/8.5.19.tsv"),
            include_str!("../tests/data/native_namespace_command_names/8.6.18.tsv"),
            include_str!("../tests/data/native_namespace_command_names/9.0.4.tsv"),
            include_str!("../tests/data/native_namespace_command_names/9.1.0.tsv"),
        ];
        let mut compared = 0;
        for (version, fixture) in tcl_dialect::TclVersion::ALL.into_iter().zip(fixtures) {
            assert_eq!(fixture.lines().count(), 6);
            for row in fixture.lines() {
                let columns = row.split('\t').collect::<Vec<_>>();
                assert_eq!(columns.len(), 6);
                let op = match columns[0] {
                    "which" => Op::RESOLVE_CMD,
                    "origin" => Op::ORIGIN_CMD,
                    _ => panic!("unknown native namespace observation"),
                };
                let written = unhex(columns[5]);
                let expected = unhex(columns[3]);
                // Compare the command and opcode through independent original
                // objects, so the command cannot donate a cache to the opcode.
                for compiled in [false, true] {
                    let mut vm = Vm::new();
                    vm.set_runtime_version(version);
                    vm.register_command_in_slot(
                        CommandSlot {
                            namespace: ROOT_NS,
                            simple: NameBytes::from(b"opaque\xff".as_slice()),
                        },
                        crate::command::Command::Native(Rc::new(EmptyCommand)),
                    );
                    let original = Value::new_native_string_bytes(written.clone());
                    let completion = if compiled {
                        let unit = vm.compiled_unit(
                            Rc::new(FunctionAsm {
                                instructions: vec![Instruction::new(op, vec![])],
                                ..FunctionAsm::default()
                            }),
                            vm.source_namespace_path(),
                        );
                        let mut frame = super::Frame::new(unit, false);
                        frame.stack.push(original.clone());
                        match vm.tick(&mut frame) {
                            super::Tick::Continue => crate::interp::ok(frame.stack.pop().unwrap()),
                            super::Tick::Return(completion) => completion,
                            _ => panic!("namespace introspection must finish its original getter"),
                        }
                    } else {
                        let mut args = vec![Value::string(columns[0])];
                        if op == Op::RESOLVE_CMD {
                            args.push(Value::string("-command"));
                        }
                        args.push(original.clone());
                        vm.try_invoke_command("namespace", &args).unwrap()
                    };
                    assert_eq!(
                        completion.code.as_int().to_string(),
                        columns[2],
                        "{version:?} {row} compiled={compiled}"
                    );
                    assert_eq!(
                        vm.native_string_bytes(&completion.result).unwrap().as_ref(),
                        expected,
                        "{version:?} {row} compiled={compiled}"
                    );
                    assert_eq!(
                        original.native_command_name_cache_origin(),
                        (columns[4] == "cmdName").then_some(version)
                    );
                    assert_eq!(original.string_bytes().as_ref(), written);
                    compared += 1;
                }
            }
        }
        assert_eq!(compared, 60);
    }

    /// A dict `Value`'s top-level `(key, value-string)` pairs, mirroring the
    /// production `dict_pairs` decode.
    fn top_pairs(v: &Value) -> Vec<(String, String)> {
        v.as_list()
            .expect("dict value is a valid list")
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| (c[0].to_str().to_string(), c[1].to_str().to_string()))
            .collect()
    }

    #[test]
    fn imm_index_decodes_literal_and_end_relative() {
        // A plain non-negative immediate is the literal index.
        assert_eq!(imm_index(3, 10), 3);
        assert_eq!(imm_index(0, 10), 0);
        // `INDEX_END` is `end`; `INDEX_END - k` is `end-k`.
        assert_eq!(imm_index(INDEX_END, 10), 9);
        assert_eq!(imm_index(INDEX_END - 1, 10), 8);
        // `end` of an empty list is -1 (one before the first slot).
        assert_eq!(imm_index(INDEX_END, 0), -1);
    }

    #[test]
    fn char_find_returns_character_indices_not_byte_offsets() {
        // ASCII: first vs last occurrence, and a miss.
        assert_eq!(char_find("hello", "l", false), 2);
        assert_eq!(char_find("hello", "l", true), 3);
        assert_eq!(char_find("hello", "z", false), -1);
        // Multi-byte: `é` is two UTF-8 bytes, but the result is a *character*
        // index — `string first`/`last` operate on characters, not bytes.
        assert_eq!(char_find("héllo", "é", false), 1);
        assert_eq!(char_find("héllo", "llo", false), 2);
        assert_eq!(char_find("héllo", "l", true), 3);
        // An empty needle is a *miss*, not a match at 0 / at the end: C's
        // `TclStringFirst`/`TclStringLast` special-case `ln == 0` and leave the
        // result at -1 ("We don't find empty substrings.  Bizarre!",
        // `tclStringObj.c:3853-3858`, `:3948-3956`), where Rust's
        // `str::find`/`rfind` would answer 0 and 5.
        assert_eq!(char_find("hello", "", false), -1);
        assert_eq!(char_find("hello", "", true), -1);
        assert_eq!(char_find("", "", false), -1);
    }

    #[test]
    fn dict_set_path_creates_updates_and_autovivifies() {
        let mut vm = Vm::new();
        let s = Value::string;
        // `dict set {} a 1` → {a 1}.
        let d = vm
            .dict_set_path(&Value::list(vec![]), &[s("a")], s("1"))
            .unwrap();
        assert_eq!(top_pairs(&d), [("a".into(), "1".into())]);

        // Updating an existing key keeps its position (order-preserving).
        let base = Value::list(vec![s("a"), s("1"), s("b"), s("2")]);
        let updated = vm.dict_set_path(&base, &[s("a")], s("9")).unwrap();
        assert_eq!(
            top_pairs(&updated),
            [("a".into(), "9".into()), ("b".into(), "2".into())]
        );

        // A new key appends at the end.
        let appended = vm.dict_set_path(&base, &[s("c")], s("3")).unwrap();
        assert_eq!(
            top_pairs(&appended),
            [
                ("a".into(), "1".into()),
                ("b".into(), "2".into()),
                ("c".into(), "3".into())
            ]
        );

        // A multi-key path auto-vivifies intermediate dicts: the inner dict's
        // list string-rep is "b 1".
        let nested = vm
            .dict_set_path(&Value::list(vec![]), &[s("a"), s("b")], s("1"))
            .unwrap();
        assert_eq!(top_pairs(&nested), [("a".into(), "b 1".into())]);
    }

    #[test]
    fn dict_unset_path_removes_and_is_a_noop_when_absent() {
        let mut vm = Vm::new();
        let s = Value::string;
        let base = Value::list(vec![s("a"), s("1"), s("b"), s("2")]);

        // Removing a present key drops just that pair.
        let removed = vm.dict_unset_path(&base, &[s("a")]).unwrap();
        assert_eq!(top_pairs(&removed), [("b".into(), "2".into())]);

        // Removing an absent key leaves the dict unchanged (matching Tcl).
        let untouched = vm.dict_unset_path(&base, &[s("z")]).unwrap();
        assert_eq!(
            top_pairs(&untouched),
            [("a".into(), "1".into()), ("b".into(), "2".into())]
        );

        // A nested unset rewrites only the inner dict: {a {b 1 c 2}} → {a {c 2}}.
        let inner = Value::list(vec![s("b"), s("1"), s("c"), s("2")]);
        let outer = Value::list(vec![s("a"), inner]);
        let nested = vm.dict_unset_path(&outer, &[s("a"), s("b")]).unwrap();
        assert_eq!(top_pairs(&nested), [("a".into(), "c 2".into())]);
    }

    /// The shared `lset` core backs the compiled `INST_LSET_LIST`/`INST_LSET_FLAT`
    /// opcodes; a naive implementation recursing once per index in `lset`'s
    /// (possibly nested) index path has no depth cap, so a flat index path is
    /// trivially inflated via `lset listVar {*}[lrepeat 100000 0] v`,
    /// overflowing the native stack (SIGABRT) between depth 1800 and 2000 on
    /// a 2 MiB thread (`cargo test`'s per-test default). The iterative
    /// implementation has no such cap; this test checks exact correctness at
    /// depth 2000, comfortably past that crash range — descending back down
    /// the same index path lands on the value that was set, not merely
    /// survival.
    ///
    /// Deliberately NOT 50,000+: constructing (and, at the end of this
    /// test, dropping) a `Value::list` chain nested that deep is its own,
    /// unrelated native-stack risk — `Value` has no custom `Drop` impl, so
    /// the compiler-generated recursive drop glue walks the same chain
    /// (empirically, SIGABRT between depth 3500 and 4000 on a 2 MiB thread
    /// for construction+drop alone, independent of any operation performed
    /// on the value). That is a separate, genuinely unbounded-depth concern
    /// in `Value`'s representation itself, not in the core's iterative
    /// logic, and this test does not cover it.
    #[test]
    fn deeply_nested_lset_survives_and_is_correct() {
        const DEPTH: usize = 2_000;
        let mut v = Value::string("orig");
        for _ in 0..DEPTH {
            v = Value::list(vec![v]);
        }
        let path: Vec<Value> = (0..DEPTH).map(|_| Value::int(0)).collect();
        let mut vm = Vm::new();
        let result = tcl_cmd_core::list::lset(
            &mut vm,
            &v,
            &path,
            Value::string("new"),
            tcl_dialect::TclVersion::V9_0,
        )
        .expect("the lset core survives");
        let mut cur = result;
        for _ in 0..DEPTH {
            let items = cur.as_list().expect("valid list at every level");
            assert_eq!(items.len(), 1);
            cur = items[0].clone();
        }
        assert_eq!(&*cur.to_str(), "new");
    }

    /// A moderately nested `lset` index path (well within realistic use) is
    /// byte-for-byte unaffected by the iterative rewrite.
    #[test]
    fn moderately_nested_lset_matches_previous_behavior() {
        let n = Value::int;
        let mut vm = Vm::new();
        let mut lset = |list: &Value, path: &[Value], value: Value| {
            tcl_cmd_core::list::lset(&mut vm, list, path, value, tcl_dialect::TclVersion::V9_0)
        };
        // Set an existing element two levels deep.
        let list = Value::list(vec![
            Value::list(vec![n(1), n(2)]),
            Value::list(vec![n(3), n(4)]),
        ]);
        let updated = lset(&list, &[n(1), n(0)], n(99)).unwrap();
        assert_eq!(&*updated.to_str(), "{1 2} {99 4}");

        // An empty path replaces the whole value (`lset x {} v` == `set x v`).
        let replaced = lset(&list, &[], Value::string("whole")).unwrap();
        assert_eq!(&*replaced.to_str(), "whole");

        // `idx == len` appends a fresh slot.
        let flat = Value::list(vec![n(1), n(2)]);
        let appended = lset(&flat, &[n(2)], n(3)).unwrap();
        assert_eq!(&*appended.to_str(), "1 2 3");

        // Out-of-range and non-numeric indices still error.
        assert!(lset(&flat, &[n(5)], n(0)).is_err());
        assert!(lset(&flat, &[Value::string("bogus")], n(0)).is_err());
    }
}

#[cfg(test)]
#[path = "exec/native_missing_command_result_tests.rs"]
mod native_missing_command_result_tests;

#[cfg(test)]
#[path = "exec/active_manifest_tests.rs"]
mod active_manifest_tests;
