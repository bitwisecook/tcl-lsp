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
mod native_limit_error_tests;
#[cfg(test)]
#[path = "exec/native_list_index_tests.rs"]
mod native_list_index_tests;
mod native_list_insertion;
#[cfg(test)]
#[path = "exec/native_list_operations_tests.rs"]
mod native_list_operations_tests;
#[cfg(test)]
mod native_namespace_string_tests;
mod native_scalar;
#[cfg(test)]
#[path = "exec/native_string_result_effect_tests.rs"]
mod native_string_result_effect_tests;
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

use crate::interp::native_operation_currency::{NativeOperationCurrency, NativeOperationScope};
#[cfg(test)]
use tcl_bytecode::Instruction;
use tcl_bytecode::{FunctionAsm, ModuleAsm, Op};
use tcl_runtime_api::{Code, Completion, FatalTail};

use crate::command::Command;
#[cfg(test)]
use crate::interp::ok;
use crate::interp::{CmdTraceEntry, CommandSidecarHandle, CommandSidecarKey, Vm, err};
use crate::value::Value;

/// Private dispatch entry metadata; only the sealed C import preload owner
/// supplies the direct route that bypasses ordinary evaluator reset/trace stages.
struct ResolvedCommandEntry<'a> {
    sidecar: Option<CommandSidecarKey>,
    handle: Option<&'a CommandSidecarHandle>,
    direct_import: Option<crate::interp::OriginalCImportPreload>,
}

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

/// Whether bytecode-entry bookkeeping has completed for this activation.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BytecodeEntry {
    Pending,
    Entered,
}

/// How an error leaving an activation is logged.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ActivationErrorLog {
    /// By the activation's own instructions and body frames.
    Own,
    /// A compiled `subst` template or a runtime expression: like C's TEBC
    /// leaving `Tcl_SubstObj`'s or `Tcl_ExprObj`'s bytecode, the command that
    /// invoked it is logged too.
    InvokingCommand,
}

/// One activation record owned by the interpreter or a parked coroutine.
pub(crate) struct Frame {
    jim_script_entry: Option<crate::native_jim_script::NativeJimScriptEntry>,
    jim_script: Option<crate::native_jim_script::NativeJimScriptState>,
    asm: Rc<FunctionAsm>,
    fixed_math_calls: Option<Rc<HashMap<usize, crate::interp::NativeFixedMathCall>>>,
    literal_pool: crate::literal_pool::NativeLiteralPoolReceipt,
    direct_source_operands: Option<Box<crate::literal_pool::PlainSourceOperands>>,
    jim_evaluation: crate::interp::OriginalJimEvaluationFrame,
    source_location: Option<tcl_runtime_api::script_source_location::ScriptSourceLocation>,
    /// Canonical unrooted namespace whose command bindings this bytecode was
    /// specialised against.
    source_namespace: crate::compiled::CompiledUnitNamespace,
    /// Dialect-profile generation under which this activation's bytecode was
    /// compiled.
    /// Stored frames cannot be rewound/recompiled after a profile switch (in
    /// particular a suspended coroutine has a live PC and operand stack), so
    /// the trampoline rejects stale execution before another specialised
    /// opcode can run.
    profile_generation: u64,
    native_policy: Option<crate::compiled::NativeCompilerPolicy>,
    /// The policy stamp at which this frame's policy checks last passed.
    policy_current_at: std::cell::Cell<Option<u64>>,
    /// Name-world generation, namespace token and actual profile under which
    /// this frame's compiled namespace last proved current.
    namespace_current_at: std::cell::Cell<Option<(u64, tcl_core_types::NsId, usize)>>,
    /// Distinguishes this activation from any other at the same stack depth.
    serial: u64,
    /// Command-binding generation last validated for this live activation.
    command_epoch: u64,
    /// Compiler-service generation that produced this activation's bytecode.
    /// Native Script activations have no compiler generation, including after
    /// their deferred preparation fails or their active backing is retired.
    compiler_generation: Option<u64>,
    /// The manifest of the module this activation's unit came from.
    manifest: Option<std::sync::Arc<tcl_runtime_api::ArtefactIdentityManifest>>,
    off2idx: Rc<rustc_hash::FxHashMap<i32, usize>>,
    /// `FOREACH_START` index → paired `FOREACH_STEP` index (the implicit jump).
    foreach_pairs: Rc<HashMap<usize, usize>>,
    pc: usize,
    stack: Vec<Value>,
    /// Original tailcall Lists retain their member headers through target completion.
    tailcall_owners: Vec<Value>,
    original_invocation: Option<Value>,
    /// The original invocation this coroutine base activation dispatched
    /// (`[namespace, words…]`), kept for the error trace.
    dispatched_invocation: Option<Value>,
    /// Original invoking stack references remain owned through the callee
    /// activation; the reporting `CallFrame` separately borrows lifetime views.
    procedure_invocation: Option<(Value, Vec<Value>)>,
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
    /// The stack operands the instruction at this index consumed before it
    /// failed, for TIP 348's `INNER` context (`TclGetInnerContext`).
    failed_operands: Option<(usize, crate::NativeListItems)>,
    native_bytecode_entry: BytecodeEntry,
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
    /// Authentic OO procedure exit keeps readonly original cleanup observations.
    oo_call_exit: Option<crate::interp::oo_cleanup::OriginalOoCallExit>,
    /// Original physical target and weak plain-procedure cleanup births.
    /// Neither retains an entered Scope across coroutine suspension.
    original_target: Option<crate::interp::native_operation_currency::SavedExecutionTarget>,
    procedure_call_exit: Option<crate::interp::oo_cleanup::OriginalOoCallExit>,
    /// A *transparent* script activation (`eval`/`uplevel`/`catch`/`subst` body
    /// run on the explicit stack so a `yield` inside it stays yieldable). It owns
    /// no `Vm` call-frame; on completion its result is delivered to the parent
    /// exactly as an inline command's would be — an `ok` result is pushed to the
    /// parent's operand stack, and a `break`/`continue` is offered to the parent's
    /// enclosing loop rather than unwinding through it. Mutually exclusive with
    /// `is_proc`.
    is_script: bool,
    /// The instruction starting the command of a directly evaluated script
    /// this activation last began (`Instruction::script_command`): the
    /// outermost command C's level-0 `TclEvalEx` names when a completion
    /// leaves it, whichever command inside it raised the code.
    script_command: Option<usize>,
    /// A coroutine's base activation created by a top-level statement: its
    /// completion is settled as at `numLevels == 0` on every resume.
    settles_as_top_level: bool,
    /// The units an emulated release that keeps the commands running bytecode
    /// compiled runs this activation's instructions in, and when it entered
    /// each ([`crate::release_units`]).
    release_units: crate::release_units::ReleaseUnits,
    /// Ambient frame namespace replaced by a transparent boundary-replay
    /// activation.  Replay changes command resolution without adding a Tcl
    /// call frame, so the namespace entry at the current frame depth is
    /// replaced in place and restored when this activation unwinds.
    replay_namespace_restore: Option<(
        Option<tcl_core_types::NameBytes>,
        crate::interp::namespace_context::NamespaceExecutionContext,
    )>,
    /// Caller frames hidden by a foreign `uplevel`, retained across suspension.
    selected_frame_restore: Option<crate::interp::SelectedFrameRestore>,
    /// For a script activation, the `errorInfo` body label the uncompiled command
    /// would add on error (`("eval" body line N)` / `("uplevel" body line N)`).
    /// `None` for a command-substitution `EVAL_STK`, which adds no body frame.
    body_label: Option<&'static str>,
    /// Whether an error leaving this activation logs its invoking command.
    error_log: ActivationErrorLog,
    /// Entered invocation retained when a tailcall removes its issuing frame.
    body_invocation: Option<(crate::NativeListItems, u32)>,
    /// Set on a package discovery activation: the `package require` that
    /// resumes once it completes.
    package: Option<Box<crate::cmd_package::SuspendedRequire>>,
    /// The words of a tailcall replacement this activation dispatched in place
    /// of a procedure it called: an error leaving the replacement logs them
    /// (`invoked from within "r {x y} z"`) before this activation's command.
    tailcall_invocation: Option<crate::NativeListItems>,
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

/// A suspended continuation retains currency without an entered invocation.
/// Actual entered execution or readonly saved currency on an authentic continuation.
#[derive(Clone)]
pub(crate) enum OriginalExecutionOwner {
    Entered(NativeOperationScope),
    Parked(NativeOperationCurrency),
}

impl OriginalExecutionOwner {
    pub(crate) fn currency(&self) -> &NativeOperationCurrency {
        match self {
            Self::Entered(scope) => scope.currency(),
            Self::Parked(currency) => currency,
        }
    }

    fn check(&self, vm: &mut Vm) -> Result<(), tcl_cmd_core::CmdError> {
        self.currency().check(vm)
    }

    pub(crate) fn park(&mut self) {
        *self = Self::Parked(self.currency().clone());
    }

    pub(crate) fn resume(
        &mut self,
        vm: &mut Vm,
    ) -> Result<NativeOperationScope, tcl_cmd_core::CmdError> {
        let entered = match self {
            Self::Entered(scope) => {
                scope.check(vm)?;
                scope.clone()
            }
            Self::Parked(currency) => NativeOperationScope::resume_saved(vm, currency)
                .map_err(tcl_cmd_core::CmdError::from_execution_refusal)?,
        };
        *self = Self::Entered(entered.clone());
        Ok(entered)
    }
}

/// Original execution owners retained through deferred activation and cleanup.
/// Trace eligibility separately retains the invoked command string, attached
/// leave owner and step scopes (popped before firing).
pub(crate) struct ExecLeaveCtx {
    original_scopes: Vec<OriginalExecutionOwner>,
    traces: bool,
    jim_commands: Vec<crate::interp::native_jim_lookup::JimCommandLease>,
    cmd_string: Option<Value>,
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
    evaluates_word: bool,
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
    /// A `package require` this discovery script belongs to, resumed when
    /// the script completes ([`crate::cmd_package::advance_require`]).
    pub(crate) package: Option<Box<crate::cmd_package::SuspendedRequire>>,
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
    /// The release compiles this `catch` and evaluates its script word inside
    /// the catch range of the code that holds it (C's `INST_EVAL_STK`).
    pub(crate) evaluates_word: bool,
}

/// What [`Vm::settle_native_invoke`] runs for a natively settled invocation.
enum NativeSettlement {
    /// Nothing was deferred: the invocation's own completion stands.
    Unchanged,
    /// The deferred work finished before any activation ran.
    Finished(Completion<Value>),
    /// Run this activation; its completion is the result.
    Run(Box<Frame>),
    /// Run a deferred `catch` body, then its epilogue.
    Catch(Box<Frame>, Box<NativeCatchEpilogue>),
}

/// The part of a [`CatchReq`] its epilogue consumes after the body ran.
struct NativeCatchEpilogue {
    resvar: Option<Value>,
    optvar: Option<Value>,
    ignored_codes: u64,
    fatal_tail: Option<FatalTail>,
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
    context: &'a crate::interp::namespace_context::NamespaceExecutionContext,
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

/// The interpreter's reference to a traced command's result, given up
/// without releasing an object the completion still carries unowned.
struct HeldResult(Option<Value>);

impl Drop for HeldResult {
    fn drop(&mut self) {
        if let Some(result) = self.0.take() {
            drop(result.into_native_unowned_lifetime());
        }
    }
}

struct EnteredNativeOperation {
    site: (usize, usize),
    start: usize,
    end: usize,
    requirements: Vec<tcl_runtime_api::CommandBindingIdentity>,
}

/// A process-unique activation serial.
fn next_frame_serial() -> u64 {
    static NEXT_SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT_SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// The current native compiler policy's stamp, projecting the policy itself
/// only when a frame's checks must compare it.
pub(crate) struct PolicyAtStamp<'a> {
    vm: &'a Vm,
    stamp: u64,
}

impl<'a> PolicyAtStamp<'a> {
    fn of(vm: &'a Vm) -> Self {
        Self {
            vm,
            stamp: vm.native_compiler_policy_stamp(),
        }
    }

    fn policy(&self) -> crate::compiled::NativeCompilerPolicy {
        self.vm.native_compiler_policy()
    }
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
    pub(crate) original_scope: OriginalExecutionOwner,
}

/// How a [`Vm::drive`](Vm) invocation ended: the activation stack emptied
/// (`Done`), or a coroutine suspended (`Yielded`, `acts` left frozen).
pub(crate) enum RunExit {
    Done(Completion<Value>),
    Yielded(YieldReq),
}

macro_rules! variable_try_op {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(c) => return Tick::Return(c),
        }
    };
}

// A failing compiled variable instruction logs its command, as C's bytecode
// engine does before it looks for a catch range.
macro_rules! variable_try_logged {
    ($vm:expr, $instr:expr, $e:expr) => {
        match $e {
            Ok(v) => v,
            Err(c) => {
                if c.code == Code::Error {
                    let (text, line, enclosing) = logged_command($instr);
                    let message = c.result.string_bytes();
                    let context = if $instr.evaluation == tcl_runtime_api::CommandEvaluation::Direct
                    {
                        Value::from_native_string_bytes(text.clone())
                    } else {
                        crate::exec::completion::instruction_inner_context(&$vm, $instr, &[])
                    };
                    $vm.log_command_with_enclosing(&text, context, &enclosing, &message, line);
                }
                return Tick::Return(c);
            }
        }
    };
}

// Portable operation failures retain byte diagnostics and host refusal tags.
macro_rules! variable_try_core {
    ($vm:expr, $operation:expr) => {
        match $operation {
            Ok(value) => value,
            Err(error) => {
                return Tick::Return(crate::command::completion_from_cmd_error($vm, error.into()));
            }
        }
    };
}

macro_rules! variable_operand_bytes {
    ($vm:expr, $value:expr) => {{
        let operand = $value;
        match $vm.native_name_operand_bytes_checked(&operand) {
            Ok(bytes) => bytes,
            Err(error) => {
                return Tick::Return(crate::command::completion_from_cmd_error($vm, error));
            }
        }
    }};
}

mod activation;
mod completion;
mod control_activations;
mod dispatch;
mod instructions;

pub(crate) use completion::LoggedCommand;
#[cfg(test)]
use instructions::cmp;
use instructions::pop;
#[cfg(test)]
use instructions::{bin, char_find, imm_index};

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
        self.run_activation(Box::new(Frame::new(unit, false)))
    }

    /// Run a retained canonical object body through the ordinary control driver.
    pub(crate) fn run_original_object_body(
        &mut self,
        original: Value,
        words: crate::NativeListItems,
    ) -> Completion<Value> {
        let state = crate::cmd_control::ControlState::object_body_eval(original, words);
        let placeholder = self.current_placeholder_unit();
        self.run_activation(Box::new(Frame::new_control(state, placeholder)))
    }

    fn original_scope_context(original: NativeOperationScope) -> ExecLeaveCtx {
        ExecLeaveCtx {
            original_scopes: vec![OriginalExecutionOwner::Entered(original)],
            traces: false,
            jim_commands: Vec::new(),
            cmd_string: None,
            leave_owner: None,
            step_scopes: 0,
        }
    }

    fn original_scope_refusal(
        &mut self,
        original: &NativeOperationScope,
    ) -> Option<Completion<Value>> {
        original
            .check(self)
            .err()
            .map(|error| crate::command::completion_from_cmd_error(self, error))
    }

    fn original_owner_refusal(
        &mut self,
        original: &OriginalExecutionOwner,
    ) -> Option<Completion<Value>> {
        original
            .check(self)
            .err()
            .map(|error| crate::command::completion_from_cmd_error(self, error))
    }

    fn finish_original_completion(
        &mut self,
        original: &NativeOperationScope,
        reached: Completion<Value>,
    ) -> Completion<Value> {
        if let Some(first) = self.original_scope_refusal(original) {
            drop(reached);
            let _ = original.check(self);
            first
        } else {
            reached
        }
    }

    fn finish_original_dispatch<T>(
        &mut self,
        original: &NativeOperationScope,
        reached: Result<T, Completion<Value>>,
    ) -> Result<T, Completion<Value>> {
        if let Some(first) = self.original_scope_refusal(original) {
            drop(reached);
            let _ = original.check(self);
            Err(first)
        } else {
            reached
        }
    }

    fn exec_leave_scope_refusal(&mut self, context: &ExecLeaveCtx) -> Option<Completion<Value>> {
        context
            .original_scopes
            .iter()
            .find_map(|original| self.original_owner_refusal(original))
    }

    fn settle_original_exec_leave(
        &mut self,
        context: ExecLeaveCtx,
        reached: Completion<Value>,
    ) -> Completion<Value> {
        let original = context.original_scopes.clone();
        let replacement = self.finish_exec_leave(&context, &reached);
        drop(context);
        let reached = replacement.unwrap_or(reached);
        for scope in &original {
            if let Some(first) = self.original_owner_refusal(scope) {
                drop(reached);
                let _ = scope.check(self);
                return first;
            }
        }
        reached
    }

    fn settle_pending_exec_leave(&mut self, completion: Completion<Value>) -> Completion<Value> {
        match self.pending_exec_leave.take() {
            Some(ctx) => self.settle_original_exec_leave(ctx, completion),
            None => completion,
        }
    }

    fn settle_exec_leaves(
        &mut self,
        contexts: &mut Vec<ExecLeaveCtx>,
        mut completion: Completion<Value>,
    ) -> Completion<Value> {
        for ctx in contexts.drain(..).rev() {
            completion = self.settle_original_exec_leave(ctx, completion);
        }
        completion
    }
}

#[cfg(test)]
mod original_name_dispatch_tests;

#[cfg(test)]
mod original_activation_cleanup_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "exec/native_missing_command_result_tests.rs"]
mod native_missing_command_result_tests;

#[cfg(test)]
#[path = "exec/active_manifest_tests.rs"]
mod active_manifest_tests;
